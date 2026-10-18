//! Human-intent storage/privacy proof, using actual source-only native publication
//! and human API edits. Explicit staged NER rows below exercise FK deletion only;
//! they are never producer qualification or semantic quality evidence.
use super::*;
use recollect_server::{db, graph::analytics, privacy_journal};

async fn edited(h: &Harness, owner: &Login, base: &str) -> (Uuid, Vec<Value>, Uuid) {
    let (brain, docs, _) = Box::pin(organization_hybrid::prepare(h, owner, base)).await;
    // Match production's owned scheduler/worker task boundaries without
    // increasing the default test-thread stack for their large debug futures.
    let state = h.state.clone();
    tokio::spawn(async move { analytics::run_once(&state).await })
        .await
        .unwrap()
        .unwrap();
    let report: Uuid = sqlx::query_scalar("SELECT id FROM analytics_reports WHERE brain_id=$1 AND purpose='organization' AND state='queued'")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    let state = h.state.clone();
    assert!(
        tokio::spawn(async move { worker::run_once(&state, "heavy").await })
            .await
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM analytics_reports WHERE id=$1")
            .bind(report)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "ready"
    );
    let page = ok(
        h,
        "POST",
        &format!("{base}/organization/view"),
        owner,
        json!({"scope":{"kind":"knowledge"}}),
    )
    .await;
    let topic = &page["topics"][0];
    let id: Uuid = topic["id"].as_str().unwrap().parse().unwrap();
    let basis = &page["source_memberships"][0]["input_id"];
    let changed = h
        .keyed(
            "PUT",
            &format!("{base}/organization/topics/{id}"),
            Some(owner),
            json!({"base_change_id":topic["change_id"],"change":{"action":"rename_source",
        "label":"Human-adopted architecture","basis_input":basis}}),
            Some(&format!("intent-privacy-name-{id}")),
        )
        .await;
    assert_eq!(changed.0, StatusCode::OK, "{}", changed.1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_topic_intent_sources WHERE brain_id=$1 AND topic_id=$2"
        )
        .bind(brain)
        .bind(id)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        2
    );
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
    (brain, docs, id)
}

// Deliberately staged, non-production fixture metadata. No provider, origin,
// visible entity or automatic grouping is fabricated by this helper.
async fn staged(h: &Harness, brain: Uuid, doc: &Value, topic: Option<Uuid>) -> Uuid {
    let input = Uuid::new_v4();
    let context = Uuid::new_v4();
    let source: Uuid = doc["id"].as_str().unwrap().parse().unwrap();
    let version: Uuid = doc["version"]["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_contexts(id,brain_id,source_id,context) SELECT $1,$2,$3,context FROM knowledge_source_inputs WHERE brain_id=$2 AND source_version_id=$4")
        .bind(context).bind(brain).bind(source).bind(version).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_inputs(id,brain_id,source_version_id,schema_revision,adapter_revision,preparation_epoch,context_id,payload,created_by) SELECT $1,$2,$3,'source-mentions-1','privacy-fixture-1',analytics_epoch,$4,'{}',owner_id FROM brains WHERE id=$2")
        .bind(input).bind(brain).bind(version).bind(context).execute(&h.admin).await.unwrap();
    if let Some(topic) = topic {
        let entity = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_entities(id,brain_id,kind,realm,normalized_label) VALUES($1,$2,'concept','{}','privacy-fixture')")
            .bind(entity).bind(brain).execute(&h.admin).await.unwrap();
        sqlx::query("INSERT INTO knowledge_mentions(id,brain_id,input_id,entity_id,local_key,byte_start,byte_end,quote,confidence) VALUES($1,$2,$3,$4,'privacy-fixture',0,9,'Synthetic',1)")
            .bind(Uuid::new_v4()).bind(brain).bind(input).bind(entity).execute(&h.admin).await.unwrap();
        sqlx::query("INSERT INTO knowledge_topic_intent_mappings(id,brain_id,topic_id,input_id,entity_id) VALUES($1,$2,$3,$4,$5)")
            .bind(Uuid::new_v4()).bind(brain).bind(topic).bind(input).bind(entity).execute(&h.admin).await.unwrap();
    }
    input
}

