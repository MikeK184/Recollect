use super::*;
use rmcp::{
    RoleClient, ServiceExt,
    model::CallToolRequestParams,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};

pub(crate) async fn tool(peer: &rmcp::Peer<RoleClient>, name: &str, arguments: Value) -> Value {
    let reply = peer
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(arguments.as_object().unwrap().clone()),
        )
        .await
        .unwrap_or_else(|error| panic!("{name}: {error:?}"));
    assert_ne!(reply.is_error, Some(true), "{name}: {reply:?}");
    reply
        .structured_content
        .expect("structured Recollect result")
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_agent_http_sdk_scoped_tools_and_cold_discovery() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("recollect_server::mcp::agent=warn")
        .with_test_writer()
        .try_init();
    let (h, owner, brain, _, profile) = setup().await;
    let profile = &profile["profile"];
    let (device, token) = h.pair_device(&owner, "Agent HTTP fixture").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap();
    let uri = format!("{endpoint}/api/brains/{brain}/mcp/agent");
    assert_eq!(
        http.post(&uri)
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call(
            "POST",
            &format!("/api/brains/{brain}/mcp/agent"),
            Some(&owner),
            json!({})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut config = StreamableHttpClientTransportConfig::with_uri(uri.clone());
    config.custom_headers.insert(
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    let service = ().serve(StreamableHttpClientTransport::from_config(config)).await.unwrap();
    let peer = service.peer();
    let names = peer
        .list_all_tools()
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.name.to_string())
        .collect::<Vec<_>>();
    assert!(
        names.contains(&"memory.recall".into()) && names.contains(&"workspace.set_scope".into())
    );
    assert!(
        !names
            .iter()
            .any(|n| n.contains("erase") || n == "memory.review")
    );
    let profiles = tool(peer, "mcp.profiles", json!({})).await;
    assert_eq!(profiles["profiles"][0]["id"], profile["id"]);
    let task = tool(peer, "workspace.start_task", json!({"input":{"label":"MCP context proof","parent_task_id":null,"workspace_id":null,"selection":null},"context_query":"known fixture"})).await;
    assert_eq!(task["context"]["state"], "ready", "{task}");
    assert_eq!(
        task["context"]["scope_id"],
        task["handoff"]["current_scope_id"]
    );
    let operation = tool(peer,"workspace.begin",json!({"id":task["task"]["id"],"input":{"kind":"write","expected_scope":task["task"]["scope"]["id"]}})).await;
    let base = format!("/api/brains/{brain}");
    let source = ok(&h,&owner,"POST",&format!("{base}/sources"),json!({"title":"Agent tool evidence","media_type":"text/plain","content":"Bridge fixture uses exact operations.\n","retain_content":true,"source_uri":null,"group_ids":[]})).await;
    let mut input = crate::review::proposal(
        &source["version"]["id"],
        "Bridge fixture",
        "exact operations",
    );
    input.as_object_mut().unwrap().remove("operation_id");
    input["content"]
        .as_object_mut()
        .unwrap()
        .remove("selection");
    let key = Uuid::new_v4();
    let arguments = json!({"operation_id":operation["id"],"request_id":key,"input":input});
    let claim = tool(peer, "memory.contribute", arguments.clone()).await;
    assert_eq!(claim["origin"], "device_authored");
    assert_eq!(claim["review"], "proposed");
    assert_eq!(
        tool(peer, "memory.contribute", arguments).await,
        claim,
        "shared idempotency preserves one revision"
    );
    let read = task["context"]["operation_id"].clone();
    let recalled = tool(
        peer,
        "memory.recall",
        json!({"operation_id":read,"input":{"query":"Bridge fixture"}}),
    )
    .await;
    assert!(
        recalled.to_string().contains("exact operations"),
        "{recalled}"
    );
    let inspected = tool(
        peer,
        "memory.inspect",
        json!({"operation_id":read,"id":claim["claim_id"]}),
    )
    .await;
    assert_eq!(inspected["selected"]["revision"]["id"], claim["id"]);
    let history = tool(
        peer,
        "memory.review_history",
        json!({"operation_id":read,"id":claim["claim_id"]}),
    )
    .await;
    assert_eq!(history["decisions"], json!([]));
    let denied = peer
        .call_tool(
            CallToolRequestParams::new("mcp.discover").with_arguments(
                json!({"operation_id":read,"input":{"profile_id":profile["id"]}})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        denied.is_error,
        Some(true),
        "Brain owner still needs separate Use"
    );
    let invalid = peer
        .call_tool(
            CallToolRequestParams::new("memory.recall").with_arguments(
                json!({"brain_id":Uuid::new_v4(),"operation_id":read,"input":{"query":"fixture"}})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await;
    assert!(invalid.is_err(), "model cannot replace fixed Brain");
    for (name, arguments) in [
        (
            "memory.recall",
            json!({"operation_id":read,"input":{"query":"fixture","selection":{"repository_ids":[]}}}),
        ),
        (
            "memory.contribute",
            json!({"operation_id":operation["id"],"request_id":Uuid::new_v4(),"input":input}),
        ),
    ] {
        let mut arguments = arguments;
        if name == "memory.contribute" {
            arguments["input"]["content"]["selection"] =
                json!({"repository_ids":[],"area_ids":[],"environment_id":null});
        }
        assert!(
            peer.call_tool(
                CallToolRequestParams::new(name)
                    .with_arguments(arguments.as_object().unwrap().clone(),),
            )
            .await
            .is_err(),
            "model-selected nested authority must be rejected for {name}"
        );
    }
    let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM mcp_calls),(SELECT count(*) FROM mcp_instances),(SELECT count(*) FROM model_requests)")
        .fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        counts,
        (0, 0, 0),
        "discovery and default context do not start providers or models"
    );
    // A retrieval failure after a committed scope change cannot imply rollback
    // or return context from the old scope. Inject the failure only in this DB.
    let area = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/evidence/groups"),
        json!({"kind":"area","name":"Refresh failure control"}),
    )
    .await;
    sqlx::query("ALTER TABLE operation_bindings ADD CONSTRAINT mcp_refresh_fixture_unavailable CHECK(kind <> 'retrieval') NOT VALID")
        .execute(&h.admin).await.unwrap();
    let changed = tool(
        peer,
        "workspace.set_scope",
        json!({"id":task["task"]["id"],
        "input":{"base_scope":task["task"]["scope"]["id"],
            "selection":{"repository_ids":[],"area_ids":[area["id"]],"environment_id":null}},
        "context_query":"Bridge fixture"}),
    )
    .await;
    assert_eq!(changed["context"]["state"], "unavailable");
    assert!(changed["context"].get("recall").is_none());
    let committed = tool(
        peer,
        "workspace.inspect_task",
        json!({"id":task["task"]["id"]}),
    )
    .await;
    assert_eq!(
        committed["task"]["scope"]["id"],
        changed["handoff"]["current_scope_id"]
    );
    sqlx::query("ALTER TABLE operation_bindings DROP CONSTRAINT mcp_refresh_fixture_unavailable")
        .execute(&h.admin)
        .await
        .unwrap();
    let recovered = tool(
        peer,
        "workspace.begin",
        json!({"id":task["task"]["id"],
        "input":{"kind":"retrieval","expected_scope":changed["handoff"]["current_scope_id"]}}),
    )
    .await;
    let fresh = tool(
        peer,
        "memory.recall",
        json!({"operation_id":recovered["id"],"input":{"query":"Bridge fixture"}}),
    )
    .await;
    assert_eq!(fresh["scope_id"], changed["handoff"]["current_scope_id"]);
    assert_eq!(
        http.post(&uri)
            .bearer_auth(&token)
            .header("origin", "https://wrong.example")
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http.post(&uri)
            .bearer_auth(&token)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .body(" ".repeat(256 * 1024 + 1))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    // This is a Recollect device, never a Vault token or lease.
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
    assert!(
        peer.list_all_tools().await.is_err(),
        "every HTTP request checks current pairing"
    );
    service.cancel().await.unwrap();
    server.abort();
    let _ = server.await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn plugin_source_tools_preserve_scope_idempotency_and_observed_usage() {
    let (h, owner, brain, _, _) = setup().await;
    let (device, token) = h
        .pair_with(&owner, "Shared plugin", Some("opencode"), Some("plugin"))
        .await;
    let base = format!("/api/brains/{brain}");
    let mut environments = Vec::new();
    for name in ["Scope A", "Scope B"] {
        let env = ok(
            &h,
            &owner,
            "POST",
            &format!("{base}/evidence/groups"),
            json!({"kind":"environment","name":name}),
        )
        .await;
        environments.push(env["id"].clone());
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let router = h.router.clone();
    let endpoint = format!(
        "http://{}/api/brains/{brain}/mcp/agent",
        listener.local_addr().unwrap()
    );
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = StreamableHttpClientTransportConfig::with_uri(endpoint);
    config.custom_headers.insert(
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    config.custom_headers.insert(
        axum::http::HeaderName::from_static("x-recollect-host"),
        "codex".parse().unwrap(),
    );
    let service = ().serve(StreamableHttpClientTransport::from_config(config)).await.unwrap();
    let peer = service.peer();
    let mut tasks = Vec::new();
    for env in &environments {
        tasks.push(tool(peer,"workspace.start_task",json!({"input":{"label":"Source workflow","selection":{"repository_ids":[],"area_ids":[],"environment_id":env}},"context_query":"Scoped evidence"})).await);
    }
    let write=tool(peer,"workspace.begin",json!({"id":tasks[0]["task"]["id"],"input":{"kind":"write","expected_scope":tasks[0]["task"]["scope"]["id"]}})).await;
    let args = json!({"operation_id":write["id"],"request_id":Uuid::new_v4(),"input":{"title":"Scoped document","media_type":"text/plain","content":"Amber.port = 8080\n","retain_content":true}});
    let source = tool(peer, "source.import", args.clone()).await;
    assert_eq!(tool(peer, "source.import", args).await, source);
    let inspect = json!({"id":source["id"],"version_id":source["version"]["id"],"operation_id":tasks[0]["context"]["operation_id"]});
    assert_eq!(
        tool(peer, "source.inspect", inspect.clone()).await["content"],
        "Amber.port = 8080\n"
    );
    let list = tool(
        peer,
        "source.list",
        json!({"operation_id":tasks[0]["context"]["operation_id"],"query":{"q":"Scoped"}}),
    )
    .await;
    assert_eq!(list["total"], 1);
    assert_eq!(
        tool(
            peer,
            "source.list",
            json!({"operation_id":tasks[1]["context"]["operation_id"]})
        )
        .await["total"],
        0
    );
    let mut incompatible = inspect;
    incompatible["operation_id"] = tasks[1]["context"]["operation_id"].clone();
    let denied = peer
        .call_tool(
            CallToolRequestParams::new("source.inspect")
                .with_arguments(incompatible.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(denied.is_error, Some(true));
    let scope: Value =
        sqlx::query_scalar("SELECT selection FROM source_import_scopes WHERE version_id=$1")
            .bind(
                source["version"]["id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
            )
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(scope["environment_id"], environments[0]);
    assert_eq!(source["version"]["device_id"], json!(device));
    let appended=ok(&h,&owner,"POST",&format!("{base}/sources/{}/versions",source["id"].as_str().unwrap()),json!({"title":"Scoped document","media_type":"text/plain","content":"Amber.port = 8181\n","retain_content":true,"base_version":source["version"]["id"]})).await;
    let inherited: Value =
        sqlx::query_scalar("SELECT selection FROM source_import_scopes WHERE version_id=$1")
            .bind(
                appended["version"]["id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
            )
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(inherited, scope);
    let (_, roster, _) = h
        .call("GET", &format!("{base}/agents"), Some(&owner), Value::Null)
        .await;
    assert_eq!(roster["groups"][0]["agents"][0]["host_kind"], "codex");
    assert_eq!(
        roster["groups"][0]["agents"][0]["observed_hosts"],
        json!(["codex"])
    );
    let (_, global, _) = h
        .call("GET", "/api/agents", Some(&owner), Value::Null)
        .await;
    assert_eq!(
        global["groups"][0]["agents"][0]["brains"][0]["brain_id"],
        json!(brain)
    );
    let (_, devices, _) = h
        .call("GET", "/api/devices", Some(&owner), Value::Null)
        .await;
    assert_eq!(devices[0]["observed_hosts"], json!(["codex"]));
    let (_, foreign) = h.fixture_member().await;
    let (foreign_device, foreign_token) = h.pair_device(&foreign, "No Brain access").await;
    assert_eq!(
        h.bearer(
            "GET",
            &format!("{base}/workspace"),
            &foreign_token,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM agent_brain_usage WHERE device_id=$1")
            .bind(foreign_device)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(count, 0);
    service.cancel().await.unwrap();
    server.abort();
    h.finish().await;
}
