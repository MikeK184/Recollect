//! Actual gateway/automatic-learning plumbing under explicitly synthetic quality
//! profiles installed only in disposable DBs. This is not model quality proof.
use super::*;
use recollect_server::knowledge_mapping::production;

fn brain_id(base: &str) -> Uuid {
    base.rsplit('/').next().unwrap().parse().unwrap()
}
pub(super) fn output() -> Value {
    json!({"claims":[],"retirements":[],"mapping":{"language":"en","mentions":[
        {"key":"rust","kind":"technology","span":{"quote":"Rust","line_from":1,"line_to":1,"occurrence":null},"confidence":0.9},
        {"key":"postgres","kind":"technology","span":{"quote":"PostgreSQL","line_from":1,"line_to":1,"occurrence":null},"confidence":0.9}],
        "aliases":[],"relations":[{"from":"rust","to":"postgres","relation":"related_to","span":{"quote":"é Rust uses PostgreSQL.","line_from":1,"line_to":1,"occurrence":null}}]}})
}
pub(super) async fn synthetic_profile(h: &Harness, base: &str, automatic: bool) -> Uuid {
    let change = Uuid::new_v4();
    let prompt = if automatic {
        recollect_server::autonomous::PROMPT
    } else {
        gateway::EXTRACT_PROMPT
    };
    // Receipt hash is an explicit synthetic fixture marker. No such profile is
    // seeded or selected in a real installation by migrations or runtime code.
    sqlx::query("INSERT INTO knowledge_mapping_producer_profiles(change_id,brain_id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_state,quality_receipt_sha256) SELECT $1,$2,p.policy->>'provider',p.policy->>'text_model',p.policy->>'text_model',$3,$3,$4,'source-mentions-1','qualified',repeat('b',64) FROM model_policy_heads ph JOIN model_policies p ON p.id=ph.policy_id AND p.brain_id=ph.brain_id WHERE ph.brain_id=$2")
        .bind(change).bind(brain_id(base)).bind(prompt).bind(production::ADAPTER).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_producer_heads(brain_id,change_id) VALUES($1,$2)")
        .bind(brain_id(base))
        .bind(change)
        .execute(&h.admin)
        .await
        .unwrap();
    change
}
pub(super) async fn process(h: &Harness) {
    while worker::run_once(&h.state, "capture").await.unwrap() {}
}
async fn visible(h: &Harness, owner: &Login, base: &str) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/organization/view"),
        owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback provider"]
