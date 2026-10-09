use super::*;
use axum::extract::FromRequestParts;
use recollect_server::{auth::Auth, knowledge_mapping::storage, privacy_journal, worker};
use review::ok;
use sqlx::types::Json;

async fn auth(h: &Harness, login: &Login) -> Auth {
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
async fn brain(h: &Harness, login: &Login) -> (Uuid, String) {
    let value = ok(
        h,
        "POST",
        "/api/brains",
        login,
        json!({"name":"Synthetic mapping fixture"}),
    )
    .await;
    let id = value["id"].as_str().unwrap().parse().unwrap();
    (id, format!("/api/brains/{id}"))
}
async fn source(h: &Harness, login: &Login, base: &str, text: &str) -> Value {
    let value = ok(h,"POST",&format!("{base}/sources"),login,json!({"title":"Synthetic source mapping","media_type":"text/plain","content":text,"retain_content":true})).await;
    for _ in 0..20 {
        if !worker::run_once(&h.state, "capture").await.unwrap() {
            return value;
        }
    }
    panic!("Owned synthetic capture did not drain");
}
fn candidates(version: Uuid, text: &str, technology: &str, alias: bool) -> Value {
    let mention = |key: &str, kind: &str, quote: &str| {
        let start = text.find(quote).unwrap();
        json!({"key":key,"kind":kind,"span":{"byte_start":start,"byte_end":start+quote.len(),"quote":quote},"confidence":0.99})
    };
    let mut value = json!({"schema":"source-mentions-1","adapter_revision":"synthetic-untrusted-1","source_version_id":version,"language":"en","mentions":[mention("tech","technology",technology),mention("app","service","app")],"relations":[],"aliases":[]});
    if alias {
        value["mentions"]
            .as_array_mut()
            .unwrap()
            .push(mention("service", "service", "service"));
        let start = text.find("app is also").unwrap();
        let end = text.find("service").unwrap() + "service".len();
        // Point both alias endpoints at these actual occurrences.
        value["mentions"][1]["span"] = json!({"byte_start":start,"byte_end":start+3,"quote":"app"});
        value["aliases"] = json!([{"left":"app","right":"service","span":{"byte_start":start,"byte_end":end,"quote":&text[start..end]}}]);
    }
    value
}
fn version(source: &Value) -> Uuid {
    source["version"]["id"].as_str().unwrap().parse().unwrap()
}
async fn prepare(
    h: &Harness,
    auth: &Auth,
    brain: Uuid,
    source: &Value,
    candidates: &Value,
) -> storage::PreparedSource {
    storage::prepare_source(
        &h.state,
        auth,
        brain,
        version(source),
        &serde_json::to_vec(candidates).unwrap(),
    )
    .await
    .unwrap_or_else(|e| panic!("{}", e.1))
}
async fn stage(h: &Harness, auth: &Auth, brain: Uuid, source: &Value, candidates: &Value) -> Uuid {
    let prepared = prepare(h, auth, brain, source, candidates).await;
    storage::stage_source(&h.state, auth, prepared)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1))
}
async fn entities(h: &Harness, input: Uuid) -> Vec<(String, Uuid)> {
    sqlx::query_as(
        "SELECT local_key,entity_id FROM knowledge_mentions WHERE input_id=$1 ORDER BY local_key",
    )
    .bind(input)
    .fetch_all(&h.admin)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mapping_staging_is_atomic_scoped_idempotent_and_unpublished() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let auth = auth(&h, &owner).await;
    let (b, base) = brain(&h, &owner).await;
    let (other, other_base) = brain(&h, &owner).await;
    let text = "🔐 Vault uses app. app is also known as service.";
    let a = source(&h, &owner, &base, text).await;
    let a_candidates = candidates(version(&a), text, "Vault", true);
    let a_input = stage(&h, &auth, b, &a, &a_candidates).await;
    let ids = entities(&h, a_input).await;
    assert_eq!(ids.len(), 3);
    assert_eq!(stage(&h, &auth, b, &a, &a_candidates).await, a_input);
    assert_eq!(entities(&h, a_input).await, ids);
    let x = source(&h, &owner, &base, "VAULT uses app.").await;
    let x_input = stage(
        &h,
        &auth,
        b,
        &x,
        &candidates(version(&x), "VAULT uses app.", "VAULT", false),
    )
    .await;
    let x_ids = entities(&h, x_input).await;
    let lookup = |rows: &[(String, Uuid)], key: &str| rows.iter().find(|r| r.0 == key).unwrap().1;
    assert_eq!(lookup(&ids, "tech"), lookup(&x_ids, "tech"));
    assert_ne!(
        lookup(&ids, "app"),
        lookup(&x_ids, "app"),
        "Unknown instances in independent sources must remain separate"
    );
    let y = source(&h, &owner, &other_base, "Vault uses app.").await;
    let y_input = stage(
        &h,
        &auth,
        other,
        &y,
        &candidates(version(&y), "Vault uses app.", "Vault", false),
    )
    .await;
    assert_ne!(
        lookup(&ids, "tech"),
        lookup(&entities(&h, y_input).await, "tech")
    );
    let aliases: Vec<(String, String)> = sqlx::query_as(
        "SELECT relation,disposition FROM knowledge_relation_candidates WHERE input_id=$1",
    )
    .bind(a_input)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    assert_eq!(aliases, vec![("alias".into(), "pending".into())]);
    assert_ne!(lookup(&ids, "app"), lookup(&ids, "service"));
    let mut malformed = a_candidates.clone();
    malformed["aliases"][0]["right"] = json!("foreign-endpoint");
    assert!(
        storage::prepare_source(
            &h.state,
            &auth,
            b,
            version(&a),
            &serde_json::to_vec(&malformed).unwrap()
        )
        .await
        .is_err()
    );
    let mut different = a_candidates.clone();
    different["mentions"][0]["confidence"] = json!(0.75);
    let prepared = prepare(&h, &auth, b, &a, &different).await;
    assert_eq!(
        storage::stage_source(&h.state, &auth, prepared)
            .await
            .err()
            .unwrap()
            .1,
        "mapping_input_changed"
    );
    assert!(
        storage::prepare_source(
            &h.state,
            &auth,
            other,
            version(&a),
            &serde_json::to_vec(&a_candidates).unwrap()
        )
        .await
        .is_err()
    );
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_mapping_inputs),(SELECT count(*) FROM knowledge_mapping_inputs WHERE state='published'),(SELECT count(*) FROM model_requests)").fetch_one(&h.admin).await.unwrap();
    assert_eq!(counts, (3, 0, 0));
    assert_eq!(entities(&h, a_input).await, ids);
    // Model-input removal changes the preparation epoch and closes publication.
    let prepared = prepare(&h, &auth, b, &a, &a_candidates).await;
    let target = json!({"kind":"source","id":a["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    let removal = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert_eq!(
        storage::stage_source(&h.state, &auth, prepared)
            .await
            .err()
            .unwrap()
            .1,
        "mapping_input_changed"
    );
    let x_candidates = candidates(version(&x), "VAULT uses app.", "VAULT", false);
    let prepared = prepare(&h, &auth, b, &x, &x_candidates).await;
    sqlx::query(
        "INSERT INTO model_input_fences(brain_id,source_version_id,request_id) VALUES($1,$2,$3)",
    )
    .bind(b)
    .bind(version(&x))
    .bind(removal["id"].as_str().unwrap().parse::<Uuid>().unwrap())
    .execute(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        storage::stage_source(&h.state, &auth, prepared)
            .await
            .err()
            .unwrap()
            .1,
        "mapping_input_changed"
    );
    assert!(
        storage::prepare_source(
            &h.state,
            &auth,
            b,
            version(&a),
            &serde_json::to_vec(&a_candidates).unwrap()
        )
        .await
        .is_err()
    );
    // Revocation is checked on the final transaction, not just preparation.
    let y_candidates = candidates(version(&y), "Vault uses app.", "Vault", false);
    let prepared = prepare(&h, &auth, other, &y, &y_candidates).await;
    sqlx::query("DELETE FROM sessions WHERE token=$1")
        .bind(auth.session)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        storage::stage_source(&h.state, &auth, prepared)
            .await
            .err()
            .unwrap()
            .0,
        StatusCode::UNAUTHORIZED
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn mapping_derivatives_erase_and_replay_into_an_actual_older_database() {
    let mut h = Harness::new().await;
    let owner = h.login().await;
    let auth = auth(&h, &owner).await;
    let (b, base) = brain(&h, &owner).await;
    let text = "Vault uses app. app is also known as service.";
    let a = source(&h, &owner, &base, text).await;
    let a_candidates = candidates(version(&a), text, "Vault", true);
    let a_input = stage(&h, &auth, b, &a, &a_candidates).await;
    let x = source(&h, &owner, &base, "VAULT uses app.").await;
    let x_input = stage(
        &h,
        &auth,
        b,
        &x,
        &candidates(version(&x), "VAULT uses app.", "VAULT", false),
    )
    .await;
    let independent_ids = entities(&h, x_input).await;
    let group = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"collection","name":"Manual fixture group","description":"Synthetic"}),
    )
    .await;
    ok(
        &h,
        "PUT",
        &format!("{base}/sources/{}/groups", x["id"].as_str().unwrap()),
        &owner,
        json!({"group_ids":[group["id"]]}),
    )
    .await;
    privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable older-mapping database: {backup}");
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
    let mut backup_config = (*h.state.config).clone();
    let mut url = reqwest::Url::parse(&backup_config.database_url).unwrap();
    url.set_path(&backup);
    backup_config.database_url = url.to_string();
    // Share the real journal but isolate the owned older-database artifact root.
    backup_config.artifact_dir = format!(".cache/{backup}-artifacts");
    let backup_pool = db::pool(&backup_config.database_url).await.unwrap();
    let target = json!({"kind":"source","id":a["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    let erasure = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let manifest: Json<Value> =
        sqlx::query_scalar("SELECT manifest FROM privacy_requests WHERE id=$1")
            .bind(erasure["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(
        manifest["knowledge_mapping_inputs"]
            .as_array()
            .unwrap()
            .contains(&json!(a_input))
    );
    privacy_journal::maintain(&h.state).await.unwrap();
    assert!(
        privacy_journal::barrier(&backup_pool, &backup_config)
            .await
            .is_err(),
        "An older database cannot serve before replay"
    );
    privacy_journal::reconcile(&backup_admin, &backup_config)
        .await
        .unwrap();
    privacy_journal::barrier(&backup_pool, &backup_config)
        .await
        .unwrap();
    for pool in [&h.admin, &backup_admin] {
        let counts:(i64,i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_mapping_inputs WHERE id=$1),(SELECT count(*) FROM knowledge_mentions WHERE input_id=$1),(SELECT count(*) FROM knowledge_relation_candidates WHERE input_id=$1),(SELECT count(*) FROM knowledge_entities WHERE brain_id=$2),(SELECT count(*) FROM knowledge_mapping_contexts WHERE brain_id=$2)")
            .bind(a_input).bind(b).fetch_one(pool).await.unwrap();
        assert_eq!(
            counts,
            (0, 0, 0, 2, 1),
            "Remove every derivative while retaining independent support"
        );
        let ids: Vec<(String,Uuid)>=sqlx::query_as("SELECT local_key,entity_id FROM knowledge_mentions WHERE input_id=$1 ORDER BY local_key").bind(x_input).fetch_all(pool).await.unwrap();
        assert_eq!(ids, independent_ids);
        let manual: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM evidence_memberships WHERE source_id=$1 AND group_id=$2",
        )
        .bind(x["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .bind(group["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(manual, 1);
    }
    assert!(
        storage::prepare_source(
            &h.state,
            &auth,
            b,
            version(&a),
            &serde_json::to_vec(&a_candidates).unwrap()
        )
        .await
        .is_err()
    );
    backup_pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup} WITH (FORCE)"))
        .execute(&h.root)
        .await
        .unwrap();
    h.finish().await;
}
