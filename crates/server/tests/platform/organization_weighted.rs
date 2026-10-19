//! True automatic producer + native weighted Leiden publication. Profiles and
//! baseline receipts are explicitly synthetic; this proves plumbing, not quality.
use super::*;
use recollect_server::{graph::analytics, knowledge_mapping::affinity};

fn brain_id(base: &str) -> Uuid {
    base.rsplit('/').next().unwrap().parse().unwrap()
}

async fn select_recipe(h: &Harness, base: &str) -> Uuid {
    select_recipe_version(h, base, "2026.08.1").await
}
async fn select_recipe_version(h: &Harness, base: &str, version: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_organization_recipes(change_id,brain_id,recipe,gds_version,quality_state,quality_receipt_sha256) VALUES($1,$2,$3,$4,'qualified',repeat('c',64))")
        .bind(id).bind(brain_id(base)).bind(affinity::RECIPE).bind(version).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_organization_heads(brain_id,change_id) VALUES($1,$2)")
        .bind(brain_id(base))
        .bind(id)
        .execute(&h.admin)
        .await
        .unwrap();
    id
}
/// The long tests below call these helpers many times. Constructing their futures
/// inside a plain function and returning them boxed keeps each large future out
/// of the caller's debug-build stack frame (and off its stack), so the test body
/// fits the default 2 MiB test-thread stack.
pub(super) fn mapped<'a>(
    h: &'a Harness,
    owner: &'a Login,
    base: &'a str,
    p: &'a Provider,
    text: &'a str,
    other: bool,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Uuid> + 'a>> {
    Box::pin(mapped_inner(h, owner, base, p, text, other))
}
async fn mapped_inner(
    h: &Harness,
    owner: &Login,
    base: &str,
    p: &Provider,
    text: &str,
    other: bool,
) -> Uuid {
    let mut output = production_mapping::output();
    if other {
        output["mapping"]["mentions"][0]["span"]["quote"] = json!("Python");
        output["mapping"]["mentions"][1]["span"]["quote"] = json!("Redis");
        output["mapping"]["relations"][0]["span"]["quote"] = json!("é Python uses Redis.");
    }
    *p.candidates.lock().unwrap() = output;
    let source = source(h, owner, base, text).await;
    production_mapping::process(h).await;
    recollect_server::autonomous::run_once(&h.state)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    model_job(h).await;
    while worker::run_once(&h.state, "heavy").await.unwrap() {}
    source["version"]["id"].as_str().unwrap().parse().unwrap()
}
fn calculate(
    h: &Harness,
    brain: Uuid,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Uuid> + '_>> {
    Box::pin(calculate_inner(h, brain))
}
async fn calculate_inner(h: &Harness, brain: Uuid) -> Uuid {
    assert!(analytics::run_once(&h.state).await.unwrap() > 0);
    let report:Uuid=sqlx::query_scalar("SELECT id FROM analytics_reports WHERE brain_id=$1 AND purpose='organization' AND state='queued'")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let status: (String, Option<String>, Option<String>) =
        sqlx::query_as("SELECT state,error_code,gds_version FROM analytics_reports WHERE id=$1")
            .bind(report)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(status.0, "ready", "{status:?}");
    assert_eq!(status.2.as_deref(), Some("2026.08.1"));
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM analytics_attempts WHERE report_id=$1 AND cleaned_at IS NULL",
    )
    .bind(report)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(pending, 0);
    report
}
async fn groups(h: &Harness, brain: Uuid) -> Vec<(Uuid, Vec<Uuid>)> {
    sqlx::query_as("SELECT t.id,array_agg(DISTINCT i.source_version_id ORDER BY i.source_version_id) FROM knowledge_topics t JOIN knowledge_topic_memberships m ON m.topic_id=t.id AND m.brain_id=t.brain_id JOIN knowledge_mapping_inputs i ON i.id=m.input_id AND i.brain_id=m.brain_id WHERE t.brain_id=$1 AND t.kind='community' GROUP BY t.id ORDER BY t.id")
        .bind(brain).fetch_all(&h.admin).await.unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS and controlled loopback provider"]
async fn organization_weighted_native_automatic_idempotent_and_exact_contributor_erasure() {
    let (h, owner, base, p, server) = setup().await;
    let brain = brain_id(&base);
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    production_mapping::synthetic_profile(&h, &base, true).await;
    let a = mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nSession A.",
        false,
    )
    .await;
    let b = mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nSession B.",
        false,
    )
    .await;
    let c = mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Python uses Redis.\nUnrelated C.",
        true,
    )
    .await;
    let provider_calls = p.calls.load(Ordering::SeqCst);
    assert_eq!(provider_calls, 3, "One existing extraction per source");
    analytics::run_once(&h.state).await.unwrap();
    let blocked: String =
        sqlx::query_scalar("SELECT state FROM knowledge_organization_outbox WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        blocked, "blocked_recipe",
        "No default unproven recipe selection"
    );
    assert!(groups(&h, brain).await.is_empty());
    let recipe = select_recipe(&h, &base).await;
    let report = calculate(&h, brain).await;
    let original_report_deadline: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT expires_at FROM analytics_reports WHERE id=$1")
            .bind(report)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let report_deadline: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "UPDATE analytics_reports SET expires_at=clock_timestamp()+interval '1 minute' WHERE id=$1 RETURNING expires_at",
    )
    .bind(report)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let bounded_page = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":1}),
    )
    .await;
    assert_eq!(
        serde_json::from_value::<chrono::DateTime<chrono::Utc>>(bounded_page["expires_at"].clone())
            .unwrap(),
        report_deadline,
        "Every page must expire with its full eligible-window community report, even when the community card is on a later page"
    );
    sqlx::query("UPDATE analytics_reports SET expires_at=$2 WHERE id=$1")
        .bind(report)
        .bind(original_report_deadline)
        .execute(&h.admin)
        .await
        .unwrap();
    let first = groups(&h, brain).await;
    assert_eq!(first.len(), 2, "{first:?}");
    let related = first
        .iter()
        .find(|(_, versions)| versions.contains(&a))
        .unwrap();
    assert_eq!(
        related
            .1
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([a, b])
    );
    assert!(first.iter().any(|(_, versions)| versions == &vec![c]));
    let page = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await;
    let visible_topics: std::collections::BTreeSet<_> = page["topics"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    assert!(
        first
            .iter()
            .all(|(id, _)| visible_topics.contains(id.to_string().as_str())),
        "Current native communities are actually visible through qualified API: {page}"
    );
    let basis:Uuid=sqlx::query_scalar("SELECT id FROM knowledge_mapping_inputs WHERE brain_id=$1 AND source_version_id=$2 AND state='published'").bind(brain).bind(b).fetch_one(&h.admin).await.unwrap();
    let related_card = page["topics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == json!(related.0))
        .unwrap();
    let renamed=h.keyed("PUT",&format!("{base}/organization/topics/{}",related.0),Some(&owner),json!({"base_change_id":related_card["change_id"],"change":{"action":"rename","label":"Related native sources","basis_input":basis}}),Some("weighted-rename")).await;
    assert_eq!(renamed.0, StatusCode::OK, "{}", renamed.1);
    let pinned=h.keyed("PUT",&format!("{base}/organization/topics/{}",related.0),Some(&owner),json!({"base_change_id":renamed.1["change_id"],"change":{"action":"pin","pinned":true}}),Some("weighted-pin")).await;
    assert_eq!(pinned.0, StatusCode::OK, "{}", pinned.1);
    let renamed_page = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await;
    assert!(
        renamed_page["topics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == json!(related.0)
                && t["label"] == "Related native sources"
                && t["pinned"] == true),
        "Pure manual epoch changes preserve qualified community cards: {renamed_page}"
    );
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE reason='weighted_leiden'),count(*) FILTER(WHERE reason='entity_facet') FROM knowledge_topic_memberships WHERE brain_id=$1")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        counts,
        (12, 6, 6),
        "Overlapping facets coexist with native communities"
    );
    let hidden = ok(
        &h,
        "GET",
        &format!("{base}/graph/analytics"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        hidden["total"], 0,
        "Internal semantic reports are absent from topology list"
    );
    let direct = h
        .call(
            "POST",
            &format!("{base}/graph/analytics/{report}/view"),
            Some(&owner),
            json!({"offset":0}),
        )
        .await;
    assert_eq!(
        direct.0,
        StatusCode::NOT_FOUND,
        "Direct internal report IDs cannot bypass public topology gate"
    );
    assert_eq!(
        analytics::run_once(&h.state).await.unwrap(),
        0,
        "Repeated scheduler is idempotent"
    );
    // A selected immutable recipe revision queues a deterministic native rebuild.
    sqlx::query("DELETE FROM knowledge_organization_heads WHERE brain_id=$1")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    let withdrawn = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await;
    assert!(
        !withdrawn["topics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| first.iter().any(|(id, _)| t["id"] == json!(id))),
        "Withdrawn recipe/outbox revision hides every old native community: {withdrawn}"
    );
    sqlx::query("INSERT INTO knowledge_organization_heads(brain_id,change_id) VALUES($1,$2)")
        .bind(brain)
        .bind(recipe)
        .execute(&h.admin)
        .await
        .unwrap();
    let rebuilt = calculate(&h, brain).await;
    assert_ne!(rebuilt, report);
    assert_eq!(
        groups(&h, brain).await,
        first,
        "Native community numbers do not replace canonical IDs"
    );
    let after = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await;
    assert!(
        after["topics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == json!(related.0)
                && t["label"] == "Related native sources"
                && t["pinned"] == true),
        "Matched rebuild preserves explicit name and pin: {after}"
    );
    let lineage: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM knowledge_organization_lineage WHERE report_id=$1",
    )
    .bind(rebuilt)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(lineage, 2);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        provider_calls,
        "Native grouping makes no model/embedding calls"
    );
    sqlx::query(
        "UPDATE analytics_reports SET created_at=clock_timestamp()-interval '8 days' WHERE id=$1",
    )
    .bind(rebuilt)
    .execute(&h.admin)
    .await
    .unwrap();
    analytics::run_once(&h.state).await.unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM analytics_reports WHERE id=$1)")
            .bind(rebuilt)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "Canonical current provenance survives disposable topology-history cleanup"
    );
    // Suppress the actual producing request: exact mapping dependencies remove
    // physical memberships and the complete provisional calculation payload.
    let request:Uuid=sqlx::query_scalar("SELECT producer_request_id FROM knowledge_mapping_inputs WHERE brain_id=$1 AND source_version_id=$2").bind(brain).bind(a).fetch_one(&h.admin).await.unwrap();
    sqlx::query("UPDATE model_requests SET suppressed=true WHERE id=$1")
        .bind(request)
        .execute(&h.admin)
        .await
        .unwrap();
    let erased:(Option<Value>,Option<Value>,i64)=sqlx::query_as("SELECT descriptor,scores,(SELECT count(*) FROM knowledge_organization_calculations WHERE report_id=r.id) FROM analytics_reports r WHERE id=$1")
        .bind(rebuilt).fetch_one(&h.admin).await.unwrap();
    assert_eq!(erased, (None, None, 0));
    let surviving:i64=sqlx::query_scalar("SELECT count(*) FROM knowledge_mentions m JOIN knowledge_mapping_inputs i ON i.id=m.input_id WHERE i.brain_id=$1 AND i.source_version_id IN ($2,$3)")
        .bind(brain).bind(b).bind(c).fetch_one(&h.admin).await.unwrap();
    assert_eq!(surviving, 4, "Independent source mentions survive");
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS and controlled loopback provider"]
async fn organization_weighted_engine_upgrade_cannot_reuse_old_quality_receipt() {
    let (h, owner, base, p, server) = setup().await;
    let brain = brain_id(&base);
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    production_mapping::synthetic_profile(&h, &base, true).await;
    mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nEngine version control.",
        false,
    )
    .await;
    select_recipe_version(&h, &base, "unqualified-engine-version").await;
    assert!(analytics::run_once(&h.state).await.unwrap() > 0);
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let status:(String,Option<String>)=sqlx::query_as("SELECT state,error_code FROM analytics_reports WHERE brain_id=$1 AND purpose='organization'").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        status,
        (
            "failed".into(),
            Some("organization_engine_version_changed".into())
        )
    );
    assert!(groups(&h, brain).await.is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM analytics_attempts WHERE brain_id=$1 AND cleaned_at IS NULL"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0,
        "Version mismatch still closes the owned native projection"
    );
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM analytics_reports WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        analytics::run_once(&h.state).await.unwrap(),
        0,
        "A fixed failed cohort cannot enqueue an unlimited stream of fresh jobs"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM analytics_reports WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        before
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS and controlled loopback provider"]
async fn organization_weighted_manual_learning_does_not_require_autonomous_model_authority() {
    let (h, owner, base, p, server) = setup().await;
    let brain = brain_id(&base);
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(false);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    production_mapping::synthetic_profile(&h, &base, false).await;
    *p.candidates.lock().unwrap() = production_mapping::output();
    let source = source(
        &h,
        &owner,
        &base,
        "é Rust uses PostgreSQL.\nExplicit learning control.",
    )
    .await;
    production_mapping::process(&h).await;
    assert_eq!(
        recollect_server::autonomous::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        0
    );
    learn(&h, &owner, &base, &source).await;
    model_job(&h).await;
    while worker::run_once(&h.state, "heavy").await.unwrap() {}
    select_recipe(&h, &base).await;
    calculate(&h, brain).await;
    assert_eq!(groups(&h, brain).await.len(), 1);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Native organization adds no autonomous model call authority"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS and controlled loopback provider"]
