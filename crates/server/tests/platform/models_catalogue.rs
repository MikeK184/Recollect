use super::*;
use recollect_server::semantic;

async fn catalogue(h: &Harness, owner: &Login, base: &str, refresh: bool) -> Value {
    ok(
        h,
        if refresh { "POST" } else { "GET" },
        &format!("{base}/models/catalogue"),
        owner,
        Value::Null,
    )
    .await
}
async fn policy(h: &Harness, owner: &Login, base: &str) -> Value {
    ok(
        h,
        "GET",
        &format!("{base}/models/policy"),
        owner,
        Value::Null,
    )
    .await
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn model_catalogue_refresh_is_explicit_cached_scoped_and_bounded() {
    let (mut h, owner, base, p, server) = setup().await;
    let empty = catalogue(&h, &owner, &base, false).await;
    assert!(empty["observed_at"].is_null());
    assert!(
        empty["models"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["selectable"] == false)
    );
    assert_eq!(p.catalogue_calls.load(Ordering::SeqCst), 0);
    let listed = catalogue(&h, &owner, &base, true).await;
    assert_eq!(listed["stale"], false);
    let rows = listed["models"].as_array().unwrap();
    assert!(!rows.iter().any(|m| matches!(
        m["id"].as_str(),
        Some("gpt-4.1-nano" | "gpt-realtime" | "unknown-future-model")
    )));
    let mini = rows.iter().find(|m| m["id"] == "gpt-4.1-mini").unwrap();
    assert_eq!(mini["input_usd_per_million"], 0.4);
    assert_eq!(mini["cached_input_usd_per_million"], 0.1);
    assert_eq!(mini["output_usd_per_million"], 1.6);
    assert_eq!(mini["checked_on"], "2026-10-05");
    assert!(
        mini["source_url"]
            .as_str()
            .unwrap()
            .starts_with("https://developers.openai.com/")
    );
    catalogue(&h, &owner, &base, true).await;
    assert_eq!(p.catalogue_calls.load(Ordering::SeqCst), 1);
    let no_csrf = Login {
        cookie: owner.cookie.clone(),
        csrf: String::new(),
    };
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/models/catalogue"),
            Some(&no_csrf),
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (reader_id, reader) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    catalogue(&h, &reader, &base, false).await;
    assert!(
        h.call(
            "POST",
            &format!("{base}/models/catalogue"),
            Some(&reader),
            Value::Null
        )
        .await
        .0
        .is_client_error()
    );
    let (_, foreign) = h.fixture_member().await;
    assert!(
        h.call(
            "GET",
            &format!("{base}/models/catalogue"),
            Some(&foreign),
            Value::Null
        )
        .await
        .0
        .is_client_error()
    );
    assert_eq!(p.catalogue_calls.load(Ordering::SeqCst), 1);
    for (mode, code) in [
        (1, "model_catalogue_rate_limited"),
        (11, "model_catalogue_shape"),
        (12, "model_catalogue_body_limit"),
    ] {
        // Restart-local cache; provider endpoint/data and canonical Brain stay intact.
        h.state = AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap();
        h.router = app(h.state.clone());
        p.mode.store(mode, Ordering::SeqCst);
        let failed = catalogue(&h, &owner, &base, true).await;
        assert_eq!(failed["error_code"], code);
        assert_eq!(failed["stale"], true);
        assert!(!failed.to_string().contains("provider secret"));
        let calls = p.catalogue_calls.load(Ordering::SeqCst);
        catalogue(&h, &owner, &base, true).await;
        assert_eq!(
            p.catalogue_calls.load(Ordering::SeqCst),
            calls,
            "Failures back off"
        );
    }
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        0,
        "A model list never runs paid probes"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn model_selection_rebuild_is_atomic_idempotent_dimension_safe_and_policy_fenced() {
    let (mut h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    let src = source(
        &h,
        &owner,
        &base,
        "Amber uses a stable synthetic endpoint. NEAR_VECTOR",
    )
    .await;
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(true);
    })
    .await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    assert!(
        semantic::maintain_brain(&h.state, cx.brain, cx.actor)
            .await
            .is_ok()
    );
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let before = ok(&h, "GET", &format!("{base}/semantic"), &owner, Value::Null).await;
    assert_eq!(before["counts"]["ready"], 1);
    let initial = policy(&h, &owner, &base).await;
    let mut next = initial["current"]["policy"].clone();
    next["text_model"] = json!("gpt-4.1-mini");
    let body = json!({"base_change":initial["current"]["change_id"],"policy":next});
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
    catalogue(&h, &owner, &base, true).await;
    let mut unavailable = body.clone();
    unavailable["policy"]["text_model"] = json!("gpt-4.1-mini-2025-04-14");
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/models/policy"),
            Some(&owner),
            unavailable
        )
        .await
        .1["code"],
        "model_unavailable"
    );
    let mut unsupported = body.clone();
    unsupported["policy"]["text_model"] = json!("unknown-future-model");
    // Even a configured legacy ID listed by the account cannot become a new
    // selection unless its adapter compatibility has been reviewed.
    let mut configured = (*h.state.config).clone();
    configured.models.text_model = "unknown-future-model".into();
    h.state = AppState::new(h.state.pool.clone(), configured).unwrap();
    h.router = app(h.state.clone());
    catalogue(&h, &owner, &base, true).await;
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/models/policy"),
            Some(&owner),
            unsupported
        )
        .await
        .1["code"],
        "model_configuration_changed"
    );
    let selected = ok(&h, "PUT", &format!("{base}/models/policy"), &owner, body).await;
    assert_eq!(
        ok(&h, "GET", &format!("{base}/semantic"), &owner, Value::Null).await["profile"]["id"],
        before["profile"]["id"]
    );
    // The same selected text model can perform the existing strict-output gateway check.
    let check = ok(
        &h,
        "POST",
        &format!("{base}/models/check"),
        &owner,
        json!({"operation_id":Uuid::new_v4()}),
    )
    .await;
    assert_eq!(check["requests"][0]["model"], "gpt-4.1-mini");
    let mut next = selected["policy"].clone();
    next["embedding_model"] = json!("text-embedding-3-small");
    next["embedding_dimensions"] = json!(1536);
    next["enabled"] = json!(false);
    next["automatic_learning"] = json!(false);
    next["automatic_embedding"] = json!(false);
    let mut body = json!({"base_change":selected["change_id"],"policy":next});
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
    let path = format!("{base}/models/policy");
    let (status, saved, _) = h
        .keyed(
            "PUT",
            &path,
            Some(&owner),
            body.clone(),
            Some("small-rebuild"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let blocked = ok(&h, "GET", &format!("{base}/semantic"), &owner, Value::Null).await;
    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["profile"]["model"], "text-embedding-3-small");
    assert_eq!(blocked["profile"]["dimensions"], 1536);
    assert_ne!(blocked["profile"]["id"], before["profile"]["id"]);
    assert_eq!(blocked["counts"]["ready"], 0);
    let calls = p.calls.load(Ordering::SeqCst);
    assert_eq!(
        semantic::maintain_brain(&h.state, cx.brain, cx.actor)
            .await
            .unwrap_or_else(|e| panic!("Semantic maintenance: {}", e.1)),
        0
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), calls);
    let replay = h
        .keyed("PUT", &path, Some(&owner), body, Some("small-rebuild"))
        .await;
    assert_eq!(replay.0, StatusCode::OK);
    assert_eq!(replay.1["change_id"], saved["change_id"]);
    assert_eq!(
        ok(&h, "GET", &format!("{base}/semantic"), &owner, Value::Null).await["profile"]["id"],
        blocked["profile"]["id"]
    );
    let vectors: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM semantic_entries WHERE brain_id=$1 AND embedding IS NOT NULL",
    )
    .bind(cx.brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(vectors, 0);
    let recall = ok(&h,"POST",&format!("{base}/recall"),&owner,json!({"query":"Amber","selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"mode":"investigation","channels":["lexical"],"limit":10})).await;
    assert!(
        !recall["context"]["items"].as_array().unwrap().is_empty(),
        "Lexical access survives rebuild"
    );
    let mut body = json!({"base_change":saved["change_id"],"policy":saved["policy"]});
    body["policy"]["enabled"] = json!(true);
    body["policy"]["automatic_embedding"] = json!(true);
    ok(&h, "PUT", &path, &owner, body.clone()).await;
    assert_eq!(
        h.call("PUT", &path, Some(&owner), body).await.1["code"],
        "model_policy_changed"
    );
    semantic::maintain_brain(&h.state, cx.brain, cx.actor)
        .await
        .unwrap_or_else(|e| panic!("Semantic maintenance: {}", e.1));
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let dimensions: i32 = sqlx::query_scalar("SELECT vector_dims(embedding) FROM semantic_entries WHERE brain_id=$1 AND profile_id=$2 AND state='ready'")
        .bind(cx.brain).bind(blocked["profile"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    assert_eq!(dimensions, 1536);
    assert!(sqlx::query("UPDATE semantic_entries SET embedding='[1,2]'::vector WHERE brain_id=$1 AND state='ready'").bind(cx.brain).execute(&h.admin).await.is_err());
    assert!(
        sqlx::query("UPDATE semantic_profiles SET dimensions=512 WHERE brain_id=$1")
            .bind(cx.brain)
            .execute(&h.admin)
            .await
            .is_err()
    );
    // Erasure trigger remains independent of dimension/profile selection.
    sqlx::query("UPDATE source_versions SET privacy_state='erased' WHERE id=$1")
        .bind(
            src["version"]["id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap(),
        )
        .execute(&h.admin)
        .await
        .unwrap();
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM semantic_entries WHERE brain_id=$1 AND embedding IS NOT NULL",
    )
    .bind(cx.brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(remaining, 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn embedding_selection_suppresses_old_inflight_output_and_rebuilds_new_dimensions() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    source(&h, &owner, &base, "A bounded synthetic Amber source.").await;
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(true);
    })
    .await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    semantic::maintain_brain(&h.state, cx.brain, cx.actor)
        .await
        .unwrap_or_else(|e| panic!("Semantic maintenance: {}", e.1));
    p.delay.store(1000, Ordering::SeqCst);
    let state = h.state.clone();
    let in_flight = tokio::spawn(async move { worker::run_once(&state, "model").await.unwrap() });
    wait_calls(&p, 1).await;
    catalogue(&h, &owner, &base, true).await;
    let current = policy(&h, &owner, &base).await;
    let mut next = current["current"]["policy"].clone();
    next["embedding_model"] = json!("text-embedding-3-small");
    next["embedding_dimensions"] = json!(256);
    ok(&h,"PUT",&format!("{base}/models/policy"),&owner,json!({"base_change":current["current"]["change_id"],"policy":next,"rebuild_embeddings":true})).await;
    assert!(in_flight.await.unwrap());
    let suppressed: bool = sqlx::query_scalar("SELECT bool_and(suppressed) FROM model_requests WHERE brain_id=$1 AND model='text-embedding-3-large'").bind(cx.brain).fetch_one(&h.admin).await.unwrap();
    assert!(
        suppressed,
        "Already-transmitted old generation cannot publish"
    );
    let old_vectors: i64 = sqlx::query_scalar("SELECT count(*) FROM semantic_entries e JOIN semantic_profiles p ON p.id=e.profile_id WHERE e.brain_id=$1 AND p.model='text-embedding-3-large' AND embedding IS NOT NULL").bind(cx.brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(old_vectors, 0);
    p.delay.store(0, Ordering::SeqCst);
    assert_eq!(
        semantic::maintain_brain(&h.state, cx.brain, cx.actor)
            .await
            .unwrap_or_else(|e| panic!("Semantic maintenance: {}", e.1)),
        1
    );
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let rebuilt = ok(&h, "GET", &format!("{base}/semantic"), &owner, Value::Null).await;
    assert_eq!(rebuilt["state"], "ready");
    assert_eq!(rebuilt["profile"]["dimensions"], 256);
    assert_eq!(rebuilt["counts"]["ready"], 1);
    let recall = ok(&h,"POST",&format!("{base}/recall"),&owner,json!({"query":"Amber","channels":["semantic","lexical"],"semantic_request_id":Uuid::new_v4()})).await;
    assert!(!recall["context"]["items"].as_array().unwrap().is_empty());
    assert_eq!(recall["semantic"]["profile"]["dimensions"], 256);
    server.abort();
    h.finish().await;
}
