use super::*;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn installation_restore_fences_queued_and_running_calls_and_model_accounting() {
    use recollect_server::recovery;
    let (h, owner, brain, connection, profile) = setup().await;
    allow_owner(&h, &owner, brain, &profile, true).await;
    let calls = format!("/api/brains/{brain}/mcp/calls");
    let runner = central(&h).await;
    let running = ok(&h, &owner, "POST", &calls, body(&connection, &profile)).await;
    let plan = checked(runner.claim(&h.state).await).plan.unwrap();
    let owned = instance(&h, &runner, &plan).await;
    let start = McpStart {
        attempt: attempt(&plan),
        instance_id: owned,
    };
    assert!(checked(runner.start(&h.state, &start).await).dispatch);
    let queued_input = body(&connection, &profile);
    let queued = ok(&h, &owner, "POST", &calls, queued_input.clone()).await;
    let actor: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let policy = Uuid::new_v4();
    sqlx::query("INSERT INTO model_policies(id,brain_id,policy,created_by) VALUES($1,$2,'{}',$3)")
        .bind(policy)
        .bind(brain)
        .bind(actor)
        .execute(&h.admin)
        .await
        .unwrap();
    let request = Uuid::new_v4();
    sqlx::query("INSERT INTO model_requests(id,brain_id,actor_id,operation_id,policy_id,purpose,model,prompt_label,schema_label,state,call_token,reserved_tokens,charged_tokens) VALUES($1,$2,$3,$4,$5,'extraction','fixture','fixture','fixture','running',$6,100,100)")
        .bind(request).bind(brain).bind(actor).bind(Uuid::new_v4()).bind(policy).bind(Uuid::new_v4()).execute(&h.admin).await.unwrap();
    let installation: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let origin = "https://source-recovery-fixture.invalid";
    let directory = recollect_server::mcp::runtime::central::receipt_directory(&h.state.config);
    let original_outbox =
        recollect_mcp_runtime::outbox::Outbox::open(directory.clone(), format!("central:{origin}"))
            .await
            .unwrap();
    let receipt = McpCompletion {
        attempt: attempt(&plan),
        state: "succeeded".into(),
        code: "tool_completed".into(),
        result: Some(
            json!({"content":[{"type":"text","text":"Retained controlled receipt"}],"isError":false}),
        ),
    };
    original_outbox
        .store(&receipt, std::time::SystemTime::now())
        .await
        .unwrap();
    recovery::guard(&h.state.config).await.unwrap();
    assert!(recovery::prepare(&h.admin, &h.state.config).await.is_err());
    let hold = recovery::Hold {
        restore_id: Uuid::new_v4(),
        backup_id: Uuid::new_v4(),
        installation_id: installation,
        source_origin: origin.into(),
    };
    let path = recovery::hold_path(&h.state.config);
    std::fs::write(&path, serde_json::to_vec(&hold).unwrap()).unwrap();
    assert!(recovery::guard(&h.state.config).await.is_err());
    assert!(recovery::release(&h.admin, &h.state.config).await.is_err());
    recovery::prepare(&h.admin, &h.state.config).await.unwrap();
    recovery::prepare(&h.admin, &h.state.config).await.unwrap();
    let moved = recollect_mcp_runtime::outbox::Outbox::open(
        directory.clone(),
        format!("central:{}", h.state.config.public_origin),
    )
    .await
    .unwrap();
    assert_eq!(
        moved
            .pending(std::time::SystemTime::now())
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        recollect_mcp_runtime::outbox::Outbox::open(directory, format!("central:{origin}"))
            .await
            .is_err()
    );
    for call in [&queued, &running] {
        let restored = ok(
            &h,
            &owner,
            "GET",
            &format!("{calls}/{}", call["id"].as_str().unwrap()),
            Value::Null,
        )
        .await;
        assert_eq!(restored["state"], "unknown");
        assert_eq!(restored["code"], "installation_restored");
    }
    let model: (String, i64, i64) = sqlx::query_as(
        "SELECT state,reserved_tokens,charged_tokens FROM model_requests WHERE id=$1",
    )
    .bind(request)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(model, ("uncertain".into(), 100, 100));
    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_audit WHERE action='mcp.recovery' AND disposition='unknown'",
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        audit, 2,
        "Preparing an interrupted restore does not duplicate recovery events"
    );
    assert!(
        !runner
            .start(&h.state, &start)
            .await
            .is_ok_and(|p| p.dispatch)
    );
    let current = central(&h).await;
    assert!(checked(current.claim(&h.state).await).plan.is_none());
    let replay = ok(&h, &owner, "POST", &calls, queued_input).await;
    assert_eq!(replay["id"], queued["id"]);
    assert_eq!(replay["state"], "unknown");
    // A stored receipt can still reconcile the old attempt, without repeating
    // its external dispatch. The original unknown state remains historical.
    checked(
        runner
            .complete(
                &h.state,
                &moved.pending(std::time::SystemTime::now()).await.unwrap()[0],
            )
            .await,
    );
    moved.acknowledge(&receipt).await.unwrap();
    assert!(
        moved
            .pending(std::time::SystemTime::now())
            .await
            .unwrap()
            .is_empty()
    );
    let resolved = ok(
        &h,
        &owner,
        "GET",
        &format!("{calls}/{}", running["id"].as_str().unwrap()),
        Value::Null,
    )
    .await;
    assert_eq!(resolved["state"], "unknown");
    assert_eq!(resolved["resolutions"][0]["kind"], "late_receipt");
    recovery::release(&h.admin, &h.state.config).await.unwrap();
    recovery::release(&h.admin, &h.state.config).await.unwrap();
    recovery::guard(&h.state.config).await.unwrap();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_evidence_reconciliation_identity_and_source_retention() {
    let (h, owner, brain, connection, profile) = setup().await;
    allow_owner(&h, &owner, brain, &profile, true).await;
    let runner = central(&h).await;
    let base = format!("/api/brains/{brain}");
    let calls = format!("{base}/mcp/calls");
    let mut ids = Vec::new();
    for _ in 0..2 {
        let call = ok(&h, &owner, "POST", &calls, body(&connection, &profile)).await;
        let plan = checked(runner.claim(&h.state).await).plan.unwrap();
        let owned = instance(&h, &runner, &plan).await;
        assert!(
            checked(
                runner
                    .start(
                        &h.state,
                        &McpStart {
                            attempt: attempt(&plan),
                            instance_id: owned
                        }
                    )
                    .await
            )
            .dispatch
        );
        sqlx::query("UPDATE mcp_calls SET lease_until=now()-interval '1 second' WHERE id=$1")
            .bind(plan.call_id)
            .execute(&h.admin)
            .await
            .unwrap();
        checked(runner::maintain(&h.state).await);
        ids.push(call["id"].as_str().unwrap().to_string());
    }
    let path = format!("{calls}/{}/resolve", ids[0]);
    let mut input = json!({"request_id":Uuid::new_v4(),"outcome":"succeeded","explanation":"The retained fixture receipt establishes completion.","source_version_id":Uuid::new_v4()});
    assert_eq!(
        h.call("POST", &path, Some(&owner), input.clone()).await.0,
        StatusCode::BAD_REQUEST
    );
    let source=ok(&h,&owner,"POST",&format!("{base}/sources"),json!({"title":"Resolution evidence","media_type":"text/plain","content":"Synthetic operation receipt.","retain_content":true})).await;
    input["source_version_id"] = source["version"]["id"].clone();
    let resolved = ok(&h, &owner, "POST", &path, input.clone()).await;
    assert_eq!(resolved["state"], "unknown");
    assert_eq!(resolved["resolutions"][0]["kind"], "evidence");
    assert_eq!(
        resolved["resolutions"][0]["source_version_id"],
        source["version"]["id"]
    );
    assert_eq!(
        ok(&h, &owner, "POST", &path, input.clone()).await["resolutions"],
        resolved["resolutions"]
    );
    let mut changed = input.clone();
    changed["outcome"] = json!("failed");
    assert_eq!(
        h.call("POST", &path, Some(&owner), changed).await.1["code"],
        "mcp_request_conflict"
    );
    let other = format!("{calls}/{}/resolve", ids[1]);
    assert_eq!(
        h.call("POST", &other, Some(&owner), input.clone()).await.1["code"],
        "mcp_request_conflict"
    );
    let mut concurrent = input.clone();
    concurrent["request_id"] = json!(Uuid::new_v4());
    let (left, right) = tokio::join!(
        h.call("POST", &path, Some(&owner), concurrent.clone()),
        h.call("POST", &other, Some(&owner), concurrent)
    );
    assert!(matches!(
        (left.0, right.0),
        (StatusCode::OK, StatusCode::CONFLICT) | (StatusCode::CONFLICT, StatusCode::OK)
    ));
    let target = json!({"kind":"source","id":source["id"]});
    let preview = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/erasures/preview"),
        target.clone(),
    )
    .await;
    ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/erasures"),
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let hidden = ok(
        &h,
        &owner,
        "GET",
        &format!("{calls}/{}", ids[0]),
        Value::Null,
    )
    .await;
    assert!(
        hidden["resolutions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["explanation"].is_null()),
        "erasure immediately masks dependent explanations before maintenance"
    );
    assert_eq!(
        h.call("POST", &path, Some(&owner), input).await.1["code"],
        "mcp_replay_expired"
    );
    checked(runner::maintain(&h.state).await);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM mcp_call_resolutions WHERE explanation IS NOT NULL"
        )
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_capacity_deferral_queue_expiry_and_dispatch_ack_loss() {
    let (h, owner, brain, connection, profile) = setup().await;
    allow_owner(&h, &owner, brain, &profile, true).await;
    let base = format!("/api/brains/{brain}/mcp/calls");
    let input = body(&connection, &profile);
    let first = ok(&h, &owner, "POST", &base, input.clone()).await;
    for _ in 0..31 {
        ok(&h, &owner, "POST", &base, body(&connection, &profile)).await;
    }
    assert_eq!(
        h.call("POST", &base, Some(&owner), body(&connection, &profile))
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        ok(&h, &owner, "POST", &base, input).await["id"],
        first["id"],
        "replay remains available at capacity"
    );
    let runner = central(&h).await;
    let first_plan = checked(runner.claim(&h.state).await).plan.unwrap();
    checked(runner.defer(&h.state, &attempt(&first_plan)).await);
    assert!(
        runner
            .complete(
                &h.state,
                &McpCompletion {
                    attempt: attempt(&first_plan),
                    state: "failed".into(),
                    code: "startup_failed".into(),
                    result: None
                }
            )
            .await
            .is_err(),
        "a deferred pre-dispatch attempt is fenced"
    );
    let plan = checked(runner.claim(&h.state).await).plan.unwrap();
    assert_ne!(
        plan.call_id, first_plan.call_id,
        "capacity wait does not busy-loop on its original queued call"
    );
    checked(
        runner
            .complete(
                &h.state,
                &McpCompletion {
                    attempt: attempt(&plan),
                    state: "unknown".into(),
                    code: "dispatch_acknowledgment_lost".into(),
                    result: None,
                },
            )
            .await,
    );
    let stopped = ok(
        &h,
        &owner,
        "GET",
        &format!("{base}/{}", plan.call_id),
        Value::Null,
    )
    .await;
    assert_eq!(stopped["state"], "failed");
    assert_eq!(stopped["code"], "dispatch_not_committed");
    let plan = checked(runner.claim(&h.state).await).plan.unwrap();
    let owned = instance(&h, &runner, &plan).await;
    assert!(
        checked(
            runner
                .start(
                    &h.state,
                    &McpStart {
                        attempt: attempt(&plan),
                        instance_id: owned
                    }
                )
                .await
        )
        .dispatch
    );
    assert!(
        runner.defer(&h.state, &attempt(&plan)).await.is_err(),
        "running calls cannot return to the queue"
    );
    checked(
        runner
            .complete(
                &h.state,
                &McpCompletion {
                    attempt: attempt(&plan),
                    state: "unknown".into(),
                    code: "dispatch_acknowledgment_lost".into(),
                    result: None,
                },
            )
            .await,
    );
    assert_eq!(
        ok(
            &h,
            &owner,
            "GET",
            &format!("{base}/{}", plan.call_id),
            Value::Null
        )
        .await["state"],
        "unknown"
    );
    sqlx::query(
        "UPDATE mcp_calls SET queue_expires_at=now()-interval '1 second' WHERE state='queued'",
    )
    .execute(&h.admin)
    .await
    .unwrap();
    checked(runner::maintain(&h.state).await);
    assert!(checked(runner.claim(&h.state).await).plan.is_none());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_calls WHERE state='queued'")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    h.finish().await;
}
