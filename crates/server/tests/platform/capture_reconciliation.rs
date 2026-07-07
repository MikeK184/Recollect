use super::*;

#[path = "capture_publication.rs"]
mod publication;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn recall_capture_source_groups_use_binding_session_agent_and_original_version() {
    let (h, owner, base, p, server) = setup().await;
    capture_policy(&h, &owner, &base, true).await;
    let (_, token) = h.pair_device(&owner, "Recall capture source groups").await;
    let first = binding(&h, &base, &token, json!({})).await;
    let second = binding(&h, &base, &token, json!({})).await;
    let mut receipts = Vec::new();
    for (i, bound, agent) in [
        (0, &first, Value::Null),
        (1, &first, Value::Null),
        (2, &second, Value::Null),
        (3, &first, json!("child")),
    ] {
        let mut input = event(
            bound,
            &format!("group-{i}"),
            "Lineagecaptures retained evidence.\n",
        );
        input["event"]["agent_id"] = agent;
        receipts
            .push(device_ok(&h, "POST", &format!("{base}/capture/events"), &token, input).await);
    }
    source(&h, &owner, &base, "Lineagecaptures independent document.").await;
    process(&h).await;
    let query = json!({"query":"Lineagecaptures","context_bytes":32768,"limit":20});
    let current = recalled(&h, &owner, &base, query.clone()).await;
    assert_eq!(current["context"]["items"].as_array().unwrap().len(), 5);
    assert_eq!(
        current["context_selection"]["distinct_source_groups"], 4,
        "Same session spelling in different bindings or agents is not one source"
    );
    assert_eq!(current["context_selection"]["unknown_lineage_items"], 0);
    let original = &receipts[0];
    let later=ok(&h,"POST",&format!("{base}/sources/{}/versions",original["source_id"].as_str().unwrap()),&owner,
        json!({"base_version":original["source_version_id"],"title":"Appended ordinary source","media_type":"text/plain","retain_content":true,"content":"Lineagecaptures appended ordinary version.\n"})).await;
    process(&h).await;
    let appended = recalled(&h, &owner, &base, query.clone()).await;
    assert!(contains(&appended, &later["version"]["id"]));
    assert!(!contains(&appended, &original["source_version_id"]));
    assert_eq!(
        appended["context_selection"]["distinct_source_groups"], 5,
        "Later versions do not inherit the original capture identity"
    );
    let retained=ok(&h,"POST",&format!("{base}/excerpts"),&owner,json!({"source_id":receipts[1]["source_id"],"version_id":receipts[1]["source_version_id"],"first_line":1,"last_line":1,"title":"Independent retained capture excerpt"})).await;
    process(&h).await;
    let event_id = receipts[1]["event_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    let version = receipts[1]["source_version_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    sqlx::query(
        "UPDATE capture_events SET captured_at=clock_timestamp()-interval '1000 days' WHERE id=$1",
    )
    .bind(event_id)
    .execute(&h.admin)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '1000 days' WHERE id=$1",
    )
    .bind(version)
    .execute(&h.admin)
    .await
    .unwrap();
    let expired = recalled(&h, &owner, &base, query).await;
    assert!(contains(&expired, &retained["version"]["id"]));
    assert!(!contains(&expired, &receipts[1]["source_version_id"]));
    assert_eq!(expired["context_selection"]["distinct_source_groups"], 5);
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

fn contains(result: &Value, id: &Value) -> bool {
    result["context"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == *id)
}
async fn recalled(h: &Harness, owner: &Login, base: &str, input: Value) -> Value {
    ok(h, "POST", &format!("{base}/recall"), owner, input).await
}
async fn server_time(h: &Harness) -> chrono::DateTime<Utc> {
    sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&h.admin)
        .await
        .unwrap()
}

