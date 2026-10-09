use super::*;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

#[derive(Default)]
struct RoutedProvider {
    list_calls: AtomicUsize,
    calls: AtomicUsize,
    mode: AtomicUsize,
    bodies: Mutex<Vec<Value>>,
}
async fn list(State(p): State<Arc<RoutedProvider>>, headers: HeaderMap) -> Json<Value> {
    assert_eq!(
        headers["authorization"],
        "Bearer synthetic-openrouter-credential"
    );
    p.list_calls.fetch_add(1, Ordering::SeqCst);
    Json(
        json!({"data":[{"id":"z-ai/glm-5.3-flash"},{"id":"z-ai/glm-4.7-flash"},{"id":"openai/gpt-6-luna"},{"id":"qwen/qwen3-embedding-8b"},{"id":"~z-ai/glm-flash-latest"}]}),
    )
}
async fn routed(
    State(p): State<Arc<RoutedProvider>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    assert_eq!(
        headers["authorization"],
        "Bearer synthetic-openrouter-credential"
    );
    assert_eq!(body["provider"]["allow_fallbacks"], false);
    p.calls.fetch_add(1, Ordering::SeqCst);
    p.bodies.lock().unwrap().push(body.clone());
    if body["encoding_format"] == "float" {
        assert_eq!(body["model"], "qwen/qwen3-embedding-8b");
        let input = body["input"].as_array().unwrap();
        let dims = body["dimensions"].as_u64().unwrap() as usize;
        let data: Vec<Value> = input
            .iter()
            .enumerate()
            .map(|(i, _)| json!({"index":i,"embedding":vec![0.01;dims]}))
            .collect();
        Json(json!({"model":"Qwen/Qwen3-Embedding-8B","data":data,
            "usage":{"prompt_tokens":12,"total_tokens":12,"cost":0.00000012}}))
        .into_response()
    } else {
        assert!(matches!(
            body["model"].as_str(),
            Some("z-ai/glm-5.3-flash" | "z-ai/glm-4.7-flash" | "openai/gpt-6-luna")
        ));
        assert_eq!(body["provider"]["require_parameters"], true);
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        if body["model"] != "openai/gpt-6-luna" {
            if body["model"] == "z-ai/glm-5.3-flash" {
                assert_eq!(body["provider"]["only"], json!(["deepinfra/fp4"]));
                assert!(body["provider"].get("sort").is_none());
            } else {
                assert_eq!(body["provider"]["sort"], "throughput");
            }
            assert_eq!(
                body["provider"]["max_price"],
                json!({"prompt":0.15,"completion":0.5})
            );
        } else {
            assert!(body["provider"].get("sort").is_none());
            assert!(body["provider"].get("max_price").is_none());
        }
        if body["model"] == "z-ai/glm-5.3-flash" {
            assert_eq!(body["reasoning"]["effort"], "low");
        } else if body["model"] == "openai/gpt-6-luna" {
            assert_eq!(body["reasoning"]["effort"], "none");
        } else {
            assert_eq!(body["reasoning"]["enabled"], false);
        }
        assert!(body.get("input").is_none());
        assert_eq!(body["messages"][0]["role"], "system");
        let content = body["messages"][1]["content"].as_str().unwrap();
        assert!(content.contains("Amber"));
        let mode = p.mode.load(Ordering::SeqCst);
        if mode == 3 {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({
                "error":{"message":"Synthetic provider refusal"},
                "usage":{"prompt_tokens":11,"completion_tokens":0,"total_tokens":11,"cost":0.000004}
            }))).into_response();
        }
        if mode == 4 {
            return (StatusCode::TOO_MANY_REQUESTS, "Synthetic non-JSON error").into_response();
        }
        Json(
            json!({"model":if mode==1 {json!("other/unapproved")} else {body["model"].clone()},
            "choices":[{"finish_reason":if mode==2 {"length"} else {"stop"},
                "message":{"content":"{\"service\":\"Amber\",\"environment\":\"test\"}"}}],
            "usage":{"prompt_tokens":23,"completion_tokens":37,"total_tokens":60,
                "completion_tokens_details":{"reasoning_tokens":20},"cost":0.00002195}}),
        )
        .into_response()
    }
}
async fn configure(h: &mut Harness) -> (Arc<RoutedProvider>, tokio::task::JoinHandle<()>) {
    let p = Arc::new(RoutedProvider::default());
    let router = Router::new()
        .route("/api/v1/models", axum::routing::get(list))
        .route("/api/v1/embeddings/models", axum::routing::get(list))
        .route("/api/v1/chat/completions", post(routed))
        .route("/api/v1/embeddings", post(routed))
        .with_state(p.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/api/v1", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut config = (*h.state.config).clone();
    config.models.openrouter_endpoint = endpoint;
    config.models.openrouter_key = Some("synthetic-openrouter-credential".into());
    h.state = AppState::new(h.state.pool.clone(), config).unwrap();
    h.router = app(h.state.clone());
    (p, task)
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn openrouter_selection_rebuild_wire_usage_replay_and_failure_fences() {
    let (mut h, owner, base, openai, oa_server) = setup().await;
    let (p, server) = configure(&mut h).await;
    let settings = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(settings["providers"].as_array().unwrap().len(), 2);
    let mut policy = settings["current"]["policy"].clone();
    policy["provider"] = json!("openrouter");
    policy["text_model"] = json!("z-ai/glm-5.3-flash");
    policy["embedding_model"] = json!("qwen/qwen3-embedding-8b");
    policy["embedding_dimensions"] = json!(1024);
    policy["enabled"] = json!(true);
    let mut body = json!({"base_change":settings["current"]["change_id"],"policy":policy});
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/models/policy"),
            Some(&owner),
            body.clone()
        )
        .await
        .1["code"],
        "model_catalogue_stale"
    );
    let passive = ok(
        &h,
        "GET",
        &format!("{base}/models/catalogue?provider=openrouter"),
        &owner,
        Value::Null,
    )
    .await;
    assert!(passive["observed_at"].is_null());
    assert_eq!(p.list_calls.load(Ordering::SeqCst), 0);
    ok(
        &h,
        "POST",
        &format!("{base}/models/catalogue?provider=openrouter"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(p.list_calls.load(Ordering::SeqCst), 2);
    // Provider-specific discovery cannot make another provider's cache fresh.
    let oa = ok(
        &h,
        "GET",
        &format!("{base}/models/catalogue?provider=openai"),
        &owner,
        Value::Null,
    )
    .await;
    assert!(oa["observed_at"].is_null());
    assert_eq!(openai.catalogue_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/models/policy"),
            Some(&owner),
            body.clone()
        )
        .await
        .1["code"],
        "embedding_rebuild_required"
    );
    body["rebuild_embeddings"] = json!(true);
    ok(&h, "PUT", &format!("{base}/models/policy"), &owner, body).await;
    let selected = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(selected["installed"]["provider"], "openrouter");
    let profile: (String, i32) =
        sqlx::query_as("SELECT provider,dimensions FROM semantic_profiles WHERE brain_id=$1")
            .bind(context(&h, &base).await.brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(profile, ("openrouter".into(), 1024));
    let input = json!({"operation_id":Uuid::new_v4()});
    let result = ok(
        &h,
        "POST",
        &format!("{base}/models/check"),
        &owner,
        input.clone(),
    )
    .await;
    assert_eq!(result["requests"][0]["provider"], "openrouter");
    assert_eq!(result["requests"][0]["output_tokens"], 37);
    assert_eq!(result["requests"][0]["cost_usd"], 0.00002195);
    assert_eq!(result["requests"][1]["cost_usd"], 0.00000012);
    assert!(
        p.bodies.lock().unwrap()[1]["input"][0]
            .as_str()
            .unwrap()
            .starts_with("Instruct:")
    );
    ok(&h, "POST", &format!("{base}/models/check"), &owner, input).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        2,
        "Replay must not resubmit a paid call"
    );
    for (mode, code) in [
        (1, "provider_shape"),
        (2, "provider_incomplete"),
        (3, "provider_rate_limited"),
        (4, "provider_rate_limited"),
    ] {
        p.mode.store(mode, Ordering::SeqCst);
        let operation = Uuid::new_v4();
        let failed = h
            .call(
                "POST",
                &format!("{base}/models/check"),
                Some(&owner),
                json!({"operation_id":operation}),
            )
            .await;
        assert_eq!(failed.1["code"], code);
        let usage = ok(
            &h,
            "GET",
            &format!("{base}/models/usage"),
            &owner,
            Value::Null,
        )
        .await;
        let receipt = usage["requests"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["operation_id"] == operation.to_string())
            .unwrap();
        assert_eq!(receipt["state"], "failed");
        if mode == 3 {
            assert_eq!(receipt["cost_usd"], 0.000004);
            assert_eq!(receipt["charged_tokens"], 11);
        } else if mode == 4 {
            assert!(receipt["cost_usd"].is_null());
            assert!(receipt["charged_tokens"].as_i64().unwrap() > 0);
        }
    }
    assert_eq!(openai.calls.load(Ordering::SeqCst), 0);
    p.mode.store(0, Ordering::SeqCst);
    allow(&h, &owner, &base, |policy| {
        policy["text_model"] = json!("z-ai/glm-4.7-flash");
        policy["automatic_embedding"] = json!(true);
    })
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/models/check"),
        &owner,
        json!({"operation_id":Uuid::new_v4()}),
    )
    .await;
    allow(&h, &owner, &base, |policy| {
        policy["text_model"] = json!("openai/gpt-6-luna");
    })
    .await;
    let luna_check = json!({"operation_id":Uuid::new_v4()});
    let luna = ok(
        &h,
        "POST",
        &format!("{base}/models/check"),
        &owner,
        luna_check.clone(),
    )
    .await;
    assert_eq!(luna["requests"][0]["model"], "openai/gpt-6-luna");
    assert_eq!(luna["requests"][0]["cost_usd"], 0.00002195);
    let after_luna = p.calls.load(Ordering::SeqCst);
    ok(
        &h,
        "POST",
        &format!("{base}/models/check"),
        &owner,
        luna_check,
    )
    .await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        after_luna,
        "Luna replay must not resubmit"
    );
    source(
        &h,
        &owner,
        &base,
        "A synthetic retained document uses Rust.",
    )
    .await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let cx = context(&h, &base).await;
    assert!(
        recollect_server::semantic::maintain_brain(&h.state, cx.brain, cx.actor)
            .await
            .is_ok()
    );
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let index = ok(&h, "GET", &format!("{base}/semantic"), &owner, Value::Null).await;
    assert!(
        index["counts"]["ready"].as_i64().unwrap() > 0,
        "The Qwen alias must pass durable vector publication: {index}"
    );
    let automation = ok(
        &h,
        "GET",
        &format!("{base}/automation"),
        &owner,
        Value::Null,
    )
    .await;
    let adopted = ok(&h, "PUT", &format!("{base}/automation"), &owner,
        json!({"model_change":automation["models"]["current"]["change_id"], "capture_change":automation["capture"]["change_id"]})).await;
    assert_eq!(
        adopted["models"]["current"]["policy"]["provider"],
        "openrouter"
    );
    assert_eq!(
        adopted["models"]["current"]["policy"]["embedding_model"],
        "qwen/qwen3-embedding-8b"
    );
    assert_eq!(
        adopted["models"]["current"]["policy"]["embedding_dimensions"],
        1024
    );
    assert_eq!(
        adopted["models"]["current"]["policy"]["text_model"],
        "openai/gpt-6-luna"
    );
    let after = ok(&h, "GET", &format!("{base}/semantic"), &owner, Value::Null).await;
    assert_eq!(
        after["profile"]["id"], index["profile"]["id"],
        "Adopting automation must retain the selected provider and index generation"
    );
    let before = p.calls.load(Ordering::SeqCst);
    let mut config = (*h.state.config).clone();
    config.models.openrouter_key = None;
    h.state = AppState::new(h.state.pool.clone(), config).unwrap();
    h.router = app(h.state.clone());
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/models/check"),
            Some(&owner),
            json!({"operation_id":Uuid::new_v4()})
        )
        .await
        .1["code"],
        "model_credentials_missing"
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        before,
        "Missing credentials stop before dispatch"
    );
    server.abort();
    oa_server.abort();
    h.finish().await;
}
