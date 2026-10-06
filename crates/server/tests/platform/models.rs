use super::*;
#[path = "answers.rs"]
mod answers;
#[path = "autonomous.rs"]
mod autonomous;
#[path = "capture.rs"]
mod capture;
#[path = "models_catalogue.rs"]
mod catalogue;
#[path = "mcp_memory.rs"]
mod mcp_memory;
#[path = "procedures.rs"]
mod procedures;
#[path = "semantic.rs"]
mod semantic;
use axum::{Json, extract::State, routing::post};
use recollect_server::{model_gateway as gateway, worker};
use review::ok;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

pub(crate) struct Provider {
    pub(crate) calls: AtomicUsize,
    catalogue_calls: AtomicUsize,
    pub(crate) delay: AtomicU64,
    mode: AtomicUsize,
    pub(crate) candidates: Mutex<Value>,
    pub(crate) bodies: Mutex<Vec<Value>>,
    entered: tokio::sync::Notify,
}
fn candidate(subject: &str, predicate: &str, value: &str, line: i32) -> Value {
    json!({"subject":subject,"predicate":predicate,"value":value,"rationale":"Declared by this synthetic source.","line_from":line,"line_to":line})
}
async fn wire(
    State(provider): State<Arc<Provider>>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    provider.calls.fetch_add(1, Ordering::SeqCst);
    provider.bodies.lock().unwrap().push(body.clone());
    provider.entered.notify_one();
    tokio::time::sleep(std::time::Duration::from_millis(
        provider.delay.load(Ordering::SeqCst),
    ))
    .await;
    match provider.mode.load(Ordering::SeqCst) {
        1 => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({"error":{"message":"Synthetic provider rate limit"}})),
            );
        }
        2 => {
            return (
                StatusCode::OK,
                Json(
                    json!({"model":body["model"],"status":"incomplete","usage":{"input_tokens":10,"output_tokens":4,"total_tokens":14}}),
                ),
            );
        }
        3 => {
            return (
                StatusCode::OK,
                Json(
                    json!({"model":body["model"],"status":"completed","output":[{"content":[{"type":"refusal","refusal":"Synthetic refusal"}]}],"usage":{"input_tokens":10,"output_tokens":4,"total_tokens":14}}),
                ),
            );
        }
        4 => {
            return (
                StatusCode::OK,
                Json(
                    json!({"model":body["model"],"data":[{"index":0,"embedding":[0.1]}],"usage":{"prompt_tokens":6,"total_tokens":6}}),
                ),
            );
        }
        _ => (),
    }
    if body["encoding_format"] == "float" {
        assert!(matches!(
            body["model"].as_str(),
            Some("text-embedding-3-large" | "text-embedding-3-small")
        ));
        let dimensions = body["dimensions"].as_u64().unwrap() as usize;
        let mut data: Vec<Value> = body["input"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, input)| {
                let vector = if [5, 7].contains(&provider.mode.load(Ordering::SeqCst)) {
                    let text = input.as_str().unwrap();
                    let mut vector = vec![0.0f64; dimensions];
                    let (x, y) = if text.contains("NEAR_VECTOR") {
                        (3.0, 4.0)
                    } else if text.contains("DISTRACTOR_VECTOR") {
                        (0.0, 1.0)
                    } else {
                        (1.0, 0.0)
                    };
                    vector[0] = x;
                    vector[1] = y;
                    vector
                } else if provider.mode.load(Ordering::SeqCst) == 9 {
                    vec![0.0f64; dimensions]
                } else if provider.mode.load(Ordering::SeqCst) == 6 {
                    vec![0.00123456789123456f64; dimensions]
                } else {
                    vec![0.001f64; dimensions]
                };
                json!({"index":i,"embedding":vector})
            })
            .collect();
        if provider.mode.load(Ordering::SeqCst) == 7 {
            data.reverse();
        }
        if provider.mode.load(Ordering::SeqCst) == 10 && data.len() > 1 {
            data[1]["index"] = json!(0);
        }
        let returned = if provider.mode.load(Ordering::SeqCst) == 8 {
            "unapproved-embedding-model"
        } else {
            body["model"].as_str().unwrap()
        };
        (
            StatusCode::OK,
            Json(
                json!({"model":returned,"data":data,"usage":{"prompt_tokens":6,"total_tokens":6}}),
            ),
        )
    } else {
        assert!(matches!(
            body["model"].as_str(),
            Some("gpt-5.6-luna" | "gpt-4.1-mini" | "gpt-4.1")
        ));
        assert_eq!(body["store"], false);
        if body["model"] == "gpt-5.6-luna" {
            assert_eq!(body["reasoning"]["effort"], "none");
        } else {
            assert!(body.get("reasoning").is_none());
        }
        assert!(body.get("tools").is_none());
        let extracted = if body["text"]["format"]["name"] == "synthetic_connection" {
            json!({"service":"Amber","environment":"test"})
        } else {
            provider.candidates.lock().unwrap().clone()
        };
        (
            StatusCode::OK,
            Json(
                json!({"model":body["model"],"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":extracted.to_string()}]}],"usage":{"input_tokens":23,"output_tokens":17,"total_tokens":40}}),
            ),
        )
    }
}
async fn catalogue_wire(State(provider): State<Arc<Provider>>) -> (StatusCode, Json<Value>) {
    provider.catalogue_calls.fetch_add(1, Ordering::SeqCst);
    match provider.mode.load(Ordering::SeqCst) {
        1 => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error":"provider secret must never appear in output"})),
        ),
        11 => (StatusCode::OK, Json(json!({"data":"invalid"}))),
        12 => (
            StatusCode::OK,
            Json(json!({"data":[],"oversized":"x".repeat(1_048_577)})),
        ),
        _ => (
            StatusCode::OK,
            Json(json!({"data":[
                {"id":"gpt-5.6-luna"},{"id":"gpt-4.1-mini"},{"id":"gpt-4.1"},
                {"id":"text-embedding-3-large"},{"id":"text-embedding-3-small"},
                {"id":"gpt-4.1-nano"},{"id":"gpt-realtime"},{"id":"unknown-future-model"}
            ]})),
        ),
    }
}

