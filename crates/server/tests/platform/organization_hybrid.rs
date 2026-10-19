//! Synthetic finite vectors prove retained-only plumbing and privacy. They are
//! deliberately not an extractor/vector quality receipt or a selected release.
use super::*;
use recollect_server::graph::analytics;

pub(super) async fn select(h: &Harness, brain: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_organization_recipes(change_id,brain_id,recipe,gds_version,quality_state,quality_receipt_sha256,embedding_provider,embedding_model,embedding_dimensions,representation) SELECT $1,$2,'source-idf-retained-vector-leiden-1','2026.08.1','qualified',repeat('d',64),policy->>'provider',policy->>'embedding_model',(policy->>'embedding_dimensions')::int,'engineering-text-1' FROM model_policies p JOIN model_policy_heads h ON h.policy_id=p.id AND h.brain_id=p.brain_id WHERE h.brain_id=$2")
        .bind(id).bind(brain).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_organization_heads(brain_id,change_id) VALUES($1,$2) ON CONFLICT(brain_id) DO UPDATE SET change_id=excluded.change_id").bind(brain).bind(id).execute(&h.admin).await.unwrap();
    id
}
pub(super) async fn retained_profile(h: &Harness, brain: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO semantic_profiles(id,brain_id,provider,model,dimensions,representation,created_by,policy_id) SELECT $1,$2,policy->>'provider',policy->>'embedding_model',(policy->>'embedding_dimensions')::int,'engineering-text-1',created_by,p.id FROM model_policies p JOIN model_policy_heads h ON h.policy_id=p.id AND h.brain_id=p.brain_id WHERE h.brain_id=$2")
        .bind(id).bind(brain).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO semantic_heads(brain_id,profile_id) VALUES($1,$2) ON CONFLICT(brain_id) DO UPDATE SET profile_id=excluded.profile_id").bind(brain).bind(id).execute(&h.admin).await.unwrap();
    id
}
pub(super) async fn retained(
    h: &Harness,
    brain: Uuid,
    profile: Uuid,
    version: Uuid,
    vector: &[f32],
) -> (Uuid, Vec<Uuid>) {
    let request = Uuid::new_v4();
    sqlx::query("INSERT INTO model_requests(id,brain_id,actor_id,operation_id,policy_id,purpose,model,returned_model,prompt_label,schema_label,state,call_token,reserved_tokens,charged_tokens,dimensions,provider,finished_at) SELECT $1,$2,p.created_by,$3,p.id,'embedding',policy->>'embedding_model',policy->>'embedding_model','engineering-text-1','embedding-float-1','succeeded',$4,1,0,(policy->>'embedding_dimensions')::int,policy->>'provider',clock_timestamp() FROM model_policies p JOIN model_policy_heads h ON h.brain_id=p.brain_id AND h.policy_id=p.id WHERE p.brain_id=$2")
        .bind(request).bind(brain).bind(Uuid::new_v4()).bind(Uuid::new_v4()).execute(&h.admin).await.unwrap();
    let entries:Vec<Uuid>=sqlx::query_scalar("INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id,state,embedding,request_id) SELECT gen_random_uuid(),$1,$2,'source_chunk',id,id,version_id,'ready',$4::text::vector,$5 FROM source_chunks WHERE brain_id=$1 AND version_id=$3 RETURNING id")
        .bind(brain).bind(profile).bind(version).bind(serde_json::to_string(vector).unwrap()).bind(request).fetch_all(&h.admin).await.unwrap();
    assert!(!entries.is_empty());
    sqlx::query("INSERT INTO model_request_inputs(request_id,brain_id,kind,input_id) VALUES($1,$2,'source_version',$3)").bind(request).bind(brain).bind(version).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO model_request_inputs(request_id,brain_id,kind,input_id) SELECT $1,$2,'semantic_entry',unnest($3::uuid[])").bind(request).bind(brain).bind(&entries).execute(&h.admin).await.unwrap();
    (request, entries)
}
pub(super) async fn calculate(h: &Harness, state: &AppState, brain: Uuid) -> Uuid {
    // Use the production scheduler/worker task boundary so their large debug
    // futures are allocated independently of the test executor's poll stack.
    let scheduler_state = state.clone();
    tokio::spawn(async move { analytics::run_once(&scheduler_state).await })
        .await
        .unwrap()
        .unwrap();
    let report:Uuid=sqlx::query_scalar("SELECT id FROM analytics_reports WHERE brain_id=$1 AND state='queued' AND purpose='organization'").bind(brain).fetch_one(&h.admin).await.unwrap();
    let worker_state = state.clone();
    assert!(
        tokio::spawn(async move { worker::run_once(&worker_state, "heavy").await })
            .await
            .unwrap()
            .unwrap()
    );
    let status: (String, Option<String>) =
        sqlx::query_as("SELECT state,error_code FROM analytics_reports WHERE id=$1")
            .bind(report)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(status, ("ready".into(), None));
    report
}
async fn enable_retained(h: &Harness, owner: &Login, base: &str) {
    let settings = ok(
        h,
        "GET",
        &format!("{base}/models/policy"),
        owner,
        Value::Null,
    )
    .await;
    let mut policy = settings["current"]["policy"].clone();
    policy["enabled"] = json!(true);
    policy["purposes"] = json!(["embedding"]);
    policy["embedding_dimensions"] = json!(8);
    policy["content_classes"] = json!(["document", "raw_session", "support_excerpt"]);
    policy["automatic_embedding"] = json!(false);
    policy["automatic_learning"] = json!(false);
    policy["autonomous_memory"] = json!(false);
    ok(h,"PUT",&format!("{base}/models/policy"),owner,json!({"base_change":settings["current"]["change_id"],"policy":policy,"rebuild_embeddings":true})).await;
}

