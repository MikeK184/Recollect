use super::*;
use axum::{extract::State, http::HeaderMap, response::IntoResponse};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// Counts every Query API request and can hold the next one, forwarding to the
/// real Neo4j unchanged.
struct Counter {
    http: reqwest::Client,
    target: String,
    calls: AtomicUsize,
    statements: std::sync::Mutex<Vec<String>>,
    hold: AtomicBool,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

async fn forward(
    State(c): State<Arc<Counter>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    c.calls.fetch_add(1, Ordering::SeqCst);
    let statement = serde_json::from_slice::<Value>(&body)
        .ok()
        .and_then(|v| v["statement"].as_str().map(str::to_owned))
        .unwrap_or_default();
    c.statements.lock().unwrap().push(statement);
    if c.hold.swap(false, Ordering::SeqCst) {
        c.entered.notify_one();
        c.release.notified().await;
    }
    let mut request = c.http.post(&c.target).body(body);
    for name in ["authorization", "content-type", "accept"] {
        if let Some(value) = headers.get(name) {
            request = request.header(name, value);
        }
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    (status, response.bytes().await.unwrap()).into_response()
}

/// Route this harness's native traffic through a counting proxy.
async fn counted(h: &mut Harness) -> (Arc<Counter>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let counter = Arc::new(Counter {
        http: h.state.http.clone(),
        target: format!("{}/db/neo4j/query/v2", h.state.config.neo4j_url),
        calls: AtomicUsize::new(0),
        statements: std::sync::Mutex::new(vec![]),
        hold: AtomicBool::new(false),
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
    });
    let router = Router::new()
        .route("/db/neo4j/query/v2", axum::routing::post(forward))
        .with_state(counter.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut config = (*h.state.config).clone();
    config.neo4j_url = url;
    h.state.config = Arc::new(config);
    h.router = app(h.state.clone());
    (counter, task)
}
fn calls(c: &Counter) -> usize {
    c.calls.load(Ordering::SeqCst)
}

fn id(value: &Value) -> Uuid {
    value["id"].as_str().unwrap().parse().unwrap()
}
async fn stamp(h: &Harness, generation: Uuid) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar("SELECT native_verified_at FROM graph_generations WHERE id=$1")
        .bind(generation)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}
async fn age(h: &Harness, generation: Uuid) {
    sqlx::query(
        "UPDATE graph_generations SET native_verified_at=now()-interval '25 hours' WHERE id=$1",
    )
    .bind(generation)
    .execute(&h.admin)
    .await
    .unwrap();
}
async fn recent(h: &Harness, generation: Uuid) -> bool {
    sqlx::query_scalar(
        "SELECT native_verified_at>now()-interval '1 minute' FROM graph_generations WHERE id=$1",
    )
    .bind(generation)
    .fetch_one(&h.admin)
    .await
    .unwrap()
}
async fn analytics_epoch(h: &Harness, brain: Uuid) -> i64 {
    sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}
async fn damage_code(h: &Harness, generation: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT error_code FROM graph_generations WHERE id=$1 AND state='ready'")
        .bind(generation)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}
async fn generations(h: &Harness, brain: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM graph_generations WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}
async fn maintain(h: &Harness, brain: Uuid) -> usize {
    let owner: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    graph::maintain_brain(&h.state, brain, owner)
        .await
        .unwrap_or_else(|e| panic!("{}: {}", e.1, e.2))
}
/// Physically remove one relationship of the generation, as an operator or a
/// failing disk might, bypassing PostgreSQL entirely.
async fn delete_first_edge(h: &Harness, brain: Uuid, generation: Uuid) {
    let edge: String = sqlx::query_scalar(
        "SELECT descriptor->'edges'->0->>'id' FROM graph_generations WHERE id=$1",
    )
    .bind(generation)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let removed = cypher(h,"MATCH ()-[r:RECOLLECT_GRAPH_EDGE {brain:$brain,generation:$generation,id:$id}]->() DELETE r RETURN count(r)",json!({"brain":brain,"generation":generation,"id":edge})).await;
    assert_eq!(removed, json!([[1]]));
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_reads_seal_in_one_request_see_damage_at_once_and_survive_unselected_erasure() {
    let mut h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, facts) = materialized(&h, &owner, &base, 'a', None).await;
    let scope =
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}});
    let first = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    assert!(
        stamp(&h, id(&first)).await.is_some(),
        "Readiness is the first verification stamp"
    );
    let (native, _proxy) = counted(&mut h).await;
    let view = format!("{base}/graph/view");
    let path = format!("{base}/graph/path");

    // The happy path is one request however many entities and relationships
    // the view holds, and repeating it re-checks the same way (no cache).
    for _ in 0..2 {
        let before = calls(&native);
        let shown = ok(&h, "POST", &view, &owner, json!({"scope":scope})).await;
        assert_eq!(shown["total_nodes"], 7);
        assert_eq!(shown["total_edges"], 6);
        assert_eq!(calls(&native) - before, 1, "view: one seal request");
        let statements = native.statements.lock().unwrap();
        assert!(
            statements
                .last()
                .unwrap()
                .contains("COUNT{(g)-[:RECOLLECT_GRAPH_MEMBER]->()}")
        );
    }
    // A traversal also proves the relationships it actually walked: seal, native
    // path, and one batch for the two returned relationships.
    let before = calls(&native);
    let found = ok(&h, "POST", &path, &owner, path_input(&scope, &facts)).await;
    assert_eq!(found["edges"].as_array().unwrap().len(), 2);
    assert_eq!(
        calls(&native) - before,
        3,
        "path: seal, search, returned edges"
    );

    // Damage on a warm generation is seen by the very next read.
    delete_first_edge(&h, brain, id(&first)).await;
    for (endpoint, body) in [
        (&view, json!({"scope":scope})),
        (&path, path_input(&scope, &facts)),
    ] {
        let damaged = h.call("POST", endpoint, Some(&owner), body).await;
        assert_eq!(damaged.0, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(damaged.1["code"], "graph_import_mismatch");
    }
    assert!(
        stamp(&h, id(&first)).await.is_none(),
        "A read that fails exact verification clears the stamp so maintenance re-verifies it first"
    );

    // A replacement restores the single-request path.
    let second = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    assert!(stamp(&h, id(&second)).await.is_some());
    let before = calls(&native);
    ok(&h, "POST", &view, &owner, json!({"scope":scope})).await;
    assert_eq!(calls(&native) - before, 1);

    // Reject the bridge fact so the view no longer selects it, then physically
    // erase that unselected entity. Counts now differ from the seal, but nothing
    // the view relies on is missing: the exact fallback passes and reads continue.
    let mut claim = proposal(&facts[1], "Bridge configuration", "unsupported route");
    claim["content"]["selection"] = scope["selection"].clone();
    claim["content"]["supports"][0]["kind"] = json!("repository_fact");
    claim["content"]["supports"][0]["line_from"] = Value::Null;
    claim["content"]["supports"][0]["line_to"] = Value::Null;
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, claim).await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":claim["id"],"action":"reject","reason":"The bridge assertion is rejected by this fixture."})).await;
    let removed = cypher(
        &h,
        "MATCH (n:RecollectGraphEntity {brain:$brain,key:$key}) DETACH DELETE n RETURN count(n)",
        json!({"brain":brain,"key":format!("repository_fact:{}",facts[1].as_str().unwrap())}),
    )
    .await;
    assert_eq!(removed, json!([[1]]));
    let before = calls(&native);
    let shown = ok(&h, "POST", &view, &owner, json!({"scope":scope})).await;
    assert_eq!(
        shown["total_nodes"], 6,
        "The rejected bridge is not selected"
    );
    assert!(
        calls(&native) - before > 1,
        "A seal mismatch falls back to exact verification of the selection"
    );
    let alternative = ok(&h, "POST", &path, &owner, path_input(&scope, &facts)).await;
    assert_eq!(alternative["edges"].as_array().unwrap().len(), 3);
    assert!(
        stamp(&h, id(&second)).await.is_some(),
        "Passing the exact fallback proves nothing is wrong, so it does not clear the stamp"
    );
    // The same fallback still fails closed once a selected entity is damaged.
    let removed = cypher(
        &h,
        "MATCH (n:RecollectGraphEntity {brain:$brain,key:$key}) DETACH DELETE n RETURN count(n)",
        json!({"brain":brain,"key":format!("repository_fact:{}",facts[0].as_str().unwrap())}),
    )
    .await;
    assert_eq!(removed, json!([[1]]));
    let damaged = h
        .call("POST", &view, Some(&owner), json!({"scope":scope}))
        .await;
    assert_eq!(damaged.1["code"], "graph_import_mismatch");

    // The proxy keeps serving through cleanup; dropping the runtime ends it.
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_deep_verification_stamps_ignores_a_raced_token_and_queues_one_rebuild_per_damage() {
    let mut h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, _) = materialized(&h, &owner, &base, 'a', None).await;
    let scope =
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}});
    let first = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let (native, _proxy) = counted(&mut h).await;

    // A due, healthy generation is fully verified and restamped. Bookkeeping
    // must not invalidate analytics or discover new work.
    let epoch = analytics_epoch(&h, brain).await;
    age(&h, id(&first)).await;
    assert_eq!(
        analytics_epoch(&h, brain).await,
        epoch,
        "A stamp-only change is not a canonical input change"
    );
    let before = calls(&native);
    maintain(&h, brain).await;
    assert!(
        recent(&h, id(&first)).await,
        "The deep check restamps a healthy generation"
    );
    assert!(
        calls(&native) > before + 1,
        "The deep check reads every node and relationship"
    );
    assert_eq!(analytics_epoch(&h, brain).await, epoch);
    assert_eq!(generations(&h, brain).await, 1);
    let before = calls(&native);
    maintain(&h, brain).await;
    assert_eq!(
        calls(&native),
        before,
        "A fresh stamp costs no native request"
    );

    // A generation replaced or re-imported while verification is in flight must
    // not receive the stamp of a check that began before the change.
    age(&h, id(&first)).await;
    let original: Uuid =
        sqlx::query_scalar("SELECT native_attempt_token FROM graph_generations WHERE id=$1")
            .bind(id(&first))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    native.hold.store(true, Ordering::SeqCst);
    let state = h.state.clone();
    let owner_id: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let racing = tokio::spawn(async move {
        graph::maintain_brain(&state, brain, owner_id)
            .await
            .map_err(|e| e.1)
    });
    native.entered.notified().await;
    sqlx::query("UPDATE graph_generations SET native_attempt_token=gen_random_uuid() WHERE id=$1")
        .bind(id(&first))
        .execute(&h.admin)
        .await
        .unwrap();
    native.release.notify_one();
    racing.await.unwrap().unwrap();
    assert!(
        !recent(&h, id(&first)).await,
        "A changed sealing token must not be stamped as verified"
    );
    assert_eq!(
        damage_code(&h, id(&first)).await,
        None,
        "The check itself passed"
    );
    sqlx::query("UPDATE graph_generations SET native_attempt_token=$2 WHERE id=$1")
        .bind(id(&first))
        .bind(original)
        .execute(&h.admin)
        .await
        .unwrap();

    // Damage found by the deep check, then by a failing read: each queues exactly
    // one replacement, however many passes follow, and the damaged generation is
    // replaced rather than repaired in place.
    let mut damaged = id(&first);
    for round in 0..2 {
        let baseline = generations(&h, brain).await;
        delete_first_edge(&h, brain, damaged).await;
        if round == 0 {
            age(&h, damaged).await;
        } else {
            let read = h
                .call(
                    "POST",
                    &format!("{base}/graph/view"),
                    Some(&owner),
                    json!({"scope":scope}),
                )
                .await;
            assert_eq!(read.1["code"], "graph_import_mismatch");
            assert!(stamp(&h, damaged).await.is_none());
        }
        maintain(&h, brain).await;
        assert_eq!(
            damage_code(&h, damaged).await.as_deref(),
            Some("graph_import_mismatch"),
            "Round {round}: the damaged ready generation is marked"
        );
        assert_eq!(
            generations(&h, brain).await,
            baseline,
            "The mark itself builds nothing"
        );
        assert_eq!(
            maintain(&h, brain).await,
            1,
            "The next pass queues the replacement"
        );
        for _ in 0..3 {
            assert_eq!(maintain(&h, brain).await, 0, "No rebuild storm");
        }
        assert_eq!(generations(&h, brain).await, baseline + 1);
        // Until it is replaced the generation stays fail-closed for reads.
        let still = h
            .call(
                "POST",
                &format!("{base}/graph/view"),
                Some(&owner),
                json!({"scope":scope}),
            )
            .await;
        assert_eq!(still.1["code"], "graph_import_mismatch");
        // Run the heavy lane: the generation maintenance queued is the one built.
        while worker::run_once(&h.state, "heavy").await.unwrap() {}
        let replacement: Uuid = sqlx::query_scalar("SELECT id FROM graph_generations WHERE brain_id=$1 AND snapshot_id=$2 AND state='ready'")
            .bind(brain)
            .bind(snapshot.as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
        assert_ne!(replacement, damaged);
        assert_eq!(generations(&h, brain).await, baseline + 1);
        assert!(stamp(&h, replacement).await.is_some());
        let state: String = sqlx::query_scalar("SELECT state FROM graph_generations WHERE id=$1")
            .bind(damaged)
            .fetch_one(&h.admin)
            .await
            .unwrap();
        assert_eq!(state, "superseded");
        let shown = ok(
            &h,
            "POST",
            &format!("{base}/graph/view"),
            &owner,
            json!({"scope":scope}),
        )
        .await;
        assert_eq!(shown["generation"]["id"], json!(replacement));
        maintain(&h, brain).await;
        damaged = replacement;
    }

    // The proxy keeps serving through cleanup; dropping the runtime ends it.
    cleanup(&h, brain).await;
    h.finish().await;
}
