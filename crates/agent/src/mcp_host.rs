//! Secret-free settings for one fixed native bridge. Nothing writes host profiles.
use anyhow::{Result, anyhow, ensure};
use serde_json::{Value, json};
use std::path::Path;
use uuid::Uuid;

pub const USAGE: &str = "mcp-serve [--brain UUID] [--directory PATH]; mcp-config codex|claude|opencode [--brain UUID] [--directory PATH]; mcp-config codex-remote|claude-remote|opencode-remote --brain UUID [--directory PATH]";
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
    let mut server = json!({"command":executable, "args":args.clone(),
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
    } else if host == "opencode" {
        // OpenCode v2 runs local MCP servers over stdio with the same bridge
        // binary. Its project configuration uses a command array and an
        // environment map under mcp.servers; there is no host CLI flag that
        // carries this server, so callers copy the returned configuration.
        let mut command = vec![executable.to_owned()];
        command.extend(args);
        let mut environment = json!({"RECOLLECT_URL":endpoint, "RECOLLECT_DEVICE_PROFILE":profile});
        if let Some(path) = recollect_mcp_runtime::agent_tls::configured_path()? {
            environment["RECOLLECT_CA_FILE"] = json!(
                path.to_str()
                    .ok_or_else(|| anyhow!("Use a UTF-8 CA certificate path."))?
            );
        }
        (
            json!({"mcp":{"servers":{"recollect":{"type":"local","command":command,"cwd":directory,"environment":environment}}}}),
            vec![],
        )
    } else {
        ensure!(
            matches!(host, "claude" | "claude_code"),
            "Choose codex, claude or opencode."
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

/// Secret-free remote HTTP configuration for plugin-only clients: no
/// companion binary, no bridge process. The user token is always read from
/// the host environment at connection time under the literal variable name
/// `RECOLLECT_MCP_TOKEN`; the value never appears in rendered output. The
/// endpoint selects exactly one Brain; the token itself inherits the
/// account's current grants and is not Brain-restricted.
pub fn configuration_remote(host: &str, endpoint: &str, brain: Uuid) -> Result<Value> {
    let short = host
        .strip_suffix("-remote")
        .filter(|_| matches!(host, "codex-remote" | "claude-remote" | "opencode-remote"))
        .ok_or_else(|| anyhow!("Choose codex-remote, claude-remote or opencode-remote."))?;
    ensure!(
        !brain.is_nil(),
        "Use a Brain UUID and a valid service endpoint."
    );
    ensure!(
        !endpoint.is_empty() && endpoint.len() <= 2048 && !endpoint.chars().any(char::is_control),
        "Use a Brain UUID and a valid service endpoint."
    );
    let url = format!(
        "{}/api/brains/{brain}/mcp/agent",
        endpoint.trim_end_matches('/')
    );
    let configuration = if short == "codex" {
        let server = json!({"url":url, "bearer_token_env_var":"RECOLLECT_MCP_TOKEN"});
        Value::String(toml::to_string_pretty(
            &json!({"mcp_servers":{"recollect":server}}),
        )?)
    } else if short == "claude" {
        json!({"mcpServers":{"recollect":{"type":"http","url":url,
            "headers":{"Authorization":"Bearer ${RECOLLECT_MCP_TOKEN}"}}}})
    } else {
        json!({"mcp":{"servers":{"recollect":{"type":"remote","url":url,"oauth":false,
            "headers":{"Authorization":"Bearer {env:RECOLLECT_MCP_TOKEN}"}}}}})
    };
    Ok(
        json!({"host":host,"brain_id":brain,"configuration":configuration,"host_arguments":[],
        "state":"configured_only", "credential_source":"RECOLLECT_MCP_TOKEN environment variable",
        "instruction":"Create an access token in Recollect (Connections, coding-agent dialog, or the plugin device-code flow), export RECOLLECT_MCP_TOKEN in the terminal that starts your agent, and copy configuration into its project MCP settings. A successful Recollect tool call confirms the connection. Secrets are never embedded. Vault is optional per managed connection."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_renders_local_stdio_configuration() {
        let brain = Uuid::new_v4();
        let value = configuration(
            "opencode",
            Path::new("/tmp/recollect-agent"),
            "http://127.0.0.1:8787",
            "default",
            brain,
            Path::new("/tmp/work"),
            None,
        )
        .unwrap();
        assert_eq!(value["host"], json!("opencode"));
        let server = &value["configuration"]["mcp"]["servers"]["recollect"];
        assert_eq!(server["type"], json!("local"));
        let command = server["command"].as_array().unwrap();
        assert_eq!(command[0], json!("/tmp/recollect-agent"));
        assert!(command.iter().any(|entry| entry == "mcp-serve"));
        assert_eq!(
            server["environment"]["RECOLLECT_URL"],
            json!("http://127.0.0.1:8787")
        );
        assert_eq!(server["cwd"], json!("/tmp/work"));
        assert!(value["host_arguments"].as_array().unwrap().is_empty());
        assert_eq!(value["state"], json!("configured_only"));
        assert!(!serde_json::to_string(&value).unwrap().contains("Bearer"));
    }

    #[test]
    fn unknown_host_is_rejected() {
        let brain = Uuid::new_v4();
        assert!(
            configuration(
                "cursor",
                Path::new("/tmp/recollect-agent"),
                "http://127.0.0.1:8787",
                "default",
                brain,
                Path::new("/tmp/work"),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn remote_hosts_render_secret_free_http_configuration() {
        let brain = Uuid::new_v4();
        let endpoint = "http://127.0.0.1:8787";
        let expected_url = format!("{endpoint}/api/brains/{brain}/mcp/agent");
        let codex = configuration_remote("codex-remote", endpoint, brain).unwrap();
        let codex_config = codex["configuration"].as_str().unwrap();
        assert!(codex_config.contains(&expected_url));
        assert!(codex_config.contains(r#"bearer_token_env_var = "RECOLLECT_MCP_TOKEN""#));
        let claude = configuration_remote("claude-remote", endpoint, brain).unwrap();
        let server = &claude["configuration"]["mcpServers"]["recollect"];
        assert_eq!(server["type"], json!("http"));
        assert_eq!(server["url"], json!(expected_url));
        assert_eq!(
            server["headers"]["Authorization"],
            json!("Bearer ${RECOLLECT_MCP_TOKEN}")
        );
        let opencode = configuration_remote("opencode-remote", endpoint, brain).unwrap();
        let remote = &opencode["configuration"]["mcp"]["servers"]["recollect"];
        assert_eq!(remote["type"], json!("remote"));
        assert_eq!(remote["url"], json!(expected_url));
        assert_eq!(remote["oauth"], json!(false));
        assert_eq!(
            remote["headers"]["Authorization"],
            json!("Bearer {env:RECOLLECT_MCP_TOKEN}")
        );
        for value in [&codex, &claude, &opencode] {
            assert_eq!(value["state"], json!("configured_only"));
            assert!(value["host_arguments"].as_array().unwrap().is_empty());
            let rendered = serde_json::to_string(value).unwrap();
            assert!(rendered.contains("RECOLLECT_MCP_TOKEN"));
            assert!(
                !rendered.contains("Bearer ey"),
                "no secret literal: {rendered}"
            );
        }
    }

    #[test]
    fn remote_hosts_reject_unknown_names_and_missing_brain() {
        let brain = Uuid::new_v4();
        assert!(configuration_remote("codex", "http://127.0.0.1:8787", brain).is_err());
        assert!(configuration_remote("cursor-remote", "http://127.0.0.1:8787", brain).is_err());
        assert!(
            configuration_remote("codex-remote", "http://127.0.0.1:8787", Uuid::nil()).is_err()
        );
        assert!(configuration_remote("codex-remote", "", brain).is_err());
    }
}
