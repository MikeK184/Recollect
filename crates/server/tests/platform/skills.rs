use super::*;
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};

fn body(name: &str, environment: Option<Uuid>) -> Value {
    json!({"name":name,"description":"Read scoped documentation.","kind":"skill","environment_id":environment,"profile_ids":[],"tags":["vault"],"enabled":true,"expected_version":null,
        "files":[{"path":"SKILL.md","content":format!("---\nname: {name}\ndescription: >-\n  Read scoped documentation.\n---\nRead references/notes.md when needed.\n")},{"path":"references/notes.md","content":"Exact retained reference body."}]})
}
async fn ok(h: &Harness, owner: &Login, method: &str, path: &str, body: Value) -> Value {
    let (status, result, _) = h.call(method, path, Some(owner), body).await;
    assert_eq!(status, StatusCode::OK, "{path}: {result}");
    result
}
async fn op(h: &Harness, token: &str, brain: Uuid, environment: Option<Uuid>) -> (Value, Value) {
    let task = h.bearer("POST",&format!("/api/brains/{brain}/workspace/tasks"),token,json!({"label":"Guidance test","workspace_id":null,"parent_task_id":null,"selection":{"repository_ids":[],"area_ids":[],"environment_id":environment}})).await;
    assert_eq!(task.0, StatusCode::OK, "{}", task.1);
    let operation = h
        .bearer(
            "POST",
            &format!(
                "/api/brains/{brain}/workspace/tasks/{}/operations",
                task.1["task"]["id"].as_str().unwrap()
            ),
            token,
            json!({"kind":"retrieval","expected_scope":task.1["task"]["scope"]["id"]}),
        )
        .await;
    assert_eq!(operation.0, StatusCode::OK, "{}", operation.1);
    (task.1, operation.1)
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn managed_skills_http_permissions_versions_scopes_revocation_and_deletion() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":"Skills proof"}),
    )
    .await;
    let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain}");
    let path = format!("{base}/skills");
    let (reader_id, reader) = h.fixture_member().await;
    let (writer_id, writer) = h.fixture_member().await;
    let (_, foreign) = h.fixture_member().await;
    for (id, role) in [(reader_id, "reader"), (writer_id, "writer")] {
        ok(
            &h,
            &owner,
            "PUT",
            &format!("{base}/grants/{id}"),
            json!({"role":role}),
        )
        .await;
    }
    let prod = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/evidence/groups"),
        json!({"kind":"environment","name":"Production"}),
    )
    .await;
    let prod: Uuid = prod["id"].as_str().unwrap().parse().unwrap();
    let dev = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/evidence/groups"),
        json!({"kind":"environment","name":"Development"}),
    )
    .await;
    let dev: Uuid = dev["id"].as_str().unwrap().parse().unwrap();
    let global = ok(&h, &owner, "POST", &path, body("global-guide", None)).await;
    let scoped = ok(
        &h,
        &owner,
        "POST",
        &path,
        body("production-guide", Some(prod)),
    )
    .await;
    let id = global["skill"]["id"].as_str().unwrap();
    let exact = format!("{path}/{id}");
    let scoped_id = scoped["skill"]["id"].as_str().unwrap();
    let scoped_path = format!("{path}/{scoped_id}");
    assert_eq!(
        h.call("GET", &path, Some(&foreign), Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    for principal in [&reader, &writer] {
        assert_eq!(
            h.call("POST", &path, Some(principal), body("unauthorized", None))
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    let list = ok(&h, &reader, "GET", &path, Value::Null).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert!(!list.to_string().contains("Exact retained reference body"));
    assert_eq!(
        ok(
            &h,
            &reader,
            "GET",
            &format!("{path}?environment_id={prod}"),
            Value::Null
        )
        .await
        .as_array()
        .unwrap()
        .len(),
        2
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("{path}?include_disabled=true"),
            Some(&reader),
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call("POST", &path, Some(&owner), body("global-guide", None))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.call("PUT", &exact, Some(&owner), body("global-guide", None))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let mut replacement = body("global-guide", None);
    replacement["expected_version"] = json!(1);
    replacement["files"][1]["content"] = json!("New exact reference.");
    let updated = ok(&h, &owner, "PUT", &exact, replacement.clone()).await;
    assert_eq!(updated["skill"]["version"], 2);
    assert_eq!(
        ok(
            &h,
            &reader,
            "GET",
            &format!("{exact}?version=1&path=references%2Fnotes.md"),
            Value::Null
        )
        .await["files"][0]["content"],
        "Exact retained reference body."
    );
    replacement["expected_version"] = json!(2);
    let (one, two) = tokio::join!(
        h.call("PUT", &exact, Some(&owner), replacement.clone()),
        h.call("PUT", &exact, Some(&owner), replacement)
    );
    assert!(matches!(
        (one.0, two.0),
        (StatusCode::OK, StatusCode::CONFLICT) | (StatusCode::CONFLICT, StatusCode::OK)
    ));
    let before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_audit WHERE brain_id=$1 AND action LIKE 'skills.%'",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let mut invalid = body("unsafe-guide", None);
    invalid["files"][1]["path"] = json!("../escape.md");
    assert_eq!(
        h.call("POST", &path, Some(&owner), invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    let after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_audit WHERE brain_id=$1 AND action LIKE 'skills.%'",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(before, after);
    let mut metadata_heavy = body("large-metadata", None);
    let description = "😀".repeat(1024);
    metadata_heavy["description"] = json!(&description);
    metadata_heavy["files"][0]["content"] = json!(format!(
        "---\nname: large-metadata\ndescription: '{description}'\n---\nGuidance."
    ));
    metadata_heavy["tags"] = json!(
        (0..32)
            .map(|i| format!("{}{:02}", "😀".repeat(62), i))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        h.call("POST", &path, Some(&owner), metadata_heavy).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut moved = body("production-guide", Some(dev));
    moved["expected_version"] = json!(1);
    ok(&h, &owner, "PUT", &scoped_path, moved.clone()).await;
    for query in [
        format!("environment_id={prod}"),
        format!("environment_id={dev}&version=1"),
        "version=1".into(),
    ] {
        assert_eq!(
            h.call(
                "GET",
                &format!("{scoped_path}?{query}"),
                Some(&reader),
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        ok(
            &h,
            &reader,
            "GET",
            &format!("{scoped_path}?environment_id={dev}"),
            Value::Null
        )
        .await["skill"]["version"],
        2
    );
    let profile:Uuid=sqlx::query_scalar("INSERT INTO mcp_profiles(id,brain_id,name,revision,created_by) VALUES(gen_random_uuid(),$1,'Advisory',gen_random_uuid(),(SELECT owner_id FROM brains WHERE id=$1)) RETURNING id").bind(brain).fetch_one(&h.admin).await.unwrap();
    let mut associated = body("associated-guide", None);
    associated["profile_ids"] = json!([profile]);
    ok(&h, &owner, "POST", &path, associated.clone()).await;
    let mut tx = db::actor_tx(
        &h.state.pool,
        sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
    )
    .await
    .unwrap_or_else(|e| panic!("{}", e.1));
    let can_use: bool = sqlx::query_scalar("SELECT recollect_mcp_can($1,'use')")
        .bind(profile)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert!(!can_use);
    tx.rollback().await.unwrap();
    let other_brain = ok(
        &h,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":"Other skills Brain"}),
    )
    .await;
    let other_brain: Uuid = other_brain["id"].as_str().unwrap().parse().unwrap();
    let other_profile:Uuid=sqlx::query_scalar("INSERT INTO mcp_profiles(id,brain_id,name,revision,created_by) VALUES(gen_random_uuid(),$1,'Foreign advisory',gen_random_uuid(),(SELECT owner_id FROM brains WHERE id=$1)) RETURNING id").bind(other_brain).fetch_one(&h.admin).await.unwrap();
    associated["profile_ids"] = json!([other_profile]);
    assert_eq!(
        h.call("POST", &path, Some(&owner), associated).await.0,
        StatusCode::NOT_FOUND
    );
    let other_env = ok(
        &h,
        &owner,
        "POST",
        &format!("/api/brains/{other_brain}/evidence/groups"),
        json!({"kind":"environment","name":"Foreign environment"}),
    )
    .await;
    let other_env: Uuid = other_env["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        h.call(
            "POST",
            &path,
            Some(&owner),
            body("foreign-environment", Some(other_env))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{other_brain}/skills/{id}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (_, token) = h.pair_device(&reader, "Skills device").await;
    let (_, other) = h.pair_device(&reader, "Other skills device").await;
    let (task, operation) = op(&h, &token, brain, Some(dev)).await;
    let operation_id = operation["id"].as_str().unwrap();
    assert_eq!(
        h.bearer("GET", &path, &token, Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    let selected = format!("{scoped_path}?operation_id={operation_id}");
    assert_eq!(
        h.bearer("GET", &selected, &token, Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer("GET", &selected, &other, Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.bearer(
            "GET",
            &format!("{selected}&environment_id={prod}"),
            &token,
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    moved["expected_version"] = json!(2);
    moved["enabled"] = json!(false);
    ok(&h, &owner, "PUT", &scoped_path, moved.clone()).await;
    assert_eq!(
        h.bearer("GET", &format!("{selected}&version=2"), &token, Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        ok(
            &h,
            &owner,
            "GET",
            &format!("{scoped_path}?environment_id={dev}"),
            Value::Null
        )
        .await["skill"]["enabled"],
        false
    );
    moved["expected_version"] = json!(3);
    moved["enabled"] = json!(true);
    ok(&h, &owner, "PUT", &scoped_path, moved).await;
    let task_id = task["task"]["id"].as_str().unwrap();
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{base}/workspace/tasks/{task_id}/close"),
            &token,
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer("GET", &selected, &token, Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    let (_, fresh) = op(&h, &token, brain, None).await;
    ok(
        &h,
        &owner,
        "DELETE",
        &format!("{base}/grants/{reader_id}"),
        Value::Null,
    )
    .await;
    assert_eq!(
        h.bearer(
            "GET",
            &format!("{exact}?operation_id={}", fresh["id"].as_str().unwrap()),
            &token,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // Version rows are insert-only to the application role, even for the owner.
    let owner_id: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let mut tx = db::actor_tx(&h.state.pool, owner_id)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert!(
        sqlx::query("UPDATE managed_skill_versions SET files='[]'")
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let preview = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/deletions/preview"),
        Value::Null,
    )
    .await;
    let removed = h
        .call(
            "DELETE",
            &base,
            Some(&owner),
            json!({"closure":preview["closure"],"confirmation":"Skills proof"}),
        )
        .await;
    assert_eq!(removed.0, StatusCode::OK, "{}", removed.1);
    let remaining:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM managed_skills WHERE brain_id=$1),(SELECT count(*) FROM managed_skill_versions WHERE brain_id=$1)").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(remaining, (0, 0));
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn managed_skills_real_mcp_progressive_fetch_and_immutable_scope() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":"MCP skills proof"}),
    )
    .await;
    let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let path = format!("/api/brains/{brain}/skills");
    let skill = ok(&h, &owner, "POST", &path, body("vault-guide", None)).await;
    let prod = ok(
        &h,
        &owner,
        "POST",
        &format!("/api/brains/{brain}/evidence/groups"),
        json!({"kind":"environment","name":"Production"}),
    )
    .await;
    let prod: Uuid = prod["id"].as_str().unwrap().parse().unwrap();
    let dev = ok(
        &h,
        &owner,
        "POST",
        &format!("/api/brains/{brain}/evidence/groups"),
        json!({"kind":"environment","name":"Development"}),
    )
    .await;
    let dev: Uuid = dev["id"].as_str().unwrap().parse().unwrap();
    let prod_skill = ok(&h, &owner, "POST", &path, body("prod-only", Some(prod))).await;
    let dev_skill = ok(&h, &owner, "POST", &path, body("dev-only", Some(dev))).await;
    let (_, token) = h.pair_device(&owner, "MCP skills").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = h.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = StreamableHttpClientTransportConfig::with_uri(format!(
        "http://{address}/api/brains/{brain}/mcp/agent"
    ));
    config.custom_headers.insert(
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    let service = ().serve(StreamableHttpClientTransport::from_config(config)).await.unwrap();
    let peer = service.peer();
    let tools = peer.list_all_tools().await.unwrap();
    for name in ["skills.list", "skills.fetch"] {
        let tool = tools.iter().find(|t| t.name == name).unwrap();
        assert!(tool.annotations.as_ref().unwrap().read_only_hint.unwrap());
    }
    let (task, operation) = op(&h, &token, brain, Some(prod)).await;
    let read = operation["id"].clone();
    let list = mcp::tools::tool(
        peer,
        "skills.list",
        json!({"operation_id":read,"query":"vault-guide"}),
    )
    .await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert!(!list.to_string().contains("Read references/notes.md"));
    let entry = mcp::tools::tool(
        peer,
        "skills.fetch",
        json!({"operation_id":read,"id":skill["skill"]["id"]}),
    )
    .await;
    assert_eq!(entry["files"].as_array().unwrap().len(), 1);
    assert_eq!(entry["files"][0]["path"], "SKILL.md");
    assert_eq!(entry["skill"]["file_paths"].as_array().unwrap().len(), 2);
    let resource=mcp::tools::tool(peer,"skills.fetch",json!({"operation_id":read,"id":skill["skill"]["id"],"version":1,"path":"references/notes.md"})).await;
    assert_eq!(
        resource["files"][0]["content"],
        "Exact retained reference body."
    );
    let before = mcp::tools::tool(peer, "skills.list", json!({"operation_id":read})).await;
    assert_eq!(before.as_array().unwrap().len(), 2);
    assert!(
        before
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == prod_skill["skill"]["id"])
    );
    assert!(
        !before
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == dev_skill["skill"]["id"])
    );
    let task_id = task["task"]["id"].as_str().unwrap();
    let changed=h.bearer("PUT",&format!("/api/brains/{brain}/workspace/tasks/{task_id}/scope"),&token,json!({"base_scope":task["task"]["scope"]["id"],"selection":{"repository_ids":[],"area_ids":[],"environment_id":dev}})).await;
    assert_eq!(changed.0, StatusCode::OK, "{}", changed.1);
    assert_eq!(
        mcp::tools::tool(peer, "skills.list", json!({"operation_id":read})).await,
        before,
        "Existing read retains Production after default becomes Development"
    );
    let fresh = h
        .bearer(
            "POST",
            &format!("/api/brains/{brain}/workspace/tasks/{task_id}/operations"),
            &token,
            json!({"kind":"retrieval","expected_scope":changed.1["task"]["scope"]["id"]}),
        )
        .await;
    assert_eq!(fresh.0, StatusCode::OK);
    let after = mcp::tools::tool(peer, "skills.list", json!({"operation_id":fresh.1["id"]})).await;
    assert!(
        after
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == dev_skill["skill"]["id"])
    );
    let wrong_env = peer
        .call_tool(
            CallToolRequestParams::new("skills.fetch").with_arguments(
                json!({"operation_id":fresh.1["id"],"id":prod_skill["skill"]["id"]})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(wrong_env.is_error, Some(true));
    let tool_op = h
        .bearer(
            "POST",
            &format!("/api/brains/{brain}/workspace/tasks/{task_id}/operations"),
            &token,
            json!({"kind":"tool","expected_scope":changed.1["task"]["scope"]["id"]}),
        )
        .await;
    assert_eq!(tool_op.0, StatusCode::OK);
    let wrong_kind = peer
        .call_tool(
            CallToolRequestParams::new("skills.list").with_arguments(
                json!({"operation_id":tool_op.1["id"]})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(wrong_kind.is_error, Some(true));
    for arguments in [
        json!({"operation_id":read,"environment_id":Uuid::new_v4()}),
        json!({"operation_id":read,"brain_id":Uuid::new_v4()}),
        json!({"operation_id":read,"endpoint":"https://example.test"}),
    ] {
        assert!(
            peer.call_tool(
                CallToolRequestParams::new("skills.list")
                    .with_arguments(arguments.as_object().unwrap().clone())
            )
            .await
            .is_err()
        );
    }
    let (_, other) = h.pair_device(&owner, "Other MCP skills").await;
    let (_, foreign) = op(&h, &other, brain, None).await;
    let denied = peer
        .call_tool(
            CallToolRequestParams::new("skills.list").with_arguments(
                json!({"operation_id":foreign["id"]})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(denied.is_error, Some(true));
    service.cancel().await.unwrap();
    server.abort();
    h.finish().await;
}