async fn organization_weighted_matched_rebuild_preserves_all_versioned_overrides() {
    let (mut h, owner, base, p, server) = setup().await;
    let brain = brain_id(&base);
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    production_mapping::synthetic_profile(&h, &base, true).await;
    let a = mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nOverride A.",
        false,
    )
    .await;
    let b = mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nOverride B.",
        false,
    )
    .await;
    let c = mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nOverride C.",
        false,
    )
    .await;
    let recipe = select_recipe(&h, &base).await;
    calculate(&h, brain).await;
    let original = groups(&h, brain).await;
    assert_eq!(original.len(), 1);
    let topic = original[0].0;
    let inputs:std::collections::BTreeMap<Uuid,Uuid>=sqlx::query_as::<_,(Uuid,Uuid)>("SELECT source_version_id,id FROM knowledge_mapping_inputs WHERE brain_id=$1 AND state='published'").bind(brain).fetch_all(&h.admin).await.unwrap().into_iter().collect();
    let mut change: Uuid = sqlx::query_scalar("SELECT change_id FROM knowledge_topics WHERE id=$1")
        .bind(topic)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let actions = vec![
        json!({"action":"rename","label":"Retained manual topic","basis_input":inputs[&c]}),
        json!({"action":"pin","pinned":true}),
        json!({"action":"exclude","input_ids":[inputs[&a]],"excluded":true}),
        json!({"action":"split","input_ids":[inputs[&b]]}),
    ];
    let mut split = None;
    for (index, action) in actions.into_iter().enumerate() {
        let key = format!("weighted-override-{index}");
        let response = h
            .keyed(
                "PUT",
                &format!("{base}/organization/topics/{topic}"),
                Some(&owner),
                json!({"base_change_id":change,"change":action}),
                Some(&key),
            )
            .await;
        assert_eq!(response.0, StatusCode::OK, "{}", response.1);
        change = response.1["change_id"].as_str().unwrap().parse().unwrap();
        if !response.1["split_topic_id"].is_null() {
            split = Some(
                response.1["split_topic_id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
            );
        }
    }
    let split = split.unwrap();
    sqlx::query("DELETE FROM knowledge_organization_heads WHERE brain_id=$1")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("INSERT INTO knowledge_organization_heads(brain_id,change_id) VALUES($1,$2)")
        .bind(brain)
        .bind(recipe)
        .execute(&h.admin)
        .await
        .unwrap();
    calculate(&h, brain).await;
    assert_eq!(groups(&h, brain).await, original);
    let preserved:(String,bool,i64,i64,i64)=sqlx::query_as("SELECT l.label,t.pinned,(SELECT count(*) FROM knowledge_topic_exclusions WHERE topic_id=t.id),(SELECT count(*) FROM knowledge_topic_splits WHERE original_topic_id=t.id),(SELECT count(*) FROM knowledge_topic_memberships WHERE topic_id=$2 AND reason='explicit_split') FROM knowledge_topics t JOIN knowledge_topic_labels l ON l.topic_id=t.id WHERE t.id=$1")
        .bind(topic).bind(split).fetch_one(&h.admin).await.unwrap();
    assert_eq!(preserved, ("Retained manual topic".into(), true, 1, 1, 2));
    let view = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await;
    let main = view["topics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == json!(topic))
        .unwrap();
    let separate = view["topics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == json!(split))
        .unwrap();
    assert_eq!(main["label"], "Retained manual topic");
    assert_eq!(main["source_count"], 1);
    assert_eq!(separate["source_count"], 1);
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    // Populate an independent actual producer/native Brain before exercising
    // the public archived-Brain deletion boundary, not just dropping this DB.
    let control = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Independent weighted deletion control"}),
    )
    .await;
    let control_brain: Uuid = control["id"].as_str().unwrap().parse().unwrap();
    let control_base = format!("/api/brains/{control_brain}");
    allow(&h, &owner, &control_base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    production_mapping::synthetic_profile(&h, &control_base, true).await;
    mapped(
        &h,
        &owner,
        &control_base,
        &p,
        "é Python uses Redis.\nIndependent Brain survives.",
        true,
    )
    .await;
    select_recipe(&h, &control_base).await;
    calculate(&h, control_brain).await;
    let control_groups = groups(&h, control_brain).await;
    assert_eq!(control_groups.len(), 1);
    // An actual older database image must replay the retained journal before
    // serving. It contains populated weighted topics, lineage and every edit.
    recollect_server::privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable older-weighted database: {backup}");
    h.state.pool.close().await;
    h.admin.close().await;
    sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'")).execute(&h.root).await.unwrap();
    let mut admin_url = reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
    admin_url.set_path(&h.database);
    h.admin = recollect_server::db::pool(admin_url.as_str())
        .await
        .unwrap();
    h.state = AppState::new(
        recollect_server::db::pool(&h.state.config.database_url)
            .await
            .unwrap(),
        h.state.config.as_ref().clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    admin_url.set_path(&backup);
    let backup_admin = recollect_server::db::pool(admin_url.as_str())
        .await
        .unwrap();
    let mut backup_config = h.state.config.as_ref().clone();
    let mut backup_url = reqwest::Url::parse(&backup_config.database_url).unwrap();
    backup_url.set_path(&backup);
    backup_config.database_url = backup_url.to_string();
    backup_config.artifact_dir = format!(".cache/{backup}-artifacts");
    let backup_pool = recollect_server::db::pool(&backup_config.database_url)
        .await
        .unwrap();
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
    for table in [
        "knowledge_organization_lineage",
        "knowledge_organization_inputs",
        "knowledge_organization_calculations",
        "knowledge_organization_outbox",
        "knowledge_organization_heads",
        "knowledge_organization_recipes",
        "knowledge_topic_labels",
        "knowledge_topic_exclusions",
        "knowledge_topic_splits",
        "knowledge_topic_changes",
        "knowledge_topic_memberships",
        "knowledge_topics",
        "knowledge_mapping_origins",
        "knowledge_mapping_inputs",
    ] {
        let count: i64 =
            sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE brain_id=$1"))
                .bind(brain)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        assert_eq!(
            count, 0,
            "The actual production deletion must remove populated {table}"
        );
    }
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM brains WHERE id=$1)")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap()
    );
    recollect_server::privacy_journal::run_once(&h.state)
        .await
        .unwrap();
    assert!(
        recollect_server::privacy_journal::barrier(&backup_pool, &backup_config)
            .await
            .is_err(),
        "Older populated weighted DB must remain blocked before journal replay"
    );
    recollect_server::privacy_journal::reconcile(&backup_admin, &backup_config)
        .await
        .unwrap();
    recollect_server::privacy_journal::barrier(&backup_pool, &backup_config)
        .await
        .unwrap();
    for table in [
        "knowledge_organization_lineage",
        "knowledge_organization_inputs",
        "knowledge_organization_calculations",
        "knowledge_organization_outbox",
        "knowledge_organization_heads",
        "knowledge_organization_recipes",
        "knowledge_topic_labels",
        "knowledge_topic_exclusions",
        "knowledge_topic_splits",
        "knowledge_topic_changes",
        "knowledge_topic_memberships",
        "knowledge_topics",
        "knowledge_mapping_origins",
        "knowledge_mapping_inputs",
    ] {
        let count: i64 =
            sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE brain_id=$1"))
                .bind(brain)
                .fetch_one(&backup_admin)
                .await
                .unwrap();
        assert_eq!(
            count, 0,
            "Actual older database replay must remove populated {table}"
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_topics WHERE brain_id=$1 AND kind='community'"
        )
        .bind(control_brain)
        .fetch_one(&backup_admin)
        .await
        .unwrap(),
        1,
        "Older replay preserves independently supported Brain"
    );
    analytics::reconcile(&h.state.config, &h.admin)
        .await
        .unwrap();
    assert_eq!(
        groups(&h, control_brain).await,
        control_groups,
        "Exact Brain deletion preserves independent canonical groups"
    );
    let installation: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let native=super::super::graph::cypher(&h,"CALL gds.graph.list() YIELD graphName WHERE graphName STARTS WITH $prefix RETURN graphName",json!({"prefix":format!("recollect_analytics_{}_",installation.simple())})).await;
    assert_eq!(
        native,
        json!([]),
        "Owned journal cleanup leaves no analytical scratch allocation"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    backup_pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup} WITH (FORCE)"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}

async fn clone_fixture_inputs(h: &Harness, brain: Uuid, template: Uuid, total: usize) {
    clone_inputs_inner(h, brain, template, total, false).await;
}

/// Explicit test load only. Stable low IDs exercise cursor-prefix fairness,
/// while all copied source/request/origin rows remain exactly attributed.
pub(super) async fn clone_low_input_ids(h: &Harness, brain: Uuid, template: Uuid, total: usize) {
    clone_inputs_inner(h, brain, template, total, true).await;
}

async fn clone_inputs_inner(h: &Harness, brain: Uuid, template: Uuid, total: usize, low_ids: bool) {
    assert!((2..=5000).contains(&total));
    let extra = total - 1;
    let input_id = if low_ids {
        "lpad(to_hex(n),32,'0')::uuid"
    } else {
        "gen_random_uuid()"
    };
    let sql = format!(
        r#"
CREATE TEMP TABLE organization_load_ids ON COMMIT DROP AS
 SELECT gen_random_uuid() AS source_id,gen_random_uuid() AS version_id,gen_random_uuid() AS artifact_id,
 gen_random_uuid() AS job_id,gen_random_uuid() AS run_id,gen_random_uuid() AS request_id,
 gen_random_uuid() AS context_id,{input_id} AS input_id FROM generate_series(1,{extra}) n;
INSERT INTO sources SELECT (jsonb_populate_record(NULL::sources,to_jsonb(s)||jsonb_build_object('id',f.source_id,'current_version',NULL))).*
 FROM sources s JOIN source_versions v ON v.source_id=s.id CROSS JOIN organization_load_ids f WHERE v.id='{template}';
INSERT INTO source_versions SELECT (jsonb_populate_record(NULL::source_versions,to_jsonb(v)||jsonb_build_object('id',f.version_id,'source_id',f.source_id,'artifact_id',f.artifact_id))).*
 FROM source_versions v CROSS JOIN organization_load_ids f WHERE v.id='{template}';
UPDATE sources s SET current_version=f.version_id FROM organization_load_ids f WHERE s.id=f.source_id;
INSERT INTO source_chunks SELECT (jsonb_populate_record(NULL::source_chunks,to_jsonb(c)||jsonb_build_object('id',gen_random_uuid(),'version_id',f.version_id))).*
 FROM source_chunks c CROSS JOIN organization_load_ids f WHERE c.version_id='{template}';
INSERT INTO jobs SELECT (jsonb_populate_record(NULL::jobs,to_jsonb(j)||jsonb_build_object('id',f.job_id,'target_id',f.run_id))).*
 FROM jobs j JOIN learning_runs r ON r.job_id=j.id CROSS JOIN organization_load_ids f WHERE r.source_version_id='{template}';
INSERT INTO model_requests SELECT (jsonb_populate_record(NULL::model_requests,to_jsonb(q)||jsonb_build_object('id',f.request_id,'operation_id',f.run_id,'call_token',gen_random_uuid()))).*
 FROM model_requests q JOIN learning_runs r ON r.request_id=q.id CROSS JOIN organization_load_ids f WHERE r.source_version_id='{template}';
INSERT INTO model_request_inputs(request_id,brain_id,kind,input_id) SELECT request_id,'{brain}','source_version',version_id FROM organization_load_ids;
INSERT INTO learning_runs SELECT (jsonb_populate_record(NULL::learning_runs,to_jsonb(r)||jsonb_build_object('id',f.run_id,'source_version_id',f.version_id,'job_id',f.job_id,'request_id',f.request_id))).*
 FROM learning_runs r CROSS JOIN organization_load_ids f WHERE r.source_version_id='{template}';
INSERT INTO knowledge_mapping_contexts SELECT (jsonb_populate_record(NULL::knowledge_mapping_contexts,to_jsonb(c)||jsonb_build_object('id',f.context_id,'source_id',f.source_id))).*
 FROM knowledge_mapping_contexts c JOIN knowledge_mapping_inputs i ON i.context_id=c.id CROSS JOIN organization_load_ids f WHERE i.source_version_id='{template}';
INSERT INTO knowledge_mapping_inputs SELECT (jsonb_populate_record(NULL::knowledge_mapping_inputs,to_jsonb(i)||jsonb_build_object('id',f.input_id,'source_version_id',f.version_id,'context_id',f.context_id,'producer_request_id',f.request_id,'payload',i.payload||jsonb_build_object('source_version_id',f.version_id)))).*
 FROM knowledge_mapping_inputs i CROSS JOIN organization_load_ids f WHERE i.source_version_id='{template}';
INSERT INTO knowledge_mentions SELECT (jsonb_populate_record(NULL::knowledge_mentions,to_jsonb(m)||jsonb_build_object('id',gen_random_uuid(),'input_id',f.input_id))).*
 FROM knowledge_mentions m JOIN knowledge_mapping_inputs i ON i.id=m.input_id CROSS JOIN organization_load_ids f WHERE i.source_version_id='{template}';
INSERT INTO knowledge_mapping_origins SELECT (jsonb_populate_record(NULL::knowledge_mapping_origins,to_jsonb(o)||jsonb_build_object('input_id',f.input_id,'run_id',f.run_id,'request_id',f.request_id,'offered_inputs',jsonb_build_array(jsonb_build_object('kind','source_version','id',f.version_id))))).*
 FROM knowledge_mapping_origins o JOIN knowledge_mapping_inputs i ON i.id=o.input_id CROSS JOIN organization_load_ids f WHERE i.source_version_id='{template}';
ANALYZE sources; ANALYZE source_versions; ANALYZE source_chunks; ANALYZE jobs; ANALYZE model_requests;
ANALYZE model_request_inputs; ANALYZE learning_runs; ANALYZE knowledge_mapping_inputs;
ANALYZE knowledge_mapping_origins; ANALYZE knowledge_mentions;
"#
    );
    let mut load = h.admin.begin().await.unwrap();
    for statement in sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let mut statement = statement.to_string();
        for table in [
            "sources",
            "source_versions",
            "source_chunks",
            "jobs",
            "model_requests",
            "learning_runs",
            "knowledge_mapping_contexts",
            "knowledge_mapping_inputs",
            "knowledge_mentions",
            "knowledge_mapping_origins",
        ] {
            let prefix = format!("INSERT INTO {table} SELECT (");
            if let Some(rest) = statement.strip_prefix(&prefix) {
                let columns:Vec<String>=sqlx::query_scalar("SELECT quote_ident(attname) FROM pg_attribute WHERE attrelid=$1::regclass AND attnum>0 AND NOT attisdropped AND attgenerated='' AND attidentity='' ORDER BY attnum").bind(table).fetch_all(&h.admin).await.unwrap();
                let body = rest.replacen(").*", " AS copied", 1);
                let selection = columns
                    .iter()
                    .map(|column| format!("(copied).{column}"))
                    .collect::<Vec<_>>()
                    .join(",");
                statement = format!(
                    "INSERT INTO {table}({}) SELECT {selection} FROM (SELECT {body}) copies",
                    columns.join(",")
                );
                break;
            }
        }
        sqlx::raw_sql(&statement).execute(&mut *load).await.unwrap();
    }
    let artifacts: Vec<Uuid> = sqlx::query_scalar("SELECT artifact_id FROM organization_load_ids")
        .fetch_all(&mut *load)
        .await
        .unwrap();
    load.commit().await.unwrap();
    let template_artifact: Uuid =
        sqlx::query_scalar("SELECT artifact_id FROM source_versions WHERE id=$1")
            .bind(template)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let template_path =
        recollect_server::artifacts::path(&h.state.config.artifact_dir, brain, template_artifact);
    for artifact in artifacts {
        // Equal immutable bytes, distinct owned paths. Hard links keep this
        // filesystem fixture cheap; no installation/external artifact is used.
        tokio::fs::hard_link(
            &template_path,
            recollect_server::artifacts::path(&h.state.config.artifact_dir, brain, artifact),
        )
        .await
        .unwrap();
    }
}

async fn assert_ancestry_equivalent(h: &Harness, brain: Uuid, actor: Uuid) -> usize {
    let mut tx = recollect_server::db::preparation_tx(&h.state.pool, actor, None)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let caller_hint: Option<String> = sqlx::query_scalar(
        "SELECT nullif(current_setting('recollect.bulk_qualification',true),'')",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM knowledge_mapping_inputs WHERE brain_id=$1")
            .bind(brain)
            .fetch_all(&mut *tx)
            .await
            .unwrap();
    let actual: std::collections::BTreeSet<Uuid> =
        sqlx::query_scalar("SELECT input_id FROM recollect_mapping_current_input_ids($1,$2)")
            .bind(brain)
            .bind(&ids)
            .fetch_all(&mut *tx)
            .await
            .unwrap()
            .into_iter()
            .collect();
    let restored_hint: Option<String> = sqlx::query_scalar(
        "SELECT nullif(current_setting('recollect.bulk_qualification',true),'')",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        caller_hint, restored_hint,
        "The bulk SQL function must restore the caller's execution mode"
    );
    let expected:std::collections::BTreeSet<Uuid>=sqlx::query_scalar("SELECT i.id FROM knowledge_mapping_inputs i JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.source_version_id WHERE i.brain_id=$1 AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active' AND NOT recollect_source_model_fenced(v.brain_id,v.id)")
        .bind(brain).fetch_all(&mut *tx).await.unwrap().into_iter().collect();
    assert_eq!(
        actual, expected,
        "Complete cohort ancestry must be equivalent to049 per-source fences under actual invoker RLS"
    );
    sqlx::query("SET LOCAL recollect.bulk_qualification=on")
        .execute(&mut *tx)
        .await
        .unwrap();
    let bulk_expected:std::collections::BTreeSet<Uuid>=sqlx::query_scalar("SELECT i.id FROM knowledge_mapping_inputs i JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.source_version_id WHERE i.brain_id=$1 AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active' AND NOT recollect_source_model_fenced(v.brain_id,v.id)")
        .bind(brain).fetch_all(&mut *tx).await.unwrap().into_iter().collect();
    assert_eq!(
        expected, bulk_expected,
        "The optional bulk plan must preserve the default canonical RLS result"
    );
    tx.commit().await.unwrap();
    actual.len()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and one controlled loopback gateway call"]
async fn organization_weighted_cohort_ancestry_matches_canonical_deep_cycle_expiry_and_fences() {
    let (h, owner, base, p, server) = setup().await;
    let brain = brain_id(&base);
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    production_mapping::synthetic_profile(&h, &base, true).await;
    let template = mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nAncestry controls.",
        false,
    )
    .await;
    clone_fixture_inputs(&h, brain, template, 12).await;
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let rows: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT id,source_id FROM source_versions WHERE brain_id=$1 ORDER BY id")
            .bind(brain)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert_eq!(assert_ancestry_equivalent(&h, brain, actor).await, 12);
    for (child, parent) in [(0, 1), (1, 2), (2, 3), (5, 6), (6, 5)] {
        sqlx::query("INSERT INTO source_excerpts(brain_id,version_id,parent_source_id,parent_version_id,first_line,last_line) VALUES($1,$2,$3,$4,1,1)")
            .bind(brain).bind(rows[child].0).bind(rows[parent].1).bind(rows[parent].0).execute(&h.admin).await.unwrap();
    }
    sqlx::query("INSERT INTO source_excerpts(brain_id,version_id,parent_source_id,parent_version_id,first_line,last_line) VALUES($1,$2,$3,$4,1,1)")
        .bind(brain).bind(rows[7].0).bind(rows[4].1).bind(rows[2].0).execute(&h.admin).await.unwrap();
    assert_eq!(assert_ancestry_equivalent(&h, brain, actor).await, 9);
    // Natural expiry of an ancestor alone must not erase independent retained
    // descendants; compare with the original exact canonical predicate.
    sqlx::query("UPDATE source_versions SET privacy_state='expired' WHERE id=$1")
        .bind(rows[3].0)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(assert_ancestry_equivalent(&h, brain, actor).await, 8);
    let target = json!({"kind":"source","id":rows[2].1});
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
    assert_eq!(assert_ancestry_equivalent(&h, brain, actor).await, 5);
    let (foreign, _) = h.fixture_member().await;
    let mut denied = recollect_server::db::preparation_tx(&h.state.pool, foreign, None)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let targets: Vec<_> =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM knowledge_mapping_inputs WHERE brain_id=$1")
            .bind(brain)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert!(
        sqlx::query_scalar::<_, Uuid>(
            "SELECT input_id FROM recollect_mapping_current_input_ids($1,$2)"
        )
        .bind(brain)
        .bind(targets)
        .fetch_all(&mut *denied)
        .await
        .unwrap()
        .is_empty(),
        "Cohort materialization does not bypass inaccessible Brain RLS"
    );
    denied.commit().await.unwrap();
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS; synthetic5000-source load, one fake gateway call"]
async fn organization_weighted_5000_inputs_publish_without_a_long_brain_lock() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("recollect_server=warn,tower_http=error")
        .try_init();
    let (h, owner, base, p, server) = setup().await;
    let brain = brain_id(&base);
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    production_mapping::synthetic_profile(&h, &base, true).await;
    let template = mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nSynthetic load template.",
        false,
    )
    .await;
    // Copy the complete exact fixture provenance into distinct immutable
    // sources. This is explicit admin fixture construction, never evidence of
    // 5000 real provider calls or model quality. Every runtime gate still runs.
    clone_fixture_inputs(&h, brain, template, 5000).await;
    select_recipe(&h, &base).await;
    let started = std::time::Instant::now();
    let report = calculate(&h, brain).await;
    let complete = started.elapsed().as_secs_f64();
    let counts:(i32,i32,i64)=sqlx::query_as("SELECT node_count,edge_count,(SELECT count(*) FROM knowledge_topic_memberships WHERE brain_id=r.brain_id AND reason='weighted_leiden') FROM analytics_reports r WHERE id=$1").bind(report).fetch_one(&h.admin).await.unwrap();
    assert_eq!(counts, (5002, 10000, 10000));
    let publication_body_seconds:f64=sqlx::query_scalar("SELECT extract(epoch FROM r.published_at-min(t.created_at))::float8 FROM analytics_reports r JOIN knowledge_topics t ON t.organization_report_id=r.id WHERE r.id=$1 GROUP BY r.published_at").bind(report).fetch_one(&h.admin).await.unwrap();
    let receipt = json!({"inputs":5000,"nodes":counts.0,"edges":counts.1,"memberships":counts.2,"total_seconds":complete,"topic_insert_to_report_publish_seconds":publication_body_seconds,"gateway_calls":p.calls.load(Ordering::SeqCst),"paid_calls":0,"writer_timeout_seconds":1.5});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.cache/automatic-mapping/weighted-5000-concurrency.json");
    std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert!(
        publication_body_seconds < 1.0,
        "All10000 native memberships must publish in bounded bulk statements: {receipt}"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    // The candidate's complete source bound must preserve this full existing
    // entity branch, without allocating5000 source inputs or a first250 prefix.
    organization_hybrid::select(&h, brain).await;
    let hybrid_started = std::time::Instant::now();
    let fallback = calculate(&h, brain).await;
    let fallback_counts:(i32,i32,i64,Option<String>)=sqlx::query_as("SELECT r.node_count,r.edge_count,(SELECT count(*) FROM knowledge_source_inputs WHERE brain_id=r.brain_id),o.vector_coverage FROM analytics_reports r JOIN knowledge_organization_outbox o ON o.brain_id=r.brain_id WHERE r.id=$1").bind(fallback).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        fallback_counts,
        (5002, 10000, 0, Some("budget_sources".into()))
    );
    let receipt = json!({"inputs":5000,"nodes":fallback_counts.0,"edges":fallback_counts.1,"source_inputs":fallback_counts.2,"vector_coverage":fallback_counts.3,"complete_entity_fallback":true,"total_seconds":hybrid_started.elapsed().as_secs_f64(),"gateway_calls":p.calls.load(Ordering::SeqCst),"paid_calls":0});
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache/automatic-mapping/hybrid-5000-complete-fallback.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS and controlled loopback provider"]
async fn organization_weighted_canonical_assertion_gate_and_manual_edit_requeues_frozen_work() {
    let (h, owner, base, p, server) = setup().await;
    let brain = brain_id(&base);
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    production_mapping::synthetic_profile(&h, &base, true).await;
    let version = mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Rust uses PostgreSQL.\nCanonical assertion control.",
        false,
    )
    .await;
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&json!(version), "Rust storage", "PostgreSQL"),
    )
    .await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":claim["id"],"action":"reject","reason":"Reject exact source assertion for canonical exclusion control."})).await;
    select_recipe(&h, &base).await;
    analytics::run_once(&h.state).await.unwrap();
    let state: String =
        sqlx::query_scalar("SELECT state FROM knowledge_organization_outbox WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        state, "empty",
        "Origin-current alone must not admit a source containing a rejected assertion"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM analytics_reports WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    mapped(
        &h,
        &owner,
        &base,
        &p,
        "é Python uses Redis.\nIndependent eligible source.",
        true,
    )
    .await;
    assert!(analytics::run_once(&h.state).await.unwrap() > 0);
    let first:Uuid=sqlx::query_scalar("SELECT id FROM analytics_reports WHERE brain_id=$1 AND purpose='organization' AND state='queued'").bind(brain).fetch_one(&h.admin).await.unwrap();
    let before: i64 =
        sqlx::query_scalar("SELECT revision FROM knowledge_organization_outbox WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    // This is the same helper called by authenticated versioned topic/alias
    // commands. It coalesces the actual human edit, not a worker retry loop.
    let actor: Uuid = sqlx::query_scalar("SELECT actor_id FROM analytics_reports WHERE id=$1")
        .bind(first)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let mut tx = recollect_server::db::actor_tx(&h.state.pool, actor)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    recollect_server::db::require_writer(&mut tx, brain)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    sqlx::query("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1")
        .bind(brain)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT recollect_organization_override_changed($1)")
        .bind(brain)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let old: String = sqlx::query_scalar("SELECT state FROM analytics_reports WHERE id=$1")
        .bind(first)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(old, "stale");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT revision FROM knowledge_organization_outbox WHERE brain_id=$1"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        before + 1
    );
    let current = calculate(&h, brain).await;
    assert_ne!(current, first);
    assert_eq!(groups(&h, brain).await.len(), 1);
    assert_eq!(
        analytics::run_once(&h.state).await.unwrap(),
        0,
        "Success does not dirty or requeue itself"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}