fn extracted(subject: &str, value: &str, replaces: Value, line: i32) -> Value {
    let mut claim = candidate(subject, "port", value, line);
    claim["replaces_revision"] = replaces;
    json!({"claims":[claim],"retirements":[]})
}
fn provider_inputs(p: &Provider) -> Vec<Value> {
    p.bodies.lock().unwrap().last().unwrap()["input"]
        .as_str()
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
async fn learn_capture(h: &Harness, owner: &Login, base: &str, token: &str, input: Value) -> Value {
    device_ok(h, "POST", &format!("{base}/capture/events"), token, input).await;
    process(h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    let restarted = AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap();
    assert!(worker::run_once(&restarted, "model").await.unwrap());
    let result = runs(h, owner, base).await["items"][0].clone();
    assert_eq!(result["state"], "succeeded", "{result}");
    result
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn bound_capture_reconciliation_preserves_provenance_scope_overrides_and_erasure() {
    let (h, owner, base, p, server) = setup().await;
    capture_policy(&h, &owner, &base, true).await;
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
        policy["content_classes"] =
            json!(["raw_session", "tool_output", "claim", "query", "document"]);
    })
    .await;
    let (_, token) = h.pair_device(&owner, "Session reconciliation").await;
    let scope = json!({"repository_ids":[],"area_ids":[],"environment_id":null});
    let bound = binding(&h, &base, &token, scope.clone()).await;
    *p.candidates.lock().unwrap() = extracted("Amber", "8080", Value::Null, 1);
    let initial_event = event(&bound, "first", "Amber.port = 8080\n");
    let initial = learn_capture(&h, &owner, &base, &token, initial_event.clone()).await;
    let id = initial["claim_ids"][0].as_str().unwrap();
    let claim_url = format!("{base}/claims/{id}");
    let first =
        ok(&h, "GET", &claim_url, &owner, Value::Null).await["selected"]["revision"].clone();
    let input = &provider_inputs(&p)[0];
    assert_eq!(input["data"], "Amber.port = 8080\n");
    assert_eq!(input["provenance"]["role"], "user_assertion");
    assert_eq!(
        input["provenance"]["capture"]["event_id"],
        initial_event["id"]
    );
    assert_eq!(input["provenance"]["capture"]["binding_id"], bound["id"]);
    assert_eq!(input["provenance"]["capture"]["selection"], scope);
    assert!(first["reviewer_id"].is_null());

    // Same session spelling cannot broaden the authenticated operation, scope
    // or child attribution. These inputs are legitimate evidence, but have no
    // authority to revise the first claim through session reconciliation.
    let other_bound = binding(&h, &base, &token, scope.clone()).await;
    let environment = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Other environment"}),
    )
    .await;
    let other_environment = binding(
        &h,
        &base,
        &token,
        json!({"repository_ids":[],"area_ids":[],"environment_id":environment["id"]}),
    )
    .await;
    for (turn, binding, session, agent) in [
        (
            "other-operation",
            &other_bound,
            "synthetic-session",
            Value::Null,
        ),
        ("other-session", &bound, "another-session", Value::Null),
        (
            "other-child",
            &bound,
            "synthetic-session",
            json!("child-agent"),
        ),
        (
            "other-environment",
            &other_environment,
            "synthetic-session",
            Value::Null,
        ),
    ] {
        let mut unrelated = event(
            binding,
            turn,
            "Separate context mentions Amber.port = 8080\n",
        );
        unrelated["event"]["host_session_id"] = json!(session);
        unrelated["event"]["agent_id"] = agent;
        *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
        let result = learn_capture(&h, &owner, &base, &token, unrelated).await;
        assert!(
            result["reconciliation_inputs"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{turn}: {result}"
        );
        assert_eq!(provider_inputs(&p).len(), 1);
    }
    for (turn, role, content) in [
        (
            "assistant-proposal",
            "reply",
            "I suggest changing Amber to port 9090; I have not changed or tested it.\n",
        ),
        (
            "historical-quote",
            "prompt",
            "Historical quotation: Amber used port 9090 last year. This is not a current correction.\n",
        ),
    ] {
        let mut ambiguous = event(&bound, turn, content);
        if role == "reply" {
            ambiguous["event"]["host_event"] = json!("Stop");
            ambiguous["event"]["kind"] = json!("reply");
        }
        *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
        let result = learn_capture(&h, &owner, &base, &token, ambiguous).await;
        assert_eq!(result["reconciliation_inputs"], json!([first["id"]]));
        let inputs = provider_inputs(&p);
        assert_eq!(inputs[0]["data"], content);
        assert_eq!(
            inputs[0]["provenance"]["role"],
            if role == "reply" {
                "assistant_statement"
            } else {
                "user_assertion"
            }
        );
        assert_eq!(
            ok(&h, "GET", &claim_url, &owner, Value::Null).await["selected"]["revision"]["id"],
            first["id"]
        );
    }
    let tool_text = serde_json::to_string_pretty(&json!({"input":{"command":"inspect synthetic declaration"},
        "output":{"stdout":"Amber.port = 9090; explicitly replaces the previous 8080 declaration."}})).unwrap();
    let mut correction = event(&bound, "tool-observation", &tool_text);
    correction["event"]["host_event"] = json!("PostToolUse");
    correction["event"]["kind"] = json!("tool_result");
    correction["event"]["tool_name"] = json!("Bash");
    correction["event"]["tool_use_id"] = json!("synthetic-inspection");
    let line = tool_text
        .lines()
        .position(|line| line.contains("Amber.port"))
        .unwrap() as i32
        + 1;
    *p.candidates.lock().unwrap() = extracted("Amber", "9090", first["id"].clone(), line);
    let result = learn_capture(&h, &owner, &base, &token, correction).await;
    assert_eq!(result["claim_ids"], json!([id]));
    assert_eq!(result["revised"], 1);
    let inputs = provider_inputs(&p);
    assert_eq!(inputs[0]["data"], tool_text);
    assert_eq!(inputs[0]["provenance"]["role"], "reported_tool_observation");
    assert_eq!(inputs[0]["provenance"]["capture"]["outcome"], "reported");
    assert_eq!(inputs[0]["provenance"]["capture"]["tool_name"], "Bash");
    let changed = ok(&h, "GET", &claim_url, &owner, Value::Null).await;
    let second = changed["selected"]["revision"].clone();
    assert_eq!(second["content"]["value"], "9090");
    assert_eq!(changed["selected"]["eligibility"]["strict_accepted"], true);
    assert_eq!(
        changed["selected"]["eligibility"]["strict_operational"],
        false
    );
    assert_eq!(second["content"]["supports"][0]["line_from"], line);
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );

    // A human override arriving during the external call invalidates its offered
    // input. This is an optional override test, not the routine acceptance path.
    device_ok(
        &h,
        "POST",
        &format!("{base}/capture/events"),
        &token,
        event(&bound, "in-flight", "Amber.port = 7070; replaces 9090.\n"),
    )
    .await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    *p.candidates.lock().unwrap() = extracted("Amber", "7070", second["id"].clone(), 1);
    p.delay.store(300, Ordering::SeqCst);
    let next_call = p.calls.load(Ordering::SeqCst) + 1;
    let state = h.state.clone();
    let work = tokio::spawn(async move { worker::run_once(&state, "model").await });
    wait_calls(&p, next_call).await;
    let override_result = ok(&h, "POST", &format!("{claim_url}/review"), &owner,
        json!({"base_revision":second["id"],"action":"revalidate","revalidation_basis":"review_correction","reason":"Optional synthetic human override."})).await;
    work.await.unwrap().unwrap();
    p.delay.store(0, Ordering::SeqCst);
    assert_eq!(runs(&h, &owner, &base).await["items"][0]["state"], "failed");
    let reviewed = override_result["claims"][0]["revision"].clone();
    assert_eq!(
        ok(&h, "GET", &claim_url, &owner, Value::Null).await["selected"]["revision"]["id"],
        reviewed["id"]
    );
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
    let after_override = learn_capture(
        &h,
        &owner,
        &base,
        &token,
        event(
            &bound,
            "after-override",
            "Another statement proposes port 6060.\n",
        ),
    )
    .await;
    assert!(
        after_override["reconciliation_inputs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    ok(&h, "POST", &format!("{claim_url}/review"), &owner,
        json!({"base_revision":reviewed["id"],"action":"reject","reason":"Reject this synthetic declaration."})).await;
    *p.candidates.lock().unwrap() = extracted("Amber", "9090", Value::Null, 1);
    let blocked = learn_capture(
        &h,
        &owner,
        &base,
        &token,
        event(&bound, "rejected-repeat", "Amber.port = 9090\n"),
    )
    .await;
    assert!(
        blocked["reconciliation_inputs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(blocked["blocked"], 1);
    assert_eq!(blocked["accepted"], 0);

    // Prove an eligible target and its reconciliation dependency before erasure;
    // prior rejection or a different binding must not explain later absence.
    *p.candidates.lock().unwrap() = extracted("Zircon", "4040", Value::Null, 1);
    let zircon = learn_capture(
        &h,
        &owner,
        &base,
        &token,
        event(&bound, "zircon", "Zircon.port = 4040\n"),
    )
    .await;
    let erase_id = zircon["claim_ids"][0].as_str().unwrap();
    let zircon_url = format!("{base}/claims/{erase_id}");
    let zircon_first =
        ok(&h, "GET", &zircon_url, &owner, Value::Null).await["selected"]["revision"].clone();
    *p.candidates.lock().unwrap() = extracted("Zircon", "4041", zircon_first["id"].clone(), 1);
    let zircon_run = learn_capture(
        &h,
        &owner,
        &base,
        &token,
        event(
            &bound,
            "zircon-updated",
            "Zircon.port = 4041; explicitly replaces 4040.\n",
        ),
    )
    .await;
    assert_eq!(zircon_run["revised"], 1);
    let zircon_current =
        ok(&h, "GET", &zircon_url, &owner, Value::Null).await["selected"]["revision"].clone();
    let guard: LearningRun = serde_json::from_value(zircon_run).unwrap();
    let mut tx = db::actor_tx(&h.state.pool, guard.actor_id)
        .await
        .map_err(|e| e.1)
        .unwrap();
    db::require_writer(&mut tx, guard.brain_id)
        .await
        .map_err(|e| e.1)
        .unwrap();
    assert!(
        autonomous::targets(&h.state, &mut tx, &guard)
            .await
            .map_err(|e| e.1)
            .unwrap()
            .contains(&serde_json::from_value(zircon_current["id"].clone()).unwrap())
    );
    tx.commit().await.unwrap();

    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        json!({"kind":"claim","id":erase_id}),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":{"kind":"claim","id":erase_id},"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    privacy_journal::run_once(&h.state).await.unwrap();
    let removed: i64 = sqlx::query_scalar("SELECT count(*) FROM claim_revisions WHERE claim_id=$1 AND privacy_state='erased' AND revision='{}' AND subject_key='' AND predicate_key='' AND value_key=''")
        .bind(Uuid::parse_str(erase_id).unwrap()).fetch_one(&h.admin).await.unwrap();
    assert_eq!(removed, 2);
    for source in [
        zircon_first["content"]["supports"][0]["id"].clone(),
        zircon_current["content"]["supports"][0]["id"].clone(),
    ] {
        let fenced: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM model_input_fences WHERE brain_id=$1 AND source_version_id=$2)")
            .bind(guard.brain_id).bind(Uuid::parse_str(source.as_str().unwrap()).unwrap()).fetch_one(&h.admin).await.unwrap();
        assert!(fenced, "Both reconciliation source inputs must be fenced");
    }
    *p.candidates.lock().unwrap() = extracted("Quartz", "5050", Value::Null, 1);
    let independent = learn_capture(
        &h,
        &owner,
        &base,
        &token,
        event(&bound, "independent-control", "Quartz.port = 5050\n"),
    )
    .await;
    assert_eq!(independent["accepted"], 1);
    assert!(
        independent["reconciliation_inputs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !provider_inputs(&p)
            .iter()
            .any(|input| input.to_string().contains(erase_id))
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn capture_knowledge_time_delayed_arrival_append_replay_lock_and_feedback_exclusion() {
    let (h, owner, base, p, server) = setup().await;
    capture_policy(&h, &owner, &base, true).await;
    let (_, token) = h.pair_device(&owner, "Temporal capture proof").await;
    let bound = binding(
        &h,
        &base,
        &token,
        json!({"repository_ids":[],"area_ids":[],"environment_id":null}),
    )
    .await;
    let brain = Uuid::parse_str(base.rsplit('/').next().unwrap()).unwrap();
    let control = source(&h, &owner, &base, "chronology INDEPENDENT_CONTROL\n").await;
    process(&h).await;
    // Model a binding that already existed when the offline event occurred.
    sqlx::query(
        "UPDATE capture_bindings SET created_at=clock_timestamp()-interval '3 days' WHERE id=$1",
    )
    .bind(Uuid::parse_str(bound["id"].as_str().unwrap()).unwrap())
    .execute(&h.admin)
    .await
    .unwrap();
    let before_receipt = server_time(&h).await;
    let mut delayed = event(&bound, "offline", "chronology LATE_EVENT\n");
    delayed["event"]["captured_at"] = json!(before_receipt - Duration::days(2));
    let url = format!("{base}/capture/events");
    let receipt = device_ok(&h, "POST", &url, &token, delayed.clone()).await;
    let replay = device_ok(&h, "POST", &url, &token, delayed.clone()).await;
    assert_eq!(receipt["received_at"], replay["received_at"]);
    assert_eq!(receipt["source_version_id"], replay["source_version_id"]);
    process(&h).await;
    let historical = recalled(
        &h,
        &owner,
        &base,
        json!({"query":"chronology","knowledge_at":before_receipt}),
    )
    .await;
    assert!(contains(&historical, &control["version"]["id"]));
    assert!(!contains(&historical, &receipt["source_version_id"]));
    let exact = recalled(
        &h,
        &owner,
        &base,
        json!({"mode":"history","knowledge_at":before_receipt,
        "exact":{"kind":"source_version","id":receipt["source_version_id"]}}),
    )
    .await;
    assert!(exact["context"]["items"].as_array().unwrap().is_empty());
    let visible = recalled(&h, &owner, &base, json!({"query":"chronology"})).await;
    let item = visible["context"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == receipt["source_version_id"])
        .unwrap();
    assert_eq!(item["recorded_at"], receipt["received_at"]);
    let (captured, known, expires): (chrono::DateTime<Utc>, chrono::DateTime<Utc>, chrono::DateTime<Utc>) =
        sqlx::query_as("SELECT v.created_at,v.recorded_at,recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) FROM recollect_source_knowledge v WHERE v.id=$1")
        .bind(Uuid::parse_str(receipt["source_version_id"].as_str().unwrap()).unwrap()).fetch_one(&h.admin).await.unwrap();
    assert_eq!(captured, before_receipt - Duration::days(2));
    assert!(known > before_receipt);
    assert_eq!(expires, captured + Duration::days(30));

    let before_append = server_time(&h).await;
    let appended = ok(&h, "POST", &format!("{base}/sources/{}/versions",receipt["source_id"].as_str().unwrap()), &owner,
        json!({"base_version":receipt["source_version_id"],"title":"Later chronology source","media_type":"text/plain",
            "retain_content":true,"content":"chronology APPENDED_VERSION\n"})).await;
    process(&h).await;
    let between = recalled(
        &h,
        &owner,
        &base,
        json!({"query":"chronology","knowledge_at":before_append}),
    )
    .await;
    assert!(contains(&between, &receipt["source_version_id"]));
    assert!(!contains(&between, &appended["version"]["id"]));
    let early = recalled(
        &h,
        &owner,
        &base,
        json!({"mode":"history","knowledge_at":before_append,
        "exact":{"kind":"source_version","id":appended["version"]["id"]}}),
    )
    .await;
    assert!(early["context"]["items"].as_array().unwrap().is_empty());
    let current = recalled(&h, &owner, &base, json!({"query":"chronology"})).await;
    assert!(contains(&current, &appended["version"]["id"]));
    assert!(!contains(&current, &receipt["source_version_id"]));
    assert_eq!(
        appended["version"]["recorded_at"],
        appended["version"]["created_at"]
    );
    let history = ok(
        &h,
        "GET",
        &format!(
            "{base}/sources/{}/versions",
            receipt["source_id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(history["versions"][0]["id"], appended["version"]["id"]);
    assert_eq!(
        history["versions"][1]["recorded_at"],
        receipt["received_at"]
    );

    let mut lock = h.admin.begin().await.unwrap();
    sqlx::query("SELECT id FROM brains WHERE id=$1 FOR UPDATE")
        .bind(brain)
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    let future_time = server_time(&h).await + Duration::minutes(2);
    let mut future = event(&bound, "clock-skew", "chronology PERMITTED_CLOCK_SKEW\n");
    future["event"]["captured_at"] = json!(future_time);
    let (published, unlock_time) = tokio::join!(h.bearer("POST", &url, &token, future), async {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock')")
                .fetch_one(&h.admin).await.unwrap();
        assert!(
            waiting,
            "Prove the admission request is actually waiting behind a writer"
        );
        let at = server_time(&h).await;
        lock.commit().await.unwrap();
        at
    });
    assert_eq!(published.0, StatusCode::OK, "{}", published.1);
    let skew = published.1;
    let received: chrono::DateTime<Utc> =
        serde_json::from_value(skew["received_at"].clone()).unwrap();
    assert!(received >= unlock_time);
    assert!(received < future_time);
    process(&h).await;
    let now = recalled(
        &h,
        &owner,
        &base,
        json!({"exact":{"kind":"source_version","id":skew["source_version_id"]}}),
    )
    .await;
    assert!(contains(&now, &skew["source_version_id"]));

    for (turn, command, permitted) in [
        (
            "own-recall",
            "'/local/with spaces/recollect-agent' scope recall brain operation query",
            false,
        ),
        (
            "own-recall-encoded",
            "\"/local/recollect-agent\" scope recall brain operation query",
            false,
        ),
        (
            "permitted-tool",
            "inspect independent synthetic service",
            true,
        ),
    ] {
        let mut tool = event(&bound, turn, "");
        tool["event"]["host_event"] = json!("PostToolUse");
        tool["event"]["kind"] = json!("tool_result");
        tool["event"]["tool_name"] = json!("Bash");
        tool["event"]["tool_use_id"] = json!(turn);
        let mut content =
            json!({"input":{"command":command},"output":"TOOL_COPY_SENTINEL\nDocumentation: recollect-agent scope recall BRAIN OPERATION QUERY"}).to_string();
        if turn.ends_with("encoded") {
            content = content.replace("recollect-agent", "\\u0072ecollect-agent");
        }
        tool["event"]["content"] = json!(content);
        let admitted = device_ok(&h, "POST", &url, &token, tool).await;
        assert_eq!(admitted["source_id"].is_null(), !permitted);
        if !permitted {
            let metadata: Value =
                sqlx::query_scalar("SELECT metadata FROM capture_events WHERE id=$1")
                    .bind(Uuid::parse_str(admitted["event_id"].as_str().unwrap()).unwrap())
                    .fetch_one(&h.admin)
                    .await
                    .unwrap();
            assert!(metadata["content"].is_null());
            assert!(
                metadata["coverage"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("excluded_tool_content"))
            );
            assert!(!metadata.to_string().contains("TOOL_COPY_SENTINEL"));
        }
    }
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}
