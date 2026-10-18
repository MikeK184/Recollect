//! Actual shadow request binding and privacy/old-backup lifecycle. Synthetic
//! retained vectors exercise plumbing only; no quality or paid-model claim.
use super::*;
use axum::extract::FromRequestParts;
use recollect_server::{
    auth::Auth,
    db,
    graph::analytics::{
        self,
        shadow::{self, Mode, ShadowSource},
    },
    knowledge_mapping::source_features::HybridParameters,
    privacy_journal,
};

pub(super) async fn evaluator(h: &Harness, login: &Login) -> Auth {
    let (mut parts, _) = Request::builder()
        .method("POST")
        .header("cookie", &login.cookie)
        .header("x-csrf-token", &login.csrf)
        .body(())
        .unwrap()
        .into_parts();
    Auth::from_request_parts(&mut parts, &h.state)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1))
}
fn sources(docs: &[Value], candidate: Option<Uuid>) -> Vec<ShadowSource> {
    docs.iter()
        .enumerate()
        .map(|(n, d)| ShadowSource {
            source_version_id: d["version"]["id"].as_str().unwrap().parse().unwrap(),
            candidate_id: if n == 0 { candidate } else { None },
        })
        .collect()
}
async fn calculate(h: &Harness, auth: &Auth, brain: Uuid, inputs: &[ShadowSource]) -> Uuid {
    let report = shadow::queue(
        &h.state,
        auth,
        brain,
        inputs,
        Mode::EligibleVector,
        HybridParameters::default(),
        "2026.08.1",
    )
    .await
    .unwrap_or_else(|e| panic!("queue: {}", e.1));
    let state = h.state.clone();
    assert!(
        tokio::spawn(async move { worker::run_once(&state, "heavy").await })
            .await
            .unwrap()
            .unwrap()
    );
    let result = shadow::result(&h.state, auth, brain, report)
        .await
        .unwrap_or_else(|e| panic!("result: {}", e.1));
    assert_eq!(result.sources.len(), inputs.len());
    assert_eq!(result.groups.len(), 1);
    assert_eq!(result.groups[0].source_versions.len(), 2);
    report
}

