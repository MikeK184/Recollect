use super::*;

#[tokio::test]
#[ignore = "Requires owned PostgreSQL and loopback gateway; preview admits zero calls"]
async fn model_readonly_preview_preserves_request_counts_and_blocks_body_drift() {
    let (h, owner, base, provider, server) = setup().await;
    allow(&h, &owner, &base, |p| {
        p["purposes"] = json!(["extraction"]);
    })
    .await;
    let source = source(&h, &owner, &base, "Orion uses PostgreSQL.\n").await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let policy: Uuid =
        sqlx::query_scalar("SELECT policy_id FROM model_policy_heads WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let context = gateway::Context {
        brain,
        actor,
        device: None,
    };
    let version: Uuid = source["version"]["id"].as_str().unwrap().parse().unwrap();
    let operation = Uuid::new_v4();
    let mut inv = gateway::extraction(operation, version);
    let before: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM model_requests),(SELECT count(*) FROM mutation_audit)",
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let preview = gateway::preview_for_policy(&h.state, context, &inv, policy)
        .await
        .unwrap_or_else(|e| panic!("preview {}", e.1));
    let after: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM model_requests),(SELECT count(*) FROM mutation_audit)",
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        before, after,
        "Read-only canonical request construction must not reserve or audit a model request"
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    assert!(serde_json::to_vec(&preview.body).unwrap().len() <= 65536);
    assert!(preview.body.to_string().contains("Orion uses PostgreSQL."));
    inv.instructions
        .push_str(" Changed instruction must invalidate the frozen body.");
    let failure = gateway::invoke_previewed_for_policy(&h.state, context, inv, &preview)
        .await
        .err()
        .expect("Changed instructions cannot use the prior approved body");
    assert_eq!(failure.1, "model_request_changed");
    let requests: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(requests, before.0);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let response = gateway::invoke_previewed_for_policy(
        &h.state,
        context,
        gateway::extraction(operation, version),
        &preview,
    )
    .await
    .unwrap_or_else(|e| panic!("previewed invocation {}", e.1));
    assert_eq!(response.request.state, "succeeded");
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        provider.bodies.lock().unwrap()[0],
        preview.body,
        "Actual admitted body must equal exact preview, including real resolved provenance"
    );
    server.abort();
    h.finish().await;
}
