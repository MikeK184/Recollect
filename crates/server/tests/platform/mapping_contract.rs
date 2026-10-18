//! Zero-paid production contract proof; all catalogue quality rows are synthetic
//! and live only in this disposable database. No model-quality gate is inferred.
use super::*;
use recollect_server::{
    db,
    knowledge_mapping::{producer, production},
};
fn brain(base: &str) -> Uuid {
    base.rsplit('/').next().unwrap().parse().unwrap()
}
fn output() -> Value {
    json!({"claims":[],"mapping":{"language":"en","mentions":[
        {"key":"m1","kind":"service","confidence":1.0,"span":{"quote":"Orion","line_from":1,"line_to":1,"occurrence":null}},
        {"key":"m2","kind":"technology","confidence":1.0,"span":{"quote":"Redis","line_from":1,"line_to":1,"occurrence":null}},
        {"key":"m3","kind":"service","confidence":1.0,"span":{"quote":"Orion","line_from":2,"line_to":2,"occurrence":null}},
        {"key":"m4","kind":"person","confidence":1.0,"span":{"quote":"Zoë","line_from":2,"line_to":2,"occurrence":null}}],"aliases":[],"relations":[
        {"from":"m1","to":"m2","relation":"does_not_use","span":{"quote":"Orion does not use Redis.","line_from":1,"line_to":1,"occurrence":null}},
        {"from":"m3","to":"m4","relation":"has_owner","span":{"quote":"Orion.owner = Zoë","line_from":2,"line_to":2,"occurrence":null}}]}})
}
async fn catalogue(h: &Harness, b: Uuid, name: &str, prompt: &str, adapter: &str) {
    sqlx::query("INSERT INTO knowledge_mapping_release_producers(id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_receipt_sha256,fixture_sha256,recipe_sha256,held_out_documents,entity_precision,entity_recall,endpoint_precision,endpoint_recall) SELECT $1,p.policy->>'provider',p.policy->>'text_model',p.policy->>'text_model',$3,$3,$4,'source-mentions-1',repeat('a',64),repeat('b',64),repeat('c',64),18,0.96,0.85,0.96,0.85 FROM model_policy_heads h JOIN model_policies p ON p.brain_id=h.brain_id AND p.id=h.policy_id WHERE h.brain_id=$2")
        .bind(name).bind(b).bind(prompt).bind(adapter).execute(&h.admin).await.unwrap();
}
#[tokio::test]
#[ignore = "Owned PostgreSQL and controlled loopback provider; synthetic catalogue only"]
async fn mapping_contract_new_identity_bootstraps_and_preserves_negative_owner_navigation() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = output();
    let doc = source(
        &h,
        &owner,
        &base,
        "Orion does not use Redis.\nOrion.owner = Zoë\n",
    )
    .await;
    production_mapping::process(&h).await;
    let run = learn(&h, &owner, &base, &doc).await;
    model_job(&h).await;
    let rid: Uuid = run["id"].as_str().unwrap().parse().unwrap();
    let actual:(String,String,String)=sqlx::query_as("SELECT q.prompt_label,q.schema_label,s.payload->>'adapter_revision' FROM learning_runs r JOIN model_requests q ON q.id=r.request_id JOIN knowledge_mapping_stages s ON s.run_id=r.id WHERE r.id=$1")
        .bind(rid).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        actual,
        (
            gateway::EXTRACT_PROMPT.into(),
            gateway::EXTRACT_SCHEMA.into(),
            production::ADAPTER.into()
        )
    );
    catalogue(
        &h,
        brain(&base),
        "old-qualified-fixture",
        "source-facts-2",
        "learning-quote-anchors-1",
    )
    .await;
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain(&base))
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let mut tx = db::actor_tx(&h.state.pool, actor)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let selected: Option<Uuid> = sqlx::query_scalar("SELECT recollect_mapping_bootstrap($1)")
        .bind(brain(&base))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(
        selected, None,
        "Old catalogue identity cannot bootstrap the new producer"
    );
    tx.commit().await.unwrap();
    catalogue(
        &h,
        brain(&base),
        "current-qualified-fixture",
        gateway::EXTRACT_PROMPT,
        production::ADAPTER,
    )
    .await;
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        1
    );
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let relations:Vec<(String,String)>=sqlx::query_as("SELECT relation,quote FROM knowledge_relation_candidates WHERE brain_id=$1 ORDER BY relation")
        .bind(brain(&base)).fetch_all(&h.admin).await.unwrap();
    assert_eq!(
        relations,
        vec![
            ("does_not_use".into(), "Orion does not use Redis.".into()),
            ("has_owner".into(), "Orion.owner = Zoë".into())
        ]
    );
    let view = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await;
    let returned: std::collections::BTreeSet<_> = view["connections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["relation"].as_str().unwrap())
        .collect();
    assert_eq!(
        returned,
        std::collections::BTreeSet::from(["does_not_use", "has_owner"])
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Activation reuses the exact existing extraction"
    );
    server.abort();
    h.finish().await;
}
#[tokio::test]
#[ignore = "Owned PostgreSQL and controlled loopback provider; adversarial retained adapter tuple"]
async fn mapping_contract_retained_payload_cannot_borrow_another_adapter_profile() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = output();
    let doc = source(
        &h,
        &owner,
        &base,
        "Orion does not use Redis.\nOrion.owner = Zoë\n",
    )
    .await;
    production_mapping::process(&h).await;
    let run = learn(&h, &owner, &base, &doc).await;
    model_job(&h).await;
    let rid: Uuid = run["id"].as_str().unwrap().parse().unwrap();
    // Deliberately contradictory trusted-storage fixture: a genuine new request
    // paired with old adapter bytes must never become a qualified generation.
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::query("CREATE TEMP TABLE saved_stage ON COMMIT DROP AS SELECT * FROM knowledge_mapping_stages WHERE run_id=$1").bind(rid).execute(&mut *tx).await.unwrap();
    sqlx::query("DELETE FROM knowledge_mapping_stages WHERE run_id=$1")
        .bind(rid)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE saved_stage SET payload=jsonb_set(payload,'{adapter_revision}','\"learning-quote-anchors-1\"'::jsonb)").execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_stages SELECT * FROM saved_stage")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    production_mapping::synthetic_profile(&h, &base, false).await;
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        0
    );
    let inputs: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_inputs WHERE brain_id=$1")
            .bind(brain(&base))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        inputs, 0,
        "No mismatched adapter payload is promoted or reinterpreted"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Owned PostgreSQL and controlled loopback provider; exact legacy retained tuple"]
