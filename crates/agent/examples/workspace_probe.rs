//! Opt-in integration probe for an explicitly supplied external workspace.
//! The JSON file lists already discovered checkout paths; no selector is written.
use anyhow::{Result, anyhow, bail, ensure};
use recollect_agent::{Client, CredentialSlot, workspace::observe_checkout};
use recollect_protocol::CheckoutRefresh;
use serde_json::{Value, json};
use std::path::Path;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3,
        "Use observe ROOT PATHS_JSON or publish BRAIN REFRESH_JSON"
    );
    let input = tokio::fs::read(&args[2]).await?;
    ensure!(input.len() <= 2 * 1024 * 1024, "Probe input exceeds 2 MiB");
    let output = match args[0].as_str() {
        "observe" => {
            let root = tokio::fs::canonicalize(&args[1]).await?;
            ensure!(root.is_dir(), "Select an existing workspace directory");
            let paths: Vec<String> = serde_json::from_slice(&input)?;
            ensure!(
                paths.len() <= 200,
                "Probe accepts at most 200 checkout paths"
            );
            let mut checkouts = Vec::new();
            for path in paths {
                let path = tokio::fs::canonicalize(Path::new(&path)).await?;
                ensure!(
                    path.starts_with(&root),
                    "Checkout is outside the selected root"
                );
                ensure!(
                    tokio::fs::symlink_metadata(path.join(".git")).await.is_ok(),
                    "Confirm each checkout's Git marker before observing it"
                );
                checkouts.push(observe_checkout(&path).await);
            }
            serde_json::to_value(CheckoutRefresh {
                workspace_root: root.to_string_lossy().into_owned(),
                // An explicit list cannot establish that every checkout was scanned.
                complete: false,
                notes: vec!["Explicit metadata-only integration probe; no selector-based discovery or source publication.".into()],
                checkouts,
            })?
        }
        "publish" => {
            let brain: Uuid = args[1].parse()?;
            let refresh: CheckoutRefresh = serde_json::from_slice(&input)?;
            let client = Client::new(&std::env::var("RECOLLECT_URL")?)?;
            let profile = std::env::var("RECOLLECT_DEVICE_PROFILE")?;
            ensure!(
                !profile.is_empty() && profile != "default",
                "Use a dedicated probe profile"
            );
            let device = CredentialSlot::new(&client.endpoint, &profile)?
                .load()?
                .ok_or_else(|| anyhow!("Pair the probe profile first"))?;
            client.whoami(&device).await?;
            let response = client
                .send(
                    reqwest::Method::POST,
                    &format!("/api/brains/{brain}/workspace/checkouts"),
                    Some(device.token),
                    Some(serde_json::to_value(refresh)?),
                )
                .await?;
            ensure!(
                response.status().is_success(),
                "Refresh failed with HTTP {}",
                response.status()
            );
            let result: Value = response.json().await?;
            json!({"refresh": result})
        }
        _ => bail!("Use observe ROOT PATHS_JSON or publish BRAIN REFRESH_JSON"),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
