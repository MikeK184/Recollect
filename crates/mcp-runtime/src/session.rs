use crate::{
    RESULT_LIMIT, Result, RuntimeError, WIRE_LIMIT, credentials::ResolvedCredentials,
    http::BoundedHttp,
};
use recollect_protocol::McpDefinitionManifest;
use rmcp::{
    ClientHandler, ClientLifecycleMode, ClientServiceExt, Peer, RoleClient,
    model::{
        CallToolRequest, CallToolRequestParams, CancelledNotificationParam, ClientRequest,
        ErrorCode, PaginatedRequestParams, ProtocolVersion, ServerResult, SubscriptionFilter,
    },
    service::{NotificationContext, PeerRequestOptions, RunningService, ServiceError},
    transport::{
        TokioChildProcess,
        common::client_side_sse::NeverRetry,
        streamable_http_client::{
            StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
        },
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct Handler(Arc<AtomicBool>);
impl ClientHandler for Handler {
    async fn on_tool_list_changed(&self, _: NotificationContext<RoleClient>) {
        self.0.store(true, Ordering::Release);
    }
}
struct ToolValidation {
    approved_input: jsonschema::Validator,
    live_input: jsonschema::Validator,
    outputs: Vec<jsonschema::Validator>,
}

/// The disposition records an observed MCP response, not a claim about all
/// external effects. Only the coordinator may persist authoritative transitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchOutcome {
    pub state: String,
    pub code: String,
    pub result: Option<Value>,
}
impl DispatchOutcome {
    fn unknown(code: &'static str) -> Self {
        Self {
            state: "unknown".into(),
            code: code.into(),
            result: None,
        }
    }
    fn failed(code: &'static str) -> Self {
        Self {
            state: "failed".into(),
            code: code.into(),
            result: None,
        }
    }
}

pub struct Connected {
    service: Mutex<RunningService<RoleClient, Handler>>,
    peer: Peer<RoleClient>,
    changed: Arc<AtomicBool>,
    credentials: Arc<ResolvedCredentials>,
    tools: BTreeMap<String, ToolValidation>,
    subscription: Option<tokio_util::task::AbortOnDropHandle<()>>,
    pub owned_pid: Option<u32>,
}
impl Connected {
    /// Callers persist instance ownership before this constructor. The supplied
    /// supervisor executable is the current trusted Recollect binary, not input.
    pub async fn open(
        definition: &McpDefinitionManifest,
        target: &str,
        configuration: &Value,
        credentials: Arc<ResolvedCredentials>,
        supervisor_executable: &Path,
    ) -> Result<Self> {
        if !credentials.valid() {
            return Err(RuntimeError("credential_expired"));
        }
        tokio::select! {
        _ = credentials.expired() => Err(RuntimeError("credential_expired")),
        connected = tokio::time::timeout(
            Duration::from_secs(30),
            Self::connect(
                definition,
                target,
                configuration,
                credentials.clone(),
                supervisor_executable,
            ),
        ) => connected.map_err(|_| RuntimeError("provider_startup_timeout"))?,
        }
    }
    async fn connect(
        definition: &McpDefinitionManifest,
        target: &str,
        configuration: &Value,
        credentials: Arc<ResolvedCredentials>,
        supervisor_executable: &Path,
    ) -> Result<Self> {
        let changed = Arc::new(AtomicBool::new(false));
        let handler = Handler(changed.clone());
        let lifecycle = ClientLifecycleMode::Auto {
            preferred_versions: vec![ProtocolVersion::V_2026_07_28],
            legacy_version: Some(ProtocolVersion::V_2025_11_25),
        };
        let mut owned_pid = None;
        let mut service = match definition.transport.as_str() {
            "stdio" => {
                let executable = definition
                    .command
                    .as_deref()
                    .filter(|c| Path::new(c).is_absolute())
                    .ok_or(RuntimeError("provider_command_invalid"))?;
                if !credentials.headers.is_empty() {
                    return Err(RuntimeError("credential_transport_mismatch"));
                }
                let mut command = tokio::process::Command::new(supervisor_executable);
                command
                    .arg("mcp-supervise")
                    .env_clear()
                    .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
                    .env(crate::supervisor::COMMAND, executable)
                    .env(crate::supervisor::PARENT, std::process::id().to_string())
                    .env(
                        crate::supervisor::ARGUMENTS,
                        serde_json::to_string(&definition.arguments)
                            .map_err(|_| RuntimeError("provider_configuration_invalid"))?,
                    )
                    .env("RECOLLECT_MCP_TARGET", target)
                    .env(
                        "RECOLLECT_MCP_CONFIGURATION",
                        serde_json::to_string(configuration)
                            .map_err(|_| RuntimeError("provider_configuration_invalid"))?,
                    )
                    .envs(&credentials.environment);
                let mut wrapped = process_wrap::tokio::CommandWrap::from(command);
                wrapped.wrap(process_wrap::tokio::KillOnDrop);
                #[cfg(unix)]
                wrapped.wrap(process_wrap::tokio::ProcessGroup::leader());
                #[cfg(not(unix))]
                return Err(RuntimeError("managed_stdio_platform_unsupported"));
                let (transport, _) = TokioChildProcess::builder(wrapped)
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|_| RuntimeError("provider_spawn_failed"))?;
                owned_pid = transport.id();
                handler.serve_with_lifecycle(transport, lifecycle).await
            }
            "streamable_http" => {
                if !credentials.environment.is_empty() {
                    return Err(RuntimeError("credential_transport_mismatch"));
                }
                let url = reqwest::Url::parse(target)
                    .map_err(|_| RuntimeError("provider_target_invalid"))?;
                if !(url.scheme() == "https"
                    || (url.scheme() == "http"
                        && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))))
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some()
                {
                    return Err(RuntimeError("provider_target_invalid"));
                }
                let http = reqwest::Client::builder()
                    .no_proxy()
                    .redirect(reqwest::redirect::Policy::none())
                    .retry(reqwest::retry::never())
                    .connect_timeout(Duration::from_secs(5))
                    .build()
                    .map_err(|_| RuntimeError("provider_http_unavailable"))?;
                let mut config = StreamableHttpClientTransportConfig::with_uri(target.to_owned())
                    .reinit_on_expired_session(false)
                    .max_sse_event_size(WIRE_LIMIT)
                    .max_concurrent_requests(5)
                    .control_request_timeout(Duration::from_secs(2));
                config.retry_config = Arc::new(NeverRetry::default());
                for (name, value) in &credentials.headers {
                    config.custom_headers.insert(
                        http::HeaderName::from_bytes(name.as_bytes())
                            .map_err(|_| RuntimeError("credential_destination_invalid"))?,
                        http::HeaderValue::from_str(value)
                            .map_err(|_| RuntimeError("credential_destination_invalid"))?,
                    );
                }
                let transport =
                    StreamableHttpClientTransport::with_client(BoundedHttp(http), config);
                handler.serve_with_lifecycle(transport, lifecycle).await
            }
            _ => return Err(RuntimeError("provider_transport_unsupported")),
        }
        .map_err(|_| RuntimeError("provider_initialization_failed"))?;
        // Modern subscriptions have a separate notification stream; the legacy
        // ClientHandler callback does not receive these events. Listen before the
        // snapshot, and treat stream loss/lag as stale metadata as well.
        let subscription = if service.peer_info().is_some_and(|info| {
            info.protocol_version >= ProtocolVersion::V_2026_07_28
                && info
                    .capabilities
                    .tools
                    .as_ref()
                    .is_some_and(|t| t.list_changed == Some(true))
        }) {
            let listen = service.listen_with_capacity(
                SubscriptionFilter::builder().tools_list_changed().build(),
                std::num::NonZeroUsize::new(8).expect("nonzero subscription bound"),
            );
            let mut subscription = tokio::time::timeout(Duration::from_secs(5), listen)
                .await
                .map_err(|_| RuntimeError("provider_subscription_timeout"))?
                .map_err(|_| RuntimeError("provider_subscription_failed"))?;
            if subscription.acknowledged().tools_list_changed != Some(true) {
                return Err(RuntimeError("provider_subscription_unsupported"));
            }
            let changed = changed.clone();
            Some(tokio_util::task::AbortOnDropHandle::new(tokio::spawn(
                async move {
                    let _ = subscription.next().await;
                    changed.store(true, Ordering::Release);
                    let _ =
                        tokio::time::timeout(Duration::from_secs(2), subscription.cancel()).await;
                },
            )))
        } else {
            None
        };
        let tools = match load_tools(&service, definition).await {
            Ok(tools) => tools,
            Err(error) => {
                let _ = service.close_with_timeout(Duration::from_secs(5)).await;
                return Err(error);
            }
        };
        Ok(Self {
            peer: service.peer().clone(),
            service: Mutex::new(service),
            changed,
            credentials,
            tools,
            subscription,
            owned_pid,
        })
    }
    pub fn reusable(&self) -> bool {
        self.credentials.reusable()
            && !self.changed.load(Ordering::Acquire)
            && !self.peer.is_transport_closed()
    }
    pub fn validate(&self, name: &str, arguments: &Value) -> Result<()> {
        if !self.credentials.valid() {
            return Err(RuntimeError("credential_expired"));
        }
        if !self.reusable() {
            return Err(RuntimeError("provider_instance_stale"));
        }
        let tool = self
            .tools
            .get(name)
            .ok_or(RuntimeError("provider_tool_unavailable"))?;
        if !arguments.is_object()
            || serde_json::to_vec(arguments).map_or(true, |b| b.len() > 32768)
            || !tree_supported(arguments, 0, false)
            || self.credentials.sanitize(arguments) != *arguments
            || !tool.approved_input.is_valid(arguments)
            || !tool.live_input.is_valid(arguments)
        {
            return Err(RuntimeError("tool_arguments_invalid"));
        }
        Ok(())
    }
    /// The coordinator must commit pre-send authorization before invoking this.
    /// Each invocation sends exactly one tools/call, with no MRTR or retry loop.
    pub async fn call(
        &self,
        name: &str,
        arguments: Value,
        timeout: Duration,
        cancel: CancellationToken,
    ) -> DispatchOutcome {
        if let Err(error) = self.validate(name, &arguments) {
            return DispatchOutcome::failed(error.0);
        }
        if cancel.is_cancelled() {
            return DispatchOutcome::failed("cancelled_before_send");
        }
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(arguments.as_object().expect("validated object").clone()),
        ));
        let options = PeerRequestOptions::with_timeout(timeout).with_max_total_timeout(timeout);
        let deadline = tokio::time::Instant::now() + timeout;
        let sent = tokio::select! {
            sent = self.peer.send_request_with_option(request, options) => sent,
            _ = self.credentials.expired() => return DispatchOutcome::unknown("credential_expired"),
            _ = tokio::time::sleep_until(deadline) => return DispatchOutcome::unknown("provider_send_timeout"),
            _ = cancel.cancelled() => return DispatchOutcome::unknown("cancellation_requested"),
        };
        let handle = match sent {
            Ok(handle) => handle,
            Err(_) => return DispatchOutcome::unknown("provider_send_uncertain"),
        };
        let id = handle.id.clone();
        let response = tokio::select! {
            response = handle.await_response() => response,
            _ = self.credentials.expired() => return DispatchOutcome::unknown("credential_expired"),
            _ = cancel.cancelled() => {
                let notification = CancelledNotificationParam::new(Some(id.clone()), Some("Managed call cancelled".into()));
                let _ = tokio::time::timeout(Duration::from_secs(2), self.peer.notify_cancelled(notification)).await;
                return DispatchOutcome::unknown("cancellation_requested");
            },
            // SDK timeout cancellation can itself await a blocked transport.
            // Bound the entire send/wait lifecycle independently of that path.
            _ = tokio::time::sleep_until(deadline) => {
                let notification = CancelledNotificationParam::new(Some(id.clone()), Some("Managed call timed out".into()));
                let _ = tokio::time::timeout(Duration::from_secs(2), self.peer.notify_cancelled(notification)).await;
                return DispatchOutcome::unknown("provider_call_timeout");
            }
        };
        let result = match response {
            Ok(ServerResult::CallToolResult(result)) => result,
            Ok(_) => return DispatchOutcome::unknown("provider_continuation_unsupported"),
            Err(ServiceError::McpError(error))
                if matches!(
                    error.code,
                    ErrorCode::INVALID_REQUEST
                        | ErrorCode::METHOD_NOT_FOUND
                        | ErrorCode::INVALID_PARAMS
                ) =>
            {
                return DispatchOutcome::failed("provider_protocol_rejected");
            }
            Err(_) => return DispatchOutcome::unknown("provider_completion_uncertain"),
        };
        let value = match serde_json::to_value(&result) {
            Ok(v) => v,
            Err(_) => return DispatchOutcome::unknown("provider_result_invalid"),
        };
        if serde_json::to_vec(&value).map_or(true, |v| v.len() > RESULT_LIMIT) {
            return DispatchOutcome::unknown("provider_result_too_large");
        }
        if value
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|blocks| {
                blocks
                    .iter()
                    .any(|b| b.get("type").and_then(Value::as_str) != Some("text"))
            })
        {
            return DispatchOutcome::unknown("provider_content_unsupported");
        }
        let error = result.is_error.unwrap_or(false);
        if !error
            && self.tools[name].outputs.iter().any(|schema| {
                result
                    .structured_content
                    .as_ref()
                    .is_none_or(|v| !schema.is_valid(v))
            })
        {
            return DispatchOutcome::unknown("provider_output_schema_invalid");
        }
        let safe = self.credentials.sanitize(&value);
        if serde_json::to_vec(&safe).map_or(true, |v| v.len() > RESULT_LIMIT) {
            return DispatchOutcome::unknown("provider_result_too_large");
        }
        DispatchOutcome {
            state: if error { "tool_error" } else { "succeeded" }.into(),
            code: "connector_response".into(),
            result: Some(safe),
        }
    }
    /// Runtime ownership/lease policy must prevent calling this with active work.
    pub async fn close(&self) -> Result<()> {
        if let Some(subscription) = &self.subscription {
            subscription.abort();
        }
        match self
            .service
            .lock()
            .await
            .close_with_timeout(Duration::from_secs(8))
            .await
        {
            Ok(Some(_)) => Ok(()),
            _ => Err(RuntimeError("provider_close_incomplete")),
        }
    }
}

