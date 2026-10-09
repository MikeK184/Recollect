use super::*;
use axum::{extract::State, http::HeaderMap, response::IntoResponse};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use std::time::Instant;

struct NativeGate {
    http: reqwest::Client,
    target: String,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
    armed: AtomicBool,
}

async fn forward(
    State(gate): State<Arc<NativeGate>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    if gate.armed.swap(false, Ordering::SeqCst) {
        gate.entered.notify_one();
        gate.resume.notified().await;
    }
    let mut request = gate.http.post(&gate.target).body(body);
    for name in ["authorization", "content-type", "accept"] {
        if let Some(value) = headers.get(name) {
            request = request.header(name, value);
        }
    }
    let response = request.send().await.unwrap();
    (response.status(), response.bytes().await.unwrap()).into_response()
}

async fn gated_state(h: &Harness) -> (AppState, Arc<NativeGate>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let gate = Arc::new(NativeGate {
        http: h.state.http.clone(),
        target: format!("{}/db/neo4j/query/v2", h.state.config.neo4j_url),
        entered: tokio::sync::Notify::new(),
        resume: tokio::sync::Notify::new(),
        armed: AtomicBool::new(true),
    });
    let router = Router::new()
        .route("/db/neo4j/query/v2", axum::routing::post(forward))
        .with_state(gate.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut config = (*h.state.config).clone();
    config.neo4j_url = url;
    let mut state = h.state.clone();
    state.config = Arc::new(config);
    (state, gate, server)
}

async fn fixture(h: &Harness, owner: &Login) -> (Uuid, String, Value, Vec<String>) {
    let (brain, base) = base(h, owner).await;
    let (repo, snapshot, facts) = combined::publish(
        h,
        owner,
        &base,
        "example.test/preparation/fixture",
        'a',
        exploration::facts(&[vec![1], vec![]]),
    )
    .await;
    build(h, owner, &base, "repository", Some(&snapshot)).await;
    (
        brain,
        base,
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}}),
        facts
            .iter()
            .map(|f| format!("repository_fact:{}", f["id"].as_str().unwrap()))
            .collect(),
    )
}