pub(super) async fn prepare(h: &Harness, owner: &Login, base: &str) -> (Uuid, Vec<Value>, Uuid) {
    let brain = base.rsplit('/').next().unwrap().parse().unwrap();
    enable_retained(h, owner, base).await;
    let mut docs = vec![];
    for text in [
        "Synthetic retained architecture document A.",
        "Synthetic retained architecture document B.",
        "Independent retained operations document C.",
    ] {
        docs.push(source(h, owner, base, text).await);
    }
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let profile = retained_profile(h, brain).await;
    for (i, doc) in docs.iter().enumerate() {
        let v = doc["version"]["id"].as_str().unwrap().parse().unwrap();
        let vector = if i < 2 {
            [1., 0., 0., 0., 0., 0., 0., 0.]
        } else {
            [0., 1., 0., 0., 0., 0., 0., 0.]
        };
        retained(h, brain, profile, v, &vector).await;
    }
    select(h, brain).await;
    (brain, docs, profile)
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS; zero provider calls"]
async fn organization_hybrid_retained_source_only_no_credentials_and_exact_erasure() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, docs, _) = prepare(&h, &owner, &base).await;
    let mut config = (*h.state.config).clone();
    config.models.key = None;
    config.models.openrouter_key = None;
    let state = AppState::new(h.state.pool.clone(), config).unwrap();
    let report = calculate(&h, &state, brain).await;
    let counts:(i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_mapping_inputs WHERE brain_id=$1),(SELECT count(*) FROM knowledge_mentions WHERE brain_id=$1),(SELECT count(*) FROM knowledge_topic_sources WHERE brain_id=$1),(SELECT count(*) FROM knowledge_source_inputs WHERE brain_id=$1)").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        counts,
        (0, 0, 2, 3),
        "Two supported source contributors; isolated third source remains honestly unassigned without a fabricated entity"
    );
    let page = ok(
        &h,
        "POST",
        &format!("{base}/organization/sources/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"}}),
    )
    .await;
    assert_eq!(
        page["source_memberships"].as_array().unwrap().len(),
        2,
        "{page}"
    );
    assert_eq!(page["topics"].as_array().unwrap().len(), 1, "{page}");
    assert_eq!(page["topics"][0]["source_count"], 2);
    assert_eq!(
        page["representative_sources"][0]["provenance_kind"],
        "canonical_source_title"
    );
    assert_eq!(page["evidence"].as_array().unwrap().len(), 2);
    assert!(
        page["source_memberships"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["reason"] == "hybrid_retained_vector")
    );
    let (_, foreign) = h.fixture_member().await;
    assert!(
        !h.call(
            "POST",
            &format!("{base}/organization/sources/view"),
            Some(&foreign),
            json!({"scope":{"kind":"knowledge"}})
        )
        .await
        .0
        .is_success()
    );
    // Request audit-detail expiry does not remove the retained projection or
    // make a provider credential necessary for native regrouping.
    sqlx::query("UPDATE model_requests SET detail_expired=true,model='detail_expired',returned_model=NULL,prompt_label='detail_expired',schema_label='detail_expired',dimensions=NULL WHERE brain_id=$1").bind(brain).execute(&h.admin).await.unwrap();
    let still = ok(
        &h,
        "POST",
        &format!("{base}/organization/sources/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"}}),
    )
    .await;
    assert_eq!(still["source_memberships"], page["source_memberships"]);
    sqlx::query("SELECT recollect_analytics_invalidate(ARRAY[$1::uuid])")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    let rebuilt = calculate(&h, &state, brain).await;
    assert_ne!(rebuilt, report);
    let topic: Uuid = page["topics"][0]["id"].as_str().unwrap().parse().unwrap();
    let same: Uuid = sqlx::query_scalar(
        "SELECT topic_id FROM knowledge_topic_sources WHERE brain_id=$1 LIMIT 1",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(same, topic);
    let target = json!({"kind":"source","id":docs[0]["id"]});
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
    let removed:(Option<Value>,Option<Value>,i64,i64)=sqlx::query_as("SELECT descriptor,scores,(SELECT count(*) FROM knowledge_organization_sources WHERE report_id=r.id),(SELECT count(*) FROM knowledge_organization_vectors WHERE report_id=r.id) FROM analytics_reports r WHERE id=$1").bind(rebuilt).fetch_one(&h.admin).await.unwrap();
    assert_eq!(removed, (None, None, 0, 0));
    let version: Uuid = docs[0]["version"]["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_source_inputs WHERE source_version_id=$1"
        )
        .bind(version)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM semantic_entries WHERE brain_id=$1 AND embedding IS NOT NULL"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        2,
        "Independent retained vectors survive"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS; zero provider calls"]
async fn organization_hybrid_vector_replacement_head_and_suppression_clear_aggregate() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, _, profile) = prepare(&h, &owner, &base).await;
    let report = calculate(&h, &h.state, brain).await;
    let entry: Uuid = sqlx::query_scalar(
        "SELECT entry_id FROM knowledge_organization_vectors WHERE report_id=$1 LIMIT 1",
    )
    .bind(report)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let before: i64 = sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE semantic_entries SET embedding=embedding+embedding WHERE id=$1")
        .bind(entry)
        .execute(&h.admin)
        .await
        .unwrap();
    let cleared:(String,Option<Value>,Option<Value>,i64)=sqlx::query_as("SELECT state,descriptor,scores,(SELECT count(*) FROM knowledge_organization_calculations WHERE report_id=r.id) FROM analytics_reports r WHERE id=$1").bind(report).fetch_one(&h.admin).await.unwrap();
    assert_eq!(cleared, ("removed".into(), None, None, 0));
    assert!(
        sqlx::query_scalar::<_, i64>("SELECT analytics_epoch FROM brains WHERE id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap()
            > before
    );
    let next = calculate(&h, &h.state, brain).await;
    let request: Uuid = sqlx::query_scalar(
        "SELECT request_id FROM knowledge_organization_vectors WHERE report_id=$1 LIMIT 1",
    )
    .bind(next)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    sqlx::query("UPDATE model_requests SET suppressed=true WHERE id=$1")
        .bind(request)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_organization_vectors WHERE report_id=$1"
        )
        .bind(next)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    let hidden = ok(
        &h,
        "POST",
        &format!("{base}/organization/sources/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"}}),
    )
    .await;
    assert_eq!(hidden["topics"], json!([]));
    // A replacement current semantic generation invalidates the entire old
    // weighted report, even when provider/model/dimensions remain compatible.
    sqlx::query("UPDATE model_requests SET suppressed=false WHERE id=$1")
        .bind(request)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("SELECT recollect_analytics_invalidate(ARRAY[$1::uuid])")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    let final_report = calculate(&h, &h.state, brain).await;
    let replacement = retained_profile(&h, brain).await;
    assert_ne!(replacement, profile);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM analytics_reports WHERE id=$1")
            .bind(final_report)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "removed"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; zero provider calls"]