fn tree_supported(value: &Value, depth: usize, schema: bool) -> bool {
    if depth > if schema { 24 } else { 16 } {
        return false;
    }
    match value {
        Value::Object(fields) => fields.iter().all(|(key, value)| {
            !(schema
                && (matches!(key.as_str(), "$id" | "x-mcp-header")
                    || (matches!(key.as_str(), "$ref" | "$dynamicRef")
                        && !value.as_str().is_some_and(|s| s.starts_with('#')))))
                && tree_supported(value, depth + 1, schema)
        }),
        Value::Array(items) => items.iter().all(|v| tree_supported(v, depth + 1, schema)),
        _ => true,
    }
}
fn validator(value: &Value) -> Result<jsonschema::Validator> {
    if !tree_supported(value, 0, true)
        || serde_json::to_vec(value).map_or(true, |v| v.len() > 32768)
    {
        return Err(RuntimeError("provider_schema_unsupported"));
    }
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .offline()
        .build(value)
        .map_err(|_| RuntimeError("provider_schema_invalid"))
}
async fn load_tools(
    service: &RunningService<RoleClient, Handler>,
    definition: &McpDefinitionManifest,
) -> Result<BTreeMap<String, ToolValidation>> {
    let mut live = BTreeMap::new();
    let mut cursors = HashSet::new();
    let mut cursor = None;
    let mut complete = false;
    for _ in 0..10 {
        let page = service
            .list_tools(cursor.map(|s| PaginatedRequestParams::default().with_cursor(Some(s))))
            .await
            .map_err(|_| RuntimeError("provider_catalogue_unavailable"))?;
        for tool in page.tools {
            if live.insert(tool.name.to_string(), tool).is_some() || live.len() > 100 {
                return Err(RuntimeError("provider_catalogue_invalid"));
            }
        }
        cursor = page.next_cursor;
        if let Some(value) = &cursor {
            if value.len() > 1024 || !cursors.insert(value.clone()) {
                return Err(RuntimeError("provider_catalogue_invalid"));
            }
        } else {
            complete = true;
            break;
        }
    }
    if !complete {
        return Err(RuntimeError("provider_catalogue_limit"));
    }
    let mut tools = BTreeMap::new();
    for approved in &definition.tools {
        let Some(live) = live.remove(&approved.name) else {
            continue;
        };
        let live_input = Value::Object((*live.input_schema).clone());
        if live_input.get("type").and_then(Value::as_str) != Some("object") {
            return Err(RuntimeError("provider_schema_invalid"));
        }
        let mut outputs = vec![];
        if let Some(schema) = &approved.output_schema {
            outputs.push(validator(schema)?);
        }
        if let Some(schema) = live.output_schema {
            outputs.push(validator(&Value::Object((*schema).clone()))?);
        }
        tools.insert(
            approved.name.clone(),
            ToolValidation {
                approved_input: validator(&approved.input_schema)?,
                live_input: validator(&live_input)?,
                outputs,
            },
        );
    }
    Ok(tools)
}