async fn snapshot(pool: &sqlx::PgPool, brain: Uuid) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('topics',(SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM knowledge_topics t WHERE brain_id=$1),'sources',(SELECT jsonb_agg(to_jsonb(s) ORDER BY s.input_id) FROM knowledge_topic_sources s WHERE brain_id=$1),'intent_sources',(SELECT jsonb_agg(to_jsonb(i) ORDER BY i.id) FROM knowledge_topic_intent_sources i WHERE brain_id=$1),'intent_mappings',(SELECT jsonb_agg(to_jsonb(i) ORDER BY i.id) FROM knowledge_topic_intent_mappings i WHERE brain_id=$1),'labels',(SELECT jsonb_agg(to_jsonb(l) ORDER BY l.topic_id) FROM knowledge_topic_source_labels l WHERE brain_id=$1))")
        .bind(brain).fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL; populated 061-to-062 upgrade, zero provider calls"]
async fn organization_intent_privacy_upgrade_bridges_legacy_name_before_origin_removal() {
    let migration = db::MIGRATIONS
        .iter()
        .position(|(name, _)| name.starts_with("062_"))
        .unwrap();
    let h = Harness::new_through(migration).await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Legacy human intent upgrade"}),
    )
    .await;
    let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain}");
    let doc = source(&h, &owner, &base, "Synthetic legacy source.").await;
    production_mapping::process(&h).await;
    let version: Uuid = doc["version"]["id"].as_str().unwrap().parse().unwrap();
    let source: Uuid = doc["id"].as_str().unwrap().parse().unwrap();
    let context = Uuid::new_v4();
    let input = Uuid::new_v4();
    let entity = Uuid::new_v4();
    let topic = Uuid::new_v4();
    let change = Uuid::new_v4();
    // Legacy entity-topic storage permits 256 bytes, beyond source-rename's bound.
    let label = "é".repeat(128);
    sqlx::query("INSERT INTO knowledge_mapping_contexts(id,brain_id,source_id,context) SELECT $1,$2,$3,recollect_organization_source_context($2,$4,$3)")
        .bind(context).bind(brain).bind(source).bind(version).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_inputs(id,brain_id,source_version_id,schema_revision,adapter_revision,preparation_epoch,context_id,payload,created_by) SELECT $1,$2,$3,'source-mentions-1','legacy-privacy-fixture-1',analytics_epoch,$4,'{}',owner_id FROM brains WHERE id=$2")
        .bind(input).bind(brain).bind(version).bind(context).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_entities(id,brain_id,kind,realm,normalized_label) VALUES($1,$2,'concept','{}','synthetic')")
        .bind(entity).bind(brain).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mentions(id,brain_id,input_id,entity_id,local_key,byte_start,byte_end,quote,confidence) VALUES($1,$2,$3,$4,'synthetic',0,9,'Synthetic',1)")
        .bind(Uuid::new_v4()).bind(brain).bind(input).bind(entity).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'community',$3,'entity-idf-bipartite-leiden-1')")
        .bind(topic).bind(brain).bind(change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT $1,$2,$3,owner_id,'rename' FROM brains WHERE id=$2")
        .bind(change).bind(brain).bind(topic).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) VALUES($1,$2,$3,$4,'weighted_leiden',1)")
        .bind(brain).bind(topic).bind(entity).bind(input).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_labels(brain_id,topic_id,basis_input,label,actor_id) SELECT $1,$2,$3,$4,owner_id FROM brains WHERE id=$1")
        .bind(brain).bind(topic).bind(input).bind(&label).execute(&h.admin).await.unwrap();
    let excluded_topic = Uuid::new_v4();
    let excluded_change = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'community',$3,'entity-idf-bipartite-leiden-1')")
        .bind(excluded_topic).bind(brain).bind(excluded_change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT $1,$2,$3,owner_id,'rename' FROM brains WHERE id=$2")
        .bind(excluded_change).bind(brain).bind(excluded_topic).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) VALUES($1,$2,$3,$4,'weighted_leiden',1)")
        .bind(brain).bind(excluded_topic).bind(entity).bind(input).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_labels(brain_id,topic_id,basis_input,label,actor_id) SELECT $1,$2,$3,'Excluded legacy name',owner_id FROM brains WHERE id=$1")
        .bind(brain).bind(excluded_topic).bind(input).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_exclusions(brain_id,topic_id,input_id,actor_id) SELECT $1,$2,$3,owner_id FROM brains WHERE id=$1")
        .bind(brain).bind(excluded_topic).bind(input).execute(&h.admin).await.unwrap();
    let split_parent = Uuid::new_v4();
    let split_child = Uuid::new_v4();
    for (id, kind, recipe) in [
        (split_parent, "community", "entity-idf-bipartite-leiden-1"),
        (split_child, "split", "human-split-1"),
    ] {
        let change = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,$3,$4,$5)")
            .bind(id).bind(brain).bind(kind).bind(change).bind(recipe).execute(&h.admin).await.unwrap();
        sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT $1,$2,$3,owner_id,'create' FROM brains WHERE id=$2")
            .bind(change).bind(brain).bind(id).execute(&h.admin).await.unwrap();
        sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) VALUES($1,$2,$3,$4,$5,1)")
            .bind(brain).bind(id).bind(entity).bind(input).bind(if kind=="split" {"explicit_split"} else {"weighted_leiden"}).execute(&h.admin).await.unwrap();
    }
    sqlx::query("INSERT INTO knowledge_topic_splits(brain_id,original_topic_id,split_topic_id,input_id,actor_id) SELECT $1,$2,$3,$4,owner_id FROM brains WHERE id=$1")
        .bind(brain).bind(split_parent).bind(split_child).bind(input).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_labels(brain_id,topic_id,basis_input,label,actor_id) SELECT $1,$2,$3,'Human-named split child',owner_id FROM brains WHERE id=$1")
        .bind(brain).bind(split_child).bind(input).execute(&h.admin).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_source_inputs WHERE brain_id=$1"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    let mut tx = h.admin.begin().await.unwrap();
    let (name, sql) = db::MIGRATIONS[migration];
    sqlx::raw_sql(sql).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO recollect_migrations(name) VALUES($1)")
        .bind(name)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let captured: (i64,i64,String,Uuid,Value) = sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_topic_intent_sources WHERE topic_id=$1),(SELECT count(*) FROM knowledge_topic_intent_mappings WHERE topic_id=$1),l.label,s.source_version_id,s.context FROM knowledge_topic_source_labels l JOIN knowledge_source_inputs s ON s.brain_id=l.brain_id AND s.id=l.basis_input WHERE l.topic_id=$1")
        .bind(topic).fetch_one(&h.admin).await.unwrap();
    assert_eq!((captured.0, captured.1), (1, 1));
    assert_eq!(captured.2, label);
    assert_eq!(captured.3, version);
    let expected: Value =
        sqlx::query_scalar("SELECT context FROM knowledge_mapping_contexts WHERE id=$1")
            .bind(context)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(captured.4, expected);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_topic_source_labels l JOIN knowledge_topic_source_exclusions e ON e.brain_id=l.brain_id AND e.topic_id=l.topic_id AND e.input_id=l.basis_input WHERE l.topic_id=$1"
        )
        .bind(excluded_topic)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        1,
        "An excluded legacy name stays stored behind its exact durable source exclusion"
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM knowledge_topic_source_splits x JOIN knowledge_topic_sources s ON s.brain_id=x.brain_id AND s.topic_id=x.split_topic_id AND s.input_id=x.input_id WHERE x.original_topic_id=$1 AND x.split_topic_id=$2 AND s.reason='explicit_source_split'")
        .bind(split_parent).bind(split_child).fetch_one(&h.admin).await.unwrap(),1,
        "A legacy split child gains only its exact durable source contributor");
    // Producer removal immediately after upgrade cannot remove the independent
    // human source basis, although all machine entity derivatives disappear.
    sqlx::query("DELETE FROM knowledge_mapping_inputs WHERE id=$1")
        .bind(input)
        .execute(&h.admin)
        .await
        .unwrap();
    let surviving: (i64,i64,i64,String) = sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_topic_memberships WHERE topic_id=$1),(SELECT count(*) FROM knowledge_topic_intent_mappings WHERE topic_id=$1),(SELECT count(*) FROM knowledge_topic_intent_sources WHERE topic_id=$1),label FROM knowledge_topic_source_labels WHERE topic_id=$1")
        .bind(topic).fetch_one(&h.admin).await.unwrap();
    assert_eq!(surviving, (0, 0, 1, label));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_topic_source_exclusions WHERE topic_id=$1"
        )
        .bind(excluded_topic)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        1
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM knowledge_topic_sources WHERE topic_id=$1 AND reason='explicit_source_split'")
        .bind(split_child).fetch_one(&h.admin).await.unwrap(),1,
        "Removing an old NER producer must not erase the human split child");
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT label FROM knowledge_topic_source_labels WHERE topic_id=$1"
        )
        .bind(split_child)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        "Human-named split child",
        "An edited split child's name needs its own durable canonical basis"
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j/GDS; zero provider calls"]
async fn organization_intent_privacy_source_only_survives_unrelated_prune_then_erases_exactly() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, docs, topic) = Box::pin(edited(&h, &owner, &base)).await;
    let before = snapshot(&h.admin, brain).await;
    let unrelated = staged(&h, brain, &docs[2], None).await;
    sqlx::query("DELETE FROM knowledge_mapping_inputs WHERE id=$1")
        .bind(unrelated)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        snapshot(&h.admin, brain).await,
        before,
        "An unrelated NER-input deletion cannot prune a genuine source-only edited group"
    );
    let receipt: (bool, Value, Option<Value>) = sqlx::query_as(
        "SELECT invalidated,input,response FROM command_receipts WHERE brain_id=$1 AND key=$2",
    )
    .bind(brain)
    .bind(format!("intent-privacy-name-{topic}"))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(receipt, (true, json!({}), None));

    // Vector replacement clears the actual automatic result and memberships.
    // Only separately saved human dependencies can keep the edited topic alive.
    sqlx::query("UPDATE semantic_entries SET embedding=embedding+embedding WHERE brain_id=$1")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_topic_sources WHERE topic_id=$1"
        )
        .bind(topic)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_topic_intent_sources WHERE topic_id=$1"
        )
        .bind(topic)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        2
    );
    // Omit the new keys before the first actual privacy apply, not afterwards.
    sqlx::raw_sql("CREATE FUNCTION fixture_legacy_intent_manifest() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.manifest:=NEW.manifest-'knowledge_topic_intent_sources'-'knowledge_topic_intent_mappings'; RETURN NEW; END $$; CREATE TRIGGER fixture_legacy_intent_manifest BEFORE INSERT ON privacy_requests FOR EACH ROW EXECUTE FUNCTION fixture_legacy_intent_manifest()")
        .execute(&h.admin).await.unwrap();
    for (n, doc) in docs[..2].iter().enumerate() {
        let target = json!({"kind":"source","id":doc["id"]});
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
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM knowledge_topic_intent_sources WHERE topic_id=$1"
            )
            .bind(topic)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
            1 - n as i64
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM knowledge_topics WHERE id=$1")
                .bind(topic)
                .fetch_one(&h.admin)
                .await
                .unwrap(),
            if n == 0 { 1 } else { 0 }
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_source_inputs WHERE brain_id=$1"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        1,
        "Independent third source survives"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_topic_source_labels WHERE topic_id=$1"
        )
        .bind(topic)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j/GDS; actual older-backup replay, zero provider calls"]
