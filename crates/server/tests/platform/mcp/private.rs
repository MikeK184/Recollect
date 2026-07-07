use super::*;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_private_registration_authority_and_exact_outbound_routing() {
    let (h, owner, brain, connection, profile) = setup().await;
    let base = format!("/api/brains/{brain}/mcp");
    let registrations = format!("{base}/private-runners");
    let (device, token) = h.pair_device(&owner, "Private runner host").await;
    let (wrong_device, wrong_token) = h.pair_device(&owner, "Different host").await;
    let (reader_id, caller) = h.fixture_member().await;
    reader(&h, &owner, brain, reader_id).await;
    let input =
        json!({"name":"Private fixture", "device_id":device,"enabled":true,"base_revision":null});
    assert_eq!(
        h.call("POST", &registrations, Some(&caller), input.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.bearer("POST", &registrations, &token, input.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let key = Uuid::new_v4().to_string();
    let (status, registration, _) = h
        .keyed(
            "POST",
            &registrations,
            Some(&owner),
            input.clone(),
            Some(&key),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{registration}");
    assert_eq!(
        h.keyed("POST", &registrations, Some(&owner), input, Some(&key))
            .await
            .1,
        registration
    );
    assert_eq!(registration["eligible"], true);
    assert_eq!(registration["available"], false);
    assert_eq!(
        ok(&h, &caller, "GET", &registrations, Value::Null)
            .await
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let id = registration["id"].as_str().unwrap();
    let reference = format!("private:{id}");
    let route = |action: &str| format!("/api/mcp/runner/{action}?private_runner={id}");
    assert_eq!(
        h.bearer("POST", &route("register"), &wrong_token, Value::Null)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.bearer(
            "POST",
            &format!("/api/mcp/runner/register?private_runner={}", Uuid::new_v4()),
            &token,
            Value::Null
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (status, lease) = h
        .bearer("POST", &route("register"), &token, Value::Null)
        .await;
    assert_eq!(status, StatusCode::OK, "{lease}");
    assert_eq!(lease["runner_reference"], reference);
    let epoch = json!({"epoch":lease["epoch"]});
    assert_eq!(
        ok(&h, &owner, "GET", &registrations, Value::Null).await[0]["available"],
        true
    );
    let mut edit = connection_body(None);
    edit["placement"] = json!("private");
    edit["runner_reference"] = json!(reference);
    edit["base_revision"] = connection["summary"]["revision"].clone();
    let connection = ok(
        &h,
        &owner,
        "PUT",
        &format!(
            "{base}/connections/{}",
            connection["summary"]["id"].as_str().unwrap()
        ),
        edit,
    )
    .await;
    let calls = format!("{base}/calls");
    let request = body(&connection, &profile);
    assert_eq!(
        h.call("POST", &calls, Some(&caller), request.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &username(&h, reader_id).await,
        rights(true, false, false),
    )
    .await;
    let call = ok(&h, &caller, "POST", &calls, request).await;
    // A central or personal executor cannot steal a private Brain queue.
    let central = central(&h).await;
    assert!(checked(central.claim(&h.state).await).plan.is_none());
    let personal = h
        .bearer("POST", "/api/mcp/runner/register", &token, Value::Null)
        .await;
    assert_eq!(personal.0, StatusCode::OK);
    assert_eq!(
        h.bearer(
            "POST",
            "/api/mcp/runner/claim",
            &token,
            json!({"epoch":personal.1["epoch"]})
        )
        .await
        .1["plan"],
        Value::Null
    );
    let (status, claimed) = h
        .bearer("POST", &route("claim"), &token, epoch.clone())
        .await;
    assert_eq!(status, StatusCode::OK, "{claimed}");
    let plan: McpExecutionPlan = serde_json::from_value(claimed["plan"].clone()).unwrap();
    assert_eq!(
        plan.actor_id, reader_id,
        "caller is separate from the host administrator"
    );
    assert_eq!(plan.call_id.to_string(), call["id"].as_str().unwrap());
    let request = serde_json::to_value(attempt(&plan)).unwrap();
    assert_eq!(
        h.bearer("POST", &route("credentials"), &token, request.clone())
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer("POST", &route("credentials"), &wrong_token, request.clone())
            .await
            .0,
        StatusCode::CONFLICT
    );
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &username(&h, reader_id).await,
        rights(false, false, false),
    )
    .await;
    assert_eq!(
        h.bearer("POST", &route("credentials"), &token, request.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &username(&h, reader_id).await,
        rights(true, false, false),
    )
    .await;
    assert_eq!(
        h.bearer("POST", &route("credentials"), &token, request.clone())
            .await
            .0,
        StatusCode::OK
    );
    let mut wrong_brain = request.clone();
    wrong_brain["brain_id"] = json!(Uuid::new_v4());
    assert_eq!(
        h.bearer("POST", &route("credentials"), &token, wrong_brain)
            .await
            .0,
        StatusCode::CONFLICT
    );
    let path = format!("{registrations}/{id}");
    let mut update = json!({"name":"Private renamed","enabled":false,"device_id":device,"base_revision":registration["revision"]});
    let mut moved = update.clone();
    moved["device_id"] = json!(wrong_device);
    assert_eq!(
        h.call("PUT", &path, Some(&owner), moved).await.0,
        StatusCode::BAD_REQUEST
    );
    let key = Uuid::new_v4().to_string();
    let (status, disabled, _) = h
        .keyed("PUT", &path, Some(&owner), update.clone(), Some(&key))
        .await;
    assert_eq!(status, StatusCode::OK, "{disabled}");
    assert_eq!(
        h.keyed("PUT", &path, Some(&owner), update.clone(), Some(&key))
            .await
            .1,
        disabled
    );
    assert_eq!(
        h.call("PUT", &path, Some(&owner), update.clone()).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.bearer("POST", &route("credentials"), &token, request.clone())
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.bearer("POST", &route("heartbeat"), &token, epoch.clone())
            .await
            .0,
        StatusCode::CONFLICT
    );
    update["enabled"] = json!(true);
    assert_eq!(
        ok(&h, &owner, "GET", &format!("{base}/runtime"), Value::Null).await["runners"][0]["available"],
        false,
        "a disabled registration cannot remain available on an old live lease"
    );
    update["base_revision"] = disabled["revision"].clone();
    ok(&h, &owner, "PUT", &path, update).await;
    assert_eq!(
        h.bearer("POST", &route("credentials"), &token, request.clone())
            .await
            .0,
        StatusCode::OK
    );
    // Direct RLS must preserve immutable host binding, independent of the API.
    let mut tx = checked(recollect_server::db::actor_tx(&h.state.pool, reader_id).await);
    assert_eq!(
        sqlx::query("UPDATE mcp_private_runners SET enabled=false WHERE id=$1")
            .bind(Uuid::parse_str(id).unwrap())
            .execute(&mut *tx)
            .await
            .unwrap()
            .rows_affected(),
        0
    );
    tx.rollback().await.unwrap();
    // A second host must retain its own current Brain administrator role.
    ok(
        &h,
        &owner,
        "PUT",
        &format!("/api/brains/{brain}/grants/{reader_id}"),
        json!({"role":"admin"}),
    )
    .await;
    let (secondary, secondary_token) = h.pair_device(&caller, "Secondary private host").await;
    let second=ok(&h,&caller,"POST",&registrations,json!({"name":"Secondary runner","device_id":secondary,"enabled":true,"base_revision":null})).await;
    let secondary_route = format!(
        "/api/mcp/runner/register?private_runner={}",
        second["id"].as_str().unwrap()
    );
    assert_eq!(
        h.bearer("POST", &secondary_route, &secondary_token, Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    ok(
        &h,
        &owner,
        "PUT",
        &format!("/api/brains/{brain}/grants/{reader_id}"),
        json!({"role":"writer"}),
    )
    .await;
    assert_eq!(
        h.bearer("POST", &secondary_route, &secondary_token, Value::Null)
            .await
            .0,
        StatusCode::CONFLICT
    );
    let all = ok(&h, &owner, "GET", &registrations, Value::Null).await;
    assert_eq!(
        all.as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == second["id"])
            .unwrap()["eligible"],
        false
    );
    let outsider = h.fixture_member().await;
    let mut tx = checked(recollect_server::db::actor_tx(&h.state.pool, outsider.0).await);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_private_runners")
            .fetch_one(&mut *tx)
            .await
            .unwrap(),
        0
    );
    tx.rollback().await.unwrap();
    // Recollect device disablement is distinct from the forbidden Vault APIs.
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/devices/{device}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.bearer("POST", &route("credentials"), &token, request)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        ok(&h, &owner, "GET", &registrations, Value::Null).await[0]["eligible"],
        false
    );
    h.finish().await;
}
