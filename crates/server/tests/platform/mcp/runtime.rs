use super::*;
use recollect_protocol::*;
use recollect_server::mcp::runtime::runner::{self, Runner};
#[path = "execution.rs"]
mod execution;
#[path = "private.rs"]
mod private;
#[path = "recovery.rs"]
mod recovery;

fn checked<T>(value: recollect_server::error::Result<T>) -> T {
    value.unwrap_or_else(|e| panic!("{}: {}", e.1, e.2))
}
fn body(connection: &Value, profile: &Value) -> Value {
    json!({"request_id":Uuid::new_v4(),"profile_id":profile["profile"]["id"],"connection_id":connection["summary"]["id"],
        "tool_name":"inspect","arguments":{"name":"synthetic"},"environment_id":null,"operation_id":null,
        "client_session_id":Uuid::new_v4(),"timeout_seconds":30})
}
async fn allow_owner(h: &Harness, owner: &Login, brain: Uuid, profile: &Value, allowed: bool) {
    grant(
        h,
        owner,
        brain,
        profile,
        &h.state.config.owner_username,
        rights(allowed, true, true),
    )
    .await;
}
fn attempt(plan: &McpExecutionPlan) -> McpAttempt {
    McpAttempt {
        epoch: plan.runner_epoch,
        brain_id: plan.brain_id,
        call_id: plan.call_id,
        attempt_token: plan.attempt_token,
    }
}
async fn instance(h: &Harness, runner: &Runner, plan: &McpExecutionPlan) -> Uuid {
    let id = Uuid::new_v4();
    checked(
        runner
            .instance(
                &h.state,
                &McpInstanceUpdate {
                    attempt: attempt(plan),
                    instance_id: id,
                    credential_generation: Uuid::new_v4(),
                    state: "starting".into(),
                    active_calls: 1,
                    idle_seconds: 0,
                },
            )
            .await,
    );
    id
}
async fn central(h: &Harness) -> Runner {
    let runner = Runner::central(Uuid::new_v4());
    checked(runner.register(&h.state).await);
    runner
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_runtime_durable_dispatch_fences_replay_cancel_and_expiry() {
    let (h, owner, brain, connection, profile) = setup().await;
    let base = format!("/api/brains/{brain}/mcp/calls");
    let input = body(&connection, &profile);
    assert_eq!(
        h.call("POST", &base, Some(&owner), input.clone()).await.0,
        StatusCode::FORBIDDEN
    );
    allow_owner(&h, &owner, brain, &profile, true).await;
    let admitted = ok(&h, &owner, "POST", &base, input.clone()).await;
    let id = admitted["id"].as_str().unwrap();
    let path = format!("{base}/{id}");
    assert_eq!(admitted["state"], "queued");
    let repeated = ok(&h, &owner, "POST", &base, input.clone()).await;
    assert_eq!(repeated["id"], admitted["id"]);
    let mut changed = input.clone();
    changed["arguments"]["name"] = json!("changed");
    assert_eq!(
        h.call("POST", &base, Some(&owner), changed).await.0,
        StatusCode::CONFLICT
    );
    let runner = central(&h).await;
    assert!(
        Runner::central(Uuid::new_v4())
            .register(&h.state)
            .await
            .is_err()
    );
    let plan = checked(runner.claim(&h.state).await).plan.unwrap();
    assert_eq!(plan.call_id.to_string(), id);
    assert!(checked(runner.claim(&h.state).await).plan.is_none());
    let owned = instance(&h, &runner, &plan).await;
    let start = McpStart {
        attempt: attempt(&plan),
        instance_id: owned,
    };
    assert!(checked(runner.start(&h.state, &start).await).dispatch);
    assert!(
        !checked(runner.start(&h.state, &start).await).dispatch,
        "lost start response must never authorize a second send"
    );
    let cancelled = ok(&h, &owner, "POST", &format!("{path}/cancel"), Value::Null).await;
    assert_eq!(cancelled["state"], "running");
    assert_eq!(cancelled["cancel_requested"], true);
    assert!(
        checked(runner.heartbeat(&h.state).await)
            .cancel_calls
            .contains(&plan.call_id)
    );
    let completed = McpCompletion {
        attempt: attempt(&plan),
        state: "succeeded".into(),
        code: "connector_response".into(),
        result: Some(json!({"content":[],"structuredContent":["synthetic response"]})),
    };
    checked(runner.complete(&h.state, &completed).await);
    checked(runner.complete(&h.state, &completed).await);
    let mut wrong = completed.clone();
    wrong.result = Some(json!({"replaced":true}));
    assert!(runner.complete(&h.state, &wrong).await.is_err());
    wrong.attempt.attempt_token = Uuid::new_v4();
    assert!(runner.complete(&h.state, &wrong).await.is_err());
    let final_result = ok(&h, &owner, "GET", &path, Value::Null).await;
    assert_eq!(final_result["state"], "succeeded");
    assert_eq!(final_result["result"], completed.result.unwrap());
    assert_eq!(
        ok(&h, &owner, "POST", &base, input.clone()).await["id"],
        admitted["id"]
    );
    assert_eq!(
        ok(&h, &owner, "GET", &base, Value::Null).await["calls"][0]["result"],
        Value::Null,
        "history never bulk-returns retained payloads"
    );
    sqlx::query("UPDATE mcp_calls SET payload_expires_at=now()-interval '1 second' WHERE id=$1")
        .bind(plan.call_id)
        .execute(&h.admin)
        .await
        .unwrap();
    let expired = ok(&h, &owner, "GET", &path, Value::Null).await;
    assert_eq!(expired["output_access"], "expired");
    assert!(expired["result"].is_null());
    assert_eq!(
        h.call("POST", &base, Some(&owner), input).await.1["code"],
        "mcp_replay_expired"
    );
    checked(runner::maintain(&h.state).await);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_call_payloads WHERE call_id=$1")
            .bind(plan.call_id)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    let queued = ok(&h, &owner, "POST", &base, body(&connection, &profile)).await;
    let queued_path = format!("{base}/{}/cancel", queued["id"].as_str().unwrap());
    assert_eq!(
        ok(&h, &owner, "POST", &queued_path, Value::Null).await["state"],
        "cancelled"
    );
    assert!(checked(runner.claim(&h.state).await).plan.is_none());
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_runtime_revocation_configuration_loss_and_late_receipt() {
    let (h, owner, brain, connection, profile) = setup().await;
    let base = format!("/api/brains/{brain}/mcp/calls");
    allow_owner(&h, &owner, brain, &profile, true).await;
    let runner = central(&h).await;
    let admitted = ok(&h, &owner, "POST", &base, body(&connection, &profile)).await;
    let path = format!("{base}/{}", admitted["id"].as_str().unwrap());
    let plan = checked(runner.claim(&h.state).await).plan.unwrap();
    let owned = instance(&h, &runner, &plan).await;
    allow_owner(&h, &owner, brain, &profile, false).await;
    let stopped = checked(
        runner
            .start(
                &h.state,
                &McpStart {
                    attempt: attempt(&plan),
                    instance_id: owned,
                },
            )
            .await,
    );
    assert!(!stopped.dispatch);
    assert_eq!(stopped.state, "failed");
    let metadata = ok(&h, &owner, "GET", &path, Value::Null).await;
    assert_eq!(metadata["output_access"], "permission_required");
    let mut tx = db::actor_tx(&h.state.pool, plan.actor_id)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_call_payloads WHERE call_id=$1")
            .bind(plan.call_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap(),
        0
    );
    tx.rollback().await.unwrap();
    allow_owner(&h, &owner, brain, &profile, true).await;
    let fresh = ok(&h, &owner, "POST", &base, body(&connection, &profile)).await;
    let plan = checked(runner.claim(&h.state).await).plan.unwrap();
    let owned = instance(&h, &runner, &plan).await;
    assert!(
        checked(
            runner
                .start(
                    &h.state,
                    &McpStart {
                        attempt: attempt(&plan),
                        instance_id: owned
                    }
                )
                .await
        )
        .dispatch
    );
    sqlx::query("UPDATE mcp_calls SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(plan.call_id)
        .execute(&h.admin)
        .await
        .unwrap();
    checked(runner::maintain(&h.state).await);
    let path = format!("{base}/{}", fresh["id"].as_str().unwrap());
    assert_eq!(
        ok(&h, &owner, "GET", &path, Value::Null).await["state"],
        "unknown"
    );
    checked(
        runner
            .complete(
                &h.state,
                &McpCompletion {
                    attempt: attempt(&plan),
                    state: "succeeded".into(),
                    code: "connector_response".into(),
                    result: Some(json!({"content":[],"structuredContent":["late"]})),
                },
            )
            .await,
    );
    let late = ok(&h, &owner, "GET", &path, Value::Null).await;
    assert_eq!(late["state"], "unknown");
    assert_eq!(late["resolutions"][0]["kind"], "late_receipt");
    let (foreign, foreign_login) = h.fixture_member().await;
    assert_eq!(
        h.call("GET", &path, Some(&foreign_login), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let mut tx = checked(db::actor_tx(&h.state.pool, foreign).await);
    for table in [
        "mcp_calls",
        "mcp_call_payloads",
        "mcp_instances",
        "mcp_call_resolutions",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    tx.rollback().await.unwrap();
    let changed = ok(&h, &owner, "POST", &base, body(&connection, &profile)).await;
    let mut edit = connection_body(None);
    edit["base_revision"] = connection["summary"]["revision"].clone();
    edit["target"] = json!("replacement");
    ok(
        &h,
        &owner,
        "PUT",
        &format!(
            "/api/brains/{brain}/mcp/connections/{}",
            connection["summary"]["id"].as_str().unwrap()
        ),
        edit,
    )
    .await;
    assert!(checked(runner.claim(&h.state).await).plan.is_none());
    let changed = ok(
        &h,
        &owner,
        "GET",
        &format!("{base}/{}", changed["id"].as_str().unwrap()),
        Value::Null,
    )
    .await;
    assert_eq!(changed["state"], "failed");
    assert_eq!(changed["code"], "mcp_configuration_changed");
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_runtime_local_placement_device_scope_and_no_central_fallback() {
    let (h, owner, brain, connection, profile) = setup().await;
    allow_owner(&h, &owner, brain, &profile, true).await;
    let (device, token) = h.pair_device(&owner, "Runtime fixture").await;
    let (_, wrong_token) = h.pair_device(&owner, "Other runtime fixture").await;
    let mut edit = connection_body(None);
    edit["base_revision"] = connection["summary"]["revision"].clone();
    edit["placement"] = json!("local");
    edit["runner_reference"] = json!(format!("device:{device}"));
    let connection = ok(
        &h,
        &owner,
        "PUT",
        &format!(
            "/api/brains/{brain}/mcp/connections/{}",
            connection["summary"]["id"].as_str().unwrap()
        ),
        edit,
    )
    .await;
    let base = format!("/api/brains/{brain}/mcp/calls");
    let mut input = body(&connection, &profile);
    assert_eq!(
        h.bearer("POST", &base, &token, input.clone()).await.0,
        StatusCode::BAD_REQUEST
    );
    let (status,task)=h.bearer("POST",&format!("/api/brains/{brain}/workspace/tasks"),&token,
        json!({"label":"Runtime scope","selection":{"repository_ids":[],"area_ids":[],"environment_id":null}})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, operation) = h
        .bearer(
            "POST",
            &format!(
                "/api/brains/{brain}/workspace/tasks/{}/operations",
                task["task"]["id"].as_str().unwrap()
            ),
            &token,
            json!({"kind":"tool"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    input["operation_id"] = operation["id"].clone();
    assert_eq!(
        h.bearer("POST", &base, &wrong_token, input.clone()).await.0,
        StatusCode::NOT_FOUND
    );
    let (status, call) = h.bearer("POST", &base, &token, input).await;
    assert_eq!(status, StatusCode::OK, "{call}");
    assert_eq!(call["scope"], operation["scope"]);
    let central = central(&h).await;
    assert!(checked(central.claim(&h.state).await).plan.is_none());
    assert_eq!(
        h.call(
            "POST",
            "/api/mcp/runner/register",
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (status, wrong) = h
        .bearer(
            "POST",
            "/api/mcp/runner/register",
            &wrong_token,
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        h.bearer(
            "POST",
            "/api/mcp/runner/claim",
            &wrong_token,
            json!({"epoch":wrong["epoch"]})
        )
        .await
        .1["plan"]
            .is_null()
    );
    let (status, local) = h
        .bearer("POST", "/api/mcp/runner/register", &token, Value::Null)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        h.bearer("POST", "/api/mcp/runner/register", &token, Value::Null)
            .await
            .0,
        StatusCode::CONFLICT
    );
    let (status, claimed) = h
        .bearer(
            "POST",
            "/api/mcp/runner/claim",
            &token,
            json!({"epoch":local["epoch"]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{claimed}");
    let plan: McpExecutionPlan = serde_json::from_value(claimed["plan"].clone()).unwrap();
    assert_eq!(plan.call_id.to_string(), call["id"]);
    let update = McpInstanceUpdate {
        attempt: attempt(&plan),
        instance_id: Uuid::new_v4(),
        credential_generation: Uuid::new_v4(),
        state: "starting".into(),
        active_calls: 1,
        idle_seconds: 0,
    };
    assert_eq!(
        h.bearer(
            "POST",
            "/api/mcp/runner/instance",
            &wrong_token,
            serde_json::to_value(&update).unwrap()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.bearer(
            "POST",
            "/api/mcp/runner/instance",
            &token,
            serde_json::to_value(&update).unwrap()
        )
        .await
        .0,
        StatusCode::OK
    );
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
        h.bearer(
            "POST",
            "/api/mcp/runner/start",
            &token,
            serde_json::to_value(McpStart {
                attempt: attempt(&plan),
                instance_id: update.instance_id
            })
            .unwrap()
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("UPDATE mcp_runners SET lease_until=now()-interval '1 second' WHERE reference=$1")
        .bind(format!("device:{device}"))
        .execute(&h.admin)
        .await
        .unwrap();
    checked(runner::maintain(&h.state).await);
    assert_eq!(
        ok(
            &h,
            &owner,
            "GET",
            &format!("{base}/{}", plan.call_id),
            Value::Null
        )
        .await["state"],
        "failed"
    );
    h.finish().await;
}
