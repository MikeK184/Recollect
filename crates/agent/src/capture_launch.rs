//! Managed host launches may advance future capture defaults without relabeling
//! already captured turns. All remote bindings still use the canonical API.
use crate::{
    Client, StoredDevice,
    capture::{CachedCaptureBinding, Inbox, profile_root},
    capture_cli::HookSetup,
    decode, privacy,
};
use anyhow::{Result, ensure};
use chrono::Utc;
use recollect_protocol::{CaptureBinding, CaptureSettings, OperationBinding};
use reqwest::Method;
use serde_json::json;
use uuid::Uuid;

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
