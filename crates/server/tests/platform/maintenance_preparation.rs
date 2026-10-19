//! Actual PostgreSQL preparation, publication, fairness and cancellation proof.
//! Providers below are loopback controlled fixtures; scheduling makes no calls.
use super::*;
use recollect_server::autonomous::maintenance;
use std::time::Duration as WallDuration;

struct Fixture {
    h: Harness,
    owner: Login,
    base: String,
    provider: Arc<Provider>,
    server: tokio::task::JoinHandle<()>,
    brain: Uuid,
    actor: Uuid,
    revision: Value,
}

async fn fixture() -> Fixture {
    let (h, owner, base, provider, server) = setup().await;
    capture_policy(&h, &owner, &base, true).await;
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
        policy["content_classes"] = json!(["raw_session", "claim", "query", "support_excerpt"]);
    })
    .await;
    let (_, token) = h.pair_device(&owner, "Maintenance preparation proof").await;
    let bound = binding(&h, &base, &token, json!({})).await;
    let mut claim = candidate("Amber", "port", "8080", 1);
    claim["context_role"] = json!("convention");
    claim["replaces_revision"] = Value::Null;
    *provider.candidates.lock().unwrap() = json!({"claims":[claim],"retirements":[]});
    let receipt = device_ok(
        &h,
        "POST",
        &format!("{base}/capture/events"),
        &token,
        event(&bound, "maintenance", "Amber.port = 8080\n"),
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
    model_job(&h).await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let sqlx::types::Json(revision): sqlx::types::Json<Value> = sqlx::query_scalar(
        "SELECT r.revision FROM learning_runs l JOIN claims c ON c.id=ANY(l.claim_ids) AND c.brain_id=l.brain_id JOIN claim_revisions r ON r.id=c.current_revision WHERE l.brain_id=$1 LIMIT 1")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    sqlx::query(
        "UPDATE capture_events SET received_at=clock_timestamp()-interval '6 minutes' WHERE id=$1",
    )
    .bind(
        receipt["event_id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .execute(&h.admin)
    .await
    .unwrap();
    Fixture {
        h,
        owner,
        base,
        provider,
        server,
        brain,
        actor,
        revision,
    }
}

async fn coverage(f: &Fixture) -> Value {
    sqlx::query_scalar("SELECT coverage FROM session_digest_partitions WHERE brain_id=$1")
        .bind(f.brain)
        .fetch_one(&f.h.admin)
        .await
        .unwrap()
}

async fn install_gate(h: &Harness) {
    sqlx::raw_sql("ALTER FUNCTION recollect_memory_supported(uuid,uuid) RENAME TO fixture_maintenance_supported;
      CREATE FUNCTION recollect_memory_supported(b uuid,target uuid) RETURNS boolean
      LANGUAGE plpgsql STABLE SET search_path=public,pg_temp AS $$ BEGIN
       PERFORM pg_advisory_xact_lock(67126124,8);
       RETURN fixture_maintenance_supported(b,target);
      END $$;
      ALTER FUNCTION recollect_memory_supported_many(uuid,uuid[]) RENAME TO fixture_maintenance_supported_many;
      CREATE FUNCTION recollect_memory_supported_many(b uuid,targets uuid[]) RETURNS TABLE(revision_id uuid,supported boolean)
      LANGUAGE plpgsql STABLE SET search_path=public,pg_temp AS $$ BEGIN
       PERFORM pg_advisory_xact_lock(67126124,8);
       RETURN QUERY SELECT f.revision_id,f.supported FROM fixture_maintenance_supported_many(b,targets) f;
      END $$;")
        .execute(&h.admin).await.unwrap();
}

async fn wait_for_gate(h: &Harness) {
    for _ in 0..150 {
        let entered: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND classid=67126124 AND objid=8 AND NOT granted)")
            .fetch_one(&h.admin).await.unwrap();
        if entered {
            return;
        }
        tokio::time::sleep(WallDuration::from_millis(5)).await;
    }
    panic!("Actual complete maintenance preparation did not reach canonical qualification");
}

async fn same_brain_writer(f: &Fixture) {
    tokio::time::timeout(WallDuration::from_millis(500), async {
        let mut tx = db::actor_tx(&f.h.state.pool, f.actor)
            .await
            .map_err(|e| e.1)
            .unwrap();
        db::require_writer(&mut tx, f.brain)
            .await
            .map_err(|e| e.1)
            .unwrap();
        sqlx::query("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1")
            .bind(f.brain)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    })
    .await
    .expect("Same-Brain writer must commit during actual maintenance preparation");
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn maintenance_preparation_digest_cas_policy_access_and_duplicate_gates() {
    let f = fixture().await;
    let calls = f.provider.calls.load(Ordering::SeqCst);
    let first = maintenance::prepare_digest(&f.h.state, f.brain, f.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    let competing = maintenance::prepare_digest(&f.h.state, f.brain, f.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    assert_eq!(
        maintenance::publish_digest(&f.h.state, first, 2).await.ok(),
        Some(1)
    );
    assert_eq!(
        maintenance::publish_digest(&f.h.state, competing, 2)
            .await
            .ok(),
        Some(0)
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM session_digest_generations WHERE brain_id=$1")
            .bind(f.brain)
            .fetch_one(&f.h.admin)
            .await
            .unwrap();
    assert_eq!(
        count, 1,
        "Competing/replayed preparation cannot duplicate a generation"
    );
    let before = coverage(&f).await;
    let stale = maintenance::prepare_digest(&f.h.state, f.brain, f.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    same_brain_writer(&f).await;
    assert_eq!(
        maintenance::publish_digest(&f.h.state, stale, 2).await.ok(),
        Some(0)
    );
    assert_eq!(coverage(&f).await, before);
    let policy_stale = maintenance::prepare_digest(&f.h.state, f.brain, f.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    allow(&f.h, &f.owner, &f.base, |p| {
        p["autonomous_memory"] = json!(true);
    })
    .await;
    assert_eq!(
        maintenance::publish_digest(&f.h.state, policy_stale, 2)
            .await
            .ok(),
        Some(0)
    );
    let no_access = maintenance::prepare_digest(&f.h.state, f.brain, f.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE accounts SET enabled=false WHERE id=$1")
        .bind(f.actor)
        .execute(&f.h.admin)
        .await
        .unwrap();
    assert!(
        maintenance::publish_digest(&f.h.state, no_access, 2)
            .await
            .is_err()
    );
    assert_eq!(coverage(&f).await, before);
    assert_eq!(
        f.provider.calls.load(Ordering::SeqCst),
        calls,
        "Scheduling must make zero provider requests"
    );
    f.server.abort();
    f.h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn maintenance_preparation_digest_writer_progress_timeout_and_restart() {
    let f = fixture().await;
    let first = maintenance::prepare_digest(&f.h.state, f.brain, f.actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    assert_eq!(
        maintenance::publish_digest(&f.h.state, first, 2).await.ok(),
        Some(1)
    );
    let before = coverage(&f).await;
    let calls = f.provider.calls.load(Ordering::SeqCst);
    install_gate(&f.h).await;
    let mut gate = f.h.admin.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(67126124,8)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let state = f.h.state.clone();
    let brain = f.brain;
    let actor = f.actor;
    let preparing =
        tokio::spawn(async move { maintenance::prepare_digest(&state, brain, actor).await });
    wait_for_gate(&f.h).await;
    same_brain_writer(&f).await;
    sqlx::query("SELECT pg_advisory_unlock(67126124,8)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let prepared = preparing.await.unwrap().ok().unwrap().unwrap();
    assert_eq!(
        maintenance::publish_digest(&f.h.state, prepared, 2)
            .await
            .ok(),
        Some(0)
    );
    assert_eq!(
        coverage(&f).await,
        before,
        "Changed snapshot cannot publish any page/coverage"
    );

    sqlx::query("SELECT pg_advisory_lock(67126124,8)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let state = f.h.state.clone();
    let timing_out =
        tokio::spawn(async move { maintenance::prepare_digest(&state, brain, actor).await });
    wait_for_gate(&f.h).await;
    let unresolved = tokio::time::timeout(WallDuration::from_secs(3), timing_out)
        .await
        .unwrap()
        .unwrap()
        .ok()
        .unwrap()
        .unwrap();
    sqlx::query("SELECT pg_advisory_unlock(67126124,8)")
        .execute(&mut *gate)
        .await
        .unwrap();
    assert_eq!(
        maintenance::publish_digest(&f.h.state, unresolved, 2)
            .await
            .ok(),
        Some(0)
    );
    assert_eq!(
        coverage(&f).await,
        before,
        "Statement timeout cannot fabricate empty/omitted coverage"
    );
    let active: bool =
        sqlx::query_scalar("SELECT bool_and(active) FROM session_digest_pages WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&f.h.admin)
            .await
            .unwrap();
    assert!(active, "Timeout cannot retire an existing desired page");
    let restarted = AppState::new(f.h.state.pool.clone(), (*f.h.state.config).clone()).unwrap();
    let again = maintenance::prepare_digest(&restarted, brain, actor)
        .await
        .ok()
        .unwrap()
        .unwrap();
    assert_eq!(
        maintenance::publish_digest(&restarted, again, 2).await.ok(),
        Some(0)
    );
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), calls);
    f.server.abort();
    f.h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn maintenance_preparation_handover_unavailable_prefix_wrap_and_writer_progress() {
    let f = fixture().await;
    *f.provider.candidates.lock().unwrap() =
        json!({"summary":"Amber declares port 8080.","completed":[],"next_steps":[],"risks":[]});
    ok(
        &f.h,
        "POST",
        &format!("{}/handovers", f.base),
        &f.owner,
        json!({"title":"Bounded refresh","contributions":[f.revision["id"]],"operation_id":null}),
    )
    .await;
    model_job(&f.h).await;
    // Explicit authenticated revalidation moves the contributor head while
    // retaining the same complete supported assertion.
    let reviewed = ok(&f.h,"POST",&format!("{}/claims/{}/review",f.base,f.revision["claim_id"].as_str().unwrap()),&f.owner,
        json!({"base_revision":f.revision["id"],"action":"revalidate","reason":"Synthetic current-input authority for native refresh proof.","revalidation_basis":"review_correction"})).await;
    let revised = &reviewed["claims"][0]["revision"];
    let sqlx::types::Json(template): sqlx::types::Json<Value> =
        sqlx::query_scalar("SELECT revision FROM claim_revisions WHERE brain_id=$1 AND id=$2")
            .bind(f.brain)
            .bind(revised["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&f.h.admin)
            .await
            .unwrap();
    // Canonically shaped non-candidates precede the real refresh candidate.
    for index in 1..=35_u128 {
        let id = Uuid::from_u128(index);
        let claim = Uuid::new_v4();
        let mut value = template.clone();
        value["id"] = json!(id);
        value["claim_id"] = json!(claim);
        value["origin"] = json!("browser_authored");
        sqlx::query(
            "INSERT INTO claims(id,brain_id,created_by,current_revision) VALUES($1,$2,$3,NULL)",
        )
        .bind(claim)
        .bind(f.brain)
        .bind(f.actor)
        .execute(&f.h.admin)
        .await
        .unwrap();
        sqlx::query("INSERT INTO claim_revisions(id,claim_id,brain_id,recorded_at,revision,subject_key,predicate_key,value_key) SELECT $1,$2,brain_id,recorded_at,$3,subject_key,predicate_key,value_key FROM claim_revisions WHERE brain_id=$4 AND id=$5")
            .bind(id).bind(claim).bind(sqlx::types::Json(value)).bind(f.brain).bind(template["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&f.h.admin).await.unwrap();
        sqlx::query("UPDATE claims SET current_revision=$1 WHERE id=$2")
            .bind(id)
            .bind(claim)
            .execute(&f.h.admin)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE autonomous_maintenance_cursors SET last_id=NULL,version=version+1 WHERE brain_id=$1 AND kind='handover'")
        .bind(f.brain).execute(&f.h.admin).await.unwrap();
    let calls = f.provider.calls.load(Ordering::SeqCst);
    for _ in 0..2 {
        let restarted = AppState::new(f.h.state.pool.clone(), (*f.h.state.config).clone()).unwrap();
        let prepared = maintenance::prepare_handovers(&restarted, f.brain, f.actor)
            .await
            .ok()
            .unwrap()
            .unwrap();
        assert_eq!(
            maintenance::publish_handovers(&restarted, prepared, 5)
                .await
                .ok(),
            Some(0)
        );
    }
    install_gate(&f.h).await;
    let mut gate = f.h.admin.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(67126124,8)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let state = f.h.state.clone();
    let brain = f.brain;
    let actor = f.actor;
    let preparing =
        tokio::spawn(async move { maintenance::prepare_handovers(&state, brain, actor).await });
    wait_for_gate(&f.h).await;
    same_brain_writer(&f).await;
    sqlx::query("SELECT pg_advisory_unlock(67126124,8)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let stale = preparing.await.unwrap().ok().unwrap().unwrap();
    assert_eq!(
        maintenance::publish_handovers(&f.h.state, stale, 5)
            .await
            .ok(),
        Some(0)
    );
    let mut queued = 0;
    for _ in 0..4 {
        let restarted = AppState::new(f.h.state.pool.clone(), (*f.h.state.config).clone()).unwrap();
        let prepared = maintenance::prepare_handovers(&restarted, brain, actor)
            .await
            .ok()
            .unwrap()
            .unwrap();
        queued += maintenance::publish_handovers(&restarted, prepared, 5)
            .await
            .ok()
            .unwrap();
    }
    assert_eq!(
        queued, 1,
        "The unavailable prefix cannot starve refresh after restart/wrap"
    );
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), calls);
    f.server.abort();
    f.h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn maintenance_preparation_correction_erasure_and_wall_clock_expiry_reject_publication() {
    for scenario in ["correction", "erasure", "expiry"] {
        let f = fixture().await;
        if scenario == "expiry" {
            let mut settings = ok(
                &f.h,
                "GET",
                &format!("{}/retention", f.base),
                &f.owner,
                Value::Null,
            )
            .await;
            settings["policy"]["claim_days"] = json!(1);
            ok(
                &f.h,
                "PUT",
                &format!("{}/retention", f.base),
                &f.owner,
                json!({"base_change":settings["change_id"],"policy":settings["policy"]}),
            )
            .await;
            // The canonical clock deadline crosses after preparation. No writer
            // or expiry sweep advances the epoch during the actual wait.
            sqlx::query("WITH instant AS (SELECT clock_timestamp()-interval '1 day'+interval '2 seconds' AS deadline_at)
                UPDATE claim_revisions r SET recorded_at=instant.deadline_at,revision=jsonb_set(r.revision,'{recorded_at}',to_jsonb(instant.deadline_at)) FROM instant WHERE r.brain_id=$1 AND r.id=$2")
                .bind(f.brain).bind(f.revision["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&f.h.admin).await.unwrap();
        }
        let before = coverage(&f).await;
        let calls = f.provider.calls.load(Ordering::SeqCst);
        let prepared = maintenance::prepare_digest(&f.h.state, f.brain, f.actor)
            .await
            .ok()
            .unwrap()
            .unwrap();
        assert_eq!(
            prepared.page_count(),
            Some(1),
            "{scenario}: race requires a fully prepared eligible page"
        );
        match scenario {
            "correction" => {
                let mut content = f.revision["content"].clone();
                content["value"] = json!("9090");
                ok(&f.h,"POST",&format!("{}/claims/{}/review",f.base,f.revision["claim_id"].as_str().unwrap()),&f.owner,
                    json!({"base_revision":f.revision["id"],"action":"correct","content":content,"reason":"Synthetic authoritative correction races prepared publication."})).await;
            }
            "erasure" => {
                let target = json!({"kind":"claim","id":f.revision["claim_id"]});
                let preview = ok(
                    &f.h,
                    "POST",
                    &format!("{}/erasures/preview", f.base),
                    &f.owner,
                    target.clone(),
                )
                .await;
                ok(
                    &f.h,
                    "POST",
                    &format!("{}/erasures", f.base),
                    &f.owner,
                    json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
                )
                .await;
            }
            "expiry" => {
                let epoch: i64 =
                    sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
                        .bind(f.brain)
                        .fetch_one(&f.h.admin)
                        .await
                        .unwrap();
                tokio::time::sleep(WallDuration::from_millis(2200)).await;
                let after: i64 =
                    sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
                        .bind(f.brain)
                        .fetch_one(&f.h.admin)
                        .await
                        .unwrap();
                assert_eq!(
                    epoch, after,
                    "Wall-clock expiry proof must not depend on epoch invalidation"
                );
            }
            _ => unreachable!(),
        }
        assert_eq!(
            maintenance::publish_digest(&f.h.state, prepared, 2)
                .await
                .ok(),
            Some(0),
            "{scenario}"
        );
        let generations: i64 =
            sqlx::query_scalar("SELECT count(*) FROM session_digest_generations WHERE brain_id=$1")
                .bind(f.brain)
                .fetch_one(&f.h.admin)
                .await
                .unwrap();
        assert_eq!(generations, 0, "{scenario} cannot admit stale model work");
        if scenario != "erasure" {
            assert_eq!(coverage(&f).await, before);
        }
        assert_eq!(f.provider.calls.load(Ordering::SeqCst), calls);
        f.server.abort();
        f.h.finish().await;
    }
}
