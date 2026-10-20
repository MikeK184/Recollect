use super::*;
use rmcp::{
    ServiceExt as McpServiceExt,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};

async fn task(h: &Harness, base: &str, token: &str, repo: Uuid, env: Option<Uuid>) -> Value {
    let (status,body)=h.bearer("POST",&format!("{base}/workspace/tasks"),token,json!({"label":"Synthetic coordination","selection":{"repository_ids":[repo],"environment_id":env}})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["task"].clone()
}
fn announcement(task: &Value, id: Uuid, repo: Uuid, path: &str, mode: &str) -> Value {
    json!({"id":id,"task_id":task["id"],"expected_scope":task["scope"]["id"],"host_kind":"codex","label":"Synthetic agent","goal":"Inspect vault certificate renewal","topics":["vault"],"intents":[{"repository_id":repo,"path":path,"mode":mode,"phase":"planned"}]})
}
async fn inbox(h: &Harness, base: &str, token: &str, id: Uuid, after: i64) -> Value {
    let (status, body) = h
        .bearer(
            "GET",
            &format!("{base}/coordination/sessions/{id}/inbox?after={after}"),
            token,
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}
fn assert_private(value: &Value) {
    let text = value.to_string();
    assert!(!text.contains("task_id"), "{text}");
    assert!(!text.contains("scope_id"), "{text}");
    assert!(!text.contains("/fixture/"), "{text}");
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL fixture"]
async fn coordination_authority_replay_negotiation_and_a2a() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (paul, paul_login) = h.fixture_member().await;
    let brain = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Synthetic coordination"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let base = format!("/api/brains/{brain}");
    let coord = format!("{base}/coordination");
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/grants/{paul}"),
            Some(&owner),
            json!({"role":"reader"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, mike_token) = h.pair_device(&owner, "mike-mac").await;
    let (paul_device, paul_token) = h.pair_device(&paul_login, "paul-ubuntu").await;
    let actor: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let repo = Uuid::new_v4();
    sqlx::query("INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,'example.test/synthetic/infra',$3)").bind(repo).bind(brain.parse::<Uuid>().unwrap()).bind(actor).execute(&h.admin).await.unwrap();
    let mt = task(&h, &base, &mike_token, repo, None).await;
    let pt = task(&h, &base, &paul_token, repo, None).await;
    let mid = Uuid::new_v4();
    let pid = Uuid::new_v4();
    let ma = announcement(&mt, mid, repo, "charts/vault.yaml", "write");
    let pa = announcement(&pt, pid, repo, "charts/vault.yaml", "read");
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{coord}/sessions"),
            &mike_token,
            ma.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, result) = h
        .bearer(
            "POST",
            &format!("{coord}/sessions"),
            &paul_token,
            pa.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert!(
        result["overlaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["reason"] == "file_write_overlap")
    );
    assert_private(&result["overlaps"]);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = h.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config =
        StreamableHttpClientTransportConfig::with_uri(format!("http://{address}{base}/mcp/agent"));
    config.custom_headers.insert(
        axum::http::header::AUTHORIZATION,
        format!("Bearer {mike_token}").parse().unwrap(),
    );
    let client = ().serve(StreamableHttpClientTransport::from_config(config)).await.unwrap();
    let tools = client.peer().list_all_tools().await.unwrap();
    for name in [
        "coordination.announce",
        "coordination.heartbeat",
        "coordination.peers",
        "coordination.inbox",
        "coordination.ack",
        "coordination.send",
        "coordination.close",
    ] {
        assert!(tools.iter().any(|t| t.name == name));
    }
    let peers = mcp::tools::tool(
        client.peer(),
        "coordination.peers",
        json!({"query":{"session_id":mid}}),
    )
    .await;
    assert_eq!(peers["sessions"].as_array().unwrap().len(), 2);
    assert_private(&peers);
    let announced =
        mcp::tools::tool(client.peer(), "coordination.announce", json!({"input":ma})).await;
    assert_eq!(announced["session"]["revision"], 1);
    client.cancel().await.unwrap();
    server.abort();
    let first = inbox(&h, &base, &mike_token, mid, 0).await;
    assert_private(&first);
    assert_eq!(first["events"].as_array().unwrap().len(), 2);
    let mut replay = ma.clone();
    replay["expected_revision"] = json!(1);
    let replay_result = h
        .bearer(
            "POST",
            &format!("{coord}/sessions"),
            &mike_token,
            replay.clone(),
        )
        .await;
    assert_eq!(replay_result.0, StatusCode::OK);
    assert_eq!(replay_result.1["session"]["revision"], 1);
    assert_eq!(
        inbox(&h, &base, &mike_token, mid, 0).await["events"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{coord}/sessions/{mid}/heartbeat"),
            &paul_token,
            json!({"expected_revision":1})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.bearer(
            "GET",
            &format!("{coord}/sessions/{mid}/inbox"),
            &paul_token,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let mut bad = ma.clone();
    bad["id"] = json!(Uuid::new_v4());
    bad["intents"][0]["path"] = json!("../private");
    assert_eq!(
        h.bearer("POST", &format!("{coord}/sessions"), &mike_token, bad)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let proposal = json!({"id":Uuid::new_v4(),"from_session_id":mid,"to_session_id":pid,"kind":"proposal","text":"I will edit vault; please inspect tests.","reply_to":null});
    let one = h
        .bearer(
            "POST",
            &format!("{coord}/messages"),
            &mike_token,
            proposal.clone(),
        )
        .await;
    assert_eq!(one.0, StatusCode::OK, "{}", one.1);
    assert_private(&one.1);
    let two = h
        .bearer(
            "POST",
            &format!("{coord}/messages"),
            &mike_token,
            proposal.clone(),
        )
        .await;
    assert_eq!(one, two);
    let mut wrong = proposal.clone();
    wrong["text"] = json!("Changed replay");
    assert_eq!(
        h.bearer("POST", &format!("{coord}/messages"), &mike_token, wrong)
            .await
            .0,
        StatusCode::CONFLICT
    );
    let accept = json!({"id":Uuid::new_v4(),"from_session_id":pid,"to_session_id":mid,"kind":"accept","text":"Agreed. I will inspect tests.","reply_to":proposal["id"]});
    assert_eq!(
        h.bearer("POST", &format!("{coord}/messages"), &paul_token, accept)
            .await
            .0,
        StatusCode::OK
    );
    let events = inbox(&h, &base, &paul_token, pid, 0).await;
    let event = events["events"][0]["id"].as_str().unwrap();
    for _ in 0..2 {
        assert_eq!(
            h.bearer(
                "POST",
                &format!("{coord}/sessions/{pid}/inbox/{event}/ack"),
                &paul_token,
                Value::Null
            )
            .await
            .0,
            StatusCode::OK
        )
    }
    let (_, card) = h
        .bearer(
            "GET",
            &format!("{coord}/a2a/.well-known/agent-card.json"),
            &mike_token,
            Value::Null,
        )
        .await;
    assert_eq!(card["supportedInterfaces"][0]["protocolVersion"], "1.0");
    assert_eq!(card["capabilities"]["streaming"], false);
    let _: a2a::AgentCard = serde_json::from_value(card).unwrap();
    let a2a_input = json!({"id":Uuid::new_v4(),"from_session_id":mid,"to_session_id":pid,"kind":"note","text":"A2A wire proof","reply_to":null});
    let body = json!({"message":{"messageId":a2a_input["id"],"role":"ROLE_USER","parts":[{"data":{"schema":"recollect.coordination.v1","action":"send","message":a2a_input},"mediaType":"application/json"}]}});
    let response = h
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("{coord}/a2a/message:send"))
                .header("authorization", format!("Bearer {mike_token}"))
                .header("a2a-version", "1.0")
                .header("content-type", "application/a2a+json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let parsed: a2a::SendMessageResponse = serde_json::from_value(value.clone()).unwrap();
    assert!(matches!(parsed, a2a::SendMessageResponse::Message(_)));
    assert_eq!(value["message"]["role"], "ROLE_AGENT");
    for version in [None, Some("0.3"), Some("9.9")] {
        let mut req = Request::builder()
            .method("POST")
            .uri(format!("{coord}/a2a/message:send"))
            .header("authorization", format!("Bearer {mike_token}"))
            .header("content-type", "application/a2a+json");
        if let Some(version) = version {
            req = req.header("a2a-version", version);
        }
        let response = h
            .router
            .clone()
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let value: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(
            value["error"]["details"][0]["reason"],
            "VERSION_NOT_SUPPORTED"
        );
    }
    // Retention expiry reports an explicit cursor gap and fresh peer snapshot.
    sqlx::query("UPDATE coordination_events SET expires_at=now()-interval '1 second' WHERE session_id=$1 AND sequence=1").bind(pid).execute(&h.admin).await.unwrap();
    let gap = inbox(&h, &base, &paul_token, pid, 0).await;
    assert_eq!(gap["gap"], true);
    assert!(!gap["peers"].as_array().unwrap().is_empty());
    // A reconnect can pull persisted events after process/router replacement and lease expiry.
    sqlx::query(
        "UPDATE coordination_sessions SET expires_at=now()-interval '1 second' WHERE id=$1",
    )
    .bind(mid)
    .execute(&h.admin)
    .await
    .unwrap();
    assert!(
        !inbox(&h, &base, &mike_token, mid, 0).await["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{coord}/sessions/{mid}/heartbeat"),
            &mike_token,
            json!({"expected_revision":1})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // An expired identity cannot bypass active admission capacity on reactivation.
    sqlx::query("WITH ids AS(SELECT gen_random_uuid() AS id FROM generate_series(1,100)) INSERT INTO coordination_sessions(id,brain_id,account_id,device_id,task_id,scope_id,revision,input,projection,last_seen_at,expires_at) SELECT i.id,s.brain_id,s.account_id,s.device_id,s.task_id,s.scope_id,s.revision,jsonb_set(s.input,'{id}',to_jsonb(i.id::text)),jsonb_set(s.projection,'{id}',to_jsonb(i.id::text)),now(),now()+interval '90 seconds' FROM coordination_sessions s CROSS JOIN ids i WHERE s.id=$1").bind(mid).execute(&h.admin).await.unwrap();
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{coord}/sessions"),
            &mike_token,
            replay.clone()
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    sqlx::query("DELETE FROM coordination_sessions WHERE account_id=$1 AND id<>$2")
        .bind(actor)
        .bind(mid)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.bearer("POST", &format!("{coord}/sessions"), &mike_token, replay)
            .await
            .0,
        StatusCode::OK
    );
    // Concurrent stale revisions cannot overwrite each other.
    let mut update_a = ma.clone();
    update_a["expected_revision"] = json!(1);
    update_a["label"] = json!("Update A");
    let mut update_b = update_a.clone();
    update_b["label"] = json!("Update B");
    let announce_path = format!("{coord}/sessions");
    let (a, b) = tokio::join!(
        h.bearer("POST", &announce_path, &mike_token, update_a),
        h.bearer("POST", &announce_path, &mike_token, update_b)
    );
    assert!(
        a.0 == StatusCode::OK && b.0 == StatusCode::CONFLICT
            || a.0 == StatusCode::CONFLICT && b.0 == StatusCode::OK
    );
    // Lease refresh cannot silently rebind a changed task scope.
    let changed = h
        .bearer(
            "PUT",
            &format!(
                "{base}/workspace/tasks/{}/scope",
                mt["id"].as_str().unwrap()
            ),
            &mike_token,
            json!({"base_scope":mt["scope"]["id"],"selection":{}}),
        )
        .await;
    assert_eq!(changed.0, StatusCode::OK);
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{coord}/sessions/{mid}/heartbeat"),
            &mike_token,
            json!({"expected_revision":1})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // SSE uses the same durable cursor and closes before sending another frame after revocation.
    let response = h
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("{coord}/sessions/{pid}/events"))
                .header("authorization", format!("Bearer {paul_token}"))
                .header("last-event-id", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mut stream = response.into_body();
    let frame = stream.frame().await.unwrap().unwrap();
    let data = frame.into_data().unwrap();
    assert!(String::from_utf8_lossy(&data).contains("id: 2"));
    // Device revocation stops all new reads, including retained inbox.
    sqlx::query("UPDATE devices SET revoked_at=now() WHERE id=$1")
        .bind(paul_device)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.bearer(
            "GET",
            &format!("{coord}/sessions/{pid}/inbox"),
            &paul_token,
            Value::Null
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let end = tokio::time::timeout(std::time::Duration::from_secs(5), stream.frame())
        .await
        .unwrap();
    assert!(end.is_none(), "Revocation must end a pending SSE stream");
    finish(h).await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL fixture"]
async fn coordination_heartbeat_takes_the_shared_brain_lock() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain: Uuid = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Synthetic heartbeat"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let base = format!("/api/brains/{brain}");
    let coord = format!("{base}/coordination");
    let (_, token) = h.pair_device(&owner, "mike-mac").await;
    let actor: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let repo = Uuid::new_v4();
    sqlx::query("INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,'example.test/synthetic/heartbeat',$3)").bind(repo).bind(brain).bind(actor).execute(&h.admin).await.unwrap();
    let t = task(&h, &base, &token, repo, None).await;
    let id = Uuid::new_v4();
    let (status, body) = h
        .bearer(
            "POST",
            &format!("{coord}/sessions"),
            &token,
            announcement(&t, id, repo, "charts/vault.yaml", "write"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // A reader (recall) holds the shared lock. An exclusive heartbeat would wait
    // for it, and every later reader would queue behind that waiter.
    let mut reader = db::actor_tx(&h.state.pool, actor).await.ok().unwrap();
    db::lock_brain(&mut reader, brain, false)
        .await
        .ok()
        .unwrap();
    let (status, body) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        h.bearer(
            "POST",
            &format!("{coord}/sessions/{id}/heartbeat"),
            &token,
            json!({"expected_revision":1}),
        ),
    )
    .await
    .expect("Heartbeat must not wait for the exclusive Brain lock");
    assert_eq!(status, StatusCode::OK, "{body}");
    reader.rollback().await.unwrap();
    finish(h).await;
}

async fn finish(h: Harness) {
    let graph_count: i64 = sqlx::query_scalar("SELECT count(*) FROM graph_generations")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        graph_count, 0,
        "Coordination fixture must not create external graph resources"
    );
    let _ = std::fs::remove_file(&h.state.config.credential_file);
    let _ = std::fs::remove_dir_all(&h.state.config.artifact_dir);
    let _ = std::fs::remove_dir_all(&h.state.config.erasure_journal);
    h.state.pool.close().await;
    h.admin.close().await;
    sqlx::query(&format!("DROP DATABASE {} WITH (FORCE)", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    h.root.close().await;
}
