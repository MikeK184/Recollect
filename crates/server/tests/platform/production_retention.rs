//! Real learning calls use a controlled loopback provider. Release catalogue
//! rows below are explicitly synthetic test receipts, never shipped defaults.
use super::*;
use recollect_server::{knowledge_mapping::producer, privacy_journal};

fn brain(base: &str) -> Uuid {
    base.rsplit('/').next().unwrap().parse().unwrap()
}
fn id(value: &Value) -> Uuid {
    value["id"].as_str().unwrap().parse().unwrap()
}
fn version(value: &Value) -> Uuid {
    value["version"]["id"].as_str().unwrap().parse().unwrap()
}

async fn blocked(h: &Harness, owner: &Login, base: &str, source: &Value) -> Uuid {
    let run = learn(h, owner, base, source).await;
    model_job(h).await;
    let run = id(&run);
    let state: String =
        sqlx::query_scalar("SELECT state FROM knowledge_mapping_coverage WHERE run_id=$1")
            .bind(run)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(state, "blocked_profile");
    run
}

async fn catalogue(h: &Harness, base: &str, name: &str) {
    sqlx::query("INSERT INTO knowledge_mapping_release_producers(id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_receipt_sha256,fixture_sha256,recipe_sha256,held_out_documents,entity_precision,entity_recall,endpoint_precision,endpoint_recall) SELECT $1,p.policy->>'provider',p.policy->>'text_model',p.policy->>'text_model',$3,$3,$4,'source-mentions-1',repeat('a',64),repeat('b',64),repeat('c',64),18,0.96,0.85,0.96,0.85 FROM model_policy_heads h JOIN model_policies p ON p.brain_id=h.brain_id AND p.id=h.policy_id WHERE h.brain_id=$2")
        .bind(name).bind(brain(base)).bind(gateway::EXTRACT_PROMPT).bind(recollect_server::knowledge_mapping::production::ADAPTER).execute(&h.admin).await.unwrap();
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback provider"]
async fn production_retention_bootstrap_reuses_exact_output_without_model_retry() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = production_mapping::output();
    let source = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    production_mapping::process(&h).await;
    let run = blocked(&h, &owner, &base, &source).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        0,
        "An empty release catalogue does not qualify output"
    );
    let body: sqlx::types::Json<Value> =
        sqlx::query_scalar("SELECT payload FROM knowledge_mapping_stages WHERE run_id=$1")
            .bind(run)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(body["source_version_id"], json!(version(&source)));
    assert_eq!(body["mentions"][0]["span"]["quote"], "Rust");
    // The application cannot certify a model by inserting a claimed receipt.
    assert!(sqlx::query("INSERT INTO knowledge_mapping_release_producers SELECT * FROM knowledge_mapping_release_producers").execute(&h.state.pool).await.is_err());
    let unknown: Option<Uuid> = sqlx::query_scalar("SELECT recollect_mapping_bootstrap($1)")
        .bind(brain(&base))
        .fetch_one(&h.state.pool)
        .await
        .unwrap();
    assert_eq!(unknown, None, "Missing actor authority fails closed");
    catalogue(&h, &base, "synthetic-disposable-only").await;
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        1
    );
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        0
    );
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_mapping_stages WHERE run_id=$1),(SELECT count(*) FROM knowledge_mapping_producer_heads WHERE brain_id=$2),(SELECT count(*) FROM jobs WHERE brain_id=$2 AND kind='knowledge.map')")
        .bind(run).bind(brain(&base)).fetch_one(&h.admin).await.unwrap();
    assert_eq!(counts, (0, 1, 1));
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let view = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await;
    assert_eq!(view["entities"].as_array().unwrap().len(), 2);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Activation never re-invokes the extractor"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback provider"]
