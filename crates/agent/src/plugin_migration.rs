//! Read-only discovery of conflicting first-party host setup. Never expose its values.
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub struct Conflict {
    pub host: String,
    pub kind: &'static str,
    pub path: Option<PathBuf>,
}

fn legacy_hook(value: &Value) -> bool {
    match value {
        Value::Object(fields) => {
            fields
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| {
                    command.contains("recollect-agent") && command.contains("capture hook")
                })
                || fields.values().any(legacy_hook)
        }
        Value::Array(values) => values.iter().any(legacy_hook),
        _ => false,
    }
}

fn configured(host: &str, text: &str, cwd: &Path) -> Vec<&'static str> {
    let value: Option<Value> = if host == "codex" {
        toml::from_str::<toml::Value>(text)
            .ok()
            .and_then(|v| serde_json::to_value(v).ok())
    } else {
        serde_json::from_str(text).ok()
    };
    let Some(value) = value else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let servers = if host == "codex" {
        "mcp_servers"
    } else {
        "mcpServers"
    };
    if value[servers].get("recollect").is_some()
        || (host == "claude_code"
            && cwd.to_str().is_some_and(|path| {
                value["projects"][path]["mcpServers"]
                    .get("recollect")
                    .is_some()
            }))
    {
        found.push("existing_first_party_mcp");
    }
    if legacy_hook(&value["hooks"])
        || value["plugins"]["recollect-capture@recollect-capture"]["enabled"] == true
        || value["enabledPlugins"]["recollect-capture@recollect-capture"] == true
    {
        found.push("legacy_capture_hooks");
    }
    found
}

pub fn inspect(host: &str, cwd: &Path) -> Vec<Conflict> {
    let mut result = Vec::new();
    if std::env::var_os("RECOLLECT_CAPTURE_SETUP").is_some()
        || std::env::var_os("RECOLLECT_CAPTURE_LAUNCH").is_some()
    {
        result.push(Conflict {
            host: host.into(),
            kind: "legacy_managed_launch",
            path: None,
        });
    }
    let mut files = BTreeSet::new();
    // Host project layers end at the nearest checkout, not at the filesystem
    // root. Otherwise an overridden HOME configuration is rediscovered as a
    // project layer and incorrectly blocks an isolated installation.
    let project = cwd
        .ancestors()
        .take(32)
        .find(|path| {
            let git = path.join(".git");
            git.is_file() || (git.is_dir() && git.join("HEAD").is_file())
        })
        .unwrap_or(cwd);
    for root in cwd.ancestors().take(32) {
        if host == "codex" {
            files.insert(root.join(".codex/config.toml"));
        } else if host == "claude_code" {
            files.insert(root.join(".mcp.json"));
            files.insert(root.join(".claude/settings.json"));
            files.insert(root.join(".claude/settings.local.json"));
        }
        if root == project {
            break;
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if host == "codex" {
        if let Some(root) = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|p| p.join(".codex")))
        {
            files.insert(root.join("config.toml"));
        }
    } else if host == "claude_code" {
        if let Some(root) = std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from) {
            files.insert(root.join("settings.json"));
            files.insert(root.join(".claude.json"));
        } else if let Some(home) = home {
            files.insert(home.join(".claude/settings.json"));
            files.insert(home.join(".claude.json"));
        }
    }
    // OpenCode's transform inspects the effective MCP configuration directly,
    // including JSONC and inherited settings, and refuses a conflicting entry.
    for path in files {
        // Host configuration may legitimately be linked from dotfiles. This is
        // a bounded read of the exact host path, never a write or directory scan.
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > 1024 * 1024 {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for kind in configured(host, &text, cwd) {
            result.push(Conflict {
                host: host.into(),
                kind,
                path: Some(path.clone()),
            });
        }
    }
    result
}

pub fn all(cwd: &Path) -> Vec<Conflict> {
    ["codex", "claude_code", "opencode"]
        .into_iter()
        .flat_map(|host| inspect(host, cwd))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_conflicting_setup_without_returning_credentials_or_unrelated_servers() {
        assert_eq!(
            configured(
                "codex",
                "[mcp_servers.recollect]\nurl='https://example.test/mcp'\nbearer_token_env_var='PRIVATE_TOKEN'",
                Path::new("/fixture")
            ),
            ["existing_first_party_mcp"]
        );
        assert!(
            configured(
                "codex",
                "[mcp_servers.context7]\nurl='https://example.test/mcp'",
                Path::new("/fixture")
            )
            .is_empty()
        );
        assert_eq!(
            configured(
                "claude_code",
                r#"{"hooks":{"Stop":[{"hooks":[{"command":"recollect-agent capture hook original.json"}]}]}}"#,
                Path::new("/fixture")
            ),
            ["legacy_capture_hooks"]
        );
        assert!(configured("claude_code", r#"{"hooks":{"Stop":[{"hooks":[{"command":"recollect-plugin hook claude_code"}]}]}}"#, Path::new("/fixture")).is_empty());
        assert!(
            configured(
                "codex",
                "[plugins.\"recollect-capture@recollect-capture\"]\nenabled=false",
                Path::new("/fixture")
            )
            .is_empty()
        );
        let local = r#"{"projects":{"/fixture":{"mcpServers":{"recollect":{"url":"https://example.test/mcp"}}}}}"#;
        assert_eq!(
            configured("claude_code", local, Path::new("/fixture")),
            ["existing_first_party_mcp"]
        );
        assert!(configured("claude_code", local, Path::new("/unrelated")).is_empty());
    }
}
