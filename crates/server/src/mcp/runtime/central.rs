//! Central execution uses the same fenced protocol and SDK runtime as companions.
use super::{
    runner::{self, Runner},
    *,
};
use recollect_mcp_runtime::{
    CancellationToken, RuntimeError,
    credentials::CredentialResolver,
    executor::{Coordinator, Executor},
    outbox::Outbox,
};
use std::{path::PathBuf, time::Duration};

#[derive(Clone)]
pub struct CentralCoordinator {
    pub state: AppState,
    pub runner: Runner,
}
fn safe(error: Error) -> RuntimeError {
    RuntimeError(error.1)
}
impl Coordinator for CentralCoordinator {
    async fn receipt_removals(
        &self,
        input: McpReceiptCheck,
    ) -> recollect_mcp_runtime::Result<McpReceiptRemovals> {
        self.runner
            .receipt_removals(&self.state, input)
            .await
            .map_err(safe)
    }
    async fn heartbeat(&self) -> recollect_mcp_runtime::Result<McpRunnerLease> {
        runner::maintain(&self.state).await.map_err(safe)?;
        self.runner.heartbeat(&self.state).await.map_err(safe)
    }
    async fn claim(&self) -> recollect_mcp_runtime::Result<McpClaim> {
        self.runner.claim(&self.state).await.map_err(safe)
    }
    async fn credentials(&self, input: McpAttempt) -> recollect_mcp_runtime::Result<()> {
        self.runner
            .credentials(&self.state, &input)
            .await
            .map_err(safe)
    }
    async fn instance(&self, input: McpInstanceUpdate) -> recollect_mcp_runtime::Result<()> {
        self.runner
            .instance(&self.state, &input)
            .await
            .map_err(safe)
    }
    async fn start(&self, input: McpStart) -> recollect_mcp_runtime::Result<McpStartPermit> {
        self.runner.start(&self.state, &input).await.map_err(safe)
    }
    async fn complete(&self, input: McpCompletion) -> recollect_mcp_runtime::Result<()> {
        // An old receipt retains its original attempt after executor restart.
        Runner::central(input.attempt.epoch)
            .complete(&self.state, &input)
            .await
            .map_err(safe)
    }
    async fn defer(&self, input: McpAttempt) -> recollect_mcp_runtime::Result<()> {
        self.runner.defer(&self.state, &input).await.map_err(safe)
    }
}
async fn run_owned(state: AppState, stop: CancellationToken) -> recollect_mcp_runtime::Result<()> {
    runner::maintain(&state).await.map_err(safe)?;
    let directory = receipt_directory(&state.config);
    let outbox = Outbox::open(directory, format!("central:{}", state.config.public_origin)).await?;
    let credentials = CredentialResolver::new(
        std::env::var_os("RECOLLECT_MCP_CREDENTIALS_FILE").map(PathBuf::from),
    );
    let supervisor = std::env::current_exe().map_err(|_| RuntimeError("supervisor_unavailable"))?;
    let runner = Runner::central(Uuid::new_v4());
    let lease = runner.register(&state).await.map_err(safe)?;
    Executor::new(
        CentralCoordinator { state, runner },
        credentials,
        supervisor,
        outbox,
    )
    .run(lease, stop)
    .await
}

pub fn receipt_directory(config: &crate::config::Config) -> PathBuf {
    std::env::var_os("RECOLLECT_MCP_OUTBOX_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&config.artifact_dir).join(".mcp-receipts/central"))
}
pub async fn run(state: AppState, stop: CancellationToken) {
    while !stop.is_cancelled() {
        if let Err(error) = run_owned(state.clone(), stop.clone()).await {
            tracing::warn!(
                code = error.0,
                "MCP executor stopped; waiting before ownership recovery"
            );
        }
        tokio::select! {
            _=stop.cancelled()=>return,
            _=tokio::time::sleep(Duration::from_secs(5))=>{},
        }
    }
}