async fn mapping_contract_existing_legacy_head_promotes_only_its_original_adapter_bytes() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = production_mapping::output();
    let doc = source(&h, &owner, &base, "é Rust uses PostgreSQL.\n").await;
    production_mapping::process(&h).await;
    let run = learn(&h, &owner, &base, &doc).await;
    model_job(&h).await;
    let rid: Uuid = run["id"].as_str().unwrap().parse().unwrap();
    // Trusted-storage upgrade fixture represents an exact request/stage written
    // before063. Its original labels and adapter bytes must remain meaningful.
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::query("UPDATE model_requests SET prompt_label='source-facts-2',schema_label='source-facts-2' WHERE id=(SELECT request_id FROM learning_runs WHERE id=$1)")
        .bind(rid).execute(&mut *tx).await.unwrap();
    sqlx::query("CREATE TEMP TABLE saved_legacy_stage ON COMMIT DROP AS SELECT * FROM knowledge_mapping_stages WHERE run_id=$1")
        .bind(rid).execute(&mut *tx).await.unwrap();
    sqlx::query("DELETE FROM knowledge_mapping_stages WHERE run_id=$1")
        .bind(rid)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE saved_legacy_stage SET payload=jsonb_set(payload,'{adapter_revision}','\"learning-quote-anchors-1\"'::jsonb)")
        .execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_stages SELECT * FROM saved_legacy_stage")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let change = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_mapping_producer_profiles(change_id,brain_id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_state,quality_receipt_sha256) SELECT $1,$2,p.policy->>'provider',p.policy->>'text_model',p.policy->>'text_model','source-facts-2','source-facts-2','learning-quote-anchors-1','source-mentions-1','qualified',repeat('d',64) FROM model_policy_heads ph JOIN model_policies p ON p.brain_id=ph.brain_id AND p.id=ph.policy_id WHERE ph.brain_id=$2")
        .bind(change).bind(brain(&base)).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_producer_heads(brain_id,change_id) VALUES($1,$2)")
        .bind(brain(&base))
        .bind(change)
        .execute(&h.admin)
        .await
        .unwrap();
    catalogue(
        &h,
        brain(&base),
        "new-catalogue-does-not-replace-legacy-head",
        gateway::EXTRACT_PROMPT,
        production::ADAPTER,
    )
    .await;
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        1
    );
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let published: (Uuid, String, String, String) = sqlx::query_as("SELECT i.producer_change_id,i.adapter_revision,p.prompt_label,p.adapter_revision FROM knowledge_mapping_inputs i JOIN knowledge_mapping_producer_profiles p ON p.brain_id=i.brain_id AND p.change_id=i.producer_change_id WHERE i.brain_id=$1 AND i.state='published'")
        .bind(brain(&base)).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        published,
        (
            change,
            "learning-quote-anchors-1".into(),
            "source-facts-2".into(),
            "learning-quote-anchors-1".into()
        )
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Legacy promotion reuses retained bytes without another extraction"
    );
    server.abort();
    h.finish().await;
}
