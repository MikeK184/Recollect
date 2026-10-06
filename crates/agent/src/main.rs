use anyhow::{Result, anyhow, bail};
use recollect_agent::{Client, CredentialSlot, presentation};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let remaining: Vec<String> = args.collect();
    if command == "mcp-serve" {
        return mcp_bridge(&remaining);
    }
    if command == "mcp-config" {
        let mut args = vec!["config".into()];
        args.extend(remaining);
        return mcp_bridge(&args);
    }
    if command == "mcp-runner" {
        return mcp_runner(&remaining, false);
    }
    if command == "private-runner" {
        return mcp_runner(&remaining, true);
    }
    if command == "capture" && remaining.first().is_some_and(|s| s == "hook") {
        let result = match remaining.as_slice() {
            [_, path] => recollect_agent::capture_cli::hook(std::path::Path::new(path)).await,
            [_] => match std::env::var_os("RECOLLECT_CAPTURE_SETUP") {
                Some(path) => recollect_agent::capture_cli::hook(std::path::Path::new(&path)).await,
                None => return Ok(()),
            },
            _ => Err(anyhow!("invalid_capture_command")),
        };
        if result.is_err() {
            eprintln!("Recollect capture gap. Check the companion's capture status.");
        }
        return Ok(());
    }
    // The capture hook is a hot machine path that must print nothing, and the
    // helper commands above exec into binaries that present themselves. Every
    // other command gets the banner only in a human terminal: many subcommands
    // answer with JSON on stdout that automation parses directly.
    presentation::banner_quiet();
    if command == "capture" && remaining.first().is_some_and(|s| s == "status") {
        if remaining.len() != 2 {
            bail!("{}", recollect_agent::capture_cli::USAGE);
        }
        println!(
            "{}",
            serde_json::to_string_pretty(
                &recollect_agent::capture_cli::status(std::path::Path::new(&remaining[1])).await?
            )?
        );
        return Ok(());
    }
    if command == "workspace" && remaining.first().is_some_and(|v| v == "discover") {
        if remaining.len() > 2 {
            bail!("{}", recollect_agent::workspace_cli::USAGE);
        }
        let directory = std::path::Path::new(remaining.get(1).map(String::as_str).unwrap_or("."));
        println!(
            "{}",
            serde_json::to_string_pretty(&recollect_agent::workspace::discover(directory).await?)?
        );
        return Ok(());
    }
    let saved_capture = if command == "capture"
        && remaining
            .first()
            .is_some_and(|s| matches!(s.as_str(), "drain" | "run"))
        && remaining.len() >= 2
    {
        Some(recollect_agent::capture_cli::read_setup(std::path::Path::new(&remaining[1])).await?)
    } else {
        None
    };
    let client = Client::new(
        &saved_capture
            .as_ref()
            .map(|s| s.endpoint.clone())
            .unwrap_or_else(|| {
                std::env::var("RECOLLECT_URL").unwrap_or("http://127.0.0.1:8787".into())
            }),
    )?;
    if command == "health" {
        client.health().await?;
        println!("Recollect is ready");
        return Ok(());
    }
    if !matches!(
        command.as_str(),
        "pair"
            | "whoami"
            | "brains"
            | "unpair"
            | "forget"
            | "workspace"
            | "scope"
            | "repository"
            | "capture"
            | "mcp"
    ) {
        bail!(
            "Usage: recollect-agent health | pair [device name] | whoami | brains | unpair | forget. {} {} {} {} Set RECOLLECT_URL and optionally RECOLLECT_DEVICE_PROFILE.",
            recollect_agent::workspace_cli::USAGE,
            recollect_agent::capture_cli::USAGE,
            recollect_agent::mcp_cli::USAGE,
            recollect_agent::mcp_host::USAGE
        );
    }
    let profile = saved_capture
        .map(|s| s.device_profile)
        .unwrap_or_else(|| std::env::var("RECOLLECT_DEVICE_PROFILE").unwrap_or("default".into()));
    let slot = CredentialSlot::new(&client.endpoint, &profile)?;
    if command == "pair" {
        let name = remaining.join(" ");
        let id = client
            .pair(
                &slot,
                if name.is_empty() {
                    "Local companion".into()
                } else {
                    name
                },
            )
            .await?;
        println!("Paired device {id}. Credential saved in the OS store.");
        return Ok(());
    }
    if command == "forget" {
        slot.forget()?;
        println!(
            "Local credential removed. Revoke any remaining server record in Recollect Devices."
        );
        return Ok(());
    }
    let device = slot.load()?.ok_or_else(|| {
        anyhow!("No paired device in this endpoint/profile. Run recollect-agent pair.")
    })?;
    match command.as_str() {
        "mcp" => println!(
            "{}",
            serde_json::to_string_pretty(
                &recollect_agent::mcp_cli::run(&client, &device, &remaining).await?
            )?
        ),
        "capture" => {
            let status = presentation::status::start("Capture run");
            let report = match recollect_agent::capture_cli::run(&client, &device, &remaining).await
            {
                Ok(report) => {
                    presentation::status::finish(status, true, "");
                    report
                }
                Err(error) => {
                    presentation::status::finish(status, false, &error.to_string());
                    return Err(error);
                }
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
            if let Some(code) = report["exit_code"].as_i64().filter(|code| *code != 0) {
                std::process::exit(i32::try_from(code).unwrap_or(1));
            }
        }
        "repository" => println!(
            "{}",
            serde_json::to_string_pretty(
                &recollect_agent::publication_cli::run(&client, &device, &remaining).await?
            )?
        ),
        "workspace" | "scope" => println!(
            "{}",
            serde_json::to_string_pretty(
                &recollect_agent::workspace_cli::run(&client, &device, &command, &remaining)
                    .await?
            )?
        ),
        "whoami" => {
            let identity = client.whoami(&device).await?;
            println!(
                "{}",
                serde_json::json!({"account_id":identity.user.id,"username":identity.user.username,"device_id":identity.device_id})
            );
        }
        "brains" => println!("{}", serde_json::to_string(&client.brains(&device).await?)?),
        "unpair" => {
            client.revoke(&device).await?;
            slot.forget()?;
            println!("Device revoked and local credential removed.");
        }
        _ => unreachable!(),
    }
    Ok(())
}

