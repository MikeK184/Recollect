//! Validation for the checked-in static plugin source.
//!
//! `plugins/recollect` is the source marketplace for the packaged native plugin.
//! Skills, hooks and a bundled runtime share the canonical MCP tools. Every
//! shipped file must stay secret-free: only the variable name
//! `RECOLLECT_MCP_TOKEN` is written down, never a value.
use anyhow::{Result, anyhow, ensure};
use std::path::{Path, PathBuf};

/// Marketplace root relative to the repository, resolved from this crate.
pub fn bundle_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/recollect")
}

fn read(root: &Path, name: &str) -> Result<String> {
    std::fs::read_to_string(root.join(name))
        .map_err(|_| anyhow!("Static plugin bundle is missing {name}."))
}

/// Every `Bearer ` literal must be followed by a placeholder (`${...}` or
/// `{env:...}`), never a value. A lowercase `bearer` mention in prose is not
/// an authorization header.
fn secret_free(name: &str, text: &str) -> Result<()> {
    for (index, _) in text.match_indices("Bearer ") {
        let next = text[index + "Bearer ".len()..].chars().next();
        ensure!(
            matches!(next, Some('$') | Some('{')),
            "Static plugin file {name} embeds a bearer value instead of a placeholder."
        );
    }
    ensure!(
        !text.contains("RECOLLECT_MCP_TOKEN="),
        "Static plugin file {name} assigns the token variable instead of only naming it."
    );
    Ok(())
}

fn frontmatter(skill: &str) -> Result<&str> {
    let rest = skill
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("Static SKILL.md needs frontmatter."))?;
    rest.split_once("---\n")
        .map(|(front, _)| front)
        .ok_or_else(|| anyhow!("Static SKILL.md needs frontmatter."))
}