async fn organization_hybrid_corrupt_provenance_fails_closed_and_missing_features_stay_unassigned()
{
    let (h, owner, base, p, server) = setup().await;
    let (brain, _, _) = prepare(&h, &owner, &base).await;
    sqlx::query(
        "UPDATE model_requests SET returned_model='foreign-unqualified-model' WHERE brain_id=$1",
    )
    .bind(brain)
    .execute(&h.admin)
    .await
    .unwrap();
    assert_eq!(analytics::run_once(&h.state).await.unwrap(), 0);
    let failed:(String,Option<String>,i64)=sqlx::query_as("SELECT state,vector_coverage,(SELECT count(*) FROM analytics_reports WHERE brain_id=o.brain_id) FROM knowledge_organization_outbox o WHERE brain_id=$1").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(failed, ("failed".into(), Some("corrupt".into()), 0));
    assert_eq!(
        analytics::run_once(&h.state).await.unwrap(),
        0,
        "A fixed failed revision does not spin or allocate native scratch"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_source_inputs WHERE brain_id=$1"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0,
        "No partial contributor writes on failed preparation"
    );
    // Repair exact provenance, then remove vector readiness: all sources remain
    // unassigned and no singleton/entity is invented to improve coverage.
    sqlx::query("UPDATE model_requests SET returned_model=model WHERE brain_id=$1")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE semantic_entries SET state='pending',embedding=NULL WHERE brain_id=$1")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(analytics::run_once(&h.state).await.unwrap(), 0);
    let empty:(String,Option<String>,i64)=sqlx::query_as("SELECT state,vector_coverage,(SELECT count(*) FROM knowledge_source_inputs WHERE brain_id=o.brain_id) FROM knowledge_organization_outbox o WHERE brain_id=$1").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        empty,
        ("empty".into(), Some("partial_features:3".into()), 0)
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; zero provider calls"]
async fn organization_hybrid_complete_chunk_budget_never_publishes_a_prefix() {
    let (h, owner, base, p, server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    enable_retained(&h, &owner, &base).await;
    let text = "Bounded synthetic retained source paragraph. ".repeat(1700);
    let doc = source(&h, &owner, &base, &text).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let version: Uuid = doc["version"]["id"].as_str().unwrap().parse().unwrap();
    let profile = retained_profile(&h, brain).await;
    let (_, entries) = retained(
        &h,
        brain,
        profile,
        version,
        &[1., 0., 0., 0., 0., 0., 0., 0.],
    )
    .await;
    assert!(entries.len() > 16 && entries.len() < 1000);
    select(&h, brain).await;
    assert_eq!(analytics::run_once(&h.state).await.unwrap(), 0);
    let result:(String,Option<String>,i64,i64)=sqlx::query_as("SELECT state,vector_coverage,(SELECT count(*) FROM knowledge_source_inputs WHERE brain_id=o.brain_id),(SELECT count(*) FROM analytics_reports WHERE brain_id=o.brain_id) FROM knowledge_organization_outbox o WHERE brain_id=$1").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        result,
        (
            "empty".into(),
            Some("budget_chunks_per_source".into()),
            0,
            0
        )
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}
