use super::*;
use crate::mcp::tools::tool;
use recollect_agent::{CredentialSlot, StoredDevice};
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};

fn native_environment(command: &mut tokio::process::Command) {
    command.env_clear();
    // The owned Linux fixture's actual OS store is on its private session bus.
    // Preserve that address only; do not inherit provider or root credentials.
    if let Some(address) = std::env::var_os("DBUS_SESSION_BUS_ADDRESS") {
        command.env("DBUS_SESSION_BUS_ADDRESS", address);
    }
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL, native binaries, OS credential store and mcp-fixture"]
async fn mcp_agent_native_scope_capture_and_managed_receipts() {
    let (h, owner, brain, connection, profile, marker) = execution_setup().await;
    let (device_id, token) = h.pair_device(&owner, "Native agent bridge proof").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let device = StoredDevice {
        endpoint: endpoint.clone(),
        device_id,
        token: token.parse().unwrap(),
    };
    let profile_name = format!("bridge-proof-{}", Uuid::new_v4());
    let slot = CredentialSlot::new(&endpoint, &profile_name).unwrap();
    assert!(slot.load().unwrap().is_none());
    slot.save(&device).unwrap();
    struct Forget(CredentialSlot);
    impl Drop for Forget {
        fn drop(&mut self) {
            let _ = self.0.forget();
        }
    }
    let _saved = Forget(slot);
    let executable =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/recollect-agent");
    let work = PathBuf::from(&h.state.config.artifact_dir).join("bridge-work");
    tokio::fs::create_dir_all(work.join(".recollect"))
        .await
        .unwrap();
    tokio::fs::write(
        work.join(".recollect/workspace.toml"),
        format!("brain = \"{brain}\"\n"),
    )
    .await
    .unwrap();
    let mut configuration = tokio::process::Command::new(&executable);
    native_environment(&mut configuration);
    let output = tokio::time::timeout(
        Duration::from_secs(120),
        configuration
            .env("RECOLLECT_URL", &endpoint)
            .env("RECOLLECT_DEVICE_PROFILE", &profile_name)
            .args([
                "mcp-config",
                "codex",
                "--brain",
                &brain.to_string(),
                "--directory",
                work.to_str().unwrap(),
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("bounded native configuration startup")
    .unwrap();
    assert!(
        output.status.success(),
        "config generation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output
            .stdout
            .windows(token.len())
            .any(|v| v == token.as_bytes())
    );
    let generated: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        generated["configuration"]
            .as_str()
            .unwrap()
            .contains("mcp_servers.recollect")
    );
    let base = format!("/api/brains/{brain}");
    let a = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/evidence/groups"),
        json!({"kind":"area","name":"Scope A"}),
    )
    .await;
    let b = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/evidence/groups"),
        json!({"kind":"area","name":"Scope B"}),
    )
    .await;
    let selection =
        |area: &Value| json!({"repository_ids":[],"area_ids":[area["id"]],"environment_id":null});
    let (status, original) = h
        .bearer(
            "POST",
            &format!("{base}/workspace/tasks"),
            &token,
            json!({"label":"Original managed capture","selection":selection(&a)}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let mut command = tokio::process::Command::new(&executable);
    native_environment(&mut command);
    command
        .env("RECOLLECT_URL", &endpoint)
        .env("RECOLLECT_DEVICE_PROFILE", &profile_name)
        .args([
            "mcp-serve",
            "--brain",
            &brain.to_string(),
            "--directory",
            work.to_str().unwrap(),
        ]);
    let bridge = tokio::time::timeout(
        Duration::from_secs(120),
        ().serve(TokioChildProcess::new(command).unwrap()),
    )
    .await
    .unwrap()
    .unwrap();
    let peer = bridge.peer();
    assert!(
        peer.list_all_tools()
            .await
            .unwrap()
            .iter()
            .any(|t| t.name == "workspace.refresh")
    );
    let refreshed = tool(peer, "workspace.refresh", json!({})).await;
    assert_eq!(refreshed["refresh"]["workspace"]["brain_id"], json!(brain));
    let discovery = tool(
        peer,
        "mcp.discover",
        json!({"operation_id":tool(peer,"workspace.begin",
        json!({"id":original["task"]["id"],"input":{"kind":"context"}})).await["id"],
        "input":{"profile_id":profile["profile"]["id"]}}),
    )
    .await;
    assert!(!discovery["tools"].as_array().unwrap().is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_instances")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    let child = tool(peer,"workspace.start_task",json!({"input":{"label":"Independent child", "parent_task_id":original["task"]["id"],"selection":selection(&b)},"context_query":"Native scope"})).await;
    let mut claims = Vec::new();
    for (task, area, value) in [
        (&original, &a, "SCOPE_A_CONTEXT"),
        (&child, &b, "SCOPE_B_CONTEXT"),
    ] {
        let source = ok(
            &h,
            &owner,
            "POST",
            &format!("{base}/sources"),
            json!({"title":"Native scope evidence","media_type":"text/plain",
                "content":format!("Native scope uses {value}."),"retain_content":true,
                "source_uri":null,"group_ids":[area["id"]]}),
        )
        .await;
        let operation = tool(
            peer,
            "workspace.begin",
            json!({"id":task["task"]["id"],"input":{"kind":"write"}}),
        )
        .await;
        let mut input = crate::review::proposal(&source["version"]["id"], "Native scope", value);
        input.as_object_mut().unwrap().remove("operation_id");
        input["content"]
            .as_object_mut()
            .unwrap()
            .remove("selection");
        claims.push(
            tool(
                peer,
                "memory.contribute",
                json!({"operation_id":operation["id"],"request_id":Uuid::new_v4(),"input":input}),
            )
            .await,
        );
    }
    let old_read = tool(
        peer,
        "workspace.begin",
        json!({"id":original["task"]["id"],"input":{"kind":"retrieval"}}),
    )
    .await;
    let before = tool(
        peer,
        "memory.recall",
        json!({"operation_id":old_read["id"],"input":{"query":"Native scope"}}),
    )
    .await;
    assert!(
        before.to_string().contains("SCOPE_A_CONTEXT")
            && !before.to_string().contains("SCOPE_B_CONTEXT")
    );
    let changed = tool(peer,"workspace.set_scope",json!({"id":original["task"]["id"],"input":{
        "base_scope":original["task"]["scope"]["id"],"selection":selection(&b)},"context_query":"Native scope"})).await;
    assert_eq!(changed["context"]["state"], "ready", "{changed}");
    assert!(
        changed["context"].to_string().contains("SCOPE_B_CONTEXT")
            && !changed["context"].to_string().contains("SCOPE_A_CONTEXT")
    );
    assert_eq!(
        tool(
            peer,
            "workspace.inspect_task",
            json!({"id":child["task"]["id"]})
        )
        .await["task"]["scope"],
        child["task"]["scope"]
    );
    let old_result = tool(
        peer,
        "memory.recall",
        json!({"operation_id":old_read["id"],"input":{"query":"Native scope"}}),
    )
    .await;
    assert!(
        old_result.to_string().contains("SCOPE_A_CONTEXT")
            && !old_result.to_string().contains("SCOPE_B_CONTEXT")
    );
    let runner = Runner::central(Uuid::new_v4());
    let lease = checked(runner.register(&h.state).await);
    let worker = executor(
        &h,
        runner,
        Outbox::open(
            PathBuf::from(&h.state.config.artifact_dir).join("bridge-receipts"),
            brain.to_string(),
        )
        .await
        .unwrap(),
    );
    let stop = CancellationToken::new();
    let running_stop = stop.clone();
    let running = tokio::spawn(async move { worker.run(lease, running_stop).await });
    let operation = tool(
        peer,
        "workspace.begin",
        json!({"id":original["task"]["id"],"input":{"kind":"tool"}}),
    )
    .await;
    let mut input = body(&connection, &profile);
    input.as_object_mut().unwrap().remove("environment_id");
    input.as_object_mut().unwrap().remove("operation_id");
    input["arguments"] = json!({"text":"Native managed bridge"});
    let args = json!({"operation_id":operation["id"],"input":input});
    let call = tool(peer, "mcp.call", args.clone()).await;
    assert_eq!(tool(peer, "mcp.call", args).await["id"], call["id"]);
    let path = format!("{base}/mcp/calls/{}", call["id"].as_str().unwrap());
    let final_call = finished(&h, &owner, &path).await;
    assert_eq!(final_call["state"], "succeeded", "{final_call}");
    assert_eq!(final_call["scope"], operation["scope"]);
    assert_eq!(
        tool(
            peer,
            "mcp.status",
            json!({"operation_id":operation["id"],"id":call["id"]})
        )
        .await["state"],
        "succeeded"
    );
    input["request_id"] = json!(Uuid::new_v4());
    input["tool_name"] = json!("effect");
    input["arguments"] = json!({});
    let effect = tool(
        peer,
        "mcp.call",
        json!({"operation_id":operation["id"],"input":input}),
    )
    .await;
    let effect_path = format!("{base}/mcp/calls/{}", effect["id"].as_str().unwrap());
    assert_eq!(finished(&h, &owner, &effect_path).await["state"], "unknown");
    let receipt = tool(
        peer,
        "mcp.reconcile",
        json!({"operation_id":operation["id"],"id":effect["id"],
        "input":{"request_id":Uuid::new_v4(),"client_session_id":Uuid::new_v4()}}),
    )
    .await;
    let receipt_path = format!("{base}/mcp/calls/{}", receipt["id"].as_str().unwrap());
    assert_eq!(
        finished(&h, &owner, &receipt_path).await["state"],
        "succeeded"
    );
    let effects = tokio::fs::read_to_string(&marker).await.unwrap();
    assert_eq!(
        effects
            .lines()
            .filter(|line| *line == "call effect")
            .count(),
        1,
        "receipt lookup does not replay the effect call"
    );
    assert_eq!(
        effects
            .lines()
            .filter(|line| *line == "call receipt")
            .count(),
        1
    );
    let recorded_effect = format!("effect {}", effect["id"].as_str().unwrap());
    assert_eq!(
        effects
            .lines()
            .filter(|line| *line == recorded_effect)
            .count(),
        1,
        "the original operation produced exactly one recorded effect"
    );
    input["request_id"] = json!(Uuid::new_v4());
    input["tool_name"] = json!("slow");
    input["arguments"] = json!({"millis":5000});
    let slow = tool(
        peer,
        "mcp.call",
        json!({"operation_id":operation["id"],"input":input}),
    )
    .await;
    let cancelled = tool(
        peer,
        "mcp.cancel",
        json!({"operation_id":operation["id"],"id":slow["id"]}),
    )
    .await;
    assert_eq!(cancelled["cancel_requested"], true);
    let cancelled = finished(
        &h,
        &owner,
        &format!("{base}/mcp/calls/{}", slow["id"].as_str().unwrap()),
    )
    .await;
    assert!(
        matches!(cancelled["state"].as_str(), Some("cancelled" | "unknown")),
        "{cancelled}"
    );
    let denied = peer
        .call_tool(
            CallToolRequestParams::new("mcp.status").with_arguments(
                json!({"operation_id":old_read["id"],"id":call["id"]})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(denied.is_error, Some(true));
    stop.cancel();
    running.await.unwrap().unwrap();
    // Actual loss of the fixture's application DB pool makes the remote service
    // unavailable. The native bridge must fail explicitly without driver details.
    h.state.pool.close().await;
    let unavailable = tokio::time::timeout(Duration::from_secs(5), peer.list_all_tools())
        .await
        .unwrap()
        .unwrap_err()
        .to_string();
    assert!(unavailable.contains("Recollect is unavailable or authority changed"));
    assert!(!unavailable.contains("pool closed"));
    bridge.cancel().await.unwrap();
    serving.abort();
    let _ = serving.await;
    h.finish().await;
}
