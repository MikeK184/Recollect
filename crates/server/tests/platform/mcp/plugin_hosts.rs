//! Ordinary native host launches with the installable plugin, not capture run.
use super::*;
use recollect_agent::plugin_storage::{self, Config};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Command};

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        assert!(!entry.file_type().unwrap().is_symlink());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

fn installed_runtimes(root: &Path, depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if depth == 0 {
        return found;
    }
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            found.extend(installed_runtimes(&entry.path(), depth - 1));
        } else if kind.is_file() && entry.file_name() == "recollect-plugin" {
            found.push(entry.path());
        }
    }
    found
}

async fn packaged_command(runtime: &Path, data: &Path, workspace: &Path, args: &[&str]) -> Value {
    let output = tokio::time::timeout(
        Duration::from_secs(150),
        Command::new(runtime)
            .args(args)
            .env("RECOLLECT_PLUGIN_DATA", data)
            .env_remove("RECOLLECT_ENOLA_BIN")
            .env_remove("VAULT_TOKEN")
            .current_dir(workspace)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        output.status.success(),
        "Packaged command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

async fn packaged_repository(runtime: &Path, data: &Path, workspace: &Path, brain: Uuid) {
    std::fs::create_dir_all(workspace.join("src")).unwrap();
    std::fs::write(
        workspace.join("src/lib.rs"),
        "pub fn plugin_fixture_port() -> u16 { 9191 }\n",
    )
    .unwrap();
    std::fs::create_dir_all(workspace.join(".recollect")).unwrap();
    std::fs::write(
        workspace.join(".recollect/workspace.toml"),
        format!("brain = \"{brain}\"\n"),
    )
    .unwrap();
    for args in [
        vec!["init", "--initial-branch=main", "--template="],
        vec![
            "remote",
            "add",
            "origin",
            "https://example.test/team/plugin-fixture.git",
        ],
        vec!["add", "--", "src/lib.rs"],
        vec![
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "user.name=Recollect Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "--no-gpg-sign",
            "-m",
            "Owned fixture",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(workspace)
                .output()
                .await
                .unwrap()
                .status
                .success()
        );
    }
    let catalogue = packaged_command(
        runtime,
        data,
        workspace,
        &["workspace", "refresh", workspace.to_str().unwrap()],
    )
    .await;
    let repositories = catalogue["catalogue"]["repositories"].as_array().unwrap();
    assert_eq!(repositories.len(), 1);
    let repository = repositories[0]["id"].as_str().unwrap();
    let task = packaged_command(
        runtime,
        data,
        workspace,
        &[
            "scope",
            "start",
            &brain.to_string(),
            "Packaged repository proof",
            "--repository",
            repository,
        ],
    )
    .await;
    let publication = packaged_command(
        runtime,
        data,
        workspace,
        &[
            "repository",
            "publish",
            &brain.to_string(),
            repository,
            task["task"]["id"].as_str().unwrap(),
            workspace.to_str().unwrap(),
        ],
    )
    .await;
    assert!(
        publication["snapshot"]["id"].is_string(),
        "Bundled Enola must publish a canonical snapshot"
    );
    assert_eq!(publication["snapshot"]["repository_id"], repository);
    eprintln!("Packaged checkout discovery and sibling Enola publication passed");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Owned installed host fixture; scripts/test-mcp-hosts.sh"]
async fn mcp_plugin_installed_codex_automatic_context() {
    installed_host("codex").await;
}
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Owned installed host fixture; scripts/test-mcp-hosts.sh"]
async fn mcp_plugin_installed_claude_automatic_context() {
    installed_host("claude_code").await;
}
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Owned installed host fixture; scripts/test-mcp-hosts.sh"]
async fn mcp_plugin_installed_opencode_automatic_context() {
    installed_host("opencode").await;
}
async fn installed_host(selected_host: &str) {
    assert_eq!(
        std::env::var("RECOLLECT_MCP_HOST_FIXTURE").as_deref(),
        Ok("1")
    );
    let h = Harness::new().await;
    let owner = h.login().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let source = std::env::var_os("RECOLLECT_PLUGIN_PACKAGE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/recollect")
        })
        .canonicalize()
        .unwrap();
    let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug/recollect-plugin")
        .canonicalize()
        .unwrap();
    for (host, program) in [
        ("codex", "codex"),
        ("claude_code", "claude"),
        ("opencode", "opencode"),
    ]
    .into_iter()
    .filter(|(host, _)| *host == selected_host)
    {
        let directory =
            PathBuf::from(&h.state.config.artifact_dir).join(format!("installed-{host}"));
        let package = directory.join("package");
        copy_tree(&source, &package);
        let plugin = package.join("plugins/recollect-memory");
        let mut runtime = plugin.join("bin/recollect-plugin");
        if !runtime.is_file() {
            std::fs::create_dir(plugin.join("bin")).unwrap();
            std::fs::copy(&binary, &runtime).unwrap();
        }
        let data = directory.join("data");
        let workspace = directory.join("work");
        std::fs::create_dir_all(&workspace).unwrap();
        let host_home = directory.join("host");
        std::fs::create_dir_all(&host_home).unwrap();
        let brain = ok(
            &h,
            &owner,
            "POST",
            "/api/brains",
            json!({"name":format!("Installed {host}")}),
        )
        .await;
        let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
        let base = format!("/api/brains/{brain}");
        let evidence = "Plugin fixture service port is 9191.";
        let source = ok(&h, &owner, "POST", &format!("{base}/sources"), json!({"title":"Plugin host workspace", "media_type":"text/plain","content":evidence,"retain_content":true,"source_uri":null,"group_ids":[]})).await;
        let claim =
            crate::review::proposal(&source["version"]["id"], "Plugin host workspace", evidence);
        ok(&h, &owner, "POST", &format!("{base}/claims"), claim).await;
        let policy = ok(
            &h,
            &owner,
            "GET",
            &format!("{base}/capture/policy"),
            Value::Null,
        )
        .await;
        let mut enabled = policy["policy"].clone();
        enabled["enabled"] = json!(true);
        ok(
            &h,
            &owner,
            "PUT",
            &format!("{base}/capture/policy"),
            json!({"base_change":policy["change_id"],"policy":enabled}),
        )
        .await;
        if matches!(host, "codex" | "claude_code") {
            let install = if host == "codex" { "add" } else { "install" };
            for args in [
                vec!["plugin", "marketplace", "add", package.to_str().unwrap()],
                vec!["plugin", install, "recollect-memory@recollect"],
            ] {
                let output = Command::new(program)
                    .args(args)
                    .env("CODEX_HOME", &host_home)
                    .env("CLAUDE_CONFIG_DIR", &host_home)
                    .current_dir(&workspace)
                    .output()
                    .await
                    .unwrap();
                assert!(
                    output.status.success(),
                    "Plugin installation: {} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        if host == "codex" || host == "claude_code" {
            let installed = installed_runtimes(&host_home, 12);
            assert_eq!(installed.len(), 1, "Exactly one installed plugin runtime");
            runtime = installed[0].clone();
        }
        let (device_id, token) = h.pair_device(&owner, &format!("installed-{host}")).await;
        let mut connect = Command::new(&runtime)
            .args([
                "connect",
                "--url",
                &endpoint,
                "--brain",
                &brain.to_string(),
                "--token-stdin",
            ])
            .env("RECOLLECT_PLUGIN_DATA", &data)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        connect
            .stdin
            .take()
            .unwrap()
            .write_all(token.as_bytes())
            .await
            .unwrap();
        let connected = connect.wait_with_output().await.unwrap();
        assert!(
            connected.status.success(),
            "One-time connect failed: {}",
            String::from_utf8_lossy(&connected.stderr)
        );
        let config: Config = plugin_storage::read(&data.join("config.json")).unwrap();
        assert_eq!(config.device, device_id);
        let model = Arc::new(Mutex::new(super::host_tools::Model {
            automatic: true,
            prompt: "Plugin fixture service port",
            calls: 0,
            args: Value::Null,
            saw_context: false,
            saw_recall_tool: false,
            names: vec![],
            bodies: vec![],
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let model_endpoint = format!("http://{}", listener.local_addr().unwrap());
        let router = Router::new()
            .fallback(super::host_tools::provider)
            .with_state(model.clone());
        let provider = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let mut command = Command::new(program);
        command
            .current_dir(&workspace)
            .env("PWD", &workspace)
            .env("RECOLLECT_PLUGIN_DATA", &data)
            .env_remove("VAULT_TOKEN")
            .kill_on_drop(true)
            .stdin(Stdio::null());
        if host == "codex" {
            command
                .env("CODEX_HOME", &host_home)
                .args([
                    "exec",
                    "--ephemeral",
                    "--json",
                    "--skip-git-repo-check",
                    "--dangerously-bypass-hook-trust",
                    "--sandbox",
                    "danger-full-access",
                    "--model",
                    "gpt-5.6-luna",
                    "--config",
                    "model_provider=\"fixture\"",
                    "--config",
                    "model_providers.fixture.name=\"Synthetic fixture\"",
                    "--config",
                    "model_providers.fixture.wire_api=\"responses\"",
                    "--config",
                    "model_providers.fixture.requires_openai_auth=false",
                    "--config",
                ])
                .arg(format!(
                    "model_providers.fixture.base_url=\"{model_endpoint}/v1\""
                ));
        } else if host == "opencode" {
            std::fs::write(workspace.join("opencode.json"), serde_json::to_vec_pretty(&json!({
                "plugins":[plugin.join("opencode")], "update":"disable", "share":"disabled", "snapshots":false,
                "providers":{"anthropic":{
                    "settings":{"baseURL":format!("{model_endpoint}/v1")},"models":{"claude-sonnet-4-6":{}}}},
                "model":"anthropic/claude-sonnet-4-6"
            })).unwrap()).unwrap();
            command
                .args([
                    "run",
                    "--standalone",
                    "--auto",
                    "--format",
                    "json",
                    "--model",
                    "anthropic/claude-sonnet-4-6",
                ])
                .env("XDG_CONFIG_HOME", host_home.join("config"))
                .env("XDG_DATA_HOME", host_home.join("data"))
                .env("XDG_CACHE_HOME", host_home.join("cache"))
                .env("XDG_STATE_HOME", host_home.join("state"))
                .env("ANTHROPIC_API_KEY", "synthetic-owned-fixture");
        } else {
            command
                .env("CLAUDE_CONFIG_DIR", &host_home)
                .arg("-p")
                .args([
                    "--model",
                    "claude-sonnet-4-6",
                    "--allowedTools",
                    "mcp__plugin_recollect-memory_recollect__*",
                    "--setting-sources",
                    "user",
                    "--no-session-persistence",
                    "--max-turns",
                    "4",
                    "--output-format",
                    "json",
                ])
                .env("ANTHROPIC_BASE_URL", &model_endpoint)
                .env("ANTHROPIC_API_KEY", "synthetic-owned-fixture")
                .env("DISABLE_AUTOUPDATER", "1")
                .env("DISABLE_TELEMETRY", "1")
                .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
        }
        command.arg("Plugin fixture service port");
        let initial_model = model.lock().unwrap().clone();
        // Reuse one install/connection across successive ordinary OpenCode
        // launches. Each launch must publish its own reply, not pass on an older one.
        for launch in 0..if host == "opencode" { 3 } else { 1 } {
            *model.lock().unwrap() = initial_model.clone();
            let output = tokio::time::timeout(Duration::from_secs(90), command.output())
                .await
                .expect("bounded installed host")
                .unwrap();
            std::fs::write(directory.join("stdout.txt"), &output.stdout).unwrap();
            std::fs::write(directory.join("stderr.txt"), &output.stderr).unwrap();
            let report = model.lock().unwrap().clone();
            std::fs::write(
                directory.join("model-requests.json"),
                serde_json::to_vec_pretty(&report.bodies).unwrap(),
            )
            .unwrap();
            assert!(
                output.status.success(),
                "Native {host} launch failed; inspect {}",
                directory.display()
            );
            let first = report.bodies.first().expect("model request").to_string();
            assert!(
                first.contains("9191") && first.contains(source["version"]["id"].as_str().unwrap()),
                "Automatic cited context must precede any model tool call; inspect {}",
                directory.display()
            );
            assert!(
                report.calls >= 2,
                "Installed MCP tools must execute and return to the native host"
            );
            assert!(
                report
                    .bodies
                    .last()
                    .unwrap()
                    .to_string()
                    .contains(&brain.to_string()),
                "The native tool result must identify the connected Brain"
            );
            if host == "codex" {
                let tool_output: String = report.bodies.last().unwrap()["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|item| item["type"] == "custom_tool_call_output")
                    .map(Value::to_string)
                    .collect();
                let native_mcp = String::from_utf8_lossy(&output.stdout).lines().any(|line| {
                    let Ok(event) = serde_json::from_str::<Value>(line) else {
                        return false;
                    };
                    let item = &event["item"];
                    item["type"] == "mcp_tool_call"
                        && item["tool"] == "workspace.list"
                        && item["status"] == "completed"
                        && item["error"].is_null()
                        && item["result"].to_string().contains(&brain.to_string())
                });
                assert!(
                    native_mcp
                        || (!tool_output.contains("Script error:")
                            && tool_output.contains("mcp__recollect__workspace_list")
                            && tool_output.contains(&brain.to_string())),
                    "Packaged MCP must actually return scoped workspace metadata; inspect {}",
                    directory.display()
                );
            }
            let until = tokio::time::Instant::now() + Duration::from_secs(20);
            loop {
                let kinds: Vec<String> = sqlx::query_scalar("SELECT e.metadata->>'kind' FROM capture_events e JOIN capture_bindings b ON b.id=e.binding_id WHERE e.brain_id=$1 AND b.device_id=$2 AND e.source_version_id IS NOT NULL").bind(brain).bind(device_id).fetch_all(&h.admin).await.unwrap();
                if kinds.iter().filter(|kind| *kind == "prompt").count() > launch
                    && kinds.iter().filter(|kind| *kind == "reply").count() > launch
                {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < until,
                    "Native prompt and visible reply must become retained evidence for {host}; got {kinds:?}"
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            let derived: i64 = sqlx::query_scalar("SELECT count(*) FROM capture_events e JOIN capture_bindings b ON b.id=e.binding_id WHERE e.brain_id=$1 AND b.device_id=$2 AND e.metadata->>'kind'='tool_result' AND e.source_version_id IS NOT NULL")
            .bind(brain).bind(device_id).fetch_one(&h.admin).await.unwrap();
            assert_eq!(
                derived, 0,
                "The fixture's first-party MCP result cannot become independent evidence"
            );
            assert!(!data.join("runner.lock").exists());
        }
        if std::env::var_os("RECOLLECT_PLUGIN_PACKAGE").is_some() {
            assert!(runtime.with_file_name("enola").is_file());
            packaged_repository(&runtime, &data, &workspace, brain).await;
        }
        let disconnected = Command::new(&runtime)
            .arg("disconnect")
            .env("RECOLLECT_PLUGIN_DATA", &data)
            .output()
            .await
            .unwrap();
        assert!(
            disconnected.status.success(),
            "Owned plugin disconnect failed"
        );
        provider.abort();
    }
    serving.abort();
    h.finish().await;
}
