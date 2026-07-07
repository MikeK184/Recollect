use super::*;
use recollect_agent::{Client, StoredDevice, mcp::LocalCoordinator};
use recollect_server::{mcp::runtime::observations, worker};

async fn policy(h: &Harness, owner: &Login, brain: Uuid, change: impl FnOnce(&mut Value)) {
    let path = format!("/api/brains/{brain}/capture/policy");
    let mut settings = ok(h, owner, "GET", &path, Value::Null).await;
    change(&mut settings["policy"]);
    ok(
        h,
        owner,
        "PUT",
        &path,
        json!({"base_change":settings["change_id"],"policy":settings["policy"]}),
    )
    .await;
}
pub(super) async fn enable(h: &Harness, owner: &Login, brain: Uuid) {
    policy(h, owner, brain, |p| {
        p["enabled"] = json!(true);
        p["managed_tools"] = json!(true);
    })
    .await;
}
async fn invoke(
    h: &Harness,
    owner: &Login,
    brain: Uuid,
    connection: &Value,
    profile: &Value,
    tool: &str,
    arguments: Value,
) -> Value {
    let path = format!("/api/brains/{brain}/mcp/calls");
    let mut input = body(connection, profile);
    input["tool_name"] = json!(tool);
    input["arguments"] = arguments;
    let call = ok(h, owner, "POST", &path, input.clone()).await;
    let result = finished(
        h,
        owner,
        &format!("{path}/{}", call["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(ok(h, owner, "POST", &path, input).await["id"], call["id"]);
    result
}
async fn events(h: &Harness, owner: &Login, brain: Uuid, call: &Value) -> Value {
    ok(
        h,
        owner,
        "GET",
        &format!(
            "/api/brains/{brain}/mcp/calls/{}/observations",
            call["id"].as_str().unwrap()
        ),
        Value::Null,
    )
    .await
}
async fn captured(h: &Harness, owner: &Login, brain: Uuid, event: &Value) -> Value {
    ok(
        h,
        owner,
        "GET",
        &format!(
            "/api/brains/{brain}/sources/{}/versions/{}",
            event["source_id"].as_str().unwrap(),
            event["source_version_id"].as_str().unwrap()
        ),
        Value::Null,
    )
    .await
}
async fn process(h: &Harness) {
    for _ in 0..30 {
        if !worker::run_once(&h.state, "capture").await.unwrap() {
            break;
        }
    }
}
async fn erase(h: &Harness, owner: &Login, brain: Uuid, source: &Value) {
    let base = format!("/api/brains/{brain}/erasures");
    let target = json!({"kind":"source","id":source});
    let preview = ok(h, owner, "POST", &format!("{base}/preview"), target.clone()).await;
    ok(
        h,
        owner,
        "POST",
        &base,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL and actual mcp-fixture"]
async fn managed_observations_actual_results_filtering_reconciliation_and_privacy() {
    let (h, owner, brain, connection, profile, marker) = execution_setup().await;
    let runner = central(&h).await;
    let lease = checked(runner.heartbeat(&h.state).await);
    let outbox = Outbox::open(
        PathBuf::from(&h.state.config.artifact_dir).join("receipts"),
        brain.to_string(),
    )
    .await
    .unwrap();
    let runtime = executor(&h, runner.clone(), outbox);
    let stop = CancellationToken::new();
    let running = tokio::spawn({
        let stop = stop.clone();
        async move { runtime.run(lease, stop).await }
    });
    // Enabling a standing policy must not import an already completed call.
    let disabled = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"disabled marker"}),
    )
    .await;
    assert_eq!(disabled["capture_disposition"], "disabled");
    enable(&h, &owner, brain).await;
    assert_eq!(events(&h, &owner, brain, &disabled).await["total"], 0);
    let call = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"ObservationAlphaprimary"}),
    )
    .await;
    assert_eq!(call["state"], "succeeded");
    assert_eq!(call["observations"][0]["state"], "pending");
    assert_eq!(checked(observations::run_once(&h.state).await), 1);
    let observation = events(&h, &owner, brain, &call).await["items"][0].clone();
    assert_eq!(observation["state"], "published", "{observation}");
    assert_eq!(observation["id"], call["id"]);
    let source = captured(&h, &owner, brain, &observation).await;
    let envelope: Value = serde_json::from_str(source["content"].as_str().unwrap()).unwrap();
    assert_eq!(envelope["call_id"], call["id"]);
    assert_eq!(
        envelope["selection"],
        json!({"repository_ids":[],"area_ids":[],"environment_id":null})
    );
    assert_eq!(
        envelope["result"]["structuredContent"]["arguments"]["text"],
        "ObservationAlphaprimary"
    );
    assert_eq!(
        envelope["target"]["connection_id"],
        connection["summary"]["id"]
    );
    assert_eq!(source["version"]["created_at"], observation["captured_at"]);
    assert_eq!(checked(observations::run_once(&h.state).await), 0);
    process(&h).await;
    let recall = ok(
        &h,
        &owner,
        "POST",
        &format!("/api/brains/{brain}/recall"),
        json!({"query":"ObservationAlphaprimary","limit":20}),
    )
    .await;
    assert!(
        recall["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["id"] == observation["source_version_id"]),
        "{recall}"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM model_requests")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    // A provider tool error is retained as an error observation, with its output.
    let error = invoke(&h, &owner, brain, &connection, &profile, "error", json!({})).await;
    checked(observations::run_once(&h.state).await);
    let error_event = events(&h, &owner, brain, &error).await["items"][0].clone();
    assert_eq!(error_event["outcome"], "tool_error");
    assert_eq!(error_event["state"], "published");
    assert!(
        captured(&h, &owner, brain, &error_event).await["content"]
            .as_str()
            .unwrap()
            .contains("Synthetic tool error")
    );

    policy(&h, &owner, brain, |p| {
        p["excluded_content"] = json!(["DO_NOT_CAPTURE"]);
        p["max_event_bytes"] = json!(1024);
    })
    .await;
    for text in [
        format!("{}DO_NOT_CAPTURE", "é".repeat(10000)),
        "read /tmp/.env".into(),
        "recollect-agent scope recall derived".into(),
    ] {
        let filtered = invoke(
            &h,
            &owner,
            brain,
            &connection,
            &profile,
            "inspect",
            json!({"text":text}),
        )
        .await;
        checked(observations::run_once(&h.state).await);
        let event = events(&h, &owner, brain, &filtered).await["items"][0].clone();
        assert_eq!(event["state"], "filtered", "{event}");
        assert!(event["source_id"].is_null());
        assert!(
            event["coverage"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "excluded_content" || v == "excluded_tool_content")
        );
    }
    let large = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"é".repeat(10000)}),
    )
    .await;
    checked(observations::run_once(&h.state).await);
    let large_event = events(&h, &owner, brain, &large).await["items"][0].clone();
    assert!(
        large_event["coverage"]
            .as_array()
            .unwrap()
            .contains(&json!("truncated"))
    );
    assert!(
        captured(&h, &owner, brain, &large_event).await["content"]
            .as_str()
            .unwrap()
            .len()
            <= 1024
    );
    policy(&h, &owner, brain, |p| p["max_event_bytes"] = json!(16384)).await;
    let pending = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"current policy denial"}),
    )
    .await;
    policy(&h, &owner, brain, |p| p["managed_tools"] = json!(false)).await;
    checked(observations::run_once(&h.state).await);
    enable(&h, &owner, brain).await;
    assert_eq!(
        events(&h, &owner, brain, &pending).await["items"][0]["state"],
        "filtered"
    );

    let effect = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "effect",
        json!({}),
    )
    .await;
    assert_eq!(effect["state"], "unknown");
    checked(observations::run_once(&h.state).await);
    let base = format!(
        "/api/brains/{brain}/mcp/calls/{}",
        effect["id"].as_str().unwrap()
    );
    let receipt = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/reconcile"),
        json!({"request_id":Uuid::new_v4(),"client_session_id":Uuid::new_v4()}),
    )
    .await;
    finished(
        &h,
        &owner,
        &format!(
            "/api/brains/{brain}/mcp/calls/{}",
            receipt["id"].as_str().unwrap()
        ),
    )
    .await;
    checked(observations::run_once(&h.state).await);
    let history = events(&h, &owner, brain, &effect).await;
    assert_eq!(history["total"], 2, "{history}");
    assert_eq!(history["items"][0]["outcome"], "unknown");
    assert_eq!(history["items"][1]["stage"], "connector_receipt");
    assert_eq!(history["items"][1]["outcome"], "succeeded");
    assert_eq!(history["items"][1]["state"], "published");
    assert_eq!(
        ok(&h, &owner, "GET", &base, Value::Null).await["state"],
        "unknown"
    );
    assert_eq!(
        tokio::fs::read_to_string(&marker)
            .await
            .unwrap()
            .lines()
            .filter(|s| s.starts_with("effect "))
            .count(),
        1
    );
    ok(&h,&owner,"POST",&format!("{base}/resolve"),json!({"request_id":Uuid::new_v4(),"outcome":"succeeded",
        "source_version_id":history["items"][1]["source_version_id"],"explanation":"RESOLUTION_EXPLANATION_MUST_NOT_BECOME_TOOL_OUTPUT"})).await;
    checked(observations::run_once(&h.state).await);
    let resolved = events(&h, &owner, brain, &effect).await["items"][2].clone();
    assert_eq!(resolved["stage"], "evidence");
    assert_eq!(resolved["state"], "published");
    let reference = captured(&h, &owner, brain, &resolved).await;
    assert!(
        !reference["content"]
            .as_str()
            .unwrap()
            .contains("RESOLUTION_EXPLANATION_MUST_NOT_BECOME_TOOL_OUTPUT")
    );
    assert!(
        reference["content"]
            .as_str()
            .unwrap()
            .contains(history["items"][1]["source_version_id"].as_str().unwrap())
    );
    erase(&h, &owner, brain, &history["items"][1]["source_id"]).await;
    let removed = events(&h, &owner, brain, &effect).await;
    assert!(
        removed["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["state"] == "removed")
    );
    let checked_ids = checked(
        runner
            .receipt_removals(
                &h.state,
                McpReceiptCheck {
                    call_ids: vec![effect["id"].as_str().unwrap().parse().unwrap()],
                },
            )
            .await,
    );
    assert_eq!(checked_ids.call_ids.len(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_call_payloads WHERE call_id=$1")
            .bind(checked_ids.call_ids[0])
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    assert!(
        captured(&h, &owner, brain, &observation).await["content"]
            .as_str()
            .unwrap()
            .contains("ObservationAlphaprimary")
    );
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    // Canonical pre-dispatch outcomes remain content-free observations.
    for cancel in [true, false] {
        let path = format!("/api/brains/{brain}/mcp/calls");
        let mut input = body(&connection, &profile);
        input["arguments"] = json!({});
        let queued = ok(&h, &owner, "POST", &path, input).await;
        if cancel {
            ok(
                &h,
                &owner,
                "POST",
                &format!("{path}/{}/cancel", queued["id"].as_str().unwrap()),
                Value::Null,
            )
            .await;
        } else {
            sqlx::query("UPDATE mcp_calls SET queue_expires_at=clock_timestamp()-interval '1 second' WHERE id=$1")
                .bind(queued["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
            checked(runner::maintain(&h.state).await);
        }
        checked(observations::run_once(&h.state).await);
        let observation = events(&h, &owner, brain, &queued).await["items"][0].clone();
        assert_eq!(
            observation["outcome"],
            if cancel { "cancelled" } else { "failed" }
        );
        assert!(observation["source_id"].is_null());
    }
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL and actual mcp-fixture"]
async fn managed_observations_artifact_retry_retention_and_current_authority() {
    let (h, owner, brain, connection, profile, _) = execution_setup().await;
    enable(&h, &owner, brain).await;
    let runner = central(&h).await;
    let lease = checked(runner.heartbeat(&h.state).await);
    let outbox = Outbox::open(
        PathBuf::from(&h.state.config.artifact_dir).join("receipts"),
        brain.to_string(),
    )
    .await
    .unwrap();
    let runtime = executor(&h, runner, outbox);
    let stop = CancellationToken::new();
    let running = tokio::spawn({
        let stop = stop.clone();
        async move { runtime.run(lease, stop).await }
    });
    let call = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"DurableObservation"}),
    )
    .await;
    let before = events(&h, &owner, brain, &call).await["items"][0].clone();
    let bad_root = PathBuf::from(&h.state.config.artifact_dir).join("unavailable-store");
    tokio::fs::write(
        &bad_root,
        "fixture: this is a file, not an artifact directory",
    )
    .await
    .unwrap();
    let mut config = (*h.state.config).clone();
    config.artifact_dir = bad_root.to_str().unwrap().into();
    let unavailable = AppState::new(h.state.pool.clone(), config).unwrap();
    checked(observations::run_once(&unavailable).await);
    let failed = events(&h, &owner, brain, &call).await["items"][0].clone();
    assert_eq!(failed["state"], "error", "{failed}");
    assert_eq!(failed["expires_at"], before["expires_at"]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM capture_events")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    sqlx::query("UPDATE mcp_observation_outbox SET next_attempt_at=clock_timestamp()")
        .execute(&h.admin)
        .await
        .unwrap();
    let restarted = AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap();
    checked(observations::run_once(&restarted).await);
    let published = events(&h, &owner, brain, &call).await["items"][0].clone();
    assert_eq!(published["state"], "published");
    assert_eq!(published["expires_at"], before["expires_at"]);
    assert_eq!(published["attempts"], 2);
    let held = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"preserve earlier pending evidence"}),
    )
    .await;
    let held_id: Uuid = held["id"].as_str().unwrap().parse().unwrap();
    let retention_path = format!("/api/brains/{brain}/retention");
    let mut retention = ok(&h, &owner, "GET", &retention_path, Value::Null).await;
    retention["policy"]["tool_output_days"] = json!(60);
    ok(
        &h,
        &owner,
        "PUT",
        &retention_path,
        json!({"base_change":retention["change_id"],"policy":retention["policy"]}),
    )
    .await;
    let extended = events(&h, &owner, brain, &call).await["items"][0].clone();
    let original_deadline: chrono::DateTime<chrono::Utc> =
        serde_json::from_value(published["expires_at"].clone()).unwrap();
    let extended_deadline: chrono::DateTime<chrono::Utc> =
        serde_json::from_value(extended["expires_at"].clone()).unwrap();
    assert_eq!(
        extended_deadline,
        original_deadline + chrono::Duration::days(30)
    );
    assert_eq!(extended["captured_at"], published["captured_at"]);
    assert_eq!(
        captured(&h, &owner, brain, &extended).await["version"]["expires_at"],
        extended["expires_at"]
    );
    let still_pending = events(&h, &owner, brain, &held).await["items"][0].clone();
    let pending_time: chrono::DateTime<chrono::Utc> =
        serde_json::from_value(still_pending["captured_at"].clone()).unwrap();
    let pending_deadline: chrono::DateTime<chrono::Utc> =
        serde_json::from_value(still_pending["expires_at"].clone()).unwrap();
    assert_eq!(pending_deadline, pending_time + chrono::Duration::days(30));
    // Materialize only the backlog precondition; the overflow attempt below is
    // an actual SDK call. Existing rows must survive, and no effect is retried.
    sqlx::query("INSERT INTO mcp_observation_outbox(id,call_id,brain_id,actor_id,device_id,stage,outcome,code,captured_at,expires_at,admission_policy)
        SELECT gen_random_uuid(),call_id,brain_id,actor_id,device_id,'terminal','unknown','fixture_backlog',captured_at,expires_at,admission_policy
        FROM mcp_observation_outbox CROSS JOIN generate_series(1,999) WHERE id=$1")
        .bind(held_id).execute(&h.admin).await.unwrap();
    let overflow = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"overflow must not displace prior evidence"}),
    )
    .await;
    assert_eq!(overflow["state"], "succeeded");
    let gap = events(&h, &owner, brain, &overflow).await["items"][0].clone();
    assert_eq!(gap["state"], "skipped");
    assert_eq!(gap["error_code"], "publication_capacity");
    assert_eq!(
        events(&h, &owner, brain, &held).await["items"][0]["state"],
        "pending"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM mcp_observation_outbox WHERE state IN ('pending','error')"
        )
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        1000
    );
    sqlx::query("DELETE FROM mcp_observation_outbox WHERE code='fixture_backlog'")
        .execute(&h.admin)
        .await
        .unwrap();
    checked(observations::run_once(&h.state).await);
    assert_eq!(
        events(&h, &owner, brain, &held).await["items"][0]["state"],
        "published"
    );
    let denied = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"revoke before publication"}),
    )
    .await;
    allow_owner(&h, &owner, brain, &profile, false).await;
    checked(observations::run_once(&h.state).await);
    assert_eq!(
        events(&h, &owner, brain, &denied).await["items"][0]["state"],
        "skipped"
    );
    allow_owner(&h, &owner, brain, &profile, true).await;
    let expired = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"expired pending output"}),
    )
    .await;
    let expired_id: Uuid = expired["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE mcp_observation_outbox SET expires_at=clock_timestamp()-interval '1 second' WHERE id=$1").bind(expired_id).execute(&h.admin).await.unwrap();
    checked(observations::run_once(&h.state).await);
    assert_eq!(
        events(&h, &owner, brain, &expired).await["items"][0]["state"],
        "expired"
    );
    assert!(sqlx::query_scalar::<_,bool>("SELECT event IS NULL AND admission_policy IS NULL FROM mcp_observation_outbox WHERE id=$1").bind(expired_id).fetch_one(&h.admin).await.unwrap());
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL and actual mcp-fixture"]
async fn managed_observations_local_private_original_scope_and_native_removal() {
    let (h, owner, brain, mut connection, profile, marker) = execution_setup().await;
    enable(&h, &owner, brain).await;
    let (device_id, token) = h.pair_device(&owner, "Observation runner").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let serving = tokio::spawn({
        let router = h.router.clone();
        async move { axum::serve(listener, router).await.unwrap() }
    });
    let device = StoredDevice {
        endpoint: endpoint.clone(),
        device_id,
        token: token.parse().unwrap(),
    };
    let client = Arc::new(Client::new(&endpoint).unwrap());
    let base = format!("/api/brains/{brain}");
    for host_event in ["ManagedTool", "ManagedReceipt", "ManagedResolution"] {
        let (status, _) = h
            .bearer(
                "POST",
                &format!("{base}/capture/events"),
                &token,
                json!({"id":Uuid::new_v4(),"binding_id":Uuid::new_v4(),"event":{
                "host_event":host_event,"host_session_id":"forged-managed-session",
                "turn_id":null,"agent_id":null,"tool_use_id":null,"tool_name":"inspect",
                "kind":"tool_result","outcome":"succeeded","content":"forged output",
                "coverage":[],"captured_at":chrono::Utc::now()}}),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let a = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/evidence/groups"),
        json!({"kind":"area","name":"Observation A"}),
    )
    .await;
    let b = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/evidence/groups"),
        json!({"kind":"area","name":"Observation B"}),
    )
    .await;
    let selection =
        |area: &Value| json!({"repository_ids":[],"area_ids":[area["id"]],"environment_id":null});
    for placement in ["local", "private"] {
        let private = if placement == "private" {
            let registered = ok(&h,&owner,"POST",&format!("{base}/mcp/private-runners"),
                json!({"name":"Observation private runner","device_id":device_id,"enabled":true,"base_revision":null})).await;
            Some(registered["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        } else {
            None
        };
        let mut edit = connection_body(None);
        edit["definition_key"] = json!("execution-fixture");
        edit["credential_alias"] = Value::Null;
        edit["configuration"] = json!({"marker":marker});
        edit["placement"] = json!(placement);
        edit["runner_reference"] = json!(
            private
                .map(|id| format!("private:{id}"))
                .unwrap_or_else(|| format!("device:{device_id}"))
        );
        edit["base_revision"] = connection["summary"]["revision"].clone();
        connection = ok(
            &h,
            &owner,
            "PUT",
            &format!(
                "{base}/mcp/connections/{}",
                connection["summary"]["id"].as_str().unwrap()
            ),
            edit,
        )
        .await;
        let (coordinator, lease) =
            LocalCoordinator::register_selected(client.clone(), &device, private)
                .await
                .unwrap();
        let unavailable = Arc::new(AtomicBool::new(false));
        let fault = ReceiptFault {
            inner: coordinator.clone(),
            unavailable: unavailable.clone(),
        };
        let outbox = Outbox::open(
            PathBuf::from(&h.state.config.artifact_dir).join(format!("{placement}-receipts")),
            format!("{brain}-{placement}"),
        )
        .await
        .unwrap();
        let runtime_outbox = Outbox::open(
            PathBuf::from(&h.state.config.artifact_dir).join(format!("{placement}-receipts")),
            format!("{brain}-{placement}"),
        )
        .await
        .unwrap();
        let runtime = Executor::new(
            fault,
            CredentialResolver::new(None),
            PathBuf::from(env!("CARGO_BIN_EXE_recollect-server")),
            runtime_outbox,
        );
        let stop = CancellationToken::new();
        let running = tokio::spawn({
            let stop = stop.clone();
            async move { runtime.run(lease, stop).await }
        });
        let (status, task) = h
            .bearer(
                "POST",
                &format!("{base}/workspace/tasks"),
                &token,
                json!({"label":"Original task","selection":selection(&a)}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{task}");
        let (status,child)=h.bearer("POST",&format!("{base}/workspace/tasks"),&token,
            json!({"label":"Independent child","parent_task_id":task["task"]["id"],"selection":selection(&b)})).await;
        assert_eq!(status, StatusCode::OK, "{child}");
        let mut calls = vec![];
        for (task, area) in [(&task, &a), (&child, &b)] {
            let (status, op) = h
                .bearer(
                    "POST",
                    &format!(
                        "{base}/workspace/tasks/{}/operations",
                        task["task"]["id"].as_str().unwrap()
                    ),
                    &token,
                    json!({"kind":"tool"}),
                )
                .await;
            assert_eq!(status, StatusCode::OK, "{op}");
            let mut input = body(&connection, &profile);
            input["operation_id"] = op["id"].clone();
            input["tool_name"] = json!("slow");
            input["arguments"] =
                json!({"millis":800,"text":format!("Scope observation {}",area["id"])});
            let (status, call) = h
                .bearer("POST", &format!("{base}/mcp/calls"), &token, input)
                .await;
            assert_eq!(status, StatusCode::OK, "{call}");
            calls.push((call, op, selection(area)));
        }
        let (status, changed) = h
            .bearer(
                "PUT",
                &format!(
                    "{base}/workspace/tasks/{}/scope",
                    task["task"]["id"].as_str().unwrap()
                ),
                &token,
                json!({"base_scope":task["task"]["scope"]["id"],"selection":selection(&b)}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{changed}");
        let mut published = vec![];
        for (call, op, selection) in &calls {
            let completed = finished(
                &h,
                &owner,
                &format!("{base}/mcp/calls/{}", call["id"].as_str().unwrap()),
            )
            .await;
            assert_eq!(completed["state"], "succeeded", "{completed}");
            checked(observations::run_once(&h.state).await);
            let event = events(&h, &owner, brain, call).await["items"][0].clone();
            assert_eq!(event["state"], "published", "{event}");
            let content = captured(&h, &owner, brain, &event).await;
            let envelope: Value =
                serde_json::from_str(content["content"].as_str().unwrap()).unwrap();
            assert_eq!(envelope["selection"], *selection);
            assert_eq!(envelope["operation_id"], op["id"]);
            assert_eq!(envelope["target"]["placement"], placement);
            assert_eq!(envelope["target"]["device_id"], json!(device_id));
            published.push(event);
        }
        if placement == "private" {
            let (reader_id, reader_login) = h.fixture_member().await;
            reader(&h, &owner, brain, reader_id).await;
            let username: String = sqlx::query_scalar("SELECT username FROM accounts WHERE id=$1")
                .bind(reader_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
            grant(
                &h,
                &owner,
                brain,
                &profile,
                &username,
                rights(true, false, false),
            )
            .await;
            let reader_call = invoke(
                &h,
                &reader_login,
                brain,
                &connection,
                &profile,
                "inspect",
                json!({"text":"reader execution only"}),
            )
            .await;
            assert_eq!(reader_call["state"], "succeeded");
            assert_eq!(
                reader_call["capture_disposition"],
                "knowledge_write_required"
            );
            assert_eq!(
                events(&h, &reader_login, brain, &reader_call).await["total"],
                0
            );
        }
        // Simulate a coordinator commit whose acknowledgement was lost. The
        // native outbox still holds the exact actual receipt after source Erase.
        let call_id: Uuid = calls[0].0["id"].as_str().unwrap().parse().unwrap();
        let receipt:sqlx::types::Json<McpCompletion>=sqlx::query_scalar("SELECT jsonb_build_object('attempt',jsonb_build_object(
            'brain_id',c.brain_id,'call_id',c.id,'epoch',c.runner_epoch,'attempt_token',c.attempt_token))||p.receipt
            FROM mcp_calls c JOIN mcp_call_payloads p ON p.call_id=c.id WHERE c.id=$1")
            .bind(call_id).fetch_one(&h.admin).await.unwrap();
        unavailable.store(true, Ordering::Release);
        outbox.store(&receipt.0, SystemTime::now()).await.unwrap();
        outbox.quarantine(&receipt.0).await.unwrap();
        erase(&h, &owner, brain, &published[0]["source_id"]).await;
        tokio::time::timeout(Duration::from_secs(5), async {
            while outbox.count(SystemTime::now()).await.unwrap() != 0 {
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
        })
        .await
        .expect("native removal check clears even quarantined receipt bodies");
        coordinator.complete(receipt.0).await.unwrap();
        checked(observations::run_once(&h.state).await);
        assert_eq!(
            events(&h, &owner, brain, &calls[0].0).await["items"][0]["state"],
            "removed"
        );
        assert_eq!(
            events(&h, &owner, brain, &calls[1].0).await["items"][0]["state"],
            "published"
        );
        stop.cancel();
        tokio::time::timeout(Duration::from_secs(12), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    serving.abort();
    h.finish().await;
}

async fn database_copy(h: &mut Harness) -> (String, sqlx::PgPool, AppState) {
    let name = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable observation restore fixture: {name}");
    h.state.pool.close().await;
    h.admin.close().await;
    // PostgreSQL template copies require every client backend to have left.
    // Terminate only this disposable, marked fixture after its pools/runners
    // have drained; never touch the normal development database or its clients.
    let owned:bool=sqlx::query_scalar("SELECT shobj_description(oid,'pg_database')='Recollect disposable integration test created by crates/server/tests/platform.rs' FROM pg_database WHERE datname=$1")
        .bind(&h.database).fetch_one(&h.root).await.unwrap();
    assert!(owned && h.database.starts_with("recollect_test_"));
    sqlx::query("SELECT pg_terminate_backend(pid,1000) FROM pg_stat_activity WHERE datname=$1 AND backend_type='client backend'")
        .bind(&h.database).execute(&h.root).await.unwrap();
    sqlx::query(&format!("CREATE DATABASE {name} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {name} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'"))
        .execute(&h.root).await.unwrap();
    let mut admin_url = reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
    admin_url.set_path(&h.database);
    h.admin = db::pool(admin_url.as_str()).await.unwrap();
    h.state = AppState::new(
        db::pool(&h.state.config.database_url).await.unwrap(),
        (*h.state.config).clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    admin_url.set_path(&name);
    let admin = db::pool(admin_url.as_str()).await.unwrap();
    let mut config = (*h.state.config).clone();
    let mut app_url = reqwest::Url::parse(&config.database_url).unwrap();
    app_url.set_path(&name);
    config.database_url = app_url.to_string();
    let state = AppState::new(db::pool(&config.database_url).await.unwrap(), config).unwrap();
    (name, admin, state)
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL and actual mcp-fixture"]
async fn managed_observations_journal_blocks_pending_and_missing_call_restores() {
    use recollect_server::privacy_journal;
    let (mut h, owner, brain, connection, profile, _) = execution_setup().await;
    enable(&h, &owner, brain).await;
    let retained=ok(&h,&owner,"POST",&format!("/api/brains/{brain}/sources"),
        json!({"title":"Independent retained control","media_type":"text/plain","retain_content":true,"content":"RestoreControlRetained"})).await;
    let early = database_copy(&mut h).await;
    let runner = central(&h).await;
    let lease = checked(runner.heartbeat(&h.state).await);
    let outbox = Outbox::open(
        PathBuf::from(&h.state.config.artifact_dir).join("restore-receipts"),
        brain.to_string(),
    )
    .await
    .unwrap();
    let runtime = executor(&h, runner.clone(), outbox);
    let stop = CancellationToken::new();
    let running = tokio::spawn({
        let stop = stop.clone();
        async move { runtime.run(lease, stop).await }
    });
    let call = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"MustNotReturnFromRestore"}),
    )
    .await;
    let id: Uuid = call["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        events(&h, &owner, brain, &call).await["items"][0]["state"],
        "pending"
    );
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let pending = database_copy(&mut h).await;
    checked(observations::run_once(&h.state).await);
    let event = events(&h, &owner, brain, &call).await["items"][0].clone();
    assert_eq!(event["state"], "published");
    erase(&h, &owner, brain, &event["source_id"]).await;
    privacy_journal::maintain(&h.state).await.unwrap();
    for (name, admin, state) in [early, pending] {
        assert!(
            privacy_journal::barrier(&state.pool, &state.config)
                .await
                .is_err(),
            "older database must not serve before replay"
        );
        privacy_journal::reconcile(&admin, &state.config)
            .await
            .unwrap();
        privacy_journal::barrier(&state.pool, &state.config)
            .await
            .unwrap();
        let removal = checked(
            Runner::central(Uuid::new_v4())
                .receipt_removals(&state, McpReceiptCheck { call_ids: vec![id] })
                .await,
        );
        assert_eq!(
            removal.call_ids,
            vec![id],
            "including a restore older than the call itself"
        );
        assert_eq!(checked(observations::run_once(&state).await), 0);
        let bodies:i64=sqlx::query_scalar("SELECT count(*) FROM mcp_observation_outbox WHERE call_id=$1 AND (event IS NOT NULL OR admission_policy IS NOT NULL)")
            .bind(id).fetch_one(&admin).await.unwrap();
        assert_eq!(bodies, 0);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_call_payloads WHERE call_id=$1")
                .bind(id)
                .fetch_one(&admin)
                .await
                .unwrap(),
            0
        );
        let control:(Uuid,i32)=sqlx::query_as("SELECT artifact_id,byte_length FROM source_versions WHERE id=$1 AND privacy_state='active'")
            .bind(retained["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&admin).await.unwrap();
        assert_eq!(
            recollect_server::artifacts::read(
                &state.config.artifact_dir,
                brain,
                control.0,
                control.1
            )
            .await
            .unwrap(),
            "RestoreControlRetained"
        );
        state.pool.close().await;
        admin.close().await;
        sqlx::query(&format!("DROP DATABASE {name} WITH (FORCE)"))
            .execute(&h.root)
            .await
            .unwrap();
    }
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL and actual mcp-fixture"]
async fn managed_observations_automatic_learning_keeps_receipt_provenance() {
    let (mut h, owner, brain, connection, profile, _) = execution_setup().await;
    let (provider, provider_server) = crate::models::configure_provider(&mut h).await;
    enable(&h, &owner, brain).await;
    let base = format!("/api/brains/{brain}");
    let mut model_policy = ok(
        &h,
        &owner,
        "GET",
        &format!("{base}/models/policy"),
        Value::Null,
    )
    .await;
    let settings = &mut model_policy["current"]["policy"];
    settings["enabled"] = json!(true);
    settings["automatic_learning"] = json!(true);
    settings["autonomous_memory"] = json!(true);
    settings["purposes"] = json!(["extraction", "synthesis"]);
    settings["content_classes"] = json!(["tool_output", "claim", "query", "document"]);
    ok(&h,&owner,"PUT",&format!("{base}/models/policy"),
        json!({"base_change":model_policy["current"]["change_id"],"policy":model_policy["current"]["policy"]})).await;
    let runner = central(&h).await;
    let lease = checked(runner.heartbeat(&h.state).await);
    let outbox = Outbox::open(
        PathBuf::from(&h.state.config.artifact_dir).join("learning-receipts"),
        brain.to_string(),
    )
    .await
    .unwrap();
    let runtime = executor(&h, runner, outbox);
    let stop = CancellationToken::new();
    let running = tokio::spawn({
        let stop = stop.clone();
        async move { runtime.run(lease, stop).await }
    });
    let call = invoke(
        &h,
        &owner,
        brain,
        &connection,
        &profile,
        "inspect",
        json!({"text":"Amber.port = 8080"}),
    )
    .await;
    checked(observations::run_once(&h.state).await);
    let event = events(&h, &owner, brain, &call).await["items"][0].clone();
    let source = captured(&h, &owner, brain, &event).await;
    let line = source["content"]
        .as_str()
        .unwrap()
        .lines()
        .position(|v| v.contains("Amber.port"))
        .unwrap()
        + 1;
    *provider.candidates.lock().unwrap() = json!({"claims":[{"subject":"Amber","predicate":"port","value":"8080",
        "rationale":"Reported by this managed fixture observation.","line_from":line,"line_to":line,"replaces_revision":null}],"retirements":[]});
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        0,
        "capture never invokes a model"
    );
    process(&h).await;
    assert_eq!(
        checked(recollect_server::autonomous::run_once(&h.state).await),
        1
    );
    assert!(worker::run_once(&h.state, "model").await.unwrap());
    let run =
        ok(&h, &owner, "GET", &format!("{base}/learning"), Value::Null).await["items"][0].clone();
    assert_eq!(run["state"], "succeeded", "{run}");
    assert_eq!(run["automatic"], true);
    assert_eq!(run["source_version_id"], event["source_version_id"]);
    assert_eq!(run["claim_ids"].as_array().unwrap().len(), 1);
    let input: Value = serde_json::from_str(
        provider.bodies.lock().unwrap().last().unwrap()["input"]
            .as_str()
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(input["provenance"]["role"], "reported_tool_observation");
    assert_eq!(
        input["provenance"]["capture"]["managed_call_id"],
        call["id"]
    );
    assert_eq!(
        input["provenance"]["capture"]["managed_target"]["connection_id"],
        connection["summary"]["id"]
    );
    assert_eq!(input["provenance"]["capture"]["outcome"], "succeeded");
    let claim = ok(
        &h,
        &owner,
        "GET",
        &format!("{base}/claims/{}", run["claim_ids"][0].as_str().unwrap()),
        Value::Null,
    )
    .await;
    assert!(claim["selected"]["revision"]["reviewer_id"].is_null());
    assert_eq!(
        claim["selected"]["eligibility"]["strict_operational"],
        false
    );
    erase(&h, &owner, brain, &event["source_id"]).await;
    assert_eq!(
        checked(recollect_server::autonomous::run_once(&h.state).await),
        0,
        "deleted observations cannot be learned again"
    );
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    provider_server.abort();
    h.finish().await;
}
