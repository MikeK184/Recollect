//! Terminal host lifetime and background upload are independent of quick hooks.
use crate::{
    Client, StoredDevice, capture_cli::HookSetup, capture_delivery, capture_setup, publication,
};
use anyhow::{Result, anyhow, ensure};
use serde_json::{Value, json};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::process::Command;

pub async fn run(
    client: &Client,
    device: &StoredDevice,
    setup_path: &Path,
    setup: &HookSetup,
    arguments: &[String],
) -> Result<Value> {
    ensure!(
        setup.endpoint == client.endpoint && setup.device_id == device.device_id,
        "Use this capture setup's original endpoint and paired device profile."
    );
    if setup.host == "codex" {
        capture_setup::reject_conflicting_mcp_server(&setup.working_directory).await?;
    }
    let installed = capture_setup::version(&setup.host).await?;
    let (setup_path, setup) = capture_setup::refresh_version(
        client,
        device,
        setup_path,
        setup,
        &installed,
        &std::env::current_exe()?,
    )
    .await?;
    let config = capture_setup::hooks(&setup.host, &std::env::current_exe()?, &setup_path)?;
    let hooks_file = setup_path
        .parent()
        .ok_or_else(|| anyhow!("Invalid capture setup path."))?
        .join("hooks.json");
    let saved: Value = serde_json::from_slice(&publication::artifact(&hooks_file, 32768).await?)?;
    ensure!(
        saved == config,
        "Generated hook settings changed. Create a new capture setup."
    );
    let mut host_arguments = capture_setup::host_arguments(&setup.host, &hooks_file, &config)?;
    let launch_id = uuid::Uuid::new_v4();
    let launch = crate::capture_launch::Launch {
        setup: setup.clone(),
        id: launch_id,
    };
    launch.inbox()?.start_launch(launch_id, setup.binding_id)?;
    let mcp = crate::mcp_host::configuration(
        &setup.host,
        &std::env::current_exe()?,
        &client.endpoint,
        &setup.device_profile,
        setup.brain_id,
        &setup.working_directory,
        Some((&setup_path, launch_id)),
    )?;
    host_arguments.extend(serde_json::from_value::<Vec<String>>(
        mcp["host_arguments"].clone(),
    )?);
    let host = if setup.host == "codex" {
        "codex"
    } else {
        "claude"
    };
    eprintln!(
        "Recollect capture: {}/brains/{} (original setup scope).",
        client.endpoint, setup.brain_id
    );
    if setup.host == "codex" {
        let plugin = setup
            .plugin_root
            .as_ref()
            .ok_or_else(|| anyhow!("Create a capture setup with the native Codex plugin."))?;
        eprintln!("Registering Recollect capture through Codex's plugin installer.");
        capture_setup::register_codex_plugin(plugin).await?;
        eprintln!(
            "Codex requires initial hook trust through /hooks; check Brain activity for publication."
        );
    }
    if !matches!(tokio::time::timeout(Duration::from_secs(8),capture_delivery::run_once(client,device,
        &setup.evidence_root,Some(setup.brain_id))).await,Ok(Ok(report)) if report.connected)
    {
        eprintln!(
            "Recollect could not refresh capture permission. The hook will use only its unexpired cached policy."
        );
    }
    let mut command = Command::new(host);
    let (subcommand, arguments) = if arguments
        .first()
        .is_some_and(|s| matches!(s.as_str(), "exec" | "resume" | "fork" | "review"))
    {
        (Some(&arguments[0]), &arguments[1..])
    } else {
        (None, arguments)
    };
    if let Some(subcommand) = subcommand {
        command.arg(subcommand);
    }
    command
        .args(host_arguments)
        .args(arguments)
        .env("RECOLLECT_CAPTURE_SETUP", &setup_path)
        .env("RECOLLECT_CAPTURE_LAUNCH", launch_id.to_string())
        .env_remove("VAULT_TOKEN")
        .current_dir(&setup.working_directory)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let mut report = supervise(client, device, &setup, command).await?;
    report["setup_file"] = json!(setup_path);
    report["launch_id"] = json!(launch_id);
    Ok(report)
}

/// Own the selected host's process while the companion drains independently.
pub async fn supervise(
    client: &Client,
    device: &StoredDevice,
    setup: &HookSetup,
    mut command: Command,
) -> Result<Value> {
    let mut child = command
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| anyhow!("The configured host could not start."))?;
    // Do not cancel a partially completed upload on each timer/terminal interrupt.
    // Lost replies on final termination remain recoverable through stable IDs.
    let worker = async {
        let mut last_failure = false;
        loop {
            let failure = capture_delivery::run_once(
                client,
                device,
                &setup.evidence_root,
                Some(setup.brain_id),
            )
            .await
            .map_or(true, |r| !r.failures.is_empty());
            if failure && !last_failure {
                eprintln!(
                    "Recollect capture upload is incomplete; queued events remain in the companion inbox."
                );
            }
            last_failure = failure;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    };
    let mut worker = Box::pin(worker);
    #[cfg(unix)]
    let mut termination =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let status = loop {
        tokio::select! {
            status = child.wait() => break status?,
            _ = &mut worker => unreachable!(),
            // The host shares this foreground terminal and receives SIGINT itself.
            // Keep its uploader alive when Ctrl-C merely interrupts a host turn.
            signal = tokio::signal::ctrl_c() => { signal?; },
            _ = async {
                #[cfg(unix)] { termination.recv().await; }
                #[cfg(not(unix))] { std::future::pending::<()>().await; }
            } => {
                child.start_kill()?;
                break child.wait().await?;
            }
        }
    };
    // Stop borrowing the worker's in-flight inbox before the final drain.
    drop(worker);
    let drain = tokio::time::timeout(
        Duration::from_secs(8),
        capture_delivery::run_once(client, device, &setup.evidence_root, Some(setup.brain_id)),
    )
    .await;
    let drain = match drain {
        Ok(Ok(report)) => serde_json::to_value(report)?,
        _ => json!({"state":"incomplete","code":"capture_sync_unavailable"}),
    };
    #[cfg(unix)]
    let exit_code = {
        use std::os::unix::process::ExitStatusExt;
        status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(1))
    };
    #[cfg(not(unix))]
    let exit_code = status.code().unwrap_or(1);
    Ok(
        json!({"host":setup.host,"exit_code":exit_code,"binding_id":setup.binding_id,
        "brain_url":format!("{}/brains/{}",client.endpoint,setup.brain_id),"drain":drain}),
    )
}