async fn production_retention_old_and_queued_results_never_replace_a_newer_published_generation() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = production_mapping::output();
    let source = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    production_mapping::process(&h).await;
    let old = blocked(&h, &owner, &base, &source).await;
    production_mapping::synthetic_profile(&h, &base, false).await;
    let queued = learn(&h, &owner, &base, &source).await;
    model_job(&h).await;
    let queued = id(&queued);
    sqlx::query("UPDATE jobs SET not_before=clock_timestamp()+interval '1 hour' WHERE target_id=(SELECT input_id FROM knowledge_mapping_coverage WHERE run_id=$1) AND kind='knowledge.map'").bind(queued).execute(&h.admin).await.unwrap();
    let mut fresh = production_mapping::output();
    fresh["mapping"]["mentions"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    fresh["mapping"]["relations"] = json!([]);
    *p.candidates.lock().unwrap() = fresh;
    let latest = learn(&h, &owner, &base, &source).await;
    model_job(&h).await;
    let latest = id(&latest);
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let winner: Uuid = sqlx::query_scalar(
        "SELECT input_id FROM knowledge_mapping_coverage WHERE run_id=$1 AND state='published'",
    )
    .bind(latest)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    // Clock movement cannot choose a winner; immutable admission order does.
    sqlx::query("UPDATE model_requests SET created_at=clock_timestamp()+interval '1 hour' WHERE id=(SELECT request_id FROM learning_runs WHERE id=$1)").bind(old).execute(&h.admin).await.unwrap();
    let before = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await;
    assert_eq!(before["entities"].as_array().unwrap().len(), 1);
    let topic = &before["topics"][0];
    let renamed=h.keyed("PUT",&format!("{base}/organization/topics/{}",topic["id"].as_str().unwrap()),Some(&owner),json!({"base_change_id":topic["change_id"],"change":{"action":"rename","label":"Newer generation label","basis_input":winner}}),Some("newer-generation-label")).await;
    assert_eq!(renamed.0, StatusCode::OK, "{}", renamed.1);
    let admission: i64 = sqlx::query_scalar("SELECT o.ordinal FROM knowledge_mapping_request_order o JOIN learning_runs r ON r.brain_id=o.brain_id AND r.request_id=o.request_id WHERE r.id=$1")
        .bind(latest).fetch_one(&h.admin).await.unwrap();
    // Expiring audit details must preserve both the published interpretation
    // and its monotonic order; old pending output still cannot take its place.
    sqlx::query("UPDATE model_requests SET created_at='2000-01-01T00:00:00Z' WHERE id=(SELECT request_id FROM learning_runs WHERE id=$1)")
        .bind(latest).execute(&h.admin).await.unwrap();
    sqlx::query("SELECT recollect_expire_model_details()")
        .execute(&h.state.pool)
        .await
        .unwrap();
    let after_expiry: (bool, i64) = sqlx::query_as("SELECT q.detail_expired,o.ordinal FROM learning_runs r JOIN model_requests q ON q.brain_id=r.brain_id AND q.id=r.request_id JOIN knowledge_mapping_request_order o ON o.brain_id=q.brain_id AND o.request_id=q.id WHERE r.id=$1")
        .bind(latest).fetch_one(&h.admin).await.unwrap();
    assert_eq!(after_expiry, (true, admission));
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        0
    );
    let old_state:(String,i64)=sqlx::query_as("SELECT c.state,(SELECT count(*) FROM knowledge_mapping_stages WHERE run_id=c.run_id) FROM knowledge_mapping_coverage c WHERE c.run_id=$1").bind(old).fetch_one(&h.admin).await.unwrap();
    assert_eq!(old_state, ("superseded".into(), 0));
    sqlx::query("UPDATE jobs SET not_before=clock_timestamp() WHERE target_id=(SELECT input_id FROM knowledge_mapping_coverage WHERE run_id=$1) AND kind='knowledge.map'").bind(queued).execute(&h.admin).await.unwrap();
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let queued_state:(String,String)=sqlx::query_as("SELECT c.state,j.error_code FROM knowledge_mapping_coverage c JOIN learning_runs r ON r.id=c.run_id JOIN jobs j ON j.brain_id=r.brain_id AND j.kind='knowledge.map' AND j.target_id<>(SELECT input_id FROM knowledge_mapping_coverage WHERE run_id=$2) WHERE c.run_id=$1 LIMIT 1").bind(queued).bind(latest).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        queued_state,
        ("superseded".into(), "mapping_superseded".into())
    );
    let remaining: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM knowledge_mapping_inputs WHERE brain_id=$1")
            .bind(brain(&base))
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert_eq!(remaining, vec![winner]);
    let after = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await;
    assert_eq!(after["entities"].as_array().unwrap().len(), 1);
    assert!(
        after["topics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["label"] == "Newer generation label")
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        3,
        "Resuming/discarding never invokes a provider"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback provider"]
async fn production_retention_ambiguous_catalogue_context_suppression_and_expiry() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = production_mapping::output();
    let source = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    production_mapping::process(&h).await;
    let run = blocked(&h, &owner, &base, &source).await;
    catalogue(&h, &base, "synthetic-a").await;
    catalogue(&h, &base, "synthetic-b").await;
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        0,
        "Ambiguous profiles are not selected"
    );
    let heads: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM knowledge_mapping_producer_heads WHERE brain_id=$1",
    )
    .bind(brain(&base))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(heads, 0);
    let group = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Moved after extraction"}),
    )
    .await;
    ok(
        &h,
        "PUT",
        &format!("{base}/sources/{}/groups", source["id"].as_str().unwrap()),
        &owner,
        json!({"group_ids":[group["id"]]}),
    )
    .await;
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        0
    );
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_stages WHERE run_id=$1")
            .bind(run)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        remaining, 0,
        "A changed realm cannot inherit old mapping output"
    );
    let suppressed = blocked(&h, &owner, &base, &source).await;
    sqlx::query("UPDATE model_requests SET suppressed=true WHERE id=(SELECT request_id FROM learning_runs WHERE id=$1)").bind(suppressed).execute(&h.admin).await.unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_stages WHERE run_id=$1")
            .bind(suppressed)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        remaining, 0,
        "Suppression physically removes unpublished text"
    );
    let expired = blocked(&h, &owner, &base, &source).await;
    sqlx::query("UPDATE knowledge_mapping_stages SET expires_at=clock_timestamp()-interval '1 second' WHERE run_id=$1").bind(expired).execute(&h.admin).await.unwrap();
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        0
    );
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_stages WHERE run_id=$1")
            .bind(expired)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback provider"]
