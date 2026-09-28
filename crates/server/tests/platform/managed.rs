use super::*;

async fn ok(h: &Harness, who: &Login, method: &str, path: &str, body: Value) -> Value {
    let (status, value, _) = h.call(method, path, Some(who), body).await;
    assert_eq!(status, StatusCode::OK, "{path}: {value}");
    value
}
fn adoption(settings: &Value) -> Value {
    json!({"model_change":settings["models"]["current"]["change_id"],"capture_change":settings["capture"]["change_id"]})
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn managed_memory_atomic_adoption_preserves_boundaries_and_defaults() {
    let mut h = Harness::new().await;
    let mut config = (*h.state.config).clone();
    config.models.key = None;
    h.state = AppState::new(h.state.pool.clone(), config).unwrap();
    h.router = app(h.state.clone());
    let owner = h.login().await;
    let create_key = Uuid::new_v4().to_string();
    let (status, brain, _) = h
        .keyed(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Managed fixture"}),
            Some(&create_key),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    // Pre-preset receipts retain exactly their original identity shape.
    let identity: Value = sqlx::query_scalar("SELECT input FROM command_receipts WHERE key=$1")
        .bind(&create_key)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(identity, json!({"name":"Managed fixture","description":""}));
    let (status, replay, _) = h
        .keyed(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Managed fixture", "managed_memory":false}),
            Some(&create_key),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(brain, replay);
    let id = brain["id"].as_str().unwrap();
    let base = format!("/api/brains/{id}");
    let path = format!("{base}/automation");
    let original = ok(&h, &owner, "GET", &path, Value::Null).await;
    assert_eq!(original["models"]["current"]["policy"]["enabled"], false);
    let (member_id, member) = h.fixture_member().await;
    for role in ["reader", "writer"] {
        ok(
            &h,
            &owner,
            "PUT",
            &format!("{base}/grants/{member_id}"),
            json!({"role":role}),
        )
        .await;
        assert_eq!(
            h.call("PUT", &path, Some(&member), adoption(&original))
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    let (_, token) = h.pair_device(&owner, "Managed device denial").await;
    assert_eq!(
        h.bearer("PUT", &path, &token, adoption(&original)).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.bearer(
            "POST",
            "/api/brains",
            &token,
            json!({"name":"Denied device preset","managed_memory":true})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut capture = original["capture"]["policy"].clone();
    capture["excluded_tools"] = json!(["private_tool"]);
    capture["excluded_content"] = json!(["PRIVATE_SENTINEL"]);
    ok(
        &h,
        &owner,
        "PUT",
        &format!("{base}/capture/policy"),
        json!({"base_change":original["capture"]["change_id"],"policy":capture}),
    )
    .await;
    let mut retention = original["retention"]["policy"].clone();
    retention["raw_session_days"] = json!(7);
    ok(
        &h,
        &owner,
        "PUT",
        &format!("{base}/retention"),
        json!({"base_change":original["retention"]["change_id"],"policy":retention}),
    )
    .await;
    // A stale capture head must not even update the otherwise valid model head.
    assert_eq!(
        h.call("PUT", &path, Some(&owner), adoption(&original))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let before = ok(&h, &owner, "GET", &path, Value::Null).await;
    assert_eq!(before["models"], original["models"]);
    let key = Uuid::new_v4().to_string();
    let input = adoption(&before);
    let (status, saved, _) = h
        .keyed("PUT", &path, Some(&owner), input.clone(), Some(&key))
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let (status, replay, _) = h.keyed("PUT", &path, Some(&owner), input, Some(&key)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved, replay);
    assert_eq!(saved["retention"], before["retention"]);
    assert_eq!(
        saved["capture"]["policy"]["excluded_tools"],
        capture["excluded_tools"]
    );
    assert_eq!(
        saved["capture"]["policy"]["excluded_content"],
        capture["excluded_content"]
    );
    assert_eq!(saved["capture"]["policy"]["managed_tools"], true);
    let policy = &saved["models"]["current"]["policy"];
    for flag in [
        "enabled",
        "autonomous_memory",
        "automatic_learning",
        "automatic_embedding",
    ] {
        assert_eq!(policy[flag], true);
    }
    assert_eq!(policy["daily_token_limit"], 1_000_000);
    assert_eq!(policy["max_concurrent"], 2);
    assert_eq!(policy["max_output_tokens"], 4096);
    assert!(
        policy["purposes"]
            .as_array()
            .unwrap()
            .contains(&json!("answering"))
    );
    assert_eq!(saved["models"]["installed"]["credentials_present"], false);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM model_requests")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    let new_brain = ok(
        &h,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":"New managed Brain","managed_memory":true}),
    )
    .await;
    let new_settings = ok(
        &h,
        &owner,
        "GET",
        &format!(
            "/api/brains/{}/automation",
            new_brain["id"].as_str().unwrap()
        ),
        Value::Null,
    )
    .await;
    assert_eq!(
        new_settings["models"]["current"]["policy"]["automatic_embedding"],
        true
    );
    assert_eq!(new_settings["retention"]["policy"]["raw_session_days"], 30);
    assert_eq!(ok(&h, &owner, "GET", &path, Value::Null).await, saved);
    ok(&h, &owner, "PATCH", &base, json!({"archived":true})).await;
    assert_eq!(
        h.call("PUT", &path, Some(&owner), adoption(&saved)).await.0,
        StatusCode::CONFLICT
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn managed_connector_owner_import_is_validated_inert_and_replayable() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (_, member) = h.fixture_member().await;
    let inspection = json!({"name":"Denied metadata discovery","url":"http://127.0.0.1:9/mcp"});
    assert_eq!(
        h.call(
            "POST",
            "/api/mcp/definitions/inspect-http",
            Some(&member),
            inspection
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/mcp/definitions/inspect-http",
            Some(&owner),
            json!({"name":"Invalid target","url":"http://example.invalid/mcp"})
        )
        .await
        .0,
        StatusCode::BAD_GATEWAY
    );
    let manifest: Value =
        serde_json::from_str(include_str!("../fixtures/mcp-catalogue.json")).unwrap();
    let path = "/api/mcp/definitions";
    assert_eq!(
        h.call("POST", path, Some(&member), manifest.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let (_, token) = h.pair_device(&owner, "Connector device denial").await;
    assert_eq!(
        h.bearer("POST", path, &token, manifest.clone()).await.0,
        StatusCode::FORBIDDEN
    );
    let mut bad = manifest.clone();
    bad["configuration_schema"]["$ref"] = json!("https://untrusted.invalid/schema");
    assert_eq!(
        h.call("POST", path, Some(&owner), bad).await.0,
        StatusCode::BAD_REQUEST
    );
    let saved = ok(&h, &owner, "POST", path, manifest.clone()).await;
    assert_eq!(ok(&h, &owner, "POST", path, manifest.clone()).await, saved);
    let mut changed = manifest;
    changed["command"] = json!("/different/executable");
    assert_eq!(
        h.call("POST", path, Some(&owner), changed).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM mutation_audit WHERE action='mcp.definition'"
        )
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_instances")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_calls")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    h.finish().await;
}