pub(super) async fn actual_candidate(
    h: &Harness,
    auth: &Auth,
    brain: Uuid,
    version: Uuid,
    p: &Provider,
) -> Uuid {
    let cx = gateway::Context {
        brain,
        actor: auth.user.id,
        device: None,
    };
    let mapping = json!({"language":"en","mentions":[{"key":"architecture","kind":"concept",
        "span":{"quote":"architecture","line_from":1,"line_to":1,"occurrence":null},"confidence":1.0}],"relations":[],"aliases":[]});
    *p.candidates.lock().unwrap() = json!({"claims":[],"mapping":mapping});
    let response = gateway::invoke(&h.state, cx, gateway::extraction(Uuid::new_v4(), version))
        .await
        .unwrap_or_else(|e| panic!("gateway: {}", e.1));
    shadow::admit(&h.state, &auth, cx, response.request.id, version, &mapping)
        .await
        .unwrap_or_else(|e| panic!("admit: {}", e.1))
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; one loopback request, zero paid calls"]
async fn organization_shadow_candidate_cleanup_preserves_normal_same_brain_report() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, docs, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["extraction", "embedding"])
    })
    .await;
    let normal = organization_hybrid::calculate(&h, &h.state, brain).await;
    let auth = evaluator(&h, &owner).await;
    let version = docs[0]["version"]["id"].as_str().unwrap().parse().unwrap();
    let candidate = actual_candidate(&h, &auth, brain, version, &p).await;
    let inputs = sources(&docs, Some(candidate));
    let report = calculate(&h, &auth, brain, &inputs).await;
    let before:Value=sqlx::query_scalar("SELECT jsonb_build_object('epoch',b.analytics_epoch,'report',(SELECT to_jsonb(r) FROM analytics_reports r WHERE id=$2),'topics',(SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM knowledge_topics t WHERE brain_id=b.id),'head',(SELECT to_jsonb(h) FROM knowledge_organization_heads h WHERE brain_id=b.id),'outbox',(SELECT to_jsonb(o) FROM knowledge_organization_outbox o WHERE brain_id=b.id)) FROM brains b WHERE id=$1")
        .bind(brain).bind(normal).fetch_one(&h.admin).await.unwrap();
    // Expiry maintenance deletes candidates through this same FK/trigger path.
    sqlx::query("DELETE FROM knowledge_shadow_candidates WHERE id=$1")
        .bind(candidate)
        .execute(&h.admin)
        .await
        .unwrap();
    let after:Value=sqlx::query_scalar("SELECT jsonb_build_object('epoch',b.analytics_epoch,'report',(SELECT to_jsonb(r) FROM analytics_reports r WHERE id=$2),'topics',(SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM knowledge_topics t WHERE brain_id=b.id),'head',(SELECT to_jsonb(h) FROM knowledge_organization_heads h WHERE brain_id=b.id),'outbox',(SELECT to_jsonb(o) FROM knowledge_organization_outbox o WHERE brain_id=b.id)) FROM brains b WHERE id=$1")
        .bind(brain).bind(normal).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        before, after,
        "Experiment cleanup cannot mutate normal organization state"
    );
    let removed:(String,Option<Value>,Option<Value>,i64,i64)=sqlx::query_as("SELECT state,descriptor,scores,(SELECT count(*) FROM knowledge_shadow_calculations WHERE report_id=r.id),(SELECT count(*) FROM knowledge_shadow_sources WHERE report_id=r.id) FROM analytics_reports r WHERE id=$1")
        .bind(report).fetch_one(&h.admin).await.unwrap();
    assert_eq!(removed, ("removed".into(), None, None, 0, 0));
    assert!(
        shadow::result(&h.state, &auth, brain, report)
            .await
            .is_err()
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; actual older-backup replay, one loopback request"]
async fn organization_shadow_populated_brain_delete_and_legacy_backup_replay() {
    let (mut h, owner, base, p, server) = setup().await;
    let (brain, docs, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["extraction", "embedding"])
    })
    .await;
    let auth = evaluator(&h, &owner).await;
    let version = docs[0]["version"]["id"].as_str().unwrap().parse().unwrap();
    let candidate = actual_candidate(&h, &auth, brain, version, &p).await;
    let report = calculate(&h, &auth, brain, &sources(&docs, Some(candidate))).await;
    let control = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Independent shadow control"}),
    )
    .await;
    let control_brain: Uuid = control["id"].as_str().unwrap().parse().unwrap();
    let control_base = format!("/api/brains/{control_brain}");
    let (_, control_docs, _) = organization_hybrid::prepare(&h, &owner, &control_base).await;
    let control_report = calculate(&h, &auth, control_brain, &sources(&control_docs, None)).await;
    let control_before: Value =
        sqlx::query_scalar("SELECT result FROM knowledge_shadow_calculations WHERE report_id=$1")
            .bind(control_report)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable older-shadow database: {backup}");
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
    sqlx::raw_sql("CREATE FUNCTION fixture_legacy_shadow_manifest() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.manifest:=NEW.manifest-'knowledge_shadow_candidates'-'knowledge_shadow_reports'; RETURN NEW; END $$; CREATE TRIGGER fixture_legacy_shadow_manifest BEFORE INSERT ON privacy_requests FOR EACH ROW EXECUTE FUNCTION fixture_legacy_shadow_manifest()")
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
            "knowledge_shadow_sources",
            "knowledge_shadow_calculations",
            "knowledge_shadow_candidates",
            "knowledge_organization_vectors",
            "analytics_reports",
        ] {
            let count: i64 =
                sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE brain_id=$1"))
                    .bind(brain)
                    .fetch_one(pool)
                    .await
                    .unwrap();
            assert_eq!(
                count, 0,
                "Deleted Brain resurrected {table}, report {report}"
            );
        }
        let surviving: Value = sqlx::query_scalar(
            "SELECT result FROM knowledge_shadow_calculations WHERE report_id=$1",
        )
        .bind(control_report)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(
            surviving, control_before,
            "Independent shadow result changed"
        );
    }
    analytics::reconcile(&h.state.config, &h.admin)
        .await
        .unwrap();
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    backup_pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}
