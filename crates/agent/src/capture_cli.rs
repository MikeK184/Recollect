use crate::{
    Client, StoredDevice,
    capture::{Inbox, profile_root},
    capture_delivery, publication,
};
use anyhow::{Result, anyhow, ensure};
use chrono::Utc;
use recollect_protocol::CAPTURE_STDIN_BYTES;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const USAGE: &str = "capture setup HOST DIRECTORY [--brain UUID] [--task UUID] [--agent ID] [--output DIRECTORY]; capture hook|status|drain SETUP_FILE; capture run SETUP_FILE [-- HOST_ARGUMENTS...]";
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
pub async fn read_setup(path: &Path) -> Result<HookSetup> {
    let resolved = tokio::fs::canonicalize(path)
        .await
        .map_err(|_| anyhow!("capture_setup_missing"))?;
    ensure!(
        resolved.starts_with(publication::project_root()),
        "capture_setup_outside_recollect"
    );
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
    // Validate the selected evidence root through the existing ownership boundary.
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
    let root = publication::private_root(&setup.evidence_root).await?;
    ensure!(root == setup.evidence_root, "capture_setup_unreadable");
    Ok(setup)
}
fn secrets() -> Vec<String> {
    std::env::vars()
        .filter_map(|(key, value)| {
            let key = key.to_ascii_uppercase();
            (value.len() >= 4
                && ["KEY", "TOKEN", "PASSWORD", "SECRET"]
                    .iter()
                    .any(|s| key.contains(s)))
            .then_some(value)
        })
        .collect()
}
fn inbox(setup: &HookSetup) -> Result<Inbox> {
    let inbox = Inbox::open(
        &profile_root(&setup.evidence_root, setup.device_id),
        &setup.endpoint,
        setup.device_id,
    )?;
    ensure!(
        inbox.cached_binding(setup.binding_id)?.binding.brain_id == setup.brain_id,
        "capture_binding_missing"
    );
    Ok(inbox)
}
/// The process entry point handles this before network/client/keychain setup.
pub async fn hook(path: &Path) -> Result<()> {
    let setup = read_setup(path).await?;
    let mut inbox = inbox(&setup)?;
    let mut raw = Vec::new();
    if std::io::stdin()
        .take((CAPTURE_STDIN_BYTES + 1) as u64)
        .read_to_end(&mut raw)
        .is_err()
    {
        inbox.record_gap("capture_stdin_unavailable")?;
        return Err(anyhow!("capture_stdin_unavailable"));
    }
    let launch = std::env::var("RECOLLECT_CAPTURE_LAUNCH")
        .ok()
        .map(|v| v.parse::<Uuid>())
        .transpose()
        .map_err(|_| anyhow!("invalid_capture_launch"))?;
    inbox.capture_in_launch(setup.binding_id, launch, &raw, &secrets(), Utc::now())?;
    Ok(())
}
pub async fn status(path: &Path) -> Result<Value> {
    let setup = read_setup(path).await?;
    let mut inbox = inbox(&setup)?;
    inbox.expire(Utc::now())?;
    let saved = inbox.cached_binding(setup.binding_id)?;
    Ok(
        json!({"brain_id":setup.brain_id,"binding_id":setup.binding_id,"device_id":setup.device_id,
        "scope":saved.binding.operation.scope,"capture_enabled":saved.policy.enabled,
        "policy_synchronized_at":saved.synchronized_at,"policy_stale":Utc::now()-saved.synchronized_at>chrono::Duration::hours(24),
        "connection":"not_checked","inbox":inbox.status()?}),
    )
}
pub async fn run(client: &Client, device: &StoredDevice, args: &[String]) -> Result<Value> {
    if args
        .first()
        .is_some_and(|s| matches!(s.as_str(), "setup" | "bind"))
    {
        return crate::capture_setup::run(client, device, &args[1..]).await;
    }
    ensure!(
        args.len() >= 2 && matches!(args[0].as_str(), "drain" | "run"),
        "{USAGE}"
    );
    let setup = read_setup(Path::new(&args[1])).await?;
    ensure!(
        setup.endpoint == client.endpoint && setup.device_id == device.device_id,
        "Use this capture setup's original endpoint and paired device profile."
    );
    if args[0] == "run" {
        let rest = &args[2..];
        ensure!(rest.is_empty() || rest[0] == "--", "{USAGE}");
        return crate::capture_run::run(
            client,
            device,
            Path::new(&args[1]),
            &setup,
            if rest.is_empty() { rest } else { &rest[1..] },
        )
        .await;
    }
    ensure!(args.len() == 2, "{USAGE}");
    Ok(serde_json::to_value(
        capture_delivery::run_once(client, device, &setup.evidence_root, Some(setup.brain_id))
            .await?,
    )?)
}