async fn production_retention_capacity_and_fair_bounded_maintenance() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = production_mapping::output();
    let source = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    production_mapping::process(&h).await;
    let run = blocked(&h, &owner, &base, &source).await;
    // Explicit administrative load fixture: clone the complete provenance of
    // one actual controlled call. This is a storage/maintenance bound, never a
    // claim that 1,000 provider calls ran or extractor quality was measured.
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::query("SELECT set_config('recollect.fixture_run',$1,true)")
        .bind(run.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::raw_sql("DO $$ DECLARE r learning_runs; q model_requests; j jobs; s knowledge_mapping_stages; rid uuid; qid uuid; jid uuid; n integer; BEGIN SELECT * INTO STRICT r FROM learning_runs WHERE id=current_setting('recollect.fixture_run')::uuid; SELECT * INTO STRICT q FROM model_requests WHERE id=r.request_id; SELECT * INTO STRICT j FROM jobs WHERE id=r.job_id; SELECT * INTO STRICT s FROM knowledge_mapping_stages WHERE run_id=r.id; FOR n IN 1..999 LOOP rid:=gen_random_uuid(); qid:=gen_random_uuid(); jid:=gen_random_uuid(); INSERT INTO jobs SELECT (jsonb_populate_record(NULL::jobs,to_jsonb(j)||jsonb_build_object('id',jid,'target_id',rid))).*; INSERT INTO model_requests SELECT (jsonb_populate_record(NULL::model_requests,to_jsonb(q)||jsonb_build_object('id',qid,'operation_id',rid,'call_token',gen_random_uuid()))).*; INSERT INTO model_request_inputs(request_id,brain_id,kind,input_id) SELECT qid,brain_id,kind,input_id FROM model_request_inputs WHERE request_id=q.id; INSERT INTO learning_runs SELECT (jsonb_populate_record(NULL::learning_runs,to_jsonb(r)||jsonb_build_object('id',rid,'job_id',jid,'request_id',qid))).*; INSERT INTO knowledge_mapping_stages SELECT (jsonb_populate_record(NULL::knowledge_mapping_stages,to_jsonb(s)||jsonb_build_object('run_id',rid,'request_id',qid))).*; END LOOP; END $$")
        .execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    for expected in [100_i64, 200] {
        sqlx::query("SELECT recollect_mapping_stage_maintain()")
            .execute(&h.state.pool)
            .await
            .unwrap();
        let examined:i64=sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_stages WHERE brain_id=$1 AND maintenance_checked_at>'-infinity'::timestamptz").bind(brain(&base)).fetch_one(&h.admin).await.unwrap();
        assert_eq!(
            examined, expected,
            "Each tick examines only100 and advances past the valid prefix"
        );
    }
    let overflow = learn(&h, &owner, &base, &source).await;
    model_job(&h).await;
    let states:(String,String)=sqlx::query_as("SELECT r.state,c.state FROM learning_runs r JOIN knowledge_mapping_coverage c ON c.run_id=r.id WHERE r.id=$1").bind(id(&overflow)).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        states,
        ("succeeded".into(), "blocked_capacity".into()),
        "Optional mapping capacity cannot fail existing learning"
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_stages WHERE brain_id=$1")
            .bind(brain(&base))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(count, 1000);
    sqlx::query("UPDATE knowledge_mapping_stages SET expires_at=clock_timestamp()-interval '1 second' WHERE brain_id=$1").bind(brain(&base)).execute(&h.admin).await.unwrap();
    for remaining in (0_i64..=900).rev().step_by(100) {
        sqlx::query("SELECT recollect_mapping_stage_maintain()")
            .execute(&h.state.pool)
            .await
            .unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_stages WHERE brain_id=$1")
                .bind(brain(&base))
                .fetch_one(&h.admin)
                .await
                .unwrap();
        assert_eq!(count, remaining);
    }
    let _populated = blocked(&h, &owner, &base, &source).await;
    let archived = ok(&h, "PATCH", &base, &owner, json!({"archived":true})).await;
    assert_eq!(archived["archived"], true);
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/deletions/preview"),
        &owner,
        json!({}),
    )
    .await;
    let removed = h
        .call(
            "DELETE",
            &base,
            Some(&owner),
            json!({"closure":preview["closure"],"confirmation":"Synthetic provider proof"}),
        )
        .await;
    assert_eq!(removed.0, StatusCode::OK, "{}", removed.1);
    let stages: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_stages WHERE brain_id=$1")
            .bind(brain(&base))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        stages, 0,
        "Actual Brain deletion removes a populated unpublished stage"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback provider"]
