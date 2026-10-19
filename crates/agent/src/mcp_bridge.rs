//! Native stdio framing and fixed-workspace discovery; memory policy stays remote.
use crate::{Client, StoredDevice, decode, workspace};
use anyhow::{Result, anyhow, ensure};
use recollect_protocol::{CheckoutRefreshResult, WorkspaceCatalogue};
use rmcp::{
    ErrorData, Peer, RoleClient, RoleServer, ServerHandler, ServiceExt,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
    },
    service::RequestContext,
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc, time::Duration};
use uuid::Uuid;

type Fingerprint = (
    Option<(std::time::SystemTime, u64)>,
    Option<(PathBuf, std::time::SystemTime, u64)>,
);

#[derive(Clone)]
struct Bridge {
    client: Arc<Client>,
    device: Arc<StoredDevice>,
    brain: Uuid,
    workspace_root: Option<PathBuf>,
    peer: Peer<RoleClient>,
    instructions: Option<String>,
    plugin_guard: bool,
    /// Fingerprint of the files the plugin destination was last validated from.
    destination_seen: Arc<std::sync::Mutex<Option<Fingerprint>>>,
}
fn bounded(result: CallToolResult) -> CallToolResponse {
    if serde_json::to_vec(&result).is_ok_and(|bytes| bytes.len() < 1024 * 1024 - 1024) {
        result.into()
    } else {
        CallToolResult::error(vec![ContentBlock::text("Result exceeds the bridge response limit. Reduce the requested page or context budget. Inspect mutation history before retrying.")]).into()
    }
}
fn unavailable() -> ErrorData {
    ErrorData::internal_error(
        "Recollect is unavailable or authority changed. Inspect command history before retrying a mutation.",
        None,
    )
}
fn refresh_tool() -> Tool {
    Tool::new(
        "workspace.refresh",
        "Refresh metadata for this bridge's fixed workspace using bounded Git discovery. Does not fetch, extract, read source files or change task scope.",
        Arc::new(
            json!({"type":"object","properties":{},"additionalProperties":false})
                .as_object()
                .unwrap()
                .clone(),
        ),
    )
}
impl Bridge {
    /// Files that decide the destination: the plugin configuration and the
    /// nearest workspace selector. Their metadata fingerprints the validation.
    fn destination_fingerprint() -> Result<Fingerprint> {
        let stamp = |path: &std::path::Path| {
            let meta = std::fs::symlink_metadata(path).ok()?;
            Some((meta.modified().ok()?, meta.len()))
        };
        let config = stamp(&crate::plugin_storage::root()?.join("config.json"));
        let selector = std::env::current_dir()?
            .ancestors()
            .map(|parent| parent.join(".recollect/workspace.toml"))
            .find(|path| std::fs::symlink_metadata(path).is_ok());
        let selector = selector.and_then(|path| {
            let (at, len) = stamp(&path)?;
            Some((path, at, len))
        });
        Ok((config, selector))
    }
    async fn check_plugin_destination(&self) -> Result<()> {
        if !self.plugin_guard {
            return Ok(());
        }
        // The full check can resolve a named selector over the network and runs
        // for every tool call; repeat it only after those files changed.
        let fingerprint = Self::destination_fingerprint()?;
        if self
            .destination_seen
            .lock()
            .is_ok_and(|seen| seen.as_ref() == Some(&fingerprint))
        {
            return Ok(());
        }
        let (config, _) = crate::plugin_runtime::selected(
            crate::plugin_storage::config()?,
            &std::env::current_dir()?,
        )
        .await?;
        ensure!(
            config.brain == self.brain
                && config.device == self.device.device_id
                && config.endpoint == self.client.endpoint,
            "Plugin destination changed. Reconnect this host's MCP server before using its tools."
        );
        if let Ok(mut seen) = self.destination_seen.lock() {
            *seen = Some(fingerprint);
        }
        Ok(())
    }
    async fn refresh(&self) -> Result<Value> {
        // Fresh remote discovery establishes current role before local scanning.
        let tools = self
            .peer
            .list_all_tools()
            .await
            .map_err(|_| anyhow!("workspace_authority_unavailable"))?;
        ensure!(
            tools.iter().any(|t| t.name == "memory.contribute"),
            "workspace_writer_required"
        );
        let expected = self
            .workspace_root
            .as_ref()
            .ok_or_else(|| anyhow!("workspace_unavailable"))?;
        let (root, selector) = workspace::selector(expected).await?;
        ensure!(root == *expected, "workspace_boundary_changed");
        let brains = self.client.brains(&self.device).await?;
        let selected = brains
            .iter()
            .filter(|b| b.id.to_string() == selector || b.name == selector)
            .collect::<Vec<_>>();
        ensure!(
            selected.len() == 1 && selected[0].id == self.brain,
            "workspace_brain_changed"
        );
        let report = workspace::discover(&root).await?;
        ensure!(
            report.brain_selector == selector
                && std::path::Path::new(&report.refresh.workspace_root) == root,
            "workspace_boundary_changed"
        );
        let path = format!("/api/brains/{}/workspace", self.brain);
        let refresh: CheckoutRefreshResult = decode(
            self.client
                .send(
                    reqwest::Method::POST,
                    &format!("{path}/checkouts"),
                    Some(self.device.token),
                    Some(serde_json::to_value(report.refresh)?),
                )
                .await?,
        )
        .await?;
        let catalogue: WorkspaceCatalogue = decode(
            self.client
                .send(
                    reqwest::Method::GET,
                    &format!("{path}?workspace_id={}", refresh.workspace.id),
                    Some(self.device.token),
                    None,
                )
                .await?,
        )
        .await?;
        Ok(
            json!({"refresh":refresh,"catalogue":catalogue,"excluded_workspaces":report.excluded_workspaces}),
        )
    }
}
impl ServerHandler for Bridge {
    fn get_info(&self) -> ServerConfig {
        let mut config = ServerConfig::default();
        config.capabilities = ServerCapabilities::builder().enable_tools().build();
        config.instructions = self.instructions.clone();
        config
    }
    async fn list_tools(
        &self,
        page: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        self.check_plugin_destination()
            .await
            .map_err(|_| unavailable())?;
        if page.is_some_and(|p| p.cursor.is_some()) {
            return Err(ErrorData::invalid_params(
                "Use the single bounded catalogue page.",
                None,
            ));
        }
        let mut tools = self
            .peer
            .list_all_tools()
            .await
            .map_err(|_| unavailable())?;
        if self.workspace_root.is_some() && tools.iter().any(|t| t.name == "memory.contribute") {
            tools.push(refresh_tool());
        }
        Ok(ListToolsResult::with_all_items(tools))
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        self.check_plugin_destination()
            .await
            .map_err(|_| unavailable())?;
        if request.name == "workspace.refresh" {
            if request.arguments.as_ref().is_some_and(|v| !v.is_empty()) {
                return Err(ErrorData::invalid_params(
                    "Workspace refresh accepts no directory or Brain override.",
                    None,
                ));
            }
            return Ok(match tokio::time::timeout(Duration::from_secs(60),self.refresh()).await {
                Ok(Ok(value)) => bounded(CallToolResult::structured(value)),
                _ => CallToolResult::error(vec![ContentBlock::text(json!({"code":"workspace_refresh_unavailable",
                    "message":"Workspace refresh failed. Check pairing, writer access and the fixed workspace selector. Task scope is unchanged."}).to_string())]).into(),
            });
        }
        let result = tokio::time::timeout(Duration::from_secs(65), self.peer.call_tool(request))
            .await
            .map_err(|_| unavailable())?
            .map_err(|_| unavailable())?;
        Ok(bounded(result))
    }
}

