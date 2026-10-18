use super::*;
use axum::extract::FromRequestParts;
use recollect_server::{auth::Auth, knowledge_mapping::storage, privacy_journal, worker};
use review::ok;
use sqlx::types::Json;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mapping_ancestry_bulk_deep_and_late_copies_are_bounded() {
    let h = Harness::new().await;
    sqlx::raw_sql(include_str!("../fixtures/mapping_ancestry.sql"))
        .execute(&h.admin)
        .await
        .unwrap();
    h.finish().await;
}

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

async fn populated_derivatives(
    h: &Harness,
    brain: Uuid,
    actor: Uuid,
    parent: Uuid,
    excerpt: Uuid,
    claim: Uuid,
    independent: Uuid,
) -> Value {
    let mut tx = h.admin.begin().await.unwrap();
    let fixture = json!({"brain":brain,"actor":actor,"parent":parent,"excerpt":excerpt,"claim":claim,"independent":independent});
    sqlx::query("SELECT set_config('recollect.fixture',$1,true)")
        .bind(fixture.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../fixtures/mapping_populated_derivatives.sql"
    ))
    .execute(&mut *tx)
    .await
    .unwrap();
    let Json(result): Json<Value> =
        sqlx::query_scalar("SELECT result FROM mapping_populated_derivatives")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    tx.commit().await.unwrap();
    result
}

