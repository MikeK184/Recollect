//! Secret-free settings for one fixed native bridge. Nothing writes host profiles.
use anyhow::{Result, anyhow, ensure};
use serde_json::{Value, json};
use std::path::Path;
use uuid::Uuid;

pub const USAGE: &str = "mcp-serve [--brain UUID] [--directory PATH]; mcp-config codex|claude [--brain UUID] [--directory PATH]";
pub fn configuration(
    host: &str,
    executable: &Path,
    endpoint: &str,
    profile: &str,
    brain: Uuid,
    directory: &Path,
    capture: Option<(&Path, Uuid)>,
) -> Result<Value> {
    ensure!(
        !brain.is_nil()
            && !profile.is_empty()
            && profile.len() <= 80
            && !profile.chars().any(char::is_control),
        "Use a Brain UUID and a valid paired profile name."
    );
    let executable = executable
        .to_str()
        .ok_or_else(|| anyhow!("Use a UTF-8 executable path."))?;
    let directory = directory
        .to_str()
        .ok_or_else(|| anyhow!("Use a UTF-8 workspace path."))?;
    let mut args = vec![
        "mcp-serve".to_owned(),
        "--brain".into(),
        brain.to_string(),
        "--directory".into(),
        directory.into(),
    ];
    if let Some((path, id)) = capture {
        args.extend([
            "--capture-setup".into(),
            path.to_str()
                .ok_or_else(|| anyhow!("Use a UTF-8 capture setup path."))?
                .into(),
            "--capture-launch".into(),
            id.to_string(),
        ]);
    }
    let mut server = json!({"command":executable, "args":args,
        "env":{"RECOLLECT_URL":endpoint, "RECOLLECT_DEVICE_PROFILE":profile}});
    if let Some(path) = recollect_mcp_runtime::agent_tls::configured_path()? {
        server["env"]["RECOLLECT_CA_FILE"] = json!(
            path.to_str()
                .ok_or_else(|| anyhow!("Use a UTF-8 CA certificate path."))?
        );
    }
    let (config, host_arguments) = if host == "codex" {
        // Codex filters the MCP child's inherited environment. Linux's OS store
        // needs the current user's session bus, never a copied credential.
        if cfg!(target_os = "linux") {
            server["env_vars"] = json!(["DBUS_SESSION_BUS_ADDRESS", "XDG_RUNTIME_DIR"]);
        }
        server["startup_timeout_sec"] = json!(120);
        server["tool_timeout_sec"] = json!(90);
        let value = toml::Value::try_from(&server)?;
        let config = toml::to_string_pretty(&json!({"mcp_servers":{"recollect":server}}))?;
        (
            Value::String(config),
            vec![
                "--config".to_owned(),
                format!("mcp_servers.recollect={value}"),
            ],
        )
    } else {
        ensure!(
            matches!(host, "claude" | "claude_code"),
            "Choose codex or claude."
        );
        server["type"] = json!("stdio");
        let config = json!({"mcpServers":{"recollect":server}});
        let arguments = vec!["--mcp-config".to_owned(), config.to_string()];
        (config, arguments)
    };
    Ok(
        json!({"host":host,"brain_id":brain,"configuration":config,"host_arguments":host_arguments,
        "state":"configured_only", "credential_source":"paired OS credential store",
        "instruction":"Pass host_arguments to this host launch, or copy configuration into its project MCP settings. Pair this profile first. Secrets are never embedded. Vault is optional per managed connection."}),
    )
}
