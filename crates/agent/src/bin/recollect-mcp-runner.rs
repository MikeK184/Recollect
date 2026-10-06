use anyhow::{Result, anyhow, bail};
use recollect_agent::{Client, CredentialSlot, presentation};
use recollect_mcp_runtime::CancellationToken;
use std::path::PathBuf;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "mcp-supervise") {
        return Ok(recollect_mcp_runtime::supervisor::run().await?);
    }
    presentation::banner();
    // Only our static operational codes are logged. Ignore ambient logging
    // filters, which must not enable raw SDK/HTTP credential diagnostics.
    tracing_subscriber::fmt()
        .with_env_filter("off,recollect_mcp_runtime=warn")
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    let (private_runner, directory) = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [directory] => (None, PathBuf::from(directory)),
        ["--private", id, directory] => (
            Some(
                id.parse::<uuid::Uuid>()
                    .map_err(|_| anyhow!("Use the registered private runner UUID."))?,
            ),
            PathBuf::from(directory),
        ),
        _ => bail!(
            "Use recollect-agent mcp-runner OUTBOX_DIRECTORY or private-runner RUNNER_UUID OUTBOX_DIRECTORY."
        ),
    };
    let client =
        Client::new(&std::env::var("RECOLLECT_URL").unwrap_or("http://127.0.0.1:8787".into()))?;
    let profile = std::env::var("RECOLLECT_DEVICE_PROFILE").unwrap_or("default".into());
    let device = CredentialSlot::new(&client.endpoint, &profile)?
        .load()?
        .ok_or_else(|| {
            anyhow!("No paired device in this endpoint/profile. Run recollect-agent pair.")
        })?;
    client.whoami(&device).await?;
    let stop = CancellationToken::new();
    let status = presentation::status::start("MCP runner");
    let running =
        recollect_agent::mcp::run_selected(client, device, directory, stop.clone(), private_runner);
    tokio::pin!(running);
    let outcome = tokio::select! {
        result = &mut running => result,
        _ = tokio::signal::ctrl_c() => {
            stop.cancel();
            eprintln!("Draining active MCP calls before shutdown.");
            running.await
        },
    };
    match &outcome {
        Ok(()) => presentation::status::finish(status, true, ""),
        Err(error) => presentation::status::finish(status, false, &error.to_string()),
    }
    outcome?;
    Ok(())
}
