use super::*;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn pairing_markers_and_brain_agent_roster() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (status, brain, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Roster brain"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let brain_id = brain["id"].as_str().unwrap().to_owned();

    // Unknown markers are rejected before any pairing exists.
    assert_eq!(
        h.call(
            "POST",
            "/api/devices/pairings",
            None,
            json!({"name":"Bad host","host_kind":"vim"})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/devices/pairings",
            None,
            json!({"name":"Bad integration","integration":"companion"})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    // Markers reach the device record at approval.
    let (plugin_id, _) = h
        .pair_with(&owner, "Plugin host", Some("codex"), Some("plugin"))
        .await;
    let (mcp_id, _) = h
        .pair_with(&owner, "Token host", Some("opencode"), Some("mcp"))
        .await;
    let row: (Option<String>, String) =
        sqlx::query_as("SELECT host_kind,integration FROM devices WHERE id=$1")
            .bind(plugin_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(row, (Some("codex".into()), "plugin".into()));

    // Re-pairing the same name without markers preserves existing data.
    h.pair_with(&owner, "Plugin host", None, None).await;
    let row: (Option<String>, String) =
        sqlx::query_as("SELECT host_kind,integration FROM devices WHERE id=$1")
            .bind(plugin_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(row, (Some("codex".into()), "plugin".into()));

    // Pairing alone does not associate a device with a Brain.
    let roster_path = format!("/api/brains/{brain_id}/agents");
    let (_, roster, _) = h.call("GET", &roster_path, Some(&owner), Value::Null).await;
    assert!(roster["groups"].as_array().unwrap().is_empty());
    assert_eq!(roster["hidden_count"], 0);
    let _ = mcp_id;

    // Brain members only: a foreign account gets no roster.
    let (_, member) = h.fixture_member().await;
    assert_eq!(
        h.call("GET", &roster_path, Some(&member), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn account_agent_roster() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (status, brain_a, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Usage A"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let brain_a_id: Uuid = Uuid::parse_str(brain_a["id"].as_str().unwrap()).unwrap();
    let (_, brain_b, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Usage B"}),
        )
        .await;
    let brain_b_id: Uuid = Uuid::parse_str(brain_b["id"].as_str().unwrap()).unwrap();

    let (plugin_id, _) = h
        .pair_with(&owner, "Plugin host", Some("codex"), Some("plugin"))
        .await;
    let (mcp_id, _) = h
        .pair_with(&owner, "Token host", Some("opencode"), Some("mcp"))
        .await;
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE username=$1")
        .bind(h.state.config.owner_username.clone())
        .fetch_one(&h.admin)
        .await
        .unwrap();

    // One real managed call on brain A gives the plugin device usage there.
    let definition_id: Uuid = sqlx::query_scalar(
        "INSERT INTO mcp_definitions(key,id,manifest,approved_by) VALUES('roster-fixture',gen_random_uuid(),'{\"key\":\"roster-fixture\"}'::jsonb,$1) RETURNING id",
    )
    .bind(owner_id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let connection_id: Uuid = sqlx::query_scalar(
        "INSERT INTO mcp_connections(id,brain_id,name,description,definition_key,target,placement,configuration,revision,created_by) VALUES(gen_random_uuid(),$1,'roster','','roster-fixture','fixture-target','central','{}',gen_random_uuid(),$2) RETURNING id",
    )
    .bind(brain_a_id)
    .bind(owner_id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let profile_id: Uuid = sqlx::query_scalar(
        "INSERT INTO mcp_profiles(id,brain_id,name,revision,created_by) VALUES(gen_random_uuid(),$1,'roster',gen_random_uuid(),$2) RETURNING id",
    )
    .bind(brain_a_id)
    .bind(owner_id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO mcp_calls(id,brain_id,actor_id,device_id,request_id,profile_id,connection_id,profile_revision,connection_revision,definition_revision,tool_name,client_session_id,runner_reference,state,timeout_seconds,completed_at,payload_expires_at) VALUES(gen_random_uuid(),$1,$2,$3,gen_random_uuid(),$4,$5,gen_random_uuid(),gen_random_uuid(),clock_timestamp(),'echo',gen_random_uuid(),'central','succeeded',60,clock_timestamp(),clock_timestamp())",
    )
    .bind(brain_a_id)
    .bind(owner_id)
    .bind(plugin_id)
    .bind(profile_id)
    .bind(connection_id)
    .execute(&h.admin)
    .await
    .unwrap();

    // Owner: one group, both agents, plugin carries brain A usage only.
    let (status, roster, _) = h
        .call("GET", "/api/agents", Some(&owner), Value::Null)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(roster["groups"].as_array().unwrap().len(), 1);
    assert_eq!(
        roster["groups"][0]["user_name"],
        h.state.config.owner_username
    );
    let agents = roster["groups"][0]["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 2);
    assert_eq!(roster["hidden_count"], 0);
    let plugin = agents
        .iter()
        .find(|a| a["device_id"] == json!(plugin_id.to_string()))
        .unwrap();
    let usage = plugin["brains"].as_array().unwrap();
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0]["brain_id"], json!(brain_a_id));
    assert_eq!(usage[0]["name"], "Usage A");
    let token = agents
        .iter()
        .find(|a| a["device_id"] == json!(mcp_id.to_string()))
        .unwrap();
    assert!(token["brains"].as_array().unwrap().is_empty());

    // Revocation hides the agent and bumps the hidden count.
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/devices/{mcp_id}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let (_, roster, _) = h
        .call("GET", "/api/agents", Some(&owner), Value::Null)
        .await;
    assert_eq!(roster["groups"][0]["agents"].as_array().unwrap().len(), 1);
    assert_eq!(roster["hidden_count"], 1);

    // A member account sees only their own devices: none.
    let (_, member) = h.fixture_member().await;
    let (status, roster, _) = h
        .call("GET", "/api/agents", Some(&member), Value::Null)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(roster["groups"].as_array().unwrap().is_empty());
    assert_eq!(roster["hidden_count"], 0);

    // Owner-global visibility is personal too, while a Brain reader sees safe
    // metadata for every observed contributor without private call payloads.
    let (member_device, _) = h
        .pair_with(&member, "Member host", Some("claude_code"), Some("plugin"))
        .await;
    let member_id: Uuid = sqlx::query_scalar("SELECT account_id FROM devices WHERE id=$1")
        .bind(member_device)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    sqlx::query("INSERT INTO brain_grants(brain_id,account_id,role) VALUES($1,$2,'reader')")
        .bind(brain_a_id)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("INSERT INTO mcp_calls(id,brain_id,actor_id,device_id,request_id,profile_id,connection_id,profile_revision,connection_revision,definition_revision,tool_name,client_session_id,runner_reference,state,timeout_seconds,completed_at,payload_expires_at) VALUES(gen_random_uuid(),$1,$2,$3,gen_random_uuid(),$4,$5,gen_random_uuid(),gen_random_uuid(),clock_timestamp(),'echo',gen_random_uuid(),'central','succeeded',60,clock_timestamp(),clock_timestamp())").bind(brain_a_id).bind(member_id).bind(member_device).bind(profile_id).bind(connection_id).execute(&h.admin).await.unwrap();
    let (_, global, _) = h
        .call("GET", "/api/agents", Some(&owner), Value::Null)
        .await;
    assert!(!global.to_string().contains("Member host"));
    let (_, personal, _) = h
        .call("GET", "/api/agents", Some(&member), Value::Null)
        .await;
    assert!(!personal.to_string().contains("Plugin host"));
    assert_eq!(personal["groups"][0]["agents"].as_array().unwrap().len(), 1);
    let path = format!("/api/brains/{brain_a_id}/agents");
    let (_, shared, _) = h.call("GET", &path, Some(&member), Value::Null).await;
    let rows: Vec<&Value> = shared["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["agents"].as_array().unwrap())
        .collect();
    assert_eq!(rows.len(), 2);
    let foreign = rows
        .iter()
        .find(|a| a["device_id"] == json!(plugin_id))
        .unwrap();
    assert_eq!(foreign["can_revoke"], false);
    assert!(
        foreign["created_at"].is_null()
            && foreign["expires_at"].is_null()
            && foreign["last_used_at"].is_null()
    );
    assert!(foreign["last_used_on_brain_at"].is_string());
    let own = rows
        .iter()
        .find(|a| a["device_id"] == json!(member_device))
        .unwrap();
    assert_eq!(own["can_revoke"], true);
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/devices/{plugin_id}"),
            Some(&member),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let mut tx = db::actor_tx(&h.state.pool, member_id)
        .await
        .unwrap_or_else(|_| panic!("member transaction denied"));
    let private: i64 = sqlx::query_scalar("SELECT count(*) FROM mcp_calls WHERE device_id=$1")
        .bind(plugin_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(private, 0);
    let outside: i64 = sqlx::query_scalar("SELECT count(*) FROM recollect_brain_agent_usage($1)")
        .bind(brain_b_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(outside, 0);
    tx.commit().await.unwrap();
    // Legacy capture-host fallback is Brain-local and expires on the effective
    // retention deadline, even before accepted events are swept.
    let (capture_device, _) = h
        .pair_with(&owner, "Capture-only legacy", None, Some("plugin"))
        .await;
    let task = Uuid::new_v4();
    let scope = Uuid::new_v4();
    let operation = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let event = Uuid::new_v4();
    sqlx::query("INSERT INTO workspace_tasks(id,brain_id,account_id,label) VALUES($1,$2,$3,'agent fixture')").bind(task).bind(brain_a_id).bind(owner_id).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO scope_snapshots(id,brain_id,task_id,account_id,snapshot) VALUES($1,$2,$3,$4,'{}')").bind(scope).bind(brain_a_id).bind(task).bind(owner_id).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO operation_bindings(id,brain_id,task_id,scope_id,account_id,kind) VALUES($1,$2,$3,$4,$5,'context')").bind(operation).bind(brain_a_id).bind(task).bind(scope).bind(owner_id).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO capture_bindings(id,brain_id,operation_id,actor_id,device_id,host,host_version,selection) VALUES($1,$2,$3,$4,$5,'codex','fixture','{}')").bind(binding).bind(brain_a_id).bind(operation).bind(owner_id).bind(capture_device).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO capture_events(id,brain_id,binding_id,retention_class,captured_at,expires_at,state) VALUES($1,$2,$3,'raw_session',now(),now()+interval '1 day','accepted')").bind(event).bind(brain_a_id).bind(binding).execute(&h.admin).await.unwrap();
    let (_, fresh, _) = h.call("GET", &path, Some(&member), Value::Null).await;
    let captured = fresh["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["agents"].as_array().unwrap())
        .find(|a| a["device_id"] == json!(capture_device))
        .unwrap();
    assert_eq!(captured["host_kind"], "codex");
    sqlx::query("UPDATE capture_events SET captured_at=now()-interval '2 years' WHERE id=$1")
        .bind(event)
        .execute(&h.admin)
        .await
        .unwrap();
    let (_, expired, _) = h.call("GET", &path, Some(&member), Value::Null).await;
    assert!(!expired.to_string().contains("Capture-only legacy"));
    let (_, expired_global, _) = h
        .call("GET", "/api/agents", Some(&owner), Value::Null)
        .await;
    let captured = expired_global["groups"][0]["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["device_id"] == json!(capture_device))
        .unwrap();
    assert!(captured["brains"].as_array().unwrap().is_empty());

    sqlx::query("DELETE FROM brain_grants WHERE brain_id=$1 AND account_id=$2")
        .bind(brain_a_id)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call("GET", &path, Some(&member), Value::Null).await.0,
        StatusCode::NOT_FOUND
    );

    // No credential material leaks into the payload.
    let text = roster.to_string();
    assert!(!text.contains("Bearer"));
    let _ = (definition_id, brain_b_id);
    h.finish().await;
}