async fn assert_populated_derivatives_removed(pool: &PgPool, fixture: &Value) {
    let ids = |key: &str| {
        fixture[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().parse::<Uuid>().unwrap())
            .collect::<Vec<_>>()
    };
    let vectors:(i64,bool)=sqlx::query_as("SELECT count(*),bool_and(state='removed' AND embedding IS NULL) FROM semantic_entries WHERE id=ANY($1)")
        .bind(ids("vectors")).fetch_one(pool).await.unwrap();
    assert_eq!(
        vectors,
        (3, true),
        "Both nonNULL source/claim vectors and the pending derived vector are physically closed"
    );
    let independent: bool = sqlx::query_scalar(
        "SELECT state='ready' AND embedding IS NOT NULL FROM semantic_entries WHERE id=$1",
    )
    .bind(
        fixture["independent_vector"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(independent, "Independent vector remains usable");
    let jobs:(i64,bool)=sqlx::query_as("SELECT count(*),bool_and(state='cancelled' AND lease_token IS NULL AND lease_until IS NULL) FROM jobs WHERE id=ANY($1)")
        .bind(ids("jobs")).fetch_one(pool).await.unwrap();
    assert_eq!(jobs, (2, true));
    let requests: (i64, bool) =
        sqlx::query_as("SELECT count(*),bool_and(suppressed) FROM model_requests WHERE id=ANY($1)")
            .bind(ids("requests"))
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        requests,
        (4, true),
        "Explicit assessment receipt is suppressed too"
    );
    let learning: (Value, Option<Value>, String) = sqlx::query_as(
        "SELECT payload,verdicts,privacy_state FROM learning_support_stages WHERE run_id=$1",
    )
    .bind(
        fixture["learning"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(learning, (json!({}), None, "erased".into()));
    let handover:(Value,Option<Value>,String,String,String,Value)=sqlx::query_as("SELECT s.payload,s.verdict,s.privacy_state,r.title,r.state,r.selection FROM handover_support_stages s JOIN handover_runs r ON r.id=s.run_id WHERE s.run_id=$1")
        .bind(fixture["handover"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(pool).await.unwrap();
    assert_eq!(
        handover,
        (
            json!({}),
            None,
            "erased".into(),
            "".into(),
            "removed".into(),
            json!({})
        ),
        "A succeeded handover cannot retain derived title, payload, verdict or scope"
    );
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn mapping_legacy_manifest_recomputes_populated_derivatives_without_new_fields() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let auth = auth(&h, &owner).await;
    let (b, base) = brain(&h, &owner).await;
    let parent = source(&h, &owner, &base, "Vault uses app.").await;
    let excerpt=ok(&h,"POST",&format!("{base}/excerpts"),&owner,json!({
        "source_id":parent["id"],"version_id":parent["version"]["id"],"first_line":1,"last_line":1,"title":"Legacy manifest excerpt"})).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&parent["version"]["id"], "Legacy fixture", "declared"),
    )
    .await;
    let independent = source(&h, &owner, &base, "Independent source uses app.").await;
    let original = stage(
        &h,
        &auth,
        b,
        &parent,
        &candidates(version(&parent), "Vault uses app.", "Vault", false),
    )
    .await;
    let derived = stage(
        &h,
        &auth,
        b,
        &excerpt,
        &candidates(version(&excerpt), "Vault uses app.", "Vault", false),
    )
    .await;
    let claim: Uuid = claim["id"].as_str().unwrap().parse().unwrap();
    let fixture = populated_derivatives(
        &h,
        b,
        auth.user.id,
        version(&parent),
        version(&excerpt),
        claim,
        version(&independent),
    )
    .await;
    super::feedback::seed_for_version(&h, &owner, &base, version(&parent)).await;
    super::feedback::seed_for_version(&h, &owner, &base, version(&excerpt)).await;
    let independent_feedback =
        super::feedback::seed_for_version(&h, &owner, &base, version(&independent)).await;
    let mut tx = h.admin.begin().await.unwrap();
    let settings = json!({"brain":b,"actor":auth.user.id,"claim":claim});
    sqlx::query(
        "SELECT set_config('recollect.fixture',$1,true),set_config('recollect.actor',$2,true)",
    )
    .bind(settings.to_string())
    .bind(auth.user.id.to_string())
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!("../fixtures/mapping_legacy_privacy_apply.sql"))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_populated_derivatives_removed(&h.admin, &fixture).await;
    let feedback: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM retrieval_feedback_contexts WHERE brain_id=$1 ORDER BY id",
    )
    .bind(b)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        feedback,
        vec![
            independent_feedback["context_id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap()
        ],
        "A pre-feedback manifest recomputes exact ancestry before the old apply chain clears supports"
    );
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_inputs WHERE id=ANY($1)")
            .bind(vec![original, derived])
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        remaining, 0,
        "Missing old journal fields cannot retain exact source descendants"
    );
    h.finish().await;
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
    let excerpt = ok(
        &h,
        "POST",
        &format!("{base}/excerpts"),
        &owner,
        json!({"source_id":a["id"],"version_id":a["version"]["id"],
            "first_line":1,"last_line":1,"title":"Retained ancestral mapping replay"}),
    )
    .await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let excerpt_input = stage(
        &h,
        &auth,
        b,
        &excerpt,
        &candidates(version(&excerpt), text, "Vault", true),
    )
    .await;
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&a["version"]["id"], "Synthetic mapping source", "fixture"),
    )
    .await;
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
    let derivatives = populated_derivatives(
        &h,
        b,
        auth.user.id,
        version(&a),
        version(&excerpt),
        claim["id"].as_str().unwrap().parse().unwrap(),
        version(&x),
    )
    .await;

    let feedback_parent = super::feedback::seed_for_version(&h, &owner, &base, version(&a)).await;
    let feedback_excerpt =
        super::feedback::seed_for_version(&h, &owner, &base, version(&excerpt)).await;
    let feedback_independent =
        super::feedback::seed_for_version(&h, &owner, &base, version(&x)).await;

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
    let target = json!({"kind":"claim","id":claim["claim_id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    assert_eq!(preview["semantic_entries"], json!(3));
    assert_eq!(preview["semantic_batches"], json!(1));
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
    assert!(
        manifest["knowledge_mapping_inputs"]
            .as_array()
            .unwrap()
            .contains(&json!(excerpt_input)),
        "Preview includes mapping inputs reached through an exact ancestral model fence"
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
        assert_populated_derivatives_removed(pool, &derivatives).await;
        let feedback: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM retrieval_feedback_contexts WHERE brain_id=$1 ORDER BY id",
        )
        .bind(b)
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(
            feedback,
            vec![
                feedback_independent["context_id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap()
            ],
            "Exact erasure and older-backup replay remove parent/retained-descendant utility while preserving independent input"
        );
        for removed in [&feedback_parent, &feedback_excerpt] {
            let changes: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM retrieval_feedback_changes WHERE context_id=$1",
            )
            .bind(
                removed["context_id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
            )
            .fetch_one(pool)
            .await
            .unwrap();
            assert_eq!(changes, 0);
        }

        let descendant: (i64, i64) = sqlx::query_as(
            "SELECT
            (SELECT count(*) FROM knowledge_mapping_inputs WHERE id=$1),
            (SELECT count(*) FROM model_input_fences WHERE brain_id=$2 AND source_version_id=$3)",
        )
        .bind(excerpt_input)
        .bind(b)
        .bind(version(&excerpt))
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(
            descendant,
            (0, 1),
            "Old database replay closes descendant models and physical mapping data"
        );
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

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mapping_ancestral_fences_close_retained_excerpts_but_natural_expiry_does_not() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let auth = auth(&h, &owner).await;
    let (b, base) = brain(&h, &owner).await;
    let parent = source(&h, &owner, &base, "Vault uses app.").await;
    let excerpt = ok(
        &h,
        "POST",
        &format!("{base}/excerpts"),
        &owner,
        json!({"source_id":parent["id"],"version_id":parent["version"]["id"],
            "first_line":1,"last_line":1,"title":"Independently retained excerpt"}),
    )
    .await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let output = candidates(version(&excerpt), "Vault uses app.", "Vault", false);
    let input = stage(&h, &auth, b, &excerpt, &output).await;
    let independent = source(&h, &owner, &base, "Vault uses app.").await;
    let independent_input = stage(
        &h,
        &auth,
        b,
        &independent,
        &candidates(version(&independent), "Vault uses app.", "Vault", false),
    )
    .await;
    let independent_ids = entities(&h, independent_input).await;

    // Natural raw-input expiry does not revoke independently retained excerpts.
    sqlx::query("UPDATE source_versions SET privacy_state='expired' WHERE id=$1")
        .bind(version(&parent))
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(stage(&h, &auth, b, &excerpt, &output).await, input);

    // Use a real durable privacy request to satisfy the fence ownership FK.
    let removed = source(&h, &owner, &base, "Synthetic unrelated removed source.").await;
    let target = json!({"kind":"source","id":removed["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    let request = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let prepared = prepare(&h, &auth, b, &excerpt, &output).await;
    sqlx::query(
        "INSERT INTO model_input_fences(brain_id,source_version_id,request_id) VALUES($1,$2,$3)",
    )
    .bind(b)
    .bind(version(&parent))
    .bind(request["id"].as_str().unwrap().parse::<Uuid>().unwrap())
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
            version(&excerpt),
            &serde_json::to_vec(&output).unwrap()
        )
        .await
        .is_err()
    );
    let mut tx = auth
        .preparation_tx(&h.state.pool)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let failure = recollect_server::model_gateway::source(
        &h.state,
        &mut tx,
        b,
        version(&excerpt),
        2 * 1024 * 1024,
    )
    .await
    .err()
    .unwrap();
    assert_eq!(failure.1, "model_input_fenced");
    tx.rollback().await.unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_inputs WHERE id=$1")
            .bind(input)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        remaining, 0,
        "Fencing removes descendant payloads and mentions immediately"
    );
    assert_eq!(entities(&h, independent_input).await, independent_ids);
    let excerpt_state: String =
        sqlx::query_scalar("SELECT privacy_state FROM source_versions WHERE id=$1")
            .bind(version(&excerpt))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        excerpt_state, "active",
        "A model fence does not pretend the retained source was erased"
    );
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    h.finish().await;
}