pub fn validate_bundle(root: &Path) -> Result<()> {
    let marketplace: serde_json::Value =
        serde_json::from_str(&read(root, ".agents/plugins/marketplace.json")?)?;
    let source = &marketplace["plugins"][0]["source"];
    ensure!(
        source["source"] == serde_json::json!("local"),
        "Static marketplace must use a local plugin source."
    );
    let plugin_dir = root.join(
        source["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Static marketplace needs a local plugin path."))?,
    );
    let plugin: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(plugin_dir.join(".codex-plugin/plugin.json"))
            .map_err(|_| anyhow!("Static Codex plugin manifest is missing."))?,
    )?;
    let name = plugin["name"].as_str().unwrap_or_default();
    ensure!(
        !name.is_empty() && name.len() <= 64,
        "Static Codex plugin needs a bounded name."
    );
    ensure!(
        plugin["skills"].as_str().is_some_and(|s| !s.is_empty()),
        "Static Codex plugin must point at its skills."
    );
    let claude: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(plugin_dir.join(".claude-plugin/plugin.json"))
            .map_err(|_| anyhow!("Static Claude plugin manifest is missing."))?,
    )?;
    ensure!(
        claude["name"] == plugin["name"],
        "Static Codex and Claude manifests must name the same plugin."
    );
    let skill = std::fs::read_to_string(plugin_dir.join("skills/recollect-memory/SKILL.md"))
        .map_err(|_| anyhow!("Static memory skill is missing."))?;
    let front = frontmatter(&skill)?;
    let skill_name = front
        .lines()
        .find_map(|line| line.strip_prefix("name:"))
        .map(str::trim)
        .unwrap_or_default();
    ensure!(
        !skill_name.is_empty() && skill_name.len() <= 64,
        "Static skill needs a bounded frontmatter name."
    );
    ensure!(
        front.lines().any(|line| line.starts_with("description:")),
        "Static skill needs a frontmatter description."
    );
    for required in [
        "workspace.list",
        "memory.recall",
        "memory.contribute",
        "recollect-connect",
    ] {
        ensure!(
            skill.contains(required),
            "Static skill must document {required}."
        );
    }
    secret_free("SKILL.md", &skill)?;
    let connect = std::fs::read_to_string(plugin_dir.join("skills/recollect-connect/SKILL.md"))?;
    frontmatter(&connect)?;
    ensure!(
        connect.contains("--with-runner") && connect.contains("OS store"),
        "Connection skill must describe explicit runner opt-in and credential storage."
    );
    secret_free("connect skill", &connect)?;
    for (manifest, host) in [(&plugin, "codex"), (&claude, "claude_code")] {
        ensure!(
            manifest["mcpServers"] == "./.mcp.json",
            "Plugin must bundle its MCP adapter."
        );
        let hook_path = format!("./hooks/{host}.json");
        ensure!(
            manifest["hooks"] == hook_path,
            "Plugin must select its native host hooks."
        );
        let text = std::fs::read_to_string(plugin_dir.join(&hook_path))?;
        secret_free("hooks", &text)?;
        let hooks: serde_json::Value = serde_json::from_str(&text)?;
        for event in [
            "SessionStart",
            "UserPromptSubmit",
            "PostToolUse",
            "Stop",
            "PreCompact",
            "SessionEnd",
        ] {
            let hook = &hooks["hooks"][event][0]["hooks"][0];
            ensure!(
                hook["type"] == "command"
                    && hook["command"]
                        == format!("\"${{CLAUDE_PLUGIN_ROOT}}/bin/recollect-plugin\" hook {host}"),
                "Native {event} hook must invoke only the packaged runtime."
            );
            ensure!(
                hook["timeout"].as_u64().is_some_and(|v| v <= 10),
                "Hooks must have bounded timeouts."
            );
        }
    }
    let mcp_text = std::fs::read_to_string(plugin_dir.join(".mcp.json"))?;
    secret_free("bundled MCP", &mcp_text)?;
    let mcp: serde_json::Value = serde_json::from_str(&mcp_text)?;
    ensure!(
        mcp["mcpServers"]["recollect"]["command"] == "sh"
            && mcp["mcpServers"]["recollect"]["args"][0] == "-c"
            && mcp["mcpServers"]["recollect"]["args"][1]
                .as_str()
                .is_some_and(|script| script.contains("runtime-path")
                    && script.ends_with("exec \"$runtime\" mcp")
                    && !script.contains("eval")),
        "Bundled MCP must use the plugin-owned runtime pointer without eval."
    );
    let template = std::fs::read_to_string(plugin_dir.join("mcp-template.json"))
        .map_err(|_| anyhow!("Static Claude MCP template is missing."))?;
    let template_json: serde_json::Value = serde_json::from_str(&template)?;
    ensure!(
        template_json["mcpServers"]["recollect"]["headers"]["Authorization"]
            == serde_json::json!("Bearer ${RECOLLECT_MCP_TOKEN}"),
        "Static Claude template must use the token placeholder only."
    );
    secret_free("mcp-template.json", &template)?;
    let opencode = read(root, "opencode-mcp.example.json")?;
    let opencode_json: serde_json::Value = serde_json::from_str(&opencode)?;
    ensure!(
        opencode_json["mcp"]["servers"]["recollect"]["headers"]["Authorization"]
            == serde_json::json!("Bearer {env:RECOLLECT_MCP_TOKEN}"),
        "Static OpenCode snippet must use the token placeholder only."
    );
    ensure!(
        opencode_json["mcp"]["servers"]["recollect"]["oauth"] == serde_json::json!(false),
        "Static OpenCode snippet must disable OAuth."
    );
    secret_free("opencode-mcp.example.json", &opencode)?;
    let readme = read(root, "README.md")?;
    ensure!(
        readme.contains("codex mcp add") && readme.contains("RECOLLECT_MCP_TOKEN"),
        "Static plugin README must document token setup."
    );
    secret_free("README.md", &readme)?;
    let marketplace_text = read(root, ".agents/plugins/marketplace.json")?;
    secret_free("marketplace.json", &marketplace_text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_plugin_bundle_is_secret_free_and_well_formed() {
        validate_bundle(&bundle_root()).unwrap();
    }
}
