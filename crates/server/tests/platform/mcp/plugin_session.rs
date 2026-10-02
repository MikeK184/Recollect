//! Real packaged-runtime orchestration against canonical HTTP handlers and RLS.
use super::*;
use recollect_agent::{
    Client, CredentialSlot, StoredDevice,
    plugin_storage::{self, Config},
};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Command};

async fn hook(executable: &Path, data: &Path, host: &str, payload: Value) -> Value {
    eprintln!(
        "Plugin fixture hook: {host} {} {}",
        payload["hook_event_name"], payload["session_id"]
    );
    let mut child = Command::new(executable)
        .args(["hook", host])
        .env("RECOLLECT_PLUGIN_DATA", data)
        // Legacy-host proofs deliberately install the old capture plugin in
        // their own default home. Each plugin lifecycle fixture owns a fresh
        // host configuration; migration conflicts are introduced explicitly.
        .env("CODEX_HOME", data.join("host"))
        .env("CLAUDE_CONFIG_DIR", data.join("host"))
        .env_remove("VAULT_TOKEN")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(payload.to_string().as_bytes())
        .await
        .unwrap();
    drop(stdin);
    let output = tokio::time::timeout(Duration::from_secs(15), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(output.status.success(), "Plugin hook must not block coding");
    if !output.stderr.is_empty() {
        eprintln!(
            "Plugin hook diagnostic: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires owned native runtime and Secret Service; scripts/test-mcp-hosts.sh"]
async fn mcp_plugin_native_capture_and_recall() {
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
    let executable = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug/recollect-plugin")
        .canonicalize()
        .unwrap();
    let brain = ok(
        &h,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":"Plugin memory fixture"}),
    )
    .await;
    let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain}");
    let evidence = "The plugin fixture service listens on port 9191.";
    let source = ok(&h, &owner, "POST", &format!("{base}/sources"),
        json!({"title":"Plugin fixture service","media_type":"text/plain","content":evidence,"retain_content":true,"source_uri":null,"group_ids":[]})).await;
    let claim =
        crate::review::proposal(&source["version"]["id"], "Plugin fixture service", evidence);
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
    let client = Client::new(&endpoint).unwrap();
    for host in ["codex", "claude_code", "opencode"] {
        let (device_id, token) = h.pair_device(&owner, &format!("plugin-{host}")).await;
        let device = StoredDevice {
            endpoint: endpoint.clone(),
            device_id,
            token: token.parse().unwrap(),
        };
        let profile = format!("plugin-proof-{}", Uuid::new_v4());
        let slot = CredentialSlot::new(&endpoint, &profile).unwrap();
        slot.save(&device).unwrap();
        let data = PathBuf::from(&h.state.config.artifact_dir).join(format!("plugin-{host}"));
        plugin_storage::directory(&data).unwrap();
        let config = Config {
            endpoint: endpoint.clone(),
            brain,
            device: device_id,
            profile,
            with_runner: false,
            runner_id: None,
        };
        plugin_storage::write(&data.join("config.json"), &config).unwrap();
        for session in ["session-one", "session-two"] {
            let prompt = json!({"hook_event_name":"UserPromptSubmit","session_id":session,"cwd":data,
                "host_version":"fixture-v1","turn_id":"turn-one","prompt_id":"turn-one","prompt":"Plugin fixture service port"});
            let result = hook(&executable, &data, host, prompt.clone()).await;
            let text = result["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .expect("Automatic recall without a model tool call");
            assert!(
                text.contains("9191"),
                "Relevant canonical evidence must reach the host"
            );
            assert!(
                text.contains(source["version"]["id"].as_str().unwrap()),
                "Source-version attribution must survive"
            );
            assert!(text.len() <= 8192);
            let replay = hook(&executable, &data, host, prompt).await;
            assert!(
                replay["hookSpecificOutput"]["additionalContext"]
                    .as_str()
                    .is_some_and(|s| s.contains("9191"))
            );
            hook(&executable, &data, host, json!({"hook_event_name":"Stop","session_id":session,"cwd":data,
                "turn_id":"turn-one","prompt_id":"turn-one","last_assistant_message":"The fixture answer is ready."})).await;
        }
        let a =
            recollect_agent::plugin_session::state_path(&data, &config, host, "session-one", &data)
                .unwrap();
        let b =
            recollect_agent::plugin_session::state_path(&data, &config, host, "session-two", &data)
                .unwrap();
        let a: recollect_agent::plugin_session::Session = plugin_storage::read(&a).unwrap();
        let b: recollect_agent::plugin_session::Session = plugin_storage::read(&b).unwrap();
        assert_ne!(a.setup.task_id, b.setup.task_id);
        assert_ne!(a.setup.binding_id, b.setup.binding_id);
        if host != "opencode" {
            // Exact legacy conflicts withhold duplicate capture without editing
            // the host configuration or destroying existing queue bindings.
            let host_home = data.join("host");
            plugin_storage::directory(&host_home).unwrap();
            let (path, bytes) = if host == "codex" {
                (
                    host_home.join("config.toml"),
                    "[mcp_servers.recollect]\nurl='http://legacy.invalid/mcp'\n".to_owned(),
                )
            } else {
                (host_home.join(".claude.json"), json!({"projects":{data.to_str().unwrap():{"mcpServers":{"recollect":{"url":"http://legacy.invalid/mcp"}}}}}).to_string())
            };
            std::fs::write(&path, &bytes).unwrap();
            let blocked = hook(
                &executable,
                &data,
                host,
                json!({"hook_event_name":"UserPromptSubmit",
                "session_id":"migration-conflict","cwd":data,"host_version":"fixture-v1",
                "turn_id":"blocked","prompt_id":"blocked","prompt":"Do not double capture this"}),
            )
            .await;
            assert!(blocked["hookSpecificOutput"].is_null());
            assert!(std::fs::read_to_string(&path).unwrap() == bytes);
            let inbox = recollect_agent::capture::Inbox::open(
                &recollect_agent::capture::profile_root(&data, device_id),
                &endpoint,
                device_id,
            )
            .unwrap();
            assert!(inbox.cached_binding(a.setup.binding_id).is_ok());
            std::fs::remove_file(path).unwrap();
            let resumed = hook(&executable, &data, host, json!({"hook_event_name":"UserPromptSubmit",
                "session_id":"session-one","cwd":data,"turn_id":"after-migration","prompt_id":"after-migration",
                "prompt":"Plugin fixture service port"})).await;
            assert!(resumed.to_string().contains("9191"));
        }
        // Codex/Claude identify children inside the shared root session; OpenCode
        // gives a child its own session ID and explicit parent. Both must own an
        // independent canonical task and immutable capture operations.
        let child_id = if host == "opencode" {
            "child-session"
        } else {
            "session-one"
        };
        let mut child_event = json!({"session_id":child_id,"cwd":data,"host_version":"fixture-v1",
            "turn_id":"child-one","prompt_id":"child-one"});
        if host == "opencode" {
            child_event["parent_session_id"] = json!("session-one");
            child_event["parent_cwd"] = json!(data);
        } else {
            child_event["agent_id"] = json!("child-agent");
        }
        child_event["hook_event_name"] = json!(if host == "opencode" {
            "SessionStart"
        } else {
            "SubagentStart"
        });
        hook(&executable, &data, host, child_event.clone()).await;
        let child_path = recollect_agent::plugin_session::agent_state_path(
            &data,
            &config,
            host,
            child_id,
            &data,
            if host == "opencode" {
                None
            } else {
                Some("child-agent")
            },
        )
        .unwrap();
        let child: recollect_agent::plugin_session::Session =
            plugin_storage::read(&child_path).unwrap();
        assert_ne!(child.setup.task_id, a.setup.task_id);
        assert_ne!(child.setup.binding_id, a.setup.binding_id);
        let parent: Option<Uuid> =
            sqlx::query_scalar("SELECT parent_task_id FROM workspace_tasks WHERE id=$1")
                .bind(child.setup.task_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        assert_eq!(parent, Some(a.setup.task_id));
        child_event["hook_event_name"] = json!("UserPromptSubmit");
        child_event["prompt"] = json!("Plugin fixture service port");
        let context = hook(&executable, &data, host, child_event.clone()).await;
        assert!(
            context
                .to_string()
                .contains(&child.setup.task_id.to_string())
        );
        assert!(!context.to_string().contains(&a.setup.task_id.to_string()));
        child_event["hook_event_name"] = json!("PreToolUse");
        child_event["tool_use_id"] = json!("child-delayed");
        child_event["tool_name"] = json!("Bash");
        child_event["tool_input"] = json!({"command":"inspect-fixture"});
        hook(&executable, &data, host, child_event.clone()).await;
        let area = ok(
            &h,
            &owner,
            "POST",
            &format!("{base}/evidence/groups"),
            json!({"kind":"area","name":format!("Child {host}")}),
        )
        .await;
        let updated = h
            .bearer(
                "PUT",
                &format!("{base}/workspace/tasks/{}/scope", child.setup.task_id),
                &token,
                json!({"base_scope":child.scope.id,"selection":{"area_ids":[area["id"]]}}),
            )
            .await;
        assert_eq!(updated.0, StatusCode::OK);
        child_event["hook_event_name"] = json!("UserPromptSubmit");
        child_event["turn_id"] = json!("child-two");
        child_event["prompt_id"] = json!("child-two");
        let context = hook(&executable, &data, host, child_event.clone()).await;
        assert!(
            context
                .to_string()
                .contains(updated.1["task"]["scope"]["id"].as_str().unwrap())
        );
        let parent_scope: Uuid =
            sqlx::query_scalar("SELECT current_scope FROM workspace_tasks WHERE id=$1")
                .bind(a.setup.task_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        assert_eq!(parent_scope, a.scope.id);
        child_event["hook_event_name"] = json!("PostToolUse");
        child_event["tool_response"] = json!({"stdout":"Child delayed independent observation"});
        hook(&executable, &data, host, child_event.clone()).await;
        child_event["hook_event_name"] = json!(if host == "opencode" {
            "SessionEnd"
        } else {
            "SubagentStop"
        });
        child_event["last_assistant_message"] = json!("Child scoped reply");
        hook(&executable, &data, host, child_event).await;
        let until = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            let bindings: Vec<Uuid> = sqlx::query_scalar("SELECT binding_id FROM capture_events WHERE brain_id=$1 AND metadata->>'tool_use_id'='child-delayed' AND metadata->>'kind'='tool_result'")
                .bind(brain).fetch_all(&h.admin).await.unwrap();
            if bindings.contains(&child.setup.binding_id) {
                break;
            }
            assert!(
                tokio::time::Instant::now() < until,
                "Delayed child output must retain its original capture binding"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if host != "opencode" {
            let unknown = hook(
                &executable,
                &data,
                host,
                json!({"hook_event_name":"UserPromptSubmit",
                "session_id":"session-one","agent_id":"unobserved-child","cwd":data,
                "turn_id":"unknown","prompt_id":"unknown","prompt":"Plugin fixture service port"}),
            )
            .await;
            assert!(
                unknown["hookSpecificOutput"].is_null(),
                "Unknown children cannot borrow parent recall"
            );
            let ambiguous = hook(
                &executable,
                &data,
                host,
                json!({"hook_event_name":"SubagentStart",
                "session_id":"session-one","cwd":data}),
            )
            .await;
            assert!(ambiguous["hookSpecificOutput"].is_null());
        }
        let until = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM capture_events e JOIN capture_bindings b ON b.id=e.binding_id WHERE e.brain_id=$1 AND b.device_id=$2")
                .bind(brain).bind(device_id).fetch_one(&h.admin).await.unwrap();
            if count >= 4 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < until,
                "Detached delivery must publish capture"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(
            !data.join("runner.lock").exists(),
            "Default memory must never start the runner"
        );
        // Session-end closes only its own canonical task. Resume preserves its
        // selected scope while old operations remain usable for delayed delivery.
        hook(
            &executable,
            &data,
            host,
            json!({"hook_event_name":"SessionEnd","session_id":"session-one","cwd":data}),
        )
        .await;
        let until = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            let closed: bool = sqlx::query_scalar("SELECT closed FROM workspace_tasks WHERE id=$1")
                .bind(a.setup.task_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
            if closed {
                break;
            }
            assert!(
                tokio::time::Instant::now() < until,
                "Session end must release its task"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let resumed = hook(&executable, &data, host, json!({"hook_event_name":"UserPromptSubmit","session_id":"session-one","cwd":data,
            "host_version":"fixture-v1","prompt_id":"turn-resumed","prompt":"Plugin fixture service port"})).await;
        assert!(
            resumed["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .is_some_and(|s| s.contains("9191"))
        );
        let resumed: recollect_agent::plugin_session::Session = plugin_storage::read(
            &recollect_agent::plugin_session::state_path(
                &data,
                &config,
                host,
                "session-one",
                &data,
            )
            .unwrap(),
        )
        .unwrap();
        assert_ne!(resumed.setup.task_id, a.setup.task_id);
        assert_eq!(
            serde_json::to_value(&resumed.scope.selection).unwrap(),
            serde_json::to_value(&a.scope.selection).unwrap()
        );
        let current = ok(
            &h,
            &owner,
            "GET",
            &format!("{base}/capture/policy"),
            Value::Null,
        )
        .await;
        let mut disabled = current["policy"].clone();
        disabled["enabled"] = json!(false);
        ok(
            &h,
            &owner,
            "PUT",
            &format!("{base}/capture/policy"),
            json!({"base_change":current["change_id"],"policy":disabled}),
        )
        .await;
        let read_only = hook(&executable, &data, host, json!({"hook_event_name":"UserPromptSubmit","session_id":"capture-off","cwd":data,
            "host_version":"fixture-v1","prompt_id":"turn-off","prompt":"Plugin fixture service port"})).await;
        assert!(
            read_only["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .is_some_and(|s| s.contains("9191")),
            "Disabling capture must not disable memory reads"
        );
        let current = ok(
            &h,
            &owner,
            "GET",
            &format!("{base}/capture/policy"),
            Value::Null,
        )
        .await;
        let mut enabled = current["policy"].clone();
        enabled["enabled"] = json!(true);
        ok(
            &h,
            &owner,
            "PUT",
            &format!("{base}/capture/policy"),
            json!({"base_change":current["change_id"],"policy":enabled}),
        )
        .await;
        client.revoke(&device).await.unwrap();
        let denied = hook(&executable, &data, host, json!({"hook_event_name":"UserPromptSubmit","session_id":"session-one","cwd":data,
            "turn_id":"turn-denied","prompt_id":"turn-denied","prompt":"Plugin fixture service port"})).await;
        assert!(
            denied["hookSpecificOutput"].is_null(),
            "Revoked access must not inject cached memory"
        );
        slot.forget().unwrap();
    }
    serving.abort();
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Owned plugin runtime, PostgreSQL and Secret Service; scripts/test-mcp-hosts.sh"]
async fn mcp_plugin_capture_learns_and_fresh_session_recalls_with_compaction() {
    use recollect_server::{autonomous, worker};
    assert_eq!(
        std::env::var("RECOLLECT_MCP_HOST_FIXTURE").as_deref(),
        Ok("1")
    );
    let mut h = Harness::new().await;
    let (provider, model_server) = crate::models::configure_provider(&mut h).await;
    let owner = h.login().await;
    let (reader_id, reader) = h.fixture_member().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let executable = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug/recollect-plugin")
        .canonicalize()
        .unwrap();
    for host in ["codex", "claude_code", "opencode"] {
        let brain = ok(
            &h,
            &owner,
            "POST",
            "/api/brains",
            json!({"name":format!("Plugin learning {host}")}),
        )
        .await;
        let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
        let base = format!("/api/brains/{brain}");
        crate::models::allow(&h, &owner, &base, |policy| {
            policy["autonomous_memory"] = json!(true);
            policy["purposes"] = json!(["extraction", "synthesis"]);
            policy["content_classes"] = json!(["raw_session", "claim", "query"]);
        })
        .await;
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
        let (device, token) = h.pair_device(&owner, &format!("Learning {host}")).await;
        let profile = format!("plugin-learning-{}", Uuid::new_v4());
        let slot = CredentialSlot::new(&endpoint, &profile).unwrap();
        slot.save(&StoredDevice {
            endpoint: endpoint.clone(),
            device_id: device,
            token: token.parse().unwrap(),
        })
        .unwrap();
        let data = PathBuf::from(&h.state.config.artifact_dir).join(format!("learning-{host}"));
        plugin_storage::directory(&data).unwrap();
        let config = Config {
            endpoint: endpoint.clone(),
            brain,
            device,
            profile,
            with_runner: false,
            runner_id: None,
        };
        plugin_storage::write(&data.join("config.json"), &config).unwrap();
        // There is no source upload or manually inserted claim. The native
        // runtime's documented prompt hook is the sole source of this fact.
        hook(
            &executable,
            &data,
            host,
            json!({"hook_event_name":"UserPromptSubmit","session_id":"learning-source","cwd":data,
            "host_version":"fixture-v1","turn_id":"source-turn","prompt_id":"source-turn","prompt":"Orchid.port = 7477"}),
        )
        .await;
        let until = tokio::time::Instant::now() + Duration::from_secs(20);
        let source: Uuid = loop {
            let source: Option<Uuid> = sqlx::query_scalar("SELECT source_version_id FROM capture_events WHERE brain_id=$1 AND source_version_id IS NOT NULL LIMIT 1")
                .bind(brain).fetch_optional(&h.admin).await.unwrap();
            if let Some(source) = source {
                break source;
            }
            assert!(
                tokio::time::Instant::now() < until,
                "Plugin capture must become canonical source evidence"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        };
        hook(
            &executable,
            &data,
            host,
            json!({"hook_event_name":"SessionEnd","session_id":"learning-source","cwd":data}),
        )
        .await;
        let source_session = recollect_agent::plugin_session::state_path(
            &data,
            &config,
            host,
            "learning-source",
            &data,
        )
        .unwrap();
        let source_session: recollect_agent::plugin_session::Session =
            plugin_storage::read(&source_session).unwrap();
        let until = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            let closed: bool = sqlx::query_scalar("SELECT closed FROM workspace_tasks WHERE id=$1")
                .bind(source_session.setup.task_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
            if closed {
                break;
            }
            assert!(
                tokio::time::Instant::now() < until,
                "Source session must close before background learning"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        while worker::run_once(&h.state, "capture").await.unwrap() {}
        *provider.candidates.lock().unwrap() = json!({"claims":[{"subject":"Orchid","predicate":"port","value":"7477",
            "rationale":"Declared in the captured user prompt.","line_from":1,"line_to":1,"replaces_revision":null}],"retirements":[]});
        assert!(
            autonomous::run_once(&h.state)
                .await
                .map_err(|e| e.1)
                .unwrap()
                > 0
        );
        while worker::run_once(&h.state, "model").await.unwrap() {}
        let runs = ok(&h, &owner, "GET", &format!("{base}/learning"), Value::Null).await;
        let learned = runs["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|run| run["source_version_id"] == json!(source))
            .unwrap();
        assert_eq!(learned["state"], "succeeded", "{learned}");
        assert_eq!(learned["accepted"], 1, "{learned}");
        let claim = learned["claim_ids"][0].as_str().unwrap();
        for event in ["UserPromptSubmit", "PostCompact"] {
            let result = hook(
                &executable,
                &data,
                host,
                json!({"hook_event_name":event,"session_id":"fresh-reader","cwd":data,
                "host_version":"fixture-v1","turn_id":"read-turn","prompt_id":"read-turn","prompt":"Orchid port"}),
            )
            .await;
            let context = result["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .expect("Automatic learned context");
            assert!(
                context.contains("7477")
                    && context.contains(claim)
                    && context.contains(&source.to_string()),
                "Learned claim and actual source citation must reach the fresh/compacted session: {context}"
            );
        }
        // A reader has no capture authority but still gets automatic memory.
        ok(
            &h,
            &owner,
            "PUT",
            &format!("{base}/grants/{reader_id}"),
            json!({"role":"reader"}),
        )
        .await;
        let (reader_device, reader_token) = h
            .pair_device(&reader, &format!("Plugin reader {host}"))
            .await;
        let reader_profile = format!("plugin-reader-{}", Uuid::new_v4());
        let reader_slot = CredentialSlot::new(&endpoint, &reader_profile).unwrap();
        reader_slot
            .save(&StoredDevice {
                endpoint: endpoint.clone(),
                device_id: reader_device,
                token: reader_token.parse().unwrap(),
            })
            .unwrap();
        let reader_data = data.join("reader");
        plugin_storage::directory(&reader_data).unwrap();
        let reader_config = Config {
            endpoint: endpoint.clone(),
            brain,
            device: reader_device,
            profile: reader_profile,
            with_runner: false,
            runner_id: None,
        };
        plugin_storage::write(&reader_data.join("config.json"), &reader_config).unwrap();
        let result = hook(&executable, &reader_data, host, json!({"hook_event_name":"UserPromptSubmit","session_id":"reader-only","cwd":reader_data,
            "host_version":"fixture-v1","turn_id":"reader-turn","prompt_id":"reader-turn","prompt":"Orchid port"})).await;
        assert!(
            result["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .is_some_and(|s| s.contains(claim))
        );
        let reader_session = recollect_agent::plugin_session::state_path(
            &reader_data,
            &reader_config,
            host,
            "reader-only",
            &reader_data,
        )
        .unwrap();
        let reader_session: recollect_agent::plugin_session::Session =
            plugin_storage::read(&reader_session).unwrap();
        assert!(
            reader_session.setup.binding_id.is_nil(),
            "Reader must not acquire capture authority"
        );
        reader_slot.forget().unwrap();
        slot.forget().unwrap();
    }
    serving.abort();
    model_server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Owned built plugin runtime; scripts/test-mcp-hosts.sh"]
async fn mcp_plugin_hook_deadline_includes_blocking_input() {
    assert_eq!(
        std::env::var("RECOLLECT_MCP_HOST_FIXTURE").as_deref(),
        Ok("1")
    );
    let binary =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/recollect-plugin");
    let data =
        PathBuf::from("/workspace/.cache").join(format!("plugin-deadline-{}", Uuid::new_v4()));
    let mut child = Command::new(binary)
        .args(["hook", "codex"])
        .env("RECOLLECT_PLUGIN_DATA", data)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let held_input = child.stdin.take().unwrap();
    let started = tokio::time::Instant::now();
    let output = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output())
        .await
        .expect("Synchronous input must not evade the hook deadline")
        .unwrap();
    drop(held_input);
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({})
    );
    assert!(started.elapsed() < Duration::from_secs(10));
}

async fn connect_token(
    executable: &Path,
    data: &Path,
    endpoint: &str,
    brain: Uuid,
    token: &str,
) -> std::process::Output {
    let mut child = Command::new(executable)
        .args([
            "connect",
            "--url",
            endpoint,
            "--brain",
            &brain.to_string(),
            "--token-stdin",
        ])
        .env("RECOLLECT_PLUGIN_DATA", data)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(token.as_bytes())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), child.wait_with_output())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Owned plugin runtime, PostgreSQL and Secret Service; scripts/test-mcp-hosts.sh"]
async fn mcp_plugin_reconnect_preserves_old_queued_authority_and_failed_setup() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    assert_eq!(
        std::env::var("RECOLLECT_MCP_HOST_FIXTURE").as_deref(),
        Ok("1")
    );
    let h = Harness::new().await;
    let owner = h.login().await;
    let unavailable = Arc::new(AtomicBool::new(true));
    let fence = unavailable.clone();
    let router = h.router.clone().layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let fence = fence.clone();
            async move {
                if request.uri().path().ends_with("/capture/events") && fence.load(Ordering::SeqCst)
                {
                    return axum::response::Response::builder()
                        .status(503)
                        .body(axum::body::Body::empty())
                        .unwrap();
                }
                next.run(request).await
            }
        },
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug/recollect-plugin")
        .canonicalize()
        .unwrap();
    let data = PathBuf::from(&h.state.config.artifact_dir).join("reconnect");
    let mut brains = Vec::new();
    for name in ["Original queue", "New connection"] {
        let brain = ok(&h, &owner, "POST", "/api/brains", json!({"name":name})).await;
        let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
        let path = format!("/api/brains/{brain}/capture/policy");
        let policy = ok(&h, &owner, "GET", &path, Value::Null).await;
        let mut enabled = policy["policy"].clone();
        enabled["enabled"] = json!(true);
        ok(
            &h,
            &owner,
            "PUT",
            &path,
            json!({"base_change":policy["change_id"],"policy":enabled}),
        )
        .await;
        brains.push(brain);
    }
    let (original_device, original_token) = h.pair_device(&owner, "Original plugin queue").await;
    assert!(
        connect_token(&binary, &data, &endpoint, brains[0], &original_token)
            .await
            .status
            .success()
    );
    let original: Config = plugin_storage::read(&data.join("config.json")).unwrap();
    hook(&binary,&data,"codex",json!({"hook_event_name":"UserPromptSubmit","host_version":"fixture-v1","session_id":"original","cwd":data,
        "turn_id":"queued","prompt":"Original independently captured evidence"})).await;
    let queued = recollect_agent::capture::Inbox::open(
        &recollect_agent::capture::profile_root(&data, original_device),
        &endpoint,
        original_device,
    )
    .unwrap();
    assert!(queued.status().unwrap().pending > 0);
    drop(queued);
    let (new_device, new_token) = h.pair_device(&owner, "Replacement plugin connection").await;
    // Failure after credential validation must leave the previous setup usable.
    assert!(
        !connect_token(&binary, &data, &endpoint, Uuid::new_v4(), &new_token)
            .await
            .status
            .success()
    );
    let unchanged: Config = plugin_storage::read(&data.join("config.json")).unwrap();
    assert_eq!(unchanged.device, original_device);
    let old_slot = CredentialSlot::new(&endpoint, &original.profile).unwrap();
    assert!(old_slot.load().unwrap().unwrap().token.to_string() == original_token);
    assert!(
        connect_token(&binary, &data, &endpoint, brains[1], &new_token)
            .await
            .status
            .success()
    );
    let current: Config = plugin_storage::read(&data.join("config.json")).unwrap();
    assert_eq!(current.device, new_device);
    assert_ne!(current.profile, original.profile);
    assert!(old_slot.load().unwrap().unwrap().token.to_string() == original_token);
    unavailable.store(false, Ordering::SeqCst);
    let until = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let count:i64 = sqlx::query_scalar("SELECT count(*) FROM capture_events e JOIN capture_bindings b ON b.id=e.binding_id WHERE e.brain_id=$1 AND b.device_id=$2 AND e.source_version_id IS NOT NULL")
            .bind(brains[0]).bind(original_device).fetch_one(&h.admin).await.unwrap();
        if count == 1 {
            break;
        }
        assert!(
            tokio::time::Instant::now() < until,
            "Old queued evidence must resume under its original device after reconnect"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let misplaced: i64 =
        sqlx::query_scalar("SELECT count(*) FROM capture_events WHERE brain_id=$1")
            .bind(brains[1])
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        misplaced, 0,
        "Changing connection must never move queued evidence"
    );
    old_slot.forget().unwrap();
    CredentialSlot::new(&endpoint, &current.profile)
        .unwrap()
        .forget()
        .unwrap();
    serving.abort();
    h.finish().await;
}
