//! Exercise the MCP adapters against the actual model gateway and graph handlers.
use super::*;
use crate::mcp::tools::tool;
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn mcp_agent_handover_pagination_and_native_graph_preserve_scope() {
    let (h, owner, base, provider, model_server) = setup().await;
    let mut selections = vec![];
    let mut claims = vec![];
    for label in ["Allowed handover", "Other handover"] {
        let area = ok(
            &h,
            "POST",
            &format!("{base}/evidence/groups"),
            &owner,
            json!({"name":label,"kind":"area"}),
        )
        .await;
        let selection = json!({"repository_ids":[],"area_ids":[area["id"]],"environment_id":null});
        let evidence = source(&h, &owner, &base, &format!("{label} recorded port 8080.\n")).await;
        let mut input = review::proposal(&evidence["version"]["id"], label, "8080");
        input["content"]["selection"] = selection.clone();
        claims.push(ok(&h, "POST", &format!("{base}/claims"), &owner, input).await);
        selections.push(selection);
    }
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let (_, token) = h.pair_device(&owner, "Handover and graph bridge").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let uri = format!("http://{}{base}/mcp/agent", listener.local_addr().unwrap());
    let router = h.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = StreamableHttpClientTransportConfig::with_uri(uri);
    config.custom_headers.insert(
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    let service = ().serve(StreamableHttpClientTransport::from_config(config)).await.unwrap();
    let peer = service.peer();
    let mut tasks = vec![];
    let mut writes = vec![];
    for (i, selection) in selections.iter().enumerate() {
        let parent = tasks.first().map(|task: &Value| task["task"]["id"].clone());
        let task = tool(
            peer,
            "workspace.start_task",
            json!({"input":{
            "label":format!("Independent scope {i}"),"selection":selection,"parent_task_id":parent},
            "context_query":"handover"}),
        )
        .await;
        assert_eq!(task["context"]["state"], "ready");
        writes.push(
            tool(
                peer,
                "workspace.begin",
                json!({"id":task["task"]["id"],
            "input":{"kind":"write","expected_scope":task["task"]["scope"]["id"]}}),
            )
            .await,
        );
        tasks.push(task);
    }
    let input = json!({"operation_id":writes[0]["id"],"request_id":Uuid::new_v4(),
        "input":{"title":"Allowed summary","contributions":[claims[0]["id"]]}});
    let blocked = peer
        .call_tool(
            CallToolRequestParams::new("memory.handover")
                .with_arguments(input.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(
        blocked.is_error,
        Some(true),
        "standing model policy is required"
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["synthesis"]);
        policy["content_classes"] = json!(["claim", "query"]);
    })
    .await;
    *provider.candidates.lock().unwrap() = json!({
        "summary":"The selected evidence records the service port.",
        "completed":["Recorded the scoped declaration."],
        "next_steps":["Verify the service before changing it."],
        "risks":["Declared configuration does not prove runtime health."]});
    let queued = tool(peer, "memory.handover", input.clone()).await;
    assert_eq!(
        tool(peer, "memory.handover", input).await,
        queued,
        "request replay must preserve the same generation job"
    );
    model_job(&h).await;
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let read = tasks[0]["context"]["operation_id"].clone();
    let initial = tool(peer, "memory.handover_status", json!({"operation_id":read})).await;
    assert_eq!(initial["total"], 1);
    let generated = &initial["items"][0];
    assert_eq!(generated["state"], "succeeded", "{initial}");
    let detail = tool(
        peer,
        "memory.inspect",
        json!({"operation_id":read,"id":generated["claim_id"]}),
    )
    .await;
    let revision = &detail["selected"]["revision"];
    assert_eq!(revision["origin"], "model_synthesized");
    assert_eq!(revision["review"], "proposed");
    assert_eq!(revision["reviewer_id"], Value::Null);
    assert_eq!(
        detail["selected"]["contributions"][0]["revision_id"],
        claims[0]["id"]
    );

    // Fill two genuine result pages and interleave another child's private scope.
    for i in 1..24 {
        let selected = usize::from(i % 10 == 0);
        tool(
            peer,
            "memory.handover",
            json!({"operation_id":writes[selected]["id"],
            "request_id":Uuid::new_v4(),"input":{"title":format!("Page {i}"),
                "contributions":[claims[selected]["id"]]}}),
        )
        .await;
    }
    let first = tool(peer, "memory.handover_status", json!({"operation_id":read})).await;
    let second = tool(
        peer,
        "memory.handover_status",
        json!({"operation_id":read,"query":{"offset":20}}),
    )
    .await;
    assert_eq!(first["total"], 22);
    assert_eq!(second["total"], 22);
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    assert_eq!(second["items"].as_array().unwrap().len(), 2);
    let ids = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .chain(second["items"].as_array().unwrap())
        .map(|r| {
            assert_eq!(r["selection"], selections[0]);
            r["id"].as_str().unwrap().to_owned()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 22);
    assert!(ids.contains(queued["id"].as_str().unwrap()));
    let other = tool(
        peer,
        "memory.handover_status",
        json!({"operation_id":tasks[1]["context"]["operation_id"]}),
    )
    .await;
    assert_eq!(other["total"], 2);
    assert!(
        other["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["selection"] == selections[1])
    );

    crate::graph::build(&h, &owner, &base, "knowledge", None).await;
    let explored = tool(
        peer,
        "memory.graph_explore",
        json!({"operation_id":read,
        "input":{"scope":{"kind":"knowledge"},"direction":"outgoing","max_hops":3}}),
    )
    .await;
    assert!(
        explored["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["evidence"]["revision_id"] == revision["id"])
    );
    assert!(
        explored["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| node["evidence"]["revision_id"] != claims[1]["id"])
    );
    let path_input = json!({"scope":{"kind":"knowledge"},
        "start":format!("claim:{}",claims[0]["id"].as_str().unwrap()),
        "end":format!("claim:{}",revision["id"].as_str().unwrap()),
        "direction":"outgoing","max_hops":2});
    let path = tool(
        peer,
        "memory.graph_path",
        json!({"operation_id":read,"input":path_input}),
    )
    .await;
    assert_eq!(path["edges"].as_array().unwrap().len(), 1);
    assert_eq!(path["edges"][0]["relation"], "contributed_to");
    let denied = peer
        .call_tool(
            CallToolRequestParams::new("memory.graph_path").with_arguments(
                json!({"operation_id":tasks[1]["context"]["operation_id"],"input":path_input})
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
        "the other child cannot traverse the first child's path"
    );
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        1,
        "paging and graph reads must not generate model requests"
    );
    service.cancel().await.unwrap();
    server.abort();
    model_server.abort();
    h.finish().await;
}