async fn setup() -> (
    Harness,
    Login,
    String,
    Arc<Provider>,
    tokio::task::JoinHandle<()>,
) {
    let mut h = Harness::new().await;
    let (provider, task) = configure_provider(&mut h).await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic provider proof"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    (h, owner, base, provider, task)
}
pub(crate) async fn configure_provider(
    h: &mut Harness,
) -> (Arc<Provider>, tokio::task::JoinHandle<()>) {
    let provider = Arc::new(Provider {
        calls: AtomicUsize::new(0),
        catalogue_calls: AtomicUsize::new(0),
        delay: AtomicU64::new(0),
        mode: AtomicUsize::new(0),
        candidates: Mutex::new(json!({"claims":[candidate("Amber","port","8080",1)]})),
        bodies: Mutex::new(vec![]),
        entered: tokio::sync::Notify::new(),
    });
    let router = Router::new()
        .route("/v1/models", axum::routing::get(catalogue_wire))
        .route("/v1/responses", post(wire))
        .route("/v1/embeddings", post(wire))
        .with_state(provider.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut config = (*h.state.config).clone();
    config.models.endpoint = format!("http://{address}/v1");
    config.models.key = Some("synthetic-provider-credential".into());
    h.state = AppState::new(h.state.pool.clone(), config).unwrap();
    h.router = app(h.state.clone());
    (provider, task)
}
pub(crate) async fn allow(
    h: &Harness,
    owner: &Login,
    base: &str,
    mutate: impl FnOnce(&mut Value),
) -> Value {
    let mut settings = ok(
        h,
        "GET",
        &format!("{base}/models/policy"),
        owner,
        Value::Null,
    )
    .await;
    let base_change = settings["current"]["change_id"].clone();
    let policy = &mut settings["current"]["policy"];
    policy["enabled"] = json!(true);
    // This helper exercises the original explicit/literal mode; autonomous
    // closed-loop scenarios opt in below through their own policy mutation.
    policy["autonomous_memory"] = json!(false);
    policy["content_classes"] = json!(["document", "query", "raw_session", "support_excerpt"]);
    mutate(policy);
    ok(
        h,
        "PUT",
        &format!("{base}/models/policy"),
        owner,
        json!({"base_change":base_change,"policy":policy}),
    )
    .await
}
async fn source(h: &Harness, owner: &Login, base: &str, text: &str) -> Value {
    ok(h,"POST",&format!("{base}/sources"),owner,json!({"title":"Synthetic model evidence","media_type":"text/plain","retain_content":true,"content":text})).await
}
async fn learn(h: &Harness, owner: &Login, base: &str, source: &Value) -> Value {
    ok(h,"POST",&format!("{base}/learning"),owner,json!({"source_version_id":source["version"]["id"],"selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"manifest_revision_id":null,"operation_id":null})).await
}
async fn runs(h: &Harness, owner: &Login, base: &str) -> Value {
    ok(h, "GET", &format!("{base}/learning"), owner, Value::Null).await
}
async fn model_job(h: &Harness) {
    assert!(worker::run_once(&h.state, "model").await.unwrap());
}
fn literal_rule() -> Value {
    json!({"name":"declared-ports","source_classes":["document"],"collection_ids":[],"properties":["port"]})
}

async fn context(h: &Harness, base: &str) -> gateway::Context {
    gateway::Context {
        brain: Uuid::parse_str(base.rsplit('/').next().unwrap()).unwrap(),
        actor: sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        device: None,
    }
}
async fn wait_calls(p: &Provider, count: usize) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while p.calls.load(Ordering::SeqCst) < count {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("The controlled HTTP provider must receive this call");
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn learning_device_scope_foreign_inputs_and_inflight_revocation() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    let (writer_id, writer) = h.fixture_member().await;
    let (reader_id, reader) = h.fixture_member().await;
    for (id, role) in [(writer_id, "writer"), (reader_id, "reader")] {
        ok(
            &h,
            "PUT",
            &format!("{base}/grants/{id}"),
            &owner,
            json!({"role":role}),
        )
        .await;
    }
    let evidence = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let selection = json!({"repository_ids":[],"area_ids":[],"environment_id":null});
    let mut input = json!({"source_version_id":evidence["version"]["id"],"selection":selection,"manifest_revision_id":null,"operation_id":null});
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/learning"),
            Some(&reader),
            input.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, token) = h.pair_device(&writer, "Synthetic learning device").await;
    assert_eq!(
        h.bearer("POST", &format!("{base}/learning"), &token, input.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let task = h
        .bearer(
            "POST",
            &format!("{base}/workspace/tasks"),
            &token,
            json!({"label":"Synthetic scoped learning","selection":selection}),
        )
        .await;
    assert_eq!(task.0, StatusCode::OK);
    let operation = h
        .bearer(
            "POST",
            &format!(
                "{base}/workspace/tasks/{}/operations",
                task.1["task"]["id"].as_str().unwrap()
            ),
            &token,
            json!({"kind":"write"}),
        )
        .await;
    assert_eq!(operation.0, StatusCode::OK);
    input["operation_id"] = operation.1["id"].clone();
    let started = h
        .bearer("POST", &format!("{base}/learning"), &token, input.clone())
        .await;
    assert_eq!(started.0, StatusCode::OK);
    model_job(&h).await;
    assert_eq!(
        runs(&h, &owner, &base).await["items"][0]["state"],
        "succeeded"
    );
    let other = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Foreign model inputs"}),
    )
    .await;
    let other_base = format!("/api/brains/{}", other["id"].as_str().unwrap());
    let foreign = source(&h, &owner, &other_base, "Amber.port = 8181\n").await;
    let before = p.calls.load(Ordering::SeqCst);
    input["source_version_id"] = foreign["version"]["id"].clone();
    assert!(
        h.bearer("POST", &format!("{base}/learning"), &token, input.clone())
            .await
            .0
            .is_client_error()
    );
    let context = context(&h, &base).await;
    let foreign_id: Uuid = foreign["version"]["id"].as_str().unwrap().parse().unwrap();
    assert!(
        gateway::invoke(
            &h.state,
            context,
            gateway::extraction(Uuid::new_v4(), foreign_id)
        )
        .await
        .is_err()
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), before);
    input["source_version_id"] = evidence["version"]["id"].clone();
    let queued = h
        .bearer("POST", &format!("{base}/learning"), &token, input)
        .await;
    assert_eq!(queued.0, StatusCode::OK);
    let job = worker::claim(&h.state.pool, "model")
        .await
        .unwrap()
        .unwrap();
    p.delay.store(250, Ordering::SeqCst);
    let state = h.state.clone();
    let pending = tokio::spawn(async move { worker::execute(&state, &job).await });
    wait_calls(&p, before + 1).await;
    assert_eq!(
        h.call(
            "DELETE",
            &format!("{base}/grants/{writer_id}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    pending.await.unwrap().unwrap();
    let failed = runs(&h, &owner, &base).await;
    assert_eq!(failed["items"][0]["state"], "failed");
    let calls = ok(
        &h,
        "GET",
        &format!("{base}/models/usage"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(calls["requests"][0]["suppressed"], true);
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM claims WHERE brain_id=$1")
        .bind(context.brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(claims, 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn erased_claim_fences_retained_source_even_before_claim_existed_in_backup() {
    use recollect_server::privacy_journal;
    let (mut h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |policy| {
        policy["acceptance"] = literal_rule()
    })
    .await;
    let evidence = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let version: Uuid = evidence["version"]["id"].as_str().unwrap().parse().unwrap();
    let context = context(&h, &base).await;
    // This database predates the generated claim; only the retained input exists.
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable model restore fixture: {backup}");
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
        (*h.state.config).clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    admin_url.set_path(&backup);
    let backup_admin = db::pool(admin_url.as_str()).await.unwrap();
    let mut backup_config = (*h.state.config).clone();
    let mut url = reqwest::Url::parse(&backup_config.database_url).unwrap();
    url.set_path(&backup);
    backup_config.database_url = url.to_string();
    let backup_state = AppState::new(
        db::pool(&backup_config.database_url).await.unwrap(),
        backup_config.clone(),
    )
    .unwrap();
    learn(&h, &owner, &base, &evidence).await;
    model_job(&h).await;
    let learned = runs(&h, &owner, &base).await;
    let claim = learned["items"][0]["claim_ids"][0].clone();
    let pending = learn(&h, &owner, &base, &evidence).await;
    let target = json!({"kind":"claim","id":claim});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    assert_eq!(preview["source_versions"], 0);
    assert_eq!(preview["model_input_fences"], 1);
    assert_eq!(preview["jobs"], 1);
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let removed = runs(&h, &owner, &base).await;
    assert_eq!(removed["items"][0]["id"], pending["id"]);
    assert_eq!(removed["items"][0]["state"], "removed");
    let source_content = ok(
        &h,
        "GET",
        &format!(
            "{base}/sources/{}/versions/{version}",
            evidence["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(source_content["content"], "Amber.port = 8080\n");
    let before = p.calls.load(Ordering::SeqCst);
    assert_eq!(
        gateway::invoke(
            &h.state,
            context,
            gateway::extraction(Uuid::new_v4(), version)
        )
        .await
        .err()
        .unwrap()
        .1,
        "model_input_fenced"
    );
    privacy_journal::run_once(&h.state).await.unwrap();
    assert!(
        privacy_journal::barrier(&backup_state.pool, &backup_config)
            .await
            .is_err()
    );
    privacy_journal::reconcile(&backup_admin, &backup_config)
        .await
        .unwrap();
    privacy_journal::barrier(&backup_state.pool, &backup_config)
        .await
        .unwrap();
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM claims")
        .fetch_one(&backup_admin)
        .await
        .unwrap();
    assert_eq!(claims, 0);
    assert_eq!(
        gateway::invoke(
            &backup_state,
            context,
            gateway::extraction(Uuid::new_v4(), version)
        )
        .await
        .err()
        .unwrap()
        .1,
        "model_input_fenced"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), before);
    let source_state: String =
        sqlx::query_scalar("SELECT privacy_state FROM source_versions WHERE id=$1")
            .bind(version)
            .fetch_one(&backup_admin)
            .await
            .unwrap();
    assert_eq!(source_state, "active");
    sqlx::query("UPDATE model_requests SET created_at=now()-interval '400 days'")
        .execute(&h.admin)
        .await
        .unwrap();
    privacy_journal::run_once(&h.state).await.unwrap();
    let expired: (bool, String, Option<String>) =
        sqlx::query_as("SELECT detail_expired,model,returned_model FROM model_requests LIMIT 1")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(expired, (true, "detail_expired".into(), None));
    backup_state.pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn gateway_concurrency_policy_change_failures_and_uncertain_replay() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    let evidence = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let version = Uuid::parse_str(evidence["version"]["id"].as_str().unwrap()).unwrap();
    let context = context(&h, &base).await;
    p.delay.store(250, Ordering::SeqCst);
    let state = h.state.clone();
    let first = tokio::spawn(async move {
        gateway::invoke(
            &state,
            context,
            gateway::extraction(Uuid::new_v4(), version),
        )
        .await
    });
    wait_calls(&p, 1).await;
    let denied = gateway::invoke(
        &h.state,
        context,
        gateway::extraction(Uuid::new_v4(), version),
    )
    .await
    .err()
    .unwrap();
    assert_eq!(denied.1, "model_concurrency_full");
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    allow(&h, &owner, &base, |policy| {
        policy["max_output_tokens"] = json!(512)
    })
    .await;
    assert_eq!(
        first.await.unwrap().err().unwrap().1,
        "model_policy_changed"
    );
    let usage = ok(
        &h,
        "GET",
        &format!("{base}/models/usage"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(usage["requests"][0]["suppressed"], true);
    assert_eq!(usage["requests"][0]["total_tokens"], 40);
    let mut config = (*h.state.config).clone();
    config.models.key = None;
    let missing = AppState::new(h.state.pool.clone(), config).unwrap();
    assert_eq!(
        gateway::invoke(
            &missing,
            context,
            gateway::extraction(Uuid::new_v4(), version)
        )
        .await
        .err()
        .unwrap()
        .1,
        "model_credentials_missing"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    p.delay.store(0, Ordering::SeqCst);
    for (mode, error) in [
        (1, "provider_rate_limited"),
        (2, "provider_incomplete"),
        (3, "provider_refusal"),
    ] {
        p.mode.store(mode, Ordering::SeqCst);
        let denied = gateway::invoke(
            &h.state,
            context,
            gateway::extraction(Uuid::new_v4(), version),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(denied.1, error);
    }
    p.mode.store(0, Ordering::SeqCst);
    let operation = Uuid::new_v4();
    let complete = gateway::invoke(&h.state, context, gateway::extraction(operation, version))
        .await
        .unwrap_or_else(|e| panic!("Gateway failed: {}", e.1));
    // Represent an admitted call whose process died before recording a result.
    sqlx::query("UPDATE model_requests SET state='running',deadline=now()-interval '1 second',finished_at=NULL WHERE id=$1").bind(complete.request.id).execute(&h.admin).await.unwrap();
    let before = p.calls.load(Ordering::SeqCst);
    assert_eq!(
        gateway::invoke(&h.state, context, gateway::extraction(operation, version))
            .await
            .err()
            .unwrap()
            .1,
        "model_attempt_recorded"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), before);
    let saved: String = sqlx::query_scalar("SELECT state FROM model_requests WHERE id=$1")
        .bind(complete.request.id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(saved, "uncertain");
    p.mode.store(4, Ordering::SeqCst);
    let mut embedding = gateway::extraction(Uuid::new_v4(), version);
    embedding.purpose = "embedding".into();
    embedding.format = gateway::Format::Embedding;
    assert_eq!(
        gateway::invoke(&h.state, context, embedding)
            .await
            .err()
            .unwrap()
            .1,
        "provider_shape"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), before + 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn learning_automatic_retry_restart_atomic_failure_and_inflight_erasure() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |policy| {
        policy["automatic_learning"] = json!(true);
        policy["acceptance"] = literal_rule();
    })
    .await;
    let original = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    assert!(worker::run_once(&h.state, "capture").await.unwrap());
    let queued = runs(&h, &owner, &base).await;
    assert_eq!(queued["items"].as_array().unwrap().len(), 1);
    assert_eq!(queued["items"][0]["automatic"], true);
    let failed_id = queued["items"][0]["id"].as_str().unwrap().to_owned();
    p.mode.store(1, Ordering::SeqCst);
    model_job(&h).await;
    let failed = runs(&h, &owner, &base).await;
    assert_eq!(failed["items"][0]["state"], "failed");
    assert!(failed["items"][0]["request_id"].is_string());
    assert!(!worker::run_once(&h.state, "model").await.unwrap());
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    let retry = ok(
        &h,
        "POST",
        &format!("{base}/learning/{failed_id}/retry"),
        &owner,
        json!({"operation_id":null}),
    )
    .await;
    assert_ne!(retry["id"], failed["items"][0]["id"]);
    p.mode.store(0, Ordering::SeqCst);
    let restarted = AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap();
    assert!(worker::run_once(&restarted, "model").await.unwrap());
    assert_eq!(
        runs(&h, &owner, &base).await["items"][0]["state"],
        "succeeded"
    );
    // Reprocessing the same retained source cannot enqueue another automatic run.
    ok(
        &h,
        "POST",
        &format!(
            "{base}/sources/{}/process",
            original["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert!(worker::run_once(&h.state, "capture").await.unwrap());
    assert_eq!(runs(&h, &owner, &base).await["total"], 2);
    allow(&h, &owner, &base, |policy| {
        policy["automatic_learning"] = json!(false)
    })
    .await;
    let atomic = source(&h, &owner, &base, "Amber.port = 9191\n").await;
    *p.candidates.lock().unwrap() = json!({"claims":[candidate("Amber","port","9191",1)]});
    learn(&h, &owner, &base, &atomic).await;
    sqlx::query("ALTER TABLE mutation_audit ADD CONSTRAINT model_publication_fault CHECK(action<>'claim.learn') NOT VALID").execute(&h.admin).await.unwrap();
    model_job(&h).await;
    assert_eq!(runs(&h, &owner, &base).await["items"][0]["state"], "failed");
    let absent: i64 =
        sqlx::query_scalar("SELECT count(*) FROM claim_revisions WHERE value_key='9191'")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(absent, 0);
    sqlx::query("ALTER TABLE mutation_audit DROP CONSTRAINT model_publication_fault")
        .execute(&h.admin)
        .await
        .unwrap();
    let doomed = source(&h, &owner, &base, "Amber.port = 9292\n").await;
    *p.candidates.lock().unwrap() = json!({"claims":[candidate("Amber","port","9292",1)]});
    let run = learn(&h, &owner, &base, &doomed).await;
    let job = worker::claim(&h.state.pool, "model")
        .await
        .unwrap()
        .unwrap();
    p.delay.store(250, Ordering::SeqCst);
    let count = p.calls.load(Ordering::SeqCst) + 1;
    let state = h.state.clone();
    let pending = tokio::spawn(async move { worker::execute(&state, &job).await });
    wait_calls(&p, count).await;
    let target = json!({"kind":"source","id":doomed["id"]});
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
    pending.await.unwrap().unwrap();
    let removed = runs(&h, &owner, &base).await;
    assert_eq!(removed["items"][0]["id"], run["id"]);
    assert_eq!(removed["items"][0]["state"], "removed");
    let absent: i64 =
        sqlx::query_scalar("SELECT count(*) FROM claim_revisions WHERE value_key='9292'")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(absent, 0);
    let suppressed: bool =
        sqlx::query_scalar("SELECT suppressed FROM model_requests WHERE operation_id=$1")
            .bind(Uuid::parse_str(run["id"].as_str().unwrap()).unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(suppressed);
    assert!(
        h.call(
            "POST",
            &format!("{base}/learning/{}/retry", run["id"].as_str().unwrap()),
            Some(&owner),
            json!({"operation_id":null})
        )
        .await
        .0
        .is_client_error()
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn provider_policy_budget_roles_and_real_http_gateway() {
    let (h, owner, base, p, server) = setup().await;
    let check = format!("{base}/models/check");
    let operation = Uuid::new_v4();
    assert_eq!(
        h.call(
            "POST",
            &check,
            Some(&owner),
            json!({"operation_id":operation})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    let initial = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(initial["installed"]["credentials_present"], true);
    assert_eq!(initial["current"]["policy"]["enabled"], false);
    let (writer_id, writer) = h.fixture_member().await;
    let (reader_id, reader) = h.fixture_member().await;
    let (_, foreign) = h.fixture_member().await;
    for (id, role) in [(writer_id, "writer"), (reader_id, "reader")] {
        ok(
            &h,
            "PUT",
            &format!("{base}/grants/{id}"),
            &owner,
            json!({"role":role}),
        )
        .await;
    }
    for login in [&writer, &reader, &foreign] {
        assert!(h.call("PUT",&format!("{base}/models/policy"),Some(login),json!({"base_change":initial["current"]["change_id"],"policy":initial["current"]["policy"]})).await.0.is_client_error());
    }
    let granted = allow(&h, &owner, &base, |_| {}).await;
    let pair = ok(
        &h,
        "POST",
        &check,
        &owner,
        json!({"operation_id":operation}),
    )
    .await;
    assert_eq!(pair["requests"].as_array().unwrap().len(), 2);
    assert_eq!(pair["requests"][1]["dimensions"], 3072);
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    let replay = ok(
        &h,
        "POST",
        &check,
        &owner,
        json!({"operation_id":operation}),
    )
    .await;
    assert_eq!(replay["requests"][0]["id"], pair["requests"][0]["id"]);
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    let usage = ok(
        &h,
        "GET",
        &format!("{base}/models/usage"),
        &reader,
        Value::Null,
    )
    .await;
    assert_eq!(usage["charged_tokens"], 46);
    assert_eq!(usage["in_flight"], 0);
    assert!(usage["requests"][0].get("call_token").is_none());
    assert!(
        h.call(
            "GET",
            &format!("{base}/models/usage"),
            Some(&foreign),
            Value::Null
        )
        .await
        .0
        .is_client_error()
    );
    let mut wrong = granted["policy"].clone();
    wrong["text_model"] = json!("gpt-4.1-mini");
    assert!(
        h.call(
            "PUT",
            &format!("{base}/models/policy"),
            Some(&owner),
            json!({"base_change":granted["change_id"],"policy":wrong})
        )
        .await
        .0
        .is_client_error()
    );
    allow(&h, &owner, &base, |policy| {
        policy["daily_token_limit"] = json!(1000)
    })
    .await;
    assert_eq!(
        h.call(
            "POST",
            &check,
            Some(&owner),
            json!({"operation_id":Uuid::new_v4()})
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    allow(&h, &owner, &base, |policy| {
        policy["content_classes"] = json!(["document"])
    })
    .await;
    assert_eq!(
        h.call(
            "POST",
            &check,
            Some(&owner),
            json!({"operation_id":Uuid::new_v4()})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    let settings = ok(
        &h,
        "GET",
        &format!("{base}/models/policy/history"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(settings.as_array().unwrap().len(), 3);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn learning_literal_acceptance_interpretation_conflict_rejection_and_replay() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |policy| {
        policy["acceptance"] = literal_rule()
    })
    .await;
    let original = source(
        &h,
        &owner,
        &base,
        "Amber.port = 8080\nA friendly service is discussed.\n",
    )
    .await;
    *p.candidates.lock().unwrap() = json!({"claims":[candidate("Amber","port","8080",1),candidate("Amber","description","friendly",2)]});
    let run = learn(&h, &owner, &base, &original).await;
    model_job(&h).await;
    let result = runs(&h, &owner, &base).await;
    assert_eq!(result["items"][0]["state"], "succeeded");
    assert_eq!(result["items"][0]["accepted"], 1);
    assert_eq!(result["items"][0]["proposed"], 1);
    let claims = ok(
        &h,
        "GET",
        &format!("{base}/claims?mode=history"),
        &owner,
        Value::Null,
    )
    .await;
    let literal = claims["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["revision"]["content"]["predicate"] == "port")
        .unwrap();
    assert_eq!(literal["revision"]["review"], "accepted");
    assert!(literal["revision"]["reviewer_id"].is_null());
    assert_eq!(literal["revision"]["origin"], "model_extracted");
    assert_eq!(
        literal["revision"]["derivation"]["requested_model"],
        "gpt-5.6-luna"
    );
    assert_eq!(literal["eligibility"]["strict_accepted"], true);
    assert_eq!(literal["eligibility"]["strict_operational"], false);
    let id = literal["revision"]["claim_id"].as_str().unwrap();
    let r = literal["revision"].clone();
    ok(&h,"POST",&format!("{base}/claims/{id}/review"),&owner,json!({"base_revision":r["id"],"action":"reject","reason":"Synthetic rejection proof","content":null,"revalidation_basis":null})).await;
    let another = source(&h, &owner, &base, "Amber.port = 8080\nAmber.port = 9090\n").await;
    *p.candidates.lock().unwrap() =
        json!({"claims":[candidate("Amber","port","8080",1),candidate("Amber","port","9090",2)]});
    learn(&h, &owner, &base, &another).await;
    model_job(&h).await;
    let result = runs(&h, &owner, &base).await;
    assert_eq!(result["items"][0]["blocked"], 1);
    assert_eq!(result["items"][0]["accepted"], 0);
    assert_eq!(result["items"][0]["conflicting"], 1);
    let denied = h
        .call(
            "POST",
            &format!("{base}/jobs/{}/retry", run["job_id"].as_str().unwrap()),
            Some(&owner),
            Value::Null,
        )
        .await;
    assert!(denied.0.is_client_error());
    learn(&h, &owner, &base, &another).await;
    model_job(&h).await;
    let result = runs(&h, &owner, &base).await;
    assert_eq!(result["items"][0]["reused"], 2);
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    server.abort();
    h.finish().await;
}
