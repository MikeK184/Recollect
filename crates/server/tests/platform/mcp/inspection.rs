use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn equivalent_http_inspection_is_owner_only_and_never_approves_or_calls() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (_, member) = h.fixture_member().await;
    definitions::import_manifest(&h.admin, manifest())
        .await
        .unwrap();
    let before: Vec<(String, Value)> =
        sqlx::query_as("SELECT key,manifest FROM mcp_definitions ORDER BY key")
            .fetch_all(&h.admin)
            .await
            .unwrap();
    let audit_before: i64 = sqlx::query_scalar("SELECT count(*) FROM mutation_audit")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let lists = Arc::new(AtomicUsize::new(0));
    let fixture_calls = calls.clone();
    let fixture_lists = lists.clone();
    let schema = json!({
        "$schema":"http://json-schema.org/draft-07/schema#",
        "type":"object","properties":{
            "query":{"type":"string","minLength":1},
            "objective":{"type":"string","minLength":1,"maxLength":4096}
        },"required":["query","objective"],"additionalProperties":false
    });
    let raw_schema = schema.clone();
    let fixture = Router::new().route(
        "/mcp",
        axum::routing::post(move |axum::Json(request): axum::Json<Value>| {
            let calls = fixture_calls.clone();
            let lists = fixture_lists.clone();
            let schema = schema.clone();
            async move {
                let id = request["id"].clone();
                let result = match request["method"].as_str() {
                    Some("server/discover") => json!({
                        "supportedVersions":["2026-07-28"],
                        "capabilities":{"tools":{"listChanged":true}},
                        "resultType":"complete","ttlMs":0,"cacheScope":"private",
                        "_meta":{"io.modelcontextprotocol/serverInfo":{"name":"Owned inspection fixture","version":"1"}}
                    }),
                    Some("tools/list") => {
                        lists.fetch_add(1, Ordering::SeqCst);
                        json!({"tools":[{
                            "name":"web_search_exa","description":"Public fixture\n\n**Metadata only.**",
                            "inputSchema":schema,"annotations":{"readOnlyHint":true}
                        }]})
                    }
                    Some("tools/call") => {
                        calls.fetch_add(1, Ordering::SeqCst);
                        json!({"content":[]})
                    }
                    _ => json!({}),
                };
                axum::Json(json!({"jsonrpc":"2.0","id":id,"result":result}))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("http://{}/mcp", listener.local_addr().unwrap());
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let stop = shutdown.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, fixture)
            .with_graceful_shutdown(async move { stop.notified().await })
            .await
            .unwrap();
    });
    let body = json!({"name":"Existing approved connection check","url":target});
    let path = "/api/mcp/definitions/inspect-http";
    assert_eq!(
        h.call("POST", path, Some(&member), body.clone()).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(lists.load(Ordering::SeqCst), 0);
    for _ in 0..2 {
        let inspected = ok(&h, &owner, "POST", path, body.clone()).await;
        let mut normalized = inspected["tools"][0]["inputSchema"].clone();
        assert_eq!(
            normalized["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        normalized["$schema"] = json!("http://json-schema.org/draft-07/schema#");
        assert_eq!(normalized, raw_schema);
        let mut raw_approval = inspected.clone();
        raw_approval["tools"][0]["inputSchema"] = normalized;
        assert_eq!(
            h.call("POST", "/api/mcp/definitions", Some(&owner), raw_approval)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            inspected["tools"][0]["description"],
            "Public fixture\n\n**Metadata only.**"
        );
    }
    assert_eq!(lists.load(Ordering::SeqCst), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let after: Vec<(String, Value)> =
        sqlx::query_as("SELECT key,manifest FROM mcp_definitions ORDER BY key")
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mutation_audit")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        audit_before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_calls")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    shutdown.notify_one();
    server.await.unwrap();
    h.finish().await;
}
