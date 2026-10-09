use super::*;

async fn discovery_clone(h: &Harness, base: &str, template: &Value, id: Uuid, unavailable: bool) {
    let brain = brain_id(base);
    let claim = Uuid::new_v4();
    let mut revision = template.clone();
    revision["id"] = json!(id);
    revision["claim_id"] = json!(claim);
    if unavailable {
        revision["content"]["freshness"] = json!("superseded");
    }
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::query("INSERT INTO claims(id,brain_id,created_by,current_revision) SELECT $1,brain_id,created_by,NULL FROM claims WHERE brain_id=$2 AND id=$3")
        .bind(claim).bind(brain).bind(uuid(&template["claim_id"])).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO claim_revisions(id,claim_id,brain_id,recorded_at,revision,subject_key,predicate_key,value_key)
        SELECT $1,$2,brain_id,recorded_at,$3,subject_key,predicate_key,value_key FROM claim_revisions WHERE brain_id=$4 AND id=$5")
        .bind(id).bind(claim).bind(sqlx::types::Json(revision)).bind(brain).bind(uuid(&template["id"]))
        .execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO claim_supports SELECT $1,brain_id,ordinal,source_version_id,fact_id,manifest_revision_id FROM claim_supports WHERE brain_id=$2 AND revision_id=$3")
        .bind(id).bind(brain).bind(uuid(&template["id"])).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE claims SET current_revision=$1 WHERE brain_id=$2 AND id=$3")
        .bind(id)
        .bind(brain)
        .bind(claim)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_discovery_walk_passes_unavailable_prefix_wraps_and_survives_restart() {
    use recollect_server::autonomous::support_discovery;
    let (h, _, base, p, server, template) = direct_audit_fixture().await;
    let cx = context(&h, &base).await;
    for index in 1..=35 {
        discovery_clone(&h, &base, &template, Uuid::from_u128(index), true).await;
    }
    let target = Uuid::from_u128(u128::MAX);
    discovery_clone(&h, &base, &template, target, false).await;
    // Start this controlled walk at the prefix. The fixture already completed
    // one ordinary audit walk with a randomly ordered revision identity.
    sqlx::query("UPDATE memory_support_discovery SET last_revision_id=NULL,version=version+1 WHERE brain_id=$1")
        .bind(cx.brain).execute(&h.admin).await.unwrap();
    let calls = p.calls.load(Ordering::SeqCst);
    for (pass, expected) in [(1, 0), (2, 0), (3, 1)] {
        // New AppState each pass: progress must come from PostgreSQL, not a
        // process-local cache or a prepared object retained across iterations.
        let state = h.state.clone();
        let prepared = support_discovery::prepare(&state, cx.brain, cx.actor)
            .await
            .ok()
            .unwrap()
            .unwrap();
        assert_eq!(
            support_discovery::publish(&state, prepared)
                .await
                .ok()
                .unwrap(),
            expected
        );
        if pass < 3 {
            let last: Uuid = sqlx::query_scalar(
                "SELECT last_revision_id FROM memory_support_discovery WHERE brain_id=$1",
            )
            .bind(cx.brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
            assert_eq!(last, Uuid::from_u128(pass * 16));
        }
    }
    let recorded: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM memory_support_assessments WHERE brain_id=$1 AND revision_id=$2",
    )
    .bind(cx.brain)
    .bind(target)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(recorded, 1);
    let prepared = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    assert_eq!(
        support_discovery::publish(&h.state, prepared)
            .await
            .ok()
            .unwrap(),
        0
    );
    let last: Uuid = sqlx::query_scalar(
        "SELECT last_revision_id FROM memory_support_discovery WHERE brain_id=$1",
    )
    .bind(cx.brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!(
        last < target,
        "The completed tail must wrap within the next examination allowance"
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        calls,
        "Discovery and publication never call a provider"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_discovery_fresh_publication_fences_cursors_review_policy_erasure_and_access() {
    use recollect_server::autonomous::support_discovery;
    let (h, owner, base, p, server, template) = direct_audit_fixture().await;
    let cx = context(&h, &base).await;
    let target = Uuid::from_u128(100);
    discovery_clone(&h, &base, &template, target, false).await;
    let first = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    let stale = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    // An unrelated capture/epoch advance is not a reason to trust a prepared
    // Boolean or starve audits: complete fresh qualification remains decisive.
    let mut writer = db::actor_tx(&h.state.pool, cx.actor).await.ok().unwrap();
    db::require_writer(&mut writer, cx.brain)
        .await
        .ok()
        .unwrap();
    sqlx::query("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1")
        .bind(cx.brain)
        .execute(&mut *writer)
        .await
        .unwrap();
    writer.commit().await.unwrap();
    assert_eq!(
        support_discovery::publish(&h.state, first)
            .await
            .ok()
            .unwrap(),
        1
    );
    assert_eq!(
        support_discovery::publish(&h.state, stale)
            .await
            .ok()
            .unwrap(),
        0
    );
    // Human review changes the exact current head before the prepared revision
    // is published. The old assertion cannot be automatically queued afterward.
    let target = Uuid::from_u128(200);
    discovery_clone(&h, &base, &template, target, false).await;
    let prepared = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    let revision: sqlx::types::Json<Value> =
        sqlx::query_scalar("SELECT revision FROM claim_revisions WHERE brain_id=$1 AND id=$2")
            .bind(cx.brain)
            .bind(target)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    reviewed(&h, &owner, &base, &revision.0).await;
    assert_eq!(
        support_discovery::publish(&h.state, prepared)
            .await
            .ok()
            .unwrap(),
        0
    );
    let aged = Uuid::from_u128(300);
    discovery_clone(&h, &base, &template, aged, false).await;
    sqlx::query("UPDATE claim_revisions SET recorded_at=clock_timestamp()-interval '2 days' WHERE brain_id=$1 AND id=$2")
        .bind(cx.brain).bind(aged).execute(&h.admin).await.unwrap();
    let prepared = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut retention_policy = retention["policy"].clone();
    retention_policy["claim_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":retention_policy}),
    )
    .await;
    assert_eq!(
        support_discovery::publish(&h.state, prepared)
            .await
            .ok()
            .unwrap(),
        0,
        "Fresh retention gates reject a candidate that expired after preparation"
    );
    let prepared = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    permit(&h, &owner, &base, 100000).await;
    assert_eq!(
        support_discovery::publish(&h.state, prepared)
            .await
            .ok()
            .unwrap(),
        0,
        "Policy generations cannot inherit prepared eligibility"
    );
    let prepared = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    let source: Uuid = sqlx::query_scalar("SELECT v.source_id FROM claim_supports s JOIN source_versions v ON v.id=s.source_version_id WHERE s.brain_id=$1 AND s.revision_id=$2 LIMIT 1")
        .bind(cx.brain).bind(uuid(&template["id"])).fetch_one(&h.admin).await.unwrap();
    let target = json!({"kind":"source","id":source});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert_eq!(
        support_discovery::publish(&h.state, prepared)
            .await
            .ok()
            .unwrap(),
        0,
        "Erased targets never enter the audit queue"
    );
    let prepared = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE accounts SET enabled=false WHERE id=$1")
        .bind(cx.actor)
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(
        support_discovery::publish(&h.state, prepared)
            .await
            .is_err(),
        "Disabled actors cannot publish prepared work"
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Only the fixture's original local extraction made a model call"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_discovery_preparation_releases_writers_and_timeouts_are_revisited() {
    use recollect_server::autonomous::support_discovery;
    let (h, _, base, p, server, template) = direct_audit_fixture().await;
    let cx = context(&h, &base).await;
    let target = Uuid::from_u128(100);
    discovery_clone(&h, &base, &template, target, false).await;
    // A test-only native gate pauses a canonical check inside the real prepare
    // transaction. The production helper and complete graph remain underneath.
    sqlx::raw_sql(
        "ALTER FUNCTION recollect_memory_exact_acyclic(uuid,uuid) RENAME TO fixture_exact_acyclic;
      CREATE FUNCTION recollect_memory_exact_acyclic(b uuid,target uuid) RETURNS boolean
      LANGUAGE plpgsql STABLE SET search_path=public,pg_temp AS $$ BEGIN
       PERFORM pg_advisory_xact_lock(67126123,7);
       RETURN fixture_exact_acyclic(b,target);
      END $$;",
    )
    .execute(&h.admin)
    .await
    .unwrap();
    let mut gate = h.admin.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(67126123,7)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let state = h.state.clone();
    let preparing =
        tokio::spawn(async move { support_discovery::prepare(&state, cx.brain, cx.actor).await });
    let mut entered = false;
    for _ in 0..100 {
        entered = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND classid=67126123 AND objid=7 AND NOT granted)")
            .fetch_one(&h.admin).await.unwrap();
        if entered {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(
        entered,
        "The actual prepare transaction reached the blocked complete canonical check"
    );
    let writer = async {
        let mut tx = db::actor_tx(&h.state.pool, cx.actor).await.ok().unwrap();
        db::require_writer(&mut tx, cx.brain).await.ok().unwrap();
        sqlx::query("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1")
            .bind(cx.brain)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    };
    tokio::time::timeout(std::time::Duration::from_millis(500), writer)
        .await
        .expect("Same-Brain writer must commit while complete audit preparation is paused");
    sqlx::query("SELECT pg_advisory_unlock(67126123,7)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let prepared = preparing.await.unwrap().ok().unwrap().unwrap();
    assert_eq!(
        support_discovery::publish(&h.state, prepared)
            .await
            .ok()
            .unwrap(),
        1
    );
    // The next candidate actually reaches the native statement deadline. It
    // gets no assessment and the saved walk advances, then retries after wrap.
    let target = Uuid::from_u128(200);
    discovery_clone(&h, &base, &template, target, false).await;
    sqlx::query("SELECT pg_advisory_lock(67126123,7)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let prepared = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    assert_eq!(
        support_discovery::publish(&h.state, prepared)
            .await
            .ok()
            .unwrap(),
        0
    );
    let attempts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM memory_support_assessments WHERE brain_id=$1 AND revision_id=$2",
    )
    .bind(cx.brain)
    .bind(target)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        attempts, 0,
        "A timeout cannot be turned into a verdict or recorded attempt"
    );
    sqlx::query("SELECT pg_advisory_unlock(67126123,7)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let prepared = support_discovery::prepare(&h.state, cx.brain, cx.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    assert_eq!(
        support_discovery::publish(&h.state, prepared)
            .await
            .ok()
            .unwrap(),
        1
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    drop(gate);
    server.abort();
    h.finish().await;
}

async fn audit_candidates_match(h: &Harness, base: &str) -> Vec<Uuid> {
    let cx = context(h, base).await;
    let policy: Uuid =
        sqlx::query_scalar("SELECT policy_id FROM model_policy_heads WHERE brain_id=$1")
            .bind(cx.brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let mut tx = db::actor_tx(&h.state.pool, cx.actor).await.ok().unwrap();
    let mut selected = Vec::new();
    for (old, new) in [
        (
            include_str!("../fixtures/support_audit_current_before_qualification.sql"),
            include_str!("../../src/memory_support_audit/current_candidates.sql"),
        ),
        (
            include_str!("../fixtures/support_audit_historical_before_qualification.sql"),
            include_str!("../../src/memory_support_audit/historical_candidates.sql"),
        ),
    ] {
        for limit in [0i64, 1, 4] {
            let before: Vec<Uuid> = sqlx::query_scalar(old)
                .bind(cx.brain)
                .bind(policy)
                .bind("source-support-3")
                .bind(limit)
                .fetch_all(&mut *tx)
                .await
                .unwrap();
            let after: Vec<Uuid> = sqlx::query_scalar(new)
                .bind(cx.brain)
                .bind(policy)
                .bind("source-support-3")
                .bind(limit)
                .fetch_all(&mut *tx)
                .await
                .unwrap();
            assert_eq!(
                after, before,
                "Current/historical audit selection preserves exact order, eligibility and capacity"
            );
            if limit == 4 {
                selected.extend(after);
            }
        }
    }
    tx.rollback().await.unwrap();
    selected
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_audit_discovery_all_recorded_states_policy_verifier_and_rls_match() {
    let (h, _, base, p, server, revision) = direct_audit_fixture().await;
    let cx = context(&h, &base).await;
    let policy: Uuid =
        sqlx::query_scalar("SELECT policy_id FROM model_policy_heads WHERE brain_id=$1")
            .bind(cx.brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let old = include_str!("../fixtures/support_audit_current_before_qualification.sql");
    let new = include_str!("../../src/memory_support_audit/current_candidates.sql");
    let calls = p.calls.load(Ordering::SeqCst);
    let mut tx = db::actor_tx(&h.state.pool, cx.actor).await.ok().unwrap();
    for state in ["queued", "running", "succeeded", "failed", "removed"] {
        sqlx::query("SAVEPOINT fixture_state")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE memory_support_assessments SET state=$3,disposition=CASE WHEN $3='succeeded' THEN 'supported' ELSE NULL END,
            privacy_state=CASE WHEN $3='removed' THEN 'erased' ELSE 'active' END,reason=NULL WHERE brain_id=$1 AND revision_id=$2")
            .bind(cx.brain).bind(uuid(&revision["id"])).bind(state).execute(&mut *tx).await.unwrap();
        for (selected_policy, verifier, expected) in [
            (policy, "source-support-3", 0),
            (Uuid::new_v4(), "source-support-3", 1),
            (policy, "other-verifier", 1),
        ] {
            let before: Vec<Uuid> = sqlx::query_scalar(old)
                .bind(cx.brain)
                .bind(selected_policy)
                .bind(verifier)
                .bind(4i64)
                .fetch_all(&mut *tx)
                .await
                .unwrap();
            let after: Vec<Uuid> = sqlx::query_scalar(new)
                .bind(cx.brain)
                .bind(selected_policy)
                .bind(verifier)
                .bind(4i64)
                .fetch_all(&mut *tx)
                .await
                .unwrap();
            assert_eq!(after, before);
            assert_eq!(
                after.len(),
                expected,
                "Only the exact recorded policy/verifier tuple suppresses rediscovery, in every state"
            );
            let qualified: Option<Uuid> =
                sqlx::query_scalar(include_str!("../../src/memory_support_audit/qualify.sql"))
                    .bind(cx.brain)
                    .bind(selected_policy)
                    .bind(verifier)
                    .bind(uuid(&revision["id"]))
                    .fetch_optional(&mut *tx)
                    .await
                    .unwrap();
            assert_eq!(
                usize::from(qualified.is_some()),
                expected,
                "The production point qualification preserves every recorded state and exact tuple"
            );
            let batch: Vec<Uuid> =
                sqlx::query_scalar(include_str!("../../src/memory_support_audit/batch.sql"))
                    .bind(cx.brain)
                    .bind(selected_policy)
                    .bind(verifier)
                    .bind(None::<Uuid>)
                    .bind(16i64)
                    .bind(None::<Uuid>)
                    .fetch_all(&mut *tx)
                    .await
                    .unwrap();
            assert_eq!(
                batch.len(),
                expected,
                "Recorded identities are excluded before bounded examination"
            );
        }
        sqlx::query("ROLLBACK TO SAVEPOINT fixture_state")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("RELEASE SAVEPOINT fixture_state")
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.rollback().await.unwrap();
    let (foreign, _) = h.fixture_member().await;
    let mut tx = db::actor_tx(&h.state.pool, foreign).await.ok().unwrap();
    for query in [
        old,
        new,
        include_str!("../../src/memory_support_audit/historical_candidates.sql"),
    ] {
        let hidden: Vec<Uuid> = sqlx::query_scalar(query)
            .bind(cx.brain)
            .bind(policy)
            .bind("source-support-3")
            .bind(4i64)
            .fetch_all(&mut *tx)
            .await
            .unwrap();
        assert!(hidden.is_empty());
    }
    tx.rollback().await.unwrap();
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        calls,
        "Selection does not call a model or resend recorded work"
    );
    server.abort();
    h.finish().await;
}

async fn original_support_predicates_match(h: &Harness, brain: Uuid) {
    let mut tx = h.admin.begin().await.unwrap();
    for (source, name, original) in [
        (
            include_str!("../../migrations/037_memory_revision_support.sql"),
            "recollect_memory_dependencies",
            "original_memory_dependencies",
        ),
        (
            include_str!("../../migrations/037_memory_revision_support.sql"),
            "recollect_revision_supported",
            "original_revision_supported",
        ),
        (
            include_str!("../../migrations/037_memory_revision_support.sql"),
            "recollect_memory_supported",
            "original_pre_digest_supported",
        ),
        (
            include_str!("../../migrations/039_automatic_session_digests.sql"),
            "recollect_memory_supported",
            "original_memory_supported",
        ),
    ] {
        let block = source
            .split(&format!("CREATE FUNCTION {name}("))
            .nth(1)
            .unwrap()
            .split("$$;")
            .next()
            .unwrap();
        let mut statement = format!("CREATE FUNCTION pg_temp.{original}({block}$$;");
        statement = statement
            .replace(
                "recollect_revision_supported(",
                "pg_temp.original_revision_supported(",
            )
            .replace(
                "recollect_memory_dependencies(",
                "pg_temp.original_memory_dependencies(",
            )
            .replace(
                "recollect_pre_digest_supported(",
                "pg_temp.original_pre_digest_supported(",
            );
        sqlx::query(&statement).execute(&mut *tx).await.unwrap();
    }
    let mismatches: i64 = sqlx::query_scalar("WITH targets(id) AS (
        SELECT id FROM claim_revisions WHERE brain_id=$1 UNION SELECT gen_random_uuid()
      ) SELECT count(*) FROM targets WHERE
        ARRAY(SELECT revision_id FROM recollect_memory_dependencies($1,id) ORDER BY revision_id)
          IS DISTINCT FROM ARRAY(SELECT revision_id FROM pg_temp.original_memory_dependencies($1,id) ORDER BY revision_id)
        OR
        recollect_revision_supported($1,id) IS DISTINCT FROM pg_temp.original_revision_supported($1,id)
        OR recollect_pre_digest_supported($1,id) IS DISTINCT FROM pg_temp.original_pre_digest_supported($1,id)
        OR recollect_memory_supported($1,id) IS DISTINCT FROM pg_temp.original_memory_supported($1,id)")
        .bind(brain).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(
        mismatches, 0,
        "False-only fast paths must preserve original support predicates"
    );
    tx.rollback().await.unwrap();
}

async fn permit(h: &Harness, owner: &Login, base: &str, daily: u64) {
    allow(h, owner, base, |p| {
        p["autonomous_memory"] = json!(true);
        p["purposes"] = json!(["extraction", "synthesis"]);
        p["content_classes"] = json!(["document", "claim", "query"]);
        p["daily_token_limit"] = json!(daily);
    })
    .await;
}
fn verdicts(dispositions: &[&str]) -> Value {
    json!({"assessments":dispositions.iter().enumerate().map(|(i,d)|json!({"index":i,"disposition":d,"reason":"Controlled synthetic assessment"})).collect::<Vec<_>>()})
}

async fn reviewed(h: &Harness, owner: &Login, base: &str, r: &Value) -> Value {
    ok(h,"POST",&format!("{base}/claims/{}/review",r["claim_id"].as_str().unwrap()),owner,
       json!({"base_revision":r["id"],"action":"accept","reason":"Explicit isolated synthetic human review","content":null,"revalidation_basis":null})).await["claims"][0]["revision"].clone()
}
async fn guard(h: &Harness, base: &str, r: &Value) -> bool {
    sqlx::query_scalar("SELECT recollect_memory_supported($1,$2)")
        .bind(brain_id(base))
        .bind(uuid(&r["id"]))
        .fetch_one(&h.admin)
        .await
        .unwrap()
}
async fn direct_audit_fixture() -> (
    Harness,
    Login,
    String,
    Arc<Provider>,
    tokio::task::JoinHandle<()>,
    Value,
) {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let source = source(&h, &owner, &base, "Amber.configuration = 8080\n").await;
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        1
    );
    model_job(&h).await;
    let r = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&source["version"]["id"], "Amber", "8080"),
    )
    .await;
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        1
    );
    (h, owner, base, p, server, r)
}
async fn exhaust_database(h: &Harness) -> worker::ClaimedJob {
    exhaust_job_database(h, "claim.support").await
}
async fn exhaust_job_database(h: &Harness, kind: &str) -> worker::ClaimedJob {
    let mut last = None;
    loop {
        let job = worker::claim(&h.state.pool, "model")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(job.kind, kind);
        assert!(
            worker::fail(&h.state.pool, &job, worker::Failure::Database)
                .await
                .unwrap()
        );
        let terminal: bool = sqlx::query_scalar("SELECT state='failed' FROM jobs WHERE id=$1")
            .bind(job.id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
        last.replace(job);
        if terminal {
            break;
        }
        sqlx::query("UPDATE jobs SET not_before=clock_timestamp()-interval '1 second' WHERE id=$1")
            .bind(last.as_ref().unwrap().id)
            .execute(&h.admin)
            .await
            .unwrap();
    }
    last.unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_audit_local_recovery_is_bounded_and_preserves_operation_attempts() {
    let (h, _owner, base, p, server, r) = direct_audit_fixture().await;
    let job = exhaust_database(&h).await;
    let state: String =
        sqlx::query_scalar("SELECT state FROM memory_support_assessments WHERE id=$1")
            .bind(job.target_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        state, "failed",
        "Exhaustion cannot leave phantom queued work"
    );
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        0
    );
    for (round, minutes) in [(1, 6), (2, 31)] {
        sqlx::query(
            "UPDATE jobs SET updated_at=clock_timestamp()-make_interval(mins=>$2) WHERE id=$1",
        )
        .bind(job.id)
        .bind(minutes)
        .execute(&h.admin)
        .await
        .unwrap();
        assert_eq!(
            recollect_server::autonomous::run_once(&h.state)
                .await
                .ok()
                .unwrap(),
            1
        );
        let generation:(Uuid,i32,i32,i32)=sqlx::query_as("SELECT a.id,a.local_recoveries,j.attempts,j.max_attempts FROM memory_support_assessments a JOIN jobs j ON j.id=a.job_id WHERE a.id=$1").bind(job.target_id).fetch_one(&h.admin).await.unwrap();
        assert_eq!(
            generation,
            (job.target_id, round, round * 3, (round + 1) * 3)
        );
        assert_eq!(exhaust_database(&h).await.target_id, job.target_id);
        if round == 1 {
            sqlx::query(
                "UPDATE jobs SET updated_at=clock_timestamp()-interval '6 minutes' WHERE id=$1",
            )
            .bind(job.id)
            .execute(&h.admin)
            .await
            .unwrap();
            assert_eq!(
                recollect_server::autonomous::run_once(&h.state)
                    .await
                    .ok()
                    .unwrap(),
                0,
                "Second local backoff is thirty minutes"
            );
        }
    }
    sqlx::query("UPDATE jobs SET updated_at=clock_timestamp()-interval '8 days' WHERE id=$1")
        .bind(job.id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        0,
        "Two local rounds are the bound"
    );
    assert!(
        worker::claim(&h.state.pool, "model")
            .await
            .unwrap()
            .is_none()
    );
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE id=$1)")
        .bind(job.id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(
        exists,
        "Seven-day generic cleanup preserves audit-owned jobs"
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "No audit attempt was admitted"
    );
    assert!(!guard(&h, &base, &r).await);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_audit_recorded_attempt_and_stale_worker_lease_never_admit_another_call() {
    let (h, _owner, base, p, server, r) = direct_audit_fixture().await;
    let old = worker::claim(&h.state.pool, "model")
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE jobs SET lease_until=clock_timestamp()-interval '1 second' WHERE id=$1")
        .bind(old.id)
        .execute(&h.admin)
        .await
        .unwrap();
    let current = worker::claim(&h.state.pool, "model")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.id, old.id);
    assert_ne!(current.lease_token, old.lease_token);
    let mut invocation =
        gateway::extraction(old.target_id, uuid(&r["content"]["supports"][0]["id"]));
    invocation.work_lease = Some((old.id, old.lease_token));
    let error = gateway::invoke(&h.state, context(&h, &base).await, invocation)
        .await
        .err()
        .unwrap();
    assert_eq!(error.1, "job_lease_lost");
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "A successor lease cannot admit the old worker's request"
    );
    sqlx::raw_sql("ALTER TABLE memory_support_assessments ADD CONSTRAINT audit_verdict_probe CHECK(state<>'succeeded') NOT VALID").execute(&h.admin).await.unwrap();
    assert_eq!(
        worker::execute(&h.state, &current).await,
        Err(worker::Failure::Database)
    );
    assert!(
        worker::fail(&h.state.pool, &current, worker::Failure::Database)
            .await
            .unwrap()
    );
    sqlx::query("UPDATE jobs SET not_before=clock_timestamp()-interval '1 second' WHERE id=$1")
        .bind(old.id)
        .execute(&h.admin)
        .await
        .unwrap();
    exhaust_database(&h).await;
    sqlx::raw_sql("ALTER TABLE memory_support_assessments DROP CONSTRAINT audit_verdict_probe")
        .execute(&h.admin)
        .await
        .unwrap();
    for (state, suppressed) in [
        ("succeeded", false),
        ("running", false),
        ("uncertain", false),
        ("succeeded", true),
    ] {
        sqlx::query("UPDATE model_requests SET state=$2,suppressed=$3 WHERE operation_id=$1")
            .bind(old.target_id)
            .bind(state)
            .bind(suppressed)
            .execute(&h.admin)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE jobs SET updated_at=clock_timestamp()-interval '31 minutes' WHERE id=$1",
        )
        .bind(old.id)
        .execute(&h.admin)
        .await
        .unwrap();
        assert_eq!(
            recollect_server::autonomous::run_once(&h.state)
                .await
                .ok()
                .unwrap(),
            0,
            "Recorded {state} outcome cannot resume as a first call"
        );
    }
    let recoveries: i32 =
        sqlx::query_scalar("SELECT local_recoveries FROM memory_support_assessments WHERE id=$1")
            .bind(old.target_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(recoveries, 0);
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    assert!(!guard(&h, &base, &r).await);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_audit_saved_verdict_resumes_after_exhausted_status_and_lease_without_recharge() {
    let (h, _owner, base, p, server, r) = direct_audit_fixture().await;
    sqlx::raw_sql("ALTER TABLE jobs ADD CONSTRAINT audit_status_probe CHECK(kind<>'claim.support' OR state<>'succeeded') NOT VALID").execute(&h.admin).await.unwrap();
    let id: Uuid =
        sqlx::query_scalar("SELECT job_id FROM memory_support_assessments WHERE revision_id=$1")
            .bind(uuid(&r["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    for _ in 0..3 {
        model_job(&h).await;
        sqlx::query("UPDATE jobs SET not_before=clock_timestamp()-interval '1 second' WHERE id=$1")
            .bind(id)
            .execute(&h.admin)
            .await
            .unwrap();
    }
    let pair:(String,String)=sqlx::query_as("SELECT a.state,j.state FROM memory_support_assessments a JOIN jobs j ON j.id=a.job_id WHERE a.revision_id=$1").bind(uuid(&r["id"])).fetch_one(&h.admin).await.unwrap();
    assert_eq!(pair, ("succeeded".into(), "failed".into()));
    assert!(
        guard(&h, &base, &r).await,
        "Committed support survives bookkeeping faults"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    sqlx::raw_sql("ALTER TABLE jobs DROP CONSTRAINT audit_status_probe")
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET updated_at=clock_timestamp()-interval '6 minutes' WHERE id=$1")
        .bind(id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        1
    );
    // Claim, then expire the last native lease to exercise the same preserved
    // verdict through the worker's exhausted lease path.
    let job = worker::claim(&h.state.pool, "model")
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE jobs SET attempts=max_attempts,lease_until=clock_timestamp()-interval '1 second' WHERE id=$1").bind(id).execute(&h.admin).await.unwrap();
    assert!(
        worker::claim(&h.state.pool, "model")
            .await
            .unwrap()
            .is_none()
    );
    let pair:(String,String)=sqlx::query_as("SELECT a.state,j.error_code FROM memory_support_assessments a JOIN jobs j ON j.id=a.job_id WHERE a.id=$1").bind(job.target_id).fetch_one(&h.admin).await.unwrap();
    assert_eq!(pair, ("succeeded".into(), "lease_expired".into()));
    sqlx::query("UPDATE jobs SET updated_at=clock_timestamp()-interval '31 minutes' WHERE id=$1")
        .bind(id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        1
    );
    model_job(&h).await;
    let status: String = sqlx::query_scalar("SELECT state FROM jobs WHERE id=$1")
        .bind(id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(status, "succeeded");
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        2,
        "Saved verdict never invokes gateway again"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_audit_inflight_erasure_and_older_backup_scrub_reasons_preserve_control() {
    let (mut h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let original = source(&h, &owner, &base, "Amber.configuration = 8080\n").await;
    let independent = source(&h, &owner, &base, "Birch.configuration = 3030\n").await;
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        2
    );
    model_job(&h).await;
    model_job(&h).await;
    let old = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&original["version"]["id"], "Amber", "8080"),
    )
    .await;
    let control = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&independent["version"]["id"], "Birch", "3030"),
    )
    .await;
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        2
    );
    model_job(&h).await;
    model_job(&h).await;
    assert!(guard(&h, &base, &old).await && guard(&h, &base, &control).await);
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable support restore fixture: {backup}");
    h.state.pool.close().await;
    h.admin.close().await;
    sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'")).execute(&h.root).await.unwrap();
    let mut admin_url = reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
    admin_url.set_path(&h.database);
    h.admin = db::pool(admin_url.as_str()).await.unwrap();
    h.state = AppState::new(
        db::pool(&h.state.config.database_url).await.unwrap(),
        (*h.state.config).clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    admin_url.set_path(&backup);
    let backup_admin = db::pool(admin_url.as_str()).await.unwrap();
    let mut backup_config = (*h.state.config).clone();
    let mut url = reqwest::Url::parse(&backup_config.database_url).unwrap();
    url.set_path(&backup);
    backup_config.database_url = url.to_string();
    let backup_state = AppState::new(
        db::pool(&backup_config.database_url).await.unwrap(),
        backup_config.clone(),
    )
    .unwrap();
    let mut changed = review::proposal(&original["version"]["id"], "Amber", "8080");
    changed["base_revision"] = old["id"].clone();
    changed["content"]["rationale"] =
        json!("Recheck this same declaration against the exact retained source.");
    let current = ok(
        &h,
        "PUT",
        &format!("{base}/claims/{}", old["claim_id"].as_str().unwrap()),
        &owner,
        changed,
    )
    .await;
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        1
    );
    p.delay.store(450, Ordering::SeqCst);
    let state = h.state.clone();
    let inflight = tokio::spawn(async move { worker::run_once(&state, "model").await });
    wait_calls(&p, 5).await;
    let target = json!({"kind":"claim","id":old["claim_id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert!(inflight.await.unwrap().unwrap());
    let audits:Vec<(String,Option<String>,Option<String>)>=sqlx::query_as("SELECT privacy_state,reason,disposition FROM memory_support_assessments WHERE revision_id IN ($1,$2)")
        .bind(uuid(&old["id"])).bind(uuid(&current["id"])).fetch_all(&h.admin).await.unwrap();
    assert_eq!(audits.len(), 2);
    assert!(
        audits
            .iter()
            .all(|a| a.0 == "erased" && a.1.is_none() && a.2.is_none())
    );
    let suppressed:bool=sqlx::query_scalar("SELECT suppressed FROM model_requests WHERE operation_id=(SELECT id FROM memory_support_assessments WHERE revision_id=$1)")
        .bind(uuid(&current["id"])).fetch_one(&h.admin).await.unwrap();
    assert!(
        suppressed,
        "In-flight response cannot publish after erasure"
    );
    assert!(guard(&h, &base, &control).await);
    recollect_server::privacy_journal::run_once(&h.state)
        .await
        .unwrap();
    assert!(
        recollect_server::privacy_journal::barrier(&backup_state.pool, &backup_config)
            .await
            .is_err()
    );
    recollect_server::privacy_journal::reconcile(&backup_admin, &backup_config)
        .await
        .unwrap();
    recollect_server::privacy_journal::barrier(&backup_state.pool, &backup_config)
        .await
        .unwrap();
    let erased:(String,Option<String>,Option<String>)=sqlx::query_as("SELECT privacy_state,reason,disposition FROM memory_support_assessments WHERE revision_id=$1")
        .bind(uuid(&old["id"])).fetch_one(&backup_admin).await.unwrap();
    assert_eq!(erased, ("erased".into(), None, None));
    let positive: bool = sqlx::query_scalar("SELECT recollect_memory_supported($1,$2)")
        .bind(brain_id(&base))
        .bind(uuid(&control["id"]))
        .fetch_one(&backup_admin)
        .await
        .unwrap();
    assert!(
        positive,
        "Older-backup replay preserves the unrelated supported control"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 5);
    backup_state.pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_cutover_invalidates_cached_answers_before_first_audit() {
    let index = db::MIGRATIONS
        .iter()
        .position(|(name, _)| *name == "037_memory_revision_support")
        .unwrap();
    let mut h = Harness::new_through(index).await;
    let (p, server) = configure_provider(&mut h).await;
    let owner = h.login().await;
    let b = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic cutover"}),
    )
    .await;
    let base = format!("/api/brains/{}", b["id"].as_str().unwrap());
    permit(&h, &owner, &base, 100000).await;
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["answering", "extraction", "synthesis"])
    })
    .await;
    let cx = context(&h, &base).await;
    let id = Uuid::new_v4();
    // A completed pre-cutover answer stores only identities/epoch, never prose.
    sqlx::query("INSERT INTO answer_requests(id,brain_id,actor_id,call_token,state,policy_id,memory_epoch,expires_at) SELECT $1,$2,$3,$4,'completed',p.policy_id,e.epoch,clock_timestamp()+interval '1 hour' FROM model_policy_heads p JOIN memory_epochs e ON e.brain_id=p.brain_id WHERE p.brain_id=$2")
        .bind(id).bind(cx.brain).bind(cx.actor).bind(Uuid::new_v4()).execute(&h.admin).await.unwrap();
    let path = format!("{base}/answer-requests/{id}");
    assert_eq!(
        ok(&h, "GET", &path, &owner, Value::Null).await["state"],
        "completed"
    );
    db::migrate(&h.admin).await.unwrap();
    assert_eq!(
        ok(&h, "GET", &path, &owner, Value::Null).await["state"],
        "stale"
    );
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM memory_support_assessments")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        audits, 0,
        "Cutover invalidation must not depend on the first audit"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_guard_honors_scope_rules_half_open_time_and_exact_human_exceptions() {
    let (h, owner, base, p, server) = setup().await;
    let evidence = source(&h, &owner, &base, "Synthetic declaration only.\n").await;
    let mut areas = Vec::new();
    for name in ["Synthetic area A", "Synthetic area B"] {
        areas.push(
            ok(
                &h,
                "POST",
                &format!("{base}/evidence/groups"),
                &owner,
                json!({"kind":"area","name":name}),
            )
            .await["id"]
                .clone(),
        );
    }
    let mut first = review::proposal(&evidence["version"]["id"], "Amber", "8080");
    first["content"]["selection"]["area_ids"] = json!([areas[0]]);
    let first = ok(&h, "POST", &format!("{base}/claims"), &owner, first).await;
    let first = reviewed(&h, &owner, &base, &first).await;
    let mut second = review::proposal(&evidence["version"]["id"], "Amber", "8080");
    second["content"]["selection"]["area_ids"] = json!([areas[1]]);
    let second = ok(&h, "POST", &format!("{base}/claims"), &owner, second).await;
    let second = reviewed(&h, &owner, &base, &second).await;
    let mut adjacent = review::proposal(&evidence["version"]["id"], "Amber", "8080");
    adjacent["content"]["validity"]["from"] = json!("2026-02-01T00:00:00Z");
    adjacent["content"]["validity"]["to"] = json!("2026-03-01T00:00:00Z");
    let adjacent = ok(&h, "POST", &format!("{base}/claims"), &owner, adjacent).await;
    let adjacent = reviewed(&h, &owner, &base, &adjacent).await;
    assert!(guard(&h, &base, &second).await && guard(&h, &base, &adjacent).await);
    ok(&h,"POST",&format!("{base}/claims/{}/review",first["claim_id"].as_str().unwrap()),&owner,
       json!({"base_revision":first["id"],"action":"reject","reason":"Synthetic declaration rejected for this interval","content":null,"revalidation_basis":null})).await;
    assert!(
        !guard(&h, &base, &second).await,
        "Review rules ignore area separation just like Rust matching"
    );
    assert!(
        guard(&h, &base, &adjacent).await,
        "Adjacent half-open intervals do not overlap"
    );
    let restored=ok(&h,"POST",&format!("{base}/claims/{}/review",second["claim_id"].as_str().unwrap()),&owner,
       json!({"base_revision":second["id"],"action":"revalidate","reason":"Explicit human exception for this exact revision","content":null,"revalidation_basis":"review_correction"})).await["claims"][0]["revision"].clone();
    assert!(guard(&h, &base, &restored).await);
    assert!(
        !guard(&h, &base, &second).await,
        "An exception never leaks to an older revision"
    );
    for (ids, valid) in [
        (json!([areas[0]]), true),
        (json!([areas[0], areas[0]]), false),
        (json!([Uuid::new_v4()]), false),
    ] {
        let result: bool = sqlx::query_scalar("SELECT recollect_selection_valid($1,$2)")
            .bind(brain_id(&base))
            .bind(json!({"repository_ids":[],"area_ids":ids,"environment_id":null}))
            .fetch_one(&h.admin)
            .await
            .unwrap();
        assert_eq!(result, valid);
    }
    let historical:bool=sqlx::query_scalar("SELECT lower(recollect_validity_range($1))='1969-12-31T23:59:00Z'::timestamptz AND upper(recollect_validity_range($1))='1970-01-01T00:00:00Z'::timestamptz")
        .bind(json!({"kind":"point","from":"1969-12-31T23:59:59Z","precision":"minute"})).fetch_one(&h.admin).await.unwrap();
    assert!(
        historical,
        "Point buckets use floor before the UTC epoch too"
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        0,
        "Human scope/rule controls need no provider call"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_handover_current_children_and_reference_only_human_controls() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let evidence = source(&h, &owner, &base, "Amber.configuration = 8080\n").await;
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    recollect_server::autonomous::run_once(&h.state)
        .await
        .ok()
        .unwrap();
    model_job(&h).await;
    let child = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&evidence["version"]["id"], "Amber", "8080"),
    )
    .await;
    recollect_server::autonomous::run_once(&h.state)
        .await
        .ok()
        .unwrap();
    model_job(&h).await;
    let mut input = review::proposal(
        &evidence["version"]["id"],
        "Synthetic handover",
        "The source records the configuration declaration.",
    );
    input["content"]["kind"] = json!("handover");
    input["content"]["handover"] = json!({"completed":["Recorded the declaration."],"next_steps":["Verify runtime separately."],"risks":["A declaration does not prove health."],"contributions":[child["id"]]});
    let handover = ok(&h, "POST", &format!("{base}/claims"), &owner, input.clone()).await;
    let human = reviewed(&h, &owner, &base, &handover).await;
    assert!(guard(&h, &base, &human).await);
    let child_path = format!("{base}/claims/{}", child["claim_id"].as_str().unwrap());
    let mut changed = review::proposal(&evidence["version"]["id"], "Amber", "8080");
    changed["base_revision"] = child["id"].clone();
    changed["content"]["rationale"] = json!("A newer unreviewed exact contribution.");
    let latest = ok(&h, "PUT", &child_path, &owner, changed).await;
    assert!(
        !guard(&h, &base, &human).await,
        "Shared SQL must notice unchecked current child v2"
    );
    recollect_server::autonomous::run_once(&h.state)
        .await
        .ok()
        .unwrap();
    model_job(&h).await;
    assert!(
        guard(&h, &base, &human).await,
        "Eligible v1 plus eligible v2 preserve qualified history"
    );
    // The unreviewed original handover still references v1. Auditing existing
    // immutable contributions must not impose creation-time current-head rules.
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    recollect_server::autonomous::run_once(&h.state)
        .await
        .unwrap_or_else(|e| panic!("Coordinator failed: {}", e.1));
    while worker::run_once(&h.state, "model").await.unwrap() {}
    // Create an unreviewed copy while v1 is still the current input at creation,
    // then advance to another supported child before the coordinator audits it.
    input["content"]["subject"] = json!("Audit old contributing revision");
    input["content"]["handover"]["contributions"] = json!([latest["id"]]);
    let old = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
    let mut changed = review::proposal(&evidence["version"]["id"], "Amber", "8080");
    changed["base_revision"] = latest["id"].clone();
    let newer = ok(&h, "PUT", &child_path, &owner, changed).await;
    // Emulate the upgrade boundary: exact historical contributors have no
    // support assessments, and the current child has never been assessed.
    sqlx::query("DELETE FROM memory_support_assessments WHERE revision_id IN ($1,$2)")
        .bind(uuid(&child["id"]))
        .bind(uuid(&latest["id"]))
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        3
    );
    for _ in 0..3 {
        model_job(&h).await;
    }
    assert!(guard(&h, &base, &newer).await);
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("Coordinator failed: {}", e.1)),
        1
    );
    let calls = p.calls.load(Ordering::SeqCst);
    model_job(&h).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        calls + 1,
        "Read/audit path reaches the assessor"
    );
    assert!(guard(&h, &base, &old).await);

    let reference=ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Synthetic external reference","media_type":"text/plain","source_uri":"https://example.test/reference","retain_content":false})).await;
    let mut input = review::proposal(
        &reference["version"]["id"],
        "Reference-only declaration",
        "Inspect the original reference before use.",
    );
    input["content"]["supports"][0]["line_from"] = Value::Null;
    input["content"]["supports"][0]["line_to"] = Value::Null;
    let r = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
    assert!(!guard(&h, &base, &r).await);
    let human = reviewed(&h, &owner, &base, &r).await;
    let detail = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", human["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert!(guard(&h, &base, &human).await);
    assert_eq!(detail["selected"]["eligibility"]["investigation"], true);
    assert_eq!(detail["selected"]["eligibility"]["strict_accepted"], false);
    server.abort();
    h.finish().await;
}
fn brain_id(base: &str) -> Uuid {
    base.rsplit('/').next().unwrap().parse().unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_dependency_deadline_invalidates_investigation_and_cache_before_cleanup() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["answering", "extraction", "synthesis"])
    })
    .await;
    let evidence = source(&h, &owner, &base, "Amber.configuration = 8080\n").await;
    let child = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&evidence["version"]["id"], "Amber", "8080"),
    )
    .await;
    let child = reviewed(&h, &owner, &base, &child).await;
    let mut input = review::proposal(
        &evidence["version"]["id"],
        "Synthetic continuation",
        "Recorded Amber configuration.",
    );
    input["content"]["kind"] = json!("handover");
    input["content"]["handover"] =
        json!({"completed":[],"next_steps":[],"risks":[],"contributions":[child["id"]]});
    let parent = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
    let parent = reviewed(&h, &owner, &base, &parent).await;
    let mut corrected_content = child["content"].clone();
    corrected_content["rationale"] = json!("Synthetic revalidation of the same declaration.");
    let latest = ok(&h,"POST",&format!("{base}/claims/{}/review", child["claim_id"].as_str().unwrap()),&owner,
        json!({"base_revision":child["id"],"action":"revalidate","reason":"Synthetic exact current-child revalidation","content":corrected_content,"revalidation_basis":"review_correction"})).await["claims"][0]["revision"].clone();
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["claim_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    // Only the canonical current child ages out; the exact contributor, parent
    // and retained source remain active. No sweep or epoch mutation follows.
    sqlx::query("UPDATE claim_revisions SET recorded_at=clock_timestamp()-interval '1 day'+interval '3 seconds' WHERE id=$1")
        .bind(uuid(&latest["id"])).execute(&h.admin).await.unwrap();
    assert!(guard(&h, &base, &parent).await);
    let deadline: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT recollect_memory_deadline($1,$2)")
            .bind(brain_id(&base))
            .bind(uuid(&parent["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let recalled = ok(
        &h,
        "POST",
        &format!("{base}/recall"),
        &owner,
        json!({"query":"Synthetic continuation"}),
    )
    .await;
    assert!(
        recalled["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == parent["claim_id"])
    );
    assert_eq!(recalled["expires_at"], json!(deadline));
    let page = ok(&h, "GET", &format!("{base}/claims"), &owner, Value::Null).await;
    assert_eq!(page["expires_at"], json!(deadline));
    let cx = context(&h, &base).await;
    let cached = Uuid::new_v4();
    sqlx::query("INSERT INTO answer_requests(id,brain_id,actor_id,call_token,state,policy_id,memory_epoch,expires_at) SELECT $1,$2,$3,$4,'completed',p.policy_id,e.epoch,$5 FROM model_policy_heads p JOIN memory_epochs e ON e.brain_id=p.brain_id WHERE p.brain_id=$2")
        .bind(cached).bind(cx.brain).bind(cx.actor).bind(Uuid::new_v4()).bind(deadline).execute(&h.admin).await.unwrap();
    let path = format!("{base}/answer-requests/{cached}");
    assert_eq!(
        ok(&h, "GET", &path, &owner, Value::Null).await["state"],
        "completed"
    );
    let epoch: i64 = sqlx::query_scalar("SELECT epoch FROM memory_epochs WHERE brain_id=$1")
        .bind(cx.brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    assert!(!guard(&h, &base, &parent).await);
    assert_eq!(
        ok(&h, "GET", &path, &owner, Value::Null).await["state"],
        "stale"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT epoch FROM memory_epochs WHERE brain_id=$1")
            .bind(cx.brain)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        epoch
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT privacy_state FROM claim_revisions WHERE id=$1")
            .bind(uuid(&latest["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "active"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_legacy_lineage_audits_bottom_up_and_withholds_exact_cycles() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let evidence = source(&h, &owner, &base, "Amber.configuration = 8080\n").await;
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    recollect_server::autonomous::run_once(&h.state)
        .await
        .ok()
        .unwrap();
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let mut lineage = vec![
        ok(
            &h,
            "POST",
            &format!("{base}/claims"),
            &owner,
            review::proposal(&evidence["version"]["id"], "Amber", "8080"),
        )
        .await,
    ];
    for _ in 0..2 {
        let prior = lineage.last().unwrap();
        let mut input = review::proposal(&evidence["version"]["id"], "Amber", "8080");
        input["base_revision"] = prior["id"].clone();
        let next = ok(
            &h,
            "PUT",
            &format!("{base}/claims/{}", prior["claim_id"].as_str().unwrap()),
            &owner,
            input,
        )
        .await;
        // Emulate pre-guard autonomous::basis, which links previous revisions
        // of the same identity without changing their immutable bodies.
        sqlx::query("INSERT INTO claim_contributions(brain_id,revision_id,input_revision_id) VALUES($1,$2,$3)")
            .bind(brain_id(&base)).bind(uuid(&next["id"])).bind(uuid(&prior["id"])).execute(&h.admin).await.unwrap();
        lineage.push(next);
    }
    // Differential proof against the original canonical target predicate,
    // including exact historical revisions and an absent target. This catches
    // optimizations that accidentally check only current heads.
    let mismatches: i64 = sqlx::query_scalar("WITH targets(id) AS (
        SELECT id FROM claim_revisions WHERE brain_id=$1 UNION SELECT gen_random_uuid()
      ) SELECT count(*) FROM targets t WHERE recollect_support_audit_target($1,t.id) IS DISTINCT FROM EXISTS(
        SELECT 1 FROM claims c JOIN claim_revisions r ON r.brain_id=c.brain_id AND r.id=c.current_revision
        WHERE c.brain_id=$1 AND recollect_content_state($1,'claim',r.privacy_state,r.recorded_at)='active'
          AND r.revision->>'review'<>'rejected' AND coalesce(r.revision->>'lifecycle','active')='active'
          AND EXISTS(SELECT 1 FROM recollect_memory_dependencies($1,r.id) d WHERE d.revision_id=t.id))")
        .bind(brain_id(&base)).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        mismatches, 0,
        "The optimized function must preserve the original current-root and historical dependency predicate"
    );
    for target in &lineage {
        assert_eq!(
            audit_candidates_match(&h, &base).await,
            vec![uuid(&target["id"])],
            "Exact historical predecessors qualify bottom-up before the current root"
        );
        assert_eq!(
            recollect_server::autonomous::run_once(&h.state)
                .await
                .ok()
                .unwrap(),
            1
        );
        model_job(&h).await;
        assert!(
            sqlx::query_scalar::<_, bool>("SELECT recollect_revision_supported($1,$2)")
                .bind(brain_id(&base))
                .bind(uuid(&target["id"]))
                .fetch_one(&h.admin)
                .await
                .unwrap()
        );
    }
    assert!(guard(&h, &base, lineage.last().unwrap()).await);
    let ledger:Vec<Uuid>=sqlx::query_scalar("SELECT i.input_id FROM model_request_inputs i JOIN model_requests m ON m.id=i.request_id JOIN memory_support_assessments a ON a.id=m.operation_id WHERE a.revision_id=$1 AND i.kind='claim_revision'")
        .bind(uuid(&lineage[1]["id"])).fetch_all(&h.admin).await.unwrap();
    assert!(ledger.contains(&uuid(&lineage[0]["id"])));
    assert!(ledger.contains(&uuid(&lineage[2]["id"])));
    let bodies = p.bodies.lock().unwrap().clone();
    let second = bodies
        .iter()
        .find(|v| {
            v["text"]["format"]["name"] == "revision_support"
                && v["input"]
                    .as_str()
                    .unwrap()
                    .contains(lineage[1]["id"].as_str().unwrap())
        })
        .unwrap();
    assert!(
        !second["input"]
            .as_str()
            .unwrap()
            .contains(lineage[2]["id"].as_str().unwrap()),
        "Changed current heads are metadata dependencies, never corroborating bodies"
    );
    // A stored back-edge is a genuine invalid cycle, unlike current aliases.
    assert!(sqlx::query("INSERT INTO claim_contributions(brain_id,revision_id,input_revision_id) VALUES($1,$2,$3)")
        .bind(brain_id(&base)).bind(uuid(&lineage[0]["id"])).bind(uuid(&lineage[2]["id"])).execute(&h.admin).await.is_err());
    sqlx::query("ALTER TABLE claim_contributions DISABLE TRIGGER contribution_acyclic")
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO claim_contributions(brain_id,revision_id,input_revision_id) VALUES($1,$2,$3)",
    )
    .bind(brain_id(&base))
    .bind(uuid(&lineage[0]["id"]))
    .bind(uuid(&lineage[2]["id"]))
    .execute(&h.admin)
    .await
    .unwrap();
    sqlx::query("ALTER TABLE claim_contributions ENABLE TRIGGER contribution_acyclic")
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(!guard(&h, &base, lineage.last().unwrap()).await);
    original_support_predicates_match(&h, brain_id(&base)).await;
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        0
    );
    permit(&h, &owner, &base, 100000).await;
    assert!(
        audit_candidates_match(&h, &base).await.is_empty(),
        "A new policy with no recorded attempts cannot admit an exact cycle"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_manifest_uses_exact_snapshot_lifetime_and_rejects_missing_or_erased_inputs() {
    let (h, owner, base, p, server) = setup().await;
    let evidence = source(&h, &owner, &base, "Synthetic desired configuration only.\n").await;
    let cx = context(&h, &base).await;
    let repo = Uuid::new_v4();
    let snapshot = Uuid::new_v4();
    sqlx::query("INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,'example.test/synthetic/support',$3)")
        .bind(repo).bind(cx.brain).bind(cx.actor).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO repository_snapshots(id,brain_id,repository_id,revision,adapter,adapter_build,extractor_version,settings,coverage,file_count,fact_count,retained_file_count,created_by) VALUES($1,$2,$3,$4,'fixture','fixture','fixture','{}','{}',0,0,0,$5)")
        .bind(snapshot).bind(cx.brain).bind(repo).bind("a".repeat(40)).bind(cx.actor).execute(&h.admin).await.unwrap();
    let env = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Synthetic test"}),
    )
    .await;
    let manifest=ok(&h,"POST",&format!("{base}/revision-manifests"),&owner,json!({"name":"Desired test configuration","environment_id":env["id"],"kind":"desired","entries":[{"repository_id":repo,"revision":"a".repeat(40),"snapshot_id":snapshot,"config_paths":[]}],"notes":"Declared selection only","base_revision":null,"operation_id":null})).await;
    let mut input = review::proposal(
        &evidence["version"]["id"],
        "Synthetic manifest",
        "Desired configuration declaration",
    );
    input["content"]["selection"] =
        json!({"repository_ids":[repo],"area_ids":[],"environment_id":env["id"]});
    input["content"]["manifest_revision_id"] = manifest["id"].clone();
    let r = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
    let r = reviewed(&h, &owner, &base, &r).await;
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["repository_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    // An old manifest over a fresh snapshot remains supported. Its own age is
    // not repository retention; snapshot creation is the canonical deadline.
    sqlx::query(
        "UPDATE manifest_revisions SET created_at=clock_timestamp()-interval '10 days' WHERE id=$1",
    )
    .bind(uuid(&manifest["id"]))
    .execute(&h.admin)
    .await
    .unwrap();
    assert!(guard(&h, &base, &r).await);
    let deadline: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT recollect_memory_deadline($1,$2)")
            .bind(cx.brain)
            .bind(uuid(&r["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(deadline > chrono::Utc::now() + chrono::Duration::hours(23));
    let unrelated_repo = Uuid::new_v4();
    let extra = json!({"repository_id":unrelated_repo,"revision":"b".repeat(40),"snapshot_id":Uuid::new_v4(),"config_paths":["UNSELECTED_SCOPE_SENTINEL"]});
    sqlx::query("UPDATE manifest_revisions SET revision=jsonb_set(revision,'{entries}',(revision->'entries')||jsonb_build_array($2::jsonb)) WHERE id=$1")
        .bind(uuid(&manifest["id"])).bind(sqlx::types::Json(extra)).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE manifest_revisions SET revision=revision||$2::jsonb WHERE id=$1")
        .bind(uuid(&manifest["id"]))
        .bind(sqlx::types::Json(json!({"scope":{"id":Uuid::new_v4(),"task_id":Uuid::new_v4(),"brain_id":cx.brain,"selection":{"repository_ids":[unrelated_repo],"area_ids":[],"environment_id":null},"repositories":[{"id":unrelated_repo,"name":"UNSELECTED_SCOPE_SENTINEL"}],"areas":[],"environment":null,"created_at":chrono::Utc::now()},"notes":"UNSELECTED_NOTES_SENTINEL"})))
        .execute(&h.admin).await.unwrap();
    assert!(
        guard(&h, &base, &r).await,
        "An unavailable unselected snapshot cannot invalidate selected applicability"
    );
    permit(&h, &owner, &base, 100000).await;
    allow(&h, &owner, &base, |p| {
        p["autonomous_memory"] = json!(true);
        p["content_classes"] = json!(["document", "claim", "query", "repository"]);
    })
    .await;
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
    let mut pending = review::proposal(
        &evidence["version"]["id"],
        "Synthetic selected configuration",
        "Desired configuration declaration",
    );
    pending["content"]["selection"] = r["content"]["selection"].clone();
    pending["content"]["manifest_revision_id"] = manifest["id"].clone();
    let assessed = ok(&h, "POST", &format!("{base}/claims"), &owner, pending).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        2
    );
    model_job(&h).await;
    model_job(&h).await;
    assert!(guard(&h, &base, &assessed).await);
    *p.candidates.lock().unwrap() = handover_draft();
    let handover=ok(&h,"POST",&format!("{base}/handovers"),&owner,json!({"title":"Selected configuration continuation","contributions":[assessed["id"]],"operation_id":null})).await;
    model_job(&h).await;
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM handover_runs WHERE id=$1")
            .bind(uuid(&handover["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "succeeded"
    );
    let bodies = p.bodies.lock().unwrap().clone();
    let verifier_inputs = bodies
        .iter()
        .filter(|b| {
            matches!(
                b["text"]["format"]["name"].as_str(),
                Some("revision_support" | "handover_support")
            )
        })
        .map(|b| b["input"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(verifier_inputs.len(), 2);
    for text in verifier_inputs {
        assert!(
            text.contains("manifest_applicability")
                && text.contains(&manifest["id"].as_str().unwrap().to_string())
        );
        assert!(text.contains(&repo.to_string()) && text.contains(&snapshot.to_string()));
        assert!(
            !text.contains("UNSELECTED_SCOPE_SENTINEL")
                && !text.contains("UNSELECTED_NOTES_SENTINEL")
                && !text.contains(&unrelated_repo.to_string())
        );
    }
    drop(bodies);
    // Direct reviewed fact support has the existing qualified-expiry behavior;
    // it does not bypass the selected manifest's stricter snapshot boundary.
    let fact = Uuid::new_v4();
    sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) VALUES($1,$2,$3,0,$4)")
        .bind(fact).bind(cx.brain).bind(snapshot)
        .bind(json!({"kind":"configuration","name":"Direct fixture port","port":8080}))
        .execute(&h.admin).await.unwrap();
    let mut direct = review::proposal(&evidence["version"]["id"], "Direct fixture port", "8080");
    direct["content"]["selection"] =
        json!({"repository_ids":[repo],"area_ids":[],"environment_id":null});
    direct["content"]["supports"] =
        json!([{"kind":"repository_fact","id":fact,"line_from":null,"line_to":null}]);
    let direct = ok(&h, "POST", &format!("{base}/claims"), &owner, direct).await;
    let unchecked = direct.clone();
    let direct = reviewed(&h, &owner, &base, &direct).await;
    assert!(guard(&h, &base, &direct).await);
    sqlx::query("UPDATE manifest_revisions SET revision=jsonb_set(revision,'{environment_id}',to_jsonb($2::uuid)) WHERE id=$1").bind(uuid(&manifest["id"])).bind(Uuid::new_v4()).execute(&h.admin).await.unwrap();
    assert!(
        !guard(&h, &base, &r).await,
        "A manifest from another environment cannot support this claim"
    );
    sqlx::query("UPDATE manifest_revisions SET revision=jsonb_set(revision,'{environment_id}',to_jsonb($2::uuid)) WHERE id=$1").bind(uuid(&manifest["id"])).bind(uuid(&env["id"])).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE manifest_revisions SET revision=jsonb_set(revision,'{entries}',revision->'entries'->1 || '[]'::jsonb) WHERE id=$1").bind(uuid(&manifest["id"])).execute(&h.admin).await.unwrap();
    assert!(
        !guard(&h, &base, &r).await,
        "Every selected repository must have an exact manifest entry"
    );
    sqlx::query("UPDATE manifest_revisions SET revision=jsonb_set(revision,'{entries}',$2::jsonb) WHERE id=$1").bind(uuid(&manifest["id"])).bind(sqlx::types::Json(manifest["entries"].clone())).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE repository_snapshots SET created_at=clock_timestamp()-interval '2 days' WHERE id=$1")
        .bind(snapshot).execute(&h.admin).await.unwrap();
    assert!(
        !guard(&h, &base, &r).await,
        "Logical snapshot expiry blocks before a privacy sweep"
    );
    assert!(
        guard(&h, &base, &direct).await,
        "Explicit review preserves a qualified direct fact claim"
    );
    assert!(
        !guard(&h, &base, &unchecked).await,
        "Human review cannot leak to its unchecked predecessor"
    );
    let direct_view = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", direct["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        direct_view["selected"]["eligibility"]["investigation"],
        true
    );
    assert_eq!(
        direct_view["selected"]["eligibility"]["strict_accepted"],
        false
    );
    let expired_deadline: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT recollect_memory_deadline($1,$2)")
            .bind(cx.brain)
            .bind(uuid(&direct["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(
        expired_deadline.is_none(),
        "An expired fact is not the independently retained reviewed claim's TTL"
    );
    sqlx::query("UPDATE repository_snapshots SET created_at=clock_timestamp() WHERE id=$1")
        .bind(snapshot)
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(guard(&h, &base, &r).await);
    sqlx::query("UPDATE manifest_revisions SET revision=jsonb_set(revision,'{entries,0,snapshot_id}',to_jsonb($2::uuid)) WHERE id=$1")
        .bind(uuid(&manifest["id"])).bind(Uuid::new_v4()).execute(&h.admin).await.unwrap();
    assert!(
        !guard(&h, &base, &r).await,
        "Missing non-null snapshot never disappears through an inner join"
    );
    sqlx::query("UPDATE manifest_revisions SET revision=jsonb_set(revision,'{entries,0,snapshot_id}',to_jsonb($2::uuid)) WHERE id=$1")
        .bind(uuid(&manifest["id"])).bind(snapshot).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE repository_snapshots SET privacy_state='erased' WHERE id=$1")
        .bind(snapshot)
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(!guard(&h, &base, &r).await);
    assert!(
        !guard(&h, &base, &direct).await,
        "Erasure never receives the qualified expiry exception"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    server.abort();
    h.finish().await;
}

fn handover_draft() -> Value {
    json!({"summary":"Amber declares port 8080.","completed":["Read the supplied configuration."],"next_steps":["Confirm runtime if needed."],"risks":["Declared configuration is not runtime proof."]})
}
async fn handover_fixture() -> (
    Harness,
    Login,
    String,
    Arc<Provider>,
    tokio::task::JoinHandle<()>,
    Value,
    Value,
) {
    let (h, owner, base, p, server, r) = direct_audit_fixture().await;
    model_job(&h).await;
    *p.candidates.lock().unwrap() = handover_draft();
    let run=ok(&h,"POST",&format!("{base}/handovers"),&owner,json!({"title":"Synthetic work continuation","contributions":[r["id"]],"operation_id":null})).await;
    (h, owner, base, p, server, r, run)
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_handover_prepublication_and_unsupported_refresh_preserve_prior_head() {
    let (h, owner, base, p, server, child, run) = handover_fixture().await;
    let before = p.calls.load(Ordering::SeqCst);
    model_job(&h).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        before + 2,
        "Synthesis and independent support are separately accounted"
    );
    let page = ok(&h, "GET", &format!("{base}/handovers"), &owner, Value::Null).await;
    let result = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == run["id"])
        .unwrap();
    assert_eq!(result["state"], "succeeded");
    let prior = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", result["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await["selected"]["revision"]
        .clone();
    assert!(
        guard(&h, &base, &prior).await,
        "The canonical support verdict commits with the new head"
    );
    let mut update = review::proposal(&child["content"]["supports"][0]["id"], "Amber", "8080");
    update["base_revision"] = child["id"].clone();
    update["content"]["rationale"] = json!("Updated exact contributor declaration.");
    ok(
        &h,
        "PUT",
        &format!("{base}/claims/{}", child["claim_id"].as_str().unwrap()),
        &owner,
        update,
    )
    .await;
    recollect_server::autonomous::run_once(&h.state)
        .await
        .ok()
        .unwrap();
    model_job(&h).await;
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        1
    );
    *p.assessments.lock().unwrap() = Some(verdicts(&["contradicted"]));
    model_job(&h).await;
    let current: Uuid = sqlx::query_scalar("SELECT current_revision FROM claims WHERE id=$1")
        .bind(uuid(&prior["claim_id"]))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        current,
        uuid(&prior["id"]),
        "Unsupported generated refresh cannot displace prior supported knowledge"
    );
    let withheld:i64=sqlx::query_scalar("SELECT count(*) FROM handover_runs WHERE brain_id=$1 AND base_revision_id=$2 AND state='failed' AND error_code='support_contradicted'")
        .bind(brain_id(&base)).bind(uuid(&prior["id"])).fetch_one(&h.admin).await.unwrap();
    assert_eq!(withheld, 1);
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        0,
        "Semantic disposition is terminal for unchanged inputs"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_handover_saved_verdict_recovers_database_publication_without_recharge() {
    let (h, _owner, base, p, server, _child, run) = handover_fixture().await;
    sqlx::query("ALTER TABLE memory_support_assessments ADD CONSTRAINT synthetic_publication_fault CHECK(source_handover_stage_id IS NULL)").execute(&h.admin).await.unwrap();
    let before = p.calls.load(Ordering::SeqCst);
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), before + 2);
    let saved: bool = sqlx::query_scalar(
        "SELECT verdict IS NOT NULL FROM handover_support_stages WHERE run_id=$1",
    )
    .bind(uuid(&run["id"]))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!(saved);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM claim_revisions WHERE brain_id=$1 AND revision#>>'{content,kind}'='handover'").bind(brain_id(&base)).fetch_one(&h.admin).await.unwrap(),0);
    sqlx::query("UPDATE jobs SET not_before=clock_timestamp()-interval '1 second' WHERE id=$1")
        .bind(uuid(&run["job_id"]))
        .execute(&h.admin)
        .await
        .unwrap();
    let exhausted = exhaust_job_database(&h, "handover.generate").await;
    assert_eq!(exhausted.target_id, uuid(&run["id"]));
    sqlx::query("UPDATE jobs SET updated_at=clock_timestamp()-interval '6 minutes' WHERE id=$1")
        .bind(exhausted.id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        1
    );
    sqlx::query(
        "ALTER TABLE memory_support_assessments DROP CONSTRAINT synthetic_publication_fault",
    )
    .execute(&h.admin)
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    model_job(&h).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        before + 2,
        "Both committed draft and verdict resume locally"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM handover_runs WHERE id=$1")
            .bind(uuid(&run["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "succeeded"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_handover_budget_and_known_assessment_replacement_never_repeat_synthesis() {
    let (h, owner, base, p, server, _child, run) = handover_fixture().await;
    let before = p.calls.load(Ordering::SeqCst);
    p.usage_total.store(99900, Ordering::SeqCst);
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), before + 1);
    let saved:(String,i32,Option<Value>,Uuid,chrono::DateTime<chrono::Utc>)=sqlx::query_as("SELECT r.state,j.attempts,s.verdict,s.assessment_operation_id,s.created_at FROM handover_runs r JOIN jobs j ON j.id=r.job_id JOIN handover_support_stages s ON s.run_id=r.id WHERE r.id=$1")
        .bind(uuid(&run["id"])).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        (&saved.0, saved.1, &saved.2),
        (&"queued".to_string(), 0, &None)
    );
    // Advance the disposable accounting day; normal admission resumes the
    // original assessment key with the committed draft and synthesis receipt.
    sqlx::query(
        "UPDATE model_requests SET created_at=clock_timestamp()-interval '1 day' WHERE brain_id=$1",
    )
    .bind(brain_id(&base))
    .execute(&h.admin)
    .await
    .unwrap();
    sqlx::query("UPDATE jobs SET not_before=clock_timestamp()-interval '1 second' WHERE id=$1")
        .bind(uuid(&run["job_id"]))
        .execute(&h.admin)
        .await
        .unwrap();
    p.usage_total.store(40, Ordering::SeqCst);
    *p.assessments.lock().unwrap() = Some(json!({"assessments":[]}));
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), before + 2);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT error_code FROM handover_runs WHERE id=$1")
            .bind(uuid(&run["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "provider_shape"
    );
    let failed = ok(&h, "GET", &format!("{base}/handovers"), &owner, Value::Null).await;
    let failed = failed["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == run["id"])
        .unwrap();
    assert_eq!(failed["support"]["state"], "failed");
    assert!(failed["support"]["disposition"].is_null());
    assert!(
        failed["support"]["assessment_request_id"].is_string(),
        "A malformed response retains its exact safe receipt"
    );
    sqlx::query(
        "UPDATE handover_runs SET finished_at=clock_timestamp()-interval '6 minutes' WHERE id=$1",
    )
    .bind(uuid(&run["id"]))
    .execute(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        1
    );
    let attempts = ok(&h, "GET", &format!("{base}/handovers"), &owner, Value::Null).await;
    let child = attempts["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["retry_of"] == run["id"])
        .unwrap();
    let inherited:(Uuid,Uuid,chrono::DateTime<chrono::Utc>)=sqlx::query_as("SELECT synthesis_request_id,assessment_operation_id,created_at FROM handover_support_stages WHERE run_id=$1").bind(uuid(&child["id"])).fetch_one(&h.admin).await.unwrap();
    let original: Uuid = sqlx::query_scalar(
        "SELECT synthesis_request_id FROM handover_support_stages WHERE run_id=$1",
    )
    .bind(uuid(&run["id"]))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(inherited.0, original);
    assert_ne!(inherited.1, saved.3);
    assert_eq!(inherited.2, saved.4);
    *p.assessments.lock().unwrap() = Some(verdicts(&["supported"]));
    model_job(&h).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        before + 3,
        "Only the separately accounted failed assessment is replaced"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM handover_runs WHERE id=$1")
            .bind(uuid(&child["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "succeeded"
    );
    server.abort();
    h.finish().await;
}
fn uuid(value: &Value) -> Uuid {
    value.as_str().unwrap().parse().unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_handover_frozen_identity_and_unsupported_new_draft_allocate_nothing() {
    let (h, owner, base, p, server, _child, run) = handover_fixture().await;
    let id = uuid(&run["id"]);
    let original_count: i64 = sqlx::query_scalar("SELECT count(*) FROM claims")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    for assignment in [
        "policy_id=gen_random_uuid()",
        "base_revision_id=gen_random_uuid()",
        "contributions='{}'",
        "operation_id=gen_random_uuid()",
        "actor_id=gen_random_uuid()",
        "device_id=gen_random_uuid()",
        "job_id=gen_random_uuid()",
        "created_at=created_at-interval '1 day'",
        "title='Changed title'",
        "selection='{}'",
        "local_recoveries=2",
        "state='removed',title='',selection='{}',error_code=NULL",
        "state='removed',title='',selection='{}',error_code='unrelated'",
    ] {
        let error = sqlx::query(&format!(
            "UPDATE handover_runs SET {assignment} WHERE id=$1"
        ))
        .bind(id)
        .execute(&h.admin)
        .await
        .expect_err("Frozen run field accepted");
        let message = error.as_database_error().unwrap().message();
        assert!(
            message.starts_with("immutable handover")
                || message == "invalid handover recovery progression",
            "Wrong rejection for {assignment}: {message}"
        );
    }
    assert!(
        sqlx::query("UPDATE handover_run_inputs SET revision_id=gen_random_uuid() WHERE run_id=$1")
            .bind(id)
            .execute(&h.admin)
            .await
            .is_err()
    );
    assert!(sqlx::query("INSERT INTO handover_run_inputs SELECT id,brain_id,gen_random_uuid() FROM handover_runs WHERE id=$1")
        .bind(id).execute(&h.admin).await.is_err());
    sqlx::query("UPDATE handover_runs SET state='failed' WHERE id=$1")
        .bind(id)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE handover_runs SET state='queued',local_recoveries=1 WHERE id=$1")
        .bind(id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE handover_runs SET local_recoveries=0 WHERE id=$1")
            .bind(id)
            .execute(&h.admin)
            .await
            .is_err()
    );
    let before = p.calls.load(Ordering::SeqCst);
    *p.assessments.lock().unwrap() = Some(verdicts(&["insufficient"]));
    model_job(&h).await;
    let page = ok(&h, "GET", &format!("{base}/handovers"), &owner, Value::Null).await;
    let result = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == run["id"])
        .unwrap();
    assert_eq!(result["error_code"], "support_insufficient");
    assert!(result["claim_id"].is_null());
    assert_eq!(result["support"]["disposition"], "insufficient");
    assert_eq!(
        result["support"]["reason"],
        "Controlled synthetic assessment"
    );
    assert_ne!(
        result["support"]["assessment_request_id"],
        result["request_id"]
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM claims")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        original_count
    );
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        0
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), before + 2);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_handover_stage_expiry_preserves_independently_governed_published_memory() {
    let (h, owner, base, p, server, child, run) = handover_fixture().await;
    *p.assessments.lock().unwrap() = Some(verdicts(&["supported"]));
    model_job(&h).await;
    let page = ok(&h, "GET", &format!("{base}/handovers"), &owner, Value::Null).await;
    let published = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == run["id"])
        .unwrap();
    let canonical = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", published["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await["selected"]["revision"]
        .clone();
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["claim_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    sqlx::query("ALTER TABLE handover_support_stages DISABLE TRIGGER immutable_handover_stage")
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE handover_support_stages SET created_at=clock_timestamp()-interval '2 days' WHERE run_id=$1")
        .bind(uuid(&run["id"])).execute(&h.admin).await.unwrap();
    sqlx::query("ALTER TABLE handover_support_stages ENABLE TRIGGER immutable_handover_stage")
        .execute(&h.admin)
        .await
        .unwrap();
    let hidden = ok(&h, "GET", &format!("{base}/handovers"), &owner, Value::Null).await;
    let hidden = hidden["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == run["id"])
        .unwrap();
    assert_eq!(hidden["support"]["state"], "expired");
    assert!(hidden["support"]["reason"].is_null() && hidden["support"]["disposition"].is_null());
    assert!(guard(&h, &base, &canonical).await && guard(&h, &base, &child).await);
    recollect_server::privacy_journal::run_once(&h.state)
        .await
        .unwrap();
    let scrubbed: (Value, Option<Value>, String) = sqlx::query_as(
        "SELECT payload,verdict,privacy_state FROM handover_support_stages WHERE run_id=$1",
    )
    .bind(uuid(&run["id"]))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(scrubbed, (json!({}), None, "expired".into()));
    let reason: (String, Option<String>) = sqlx::query_as(
        "SELECT privacy_state,reason FROM memory_support_assessments WHERE revision_id=$1",
    )
    .bind(uuid(&canonical["id"]))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        reason,
        (
            "active".into(),
            Some("Controlled synthetic assessment".into())
        )
    );
    assert!(guard(&h, &base, &canonical).await);
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_handover_source_erasure_before_stage_cancels_inflight_synthesis() {
    let (h, owner, base, p, server, child, run) = handover_fixture().await;
    let before = p.calls.load(Ordering::SeqCst);
    let source_id: Uuid = sqlx::query_scalar("SELECT source_id FROM source_versions WHERE id=$1")
        .bind(uuid(&child["content"]["supports"][0]["id"]))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    p.delay.store(450, Ordering::SeqCst);
    let state = h.state.clone();
    let work = tokio::spawn(async move { worker::run_once(&state, "model").await });
    wait_calls(&p, before + 1).await;
    let target = json!({"kind":"source","id":source_id});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert!(work.await.unwrap().unwrap());
    let removed: (String, String, Value) =
        sqlx::query_as("SELECT state,title,selection FROM handover_runs WHERE id=$1")
            .bind(uuid(&run["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(removed, ("removed".into(), "".into(), json!({})));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM handover_support_stages")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM claim_revisions WHERE revision#>>'{content,kind}'='handover'"
        )
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT suppressed FROM model_requests WHERE operation_id=$1"
        )
        .bind(uuid(&run["id"]))
        .fetch_one(&h.admin)
        .await
        .unwrap()
    );
    assert!(!worker::run_once(&h.state, "model").await.unwrap());
    assert_eq!(p.calls.load(Ordering::SeqCst), before + 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_handover_local_recovery_is_bounded_without_charged_requests() {
    let (h, _owner, _base, p, server, _child, run) = handover_fixture().await;
    let before = p.calls.load(Ordering::SeqCst);
    let mut job = exhaust_job_database(&h, "handover.generate").await;
    for round in 1..=2 {
        sqlx::query(
            "UPDATE jobs SET updated_at=clock_timestamp()-interval '31 minutes' WHERE id=$1",
        )
        .bind(job.id)
        .execute(&h.admin)
        .await
        .unwrap();
        assert_eq!(
            recollect_server::autonomous::run_once(&h.state)
                .await
                .ok()
                .unwrap(),
            1
        );
        let progress:(i32,i32,i32)=sqlx::query_as("SELECT r.local_recoveries,j.attempts,j.max_attempts FROM handover_runs r JOIN jobs j ON j.id=r.job_id WHERE r.id=$1")
            .bind(uuid(&run["id"])).fetch_one(&h.admin).await.unwrap();
        assert_eq!(progress, (round, round * 3, (round + 1) * 3));
        job = exhaust_job_database(&h, "handover.generate").await;
    }
    sqlx::query("UPDATE jobs SET updated_at=clock_timestamp()-interval '31 minutes' WHERE id=$1")
        .bind(job.id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        0
    );
    let terminal:(i32,i32,i32)=sqlx::query_as("SELECT r.local_recoveries,j.attempts,j.max_attempts FROM handover_runs r JOIN jobs j ON j.id=r.job_id WHERE r.id=$1")
        .bind(uuid(&run["id"])).fetch_one(&h.admin).await.unwrap();
    assert_eq!(terminal, (2, 9, 9));
    assert_eq!(p.calls.load(Ordering::SeqCst), before);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_handover_source_erasure_replays_staged_and_published_reasons_into_older_backup() {
    let (mut h, owner, base, p, server, child, run) = handover_fixture().await;
    model_job(&h).await;
    let published =
        ok(&h, "GET", &format!("{base}/handovers"), &owner, Value::Null).await["items"][0].clone();
    let canonical = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", published["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await["selected"]["revision"]
        .clone();
    let source_id: Uuid = sqlx::query_scalar("SELECT source_id FROM source_versions WHERE id=$1")
        .bind(uuid(&child["content"]["supports"][0]["id"]))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let independent = source(&h, &owner, &base, "Birch declares port 3030.\n").await;
    let control = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&independent["version"]["id"], "Birch", "3030"),
    )
    .await;
    let control = reviewed(&h, &owner, &base, &control).await;
    let staged=ok(&h,"POST",&format!("{base}/handovers"),&owner,json!({"title":"Another synthetic continuation","contributions":[child["id"]],"operation_id":null})).await;
    sqlx::query("ALTER TABLE memory_support_assessments ADD CONSTRAINT synthetic_staged_fault CHECK(source_handover_stage_id IS NULL) NOT VALID").execute(&h.admin).await.unwrap();
    model_job(&h).await;
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT verdict IS NOT NULL FROM handover_support_stages WHERE run_id=$1"
        )
        .bind(uuid(&staged["id"]))
        .fetch_one(&h.admin)
        .await
        .unwrap()
    );
    sqlx::query("ALTER TABLE memory_support_assessments DROP CONSTRAINT synthetic_staged_fault")
        .execute(&h.admin)
        .await
        .unwrap();
    let before = p.calls.load(Ordering::SeqCst);
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable handover support restore fixture: {backup}");
    h.state.pool.close().await;
    h.admin.close().await;
    sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'")).execute(&h.root).await.unwrap();
    let mut admin_url = reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
    admin_url.set_path(&h.database);
    h.admin = db::pool(admin_url.as_str()).await.unwrap();
    h.state = AppState::new(
        db::pool(&h.state.config.database_url).await.unwrap(),
        (*h.state.config).clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    admin_url.set_path(&backup);
    let backup_admin = db::pool(admin_url.as_str()).await.unwrap();
    let mut config = (*h.state.config).clone();
    let mut url = reqwest::Url::parse(&config.database_url).unwrap();
    url.set_path(&backup);
    config.database_url = url.to_string();
    let backup_state = AppState::new(
        db::pool(&config.database_url).await.unwrap(),
        config.clone(),
    )
    .unwrap();
    let target = json!({"kind":"source","id":source_id});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    recollect_server::privacy_journal::run_once(&h.state)
        .await
        .unwrap();
    assert!(
        recollect_server::privacy_journal::barrier(&backup_state.pool, &config)
            .await
            .is_err()
    );
    recollect_server::privacy_journal::reconcile(&backup_admin, &config)
        .await
        .unwrap();
    recollect_server::privacy_journal::barrier(&backup_state.pool, &config)
        .await
        .unwrap();
    for admin in [&h.admin, &backup_admin] {
        let stages:Vec<(Value,Option<Value>,String)>=sqlx::query_as("SELECT payload,verdict,privacy_state FROM handover_support_stages WHERE run_id=ANY($1)")
            .bind(vec![uuid(&run["id"]),uuid(&staged["id"])]).fetch_all(admin).await.unwrap();
        assert_eq!(stages.len(), 2);
        assert!(
            stages
                .iter()
                .all(|s| s.0 == json!({}) && s.1.is_none() && s.2 == "erased")
        );
        let verdict:(String,Option<String>,Option<String>)=sqlx::query_as("SELECT privacy_state,reason,disposition FROM memory_support_assessments WHERE revision_id=$1")
            .bind(uuid(&canonical["id"])).fetch_one(admin).await.unwrap();
        assert_eq!(verdict, ("erased".into(), None, None));
        assert!(sqlx::query_scalar::<_,bool>("SELECT bool_and(suppressed) FROM model_requests WHERE operation_id=ANY($1) OR operation_id IN(SELECT assessment_operation_id FROM handover_support_stages WHERE run_id=ANY($1))")
            .bind(vec![uuid(&run["id"]),uuid(&staged["id"])]).fetch_one(admin).await.unwrap());
        let pending: (String, String, Value) =
            sqlx::query_as("SELECT state,title,selection FROM handover_runs WHERE id=$1")
                .bind(uuid(&staged["id"]))
                .fetch_one(admin)
                .await
                .unwrap();
        assert_eq!(pending, ("removed".into(), "".into(), json!({})));
        assert!(
            sqlx::query_scalar::<_, bool>("SELECT recollect_memory_supported($1,$2)")
                .bind(brain_id(&base))
                .bind(uuid(&control["id"]))
                .fetch_one(admin)
                .await
                .unwrap()
        );
    }
    assert!(!worker::run_once(&h.state, "model").await.unwrap());
    assert_eq!(p.calls.load(Ordering::SeqCst), before);
    backup_state.pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_direct_revisions_are_automatically_audited_before_recall_and_preserve_human_authority()
 {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let evidence = source(&h, &owner, &base, "Amber.configuration = 8080\n").await;
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .ok()
            .unwrap(),
        1
    );
    model_job(&h).await;
    let good = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&evidence["version"]["id"], "Amber", "8080"),
    )
    .await;
    let bad = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&evidence["version"]["id"], "Amber", "9090"),
    )
    .await;
    let claim_path = format!("{base}/claims/{}", good["claim_id"].as_str().unwrap());
    let before = ok(&h, "GET", &claim_path, &owner, Value::Null).await;
    assert_eq!(before["selected"]["eligibility"]["investigation"], false);
    let hidden = ok(
        &h,
        "POST",
        &format!("{base}/recall"),
        &owner,
        json!({"query":"Amber"}),
    )
    .await;
    assert!(
        !hidden["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"] == "claim")
    );
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("Autonomous coordinator failed: {}", e.1)),
        2
    );
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("Autonomous coordinator failed: {}", e.1)),
        0
    );
    for _ in 0..2 {
        let job = worker::claim(&h.state.pool, "model")
            .await
            .unwrap()
            .unwrap();
        let revision: Uuid =
            sqlx::query_scalar("SELECT revision_id FROM memory_support_assessments WHERE id=$1")
                .bind(job.target_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        *p.assessments.lock().unwrap() = Some(verdicts(&[if revision == uuid(&good["id"]) {
            "supported"
        } else {
            "contradicted"
        }]));
        worker::execute(&h.state, &job).await.unwrap();
    }
    let after = ok(&h, "GET", &claim_path, &owner, Value::Null).await;
    assert_eq!(
        after["selected"]["revision"]["id"], good["id"],
        "Audit must not rewrite the author-owned revision"
    );
    assert_eq!(after["selected"]["eligibility"]["investigation"], true);
    let recalled = ok(
        &h,
        "POST",
        &format!("{base}/recall"),
        &owner,
        json!({"query":"Amber"}),
    )
    .await;
    assert!(
        recalled["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == good["claim_id"])
    );
    assert!(
        !recalled["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == bad["claim_id"])
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    original_support_predicates_match(&h, brain_id(&base)).await;
    // A policy generation invalidates automatic support and cached eligibility.
    let epoch: i64 = sqlx::query_scalar("SELECT epoch FROM memory_epochs WHERE brain_id=$1")
        .bind(brain_id(&base))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    permit(&h, &owner, &base, 110000).await;
    let updated: i64 = sqlx::query_scalar("SELECT epoch FROM memory_epochs WHERE brain_id=$1")
        .bind(brain_id(&base))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(updated > epoch);
    let unchecked = ok(&h, "GET", &claim_path, &owner, Value::Null).await;
    assert_eq!(unchecked["selected"]["eligibility"]["investigation"], false);
    let reviewed=ok(&h,"POST",&format!("{claim_path}/review"),&owner,json!({"base_revision":good["id"],"action":"accept","reason":"Explicit synthetic human review","content":null,"revalidation_basis":null})).await;
    assert_eq!(reviewed["claims"][0]["eligibility"]["investigation"], true);
    original_support_predicates_match(&h, brain_id(&base)).await;
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_near_limit_source_uses_complete_cited_windows_with_original_coordinates() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["max_input_bytes"] = json!(4096);
    })
    .await;
    let mut lines = vec!["Unrelated filler."; 190];
    lines[99] = "Amber.port = 8080";
    lines[100] = "Declaration only; no runtime check.";
    let text = lines.join("\n");
    assert!(text.len() > 3300 && text.len() < 3500);
    let input = source(&h, &owner, &base, &text).await;
    *p.candidates.lock().unwrap() =
        json!({"claims":[candidate("Amber","port","8080",100)],"retirements":[]});
    *p.assessments.lock().unwrap() = Some(verdicts(&["supported"]));
    learn(&h, &owner, &base, &input).await;
    model_job(&h).await;
    let result = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(result["state"], "succeeded", "{result}");
    assert_eq!(result["accepted"], 1);
    let bodies = p.bodies.lock().unwrap().clone();
    assert_eq!(bodies.len(), 2);
    assert!(
        bodies[0]["input"]
            .as_str()
            .unwrap()
            .contains("Unrelated filler.")
    );
    let evidence = bodies[1]["input"]
        .as_str()
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .find(|v| v["provenance"]["kind"] == "source_version")
        .unwrap();
    assert_eq!(evidence["provenance"]["version_id"], input["version"]["id"]);
    assert_eq!(evidence["provenance"]["role"], "source_document");
    let windows: Value = serde_json::from_str(evidence["data"].as_str().unwrap()).unwrap();
    assert_eq!(windows["total_lines"], 190);
    assert_eq!(windows["omitted_lines"], 169);
    assert_eq!(windows["windows"][1]["line_from"], 98);
    assert_eq!(windows["windows"][1]["line_to"], 102);
    assert!(
        windows["windows"][1]["text"]
            .as_str()
            .unwrap()
            .contains("no runtime check")
    );
    assert!(bodies[1]["input"].as_str().unwrap().len() < 2000);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_mixed_batch_and_unsupported_correction_preserve_prior_identity() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let original = source(
        &h,
        &owner,
        &base,
        "Amber.port = 8080\nNo production deployment was observed.\n",
    )
    .await;
    *p.candidates.lock().unwrap() = json!({"claims":[candidate("Amber","port","8080",1),candidate("Amber","production deployment","completed successfully",2)],"retirements":[]});
    *p.assessments.lock().unwrap() = Some(verdicts(&["supported", "contradicted"]));
    let run = learn(&h, &owner, &base, &original).await;
    model_job(&h).await;
    let result = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(result["state"], "succeeded", "{result}");
    assert_eq!(result["accepted"], 1);
    assert_eq!(result["blocked"], 1);
    assert_eq!(result["claim_ids"].as_array().unwrap().len(), 1);
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    let claim = result["claim_ids"][0].clone();
    let first = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", claim.as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await["selected"]["revision"]
        .clone();
    let staged: Value =
        sqlx::query_scalar("SELECT verdicts FROM learning_support_stages WHERE run_id=$1")
            .bind(uuid(&run["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(staged[1]["disposition"], "contradicted");
    let newer=ok(&h,"POST",&format!("{base}/sources/{}/versions",original["id"].as_str().unwrap()),&owner,json!({"base_version":original["version"]["id"],"title":"Unproven change","media_type":"text/plain","retain_content":true,"content":"An assistant proposes changing Amber to port 9090. No change was observed.\n"})).await;
    let mut correction = candidate("Amber", "port", "9090", 1);
    correction["replaces_revision"] = first["id"].clone();
    *p.candidates.lock().unwrap() = json!({"claims":[correction],"retirements":[]});
    *p.assessments.lock().unwrap() = Some(verdicts(&["insufficient"]));
    learn(&h, &owner, &base, &newer).await;
    model_job(&h).await;
    let unchanged = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", claim.as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        unchanged["selected"]["revision"]["id"], first["id"],
        "Unsupported correction must not append or retire old memory"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM claims WHERE brain_id=$1")
        .bind(brain_id(&base))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(count, 1);
    // Erasing the new source removes the derived stage, while an older target
    // can still expire later. Erased stages must never downgrade or block expiry.
    let target = json!({"kind":"source","id":original["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let state: String =
        sqlx::query_scalar("SELECT privacy_state FROM learning_support_stages WHERE run_id=$1")
            .bind(uuid(&run["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(state, "erased");
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_whole_procedure_and_retirement_dispositions_are_independent() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let original=source(&h,&owner,&base,"When the local config exists, read it and record the declared port. Outcome: a recorded declaration. No runtime test is described.\n").await;
    let good = json!({"kind":"procedure","subject":"Inspect local port","predicate":"procedure","value":"Record the declared port","rationale":"The source states a local inspection sequence","line_from":1,"line_to":1,"replaces_revision":null,"procedure":{"conditions":"Local config exists","steps":["Read the config","Record the declared port"],"expected_outcome":"A recorded declaration"}});
    let mut false_outcome = good.clone();
    false_outcome["subject"] = json!("Verify production port");
    false_outcome["procedure"]["expected_outcome"] = json!("Production connectivity verified");
    *p.candidates.lock().unwrap() = json!({"claims":[good,false_outcome],"retirements":[]});
    *p.assessments.lock().unwrap() = Some(verdicts(&["supported", "insufficient"]));
    learn(&h, &owner, &base, &original).await;
    model_job(&h).await;
    let result = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(result["accepted"], 1);
    assert_eq!(result["blocked"], 1);
    let claim = result["claim_ids"][0].clone();
    let first = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", claim.as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await["selected"]["revision"]
        .clone();
    let changed=ok(&h,"POST",&format!("{base}/sources/{}/versions",original["id"].as_str().unwrap()),&owner,json!({"base_version":original["version"]["id"],"title":"Only a later mention","media_type":"text/plain","retain_content":true,"content":"The old port inspection is mentioned here. Nothing retires that procedure.\n"})).await;
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[{"revision_id":first["id"],"reason":"The newer source does not list the old steps","line_from":1,"line_to":1}]});
    *p.assessments.lock().unwrap() = Some(verdicts(&["insufficient"]));
    let run = learn(&h, &owner, &base, &changed).await;
    model_job(&h).await;
    let unchanged = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", claim.as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(unchanged["selected"]["revision"]["id"], first["id"]);
    let current = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(current["retired"], 0);
    assert_eq!(current["blocked"], 1);
    // Remove just this source/version, leaving the older target active; then
    // expire that different dependency to prove monotonic stage privacy.
    let brain = brain_id(&base);
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::query("UPDATE learning_support_stages SET payload='{}',verdicts=NULL,privacy_state='erased' WHERE run_id=$1").bind(uuid(&run["id"])).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE claim_revisions SET privacy_state='expired' WHERE brain_id=$1 AND id=$2")
        .bind(brain)
        .bind(uuid(&first["id"]))
        .execute(&mut *tx)
        .await
        .unwrap();
    let monotonic: String =
        sqlx::query_scalar("SELECT privacy_state FROM learning_support_stages WHERE run_id=$1")
            .bind(uuid(&run["id"]))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(monotonic, "erased");
    tx.rollback().await.unwrap();
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_lost_extraction_response_is_not_replayed() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let original = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let run = learn(&h, &owner, &base, &original).await;
    let id = uuid(&run["id"]);
    sqlx::query("ALTER TABLE learning_support_stages ADD CONSTRAINT lost_stage_probe CHECK(false) NOT VALID").execute(&h.admin).await.unwrap();
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    sqlx::query("ALTER TABLE learning_support_stages DROP CONSTRAINT lost_stage_probe")
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE jobs SET state='queued',attempts=0,not_before=clock_timestamp() WHERE target_id=$1",
    )
    .bind(id)
    .execute(&h.admin)
    .await
    .unwrap();
    sqlx::query("UPDATE learning_runs SET state='queued' WHERE id=$1")
        .bind(id)
        .execute(&h.admin)
        .await
        .unwrap();
    model_job(&h).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Recorded charged request without a saved typed response must not be replayed"
    );
    let result = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(result["error_code"], "model_attempt_recorded");
    assert_eq!(result["claim_ids"], json!([]));
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_stages_are_immutable_and_expire_as_derived_memory() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let original = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let run = learn(&h, &owner, &base, &original).await;
    model_job(&h).await;
    let id = uuid(&run["id"]);
    assert!(
        sqlx::query("UPDATE learning_support_stages SET payload='{}' WHERE run_id=$1")
            .bind(id)
            .execute(&h.admin)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE learning_support_stages SET verdicts='[]' WHERE run_id=$1")
            .bind(id)
            .execute(&h.admin)
            .await
            .is_err()
    );
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["claim_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    // The disposable fixture models an aged stage. Its current document and
    // newly published claim remain independent, within their own lifetimes.
    sqlx::query("ALTER TABLE learning_support_stages DISABLE TRIGGER immutable_support_stage")
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE learning_support_stages SET created_at=clock_timestamp()-interval '2 days' WHERE run_id=$1").bind(id).execute(&h.admin).await.unwrap();
    sqlx::query("ALTER TABLE learning_support_stages ENABLE TRIGGER immutable_support_stage")
        .execute(&h.admin)
        .await
        .unwrap();
    recollect_server::privacy_journal::run_once(&h.state)
        .await
        .unwrap();
    let erased: (Value, Option<Value>, String) = sqlx::query_as(
        "SELECT payload,verdicts,privacy_state FROM learning_support_stages WHERE run_id=$1",
    )
    .bind(id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(erased, (json!({}), None, "expired".into()));
    let source_state: String =
        sqlx::query_scalar("SELECT privacy_state FROM source_versions WHERE id=$1")
            .bind(uuid(&original["version"]["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(source_state, "active");
    let claims: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM claim_revisions WHERE brain_id=$1 AND privacy_state='active'",
    )
    .bind(brain_id(&base))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(claims, 1);
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_stage_expires_with_offered_targets_original_evidence() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
        policy["content_classes"] = json!(["document", "raw_session", "claim", "query"]);
    })
    .await;
    let original = ok(
        &h,
        "POST",
        &format!("{base}/sources"),
        &owner,
        json!({"title":"Synthetic session evidence","media_type":"text/plain","content":"Amber.port = 8080\n","retain_content":true,"retention_class":"raw_session"}),
    )
    .await;
    learn(&h, &owner, &base, &original).await;
    model_job(&h).await;
    let first = runs(&h, &owner, &base).await["items"][0].clone();
    let claim = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", first["claim_ids"][0].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    let target = claim["selected"]["revision"]["id"].clone();
    let newer = ok(
        &h,
        "POST",
        &format!("{base}/sources/{}/versions", original["id"].as_str().unwrap()),
        &owner,
        json!({"base_version":original["version"]["id"],"title":"Later session proposal","media_type":"text/plain","retain_content":true,"content":"An assistant proposes port 9090. No change was observed.\n"}),
    )
    .await;
    let mut correction = candidate("Amber", "port", "9090", 1);
    correction["replaces_revision"] = target.clone();
    *p.candidates.lock().unwrap() = json!({"claims":[correction],"retirements":[]});
    *p.assessments.lock().unwrap() = Some(verdicts(&["insufficient"]));
    let run = learn(&h, &owner, &base, &newer).await;
    model_job(&h).await;
    let id = uuid(&run["id"]);
    let offered: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM learning_run_inputs WHERE run_id=$1 AND revision_id=$2)",
    )
    .bind(id)
    .bind(uuid(&target))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!(offered, "The stage must depend on the exact prior revision");
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["raw_session_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    // Only the older supporting event ages out. The offered revision and the
    // later source/stage are still within their own retention periods.
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '2 days' WHERE id=$1",
    )
    .bind(uuid(&original["version"]["id"]))
    .execute(&h.admin)
    .await
    .unwrap();
    let due: bool =
        sqlx::query_scalar("SELECT recollect_learning_support_deadline($1,$2)<=clock_timestamp()")
            .bind(brain_id(&base))
            .bind(id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(
        due,
        "The offered target's evidence supplies the earlier deadline"
    );
    recollect_server::privacy_journal::run_once(&h.state)
        .await
        .unwrap();
    let removed: (Value, Option<Value>, String) = sqlx::query_as(
        "SELECT payload,verdicts,privacy_state FROM learning_support_stages WHERE run_id=$1",
    )
    .bind(id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(removed, (json!({}), None, "expired".into()));
    let current_source: String =
        sqlx::query_scalar("SELECT privacy_state FROM source_versions WHERE id=$1")
            .bind(uuid(&newer["version"]["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(current_source, "active");
    let target_state: String =
        sqlx::query_scalar("SELECT privacy_state FROM claim_revisions WHERE id=$1")
            .bind(uuid(&target))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        target_state, "active",
        "Ordinary expiry retains the historical claim identity"
    );
    let journaled: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM privacy_requests WHERE brain_id=$1 AND journaled AND manifest->'learning_support_stages' @> $2::jsonb)",
    )
    .bind(brain_id(&base))
    .bind(json!([id]))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!(
        journaled,
        "Restore replay must carry the transitive stage identity"
    );
    let suppressed: bool = sqlx::query_scalar(
        "SELECT m.suppressed FROM learning_support_stages s JOIN model_requests m ON m.id=s.assessment_request_id WHERE s.run_id=$1",
    )
    .bind(id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!(
        suppressed,
        "Removed target evidence suppresses its assessment receipt"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_stage_budget_resume_reuses_extraction_and_committed_verdict() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 20000).await;
    p.usage_total.store(19900, Ordering::SeqCst);
    let original = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let run = learn(&h, &owner, &base, &original).await;
    model_job(&h).await;
    let run_id = uuid(&run["id"]);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Extraction committed before assessment budget denial"
    );
    let saved:(String,i32,Option<Value>)=sqlx::query_as("SELECT r.state,j.attempts,s.verdicts FROM learning_runs r JOIN jobs j ON j.id=r.job_id JOIN learning_support_stages s ON s.run_id=r.id WHERE r.id=$1").bind(run_id).fetch_one(&h.admin).await.unwrap();
    assert_eq!(saved, ("queued".into(), 0, None));
    // Simulate the accounting day changing and the deferred job becoming due.
    sqlx::query(
        "UPDATE model_requests SET created_at=clock_timestamp()-interval '1 day' WHERE brain_id=$1",
    )
    .bind(brain_id(&base))
    .execute(&h.admin)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE jobs SET not_before=clock_timestamp()-interval '1 second' WHERE target_id=$1",
    )
    .bind(run_id)
    .execute(&h.admin)
    .await
    .unwrap();
    p.usage_total.store(40, Ordering::SeqCst);
    // Inject a database publication fault after the verdict commit. Native
    // worker recovery must resume it, without a manual work-state reset.
    sqlx::query(
        "ALTER TABLE claims ADD CONSTRAINT support_publication_probe CHECK(false) NOT VALID",
    )
    .execute(&h.admin)
    .await
    .unwrap();
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    let assessed: bool = sqlx::query_scalar(
        "SELECT verdicts IS NOT NULL FROM learning_support_stages WHERE run_id=$1",
    )
    .bind(run_id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!(assessed);
    let recovery: (String, i32, Option<String>) =
        sqlx::query_as("SELECT state,attempts,error_code FROM jobs WHERE target_id=$1")
            .bind(run_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        recovery,
        ("queued".into(), 1, Some("database_error".into()))
    );
    sqlx::query("ALTER TABLE claims DROP CONSTRAINT support_publication_probe")
        .execute(&h.admin)
        .await
        .unwrap();
    // Wait for the actual bounded worker backoff; no queued/running state or
    // attempt counter is changed by the test to make publication resume.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    model_job(&h).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        2,
        "Committed verdict resumes publication without replay charges"
    );
    let result = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(result["state"], "succeeded", "{result}");
    assert_eq!(result["accepted"], 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_automatic_assessment_replacements_preserve_extraction_and_bound_calls() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
        policy["content_classes"] = json!(["document", "claim", "query", "support_excerpt"]);
        policy["daily_token_limit"] = json!(100000);
    })
    .await;
    let original = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state).await.ok(),
        Some(1)
    );
    *p.assessments.lock().unwrap() = Some(verdicts(&[]));
    model_job(&h).await;
    let failed = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(failed["state"], "failed");
    assert_eq!(failed["error_code"], "provider_shape");
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    let original_stage: (Uuid, Value, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        "SELECT extraction_request_id,payload,created_at FROM learning_support_stages WHERE run_id=$1",
    )
    .bind(uuid(&failed["id"]))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    // Advance only the retry time. The native coordinator creates the linked
    // attempt; no job/run state, target list or payload is repaired by the test.
    sqlx::query(
        "UPDATE learning_runs SET finished_at=clock_timestamp()-interval '1 hour' WHERE id=$1",
    )
    .bind(uuid(&failed["id"]))
    .execute(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state).await.ok(),
        Some(1)
    );
    let replacement = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(replacement["retry_of"], failed["id"]);
    assert_eq!(
        replacement["reconciliation_inputs"],
        failed["reconciliation_inputs"]
    );
    let inherited: (Uuid, Value, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        "SELECT extraction_request_id,payload,created_at FROM learning_support_stages WHERE run_id=$1",
    )
    .bind(uuid(&replacement["id"]))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(inherited, original_stage);
    // Any accidental new extraction would now fabricate a different assertion.
    *p.candidates.lock().unwrap() =
        json!({"claims":[candidate("Quartz","port","5050",1)],"retirements":[]});
    *p.assessments.lock().unwrap() = Some(verdicts(&["supported"]));
    model_job(&h).await;
    let result = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(result["state"], "succeeded", "{result}");
    assert_eq!(result["accepted"], 1);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        3,
        "Only assessment is replaced"
    );
    let claim = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", result["claim_ids"][0].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(claim["selected"]["revision"]["content"]["subject"], "Amber");
    assert_eq!(original["version"]["id"], result["source_version_id"]);
    let retained: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM automatic_support_excerpts WHERE parent_version_id=$1 AND privacy_state='active'",
    ).bind(uuid(&original["version"]["id"])).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        retained, 1,
        "A replacement's inherited verdict can retain its permitted exact excerpt"
    );

    // A separate generation proves the same saved extraction cannot cause an
    // unbounded assessor loop. Initial call plus two charged replacements only.
    source(&h, &owner, &base, "Quartz.port = 5050\n").await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state).await.ok(),
        Some(1)
    );
    *p.assessments.lock().unwrap() = Some(verdicts(&[]));
    for attempt in 0..=2 {
        model_job(&h).await;
        let failed = runs(&h, &owner, &base).await["items"][0].clone();
        assert_eq!(failed["state"], "failed");
        assert_eq!(failed["automatic_attempt"], attempt);
        sqlx::query(
            "UPDATE learning_runs SET finished_at=clock_timestamp()-interval '1 hour' WHERE id=$1",
        )
        .bind(uuid(&failed["id"]))
        .execute(&h.admin)
        .await
        .unwrap();
        assert_eq!(
            recollect_server::autonomous::run_once(&h.state).await.ok(),
            Some(usize::from(attempt < 2))
        );
    }
    assert_eq!(p.calls.load(Ordering::SeqCst), 7);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_empty_and_incomplete_verdicts_cannot_publish_and_stages_erase() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let empty = source(&h, &owner, &base, "No supported facts.\n").await;
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[]});
    learn(&h, &owner, &base, &empty).await;
    model_job(&h).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Empty stage needs no assessment call"
    );
    let original = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    *p.candidates.lock().unwrap() =
        json!({"claims":[candidate("Amber","port","8080",1)],"retirements":[]});
    *p.assessments.lock().unwrap() = Some(verdicts(&[]));
    let run = learn(&h, &owner, &base, &original).await;
    model_job(&h).await;
    let result = runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(result["state"], "failed");
    assert_eq!(result["error_code"], "provider_shape");
    assert_eq!(result["claim_ids"], json!([]));
    let target = json!({"kind":"source","id":original["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let removed: (Value, Option<Value>, String) = sqlx::query_as(
        "SELECT payload,verdicts,privacy_state FROM learning_support_stages WHERE run_id=$1",
    )
    .bind(uuid(&run["id"]))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(removed, (json!({}), None, "erased".into()));
    recollect_server::privacy_journal::run_once(&h.state)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_cross_session_exact_family_correction_reuse_and_unsupported_control() {
    let (h, owner, base, p, server) = setup().await;
    permit(&h, &owner, &base, 100000).await;
    let initial = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    *p.candidates.lock().unwrap() = family_candidate("8080", Value::Null);
    let run = learn(&h, &owner, &base, &initial).await;
    model_job(&h).await;
    let current: Value = sqlx::query_scalar("SELECT revision FROM claim_revisions WHERE claim_id=(SELECT unnest(claim_ids) FROM learning_runs WHERE id=$1)")
        .bind(uuid(&run["id"])).fetch_one(&h.admin).await.unwrap();
    let correction = source(
        &h,
        &owner,
        &base,
        "Amber.port changes from 8080 to 9090. This is the current declared configuration.\n",
    )
    .await;
    *p.candidates.lock().unwrap() = family_candidate("9090", Value::Null);
    let changed = learn(&h, &owner, &base, &correction).await;
    model_job(&h).await;
    let changed_run = ok(&h, "GET", &format!("{base}/learning"), &owner, Value::Null).await;
    let changed_run = changed_run["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == changed["id"])
        .unwrap();
    assert_eq!(changed_run["state"], "succeeded", "{changed_run}");
    assert_eq!(changed_run["revised"], 1);
    assert_eq!(changed_run["claim_ids"][0], current["claim_id"]);
    let stage: Value =
        sqlx::query_scalar("SELECT payload FROM learning_support_stages WHERE run_id=$1")
            .bind(uuid(&changed["id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(stage["claims"][0]["replaces_revision"], current["id"]);
    assert_eq!(
        stage["discovery"][0]["disposition"],
        "replacement_hypothesis"
    );
    let head: Uuid = sqlx::query_scalar("SELECT current_revision FROM claims WHERE id=$1")
        .bind(uuid(&current["claim_id"]))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let repeat = source(&h, &owner, &base, "Amber.port = 9090\n").await;
    let repeated = learn(&h, &owner, &base, &repeat).await;
    model_job(&h).await;
    let reused: i32 = sqlx::query_scalar("SELECT reused FROM learning_runs WHERE id=$1")
        .bind(uuid(&repeated["id"]))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(reused, 1);
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>("SELECT current_revision FROM claims WHERE id=$1")
            .bind(uuid(&current["claim_id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        head
    );
    let unsupported = source(
        &h,
        &owner,
        &base,
        "We might change Amber.port to 7070 later; no change has happened.\n",
    )
    .await;
    *p.candidates.lock().unwrap() = family_candidate("7070", Value::Null);
    *p.assessments.lock().unwrap() = Some(verdicts(&["insufficient"]));
    learn(&h, &owner, &base, &unsupported).await;
    model_job(&h).await;
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>("SELECT current_revision FROM claims WHERE id=$1")
            .bind(uuid(&current["claim_id"]))
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        head
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM claims")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        8,
        "Two independent calls per changed evidence; discovery itself is native"
    );
    server.abort();
    h.finish().await;
}

fn family_candidate(value: &str, replaces: Value) -> Value {
    let mut c = candidate("Amber", "port", value, 1);
    c["replaces_revision"] = replaces;
    json!({"claims":[c],"retirements":[]})
}
