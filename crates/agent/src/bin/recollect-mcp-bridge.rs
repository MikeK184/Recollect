use anyhow::{Result, anyhow, ensure};
use recollect_agent::{Client, CredentialSlot, mcp_bridge, presentation};
use std::path::PathBuf;

fn configuration(
    host: &str,
    client: &Client,
    profile: &str,
    brain: uuid::Uuid,
    directory: &std::path::Path,
) -> Result<()> {
    let executable = std::env::current_exe()?
        .with_file_name(format!("recollect-agent{}", std::env::consts::EXE_SUFFIX));
    println!(
        "{}",
        serde_json::to_string_pretty(&recollect_agent::mcp_host::configuration(
            host,
            &executable,
            &client.endpoint,
            profile,
            brain,
            directory,
            None
        )?)?
    );
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // stdout is the MCP stdio protocol in serve mode; presentation appears
    // only when a human terminal owns it.
    presentation::banner_quiet();
    tracing_subscriber::fmt()
        .with_env_filter("off,recollect_mcp_runtime=warn")
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let host = if args.first().is_some_and(|s| s == "config") {
        Some(
            args.get(1)
                .ok_or_else(|| anyhow!(recollect_agent::mcp_host::USAGE))?
                .as_str(),
        )
    } else {
        None
    };
    let args = if host.is_some() {
        &args[2..]
    } else {
        &args[..]
    };
    ensure!(
        args.len().is_multiple_of(2),
        "Use mcp-serve [--brain UUID] [--directory PATH]."
    );
    let mut brain = None;
    let mut directory = None;
    let mut capture_setup = None;
    let mut capture_launch = None;
    for pair in args.chunks_exact(2) {
        match pair[0].as_str() {
            "--brain" if brain.is_none() => {
                brain = Some(
                    pair[1]
                        .parse::<uuid::Uuid>()
                        .map_err(|_| anyhow!("Use a Brain UUID."))?,
                )
            }
            "--directory" if directory.is_none() => directory = Some(PathBuf::from(&pair[1])),
            "--capture-setup" if capture_setup.is_none() => {
                capture_setup = Some(PathBuf::from(&pair[1]))
            }
            "--capture-launch" if capture_launch.is_none() => {
                capture_launch = Some(pair[1].parse::<uuid::Uuid>()?)
            }
            _ => {
                return Err(anyhow!(
                    "Use mcp-serve [--brain UUID] [--directory PATH] without repeated options."
                ));
            }
        }
    }
    let client =
        Client::new(&std::env::var("RECOLLECT_URL").unwrap_or("http://127.0.0.1:8787".into()))?;
    let profile = std::env::var("RECOLLECT_DEVICE_PROFILE").unwrap_or("default".into());
    if let Some(host) = host {
        // Remote plugin-only configuration needs no companion binary, no
        // bridge process, and no paired profile: the user token travels in
        // the host environment. Only the Brain UUID and endpoint matter.
        if matches!(host, "codex-remote" | "claude-remote" | "opencode-remote") {
            ensure!(
                capture_setup.is_none() && capture_launch.is_none(),
                "Standalone configuration cannot replace a managed capture launch."
            );
            let brain = brain.ok_or_else(|| anyhow!("Remote configuration needs --brain UUID."))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&recollect_agent::mcp_host::configuration_remote(
                    host,
                    &client.endpoint,
                    brain
                )?)?
            );
            return Ok(());
        }
    }
    let directory = tokio::fs::canonicalize(directory.unwrap_or(std::env::current_dir()?))
        .await
        .map_err(|_| anyhow!("Choose an existing workspace directory."))?;
    ensure!(directory.is_dir(), "Choose a workspace directory.");
    if let Some(host) = host {
        ensure!(
            capture_setup.is_none() && capture_launch.is_none(),
            "Standalone configuration cannot replace a managed capture launch."
        );
        if let Some(brain) = brain {
            // Rendering non-secret settings is not an authentication operation.
            // The actual bridge always authenticates and checks the fixed Brain.
            return configuration(host, &client, &profile, brain, &directory);
        }
    }
    let device = CredentialSlot::new(&client.endpoint, &profile)?
        .load()?
        .ok_or_else(|| anyhow!("Pair this device with recollect-agent pair first."))?;
    let (brain, root) = mcp_bridge::destination(&client, &device, &directory, brain).await?;
    if let Some(host) = host {
        return configuration(host, &client, &profile, brain, &directory);
    }
    let capture = match (capture_setup, capture_launch) {
        (Some(path), Some(id)) => Some(recollect_agent::capture_launch::Launch {
            setup: recollect_agent::capture_cli::read_setup(&path).await?,
            id,
        }),
        (None, None) => None,
        _ => {
            return Err(anyhow!(
                "A managed capture bridge requires its setup and launch ID together."
            ));
        }
    };
    let status = presentation::status::start_quiet("MCP bridge");
    let outcome = mcp_bridge::serve(client, device, brain, root, capture).await;
    match &outcome {
        Ok(()) => presentation::status::finish(status, true, ""),
        Err(error) => presentation::status::finish(status, false, &error.to_string()),
    }
    outcome
}