async fn production_retention_erasure_replays_into_older_database_without_new_manifest_fields() {
    let (mut h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = production_mapping::output();
    let parent = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    let independent = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    production_mapping::process(&h).await;
    let excerpt=ok(&h,"POST",&format!("{base}/excerpts"),&owner,json!({"source_id":parent["id"],"version_id":parent["version"]["id"],"first_line":1,"last_line":1,"title":"Synthetic retained mapping descendant"})).await;
    production_mapping::process(&h).await;
    let parent_run = blocked(&h, &owner, &base, &parent).await;
    let excerpt_run = blocked(&h, &owner, &base, &excerpt).await;
    let independent_run = blocked(&h, &owner, &base, &independent).await;
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(
            &parent["version"]["id"],
            "Synthetic retained mapping source",
            "fixture",
        ),
    )
    .await;
    privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
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
        h.state.config.as_ref().clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    admin_url.set_path(&backup);
    let backup_admin = db::pool(admin_url.as_str()).await.unwrap();
    let mut config = h.state.config.as_ref().clone();
    let mut url = reqwest::Url::parse(&config.database_url).unwrap();
    url.set_path(&backup);
    config.database_url = url.to_string();
    config.artifact_dir = format!(".cache/{backup}-artifacts");
    let backup_pool = db::pool(&config.database_url).await.unwrap();
    let target = json!({"kind":"claim","id":claim["claim_id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    let closure: sqlx::types::Json<Value> =
        sqlx::query_scalar("SELECT recollect_privacy_closure($1,'claim',$2,'erase')")
            .bind(brain(&base))
            .bind(claim["claim_id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(
        closure["knowledge_mapping_stages"]
            .as_array()
            .unwrap()
            .contains(&json!(parent_run))
    );
    assert!(
        closure["knowledge_mapping_stages"]
            .as_array()
            .unwrap()
            .contains(&json!(excerpt_run))
    );
    // Simulate an old journal producer before the actual first apply, so the
    // missing-key proof cannot pass merely because an earlier apply removed it.
    sqlx::raw_sql("CREATE FUNCTION fixture_legacy_mapping_stage_manifest() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.manifest:=NEW.manifest-'knowledge_mapping_stages'-'model_input_sources'; RETURN NEW; END $$; CREATE TRIGGER fixture_legacy_mapping_stage_manifest BEFORE INSERT ON privacy_requests FOR EACH ROW EXECUTE FUNCTION fixture_legacy_mapping_stage_manifest()")
        .execute(&h.admin).await.unwrap();
    let removal = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let manifest: sqlx::types::Json<Value> =
        sqlx::query_scalar("SELECT manifest FROM privacy_requests WHERE id=$1")
            .bind(id(&removal))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(
        manifest.get("knowledge_mapping_stages").is_none(),
        "Actual first apply uses the legacy manifest without stage IDs"
    );
    privacy_journal::maintain(&h.state).await.unwrap();
    assert!(
        privacy_journal::barrier(&backup_pool, &config)
            .await
            .is_err()
    );
    privacy_journal::reconcile(&backup_admin, &config)
        .await
        .unwrap();
    privacy_journal::barrier(&backup_pool, &config)
        .await
        .unwrap();
    for pool in [&h.admin, &backup_admin] {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT run_id FROM knowledge_mapping_stages WHERE brain_id=$1 ORDER BY run_id",
        )
        .bind(brain(&base))
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(ids, vec![independent_run]);
    }
    let left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_stages WHERE brain_id=$1")
            .bind(brain(&base))
            .fetch_one(&backup_admin)
            .await
            .unwrap();
    assert_eq!(left, 1);
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    backup_pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}
