//! Actual host binaries, native bridge, paired HTTP handlers, PostgreSQL and OS
//! Secret Service. Synthetic model endpoints never need a paid provider key.
use super::*;
use axum::{
    body::to_bytes,
    extract::{Request, State},
    response::{IntoResponse, Response},
};
use recollect_agent::{
    Client, CredentialSlot, StoredDevice,
    capture::{Inbox, profile_root},
    capture_cli::read_setup,
    capture_setup::{self, SetupOptions},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::process::Command;

pub(super) const PROMPT: &str = "RECOLLECT_MCP_HOST_PROOF: inspect the workspace and change the given task to its new scope, then report refreshed context.";
const MARKER: &str = "HOST_FRESH_SCOPE_B_CONTEXT";
#[derive(Clone)]
pub(super) struct Model {
    pub(super) automatic: bool,
    pub(super) prompt: &'static str,
    pub(super) calls: usize,
    pub(super) args: Value,
    pub(super) saw_context: bool,
    pub(super) saw_recall_tool: bool,
    pub(super) names: Vec<String>,
    pub(super) bodies: Vec<Value>,
}
fn stream(events: Vec<(&str, Value)>) -> Response {
    let text: String = events
        .into_iter()
        .map(|(name, v)| format!("event: {name}\ndata: {v}\n\n"))
        .collect();
    ([("content-type", "text/event-stream")], text).into_response()
}
pub(super) async fn provider(State(state): State<Arc<Mutex<Model>>>, request: Request) -> Response {
    let path = request.uri().path().to_string();
    let bytes = to_bytes(request.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or_default();
    if path.ends_with("count_tokens") {
        return axum::Json(json!({"input_tokens":42})).into_response();
    }
    let mut callable = Vec::new();
    let mut code_mode = None;
    let specifications = body["tools"].as_array().into_iter().flatten().chain(
        body["input"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| item["type"] == "additional_tools")
            .flat_map(|item| item["tools"].as_array().into_iter().flatten()),
    );
    for spec in specifications {
        if spec["type"] == "namespace" {
            for tool in spec["tools"].as_array().into_iter().flatten() {
                if let (Some(namespace), Some(name)) =
                    (spec["name"].as_str(), tool["name"].as_str())
                {
                    if namespace == "functions" && name == "exec" && tool["type"] == "custom" {
                        code_mode = Some((namespace.to_owned(), name.to_owned()));
                    }
                    callable.push((Some(namespace.to_owned()), name.to_owned()));
                }
            }
        } else if let Some(name) = spec["name"].as_str() {
            callable.push((None, name.to_owned()));
        }
    }
    let names = callable
        .iter()
        .map(|(namespace, name)| {
            namespace
                .as_ref()
                .map_or_else(|| name.clone(), |ns| format!("{ns}__{name}"))
        })
        .collect::<Vec<_>>();
    let mut model = state.lock().unwrap();
    let main = body.to_string().contains(model.prompt)
        && (path.ends_with("responses") || path.ends_with("messages"))
        && (!model.automatic || !callable.is_empty());
    if !main {
        return axum::Json(json!({"id":"ancillary","type":"message","role":"assistant","model":"claude-sonnet-4-6","content":[{"type":"text","text":"Synthetic auxiliary reply"}],"stop_reason":"end_turn","usage":{"input_tokens":42,"output_tokens":8}})).into_response();
    }
    model.names.extend(names.clone());
    let step = model.calls;
    model.calls += 1;
    model.saw_context |= step >= 2 && body.to_string().contains(MARKER);
    // Code mode advertises deferred MCP tools through its live ALL_TOOLS list.
    // The generated script below never contains this name; seeing it on a later
    // model turn proves the host actually returned its tool catalogue.
    model.saw_recall_tool |=
        step >= 1 && body.to_string().contains("mcp__recollect__memory_recall");
    model.bodies.push(body.clone());
    let suffix = if step == 0 {
        "workspace_list"
    } else {
        "workspace_set_scope"
    };
    let steps = if model.automatic { 1 } else { 2 };
    let mut tool = if step < steps {
        callable
            .iter()
            .find(|(_, name)| name.replace('.', "_").ends_with(suffix))
            .cloned()
    } else {
        None
    };
    let mut args = if step == 0 {
        json!({})
    } else {
        model.args.clone()
    };
    // OpenCode V2 exposes MCP through its native execute tool by default.
    // Exercise that installed default instead of changing host configuration.
    if model.automatic
        && step == 0
        && tool.is_none()
        && callable.iter().any(|(_, name)| name == "execute")
    {
        tool = Some((None, "execute".into()));
        args = json!({"code":"return await tools.recollect.workspace_list({});"});
    }
    let reply = "Verified fresh Recollect context.";
    if path.ends_with("responses") {
        let item = if let Some((namespace, name)) = code_mode.filter(|_| step < steps) {
            let script = format!(
                "const available = ALL_TOOLS.filter(t => t.name.includes('recollect'));\n\
                 text({{available_tool_names: ALL_TOOLS.map(t => t.name)}});\n\
                 const chosen = available.find(t => t.name.replaceAll('.', '_').endsWith('{suffix}'));\n\
                 if (!chosen) throw new Error('owned_fixture_tool_missing');\n\
                 const result = await tools[chosen.name]({args});\n\
                 text({{tool_names: available.map(t => t.name), result}});"
            );
            json!({"type":"custom_tool_call","namespace":namespace,"name":name,"input":script,
                "id":format!("ct_{step}"),"call_id":format!("call_{step}"),"status":"completed"})
        } else {
            match tool {
                Some((namespace, name)) => {
                    json!({"type":"function_call","namespace":namespace,"id":format!("fc_{step}"),"call_id":format!("call_{step}"),"name":name,"arguments":args.to_string(),"status":"completed"})
                }
                None => {
                    json!({"type":"message","id":format!("msg_{step}"),"role":"assistant","status":"completed","content":[{"type":"output_text","text":reply,"annotations":[]}]})
                }
            }
        };
        return stream(vec![
            (
                "response.created",
                json!({"type":"response.created","response":{"id":format!("resp_{step}"),"status":"in_progress","output":[]}}),
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
                json!({"type":"response.completed","response":{"id":format!("resp_{step}"),"status":"completed","output":[item],"usage":{"input_tokens":42,"output_tokens":20,"total_tokens":62}}}),
            ),
        ]);
    }
    let (block, start, delta, stop) = match tool {
        Some((_, name)) => (
            json!({"type":"tool_use","id":format!("tool_{step}"),"name":name,"input":args}),
            json!({"type":"tool_use","id":format!("tool_{step}"),"name":name,"input":{}}),
            json!({"type":"input_json_delta","partial_json":args.to_string()}),
            "tool_use",
        ),
        None => (
            json!({"type":"text","text":reply}),
            json!({"type":"text","text":""}),
            json!({"type":"text_delta","text":reply}),
            "end_turn",
        ),
    };
    let mut message = json!({"id":format!("msg_{step}"),"type":"message","role":"assistant","model":"claude-sonnet-4-6",
        "content":[block],"stop_reason":stop,"stop_sequence":null,"usage":{"input_tokens":42,"output_tokens":20}});
    if body["stream"] != true {
        return axum::Json(message).into_response();
    }
    message["content"] = json!([]);
    message["stop_reason"] = Value::Null;
    stream(vec![
        (
            "message_start",
            json!({"type":"message_start","message":message}),
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
    ])
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires isolated actual hosts and Secret Service; run scripts/test-mcp-hosts.sh"]
async fn mcp_actual_hosts_native_tools_and_fresh_context() {
    assert_eq!(
        std::env::var("RECOLLECT_MCP_HOST_FIXTURE").as_deref(),
        Ok("1"),
        "Use only the owned host fixture."
    );
    let h = Harness::new().await;
    let owner = h.login().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let executable = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug/recollect-agent")
        .canonicalize()
        .unwrap();
    let client = Client::new(&endpoint).unwrap();
    for (host, program) in [("codex", "codex"), ("claude_code", "claude")] {
        let (device_id, token) = h
            .pair_device(&owner, &format!("{host} native host proof"))
            .await;
        let device = StoredDevice {
            endpoint: endpoint.clone(),
            device_id,
            token: token.parse().unwrap(),
        };
        let profile = format!("host-proof-{}", Uuid::new_v4());
        let slot = CredentialSlot::new(&endpoint, &profile).unwrap();
        slot.save(&device).unwrap();
        let brain = ok(
            &h,
            &owner,
            "POST",
            "/api/brains",
            json!({"name":format!("{host} fixture")}),
        )
        .await;
        let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
        let base = format!("/api/brains/{brain}");
        let mut scopes = Vec::new();
        for name in ["Host scope A", "Host scope B"] {
            let area = ok(
                &h,
                &owner,
                "POST",
                &format!("{base}/evidence/groups"),
                json!({"kind":"area","name":name}),
            )
            .await;
            scopes.push(json!({"repository_ids":[],"area_ids":[area["id"]],"environment_id":null}));
        }
        let source=ok(&h,&owner,"POST",&format!("{base}/sources"),json!({"title":"Owned host evidence","media_type":"text/plain","content":"A documented scoped fixture.","retain_content":true,"source_uri":null,"group_ids":[]})).await;
        let mut claim = crate::review::proposal(&source["version"]["id"], "Host context", MARKER);
        claim["content"]["selection"] = scopes[1].clone();
        ok(&h, &owner, "POST", &format!("{base}/claims"), claim).await;
        let (status, task) = h
            .bearer(
                "POST",
                &format!("{base}/workspace/tasks"),
                &token,
                json!({"label":"Managed host task","selection":scopes[0]}),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
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
        let directory = PathBuf::from(&h.state.config.artifact_dir).join(host);
        tokio::fs::create_dir_all(directory.join(".recollect"))
            .await
            .unwrap();
        tokio::fs::write(
            directory.join(".recollect/workspace.toml"),
            format!("brain = \"{brain}\"\n"),
        )
        .await
        .unwrap();
        let installed = Command::new(program)
            .arg("--version")
            .output()
            .await
            .unwrap();
        assert!(installed.status.success());
        let installed = String::from_utf8(installed.stdout).unwrap();
        let version = installed
            .split_whitespace()
            .find(|v| v.starts_with(|c: char| c.is_ascii_digit()))
            .unwrap();
        let prepared = capture_setup::prepare(
            &client,
            &device,
            SetupOptions {
                host: host.into(),
                host_version: version.into(),
                directory: directory.clone(),
                brain: Some(brain),
                task: Some(task["task"]["id"].as_str().unwrap().parse().unwrap()),
                agent_id: None,
                output: None,
                executable: executable.clone(),
                device_profile: profile.clone(),
                evidence_root: directory.join("evidence"),
            },
        )
        .await
        .unwrap();
        assert!(
            recollect_agent::privacy::deletion_fence(&client, &device, brain)
                .await
                .expect("live Brain fence read remains available to the capture companion")
                .is_none()
        );
        let model = Arc::new(Mutex::new(Model {
            automatic: false,
            prompt: PROMPT,
            calls: 0,
            args: json!({"id":task["task"]["id"],"input":{"base_scope":task["task"]["scope"]["id"],"selection":scopes[1]},"context_query":"Host context"}),
            saw_context: false,
            saw_recall_tool: false,
            names: vec![],
            bodies: vec![],
        }));
        let model_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let model_endpoint = format!("http://{}", model_listener.local_addr().unwrap());
        let model_router = Router::new().fallback(provider).with_state(model.clone());
        let model_serving =
            tokio::spawn(async move { axum::serve(model_listener, model_router).await.unwrap() });
        let mut command = Command::new(&executable);
        command
            .args([
                "capture",
                "run",
                prepared.setup_file.to_str().unwrap(),
                "--",
            ])
            .env("RECOLLECT_URL", &endpoint)
            .env("RECOLLECT_DEVICE_PROFILE", &profile)
            .env_remove("VAULT_TOKEN")
            .kill_on_drop(true)
            .stdin(std::process::Stdio::null());
        if host == "codex" {
            command
                .args([
                    "exec",
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
                    "mcp_servers.recollect.required=true",
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
        } else {
            command
                .args([
                    "-p",
                    "--model",
                    "claude-sonnet-4-6",
                    "--allowedTools",
                    "mcp__recollect__*",
                    "--setting-sources",
                    "",
                    "--strict-mcp-config",
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
        command.arg(PROMPT);
        let output = tokio::time::timeout(Duration::from_secs(90), command.output())
            .await
            .expect("bounded real host run")
            .unwrap();
        tokio::fs::write(directory.join("stdout.txt"), &output.stdout)
            .await
            .unwrap();
        tokio::fs::write(directory.join("stderr.txt"), &output.stderr)
            .await
            .unwrap();
        let report = model.lock().unwrap().clone();
        tokio::fs::write(
            directory.join("model-requests.json"),
            serde_json::to_vec_pretty(&report.bodies).unwrap(),
        )
        .await
        .unwrap();
        assert!(
            output.status.success(),
            "{host}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            report
                .names
                .iter()
                .any(|n| n.replace('.', "_").ends_with("memory_recall"))
                || report.saw_recall_tool,
            "{host}: {:?}",
            report.names
        );
        assert_eq!(report.calls, 3, "{host}: {:?}", report.names);
        assert!(
            report.saw_context,
            "{host} did not send refreshed context to the next model turn"
        );
        drop(report);
        let setup = read_setup(&prepared.setup_file).await.unwrap();
        let inbox = Inbox::open(
            &profile_root(&setup.evidence_root, device_id),
            &endpoint,
            device_id,
        )
        .unwrap();
        assert!(inbox.cached_bindings().unwrap().iter().any(|b| {
            b.binding.operation.scope.selection.area_ids
                == vec![
                    scopes[1]["area_ids"][0]
                        .as_str()
                        .unwrap()
                        .parse::<Uuid>()
                        .unwrap(),
                ]
        }));
        if inbox.status().unwrap().delivered == 0 {
            // A diagnostic drain runs only after the actual-host delivery
            // proof has failed. It cannot turn that failure into acceptance.
            let diagnostic = recollect_agent::capture_delivery::run_once(
                &client,
                &device,
                &setup.evidence_root,
                Some(brain),
            )
            .await;
            panic!("{host}: actual hooks did not publish; diagnostic drain: {diagnostic:?}");
        }
        assert!(
            inbox.status().unwrap().delivered > 0,
            "{host}: actual hooks must publish captured evidence; local delivery metadata: {:?}",
            inbox.status().unwrap()
        );
        let captured: Vec<(Uuid, i32)> = sqlx::query_as("SELECT v.artifact_id,v.byte_length FROM capture_events e JOIN source_versions v ON v.id=e.source_version_id WHERE e.brain_id=$1 AND v.artifact_id IS NOT NULL LIMIT 100")
            .bind(brain).fetch_all(&h.admin).await.unwrap();
        assert!(captured.len() < 100);
        let mut captured_prompt = false;
        for (artifact, length) in captured {
            let body = recollect_server::artifacts::read(
                &h.state.config.artifact_dir,
                brain,
                artifact,
                length,
            )
            .await
            .unwrap();
            captured_prompt |= body.contains(PROMPT);
            assert!(
                !body.contains(MARKER),
                "{host}: recalled memory must not become independent captured evidence"
            );
        }
        assert!(
            captured_prompt,
            "{host}: the real user prompt remains captured"
        );
        drop(inbox);
        slot.forget().unwrap(); // Disposable Recollect credential, never Vault.
        model_serving.abort();
        let _ = model_serving.await;
        eprintln!("{host} {version}: native MCP tools, refreshed context and capture passed");
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM model_requests")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0,
        "synthetic host models do not invoke Recollect paid providers"
    );
    serving.abort();
    let _ = serving.await;
    h.finish().await;
}