// Keep the SDK, schema engine and process supervisor out of the per-event hook
// executable. Exec preserves the runner's PID, terminal and shutdown signals.
fn mcp_bridge(args: &[String]) -> Result<()> {
    let helper = std::env::current_exe()?.with_file_name(format!(
        "recollect-mcp-bridge{}",
        std::env::consts::EXE_SUFFIX
    ));
    let mut command = std::process::Command::new(helper);
    command.args(args).env_remove("VAULT_TOKEN");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let _ = command.exec();
        bail!("Cannot start the MCP bridge. Install recollect-mcp-bridge beside recollect-agent.");
    }
    #[cfg(not(unix))]
    {
        let status = command
            .status()
            .map_err(|_| anyhow!("Cannot start the MCP bridge."))?;
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn mcp_runner(args: &[String], private: bool) -> Result<()> {
    if args.len() != if private { 2 } else { 1 } {
        bail!("{}", recollect_agent::mcp_cli::USAGE);
    }
    let helper = std::env::current_exe()?.with_file_name(format!(
        "recollect-mcp-runner{}",
        std::env::consts::EXE_SUFFIX
    ));
    let mut command = std::process::Command::new(helper);
    command.env_remove("VAULT_TOKEN");
    if private {
        args[0]
            .parse::<uuid::Uuid>()
            .map_err(|_| anyhow!("Use the registered private runner UUID."))?;
        command.arg("--private");
    }
    command.args(args);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let _ = command.exec();
        bail!("Cannot start MCP runner. Install recollect-mcp-runner beside recollect-agent.");
    }
    #[cfg(not(unix))]
    {
        let status = command.status().map_err(|_| {
            anyhow!("Cannot start MCP runner. Install recollect-mcp-runner beside recollect-agent.")
        })?;
        std::process::exit(status.code().unwrap_or(1));
    }
}
