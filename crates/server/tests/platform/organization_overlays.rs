//! Real native publication over synthetic retained vectors. No provider or
//! production qualification is involved; each fixture owns an empty database.
use super::*;
use recollect_server::{db, graph::analytics, knowledge_mapping::manual_intent};
use std::collections::BTreeSet;

async fn change(
    h: &Harness,
    owner: &Login,
    base: &str,
    topic: Uuid,
    action: Value,
    key: &str,
) -> Value {
    let version: Uuid = sqlx::query_scalar("SELECT change_id FROM knowledge_topics WHERE id=$1")
        .bind(topic)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let r = h
        .keyed(
            "PUT",
            &format!("{base}/organization/topics/{topic}"),
            Some(owner),
            json!({"base_change_id":version,"change":action}),
            Some(key),
        )
        .await;
    assert_eq!(r.0, StatusCode::OK, "{}", r.1);
    r.1
}
async fn source_input(h: &Harness, version: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT id FROM knowledge_source_inputs WHERE source_version_id=$1")
        .bind(version)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}
async fn topic(h: &Harness, brain: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT id FROM knowledge_topics WHERE brain_id=$1 AND kind='community'")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}
async fn source_view(h: &Harness, owner: &Login, base: &str, topic: Uuid) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/organization/sources/view"),
        owner,
        json!({"scope":{"kind":"knowledge"},"topic_id":topic}),
    )
    .await
}
fn versions(docs: &[Value]) -> Vec<Uuid> {
    docs.iter()
        .map(|d| d["version"]["id"].as_str().unwrap().parse().unwrap())
        .collect()
}