fn request(
    router: Router,
    owner: &Login,
    endpoint: String,
    body: Value,
) -> tokio::task::JoinHandle<axum::response::Response> {
    let input = Request::builder()
        .method("POST")
        .uri(endpoint)
        .header("content-type", "application/json")
        .header("cookie", &owner.cookie)
        .header("x-csrf-token", &owner.csrf)
        .body(Body::from(body.to_string()))
        .unwrap();
    tokio::spawn(async move { router.oneshot(input).await.unwrap() })
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_preparation_allows_concurrent_generation_changes_and_discards_buffered_output() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope, keys) = fixture(&h, &owner).await;
    let original_adapter: String = sqlx::query_scalar(
        "SELECT adapter FROM graph_generations WHERE brain_id=$1 AND state='ready'",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    for endpoint in ["view", "explore", "path"] {
        let (state, gate, server) = gated_state(&h).await;
        let input = match endpoint {
            "view" => json!({"scope":scope,"windowed":true}),
            "path" => {
                json!({"scope":scope,"start":keys[0],"end":keys[1],"direction":"both","max_hops":1})
            }
            _ => {
                json!({"scope":scope,"windowed":true,"center":null,"direction":"both","max_hops":1})
            }
        };
        let mut read = request(
            app(state),
            &owner,
            format!("{base}/graph/{endpoint}"),
            input.clone(),
        );
        tokio::select! {
            _ = gate.entered.notified() => {},
            response = &mut read => {
                let response = response.unwrap();
                let status = response.status();
                let body = response.into_body().collect().await.unwrap().to_bytes();
                panic!("{endpoint} failed before physical verification: {status} {}", String::from_utf8_lossy(&body));
            },
            _ = tokio::time::sleep(Duration::from_secs(2)) => panic!("{endpoint} did not reach physical verification"),
        }
        tokio::time::timeout(Duration::from_secs(1), async {
            let mut tx = db::actor_tx(&h.state.pool, actor)
                .await.unwrap_or_else(|e| panic!("{}", e.1));
            db::require_writer(&mut tx, brain)
                .await.unwrap_or_else(|e| panic!("{}", e.1));
            sqlx::query("INSERT INTO evidence_groups(id,brain_id,kind,name,created_by) VALUES($1,$2,'area',$3,$4)")
                .bind(Uuid::new_v4()).bind(brain).bind(endpoint).bind(actor).execute(&mut *tx).await.unwrap();
            sqlx::query("UPDATE graph_generations SET adapter='synthetic-changed-generation' WHERE brain_id=$1 AND state='ready'")
                .bind(brain).execute(&mut *tx).await.unwrap();
            tx.commit().await.unwrap();
        }).await.expect("native I/O must not retain a Brain read lock");
        gate.resume.notify_one();
        let response = read.await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body["code"], "graph_preparation_changed", "{body}");
        assert!(
            body.get("nodes").is_none(),
            "buffered source content must be discarded"
        );
        server.abort();
        sqlx::query("UPDATE graph_generations SET adapter=$2 WHERE brain_id=$1 AND state='ready'")
            .bind(brain)
            .bind(&original_adapter)
            .execute(&h.admin)
            .await
            .unwrap();
        let current = ok(
            &h,
            "POST",
            &format!("{base}/graph/{endpoint}"),
            &owner,
            input,
        )
        .await;
        assert_eq!(current["nodes"].as_array().unwrap().len(), 2);
    }
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_preparation_rechecks_revoked_browser_session_before_returning_content() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope, _) = fixture(&h, &owner).await;
    let (state, gate, server) = gated_state(&h).await;
    let read = request(
        app(state),
        &owner,
        format!("{base}/graph/view"),
        json!({"scope":scope}),
    );
    tokio::time::timeout(Duration::from_secs(2), gate.entered.notified())
        .await
        .unwrap();
    let token: Uuid = owner.cookie.split_once('=').unwrap().1.parse().unwrap();
    tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query("DELETE FROM sessions WHERE token=$1")
            .bind(token)
            .execute(&h.admin),
    )
    .await
    .unwrap()
    .unwrap();
    gate.resume.notify_one();
    let response = read.await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    server.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_preparation_rechecks_changed_collection_membership_during_native_io() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let group = ok(&h,"POST",&format!("{base}/evidence/groups"),&owner,json!({"kind":"collection","name":"Synthetic graph scope","description":"Controlled fixture"})).await;
    let source = ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Synthetic graph scope evidence","media_type":"text/plain","content":"The synthetic app uses port 8080.","retain_content":true,"group_ids":[group["id"]]})).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(&source["version"]["id"], "Synthetic app port", "8080"),
    )
    .await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":claim["id"],"action":"accept","reason":"The retained synthetic declaration was inspected."})).await;
    build(&h, &owner, &base, "knowledge", None).await;
    let scope = json!({"kind":"knowledge","collection_id":group["id"]});
    let groups_endpoint = format!("{base}/sources/{}/groups", source["id"].as_str().unwrap());
    let mut outcomes = vec![];
    for endpoint in ["view", "explore"] {
        let input = if endpoint == "view" {
            json!({"scope":scope,"windowed":true})
        } else {
            json!({"scope":scope,"windowed":true,"center":null,"direction":"both","max_hops":1})
        };
        let (state, gate, server) = gated_state(&h).await;
        let mut read = request(
            app(state),
            &owner,
            format!("{base}/graph/{endpoint}"),
            input.clone(),
        );
        tokio::select! {
            _=gate.entered.notified()=>{},
            response=&mut read=>panic!("Read finished before the controlled membership change: {}",response.unwrap().status()),
            _=tokio::time::sleep(Duration::from_secs(2))=>panic!("Read did not reach native verification"),
        }
        tokio::time::timeout(
            Duration::from_secs(1),
            ok(&h, "PUT", &groups_endpoint, &owner, json!({"group_ids":[]})),
        )
        .await
        .expect("Membership mutation must commit while native work is paused");
        gate.resume.notify_one();
        let response = read.await.unwrap();
        let status = response.status();
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        outcomes.push((status, body));
        server.abort();
        ok(
            &h,
            "PUT",
            &groups_endpoint,
            &owner,
            json!({"group_ids":[group["id"]]}),
        )
        .await;
        let fresh = ok(
            &h,
            "POST",
            &format!("{base}/graph/{endpoint}"),
            &owner,
            input,
        )
        .await;
        assert_eq!(
            fresh["nodes"].as_array().unwrap().len(),
            2,
            "Restoring scope must make the actual current source and claim visible"
        );
    }
    let requests: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    cleanup(&h, brain).await;
    h.finish().await;
    assert_eq!(requests, 0);
    for (status, body) in outcomes {
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["code"], "graph_preparation_changed");
        assert!(
            body.get("nodes").is_none(),
            "Lost-scope buffered content must be discarded"
        );
    }
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_preparation_10k_reads_during_capture_count_all_outcomes() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let proof_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/automatic-mapping");
    std::fs::create_dir_all(&proof_dir).unwrap();
    std::fs::write(
        proof_dir.join("graph-10k-fixture.json"),
        serde_json::to_vec_pretty(
            &json!({"database":h.database,"brain":brain,"purpose":"owned 10k concurrency fixture"}),
        )
        .unwrap(),
    )
    .unwrap();
    let links: Vec<Vec<usize>> = (0..10_000)
        .map(|i| if i + 1 < 10_000 { vec![i + 1] } else { vec![] })
        .collect();
    let (repo, snapshot, _records) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/preparation/10k",
        'b',
        exploration::facts(&links),
    )
    .await;
    let generation = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    assert_eq!(generation["node_count"], 10_000);
    assert_eq!(generation["edge_count"], 9_999);
    let scope =
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}});
    // The public fact page is bounded; select the middle control by its exact
    // canonical snapshot rather than pretending that first page contains 10k.
    let center_id: Uuid = sqlx::query_scalar("SELECT id FROM repository_facts WHERE brain_id=$1 AND snapshot_id=$2 AND record->>'name'='node_5000'")
        .bind(brain).bind(snapshot.as_str().unwrap().parse::<Uuid>().unwrap())
        .fetch_one(&h.admin).await.unwrap();
    let center = format!("repository_fact:{center_id}");
    let reading = AtomicBool::new(true);
    let reads = async {
        let mut outcomes = vec![];
        for wave in 0..20 {
            let harness = &h;
            let path = &base;
            let login = &owner;
            let read = |kind: &'static str, input: Value| async move {
                let started = Instant::now();
                let (status, body, _) = harness
                    .call("POST", &format!("{path}/graph/{kind}"), Some(login), input)
                    .await;
                json!({"wave":wave,"endpoint":kind,"status":status.as_u16(),"seconds":started.elapsed().as_secs_f64(),"code":body.get("code"),"nodes":body.get("nodes").and_then(Value::as_array).map(Vec::len)})
            };
            let (overview, neighborhood) = tokio::join!(
                read("view", json!({"scope":scope,"windowed":true})),
                read(
                    "explore",
                    json!({"scope":scope,"windowed":true,"center":center,"direction":"both","max_hops":1})
                ),
            );
            outcomes.extend([overview, neighborhood]);
        }
        reading.store(false, Ordering::SeqCst);
        outcomes
    };
    let capture = async {
        let mut writes = vec![];
        while reading.load(Ordering::SeqCst) {
            let started = Instant::now();
            let (status, body, _) = h.call("POST", &format!("{base}/sources"), Some(&owner),
                json!({"title":format!("Background capture {}",writes.len()),"media_type":"text/plain","retain_content":true,"content":"Synthetic capture during graph reads.\n"})).await;
            writes.push(json!({"status":status.as_u16(),"seconds":started.elapsed().as_secs_f64(),"code":body.get("code")}));
            if status == StatusCode::OK {
                worker::run_once(&h.state, "capture").await.unwrap();
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        writes
    };
    let (outcomes, writes) = tokio::join!(reads, capture);
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let mut seconds: Vec<f64> = outcomes
        .iter()
        .map(|r| r["seconds"].as_f64().unwrap())
        .collect();
    seconds.sort_by(f64::total_cmp);
    let p95 = seconds[(seconds.len() * 95).div_ceil(100) - 1];
    let failures = outcomes.iter().filter(|r| r["status"] != 200).count();
    let receipt = json!({"fixture":"10k chain","nodes":10_000,"edges":9_999,"read_concurrency":2,"requested":outcomes.len(),"failures":failures,"p95_seconds_all_outcomes":p95,"model_requests":calls,"reads":outcomes,"capture_writes":writes});
    std::fs::write(
        proof_dir.join("graph-10k-concurrency.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    cleanup(&h, brain).await;
    h.finish().await;
    assert_eq!(calls, 0, "The fixture must make no model request");
    assert_eq!(receipt["requested"], 40);
    assert!(!receipt["capture_writes"].as_array().unwrap().is_empty());
    assert!(
        receipt["capture_writes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["status"] == 200),
        "Capture failed: {receipt}"
    );
    assert_eq!(
        failures, 0,
        "Graph failures are part of the denominator; see retained receipt"
    );
    assert!(
        p95 < 2.0,
        "p95 {p95:.3}s exceeds the populated-read target; see retained receipt"
    );
    assert!(
        receipt["reads"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["nodes"].as_u64().is_some_and(|n| n <= 250))
    );
}
