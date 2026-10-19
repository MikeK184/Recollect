//! Actual source contributor commands and canonical paging. Synthetic vectors
//! and the explicitly marked navigation load rows never qualify a recipe.
use super::*;
use recollect_server::{db, graph::analytics, privacy_journal};
use std::collections::BTreeSet;

async fn view(h: &Harness, owner: &Login, base: &str) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/organization/view"),
        owner,
        json!({"scope":{"kind":"knowledge"},"limit":50}),
    )
    .await
}
async fn change(
    h: &Harness,
    owner: &Login,
    base: &str,
    topic: &Value,
    action: Value,
    key: &str,
) -> Value {
    let r = h
        .keyed(
            "PUT",
            &format!(
                "{base}/organization/topics/{}",
                topic["id"].as_str().unwrap()
            ),
            Some(owner),
            json!({"base_change_id":topic["change_id"],"change":action}),
            Some(key),
        )
        .await;
    assert_eq!(r.0, StatusCode::OK, "{}", r.1);
    r.1
}
fn card(page: &Value, id: &Value) -> Value {
    page["topics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == *id)
        .unwrap_or_else(|| panic!("Missing {id}: {page}"))
        .clone()
}

/// Read-only API assertions shared with the actual unmatched-conversion native
/// fixture. The caller owns its explicitly synthetic producer/vector setup.
pub(super) async fn assert_manual_overlay_card(
    h: &Harness,
    owner: &Login,
    base: &str,
    topic: Uuid,
    prior_change: Uuid,
    expected_versions: &[Uuid],
) -> Value {
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let expected: BTreeSet<_> = expected_versions
        .iter()
        .map(|version| format!("source_version:{version}"))
        .collect();
    assert!(!expected.is_empty());
    let overview = view(h, owner, base).await;
    let actual = card(&overview, &json!(topic));
    assert_eq!(actual["topic_kind"], "manual_overlay");
    assert_eq!(actual["kind"], "topic");
    assert_ne!(actual["change_id"], json!(prior_change));
    assert_eq!(actual["source_count"], expected.len());
    let stored: (Uuid, String, String, Option<Uuid>) = sqlx::query_as(
        "SELECT change_id,kind,recipe,organization_report_id FROM knowledge_topics WHERE id=$1 AND brain_id=$2",
    )
    .bind(topic)
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(json!(stored.0), actual["change_id"]);
    assert_eq!(stored.1, "manual_overlay");
    assert_eq!(stored.2, "human-overlay-1");
    assert_eq!(stored.3, None);
    let typed: recollect_protocol::OrganizationCard =
        serde_json::from_value(actual.clone()).unwrap();
    assert!(matches!(
        typed.topic_kind,
        Some(recollect_protocol::OrganizationTopicKind::ManualOverlay)
    ));

    // Decode an actual current card with the additive field omitted, exactly
    // as an older stored/client envelope would present it.
    let mut legacy = actual.clone();
    legacy.as_object_mut().unwrap().remove("topic_kind");
    let legacy: recollect_protocol::OrganizationCard = serde_json::from_value(legacy).unwrap();
    assert!(legacy.topic_kind.is_none());

    let expanded = ok(
        h,
        "POST",
        &format!("{base}/organization/view"),
        owner,
        json!({"scope":{"kind":"knowledge"},"center":{"kind":"topic","id":topic},"limit":50}),
    )
    .await;
    assert_eq!(card(&expanded, &json!(topic)), actual);
    let actual_keys = |page: &Value| -> BTreeSet<String> {
        page["source_memberships"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["topic_id"] == json!(topic))
            .map(|row| row["source_key"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(actual_keys(&expanded), expected);
    let evidence: BTreeSet<_> = expanded["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["key"].as_str().unwrap().to_owned())
        .collect();
    assert!(expected.is_subset(&evidence));

    let independent = ok(
        h,
        "POST",
        &format!("{base}/organization/sources/view"),
        owner,
        json!({"scope":{"kind":"knowledge"},"topic_id":topic}),
    )
    .await;
    assert_eq!(card(&independent, &json!(topic)), actual);
    assert_eq!(actual_keys(&independent), expected);
    assert!(independent["next_cursor"].is_null());

    // This fixture deliberately leaves the edited group unpinned and names it
    // after the automatic group, so limit=1 exercises the real context path.
    let limited = ok(
        h,
        "POST",
        &format!("{base}/organization/view"),
        owner,
        json!({"scope":{"kind":"knowledge"},"limit":1}),
    )
    .await;
    assert!(
        limited["topics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["id"] != json!(topic))
    );
    let context = limited["context_cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == json!(topic))
        .expect("The overlay must keep its stored kind outside the ranked card page");
    assert_eq!(*context, actual);
    actual
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS; zero provider calls"]
async fn organization_source_api_overrides_rebuild_and_older_brain_delete() {
    let (mut h, owner, base, p, server) = setup().await;
    let (brain, _, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    organization_hybrid::calculate(&h, &h.state, brain).await;
    let initial = view(&h, &owner, &base).await;
    assert_eq!(initial["state"], "ready");
    assert_eq!(initial["coverage"]["qualified_sources"], 3);
    assert_eq!(initial["coverage"]["mapped_sources"], 2);
    assert_eq!(initial["coverage"]["unassigned_sources"], 1);
    assert_eq!(initial["entities"], json!([]));
    assert_eq!(initial["mentions"], json!([]));
    assert_eq!(initial["source_memberships"].as_array().unwrap().len(), 2);
    assert_eq!(initial["community_status"]["vector_features"], "ready");
    let topic = initial["topics"][0].clone();
    let tid = topic["id"].clone();
    let ids: Vec<_> = initial["source_memberships"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["input_id"].clone())
        .collect();
    change(
        &h,
        &owner,
        &base,
        &topic,
        json!({"action":"pin","pinned":true}),
        "source-pin",
    )
    .await;
    let pinned = card(&view(&h, &owner, &base).await, &tid);
    assert_eq!(pinned["pinned"], true);
    let renamed = change(
        &h,
        &owner,
        &base,
        &pinned,
        json!({"action":"rename_source","label":"Reviewed architecture","basis_input":ids[0]}),
        "source-rename",
    )
    .await;
    let same = change(
        &h,
        &owner,
        &base,
        &pinned,
        json!({"action":"rename_source","label":"Reviewed architecture","basis_input":ids[0]}),
        "source-rename",
    )
    .await;
    assert_eq!(
        same, renamed,
        "Current replay is metadata-only and idempotent"
    );
    let named = card(&view(&h, &owner, &base).await, &tid);
    assert_eq!(named["label"], "Reviewed architecture");
    change(
        &h,
        &owner,
        &base,
        &named,
        json!({"action":"exclude_sources","input_ids":[ids[1]],"excluded":true}),
        "source-exclude",
    )
    .await;
    let excluded = card(&view(&h, &owner, &base).await, &tid);
    assert_eq!(excluded["source_count"], 1);
    change(
        &h,
        &owner,
        &base,
        &excluded,
        json!({"action":"exclude_sources","input_ids":[ids[1]],"excluded":false}),
        "source-unexclude",
    )
    .await;
    let restored = card(&view(&h, &owner, &base).await, &tid);
    let split = change(
        &h,
        &owner,
        &base,
        &restored,
        json!({"action":"split_sources","input_ids":[ids[1]]}),
        "source-split",
    )
    .await;
    let split_id = split["split_topic_id"].clone();
    assert!(split_id.is_string());
    assert_eq!(
        change(
            &h,
            &owner,
            &base,
            &restored,
            json!({"action":"split_sources","input_ids":[ids[1]]}),
            "source-split"
        )
        .await,
        split
    );
    let split_view = view(&h, &owner, &base).await;
    assert_eq!(card(&split_view, &tid)["source_count"], 1);
    assert_eq!(card(&split_view, &split_id)["source_count"], 1);
    let repeated_split = h
        .keyed(
            "PUT",
            &format!("{base}/organization/topics/{}", tid.as_str().unwrap()),
            Some(&owner),
            json!({"base_change_id":card(&split_view,&tid)["change_id"],
        "change":{"action":"split_sources","input_ids":[ids[1]]}}),
            Some("source-split-again"),
        )
        .await;
    assert_eq!(
        repeated_split.0,
        StatusCode::CONFLICT,
        "{}",
        repeated_split.1
    );
    sqlx::query("SELECT recollect_analytics_invalidate(ARRAY[$1::uuid])")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    organization_hybrid::calculate(&h, &h.state, brain).await;
    let rebuilt = view(&h, &owner, &base).await;
    assert_eq!(card(&rebuilt, &tid)["label"], "Reviewed architecture");
    assert_eq!(card(&rebuilt, &tid)["pinned"], true);
    assert_eq!(card(&rebuilt, &split_id)["source_count"], 1);
    change(
        &h,
        &owner,
        &base,
        &card(&rebuilt, &tid),
        json!({"action":"exclude_sources","input_ids":[ids[0]],"excluded":true}),
        "source-final-exclusion",
    )
    .await;
    let control = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Independent source deletion control"}),
    )
    .await;
    let control_brain: Uuid = control["id"].as_str().unwrap().parse().unwrap();
    let control_base = format!("/api/brains/{control_brain}");
    organization_hybrid::prepare(&h, &owner, &control_base).await;
    organization_hybrid::calculate(&h, &h.state, control_brain).await;
    let control_before = view(&h, &owner, &control_base).await;
    privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable older-source database: {backup}");
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
    // This old journal representation omits the new key before the first apply.
    sqlx::raw_sql("CREATE FUNCTION fixture_legacy_source_manifest() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.manifest:=NEW.manifest-'knowledge_source_inputs'; RETURN NEW; END $$; CREATE TRIGGER fixture_legacy_source_manifest BEFORE INSERT ON privacy_requests FOR EACH ROW EXECUTE FUNCTION fixture_legacy_source_manifest()").execute(&h.admin).await.unwrap();
    ok(&h, "PATCH", &base, &owner, json!({"archived":true})).await;
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
    privacy_journal::run_once(&h.state).await.unwrap();
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
        for table in [
            "knowledge_source_inputs",
            "knowledge_topic_sources",
            "knowledge_topic_source_labels",
            "knowledge_topic_source_exclusions",
            "knowledge_topic_source_splits",
            "knowledge_organization_source_lineage",
            "knowledge_organization_sources",
            "knowledge_organization_vectors",
            "knowledge_organization_calculations",
            "knowledge_organization_outbox",
            "knowledge_topics",
        ] {
            let count: i64 =
                sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE brain_id=$1"))
                    .bind(brain)
                    .fetch_one(pool)
                    .await
                    .unwrap();
            assert_eq!(count, 0, "Deleted Brain resurrected {table}");
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM knowledge_topic_sources WHERE brain_id=$1"
            )
            .bind(control_brain)
            .fetch_one(pool)
            .await
            .unwrap(),
            2
        );
    }
    analytics::reconcile(&h.state.config, &h.admin)
        .await
        .unwrap();
    let control_after = view(&h, &owner, &control_base).await;
    assert_eq!(control_after["topics"], control_before["topics"]);
    assert_eq!(
        control_after["source_memberships"],
        control_before["source_memberships"]
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    backup_pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; explicitly synthetic typed navigation load, zero provider calls"]
async fn organization_source_api_paging_center_scope_brain_and_expiry() {
    let (h, owner, base, p, server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let mut selected = vec![];
    let mut all = vec![];
    for n in 0..360 {
        let doc = source(
            &h,
            &owner,
            &base,
            &format!("Source contributor navigation fixture {n}.\n"),
        )
        .await;
        all.push(
            doc["version"]["id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap(),
        );
        if n < 100 {
            selected.push(
                doc["version"]["id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
            );
        }
    }
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let topic = Uuid::new_v4();
    let initial_change = Uuid::new_v4();
    // Load-only manual groups use actual current source contexts and canonical
    // materialized evidence. No NER/entity, automatic output or quality seed.
    sqlx::query("INSERT INTO knowledge_source_inputs(id,brain_id,source_id,source_version_id,context,created_by) SELECT gen_random_uuid(),$1,v.source_id,v.id,recollect_organization_source_context($1,v.id,v.source_id),b.owner_id FROM source_versions v JOIN brains b ON b.id=v.brain_id WHERE v.brain_id=$1 AND v.id=ANY($2)").bind(brain).bind(&selected).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'split',$3,'human-split-1')").bind(topic).bind(brain).bind(initial_change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT $3,$1,$2,owner_id,'create' FROM brains WHERE id=$1").bind(brain).bind(topic).bind(initial_change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT brain_id,$2,id,'explicit_source_split',1 FROM knowledge_source_inputs WHERE brain_id=$1").bind(brain).bind(topic).execute(&h.admin).await.unwrap();
    let first = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"center":{"kind":"topic","id":topic},"limit":1}),
    )
    .await;
    assert_eq!(first["state"], "partial");
    assert_eq!(first["coverage"]["mapped_sources"], 100);
    assert_eq!(first["source_memberships"].as_array().unwrap().len(), 80);
    assert_eq!(first["topics"][0]["source_count"], 100);
    let mut cursor = first["source_next_cursor"].clone();
    assert!(cursor.is_object());
    let mut seen: BTreeSet<String> = first["source_memberships"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["input_id"].as_str().unwrap().into())
        .collect();
    let other = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Foreign cursor control"}),
    )
    .await;
    let other_brain: Uuid = other["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE brains SET analytics_epoch=$2 WHERE id=$1")
        .bind(other_brain)
        .bind(first["epoch"].as_i64().unwrap())
        .execute(&h.admin)
        .await
        .unwrap();
    let wrong = h
        .call(
            "POST",
            &format!(
                "/api/brains/{}/organization/sources/view",
                other["id"].as_str().unwrap()
            ),
            Some(&owner),
            json!({"scope":{"kind":"knowledge"},"topic_id":topic,"cursor":cursor}),
        )
        .await;
    assert_eq!(wrong.0, StatusCode::CONFLICT);
    let mut expired = cursor.clone();
    expired["expires_at"] = json!(chrono::Utc::now() - chrono::Duration::seconds(1));
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/organization/sources/view"),
            Some(&owner),
            json!({"scope":{"kind":"knowledge"},"topic_id":topic,"cursor":expired})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut pages = 1;
    while cursor.is_object() {
        let page = ok(
            &h,
            "POST",
            &format!("{base}/organization/sources/view"),
            &owner,
            json!({"scope":{"kind":"knowledge"},"topic_id":topic,"cursor":cursor}),
        )
        .await;
        assert!(page["source_memberships"].as_array().unwrap().len() <= 80);
        let evidence: BTreeSet<_> = page["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["key"].as_str().unwrap())
            .collect();
        for row in page["source_memberships"].as_array().unwrap() {
            assert!(
                seen.insert(row["input_id"].as_str().unwrap().into()),
                "Repeated source detail"
            );
            assert!(evidence.contains(row["source_key"].as_str().unwrap()));
        }
        cursor = page["next_cursor"].clone();
        pages += 1;
        assert!(pages <= 3);
    }
    assert_eq!(
        seen.len(),
        100,
        "Pinned centered window must not skip rows among260unrelated sources"
    );
    assert_eq!(pages, 2, "No redundant empty terminal page");
    // A manual topic can exceed the bounded hybrid cohort. Pinning requires
    // a qualified witness window, not loading every contributor.
    let large_topic = Uuid::new_v4();
    let large_change = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_source_inputs(id,brain_id,source_id,source_version_id,context,created_by) SELECT gen_random_uuid(),$1,v.source_id,v.id,recollect_organization_source_context($1,v.id,v.source_id),b.owner_id FROM source_versions v JOIN brains b ON b.id=v.brain_id WHERE v.brain_id=$1 AND v.id=ANY($2) AND NOT EXISTS(SELECT 1 FROM knowledge_source_inputs i WHERE i.brain_id=v.brain_id AND i.source_version_id=v.id)")
        .bind(brain).bind(&all[..300]).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'split',$3,'human-split-1')").bind(large_topic).bind(brain).bind(large_change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT $3,$1,$2,owner_id,'create' FROM brains WHERE id=$1").bind(brain).bind(large_topic).bind(large_change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT brain_id,$2,id,'explicit_source_split',1 FROM knowledge_source_inputs WHERE brain_id=$1 AND source_version_id=ANY($3)")
        .bind(brain).bind(large_topic).bind(&all[..300]).execute(&h.admin).await.unwrap();
    // Immutable old contexts may sort before every current input. They are not
    // admissible witnesses and must not permanently starve this visible topic.
    let stale_ids: Vec<_> = (1..=250).map(Uuid::from_u128).collect();
    sqlx::query("INSERT INTO knowledge_source_inputs(id,brain_id,source_id,source_version_id,context,created_by) SELECT x.id,$1,v.source_id,v.id,'{}'::jsonb,b.owner_id FROM unnest($3::uuid[]) x(id) CROSS JOIN source_versions v JOIN brains b ON b.id=v.brain_id WHERE v.brain_id=$1 AND v.id=$2")
        .bind(brain).bind(all[0]).bind(&stale_ids).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT $1,$2,x.id,'explicit_source_split',1 FROM unnest($3::uuid[]) x(id)")
        .bind(brain).bind(large_topic).bind(&stale_ids).execute(&h.admin).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM recollect_organization_current_source_ids($1,$2)"
        )
        .bind(brain)
        .bind(&stale_ids)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0,
        "All250 low-ID historical contexts are stale; Pin must continue to a current witness"
    );
    change(
        &h,
        &owner,
        &base,
        &json!({"id":large_topic,"change_id":large_change}),
        json!({"action":"pin","pinned":true}),
        "large-source-pin",
    )
    .await;
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT pinned FROM knowledge_topics WHERE id=$1")
            .bind(large_topic)
            .fetch_one(&h.admin)
            .await
            .unwrap()
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and controlled loopback; synthetic mapping/manual navigation plumbing only"]
async fn organization_source_api_mixed_detail_budget_and_actual_environment_scope() {
    let (h, owner, base, p, server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = json!({"claims":[]});
    let prod = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"PROD"}),
    )
    .await;
    let dev = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"DEV"}),
    )
    .await;
    let mut prod_keys = BTreeSet::new();
    let mut dev_keys = BTreeSet::new();
    let mut inputs = vec![];
    for n in 0..100 {
        let (group, text, terms) = if n < 60 {
            (
                &prod,
                format!("ProdOnlyTech uses ProdOnlyDatabase. Contributor {n}.\n"),
                vec!["ProdOnlyTech".to_string(), "ProdOnlyDatabase".to_string()],
            )
        } else {
            (
                &dev,
                format!("DevHiddenTech uses DevHiddenDatabase. Contributor {n}.\n"),
                vec!["DevHiddenTech".to_string(), "DevHiddenDatabase".to_string()],
            )
        };
        let doc = source(&h, &owner, &base, &text).await;
        ok(
            &h,
            "PUT",
            &format!("{base}/sources/{}/groups", doc["id"].as_str().unwrap()),
            &owner,
            json!({"group_ids":[group["id"]]}),
        )
        .await;
        let key = format!("source_version:{}", doc["version"]["id"].as_str().unwrap());
        if n < 60 {
            prod_keys.insert(key);
        } else {
            dev_keys.insert(key);
        }
        if n == 0 || n == 60 {
            inputs.push((doc, text, terms));
        }
    }
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let mut profile = None;
    for (doc, text, terms) in &inputs {
        let payload = organization::mapping(doc, text, terms);
        let staged = organization::staged(&h, &owner, &base, doc, &payload).await;
        organization::synthetic_publish(&h, &owner, &base, doc, staged, &mut profile).await;
    }
    // Explicit manual navigation load uses real source contexts after actual
    // authenticated environment assignment. The selected producer and mapping
    // origins above are synthetic test plumbing, never model-quality evidence.
    sqlx::query("INSERT INTO knowledge_source_inputs(id,brain_id,source_id,source_version_id,context,created_by) SELECT gen_random_uuid(),$1,s.id,v.id,recollect_organization_source_context($1,v.id,s.id),b.owner_id FROM sources s JOIN source_versions v ON v.brain_id=s.brain_id AND v.id=s.current_version JOIN brains b ON b.id=s.brain_id WHERE s.brain_id=$1")
        .bind(brain).execute(&h.admin).await.unwrap();
    let topic = Uuid::new_v4();
    let initial_change = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'split',$3,'human-split-1')")
        .bind(topic).bind(brain).bind(initial_change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT $3,$1,$2,owner_id,'create' FROM brains WHERE id=$1")
        .bind(brain).bind(topic).bind(initial_change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT brain_id,$2,id,'explicit_source_split',1 FROM knowledge_source_inputs WHERE brain_id=$1")
        .bind(brain).bind(topic).execute(&h.admin).await.unwrap();
    let dev_basis: Uuid = sqlx::query_scalar(
        "SELECT id FROM knowledge_source_inputs WHERE brain_id=$1 AND source_version_id=$2",
    )
    .bind(brain)
    .bind(
        inputs[1].0["version"]["id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    change(
        &h,
        &owner,
        &base,
        &json!({"id":topic,"change_id":initial_change}),
        json!({"action":"rename_source","label":"DEV-only-reviewed-label","basis_input":dev_basis}),
        "mixed-source-dev-label",
    )
    .await;
    let first = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"center":{"kind":"topic","id":topic},"limit":50}),
    )
    .await;
    assert_eq!(first["mentions"].as_array().unwrap().len(), 4);
    assert_eq!(first["connections"].as_array().unwrap().len(), 2);
    assert_eq!(first["source_memberships"].as_array().unwrap().len(), 74);
    assert_eq!(
        [
            "mentions",
            "memberships",
            "connections",
            "source_memberships"
        ]
        .iter()
        .map(|k| first[*k].as_array().unwrap().len())
        .sum::<usize>(),
        80
    );
    assert_eq!(
        card(&first, &json!(topic))["label"],
        "DEV-only-reviewed-label"
    );
    let mut seen: BTreeSet<String> = first["source_memberships"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["input_id"].as_str().unwrap().into())
        .collect();
    let cursor = first["source_next_cursor"].clone();
    assert!(cursor.is_object());
    let next = ok(
        &h,
        "POST",
        &format!("{base}/organization/sources/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"topic_id":topic,"cursor":cursor}),
    )
    .await;
    assert_eq!(next["source_memberships"].as_array().unwrap().len(), 26);
    assert!(next["next_cursor"].is_null());
    for row in next["source_memberships"].as_array().unwrap() {
        assert!(seen.insert(row["input_id"].as_str().unwrap().into()));
    }
    assert_eq!(seen.len(), 100);
    for page in [&first, &next] {
        let evidence: BTreeSet<_> = page["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["key"].as_str().unwrap())
            .collect();
        for row in page["source_memberships"].as_array().unwrap() {
            assert!(evidence.contains(row["source_key"].as_str().unwrap()));
        }
    }
    let prod_scope = json!({"kind":"knowledge","selection":{"repository_ids":[],"area_ids":[],"environment_id":prod["id"]}});
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/organization/sources/view"),
            Some(&owner),
            json!({"scope":prod_scope,"topic_id":topic,"cursor":cursor})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let scoped = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":prod_scope,"center":{"kind":"topic","id":topic},"limit":50}),
    )
    .await;
    assert_eq!(scoped["coverage"]["qualified_sources"], 60);
    assert_eq!(scoped["coverage"]["mapped_sources"], 60);
    assert_eq!(scoped["coverage"]["unassigned_sources"], 0);
    assert_eq!(card(&scoped, &json!(topic))["source_count"], 60);
    assert_eq!(scoped["mentions"].as_array().unwrap().len(), 2);
    assert_eq!(scoped["connections"].as_array().unwrap().len(), 1);
    assert!(scoped["source_next_cursor"].is_null());
    assert!(!scoped.to_string().contains("DevHidden"));
    assert!(!scoped.to_string().contains("DEV-only-reviewed-label"));
    let scoped_keys: BTreeSet<String> = scoped["source_memberships"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["source_key"].as_str().unwrap().into())
        .collect();
    assert_eq!(scoped_keys, prod_keys);
    assert!(scoped_keys.is_disjoint(&dev_keys));
    assert!(
        scoped["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| prod_keys.contains(n["key"].as_str().unwrap()))
    );
    let source_scoped = ok(
        &h,
        "POST",
        &format!("{base}/organization/sources/view"),
        &owner,
        json!({"scope":prod_scope,"topic_id":topic}),
    )
    .await;
    assert_eq!(
        source_scoped["source_memberships"]
            .as_array()
            .unwrap()
            .len(),
        60
    );
    assert_eq!(source_scoped["topics"][0]["source_count"], 60);
    assert!(!source_scoped.to_string().contains("DevHidden"));
    assert!(
        !source_scoped
            .to_string()
            .contains("DEV-only-reviewed-label")
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        2,
        "Only the two controlled extraction requests"
    );
    server.abort();
    h.finish().await;
}

async fn centered_source_topic(h: &Harness, owner: &Login, base: &str, topic: Uuid) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/organization/view"),
        owner,
        json!({"scope":{"kind":"knowledge"},"center":{"kind":"topic","id":topic},"limit":50}),
    )
    .await
}
fn topic_mapping_inputs(page: &Value, topic: Uuid) -> BTreeSet<Uuid> {
    page["memberships"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["topic_id"] == json!(topic))
        .map(|m| m["input_id"].as_str().unwrap().parse().unwrap())
        .collect()
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j/GDS; two controlled extractions and synthetic retained vectors"]
async fn organization_source_api_mixed_ner_overrides_preserve_exact_topic_and_context() {
    let (h, owner, base, p, server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let settings = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    let mut policy = settings["current"]["policy"].clone();
    policy["enabled"] = json!(true);
    policy["autonomous_memory"] = json!(false);
    policy["purposes"] = json!(["extraction", "synthesis", "embedding"]);
    policy["content_classes"] = json!([
        "document",
        "claim",
        "query",
        "raw_session",
        "support_excerpt"
    ]);
    policy["embedding_dimensions"] = json!(8);
    policy["automatic_embedding"] = json!(false);
    policy["automatic_learning"] = json!(false);
    ok(&h,"PUT",&format!("{base}/models/policy"),&owner,
        json!({"base_change":settings["current"]["change_id"],"policy":policy,"rebuild_embeddings":true})).await;
    *p.candidates.lock().unwrap() = json!({"claims":[]});
    let texts = [
        "Rust uses PostgreSQL. Contributor A.\n",
        "Rust uses PostgreSQL. Contributor B.\n",
    ];
    let mut docs = vec![];
    for text in texts {
        docs.push(source(&h, &owner, &base, text).await);
    }
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let mut producer = None;
    let mut mappings = vec![];
    for (doc, text) in docs.iter().zip(texts) {
        let payload = organization::mapping(doc, text, &["Rust".into(), "PostgreSQL".into()]);
        let mapping = organization::staged(&h, &owner, &base, doc, &payload).await;
        organization::synthetic_publish(&h, &owner, &base, doc, mapping, &mut producer).await;
        mappings.push(mapping);
    }
    // The fixture-only producer/profile/finite vectors prove native API and
    // override plumbing; no selected model or grouping quality is measured.
    let profile = organization_hybrid::retained_profile(&h, brain).await;
    for doc in &docs {
        organization_hybrid::retained(
            &h,
            brain,
            profile,
            doc["version"]["id"].as_str().unwrap().parse().unwrap(),
            &[1., 0., 0., 0., 0., 0., 0., 0.],
        )
        .await;
    }
    organization_hybrid::select(&h, brain).await;
    organization_hybrid::calculate(&h, &h.state, brain).await;
    let topic:Uuid=sqlx::query_scalar("SELECT m.topic_id FROM knowledge_topic_sources m JOIN knowledge_topics t ON t.brain_id=m.brain_id AND t.id=m.topic_id WHERE m.brain_id=$1 AND t.kind='community' GROUP BY m.topic_id HAVING count(*)=2 ORDER BY m.topic_id LIMIT 1")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    let input_a:Uuid=sqlx::query_scalar("SELECT m.input_id FROM knowledge_topic_sources m JOIN knowledge_source_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE m.brain_id=$1 AND m.topic_id=$2 AND i.source_version_id=$3")
        .bind(brain).bind(topic).bind(docs[0]["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    let input_b:Uuid=sqlx::query_scalar("SELECT m.input_id FROM knowledge_topic_sources m JOIN knowledge_source_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE m.brain_id=$1 AND m.topic_id=$2 AND i.source_version_id=$3")
        .bind(brain).bind(topic).bind(docs[1]["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    let facet:Uuid=sqlx::query_scalar("SELECT DISTINCT t.id FROM knowledge_topics t JOIN knowledge_mentions m ON m.brain_id=t.brain_id AND m.entity_id=t.anchor_entity_id WHERE t.brain_id=$1 AND t.kind='facet' AND m.input_id=$2 AND m.quote='Rust'")
        .bind(brain).bind(mappings[0]).fetch_one(&h.admin).await.unwrap();
    let initial = centered_source_topic(&h, &owner, &base, topic).await;
    assert_eq!(
        topic_mapping_inputs(&initial, topic),
        BTreeSet::from([mappings[0], mappings[1]])
    );
    assert_eq!(initial["memberships"].as_array().unwrap().len(), 4);
    assert_eq!(initial["source_memberships"].as_array().unwrap().len(), 2);
    assert_eq!(card(&initial, &json!(topic))["source_count"], 2);
    assert_eq!(card(&initial, &json!(topic))["entity_count"], 2);
    // Deliberately stale context is an explicit negative storage fault, not a
    // claimed canonical history or an accepted source command. A same-version
    // exclusion cannot suppress the current mapping through a different context.
    let stale = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_source_inputs(id,brain_id,source_id,source_version_id,context,created_by) SELECT $3,brain_id,source_id,source_version_id,context||jsonb_build_object('fixture_stale_context',$4::uuid),created_by FROM knowledge_source_inputs WHERE brain_id=$1 AND id=$2")
        .bind(brain).bind(input_a).bind(stale).bind(Uuid::new_v4()).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_source_exclusions(brain_id,topic_id,input_id) VALUES($1,$2,$3)")
        .bind(brain).bind(topic).bind(stale).execute(&h.admin).await.unwrap();
    assert_eq!(
        topic_mapping_inputs(
            &centered_source_topic(&h, &owner, &base, topic).await,
            topic
        ),
        BTreeSet::from([mappings[0], mappings[1]])
    );
    change(
        &h,
        &owner,
        &base,
        &card(&initial, &json!(topic)),
        json!({"action":"rename_source","label":"A-backed reviewed group","basis_input":input_a}),
        "mixed-ner-source-label",
    )
    .await;
    let named = centered_source_topic(&h, &owner, &base, topic).await;
    assert_eq!(
        card(&named, &json!(topic))["label"],
        "A-backed reviewed group"
    );
    change(
        &h,
        &owner,
        &base,
        &card(&named, &json!(topic)),
        json!({"action":"exclude_sources","input_ids":[input_a],"excluded":true}),
        "mixed-ner-source-exclude",
    )
    .await;
    let mut excluded = centered_source_topic(&h, &owner, &base, topic).await;
    assert_eq!(
        topic_mapping_inputs(&excluded, topic),
        BTreeSet::from([mappings[1]])
    );
    assert_eq!(excluded["source_memberships"].as_array().unwrap().len(), 1);
    assert_eq!(card(&excluded, &json!(topic))["source_count"], 1);
    assert_ne!(
        card(&excluded, &json!(topic))["label"],
        "A-backed reviewed group"
    );
    assert!(
        excluded["representative_sources"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["topic_id"] == json!(topic))
            .all(|r| r["input_id"] == json!(input_b))
    );
    let hidden_basis=h.keyed("PUT",&format!("{base}/organization/topics/{topic}"),Some(&owner),
        json!({"base_change_id":card(&excluded,&json!(topic))["change_id"],"change":{"action":"rename","label":"A-backed reviewed group","basis_input":mappings[0]}}),Some("mixed-ner-source-legacy-label")).await;
    assert_eq!(hidden_basis.0, StatusCode::OK, "{}", hidden_basis.1);
    excluded = centered_source_topic(&h, &owner, &base, topic).await;
    assert_ne!(
        card(&excluded, &json!(topic))["label"],
        "A-backed reviewed group",
        "A stored legacy name cannot surface through an excluded same-topic source basis"
    );
    let independent = view(&h, &owner, &base).await;
    assert_eq!(
        card(&independent, &json!(facet))["source_count"],
        2,
        "Other facets retain the source"
    );
    change(
        &h,
        &owner,
        &base,
        &card(&excluded, &json!(topic)),
        json!({"action":"exclude_sources","input_ids":[input_a],"excluded":false}),
        "mixed-ner-source-unexclude",
    )
    .await;
    let mut restored = centered_source_topic(&h, &owner, &base, topic).await;
    assert_eq!(
        topic_mapping_inputs(&restored, topic),
        BTreeSet::from([mappings[0], mappings[1]])
    );
    assert_eq!(
        card(&restored, &json!(topic))["label"],
        "A-backed reviewed group"
    );
    // Historical origins can retain separate exclusion rows for one exact
    // immutable contribution. An accepted legacy unexclude clears all of those
    // equivalent rows, while preserving another context, topic and split.
    let historical = Uuid::new_v4();
    let other_context = Uuid::new_v4();
    let other_mapping = Uuid::new_v4();
    let other_topic = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_mapping_contexts(id,brain_id,source_id,context) SELECT $3,brain_id,source_id,context||jsonb_build_object('fixture_other_context',true) FROM knowledge_mapping_contexts WHERE brain_id=$1 AND id=(SELECT context_id FROM knowledge_mapping_inputs WHERE brain_id=$1 AND id=$2)")
        .bind(brain).bind(mappings[0]).bind(other_context).execute(&h.admin).await.unwrap();
    for (input, context) in [(historical, None), (other_mapping, Some(other_context))] {
        sqlx::query("INSERT INTO knowledge_mapping_inputs(id,brain_id,source_version_id,schema_revision,adapter_revision,preparation_epoch,context_id,payload,state,created_by) SELECT $3,brain_id,source_version_id,schema_revision,'synthetic-unexclude-history-1',preparation_epoch,coalesce($4,context_id),payload,'staged',created_by FROM knowledge_mapping_inputs WHERE brain_id=$1 AND id=$2")
            .bind(brain).bind(mappings[0]).bind(input).bind(context).execute(&h.admin).await.unwrap();
    }
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'split',$3,'human-split-1')")
        .bind(other_topic).bind(brain).bind(Uuid::new_v4()).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_exclusions(brain_id,topic_id,input_id,actor_id) SELECT $1,x.topic,x.input,b.owner_id FROM jsonb_to_recordset($2) x(topic uuid,input uuid) JOIN brains b ON b.id=$1")
        .bind(brain).bind(sqlx::types::Json(json!([
            {"topic":topic,"input":historical},
            {"topic":topic,"input":other_mapping},
            {"topic":other_topic,"input":mappings[0]}
        ]))).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_source_exclusions(brain_id,topic_id,input_id) VALUES($1,$2,$3)")
        .bind(brain).bind(topic).bind(input_a).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_source_splits(brain_id,original_topic_id,split_topic_id,input_id) VALUES($1,$2,$3,$4)")
        .bind(brain).bind(topic).bind(other_topic).bind(stale).execute(&h.admin).await.unwrap();
    change(
        &h,
        &owner,
        &base,
        &card(&restored, &json!(topic)),
        json!({"action":"exclude","input_ids":[mappings[0]],"excluded":false}),
        "mixed-ner-legacy-unexclude-equivalent-origins",
    )
    .await;
    let retained_exclusions: Vec<(Uuid,Uuid)> = sqlx::query_as(
        "SELECT topic_id,input_id FROM knowledge_topic_exclusions WHERE brain_id=$1 ORDER BY topic_id,input_id",
    ).bind(brain).fetch_all(&h.admin).await.unwrap();
    assert_eq!(
        retained_exclusions.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([(topic, other_mapping), (other_topic, mappings[0])])
    );
    let source_exclusions: Vec<Uuid> = sqlx::query_scalar("SELECT input_id FROM knowledge_topic_source_exclusions WHERE brain_id=$1 AND topic_id=$2 ORDER BY input_id")
        .bind(brain).bind(topic).fetch_all(&h.admin).await.unwrap();
    assert_eq!(source_exclusions, vec![stale]);
    let kept_splits: i64 = sqlx::query_scalar("SELECT count(*) FROM knowledge_topic_source_splits WHERE brain_id=$1 AND original_topic_id=$2 AND split_topic_id=$3 AND input_id=$4")
        .bind(brain).bind(topic).bind(other_topic).bind(stale).fetch_one(&h.admin).await.unwrap();
    assert_eq!(kept_splits, 1);
    restored = centered_source_topic(&h, &owner, &base, topic).await;
    assert_eq!(
        topic_mapping_inputs(&restored, topic),
        BTreeSet::from([mappings[0], mappings[1]])
    );
    assert_eq!(
        card(&restored, &json!(topic))["label"],
        "A-backed reviewed group"
    );
    let split = change(
        &h,
        &owner,
        &base,
        &card(&restored, &json!(topic)),
        json!({"action":"split_sources","input_ids":[input_a]}),
        "mixed-ner-source-split",
    )
    .await;
    let child: Uuid = split["split_topic_id"].as_str().unwrap().parse().unwrap();
    let original = centered_source_topic(&h, &owner, &base, topic).await;
    assert_eq!(
        topic_mapping_inputs(&original, topic),
        BTreeSet::from([mappings[1]])
    );
    assert_eq!(card(&original, &json!(topic))["source_count"], 1);
    let child_page = centered_source_topic(&h, &owner, &base, child).await;
    assert_eq!(card(&child_page, &json!(child))["source_count"], 1);
    assert_eq!(
        card(&child_page, &json!(child))["entity_count"],
        0,
        "A source split never fabricates entity membership"
    );
    assert!(topic_mapping_inputs(&child_page, child).is_empty());
    change(
        &h,
        &owner,
        &base,
        &card(&original, &json!(topic)),
        json!({"action":"exclude_sources","input_ids":[input_b],"excluded":true}),
        "mixed-ner-source-exclude-last",
    )
    .await;
    let hidden = view(&h, &owner, &base).await;
    assert!(
        !hidden["topics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == json!(topic)),
        "The original topic cannot reappear solely through its NER representation"
    );
    assert!(topic_mapping_inputs(&hidden, topic).is_empty());
    assert_eq!(card(&hidden, &json!(facet))["source_count"], 2);
    let original_sources = ok(
        &h,
        "POST",
        &format!("{base}/organization/sources/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"},"topic_id":topic}),
    )
    .await;
    assert!(original_sources["topics"].as_array().unwrap().is_empty());
    assert!(
        original_sources["source_memberships"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    sqlx::query("SELECT recollect_analytics_invalidate(ARRAY[$1::uuid])")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    organization_hybrid::calculate(&h, &h.state, brain).await;
    let rebuilt = view(&h, &owner, &base).await;
    assert!(
        !rebuilt["topics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == json!(topic))
    );
    assert!(topic_mapping_inputs(&rebuilt, topic).is_empty());
    assert_eq!(card(&rebuilt, &json!(child))["source_count"], 1);
    assert_eq!(card(&rebuilt, &json!(facet))["source_count"], 2);
    assert_eq!(
        change(
            &h,
            &owner,
            &base,
            &card(&restored, &json!(topic)),
            json!({"action":"split_sources","input_ids":[input_a]}),
            "mixed-ner-source-split"
        )
        .await,
        split,
        "Matched rebuild preserves saved split replay"
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        2,
        "Exactly two controlled extraction calls; retained vectors are fixture-only, not paid HTTP"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j/GDS; nested human splits and synthetic manual capacity load, zero provider calls"]
async fn organization_source_nested_split_capacity_is_transitive_and_atomic() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, _, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    organization_hybrid::calculate(&h, &h.state, brain).await;
    let initial = view(&h, &owner, &base).await;
    let original = initial["topics"][0].clone();
    let parent: Uuid = original["id"].as_str().unwrap().parse().unwrap();
    let inputs: Vec<Uuid> = initial["source_memberships"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["topic_id"] == original["id"])
        .map(|row| row["input_id"].as_str().unwrap().parse().unwrap())
        .collect();
    assert_eq!(inputs.len(), 2);
    let first = change(
        &h,
        &owner,
        &base,
        &original,
        json!({"action":"split_sources","input_ids":[inputs[0]]}),
        "nested-split-first",
    )
    .await;
    let child: Uuid = first["split_topic_id"].as_str().unwrap().parse().unwrap();
    let child_card = card(&view(&h, &owner, &base).await, &json!(child));
    let second = change(
        &h,
        &owner,
        &base,
        &child_card,
        json!({"action":"split_sources","input_ids":[inputs[0]]}),
        "nested-split-second",
    )
    .await;
    let grandchild: Uuid = second["split_topic_id"].as_str().unwrap().parse().unwrap();
    let (grandchild_change, kind): (Uuid, String) =
        sqlx::query_as("SELECT change_id,kind FROM knowledge_topics WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(grandchild)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(kind, "split");
    let saved_child: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM knowledge_topic_intent_sources WHERE brain_id=$1 AND topic_id=$2",
    )
    .bind(brain)
    .bind(child)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        saved_child, 1,
        "Editing a human split captures its exact contributor cohort"
    );

    // A deliberately injected historical cycle must terminate during bounded
    // preparation. It is removed before the real pending-command proof.
    sqlx::query("INSERT INTO knowledge_topic_source_splits(brain_id,original_topic_id,split_topic_id,input_id) VALUES($1,$2,$3,$4)")
        .bind(brain).bind(grandchild).bind(parent).bind(inputs[0]).execute(&h.admin).await.unwrap();
    let auth = organization_shadow_privacy::evaluator(&h, &owner).await;
    let prepared = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        recollect_server::knowledge_mapping::manual_intent::prepare_capture(
            &h.state,
            &auth,
            brain,
            grandchild,
            grandchild_change,
        ),
    )
    .await
    .expect("Cyclic split ancestry must stay bounded")
    .unwrap_or_else(|e| panic!("{}", e.1));
    assert!(prepared.is_some());
    sqlx::query("DELETE FROM knowledge_topic_source_splits WHERE brain_id=$1 AND original_topic_id=$2 AND split_topic_id=$3 AND input_id=$4")
        .bind(brain).bind(grandchild).bind(parent).bind(inputs[0]).execute(&h.admin).await.unwrap();

    // Explicit synthetic navigation load: two genuine canonical sources per
    // human group, with no extra extraction/vector/model quality claim.
    let roots:Vec<Uuid>=sqlx::query_scalar("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) SELECT gen_random_uuid(),$1,'split',gen_random_uuid(),'human-split-1' FROM generate_series(1,9990) RETURNING id")
        .bind(brain).fetch_all(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT $1,topic,input,'explicit_source_split',1 FROM unnest($2::uuid[])topic CROSS JOIN unnest($3::uuid[])input")
        .bind(brain).bind(&roots).bind(&inputs).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_intent_sources(id,brain_id,topic_id,input_id) SELECT gen_random_uuid(),$1,topic,input FROM unnest($2::uuid[])topic CROSS JOIN unnest($3::uuid[])input")
        .bind(brain).bind(&roots).bind(&inputs).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_source_labels(brain_id,topic_id,basis_input,label) SELECT $1,unnest($2::uuid[]),$3,'Synthetic retained navigation group'")
        .bind(brain).bind(&roots).bind(inputs[0]).execute(&h.admin).await.unwrap();
    const ROW_COUNT: &str = "SELECT count(*) FROM(SELECT 1 FROM knowledge_topic_sources WHERE brain_id=$1 UNION ALL SELECT 1 FROM knowledge_topic_memberships WHERE brain_id=$1 UNION ALL SELECT 1 FROM knowledge_topic_intent_sources WHERE brain_id=$1 UNION ALL SELECT 1 FROM knowledge_topic_intent_mappings WHERE brain_id=$1 UNION ALL SELECT 1 FROM knowledge_topic_labels WHERE brain_id=$1 UNION ALL SELECT 1 FROM knowledge_topic_source_labels WHERE brain_id=$1 UNION ALL SELECT 1 FROM knowledge_topic_exclusions WHERE brain_id=$1 UNION ALL SELECT 1 FROM knowledge_topic_source_exclusions WHERE brain_id=$1 UNION ALL SELECT 1 FROM knowledge_topic_splits WHERE brain_id=$1 UNION ALL SELECT 1 FROM knowledge_topic_source_splits WHERE brain_id=$1)x";
    let before_rows: i64 = sqlx::query_scalar(ROW_COUNT)
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(before_rows < 49_998);
    let fill = (49_998 - before_rows) as usize;
    assert!(fill < roots.len());
    sqlx::query("INSERT INTO knowledge_topic_source_exclusions(brain_id,topic_id,input_id) SELECT $1,unnest($2::uuid[]),$3")
        .bind(brain).bind(&roots[..fill]).bind(inputs[0]).execute(&h.admin).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(ROW_COUNT)
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        49_998
    );
    let topic_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_topics WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(topic_count < 10_000);
    let before:(i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM mutation_audit WHERE brain_id=$1),(SELECT count(*) FROM knowledge_topic_changes WHERE brain_id=$1),(SELECT count(*) FROM knowledge_topic_source_splits WHERE brain_id=$1),(SELECT count(*) FROM knowledge_topic_intent_sources WHERE brain_id=$1 AND topic_id=$2)")
        .bind(brain).bind(grandchild).fetch_one(&h.admin).await.unwrap();
    assert_eq!(before.3, 0);
    let rejected=h.keyed("PUT",&format!("{base}/organization/topics/{grandchild}"),Some(&owner),
        json!({"base_change_id":grandchild_change,"change":{"action":"split_sources","input_ids":[inputs[0]]}}),Some("nested-split-row-capacity")).await;
    assert_eq!(
        rejected.0,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        rejected.1
    );
    assert_eq!(rejected.1["code"], "organization_overlay_capacity");
    let after:(Uuid,String,i64,i64,i64,i64,i64)=sqlx::query_as("SELECT change_id,kind,(SELECT count(*) FROM mutation_audit WHERE brain_id=$1),(SELECT count(*) FROM knowledge_topic_changes WHERE brain_id=$1),(SELECT count(*) FROM knowledge_topic_source_splits WHERE brain_id=$1),(SELECT count(*) FROM knowledge_topic_intent_sources WHERE brain_id=$1 AND topic_id=$2),(SELECT count(*) FROM knowledge_topics WHERE brain_id=$1) FROM knowledge_topics WHERE brain_id=$1 AND id=$2")
        .bind(brain).bind(grandchild).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        after,
        (
            grandchild_change,
            "split".into(),
            before.0,
            before.1,
            before.2,
            0,
            topic_count
        )
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(ROW_COUNT)
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        49_998
    );
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM command_receipts WHERE key='nested-split-row-capacity'",
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        receipts, 0,
        "A rejected preflight cannot reserve a command receipt"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}