async fn production_mapping_existing_automatic_extraction_publishes_zero_claim_source() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    let profile = synthetic_profile(&h, &base, true).await;
    *p.candidates.lock().unwrap() = output();
    let _source = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    process(&h).await;
    // No explicit learning or mapping command: the standing scheduler admits
    // the existing extraction and its successful commit queues mapping work.
    assert!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1))
            > 0
    );
    model_job(&h).await;
    let run: (Uuid, String, bool, i32, Uuid) = sqlx::query_as(
        "SELECT id,state,automatic,accepted,request_id FROM learning_runs WHERE brain_id=$1",
    )
    .bind(brain_id(&base))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!((&run.1, run.2, run.3), (&"succeeded".to_string(), true, 0));
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "NER uses the existing extraction, no extra call"
    );
    let input: Uuid = sqlx::query_scalar(
        "SELECT input_id FROM knowledge_mapping_coverage WHERE run_id=$1 AND state='staged'",
    )
    .bind(run.0)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        visible(&h, &owner, &base).await["entities"]
            .as_array()
            .unwrap()
            .len(),
        0,
        "Unpublished producer stages are invisible"
    );
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let published: (String,Uuid,Uuid,bool)=sqlx::query_as("SELECT i.state,i.producer_request_id,i.producer_change_id,o.published_at IS NOT NULL FROM knowledge_mapping_inputs i JOIN knowledge_mapping_origins o ON o.input_id=i.id WHERE i.id=$1")
        .bind(input).fetch_one(&h.admin).await.unwrap();
    assert_eq!(published, ("published".into(), run.4, profile, true));
    let page = visible(&h, &owner, &base).await;
    assert_eq!(page["entities"].as_array().unwrap().len(), 2, "{page}");
    assert_eq!(
        page["topics"].as_array().unwrap().len(),
        2,
        "Transparent technology facets overlap this source"
    );
    let span: (i32, String) = sqlx::query_as(
        "SELECT byte_start,quote FROM knowledge_mentions WHERE input_id=$1 AND local_key='rust'",
    )
    .bind(input)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        span,
        (3, "Rust".into()),
        "Unicode coordinates are resolved by server code"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    // Ordinary request detail expiry cannot erase a retained published semantic
    // origin; source fences/current profile remain authoritative.
    sqlx::query("UPDATE model_requests SET model='detail_expired',returned_model=NULL,prompt_label='detail_expired',schema_label='detail_expired',detail_expired=true WHERE id=$1")
        .bind(run.4).execute(&h.admin).await.unwrap();
    assert_eq!(
        visible(&h, &owner, &base).await["entities"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let before: i64 = sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
        .bind(brain_id(&base))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    sqlx::query("DELETE FROM knowledge_mapping_producer_heads WHERE brain_id=$1")
        .bind(brain_id(&base))
        .execute(&h.admin)
        .await
        .unwrap();
    let after: i64 = sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
        .bind(brain_id(&base))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(after > before);
    assert_eq!(
        visible(&h, &owner, &base).await["entities"]
            .as_array()
            .unwrap()
            .len(),
        0,
        "Profile withdrawal invalidates visibility"
    );
    sqlx::query("INSERT INTO knowledge_mapping_producer_heads(brain_id,change_id) VALUES($1,$2)")
        .bind(brain_id(&base))
        .bind(profile)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        visible(&h, &owner, &base).await["entities"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // Physical producer suppression must remove its exact mapping payload and
    // all copied contributor records, not merely hide it in the reader.
    sqlx::query("UPDATE model_requests SET suppressed=true WHERE id=$1")
        .bind(run.4)
        .execute(&h.admin)
        .await
        .unwrap();
    let remaining:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM knowledge_mapping_inputs WHERE id=$1)+(SELECT count(*) FROM knowledge_mentions WHERE input_id=$1)+(SELECT count(*) FROM knowledge_topic_memberships WHERE input_id=$1)")
        .bind(input).fetch_one(&h.admin).await.unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback provider"]
async fn production_mapping_scope_change_during_call_never_rebinds_the_response() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    synthetic_profile(&h, &base, false).await;
    *p.candidates.lock().unwrap() = output();
    p.delay.store(250, Ordering::SeqCst);
    let source = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    process(&h).await;
    let run = learn(&h, &owner, &base, &source).await;
    let mut working = Box::pin(worker::run_once(&h.state, "model"));
    tokio::select! {
        _=p.entered.notified()=>{},
        result=&mut working=>panic!("Worker finished before controlled existing extraction call: {result:?}"),
        _=tokio::time::sleep(std::time::Duration::from_secs(3))=>panic!("Provider fixture was not invoked"),
    }
    let group = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Moved during extraction"}),
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
    assert!(working.await.unwrap());
    let states:(String,String)=sqlx::query_as("SELECT r.state,c.state FROM learning_runs r JOIN knowledge_mapping_coverage c ON c.run_id=r.id WHERE r.id=$1")
        .bind(run["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    assert_eq!(states, ("succeeded".into(), "invalid".into()));
    let mappings: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_inputs WHERE brain_id=$1")
            .bind(brain_id(&base))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(mappings, 0);
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback provider"]
async fn production_mapping_missing_legacy_output_never_reinvokes_provider() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = json!({"claims":[]});
    let source = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    process(&h).await;
    let run = learn(&h, &owner, &base, &source).await;
    model_job(&h).await;
    let run_id: Uuid = run["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    // Restore only the durable work status to simulate restart after a retained
    // legacy support stage. The real extraction receipt and stage are unchanged.
    sqlx::query("DELETE FROM knowledge_mapping_coverage WHERE run_id=$1")
        .bind(run_id)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE learning_runs SET state='queued',finished_at=NULL WHERE id=$1")
        .bind(run_id)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET state='queued',attempts=0,not_before=clock_timestamp() WHERE id=(SELECT job_id FROM learning_runs WHERE id=$1)")
        .bind(run_id).execute(&h.admin).await.unwrap();
    model_job(&h).await;
    let state: String =
        sqlx::query_scalar("SELECT state FROM knowledge_mapping_coverage WHERE run_id=$1")
            .bind(run_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(state, "missing");
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    let jobs: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE brain_id=$1 AND kind='knowledge.map'")
            .bind(brain_id(&base))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(jobs, 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback provider"]
async fn production_mapping_blocked_profile_invalid_anchor_and_stale_scope_are_unpublished() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = output();
    let source = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    process(&h).await;
    let run = learn(&h, &owner, &base, &source).await;
    model_job(&h).await;
    let blocked: String =
        sqlx::query_scalar("SELECT state FROM knowledge_mapping_coverage WHERE run_id=$1")
            .bind(run["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(blocked, "blocked_profile");
    synthetic_profile(&h, &base, false).await;
    let mut invalid = output();
    invalid["mapping"]["mentions"][0]["span"]["quote"] = json!("Foreign input only");
    *p.candidates.lock().unwrap() = invalid;
    let bad = learn(&h, &owner, &base, &source).await;
    model_job(&h).await;
    let bad_state:(String,String)=sqlx::query_as("SELECT r.state,c.state FROM learning_runs r JOIN knowledge_mapping_coverage c ON c.run_id=r.id WHERE r.id=$1")
        .bind(bad["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    assert_eq!(bad_state, ("succeeded".into(), "invalid".into()));
    *p.candidates.lock().unwrap() = output();
    let fresh = learn(&h, &owner, &base, &source).await;
    model_job(&h).await;
    let input: Uuid = sqlx::query_scalar(
        "SELECT input_id FROM knowledge_mapping_coverage WHERE run_id=$1 AND state='staged'",
    )
    .bind(fresh["id"].as_str().unwrap().parse::<Uuid>().unwrap())
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let group = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Changed scope"}),
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
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let state:(String,String)=sqlx::query_as("SELECT i.state,j.state FROM knowledge_mapping_inputs i JOIN jobs j ON j.target_id=i.id AND j.kind='knowledge.map' WHERE i.id=$1")
        .bind(input).fetch_one(&h.admin).await.unwrap();
    assert_eq!(state, ("staged".into(), "cancelled".into()));
    assert_eq!(
        visible(&h, &owner, &base).await["entities"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        3,
        "All outcomes use only their original extraction calls"
    );
    server.abort();
    h.finish().await;
}
