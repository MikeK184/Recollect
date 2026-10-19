//! A bounded Pin must pass rejected canonical prefixes without weakening the
//! exact trusted-origin gate. Fixture copies are synthetic load, not quality.
use super::*;
use recollect_server::db;

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; two actual loopback extraction calls, zero paid"]
async fn organization_legacy_pin_scans_rejected_current_origin_prefix() {
    let (h, owner, base, p, server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    allow(&h, &owner, &base, |p| {
        p["autonomous_memory"] = json!(true);
        p["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    production_mapping::synthetic_profile(&h, &base, true).await;
    let rejected = organization_weighted::mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nRejected canonical prefix.",
        false,
    )
    .await;
    let eligible = organization_weighted::mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Python uses Redis.\nEligible later contributor.",
        true,
    )
    .await;
    organization_weighted::clone_low_input_ids(&h, brain, rejected, 251).await;
    let last: Uuid = sqlx::query_scalar(
        "SELECT id FROM knowledge_mapping_inputs WHERE brain_id=$1 AND source_version_id=$2",
    )
    .bind(brain)
    .bind(eligible)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!(last > Uuid::from_u128(250));
    let ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM knowledge_mapping_inputs WHERE brain_id=$1 ORDER BY id")
            .bind(brain)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert_eq!(ids.len(), 252);
    assert_eq!(
        &ids[..250],
        &(1..=250).map(Uuid::from_u128).collect::<Vec<_>>()
    );
    let mut proposal = review::proposal(&json!(rejected), "Rust", "PostgreSQL");
    proposal["content"]["predicate"] = json!("uses");
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, proposal).await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":claim["id"],"action":"reject","reason":"Exact rejected assertion must block all byte-identical canonical prefix sources."})).await;
    let auth = organization_shadow_privacy::evaluator(&h, &owner).await;
    let mut tx = db::preparation_tx(&h.state.pool, auth.user.id, None)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let current: Vec<Uuid> = sqlx::query_scalar(
        "SELECT input_id FROM recollect_mapping_current_input_ids($1,$2) ORDER BY input_id",
    )
    .bind(brain)
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        current, ids,
        "Rejected assertions do not invalidate exact model origins; canonical qualification is a separate mandatory gate"
    );
    tx.commit().await.unwrap();
    // An explicit owned navigation load joins only genuine current mapping
    // inputs/mentions. It does not fabricate entities, model calls or origins.
    let topic = Uuid::new_v4();
    let change = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'split',$3,'human-split-1')").bind(topic).bind(brain).bind(change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) VALUES($1,$2,$3,$4,'create')").bind(change).bind(brain).bind(topic).bind(auth.user.id).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) SELECT DISTINCT brain_id,$2,entity_id,input_id,'explicit_split',1 FROM knowledge_mentions WHERE brain_id=$1").bind(brain).bind(topic).execute(&h.admin).await.unwrap();
    let mut cursor = Value::Null;
    let mut qualified = 0;
    let mut witnessed = false;
    for _ in 0..3 {
        let view=ok(&h,"POST",&format!("{base}/organization/view"),&owner,json!({"scope":{"kind":"knowledge"},"center":{"kind":"topic","id":topic},"cursor":cursor,"limit":50})).await;
        qualified += view["coverage"]["qualified_sources"].as_u64().unwrap();
        for card in view["topics"].as_array().unwrap() {
            assert_eq!(card["id"], json!(topic));
            assert_eq!(card["source_count"], 1);
            witnessed = true;
        }
        cursor = view["next_cursor"].clone();
        if cursor.is_null() {
            break;
        }
    }
    assert_eq!(
        qualified, 1,
        "Full canonical review excludes all251 rejected sources while origin-current remains true"
    );
    assert!(witnessed);
    let (status, body, _) = h
        .keyed(
            "PUT",
            &format!("{base}/organization/topics/{topic}"),
            Some(&owner),
            json!({"base_change_id":change,"change":{"action":"pin","pinned":true}}),
            Some("legacy-pin-past-rejected-prefix"),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "Canonical eligible witness after first250 rejected inputs must be reachable: {body}"
    );
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT pinned FROM knowledge_topics WHERE id=$1")
            .bind(topic)
            .fetch_one(&h.admin)
            .await
            .unwrap()
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}
