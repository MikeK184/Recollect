//! Native rollback and privacy proofs; controlled verdicts test lifecycle only.
use super::*;
use recollect_server::artifacts;

async fn enable(h: &Harness, owner: &Login, base: &str) {
    capture_policy(h, owner, base, true).await;
    allow(h, owner, base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
        policy["content_classes"] = json!(["raw_session", "claim", "query", "support_excerpt"]);
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn automatic_excerpt_rollback_lease_cleanup_and_erasure_are_durable() {
    for erase_before_cleanup in [false, true] {
        let (mut h, owner, base, p, server) = setup().await;
        enable(&h, &owner, &base).await;
        let brain = base.rsplit('/').next().unwrap().parse::<Uuid>().unwrap();
        let (_, token) = h.pair_device(&owner, "Excerpt rollback proof").await;
        let bound = binding(&h, &base, &token, json!({})).await;
        *p.candidates.lock().unwrap() = extracted("Amber", "8080", Value::Null, 1);
        let receipt = device_ok(
            &h,
            "POST",
            &format!("{base}/capture/events"),
            &token,
            event(&bound, "rollback", "Amber.port = 8080\n"),
        )
        .await;
        process(&h).await;
        assert_eq!(autonomous::run_once(&h.state).await.ok(), Some(1));
        // record_learning occurs after retaining the file and appending the
        // canonical revision. Its failure rolls back both canonical records.
        sqlx::query("ALTER TABLE memory_support_assessments ADD CONSTRAINT excerpt_rollback_probe CHECK(source_stage_id IS NULL) NOT VALID")
            .execute(&h.admin).await.unwrap();
        let job = worker::claim(&h.state.pool, "model")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            worker::execute(&h.state, &job).await,
            Err(worker::Failure::Database)
        );
        assert_eq!(p.calls.load(Ordering::SeqCst), 2);
        let (artifact,parent):(Uuid,Uuid)=sqlx::query_as("SELECT artifact_id,parent_version_id FROM excerpt_artifact_intents WHERE brain_id=$1 AND removed_at IS NULL")
            .bind(brain).fetch_one(&h.admin).await.unwrap();
        assert_eq!(json!(parent), receipt["source_version_id"]);
        let path = artifacts::path(&h.state.config.artifact_dir, brain, artifact);
        assert!(path.exists());
        let absent: i64 =
            sqlx::query_scalar("SELECT count(*) FROM automatic_support_excerpts WHERE brain_id=$1")
                .bind(brain)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        assert_eq!(absent, 0);
        // A running lease cannot be swept, and an actor-scoped SQL transaction
        // cannot enumerate global maintenance work or forget its durable intent.
        let pending: i64 =
            sqlx::query_scalar("SELECT count(*) FROM recollect_pending_excerpt_artifacts(NULL)")
                .fetch_one(&h.state.pool)
                .await
                .unwrap();
        assert_eq!(pending, 0);
        let actor: Uuid = sqlx::query_scalar("SELECT actor_id FROM learning_runs WHERE id=$1")
            .bind(job.target_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
        let mut scoped = db::actor_tx(&h.state.pool, actor)
            .await
            .map_err(|e| e.1)
            .unwrap();
        sqlx::query("SELECT recollect_forget_excerpt_artifact($1,$2)")
            .bind(brain)
            .bind(artifact)
            .execute(&mut *scoped)
            .await
            .unwrap();
        scoped.commit().await.unwrap();
        assert!(
            sqlx::query_scalar::<_, bool>(
                "SELECT removed_at IS NULL FROM excerpt_artifact_intents WHERE artifact_id=$1"
            )
            .bind(artifact)
            .fetch_one(&h.admin)
            .await
            .unwrap()
        );
        let older = if !erase_before_cleanup {
            privacy_journal::barrier(&h.state.pool, &h.state.config)
                .await
                .unwrap();
            let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
            eprintln!("Disposable excerpt-intent restore fixture: {backup}");
            let config = (*h.state.config).clone();
            h.state.pool.close().await;
            h.admin.close().await;
            sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
                .execute(&h.root)
                .await
                .unwrap();
            sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'"))
                .execute(&h.root).await.unwrap();
            let mut admin_url =
                reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
            admin_url.set_path(&h.database);
            h.admin = db::pool(admin_url.as_str()).await.unwrap();
            h.state = AppState::new(
                db::pool(&config.database_url).await.unwrap(),
                config.clone(),
            )
            .unwrap();
            h.router = app(h.state.clone());
            admin_url.set_path(&backup);
            let older_admin = db::pool(admin_url.as_str()).await.unwrap();
            let mut older_config = config;
            let mut app_url = reqwest::Url::parse(&older_config.database_url).unwrap();
            app_url.set_path(&backup);
            older_config.database_url = app_url.to_string();
            older_config.artifact_dir = format!(".cache/{backup}-artifacts");
            artifacts::write(
                &older_config.artifact_dir,
                brain,
                artifact,
                "Amber.port = 8080\n",
            )
            .await
            .unwrap();
            let older_pool = db::pool(&older_config.database_url).await.unwrap();
            Some((backup, older_admin, older_pool, older_config))
        } else {
            None
        };
        if erase_before_cleanup {
            erase(&h, &owner, &base, &receipt["source_id"]).await;
            privacy_journal::run_once(&h.state).await.unwrap();
            assert!(
                !path.exists(),
                "Original erasure includes the running intent"
            );
        } else {
            assert!(
                worker::fail(&h.state.pool, &job, worker::Failure::Database)
                    .await
                    .unwrap()
            );
            privacy_journal::run_once(&h.state).await.unwrap();
            assert!(!path.exists());
            assert!(sqlx::query_scalar::<_,bool>("SELECT removed_at IS NOT NULL FROM excerpt_artifact_intents WHERE artifact_id=$1")
                .bind(artifact).fetch_one(&h.admin).await.unwrap());
            sqlx::query(
                "ALTER TABLE memory_support_assessments DROP CONSTRAINT excerpt_rollback_probe",
            )
            .execute(&h.admin)
            .await
            .unwrap();
            sqlx::query(
                "UPDATE jobs SET not_before=clock_timestamp()-interval '1 second' WHERE id=$1",
            )
            .bind(job.id)
            .execute(&h.admin)
            .await
            .unwrap();
            let restarted = AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap();
            assert!(worker::run_once(&restarted, "model").await.unwrap());
            assert_eq!(
                p.calls.load(Ordering::SeqCst),
                2,
                "Saved verdict resumes with no paid replay"
            );
            assert_eq!(
                runs(&h, &owner, &base).await["items"][0]["state"],
                "succeeded"
            );
            // A pre-cleanup artifact copy must still be in the later erasure
            // closure, even though routine cleanup already removed it.
            artifacts::write(
                &h.state.config.artifact_dir,
                brain,
                artifact,
                "Amber.port = 8080\n",
            )
            .await
            .unwrap();
            erase(&h, &owner, &base, &receipt["source_id"]).await;
            privacy_journal::run_once(&h.state).await.unwrap();
            assert!(
                !path.exists(),
                "Removed-intent tombstone closes later erasure"
            );
        }
        let remaining: i64 =
            sqlx::query_scalar("SELECT count(*) FROM excerpt_artifact_intents WHERE brain_id=$1")
                .bind(brain)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        assert_eq!(remaining, 0);
        if let Some((backup, admin, pool, config)) = older {
            assert!(
                privacy_journal::barrier(&pool, &config).await.is_err(),
                "An older database cannot serve before replay"
            );
            privacy_journal::reconcile(&admin, &config).await.unwrap();
            privacy_journal::barrier(&pool, &config).await.unwrap();
            assert!(!artifacts::path(&config.artifact_dir, brain, artifact).exists());
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM excerpt_artifact_intents WHERE brain_id=$1"
                )
                .bind(brain)
                .fetch_one(&admin)
                .await
                .unwrap(),
                0
            );
            pool.close().await;
            admin.close().await;
            sqlx::query(&format!("DROP DATABASE {backup} WITH (FORCE)"))
                .execute(&h.root)
                .await
                .unwrap();
            std::fs::remove_dir_all(&config.artifact_dir).unwrap();
        }
        server.abort();
        h.finish().await;
    }
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn automatic_digest_final_excerpt_expiry_scrubs_partition_and_preserves_control() {
    let (h, owner, base, p, server) = setup().await;
    enable(&h, &owner, &base).await;
    let brain = base.rsplit('/').next().unwrap().parse::<Uuid>().unwrap();
    let (_, token) = h.pair_device(&owner, "Digest expiry proof").await;
    let bound = binding(&h, &base, &token, json!({})).await;
    let mut receipts = vec![];
    for (session, subject) in [("expire", "Amber"), ("control", "Onyx")] {
        *p.candidates.lock().unwrap() = extracted(subject, "8080", Value::Null, 1);
        let mut input = event(&bound, session, &format!("{subject}.port = 8080\n"));
        input["event"]["host_session_id"] = json!(session);
        receipts
            .push(device_ok(&h, "POST", &format!("{base}/capture/events"), &token, input).await);
        process(&h).await;
        assert_eq!(autonomous::run_once(&h.state).await.ok(), Some(1));
        model_job(&h).await;
    }
    sqlx::query("UPDATE capture_events SET received_at=clock_timestamp()-interval '6 minutes' WHERE brain_id=$1")
        .bind(brain).execute(&h.admin).await.unwrap();
    assert_eq!(autonomous::run_once(&h.state).await.ok(), Some(2));
    *p.candidates.lock().unwrap() = json!({"summary":"The captured port configuration remains recorded.","completed":[],"next_steps":[],"risks":[]});
    model_job(&h).await;
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 8);
    let raw = receipts[0]["source_version_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    let copy: Uuid = sqlx::query_scalar(
        "SELECT version_id FROM automatic_support_excerpts WHERE parent_version_id=$1",
    )
    .bind(raw)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '400 days' WHERE id=$1",
    )
    .bind(raw)
    .execute(&h.admin)
    .await
    .unwrap();
    sqlx::query("UPDATE capture_events SET captured_at=clock_timestamp()-interval '400 days' WHERE source_version_id=$1").bind(raw).execute(&h.admin).await.unwrap();
    privacy_journal::run_once(&h.state).await.unwrap();
    let usable:i64=sqlx::query_scalar("SELECT count(*) FROM session_digest_claims d WHERE d.brain_id=$1 AND recollect_memory_supported(d.brain_id,d.revision_id)")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        usable, 2,
        "Independently retained support survives raw expiry"
    );
    // The last permitted excerpt expires after its raw source has physically
    // disappeared. Privacy must still reach its partition through claim lineage.
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '400 days' WHERE id=$1",
    )
    .bind(copy)
    .execute(&h.admin)
    .await
    .unwrap();
    // Supporting excerpts normally have unlimited retention; set a bounded
    // policy through the ordinary settings API before advancing this clock.
    let mut settings = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    settings["policy"]["support_excerpt_days"] = json!(30);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":settings["change_id"],"policy":settings["policy"]}),
    )
    .await;
    privacy_journal::run_once(&h.state).await.unwrap();
    let partitions:Vec<(String,String,Value)>=sqlx::query_as("SELECT host_session_id,privacy_state,coverage FROM session_digest_partitions WHERE brain_id=$1 ORDER BY host_session_id")
        .bind(brain).fetch_all(&h.admin).await.unwrap();
    assert_eq!(partitions.len(), 2);
    assert_eq!(partitions[0], ("".into(), "expired".into(), json!({})));
    assert_eq!(partitions[1].0, "control");
    assert_eq!(partitions[1].1, "active");
    let usable:i64=sqlx::query_scalar("SELECT count(*) FROM session_digest_claims d WHERE d.brain_id=$1 AND recollect_memory_supported(d.brain_id,d.revision_id)")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(usable, 1);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        8,
        "Privacy never pays for regeneration"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn automatic_digest_scanner_rotates_past_one_hundred_settled_sessions() {
    let (h, owner, base, p, server) = setup().await;
    enable(&h, &owner, &base).await;
    let brain = base.rsplit('/').next().unwrap().parse::<Uuid>().unwrap();
    let (_, token) = h.pair_device(&owner, "Digest fairness proof").await;
    let bound = binding(&h, &base, &token, json!({})).await;
    for n in 0..100 {
        let mut input = event(&bound, &format!("metadata-{n}"), "");
        input["event"]["host_session_id"] = json!(format!("empty-{n}"));
        input["event"]["kind"] = json!("lifecycle");
        input["event"]["host_event"] = json!("PreToolUse");
        input["event"]["content"] = Value::Null;
        device_ok(&h, "POST", &format!("{base}/capture/events"), &token, input).await;
    }
    *p.candidates.lock().unwrap() = extracted("Amber", "8080", Value::Null, 1);
    let mut input = event(&bound, "supported-last", "Amber.port = 8080\n");
    input["event"]["host_session_id"] = json!("supported-last");
    device_ok(&h, "POST", &format!("{base}/capture/events"), &token, input).await;
    process(&h).await;
    assert_eq!(autonomous::run_once(&h.state).await.ok(), Some(1));
    model_job(&h).await;
    sqlx::query("UPDATE capture_events SET received_at=clock_timestamp()-interval '6 minutes' WHERE brain_id=$1")
        .bind(brain).execute(&h.admin).await.unwrap();
    let mut queued = 0;
    for _ in 0..3 {
        queued += autonomous::run_once(&h.state).await.ok().unwrap();
    }
    assert_eq!(
        queued, 1,
        "Settled empty partitions cannot starve later useful work"
    );
    *p.candidates.lock().unwrap() =
        json!({"summary":"Amber declares port 8080.","completed":[],"next_steps":[],"risks":[]});
    model_job(&h).await;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM session_digest_partitions WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(count, 101);
    let mut late = event(&bound, "late-last", "");
    late["event"]["host_session_id"] = json!("supported-last");
    late["event"]["kind"] = json!("lifecycle");
    late["event"]["host_event"] = json!("PreToolUse");
    late["event"]["content"] = Value::Null;
    device_ok(&h, "POST", &format!("{base}/capture/events"), &token, late).await;
    sqlx::query("UPDATE capture_events SET received_at=clock_timestamp()-interval '6 minutes' WHERE brain_id=$1")
        .bind(brain).execute(&h.admin).await.unwrap();
    for _ in 0..3 {
        assert_eq!(autonomous::run_once(&h.state).await.ok(), Some(0));
    }
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    assert!(sqlx::query_scalar::<_,bool>("SELECT recollect_memory_supported(d.brain_id,d.revision_id) FROM session_digest_claims d WHERE d.brain_id=$1")
        .bind(brain).fetch_one(&h.admin).await.unwrap());
    server.abort();
    h.finish().await;
}
