//! Actual installed hosts and the native hook, against synthetic HTTP providers.
//! Run with scripts/test-capture-hosts.sh; never reads personal host configuration.
use axum::{
    Router,
    body::to_bytes,
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use recollect_agent::{
    Client, StoredDevice,
    capture::{CachedCaptureBinding, Inbox, profile_root},
    capture_cli::HookSetup,
    capture_run, capture_setup, publication,
};
use recollect_protocol::*;
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::TcpListener, process::Command};
use uuid::Uuid;

const PROMPT: &str =
    "CAPTURE_HOST_PROMPT_8080: run the one synthetic shell check and report its result.";
const REPLY: &str = "CAPTURE_HOST_REPLY_8080: synthetic check completed.";
const TOOL: &str = "CAPTURE_HOST_TOOL_RESULT_8080";

#[derive(Default)]
struct Model {
    codex: usize,
    claude: usize,
    paths: Vec<String>,
    tool_names: Vec<String>,
}
fn stream(events: Vec<(&str, Value)>) -> Response {
    let text: String = events
        .into_iter()
        .map(|(name, value)| format!("event: {name}\ndata: {value}\n\n"))
        .collect();
    ([("content-type", "text/event-stream")], text).into_response()
}
async fn provider(State(state): State<Arc<Mutex<Model>>>, request: Request) -> Response {
    let path = request.uri().path().to_string();
    if path == "/api/brains" {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let bytes = to_bytes(request.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or_default();
    let mut model = state.lock().unwrap();
    model.paths.push(path.clone());
    if path.ends_with("count_tokens") {
        return axum::Json(json!({"input_tokens":42})).into_response();
    }
    let names: Vec<&str> = body["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t["name"].as_str())
        .collect();
    model
        .tool_names
        .extend(names.iter().map(|n| (*n).to_string()));
    if path.ends_with("responses") {
        let call = model.codex;
        model.codex += 1;
        let item = if call == 0 {
            let name = names
                .iter()
                .copied()
                .find(|n| matches!(*n, "exec_command" | "shell_command" | "shell"))
                .unwrap_or("exec_command");
            let command = format!("printf '{TOOL}\\n'");
            let arguments = if name == "shell" {
                json!({"command":["/bin/sh","-c",command]})
            } else if name == "shell_command" {
                json!({"command":command})
            } else {
                json!({"cmd":command,"max_output_tokens":200})
            };
            json!({"type":"function_call","id":"fc_fixture","call_id":"call_fixture",
                "name":name,"arguments":arguments.to_string(),"status":"completed"})
        } else {
            json!({"type":"message","id":"msg_fixture","role":"assistant","status":"completed",
                "content":[{"type":"output_text","text":REPLY,"annotations":[]}]})
        };
        return stream(vec![
            (
                "response.created",
                json!({"type":"response.created","response":{"id":format!("resp_{call}"),"status":"in_progress","output":[]}}),
            ),
            (
                "response.output_item.added",
                json!({"type":"response.output_item.added","output_index":0,"item":item}),
            ),
            (
                "response.output_item.done",
                json!({"type":"response.output_item.done","output_index":0,"item":item}),
            ),
            (
                "response.completed",
                json!({"type":"response.completed","response":{"id":format!("resp_{call}"),"status":"completed","output":[item],"usage":{"input_tokens":42,"output_tokens":20,"total_tokens":62}}}),
            ),
        ]);
    }
    if path.ends_with("messages") {
        // Hosts can perform ancillary requests; only the fixture's tool-enabled
        // conversation exercises the two controlled model turns.
        let main = names.contains(&"Bash") && body.to_string().contains("CAPTURE_HOST_PROMPT_8080");
        let tool = main && model.claude == 0;
        if main {
            model.claude += 1;
        }
        let block = if tool {
            json!({"type":"tool_use","id":"tool_fixture","name":"Bash",
            "input":{"command":format!("printf '{TOOL}\\n'"),"description":"Synthetic capture check"}})
        } else {
            json!({"type":"text","text":REPLY})
        };
        let stop = if tool { "tool_use" } else { "end_turn" };
        let message = json!({"id":"msg_fixture","type":"message","role":"assistant",
            "model":"claude-sonnet-4-6","content":[block],"stop_reason":stop,"stop_sequence":null,
            "usage":{"input_tokens":42,"output_tokens":20}});
        if body["stream"] != true {
            return axum::Json(message).into_response();
        }
        let start = if tool {
            json!({"type":"tool_use","id":"tool_fixture","name":"Bash","input":{}})
        } else {
            json!({"type":"text","text":""})
        };
        let delta = if tool {
            json!({"type":"input_json_delta","partial_json":block["input"].to_string()})
        } else {
            json!({"type":"text_delta","text":REPLY})
        };
        let mut initial = message;
        initial["content"] = json!([]);
        initial["stop_reason"] = Value::Null;
        return stream(vec![
            (
                "message_start",
                json!({"type":"message_start","message":initial}),
            ),
            (
                "content_block_start",
                json!({"type":"content_block_start","index":0,"content_block":start}),
            ),
            (
                "content_block_delta",
                json!({"type":"content_block_delta","index":0,"delta":delta}),
            ),
            (
                "content_block_stop",
                json!({"type":"content_block_stop","index":0}),
            ),
            (
                "message_delta",
                json!({"type":"message_delta","delta":{"stop_reason":stop,"stop_sequence":null},"usage":{"output_tokens":20}}),
            ),
            ("message_stop", json!({"type":"message_stop"})),
        ]);
    }
    axum::Json(json!({})).into_response()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Uses isolated installed Codex/Claude hosts; run scripts/test-capture-hosts.sh"]
async fn actual_hosts_capture_prompt_reply_tool_and_lifecycle_with_native_hooks() {
    assert_eq!(
        std::env::var("RECOLLECT_CAPTURE_HOST_FIXTURE").as_deref(),
        Ok("1"),
        "Run in the owned test container, not a personal host profile."
    );
    let state = Arc::new(Mutex::new(Model::default()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = Router::new().fallback(provider).with_state(state.clone());
    let api = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let root = publication::project_root()
        .join(".cache")
        .join(format!("actual-hosts-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let device = StoredDevice {
        endpoint: endpoint.clone(),
        device_id: Uuid::new_v4(),
        token: Uuid::new_v4(),
    };
    let client = Client::new(&endpoint).unwrap();
    for (host, program) in [("codex", "codex"), ("claude_code", "claude")] {
        let version = Command::new(program)
            .arg("--version")
            .output()
            .await
            .unwrap();
        assert!(version.status.success());
        println!("Host: {}", String::from_utf8_lossy(&version.stdout).trim());
        let dir = root.join(host);
        fs::create_dir_all(&dir).unwrap();
        let now = Utc::now();
        let brain = Uuid::new_v4();
        let task = Uuid::new_v4();
        let binding = CaptureBinding {
            id: Uuid::new_v4(),
            brain_id: brain,
            device_id: device.device_id,
            host: host.into(),
            host_version: String::from_utf8_lossy(&version.stdout).trim().into(),
            created_at: now,
            operation: OperationBinding {
                id: Uuid::new_v4(),
                brain_id: brain,
                task_id: task,
                kind: "capture".into(),
                actor_id: Uuid::new_v4(),
                device_id: Some(device.device_id),
                created_at: now,
                scope_valid: true,
                scope: ScopeSnapshot {
                    id: Uuid::new_v4(),
                    task_id: task,
                    brain_id: brain,
                    selection: ScopeSelection::default(),
                    repositories: vec![],
                    areas: vec![],
                    environment: None,
                    created_at: now,
                },
            },
        };
        let mut inbox = Inbox::open(
            &profile_root(&dir, device.device_id),
            &endpoint,
            device.device_id,
        )
        .unwrap();
        inbox
            .remember(&CachedCaptureBinding {
                binding: binding.clone(),
                policy: CapturePolicy {
                    enabled: true,
                    ..Default::default()
                },
                retention: RetentionPolicy::default(),
                synchronized_at: now,
                agent_id: None,
            })
            .unwrap();
        let setup = HookSetup {
            endpoint: endpoint.clone(),
            device_id: device.device_id,
            brain_id: brain,
            binding_id: binding.id,
            evidence_root: dir.clone(),
            device_profile: "unused-fixture".into(),
            host: host.into(),
            working_directory: dir.clone(),
            task_id: task,
            created_task: true,
            plugin_root: (host == "codex").then(|| dir.clone()),
        };
        let setup_path = dir.join("capture.json");
        fs::write(&setup_path, serde_json::to_vec(&setup).unwrap()).unwrap();
        let hooks = capture_setup::hooks(
            host,
            Path::new(env!("CARGO_BIN_EXE_recollect-agent")),
            &setup_path,
        )
        .unwrap();
        let hooks_path = dir.join("hooks.json");
        fs::write(&hooks_path, serde_json::to_vec(&hooks).unwrap()).unwrap();
        if host == "codex" {
            capture_setup::codex_plugin(&dir, &hooks).await.unwrap();
            capture_setup::register_codex_plugin(&dir).await.unwrap();
        }
        let mut command = Command::new(program);
        if host == "codex" {
            command.arg("exec");
        }
        command
            .args(capture_setup::host_arguments(host, &hooks_path, &hooks).unwrap())
            .current_dir(&dir)
            .env("RECOLLECT_CAPTURE_SETUP", &setup_path)
            .stdin(Stdio::null())
            .stdout(fs::File::create(dir.join("stdout.txt")).unwrap())
            .stderr(fs::File::create(dir.join("stderr.txt")).unwrap());
        if host == "codex" {
            command
                .args([
                    "--ephemeral",
                    "--ignore-rules",
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
                    "features.hooks=true",
                    "--config",
                    "model_providers.fixture.name=\"Synthetic local fixture\"",
                    "--config",
                    "model_providers.fixture.wire_api=\"responses\"",
                    "--config",
                    "model_providers.fixture.requires_openai_auth=false",
                    "--config",
                ])
                .arg(format!(
                    "model_providers.fixture.base_url=\"{endpoint}/v1\""
                ))
                .arg(PROMPT);
        } else {
            command
                .args([
                    "-p",
                    "--model",
                    "claude-sonnet-4-6",
                    "--allowedTools",
                    "Bash(printf *)",
                    "--setting-sources",
                    "",
                    "--no-session-persistence",
                    "--max-turns",
                    "3",
                    "--output-format",
                    "json",
                    PROMPT,
                ])
                .env("ANTHROPIC_BASE_URL", &endpoint)
                .env("ANTHROPIC_API_KEY", "synthetic-fixture-key")
                .env("DISABLE_AUTOUPDATER", "1")
                .env("DISABLE_TELEMETRY", "1")
                .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
        }
        let report = tokio::time::timeout(
            Duration::from_secs(45),
            capture_run::supervise(&client, &device, &setup, command),
        )
        .await;
        if !matches!(&report,Ok(Ok(value)) if value["exit_code"] == 0) {
            println!("{}", fs::read_to_string(dir.join("stdout.txt")).unwrap());
            println!("{}", fs::read_to_string(dir.join("stderr.txt")).unwrap());
            let model = state.lock().unwrap();
            println!(
                "Fixture requests: {:?}; tools: {:?}",
                model.paths, model.tool_names
            );
        }
        assert_eq!(report.expect("Host timed out").unwrap()["exit_code"], 0);
        let pending = inbox
            .pending(Utc::now() + chrono::Duration::minutes(2))
            .unwrap();
        println!(
            "{host} captured: {:?}",
            pending
                .iter()
                .map(|e| (&e.event.kind, &e.event.host_event, &e.event.coverage))
                .collect::<Vec<_>>()
        );
        for (kind, text) in [
            ("prompt", "CAPTURE_HOST_PROMPT_8080"),
            ("reply", REPLY),
            ("tool_result", TOOL),
        ] {
            assert!(
                pending.iter().any(|e| e.event.kind == kind
                    && e.event.content.as_deref().is_some_and(|s| s.contains(text))),
                "{host} missing {kind}; inspect {}",
                dir.display()
            );
        }
        assert!(pending.iter().any(|e| e.event.kind == "lifecycle"));
        assert!(pending.iter().all(|e| e.binding_id == binding.id));
        if host == "codex" {
            let before = inbox.status().unwrap().pending;
            let mut unrelated = Command::new("codex");
            unrelated
                .args([
                    "exec",
                    "--ephemeral",
                    "--ignore-rules",
                    "--json",
                    "--skip-git-repo-check",
                    "--dangerously-bypass-hook-trust",
                    "--model",
                    "gpt-5.6-luna",
                    "--enable",
                    "hooks",
                    "--config",
                    "model_provider=\"fixture\"",
                    "--config",
                    "model_providers.fixture.name=\"Synthetic local fixture\"",
                    "--config",
                    "model_providers.fixture.requires_openai_auth=false",
                    "--config",
                    "model_providers.fixture.wire_api=\"responses\"",
                    "--config",
                ])
                .arg(format!(
                    "model_providers.fixture.base_url=\"{endpoint}/v1\""
                ))
                .arg("Unbound synthetic session; no Recollect capture destination.")
                .env_remove("RECOLLECT_CAPTURE_SETUP")
                .current_dir(&dir)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true);
            assert!(
                tokio::time::timeout(Duration::from_secs(20), unrelated.status())
                    .await
                    .unwrap()
                    .unwrap()
                    .success()
            );
            assert_eq!(
                inbox.status().unwrap().pending,
                before,
                "An installed plugin must not capture an unrelated host session"
            );
            let mut failure = Command::new("/bin/sh");
            failure.args(["-c", "exit 17"]);
            let report = capture_run::supervise(&client, &device, &setup, failure)
                .await
                .unwrap();
            assert_eq!(report["exit_code"], 17);
            let verified = publication::project_root().join(".cache/verified-capture-plugin");
            for name in [".codex-plugin/plugin.json", "hooks/hooks.json"] {
                let target = verified.join(name);
                fs::create_dir_all(target.parent().unwrap()).unwrap();
                fs::copy(dir.join("plugins/recollect-capture").join(name), target).unwrap();
            }
        }
    }
    let model = state.lock().unwrap();
    assert_eq!(model.codex, 3);
    assert_eq!(model.claude, 2);
    drop(model);
    api.abort();
    fs::remove_dir_all(&root).unwrap();
}