async fn organization_intent_privacy_populated_brain_delete_and_older_backup_replay() {
    let (mut h, owner, base, p, server) = setup().await;
    let (brain, docs, topic) = Box::pin(edited(&h, &owner, &base)).await;
    staged(&h, brain, &docs[0], Some(topic)).await;
    let (foreign, _) = h.fixture_member().await;
    let mut denied_read = db::actor_tx(&h.state.pool, foreign)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    for table in [
        "knowledge_topic_intent_sources",
        "knowledge_topic_intent_mappings",
    ] {
        let visible: i64 =
            sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE brain_id=$1"))
                .bind(brain)
                .fetch_one(&mut *denied_read)
                .await
                .unwrap();
        assert_eq!(visible, 0, "Foreign actor read populated {table}");
    }
    denied_read.rollback().await.unwrap();
    let source_input: Uuid = sqlx::query_scalar(
        "SELECT input_id FROM knowledge_topic_intent_sources WHERE topic_id=$1 LIMIT 1",
    )
    .bind(topic)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let mut denied_write = db::actor_tx(&h.state.pool, foreign)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let rejection = sqlx::query("INSERT INTO knowledge_topic_intent_sources(id,brain_id,topic_id,input_id) VALUES($1,$2,$3,$4)")
        .bind(Uuid::new_v4()).bind(brain).bind(topic).bind(source_input).execute(&mut *denied_write).await.unwrap_err();
    assert_eq!(
        rejection
            .as_database_error()
            .and_then(|e| e.code())
            .as_deref(),
        Some("42501"),
        "Knowing exact foreign IDs cannot bypass human-intent RLS"
    );
    denied_write.rollback().await.unwrap();
    let control = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Independent manual intent control"}),
    )
    .await;
    let control_brain: Uuid = control["id"].as_str().unwrap().parse().unwrap();
    let control_base = format!("/api/brains/{control_brain}");
    let (_, control_docs, control_topic) = Box::pin(edited(&h, &owner, &control_base)).await;
    staged(&h, control_brain, &control_docs[0], Some(control_topic)).await;
    let control_before = snapshot(&h.admin, control_brain).await;
    for table in [
        "knowledge_topic_intent_sources",
        "knowledge_topic_intent_mappings",
    ] {
        let registered: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM brain_deletion_dependents WHERE relation=$1)",
        )
        .bind(table)
        .fetch_one(&h.admin)
        .await
        .unwrap();
        assert!(
            registered,
            "{table} is missing from physical Brain deletion"
        );
    }
    privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable older-intent database: {backup}");
    h.state.pool.close().await;
    h.admin.close().await;
    sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'"))
        .execute(&h.root).await.unwrap();
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
    sqlx::raw_sql("CREATE FUNCTION fixture_legacy_intent_manifest() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.manifest:=NEW.manifest-'knowledge_topic_intent_sources'-'knowledge_topic_intent_mappings'; RETURN NEW; END $$; CREATE TRIGGER fixture_legacy_intent_manifest BEFORE INSERT ON privacy_requests FOR EACH ROW EXECUTE FUNCTION fixture_legacy_intent_manifest()")
        .execute(&h.admin).await.unwrap();
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
            "knowledge_topic_intent_sources",
            "knowledge_topic_intent_mappings",
            "knowledge_topic_sources",
            "knowledge_topic_source_labels",
            "knowledge_topics",
            "knowledge_source_inputs",
            "knowledge_mapping_inputs",
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
            snapshot(pool, control_brain).await,
            control_before,
            "Independent Brain intent changed"
        );
    }
    analytics::reconcile(&h.state.config, &h.admin)
        .await
        .unwrap();
    backup_pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}
