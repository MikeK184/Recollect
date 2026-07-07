use super::*;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn publication_rechecks_rules_expiry_and_lease_without_inheriting_old_raw_ttl() {
    let (h, owner, base, p, server) = setup().await;
    capture_policy(&h, &owner, &base, true).await;
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
        policy["content_classes"] = json!(["raw_session", "claim", "query"]);
    })
    .await;
    let (_, token) = h.pair_device(&owner, "Publication fence proof").await;
    let bound = binding(
        &h,
        &base,
        &token,
        json!({"repository_ids":[],"area_ids":[],"environment_id":null}),
    )
    .await;
    sqlx::query(
        "UPDATE capture_bindings SET created_at=clock_timestamp()-interval '40 days' WHERE id=$1",
    )
    .bind(Uuid::parse_str(bound["id"].as_str().unwrap()).unwrap())
    .execute(&h.admin)
    .await
    .unwrap();
    let mut original = event(&bound, "older-support", "Amber.port = 8080\n");
    original["event"]["captured_at"] = json!(Utc::now() - Duration::days(29));
    *p.candidates.lock().unwrap() = extracted("Amber", "8080", Value::Null, 1);
    let initial = learn_capture(&h, &owner, &base, &token, original).await;
    let id = initial["claim_ids"][0].as_str().unwrap();
    let claim_url = format!("{base}/claims/{id}");
    let first =
        ok(&h, "GET", &claim_url, &owner, Value::Null).await["selected"]["revision"].clone();
    *p.candidates.lock().unwrap() = extracted("Amber", "9090", first["id"].clone(), 1);
    let revised = learn_capture(
        &h,
        &owner,
        &base,
        &token,
        event(
            &bound,
            "current-support",
            "Amber.port = 9090; explicitly replaces 8080.\n",
        ),
    )
    .await;
    let second =
        ok(&h, "GET", &claim_url, &owner, Value::Null).await["selected"]["revision"].clone();
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["raw_session_days"] = json!(28);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    let mut current_run: LearningRun = serde_json::from_value(revised).unwrap();
    current_run.reconciliation_inputs = vec![serde_json::from_value(second["id"].clone()).unwrap()];
    let mut tx = db::actor_tx(&h.state.pool, current_run.actor_id)
        .await
        .map_err(|e| e.1)
        .unwrap();
    db::require_writer(&mut tx, current_run.brain_id)
        .await
        .map_err(|e| e.1)
        .unwrap();
    autonomous::recheck_target(
        &h.state,
        &mut tx,
        &current_run,
        current_run.reconciliation_inputs[0],
    )
    .await
    .map_err(|e| e.1)
    .expect("Current support remains valid after older reasoning evidence expires");
    assert!(
        autonomous::input_deadline(&mut tx, &current_run)
            .await
            .map_err(|e| e.1)
            .unwrap()
            .unwrap()
            > Utc::now()
    );
    tx.commit().await.unwrap();

    // Only this disposable database pauses publication. Both cases cross the
    // five-second renewal tick while the worker itself holds the job-row lock.
    sqlx::raw_sql("CREATE FUNCTION test_pause_publication() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.value_key IN ('6060','7070') THEN PERFORM pg_sleep(6.2); END IF; RETURN NEW; END $$;
        CREATE TRIGGER test_pause_publication BEFORE INSERT ON claim_revisions
          FOR EACH ROW WHEN ((NEW.revision->>'origin')='model_reconciled') EXECUTE FUNCTION test_pause_publication();")
        .execute(&h.admin).await.unwrap();
    *p.candidates.lock().unwrap() = extracted("Amber", "6060", second["id"].clone(), 1);
    let started = std::time::Instant::now();
    let successful = learn_capture(
        &h,
        &owner,
        &base,
        &token,
        event(
            &bound,
            "slow-publication",
            "Amber.port = 6060; explicitly replaces 9090.\n",
        ),
    )
    .await;
    assert!(started.elapsed().as_secs_f64() >= 6.2);
    assert_eq!(successful["revised"], 1);
    let current =
        ok(&h, "GET", &claim_url, &owner, Value::Null).await["selected"]["revision"].clone();
    let mut expiring = event(
        &bound,
        "near-expiry",
        "Amber.port = 7070; explicitly replaces 6060.\n",
    );
    expiring["event"]["captured_at"] =
        json!(server_time(&h).await - Duration::days(28) + Duration::seconds(2));
    device_ok(
        &h,
        "POST",
        &format!("{base}/capture/events"),
        &token,
        expiring,
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
    *p.candidates.lock().unwrap() = extracted("Amber", "7070", current["id"].clone(), 1);
    model_job(&h).await;
    let failed = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(failed["state"], "failed", "{failed}");
    assert_eq!(
        failed["error_code"], "learning_input_expired",
        "Must reach the final publication fence: {failed}"
    );
    assert_eq!(
        ok(&h, "GET", &claim_url, &owner, Value::Null).await["selected"]["revision"]["id"],
        current["id"]
    );

    // Rejecting another identity changes this target's current eligibility
    // without changing its revision. Test readiness under its publication lock.
    let duplicate = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        json!({"content":current["content"],"operation_id":null}),
    )
    .await;
    ok(&h, "POST", &format!("{base}/claims/{}/review",duplicate["claim_id"].as_str().unwrap()), &owner,
        json!({"base_revision":duplicate["id"],"action":"reject","reason":"Reject the duplicated assertion."})).await;
    current_run = serde_json::from_value(successful).unwrap();
    let revision = serde_json::from_value(current["id"].clone()).unwrap();
    current_run.reconciliation_inputs = vec![revision];
    let mut tx = db::actor_tx(&h.state.pool, current_run.actor_id)
        .await
        .map_err(|e| e.1)
        .unwrap();
    db::require_writer(&mut tx, current_run.brain_id)
        .await
        .map_err(|e| e.1)
        .unwrap();
    assert_eq!(
        autonomous::target(&mut tx, &current_run, revision)
            .await
            .map_err(|e| e.1)
            .unwrap()
            .id,
        revision
    );
    assert_eq!(
        autonomous::recheck_target(&h.state, &mut tx, &current_run, revision)
            .await
            .err()
            .unwrap()
            .1,
        "reconciliation_input_changed"
    );
    tx.rollback().await.unwrap();
    *p.candidates.lock().unwrap() = extracted("Quartz", "5050", Value::Null, 1);
    let permitted = learn_capture(
        &h,
        &owner,
        &base,
        &token,
        event(&bound, "permitted", "Quartz.port = 5050\n"),
    )
    .await;
    assert_eq!(permitted["accepted"], 1);
    server.abort();
    h.finish().await;
}
