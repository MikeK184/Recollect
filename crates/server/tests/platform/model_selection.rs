//! An installation without a text model: nothing is transmitted until a Brain
//! administrator chooses one, and saved policies are never rewritten.
use super::*;

/// Builds the installation with the provider fixture but no operator text model.
async fn unconfigured() -> (Harness, Login, Arc<Provider>, tokio::task::JoinHandle<()>) {
    let mut h = Harness::new().await;
    let (provider, task) = configure_provider(&mut h).await;
    let mut config = (*h.state.config).clone();
    config.models.text_model = None;
    h.state = AppState::new(h.state.pool.clone(), config).unwrap();
    h.router = app(h.state.clone());
    let owner = h.login().await;
    (h, owner, provider, task)
}

fn adoption(settings: &Value, model: Option<&str>) -> Value {
    let mut input = json!({
        "model_change": settings["models"]["current"]["change_id"],
        "capture_change": settings["capture"]["change_id"]
    });
    if let Some(model) = model {
        input["text_model"] = json!(model);
    }
    input
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn managed_brain_without_an_installation_model_captures_but_transmits_nothing_until_a_model_is_chosen()
 {
    let (h, owner, p, server) = unconfigured().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"No installation model","managed_memory":true}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let path = format!("{base}/automation");
    let settings = ok(&h, "GET", &path, &owner, Value::Null).await;
    // The installation reports no text model; the Brain has not chosen one.
    assert!(settings["models"]["installed"]["text_model"].is_null());
    let policy = &settings["models"]["current"]["policy"];
    assert_eq!(policy["text_model"], "");
    // Capture is on and the preset's limits are saved...
    assert_eq!(settings["capture"]["policy"]["enabled"], true);
    assert_eq!(settings["capture"]["policy"]["managed_tools"], true);
    assert_eq!(policy["daily_token_limit"], 1_000_000);
    assert_eq!(policy["max_concurrent"], 2);
    assert_eq!(policy["max_input_bytes"], 32 * 1024);
    assert_eq!(policy["max_output_tokens"], 4096);
    assert_eq!(policy["autonomous_memory"], true);
    assert!(
        policy["purposes"]
            .as_array()
            .unwrap()
            .contains(&json!("answering"))
    );
    // ...but model transmission stays off.
    for flag in ["enabled", "automatic_learning", "automatic_embedding"] {
        assert_eq!(policy[flag], false, "{flag}");
    }
    // The gateway refuses before any credential or provider is involved.
    let (status, denied, _) = h
        .call(
            "POST",
            &format!("{base}/models/check"),
            Some(&owner),
            json!({"operation_id":Uuid::new_v4()}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{denied}");
    assert_eq!(denied["code"], "model_not_selected");
    assert_eq!(denied["message"], "Choose a text model for this Brain");
    // A policy cannot allow transmission without a model.
    let current = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    let mut enabled = current["current"]["policy"].clone();
    enabled["enabled"] = json!(true);
    let (status, rejected, _) = h
        .call(
            "PUT",
            &format!("{base}/models/policy"),
            Some(&owner),
            json!({"base_change":current["current"]["change_id"],"policy":enabled}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{rejected}");
    assert_eq!(rejected["code"], "model_not_selected");
    // Re-applying the preset with no model changes nothing and stays off.
    let again = ok(&h, "PUT", &path, &owner, adoption(&settings, None)).await;
    assert_eq!(again["models"]["current"]["policy"]["enabled"], false);
    assert_eq!(again["models"]["current"]["policy"]["text_model"], "");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM model_requests")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    assert_eq!(p.catalogue_calls.load(Ordering::SeqCst), 0);

    // Choosing a model is checked against a fresh account model list.
    let settings = ok(&h, "GET", &path, &owner, Value::Null).await;
    let (status, stale, _) = h
        .call(
            "PUT",
            &path,
            Some(&owner),
            adoption(&settings, Some("gpt-5.6-luna")),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{stale}");
    assert_eq!(stale["code"], "model_catalogue_stale");
    ok(
        &h,
        "POST",
        &format!("{base}/models/catalogue"),
        &owner,
        Value::Null,
    )
    .await;
    for (model, code, status) in [
        (
            "gpt-4.1-mini-2025-04-14",
            "model_unavailable",
            StatusCode::CONFLICT,
        ),
        (
            "unknown-future-model",
            "model_configuration_changed",
            StatusCode::CONFLICT,
        ),
        ("", "invalid_input", StatusCode::BAD_REQUEST),
    ] {
        let (got, body, _) = h
            .call("PUT", &path, Some(&owner), adoption(&settings, Some(model)))
            .await;
        assert_eq!(got, status, "{model}: {body}");
        assert_eq!(body["code"], code, "{model}");
    }
    let still = ok(&h, "GET", &path, &owner, Value::Null).await;
    assert_eq!(still["models"]["current"], settings["models"]["current"]);
    // A reader cannot choose for the Brain.
    let (reader_id, reader) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    assert_eq!(
        h.call(
            "PUT",
            &path,
            Some(&reader),
            adoption(&settings, Some("gpt-5.6-luna"))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    // The administrator's choice re-applies the preset and enables it.
    let key = Uuid::new_v4().to_string();
    let input = adoption(&settings, Some("gpt-5.6-luna"));
    let (status, chosen, _) = h
        .keyed("PUT", &path, Some(&owner), input.clone(), Some(&key))
        .await;
    assert_eq!(status, StatusCode::OK, "{chosen}");
    let policy = &chosen["models"]["current"]["policy"];
    assert_eq!(policy["text_model"], "gpt-5.6-luna");
    for flag in [
        "enabled",
        "automatic_learning",
        "automatic_embedding",
        "autonomous_memory",
    ] {
        assert_eq!(policy[flag], true, "{flag}");
    }
    assert_eq!(policy["daily_token_limit"], 1_000_000);
    assert_eq!(chosen["capture"]["policy"]["enabled"], true);
    let (status, replay, _) = h.keyed("PUT", &path, Some(&owner), input, Some(&key)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay, chosen);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM model_policies WHERE brain_id=$1 AND policy->>'text_model'='gpt-5.6-luna'"
        )
        .bind(Uuid::parse_str(brain["id"].as_str().unwrap()).unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        1,
        "A replayed choice saves one policy revision"
    );
    // The chosen model now passes the gateway and reaches the provider.
    let check = ok(
        &h,
        "POST",
        &format!("{base}/models/check"),
        &owner,
        json!({"operation_id":Uuid::new_v4()}),
    )
    .await;
    assert_eq!(check["requests"][0]["model"], "gpt-5.6-luna");
    assert!(p.calls.load(Ordering::SeqCst) > 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn legacy_brain_without_a_policy_reports_no_model_and_saved_policies_are_untouched() {
    let (mut h, owner, p, server) = unconfigured().await;
    // A Brain created before any model choice has no saved policy revision.
    let legacy = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Legacy without policy","managed_memory":false}),
    )
    .await;
    let legacy_base = format!("/api/brains/{}", legacy["id"].as_str().unwrap());
    let settings = ok(
        &h,
        "GET",
        &format!("{legacy_base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    assert!(settings["installed"]["text_model"].is_null());
    assert_eq!(settings["current"]["policy"]["text_model"], "");
    assert_eq!(settings["current"]["policy"]["enabled"], false);
    assert!(settings["providers"][1]["text_model"].is_string());
    // Disabled with no model is a valid saved policy (a limit edit needs no model list).
    let mut limits = settings["current"]["policy"].clone();
    limits["daily_token_limit"] = json!(250_000);
    let saved = ok(
        &h,
        "PUT",
        &format!("{legacy_base}/models/policy"),
        &owner,
        json!({"base_change":settings["current"]["change_id"],"policy":limits}),
    )
    .await;
    assert_eq!(saved["policy"]["text_model"], "");

    // A Brain whose administrator already saved a Luna policy keeps working
    // when the installation names no model: the saved policy is not rewritten.
    let mut named = (*h.state.config).clone();
    named.models.text_model = Some("gpt-5.6-luna".into());
    h.state = AppState::new(h.state.pool.clone(), named).unwrap();
    h.router = app(h.state.clone());
    let existing = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Saved Luna policy","managed_memory":true}),
    )
    .await;
    let base = format!("/api/brains/{}", existing["id"].as_str().unwrap());
    let before = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(before["current"]["policy"]["text_model"], "gpt-5.6-luna");
    assert_eq!(before["current"]["policy"]["enabled"], true);
    let mut bare = (*h.state.config).clone();
    bare.models.text_model = None;
    h.state = AppState::new(h.state.pool.clone(), bare).unwrap();
    h.router = app(h.state.clone());
    let after = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    assert!(after["installed"]["text_model"].is_null());
    assert_eq!(after["current"], before["current"]);
    let check = ok(
        &h,
        "POST",
        &format!("{base}/models/check"),
        &owner,
        json!({"operation_id":Uuid::new_v4()}),
    )
    .await;
    assert_eq!(check["requests"][0]["model"], "gpt-5.6-luna");
    assert!(p.calls.load(Ordering::SeqCst) > 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM model_policies WHERE brain_id=$1")
            .bind(Uuid::parse_str(existing["id"].as_str().unwrap()).unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        1,
        "An installation without a model never rewrites a saved policy"
    );
    server.abort();
    h.finish().await;
}
