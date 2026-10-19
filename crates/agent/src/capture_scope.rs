//! Capture scope shared by the plugin runtime: the persisted per-session setup
//! record, the launch pointer that selects a session's current capture binding,
//! and the probe of the installed host's version. The managed-launch commands
//! that once produced these records were retired in favour of the plugin.
use crate::{
    Client, StoredDevice,
    capture::{CachedCaptureBinding, Inbox, profile_root},
    capture_delivery, decode, privacy, publication,
};
use anyhow::{Result, anyhow, ensure};
use chrono::Utc;
use recollect_protocol::{CaptureBinding, CaptureSettings, OperationBinding, capture_identity};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookSetup {
    pub endpoint: String,
    pub device_id: Uuid,
    pub brain_id: Uuid,
    pub binding_id: Uuid,
    pub evidence_root: PathBuf,
    pub device_profile: String,
    pub host: String,
    pub working_directory: PathBuf,
    pub task_id: Uuid,
    pub created_task: bool,
    #[serde(default)]
    pub plugin_root: Option<PathBuf>,
}

/// Read a retired managed-launch setup file, only so its queued evidence can be
/// drained before migrating to the plugin.
pub async fn read_setup(path: &Path) -> Result<HookSetup> {
    let resolved = tokio::fs::canonicalize(path)
        .await
        .map_err(|_| anyhow!("capture_setup_missing"))?;
    let setup: HookSetup = serde_json::from_slice(&publication::artifact(&resolved, 8192).await?)
        .map_err(|_| anyhow!("capture_setup_unreadable"))?;
    ensure!(
        !setup.device_id.is_nil() && !setup.brain_id.is_nil() && !setup.binding_id.is_nil(),
        "capture_setup_unreadable"
    );
    ensure!(
        matches!(setup.host.as_str(), "codex" | "claude_code")
            && !setup.task_id.is_nil()
            && !setup.device_profile.is_empty()
            && setup.device_profile.len() <= 80
            && !setup.device_profile.chars().any(char::is_control),
        "capture_setup_unreadable"
    );
    let endpoint =
        reqwest::Url::parse(&setup.endpoint).map_err(|_| anyhow!("capture_setup_unreadable"))?;
    ensure!(
        endpoint.username().is_empty()
            && endpoint.password().is_none()
            && endpoint.query().is_none()
            && endpoint.fragment().is_none()
            && (endpoint.scheme() == "https"
                || (endpoint.scheme() == "http"
                    && matches!(
                        endpoint.host_str(),
                        Some("127.0.0.1" | "localhost" | "[::1]")
                    ))),
        "capture_setup_unreadable"
    );
    // Validate the selected evidence root through the existing ownership boundary.
    let root = publication::private_root(&setup.evidence_root).await?;
    ensure!(root == setup.evidence_root, "capture_setup_unreadable");
    Ok(setup)
}
/// Upload the evidence a retired managed launch left queued.
pub async fn drain(client: &Client, device: &StoredDevice, setup: &HookSetup) -> Result<Value> {
    ensure!(
        setup.endpoint == client.endpoint && setup.device_id == device.device_id,
        "Use this capture setup's original endpoint and paired device profile."
    );
    Ok(serde_json::to_value(
        capture_delivery::run_once(client, device, &setup.evidence_root, Some(setup.brain_id))
            .await?,
    )?)
}

