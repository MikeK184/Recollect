//! Paired outbound MCP runner. Credentials and provider processes stay here.
use crate::{Client, StoredDevice};
use recollect_mcp_runtime::{
    CancellationToken, Result, RuntimeError,
    credentials::CredentialResolver,
    executor::{Coordinator, Executor},
    outbox::Outbox,
};
use recollect_protocol::*;
use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc, time::Duration};
use uuid::Uuid;

pub(crate) async fn request<T: DeserializeOwned>(
    client: &Client,
    token: Uuid,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<T> {
    let mut response = client
        .send(method, path, Some(token), body)
        .await
        .map_err(|_| RuntimeError("runner_service_unavailable"))?;
    let status = response.status();
    if response.content_length().is_some_and(|n| n > 1_048_576) {
        return Err(RuntimeError("runner_response_too_large"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| RuntimeError("runner_service_unavailable"))?
    {
        if bytes.len() + chunk.len() > 1_048_576 {
            return Err(RuntimeError("runner_response_too_large"));
        }
        bytes.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        let code = serde_json::from_slice::<ApiError>(&bytes)
            .ok()
            .map(|e| e.code);
        return Err(RuntimeError(match code.as_deref() {
            Some("mcp_runner_fenced" | "unauthorized") => "mcp_runner_fenced",
            Some("mcp_runner_already_active") => "mcp_runner_already_active",
            Some("mcp_replay_expired") => "mcp_replay_expired",
            Some("mcp_request_conflict") => "mcp_request_conflict",
            Some("mcp_use_required") => "mcp_use_required",
            Some("mcp_configuration_changed") => "mcp_configuration_changed",
            _ if status.is_server_error() => "runner_service_unavailable",
            _ => "mcp_request_rejected",
        }));
    }
    serde_json::from_slice(&bytes).map_err(|_| RuntimeError("runner_response_invalid"))
}
#[derive(Clone)]
pub struct LocalCoordinator {
    client: Arc<Client>,
    token: Uuid,
    epoch: Uuid,
    private_runner: Option<Uuid>,
}
impl LocalCoordinator {
    pub async fn register(
        client: Arc<Client>,
        device: &StoredDevice,
    ) -> Result<(Self, McpRunnerLease)> {
        Self::register_selected(client, device, None).await
    }
    pub async fn register_selected(
        client: Arc<Client>,
        device: &StoredDevice,
        private_runner: Option<Uuid>,
    ) -> Result<(Self, McpRunnerLease)> {
        if client.endpoint != device.endpoint {
            return Err(RuntimeError("runner_endpoint_mismatch"));
        }
        let lease: McpRunnerLease = request(
            &client,
            device.token,
            Method::POST,
            &path("register", private_runner),
            None,
        )
        .await?;
        if lease.runner_reference
            != private_runner
                .map(|id| format!("private:{id}"))
                .unwrap_or_else(|| format!("device:{}", device.device_id))
        {
            return Err(RuntimeError("runner_identity_mismatch"));
        }
        Ok((
            Self {
                client,
                token: device.token,
                epoch: lease.epoch,
                private_runner,
            },
            lease,
        ))
    }
    async fn post<T: DeserializeOwned>(&self, action: &str, input: impl Serialize) -> Result<T> {
        let body =
            serde_json::to_value(input).map_err(|_| RuntimeError("runner_request_invalid"))?;
        request(
            &self.client,
            self.token,
            Method::POST,
            &path(action, self.private_runner),
            Some(body),
        )
        .await
    }
}
fn path(action: &str, private_runner: Option<Uuid>) -> String {
    let base = format!("/api/mcp/runner/{action}");
    private_runner
        .map(|id| format!("{base}?private_runner={id}"))
        .unwrap_or(base)
}
impl Coordinator for LocalCoordinator {
    async fn receipt_removals(&self, input: McpReceiptCheck) -> Result<McpReceiptRemovals> {
        self.post("receipt-removals", input).await
    }
    async fn heartbeat(&self) -> Result<McpRunnerLease> {
        self.post("heartbeat", McpRunnerEpoch { epoch: self.epoch })
            .await
    }
    async fn claim(&self) -> Result<McpClaim> {
        self.post("claim", McpRunnerEpoch { epoch: self.epoch })
            .await
    }
    async fn credentials(&self, input: McpAttempt) -> Result<()> {
        self.post::<Value>("credentials", input).await.map(|_| ())
    }
    async fn start(&self, input: McpStart) -> Result<McpStartPermit> {
        self.post("start", input).await
    }
    async fn instance(&self, input: McpInstanceUpdate) -> Result<()> {
        self.post::<Value>("instance", input).await.map(|_| ())
    }
    async fn complete(&self, input: McpCompletion) -> Result<()> {
        self.post::<Value>("complete", input).await.map(|_| ())
    }
    async fn defer(&self, input: McpAttempt) -> Result<()> {
        self.post::<Value>("defer", input).await.map(|_| ())
    }
}
pub async fn run(
    client: Client,
    device: StoredDevice,
    directory: PathBuf,
    stop: CancellationToken,
) -> Result<()> {
    run_selected(client, device, directory, stop, None).await
}
pub async fn run_selected(
    client: Client,
    device: StoredDevice,
    directory: PathBuf,
    stop: CancellationToken,
    private_runner: Option<Uuid>,
) -> Result<()> {
    let client = Arc::new(client);
    let owner = format!(
        "{}#device:{}{}",
        client.endpoint,
        device.device_id,
        private_runner
            .map(|id| format!("#private:{id}"))
            .unwrap_or_default()
    );
    let supervisor = std::env::current_exe().map_err(|_| RuntimeError("supervisor_unavailable"))?;
    while !stop.is_cancelled() {
        let attempt = async {
            let outbox = Outbox::open(directory.clone(), owner.clone()).await?;
            let (coordinator, lease) =
                LocalCoordinator::register_selected(client.clone(), &device, private_runner)
                    .await?;
            let credentials = CredentialResolver::new(
                std::env::var_os("RECOLLECT_MCP_CREDENTIALS_FILE").map(PathBuf::from),
            );
            Executor::new(coordinator, credentials, supervisor.clone(), outbox)
                .run(lease, stop.clone())
                .await
        }
        .await;
        if stop.is_cancelled() {
            return attempt;
        }
        match attempt {
            Err(RuntimeError(
                "runner_service_unavailable"
                | "runner_control_timeout"
                | "mcp_runner_already_active",
            )) => {
                eprintln!(
                    "MCP runner is waiting for service availability or its previous lease to expire."
                );
            }
            result => return result,
        }
        tokio::select! {_=stop.cancelled()=>return Ok(()),_=tokio::time::sleep(Duration::from_secs(5))=>{}}
    }
    Ok(())
}