// Match the production scheduler/worker task boundary, keeping their large
// debug futures off the test executor's poll stack without changing stacks.
async fn reconcile(h: &Harness) {
    let state = h.state.clone();
    tokio::spawn(async move { analytics::run_once(&state).await })
        .await
        .unwrap()
        .unwrap();
}
async fn heavy(h: &Harness) -> bool {
    let state = h.state.clone();
    tokio::spawn(async move { worker::run_once(&state, "heavy").await })
        .await
        .unwrap()
        .unwrap()
}
async fn calculate(h: &Harness, brain: Uuid) {
    reconcile(h).await;
    let report:Uuid=sqlx::query_scalar("SELECT id FROM analytics_reports WHERE brain_id=$1 AND purpose='organization' AND state='queued'").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert!(heavy(h).await);
    let status: (String, Option<String>) =
        sqlx::query_as("SELECT state,error_code FROM analytics_reports WHERE id=$1")
            .bind(report)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(status, ("ready".into(), None));
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL; bounded malformed legacy cohort, zero provider calls"]
async fn organization_overlays_rejects_oversized_raw_contexts_before_mutation() {
    let (h, owner, base, p, server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let doc = source(&h, &owner, &base, "Synthetic oversized legacy context.").await;
    production_mapping::process(&h).await;
    let source: Uuid = doc["id"].as_str().unwrap().parse().unwrap();
    let version: Uuid = doc["version"]["id"].as_str().unwrap().parse().unwrap();
    let context = Uuid::new_v4();
    let input = Uuid::new_v4();
    let topic = Uuid::new_v4();
    let change = Uuid::new_v4();
    // One 200KiB historical context repeated across 48 genuine FK rows
    // exceeds the transient envelope without allocating an OOM fixture.
    sqlx::query("INSERT INTO knowledge_mapping_contexts(id,brain_id,source_id,context) VALUES($1,$2,$3,jsonb_build_object('historical_fixture',repeat('x',204800)))")
            .bind(context).bind(brain).bind(source).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_inputs(id,brain_id,source_version_id,schema_revision,adapter_revision,preparation_epoch,context_id,payload,created_by) SELECT $1,$2,$3,'source-mentions-1','bounded-fixture-1',analytics_epoch,$4,'{}',owner_id FROM brains WHERE id=$2")
            .bind(input).bind(brain).bind(version).bind(context).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'community',$3,'entity-idf-bipartite-leiden-1')")
            .bind(topic).bind(brain).bind(change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT $1,$2,$3,owner_id,'create' FROM brains WHERE id=$2")
            .bind(change).bind(brain).bind(topic).execute(&h.admin).await.unwrap();
    let entities:Vec<Uuid>=sqlx::query_scalar("INSERT INTO knowledge_entities(id,brain_id,kind,realm,normalized_label) SELECT gen_random_uuid(),$1,'concept','{}','legacy_fixture_'||n FROM generate_series(1,48)n RETURNING id")
            .bind(brain).fetch_all(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mentions(id,brain_id,input_id,entity_id,local_key,byte_start,byte_end,quote,confidence) SELECT gen_random_uuid(),$1,$2,id,'bounded_'||n,0,9,'Synthetic',1 FROM unnest($3::uuid[]) WITH ORDINALITY x(id,n)")
            .bind(brain).bind(input).bind(&entities).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) SELECT $1,$2,unnest($3::uuid[]),$4,'weighted_leiden',1")
            .bind(brain).bind(topic).bind(entities).bind(input).execute(&h.admin).await.unwrap();
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM mutation_audit WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let auth = organization_shadow_privacy::evaluator(&h, &owner).await;
    let error = match manual_intent::prepare_capture(&h.state, &auth, brain, topic, change).await {
        Err(error) => error,
        Ok(_) => panic!("Oversized legacy contexts must fail before their canonical qualification"),
    };
    assert_eq!(error.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error.1, "organization_overlay_capacity");
    let unchanged:(Uuid,bool,i64,i64)=sqlx::query_as("SELECT change_id,pinned,(SELECT count(*) FROM knowledge_topic_intent_sources WHERE brain_id=$1),(SELECT count(*) FROM mutation_audit WHERE brain_id=$1) FROM knowledge_topics WHERE id=$2")
            .bind(brain).bind(topic).fetch_one(&h.admin).await.unwrap();
    assert_eq!(unchanged, (change, false, 0, audits));
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL; ambiguous historical split controls, zero provider calls"]
async fn organization_overlays_ambiguous_legacy_split_preserves_both_destinations() {
    let (h, owner, base, p, server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let doc = source(&h, &owner, &base, "Synthetic ambiguous legacy source.").await;
    production_mapping::process(&h).await;
    let version: Uuid = doc["version"]["id"].as_str().unwrap().parse().unwrap();
    let source: Uuid = doc["id"].as_str().unwrap().parse().unwrap();
    let context = Uuid::new_v4();
    let input1 = Uuid::new_v4();
    let input2 = Uuid::new_v4();
    let parent = Uuid::new_v4();
    let child1 = Uuid::new_v4();
    let child2 = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_mapping_contexts(id,brain_id,source_id,context) SELECT $1,$2,$3,recollect_organization_source_context($2,$4,$3)").bind(context).bind(brain).bind(source).bind(version).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_inputs(id,brain_id,source_version_id,schema_revision,adapter_revision,preparation_epoch,context_id,payload,created_by) SELECT input,$1,$2,'source-mentions-1','legacy-split-fixture-'||input::text,analytics_epoch,$3,'{}',owner_id FROM brains,unnest($4::uuid[]) input WHERE id=$1").bind(brain).bind(version).bind(context).bind(vec![input1,input2]).execute(&h.admin).await.unwrap();
    for (topic, kind, recipe) in [
        (parent, "community", "entity-idf-bipartite-leiden-1"),
        (child1, "split", "human-split-1"),
        (child2, "split", "human-split-1"),
    ] {
        let change = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe,pinned) VALUES($1,$2,$3,$4,$5,true)").bind(topic).bind(brain).bind(kind).bind(change).bind(recipe).execute(&h.admin).await.unwrap();
        sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT $1,$2,$3,owner_id,'create' FROM brains WHERE id=$2").bind(change).bind(brain).bind(topic).execute(&h.admin).await.unwrap();
    }
    for (input, child) in [(input1, child1), (input2, child2)] {
        sqlx::query("INSERT INTO knowledge_topic_splits(brain_id,original_topic_id,split_topic_id,input_id,actor_id) SELECT $1,$2,$3,$4,owner_id FROM brains WHERE id=$1").bind(brain).bind(parent).bind(child).bind(input).execute(&h.admin).await.unwrap();
    }
    let before:Value=sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(s) ORDER BY input_id) FROM knowledge_topic_splits s WHERE brain_id=$1").bind(brain).fetch_one(&h.admin).await.unwrap();
    let auth = organization_shadow_privacy::evaluator(&h, &owner).await;
    let epoch: i64 = sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let error = match manual_intent::prepare_saved(&h.state, &auth, brain, epoch).await {
        Err(error) => error,
        Ok(_) => panic!(
            "Different children on the same exact source context cannot be chosen arbitrarily"
        ),
    };
    assert_eq!(error.1, "organization_intent_conflict");
    let mut tx = db::device_tx(&h.state.pool, auth.user.id, None)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    db::require_writer(&mut tx, brain)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let error = manual_intent::copy_before_replacement(&mut tx, brain, &[input1, input2])
        .await
        .err()
        .expect("Ordinary replacement must stop before conflicting split origins cascade");
    assert_eq!(error.1, "organization_intent_conflict");
    tx.rollback().await.unwrap();
    let after:Value=sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(s) ORDER BY input_id) FROM knowledge_topic_splits s WHERE brain_id=$1").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(after, before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_topic_source_splits WHERE brain_id=$1"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_mapping_inputs WHERE brain_id=$1"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        2
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j/GDS; prospective aggregate topic capacity, zero provider calls"]
async fn organization_overlays_pending_split_capacity_has_no_partial_capture() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, docs, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    calculate(&h, brain).await;
    let original = topic(&h, brain).await;
    let a = source_input(&h, versions(&docs)[0]).await;
    let topics:Vec<Uuid>=sqlx::query_scalar("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) SELECT gen_random_uuid(),$1,'manual_overlay',gen_random_uuid(),'human-overlay-1' FROM generate_series(1,9999) RETURNING id")
            .bind(brain).fetch_all(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT $1,unnest($2::uuid[]),$3,'explicit_source_overlay',1").bind(brain).bind(&topics).bind(a).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_intent_sources(id,brain_id,topic_id,input_id) SELECT gen_random_uuid(),$1,unnest($2::uuid[]),$3").bind(brain).bind(&topics).bind(a).execute(&h.admin).await.unwrap();
    let change: Uuid = sqlx::query_scalar("SELECT change_id FROM knowledge_topics WHERE id=$1")
        .bind(original)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM mutation_audit WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let result = h
        .keyed(
            "PUT",
            &format!("{base}/organization/topics/{original}"),
            Some(&owner),
            json!({"base_change_id":change,"change":{"action":"split_sources","input_ids":[a]}}),
            Some("prospective-split-capacity"),
        )
        .await;
    assert_eq!(result.0, StatusCode::UNPROCESSABLE_ENTITY, "{}", result.1);
    assert_eq!(result.1["code"], "organization_overlay_capacity");
    let unchanged:(Uuid,i64,i64,i64,i64)=sqlx::query_as("SELECT change_id,(SELECT count(*) FROM knowledge_topics WHERE brain_id=$1),(SELECT count(*) FROM knowledge_topic_intent_sources WHERE topic_id=$2),(SELECT count(*) FROM knowledge_topic_source_splits WHERE brain_id=$1),(SELECT count(*) FROM mutation_audit WHERE brain_id=$1) FROM knowledge_topics WHERE id=$2")
            .bind(brain).bind(original).fetch_one(&h.admin).await.unwrap();
    assert_eq!(unchanged, (change, 10000, 0, 0, audits));
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j/GDS; two loopback extraction calls, zero paid"]
async fn organization_overlays_matched_entity_group_keeps_source_name_after_ner_replacement() {
    let (h, owner, base, p, server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    let producer = production_mapping::synthetic_profile(&h, &base, true).await;
    let mut versions = vec![];
    for text in [
        "é Rust uses PostgreSQL.\nMatched A.",
        "é Rust uses PostgreSQL.\nMatched B.",
    ] {
        *p.candidates.lock().unwrap() = production_mapping::output();
        let doc = source(&h, &owner, &base, text).await;
        production_mapping::process(&h).await;
        recollect_server::autonomous::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1));
        model_job(&h).await;
        while heavy(&h).await {}
        versions.push(
            doc["version"]["id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap(),
        );
    }
    let recipe = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_organization_recipes(change_id,brain_id,recipe,gds_version,quality_state,quality_receipt_sha256) VALUES($1,$2,'entity-idf-bipartite-leiden-1','2026.08.1','qualified',repeat('c',64))")
        .bind(recipe).bind(brain).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_organization_heads(brain_id,change_id) VALUES($1,$2)")
        .bind(brain)
        .bind(recipe)
        .execute(&h.admin)
        .await
        .unwrap();
    calculate(&h, brain).await;
    let original = topic(&h, brain).await;
    let basis:Uuid=sqlx::query_scalar("SELECT input_id FROM knowledge_topic_memberships WHERE topic_id=$1 ORDER BY input_id LIMIT 1").bind(original).fetch_one(&h.admin).await.unwrap();
    change(
        &h,
        &owner,
        &base,
        original,
        json!({"action":"rename","basis_input":basis,"label":"Human adopted entity sources"}),
        "entity-overlay-name",
    )
    .await;
    sqlx::query("SELECT recollect_analytics_invalidate(ARRAY[$1::uuid])")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    calculate(&h, brain).await;
    assert_eq!(
        topic(&h, brain).await,
        original,
        "Unique matching refreshes the existing human cohort"
    );
    let current = source_view(&h, &owner, &base, original).await;
    assert_eq!(current["topics"][0]["topic_kind"], "community");
    assert_eq!(
        current["topics"][0]["label"],
        "Human adopted entity sources"
    );
    let saved:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_topic_intent_sources WHERE topic_id=$1),(SELECT count(*) FROM knowledge_topic_intent_mappings WHERE topic_id=$1)").bind(original).fetch_one(&h.admin).await.unwrap();
    assert_eq!(saved, (2, 4));
    let replacement = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_mapping_producer_profiles(change_id,brain_id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_state,quality_receipt_sha256) SELECT $2,brain_id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_state,quality_receipt_sha256 FROM knowledge_mapping_producer_profiles WHERE change_id=$1")
        .bind(producer).bind(replacement).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE knowledge_mapping_producer_heads SET change_id=$2 WHERE brain_id=$1")
        .bind(brain)
        .bind(replacement)
        .execute(&h.admin)
        .await
        .unwrap();
    // Physical origin removal models the final FK cascade of ordinary NER
    // replacement, after the human command has atomically saved its cohort.
    sqlx::query("DELETE FROM knowledge_mapping_inputs WHERE brain_id=$1")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    reconcile(&h).await;
    let overlay = source_view(&h, &owner, &base, original).await;
    assert_eq!(overlay["topics"][0]["topic_kind"], "manual_overlay");
    assert_eq!(
        overlay["topics"][0]["label"],
        "Human adopted entity sources"
    );
    assert_eq!(overlay["source_memberships"].as_array().unwrap().len(), 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_topic_intent_mappings WHERE brain_id=$1"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Requires owned PostgreSQL/Neo4j/GDS; synthetic retained vectors, zero provider calls"]
async fn organization_overlays_source_snapshot_survives_vector_replacement_and_no_recipe() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, docs, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    let versions = versions(&docs);
    calculate(&h, brain).await;
    let original = topic(&h, brain).await;
    let a = source_input(&h, versions[0]).await;
    change(
        &h,
        &owner,
        &base,
        original,
        json!({"action":"rename_source","basis_input":a,"label":"Z retained human group"}),
        "overlay-source-name",
    )
    .await;
    let before: Uuid = sqlx::query_scalar("SELECT change_id FROM knowledge_topics WHERE id=$1")
        .bind(original)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let saved:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_topic_intent_sources WHERE topic_id=$1),(SELECT count(*) FROM knowledge_topic_intent_mappings WHERE brain_id=$2),(SELECT count(*) FROM knowledge_mapping_inputs WHERE brain_id=$2)")
        .bind(original).bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        saved,
        (2, 0, 0),
        "Entity membership cannot mask missing source-only intent"
    );
    // Entry replacement invalidates the report and clears its automatic rows
    // BEFORE the publisher can inspect them. The adopted A/B cohort survives.
    sqlx::query("UPDATE semantic_entries SET embedding=CASE WHEN source_version_id=$2 THEN '[0,1,0,0,0,0,0,0]'::vector ELSE '[1,0,0,0,0,0,0,0]'::vector END WHERE brain_id=$1 AND source_version_id=ANY($3)")
        .bind(brain).bind(versions[1]).bind(vec![versions[1],versions[2]]).execute(&h.admin).await.unwrap();
    let after:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_topic_sources WHERE topic_id=$1),(SELECT count(*) FROM knowledge_topic_intent_sources WHERE topic_id=$1)").bind(original).fetch_one(&h.admin).await.unwrap();
    assert_eq!(after, (0, 2));
    calculate(&h, brain).await;
    let automatic = topic(&h, brain).await;
    assert_ne!(automatic, original);
    let automatic_versions:Vec<Uuid>=sqlx::query_scalar("SELECT i.source_version_id FROM knowledge_topic_sources m JOIN knowledge_source_inputs i ON i.id=m.input_id WHERE m.topic_id=$1 ORDER BY i.source_version_id")
        .bind(automatic).fetch_all(&h.admin).await.unwrap();
    assert_eq!(
        automatic_versions.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([versions[0], versions[2]])
    );
    organization_source_api::assert_manual_overlay_card(
        &h,
        &owner,
        &base,
        original,
        before,
        &versions[..2],
    )
    .await;
    organization_browser::assert_manual_overlay_browser(
        &h,
        &owner,
        &base,
        original,
        "Z retained human group",
        &versions[..2],
    )
    .await;
    let reasons: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT reason FROM knowledge_topic_sources WHERE topic_id=$1")
            .bind(original)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert_eq!(reasons, vec!["explicit_source_overlay"]);
    // A later automatic A/B result must never match a persisted overlay back
    // into an automatic group, even with identical source contributors.
    sqlx::query("UPDATE semantic_entries SET embedding=CASE WHEN source_version_id=$2 THEN '[0,1,0,0,0,0,0,0]'::vector ELSE '[1,0,0,0,0,0,0,0]'::vector END WHERE brain_id=$1")
        .bind(brain).bind(versions[2]).execute(&h.admin).await.unwrap();
    calculate(&h, brain).await;
    assert_ne!(topic(&h, brain).await, original);
    let current = source_view(&h, &owner, &base, original).await;
    assert_eq!(current["topics"][0]["topic_kind"], "manual_overlay");
    assert_eq!(current["topics"][0]["label"], "Z retained human group");
    // Exact controls remain editable on an overlay. Its excluded naming basis
    // is hidden and last accepted unexclude restores that same saved name.
    change(
        &h,
        &owner,
        &base,
        original,
        json!({"action":"exclude_sources","input_ids":[a],"excluded":true}),
        "overlay-exclude-basis",
    )
    .await;
    let hidden = source_view(&h, &owner, &base, original).await;
    assert_eq!(hidden["source_memberships"].as_array().unwrap().len(), 1);
    assert_ne!(hidden["topics"][0]["label"], "Z retained human group");
    change(
        &h,
        &owner,
        &base,
        original,
        json!({"action":"exclude_sources","input_ids":[a],"excluded":false}),
        "overlay-unexclude-basis",
    )
    .await;
    assert_eq!(
        source_view(&h, &owner, &base, original).await["topics"][0]["label"],
        "Z retained human group"
    );
    // A disabled model policy and absent recipe still reconcile human intent.
    // No retained read is turned into a provider call.
    allow(&h, &owner, &base, |policy| policy["enabled"] = json!(false)).await;
    sqlx::query("DELETE FROM knowledge_organization_heads WHERE brain_id=$1")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("SELECT recollect_analytics_invalidate(ARRAY[$1::uuid])")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    reconcile(&h).await;
    let state: String =
        sqlx::query_scalar("SELECT state FROM knowledge_organization_outbox WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(state, "blocked_recipe");
    let human = source_view(&h, &owner, &base, original).await;
    assert_eq!(human["topics"][0]["label"], "Z retained human group");
    // Correction removes only the old exact basis. It cannot transfer its
    // adopted name or membership onto the replacement source version.
    let corrected=ok(&h,"POST",&format!("{base}/sources/{}/versions",docs[0]["id"].as_str().unwrap()),&owner,
        json!({"base_version":versions[0],"title":"Corrected A","media_type":"text/plain","content":"Replacement immutable source A.","retain_content":true})).await;
    assert_ne!(corrected["version"]["id"], json!(versions[0]));
    production_mapping::process(&h).await;
    reconcile(&h).await;
    let surviving = source_view(&h, &owner, &base, original).await;
    assert_eq!(surviving["source_memberships"].as_array().unwrap().len(), 1);
    assert_ne!(surviving["topics"][0]["label"], "Z retained human group");
    assert_eq!(
        surviving["source_memberships"][0]["source_key"],
        format!("source_version:{}", versions[1])
    );
    // A context change of the final contributor prunes its intent and name.
    let area = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Changed source context"}),
    )
    .await;
    ok(
        &h,
        "PUT",
        &format!("{base}/sources/{}/groups", docs[1]["id"].as_str().unwrap()),
        &owner,
        json!({"group_ids":[area["id"]]}),
    )
    .await;
    sqlx::query("SELECT recollect_analytics_invalidate(ARRAY[$1::uuid])")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    reconcile(&h).await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM knowledge_topics WHERE id=$1")
            .bind(original)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}
