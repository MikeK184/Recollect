use super::tools::tool;
use super::*;
use rmcp::{Peer, RoleClient, model::CallToolRequestParams};

async fn denied(peer: &Peer<RoleClient>, name: &str, args: Value) {
    let result = peer
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await;
    assert!(
        result.is_err() || result.as_ref().is_ok_and(|r| r.is_error == Some(true)),
        "{name} unexpectedly succeeded"
    );
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_agent_current_permissions_retention_rejection_and_erasure() {
    let (h, owner, brain, _, _) = setup().await;
    let (_, token) = h.pair_device(&owner, "MCP policy fixture").await;
    let (_, other_token) = h.pair_device(&owner, "Other device").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let bridge =
        recollect_mcp_runtime::agent_transport::connect(&endpoint, brain, token.parse().unwrap())
            .await
            .unwrap();
    let other = recollect_mcp_runtime::agent_transport::connect(
        &endpoint,
        brain,
        other_token.parse().unwrap(),
    )
    .await
    .unwrap();
    let peer = bridge.peer();
    let task = tool(
        peer,
        "workspace.start_task",
        json!({"input":{"label":"Policy task"},"context_query":"Bridge"}),
    )
    .await;
    let read = task["context"]["operation_id"].clone();
    denied(
        other.peer(),
        "memory.recall",
        json!({"operation_id":read,"input":{"query":"Bridge"}}),
    )
    .await;
    let base = format!("/api/brains/{brain}");
    let source=ok(&h,&owner,"POST",&format!("{base}/sources"),json!({"title":"Policy proof","media_type":"text/plain","content":"Independent documented observations.","retain_content":true,"source_uri":null,"group_ids":[]})).await;
    let mut claims = Vec::new();
    for (subject, value) in [
        ("Bridge control", "PERMITTED_CONTROL"),
        ("Bridge rejected", "REJECTED_PAYLOAD"),
        ("Bridge expired", "EXPIRED_PAYLOAD"),
        ("Bridge erased", "ERASED_PAYLOAD"),
    ] {
        claims.push(
            ok(
                &h,
                &owner,
                "POST",
                &format!("{base}/claims"),
                crate::review::proposal(&source["version"]["id"], subject, value),
            )
            .await,
        );
    }
    let before = tool(
        peer,
        "memory.recall",
        json!({"operation_id":read,"input":{"query":"Bridge"}}),
    )
    .await;
    assert!(
        before.to_string().contains("PERMITTED_CONTROL")
            && before.to_string().contains("ERASED_PAYLOAD")
    );
    let rejected = &claims[1];
    ok(&h,&owner,"POST",&format!("{base}/claims/{}/review",rejected["claim_id"].as_str().unwrap()),
        json!({"base_revision":rejected["id"],"action":"reject","reason":"Fixture rejection with durable value rule."})).await;
    let history = tool(
        peer,
        "memory.review_history",
        json!({"operation_id":read,"id":rejected["claim_id"]}),
    )
    .await;
    assert!(!history["decisions"].as_array().unwrap().is_empty());
    let write = tool(
        peer,
        "workspace.begin",
        json!({"id":task["task"]["id"],"input":{"kind":"write"}}),
    )
    .await;
    let mut retry = crate::review::proposal(
        &source["version"]["id"],
        "Bridge rejected",
        "REJECTED_PAYLOAD",
    );
    retry.as_object_mut().unwrap().remove("operation_id");
    retry["content"]
        .as_object_mut()
        .unwrap()
        .remove("selection");
    // Canonical admission retains a blocked proposal for audit. Success here
    // must never turn it into eligible knowledge or bypass the rejection rule.
    let blocked = tool(
        peer,
        "memory.contribute",
        json!({"operation_id":write["id"],"request_id":Uuid::new_v4(),"input":retry}),
    )
    .await;
    assert_eq!(blocked["admission"], "blocked_by_rule");
    denied(
        peer,
        "memory.recall",
        json!({"operation_id":write["id"],"input":{"query":"Bridge"}}),
    )
    .await;
    let retention = ok(&h, &owner, "GET", &format!("{base}/retention"), Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["claim_days"] = json!(1);
    ok(
        &h,
        &owner,
        "PUT",
        &format!("{base}/retention"),
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    sqlx::query("UPDATE claim_revisions SET recorded_at=now()-interval '2 days',revision=jsonb_set(revision,'{recorded_at}',to_jsonb(now()-interval '2 days')) WHERE claim_id=$1")
        .bind(claims[2]["claim_id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
    let target = json!({"kind":"claim","id":claims[3]["claim_id"]});
    let preview = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/erasures/preview"),
        target.clone(),
    )
    .await;
    ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/erasures"),
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let after = tool(
        peer,
        "memory.recall",
        json!({"operation_id":read,"input":{"query":"Bridge"}}),
    )
    .await;
    assert!(after.to_string().contains("PERMITTED_CONTROL"));
    assert!(
        !after.to_string().contains("ERASED_PAYLOAD")
            && !after.to_string().contains("EXPIRED_PAYLOAD")
            && !after.to_string().contains("REJECTED_PAYLOAD")
    );
    for claim in [&claims[2], &claims[3]] {
        let result = peer
            .call_tool(
                CallToolRequestParams::new("memory.inspect").with_arguments(
                    json!({"operation_id":read,"id":claim["claim_id"]})
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            )
            .await
            .unwrap();
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains("ERASED_PAYLOAD") && !text.contains("EXPIRED_PAYLOAD"));
    }
    let (member_id, member) = h.fixture_member().await;
    reader(&h, &owner, brain, member_id).await;
    let (_, member_token) = h.pair_device(&member, "Reader MCP").await;
    let reader = recollect_mcp_runtime::agent_transport::connect(
        &endpoint,
        brain,
        member_token.parse().unwrap(),
    )
    .await
    .unwrap();
    assert!(
        !reader
            .peer()
            .list_all_tools()
            .await
            .unwrap()
            .iter()
            .any(|t| t.name == "memory.contribute")
    );
    let reader_task = tool(
        reader.peer(),
        "workspace.start_task",
        json!({"input":{"label":"Reader scope"},"context_query":"Bridge"}),
    )
    .await;
    assert!(
        reader_task["context"]
            .to_string()
            .contains("PERMITTED_CONTROL")
    );
    denied(
        reader.peer(),
        "memory.contribute",
        json!({"operation_id":write["id"],"request_id":Uuid::new_v4(),"input":retry}),
    )
    .await;
    ok(
        &h,
        &owner,
        "DELETE",
        &format!("{base}/grants/{member_id}"),
        Value::Null,
    )
    .await;
    assert!(
        reader.peer().list_all_tools().await.is_err(),
        "live discovery rechecks current Brain grants"
    );
    let foreign = ok(
        &h,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":"Other fixed destination"}),
    )
    .await;
    let foreign = recollect_mcp_runtime::agent_transport::connect(
        &endpoint,
        foreign["id"].as_str().unwrap().parse().unwrap(),
        token.parse().unwrap(),
    )
    .await
    .unwrap();
    denied(
        foreign.peer(),
        "memory.recall",
        json!({"operation_id":read,"input":{"query":"Bridge"}}),
    )
    .await;
    ok(&h, &owner, "PATCH", &base, json!({"archived":true})).await;
    denied(
        peer,
        "workspace.start_task",
        json!({"input":{"label":"Archived task"},"context_query":"Bridge"}),
    )
    .await;
    denied(
        peer,
        "memory.contribute",
        json!({"operation_id":write["id"],"request_id":Uuid::new_v4(),"input":retry}),
    )
    .await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM model_requests")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    reader.cancel().await.unwrap();
    foreign.cancel().await.unwrap();
    other.cancel().await.unwrap();
    bridge.cancel().await.unwrap();
    serving.abort();
    let _ = serving.await;
    h.finish().await;
}