#[derive(Clone)]
pub struct Launch {
    pub setup: HookSetup,
    pub id: Uuid,
}
impl Launch {
    pub fn inbox(&self) -> Result<Inbox> {
        Inbox::open(
            &profile_root(&self.setup.evidence_root, self.setup.device_id),
            &self.setup.endpoint,
            self.setup.device_id,
        )
    }
    pub fn validate(&self, client: &Client, device: &StoredDevice, brain: Uuid) -> Result<()> {
        ensure!(
            self.setup.endpoint == client.endpoint
                && self.setup.device_id == device.device_id
                && self.setup.brain_id == brain,
            "capture_launch_destination_mismatch"
        );
        let inbox = self.inbox()?;
        let original = inbox.cached_binding(self.setup.binding_id)?;
        ensure!(
            original.binding.brain_id == brain
                && original.binding.device_id == device.device_id
                && original.binding.operation.task_id == self.setup.task_id
                && original.binding.host == self.setup.host,
            "capture_launch_destination_mismatch"
        );
        inbox.launch_default(self.id, self.setup.binding_id)?;
        Ok(())
    }
    pub fn current(&self) -> Result<Uuid> {
        self.inbox()?.launch_default(self.id, self.setup.binding_id)
    }
    pub fn gap(&self) {
        if let Ok(mut inbox) = self.inbox() {
            let _ = inbox.record_gap("capture_scope_refresh_failed");
        }
    }
    pub async fn refresh(
        &self,
        client: &Client,
        device: &StoredDevice,
        expected: Uuid,
        scope: Uuid,
    ) -> Result<Uuid> {
        self.validate(client, device, self.setup.brain_id)?;
        let saved = self.inbox()?.cached_binding(self.setup.binding_id)?;
        let base = format!("/api/brains/{}", self.setup.brain_id);
        // Current policy/retention precedes binding creation. An unavailable
        // refresh leaves the prior launch pointer and an explicit coverage gap.
        privacy::synchronize(
            client,
            device,
            self.setup.brain_id,
            &self.setup.evidence_root,
        )
        .await?;
        let settings: CaptureSettings = decode(
            client
                .send(
                    Method::GET,
                    &format!("{base}/capture/policy"),
                    Some(device.token),
                    None,
                )
                .await?,
        )
        .await?;
        let operation: OperationBinding = decode(
            client
                .send(
                    Method::POST,
                    &format!("{base}/workspace/tasks/{}/operations", self.setup.task_id),
                    Some(device.token),
                    Some(json!({"kind":"capture", "expected_scope":scope})),
                )
                .await?,
        )
        .await?;
        let binding: CaptureBinding = decode(
            client
                .send(
                    Method::POST,
                    &format!("{base}/capture/bindings"),
                    Some(device.token),
                    Some(json!({"id":Uuid::new_v4(),"operation_id":operation.id,
                "host":self.setup.host,"host_version":saved.binding.host_version})),
                )
                .await?,
        )
        .await?;
        ensure!(
            binding.operation.scope.id == scope
                && binding.operation.id == operation.id
                && binding.operation.task_id == self.setup.task_id,
            "capture_scope_mismatch"
        );
        let privacy = privacy::cached(
            &self.setup.evidence_root,
            &client.endpoint,
            device.device_id,
            self.setup.brain_id,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("capture_privacy_unavailable"))?;
        let id = binding.id;
        let mut inbox = self.inbox()?;
        inbox.remember(&CachedCaptureBinding {
            binding,
            policy: settings.policy,
            retention: privacy.sync.policy,
            synchronized_at: Utc::now(),
            agent_id: saved.agent_id,
        })?;
        inbox.advance_launch(self.id, self.setup.binding_id, expected, id)?;
        Ok(id)
    }
}

/// Version of the installed host binary, as recorded on its capture binding.
pub(crate) async fn host_version(host: &str) -> Result<String> {
    let mut child = Command::new(if host == "codex" { "codex" } else { "claude" })
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| anyhow!("Install the selected host before capture setup."))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Cannot inspect host version."))?;
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut bytes = vec![];
        stdout.take(4097).read_to_end(&mut bytes).await?;
        ensure!(
            bytes.len() <= 4096 && child.wait().await?.success(),
            "Cannot inspect host version."
        );
        let text = std::str::from_utf8(&bytes).map_err(|_| anyhow!("Unreadable host version."))?;
        let version = text
            .split_whitespace()
            .find(|s| s.starts_with(|c: char| c.is_ascii_digit()) && capture_identity(s))
            .ok_or_else(|| anyhow!("Unrecognized host version."))?;
        Ok(version.to_owned())
    })
    .await
    .map_err(|_| anyhow!("Host version check timed out."))?
}
