//! Remote work is kept out of the host hook. Every pass applies the shared
//! privacy position before selecting payloads from the durable inbox again.
use crate::{
    Client, StoredDevice,
    capture::{CachedCaptureBinding, Inbox, InboxStatus, profile_root},
    decode, privacy,
};
use anyhow::{Result, ensure};
use chrono::Utc;
use recollect_protocol::{ApiError, Brain, CaptureReceipt, CaptureSettings};
use reqwest::Method;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct DrainReport {
    pub connected: bool,
    pub published: usize,
    pub denied: usize,
    pub failures: Vec<(Uuid, &'static str)>,
    pub inbox: InboxStatus,
}
fn failed(status: reqwest::StatusCode, code: Option<&str>) -> (&'static str, bool) {
    if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return ("server_unavailable", true);
    }
    match code {
        Some("capture_identity_conflict") => ("capture_identity_conflict", false),
        Some("capture_scope_unavailable") => ("capture_binding_missing", false),
        _ if matches!(
            status,
            reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN
        ) =>
        {
            ("capture_denied", false)
        }
        _ if status == reqwest::StatusCode::NOT_FOUND => ("capture_binding_missing", false),
        _ => ("capture_delivery_failed", false),
    }
}

pub async fn run_once(
    client: &Client,
    device: &StoredDevice,
    root: &Path,
    focus: Option<Uuid>,
) -> Result<DrainReport> {
    ensure!(
        device.endpoint == client.endpoint,
        "Capture belongs to another endpoint."
    );
    let mut inbox = Inbox::open(
        &profile_root(root, device.device_id),
        &client.endpoint,
        device.device_id,
    )?;
    inbox.expire(Utc::now())?;
    let mut saved = inbox.cached_bindings()?;
    saved.sort_by_key(|b| b.synchronized_at);
    let bindings: BTreeMap<_, _> = saved
        .iter()
        .map(|b| (b.binding.id, b.binding.brain_id))
        .collect();
    let ready = inbox.pending(Utc::now())?;
    let mut brains: BTreeSet<Uuid> = ready
        .iter()
        .filter_map(|e| bindings.get(&e.binding_id).copied())
        .collect();
    if let Some(brain) = focus {
        brains.insert(brain);
    }
    // Idle independent drain also refreshes cached permission for its next hook.
    if brains.is_empty() {
        for binding in &saved {
            brains.insert(binding.binding.brain_id);
            if brains.len() == 20 {
                break;
            }
        }
    }
    let mut report = DrainReport {
        connected: false,
        published: 0,
        denied: 0,
        failures: vec![],
        inbox: inbox.status()?,
    };
    let access: std::result::Result<Vec<Brain>, (&'static str, bool)> = async {
        let response = client
            .send(Method::GET, "/api/brains", Some(device.token), None)
            .await
            .map_err(|_| ("transport_unavailable", true))?;
        if !response.status().is_success() {
            return Err(failed(response.status(), None));
        }
        response
            .json()
            .await
            .map_err(|_| ("server_unavailable", true))
    }
    .await;
    let accessible = match access {
        Ok(brains) => brains,
        Err((code, retry)) => {
            for event in ready {
                inbox.failed_delivery(event.id, code, retry, Utc::now())?;
                report.denied += usize::from(!retry);
            }
            inbox.record_gap("capture_sync_unavailable")?;
            report
                .failures
                .extend(brains.into_iter().map(|b| (b, code)));
            report.inbox = inbox.status()?;
            return Ok(report);
        }
    };
    report.connected = true;
    let mut refreshed = BTreeSet::new();
    let report_brains = brains.clone();
    for brain in brains {
        let access = accessible.iter().find(|b| b.id == brain);
        let Some(access) = access else {
            report.failures.push((brain, "capture_denied"));
            continue;
        };
        // Synchronization also clears the inbox through the shared native
        // cleanup path before acknowledging the server's deletion position.
        if privacy::synchronize(client, device, brain, root)
            .await
            .is_err()
        {
            report.failures.push((brain, "capture_sync_unavailable"));
            continue;
        }
        let settings: Result<CaptureSettings> = async {
            decode(
                client
                    .send(
                        Method::GET,
                        &format!("/api/brains/{brain}/capture/policy"),
                        Some(device.token),
                        None,
                    )
                    .await?,
            )
            .await
        }
        .await;
        let settings = match settings {
            Ok(value) if value.brain_id == brain => value,
            _ => {
                report.failures.push((brain, "capture_sync_unavailable"));
                continue;
            }
        };
        let privacy = privacy::cached(root, &client.endpoint, device.device_id, brain)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Capture privacy state is missing after synchronization.")
            })?;
        for old in saved.iter().filter(|b| b.binding.brain_id == brain) {
            let fresh = CachedCaptureBinding {
                policy: settings.policy.clone(),
                retention: privacy.sync.policy.clone(),
                synchronized_at: Utc::now(),
                ..old.clone()
            };
            inbox.remember(&fresh)?;
        }
        if matches!(access.role.as_str(), "writer" | "admin")
            && !access.archived
            && settings.policy.enabled
        {
            inbox.retry_permissions(brain, Utc::now())?;
            refreshed.insert(brain);
        } else {
            report.failures.push((brain, "capture_denied"));
        }
    }
    // Discard the pre-sync batch, including its in-memory text, before upload.
    drop(ready);
    for input in inbox.pending(Utc::now())? {
        let brain = *bindings
            .get(&input.binding_id)
            .ok_or_else(|| anyhow::anyhow!("Capture binding is unavailable."))?;
        if !refreshed.contains(&brain) {
            let denied = report
                .failures
                .iter()
                .any(|(b, c)| *b == brain && *c == "capture_denied");
            inbox.failed_delivery(
                input.id,
                if denied {
                    "capture_denied"
                } else {
                    "transport_unavailable"
                },
                !denied,
                Utc::now(),
            )?;
            report.denied += usize::from(denied);
            continue;
        }
        let response = match client
            .send(
                Method::POST,
                &format!("/api/brains/{brain}/capture/events"),
                Some(device.token),
                Some(serde_json::to_value(&input)?),
            )
            .await
        {
            Ok(response) => response,
            Err(_) => {
                inbox.failed_delivery(input.id, "transport_unavailable", true, Utc::now())?;
                report.failures.push((brain, "transport_unavailable"));
                continue;
            }
        };
        if response.status().is_success() {
            match response.json::<CaptureReceipt>().await {
                Ok(receipt)
                    if receipt.binding_id == input.binding_id
                        && !receipt.event_id.is_nil()
                        && matches!(receipt.state.as_str(), "accepted" | "expired" | "removed") =>
                {
                    inbox.acknowledge_delivery(input.id, &receipt)?;
                    report.published += usize::from(receipt.state == "accepted");
                }
                _ => {
                    inbox.failed_delivery(input.id, "server_unavailable", true, Utc::now())?;
                    report.failures.push((brain, "capture_response_unreadable"));
                }
            }
        } else {
            let status = response.status();
            let error = response.json::<ApiError>().await.ok();
            let (code, retry) = failed(status, error.as_ref().map(|e| e.code.as_str()));
            inbox.failed_delivery(input.id, code, retry, Utc::now())?;
            report.denied += usize::from(!retry);
            report.failures.push((brain, code));
        }
    }
    report.failures.sort();
    report.failures.dedup();
    report.inbox = inbox.status()?;
    for brain in report_brains {
        if !accessible.iter().any(|b| b.id == brain) {
            continue;
        }
        let mut snapshot = inbox.brain_report(brain)?;
        snapshot.issue = report
            .failures
            .iter()
            .find(|(b, _)| *b == brain)
            .map(|(_, code)| (*code).into());
        let response = client
            .send(
                Method::POST,
                &format!("/api/brains/{brain}/capture/devices"),
                Some(device.token),
                Some(serde_json::to_value(snapshot)?),
            )
            .await;
        if response.map_or(true, |r| !r.status().is_success()) {
            report.failures.push((brain, "capture_sync_unavailable"));
        }
    }
    report.failures.sort();
    report.failures.dedup();
    Ok(report)
}