pub async fn serve(
    client: Client,
    device: StoredDevice,
    brain: Uuid,
    workspace_root: Option<PathBuf>,
) -> Result<()> {
    serve_inner(client, device, brain, workspace_root, false).await
}

pub async fn serve_plugin(
    client: Client,
    device: StoredDevice,
    brain: Uuid,
    workspace_root: Option<PathBuf>,
) -> Result<()> {
    serve_inner(client, device, brain, workspace_root, true).await
}

async fn serve_inner(
    client: Client,
    device: StoredDevice,
    brain: Uuid,
    workspace_root: Option<PathBuf>,
    plugin_guard: bool,
) -> Result<()> {
    ensure!(
        client.endpoint == device.endpoint,
        "Saved pairing belongs to another endpoint."
    );
    client.whoami(&device).await?;
    let remote =
        recollect_mcp_runtime::agent_transport::connect(&client.endpoint, brain, device.token)
            .await?;
    let instructions = remote
        .peer()
        .peer_info()
        .and_then(|i| i.instructions.clone())
        .unwrap_or_default();
    let bridge = Bridge {
        instructions: Some(instructions),
        peer: remote.peer().clone(),
        client: Arc::new(client),
        device: Arc::new(device),
        brain,
        workspace_root,
        plugin_guard,
        destination_seen: Arc::new(std::sync::Mutex::new(None)),
    };
    // Reuse the same bounded, cancellation-safe pipe forwarding as the managed
    // process supervisor. The SDK remains responsible for JSON-RPC parsing.
    let (input_reader, input_writer) = tokio::io::duplex(65536);
    let (output_reader, output_writer) = tokio::io::duplex(65536);
    let inbound = recollect_mcp_runtime::supervisor::forward_lines(
        tokio::io::stdin(),
        input_writer,
        256 * 1024,
    );
    let outbound = recollect_mcp_runtime::supervisor::forward_lines(
        output_reader,
        tokio::io::stdout(),
        1024 * 1024,
    );
    let protocol = async {
        bridge
            .serve((input_reader, output_writer))
            .await?
            .waiting()
            .await?;
        Ok::<_, anyhow::Error>(())
    };
    tokio::pin!(inbound, outbound, protocol);
    let result = tokio::select! {
        result = &mut protocol => {
            result?;
            tokio::time::timeout(Duration::from_secs(5), &mut outbound).await
                .map_err(|_|anyhow!("Bridge output did not drain."))?.map_err(Into::into)
        },
        result = &mut inbound => {
            result?;
            tokio::time::timeout(Duration::from_secs(5), async {
                tokio::try_join!(&mut protocol, async { outbound.await.map_err(anyhow::Error::from) })?;
                Ok::<_,anyhow::Error>(())
            }).await.map_err(|_|anyhow!("Bridge did not finish after input closed."))?
        },
        result = &mut outbound => result.map_err(Into::into),
        _ = tokio::signal::ctrl_c() => Ok(()),
    };
    remote
        .cancel()
        .await
        .map_err(|_| anyhow!("Recollect bridge connection closed."))?;
    result
}

pub async fn destination(
    client: &Client,
    device: &StoredDevice,
    directory: &std::path::Path,
    brain: Option<Uuid>,
) -> Result<(Uuid, Option<PathBuf>)> {
    let brains = client.brains(device).await?;
    if let Some(brain) = brain {
        ensure!(
            brains.iter().any(|b| b.id == brain),
            "Brain is unavailable to this paired device."
        );
        let root = match workspace::selector(directory).await {
            Ok((root, selector)) => {
                let selected = brains
                    .iter()
                    .filter(|b| b.id.to_string() == selector || b.name == selector)
                    .collect::<Vec<_>>();
                (selected.len() == 1 && selected[0].id == brain).then_some(root)
            }
            _ => None,
        };
        return Ok((brain, root));
    }
    let (root, selector) = workspace::selector(directory).await?;
    let matches = brains
        .iter()
        .filter(|b| b.id.to_string() == selector || b.name == selector)
        .collect::<Vec<_>>();
    ensure!(
        matches.len() == 1,
        "Workspace selector must match exactly one accessible Brain. Use its UUID when names are ambiguous."
    );
    Ok((matches[0].id, Some(root)))
}
