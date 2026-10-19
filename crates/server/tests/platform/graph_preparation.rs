use super::*;
use axum::{extract::State, http::HeaderMap, response::IntoResponse};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;
use std::time::Instant;

struct NativeGate {
    http: reqwest::Client,
    calls: AtomicUsize,
    target: String,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
    armed: AtomicBool,
    nodes_only: AtomicBool,
    import_bodies: std::sync::Mutex<Vec<Value>>,
    import_counts: std::sync::Mutex<Vec<Option<u64>>>,
    imports: std::sync::Mutex<Vec<Instant>>,
    import_windows: std::sync::Mutex<Vec<(Instant, Instant)>>,
}

async fn forward(
    State(gate): State<Arc<NativeGate>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    // A proxy may already have accepted a request when its caller disconnects.
    // Keep that actual outbound operation alive to exercise native fencing.
    tokio::spawn(async move {
        gate.calls.fetch_add(1, Ordering::SeqCst);
        let importing = serde_json::from_slice::<Value>(&body)
            .ok()
            .and_then(|v| v["statement"].as_str().map(str::to_owned))
            .is_some_and(|query| {
                query.contains("MERGE (g:RecollectGraphGeneration")
                    || query.contains("MERGE (n:RecollectGraphEntity")
            });
        let import_started = Instant::now();
        if importing {
            gate.imports.lock().unwrap().push(import_started);
            gate.import_bodies
                .lock()
                .unwrap()
                .push(serde_json::from_slice(&body).unwrap());
        }
        let selected = !gate.nodes_only.load(Ordering::SeqCst)
            || serde_json::from_slice::<Value>(&body).unwrap()["statement"]
                .as_str()
                .unwrap()
                .contains("MERGE (n:RecollectGraphEntity");
        if selected && gate.armed.swap(false, Ordering::SeqCst) {
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
        let status = response.status();
        let bytes = response.bytes().await.unwrap();
        if importing {
            gate.import_windows
                .lock()
                .unwrap()
                .push((import_started, Instant::now()));
            gate.import_counts.lock().unwrap().push(
                serde_json::from_slice::<Value>(&bytes)
                    .ok()
                    .and_then(|body| body["data"]["values"][0][0].as_u64()),
            );
        }
        (status, bytes).into_response()
    })
    .await
    .unwrap()
}

async fn gated_state(h: &Harness) -> (AppState, Arc<NativeGate>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let gate = Arc::new(NativeGate {
        http: h.state.http.clone(),
        calls: AtomicUsize::new(0),
        target: format!("{}/db/neo4j/query/v2", h.state.config.neo4j_url),
        entered: tokio::sync::Notify::new(),
        resume: tokio::sync::Notify::new(),
        armed: AtomicBool::new(true),
        nodes_only: AtomicBool::new(false),
        import_bodies: std::sync::Mutex::new(Vec::new()),
        import_counts: std::sync::Mutex::new(Vec::new()),
        imports: std::sync::Mutex::new(Vec::new()),
        import_windows: std::sync::Mutex::new(Vec::new()),
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

async fn install_contention_gate(h: &Harness) {
    sqlx::raw_sql("ALTER FUNCTION recollect_content_state(uuid,text,text,timestamptz) RENAME TO fixture_graph_content_state;
      CREATE FUNCTION recollect_content_state(b uuid,k text,s text,at_time timestamptz) RETURNS text
      LANGUAGE plpgsql STABLE SET search_path=public,pg_temp AS $$ BEGIN
       IF current_setting('recollect.preparation',true)='on' THEN PERFORM pg_advisory_xact_lock(67126124,9); END IF;
       RETURN fixture_graph_content_state(b,k,s,at_time);
      END $$;")
        .execute(&h.admin).await.unwrap();
}
async fn contend_before_native(h: &Harness, state: &AppState, job: &worker::ClaimedJob) {
    let mut hold = h.admin.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(67126124,9)")
        .execute(&mut *hold)
        .await
        .unwrap();
    let mut running = Box::pin(graph::execute(state, job));
    let blocked = async {
        loop {
            let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND classid=67126124 AND objid=9 AND NOT granted)")
                .fetch_one(&h.admin).await.unwrap();
            if waiting {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::select! {
        _=blocked=>{},
        result=&mut running=>panic!("Preparation finished before the real canonical gate: {result:?}"),
        _=tokio::time::sleep(Duration::from_secs(3))=>panic!("Canonical preparation did not reach test gate"),
    }
    let mut tx = db::actor_tx(&h.state.pool, job.actor_id)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    db::require_writer(&mut tx, job.brain_id)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    sqlx::query("INSERT INTO evidence_groups(id,brain_id,kind,name,created_by) VALUES($1,$2,'area','Concurrent preparation '||$1::text,$3)")
        .bind(Uuid::new_v4()).bind(job.brain_id).bind(job.actor_id).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    sqlx::query("SELECT pg_advisory_unlock(67126124,9)")
        .execute(&mut *hold)
        .await
        .unwrap();
    running.await.unwrap();
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_preparation_contention_budget_and_native_attempt_ceiling_are_separate() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope, _) = fixture(&h, &owner).await;
    install_contention_gate(&h).await;
    let queued = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &owner,
        json!({"kind":"repository","snapshot_id":scope["snapshot_id"]}),
    )
    .await;
    let id: Uuid = queued["id"].as_str().unwrap().parse().unwrap();
    let (state, gate, proxy) = gated_state(&h).await;
    gate.armed.store(false, Ordering::SeqCst);
    for ordinal in 0..9 {
        let job = worker::claim(&h.state.pool, "heavy")
            .await
            .unwrap()
            .unwrap();
        contend_before_native(&h, &state, &job).await;
        let row:(String,i32,i32,Option<Uuid>)=sqlx::query_as("SELECT j.state,j.attempts,j.preparation_deferrals,g.native_attempt_token FROM jobs j JOIN graph_generations g ON g.job_id=j.id WHERE g.id=$1")
            .bind(id).fetch_one(&h.admin).await.unwrap();
        assert_eq!(
            gate.calls.load(Ordering::SeqCst),
            0,
            "Refunded claims must issue NO native request, including schema/setup"
        );
        assert_eq!(row.3, None);
        assert_eq!(row.2, (ordinal + 1).min(8));
        let again: bool = sqlx::query_scalar(
            "SELECT recollect_fail_job($1,$2,'queued','graph_preparation_contended')",
        )
        .bind(job.id)
        .bind(job.lease_token)
        .fetch_one(&h.state.pool)
        .await
        .unwrap();
        assert!(
            !again,
            "Old lease cannot refund twice or reset the durable timer"
        );
        if ordinal < 8 {
            assert_eq!((row.0, row.1), ("queued".into(), 0));
            assert!(
                worker::claim(&h.state.pool, "heavy")
                    .await
                    .unwrap()
                    .is_none(),
                "Existing backoff is enforced"
            );
            tokio::time::sleep(Duration::from_millis(1100)).await;
        } else {
            assert_eq!((row.0, row.1), ("failed".into(), 1));
        }
    }
    let next = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &owner,
        json!({"kind":"repository","snapshot_id":scope["snapshot_id"]}),
    )
    .await;
    let next_id: Uuid = next["id"].as_str().unwrap().parse().unwrap();
    for ordinal in 1..=3 {
        let job = worker::claim(&h.state.pool, "heavy")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(job.target_id, next_id);
        // Exact persistent phase marker is written before any outbound native
        // request. Such a claim can never use the preparation-only refund.
        sqlx::query("UPDATE graph_generations SET native_attempt_token=$2 WHERE id=$1")
            .bind(next_id)
            .bind(job.lease_token)
            .execute(&h.admin)
            .await
            .unwrap();
        let refund: bool = sqlx::query_scalar(
            "SELECT recollect_fail_job($1,$2,'queued','graph_preparation_contended')",
        )
        .bind(job.id)
        .bind(job.lease_token)
        .fetch_one(&h.state.pool)
        .await
        .unwrap();
        assert!(!refund);
        let failed: bool = sqlx::query_scalar(
            "SELECT recollect_fail_job($1,$2,'queued','graph_preparation_changed')",
        )
        .bind(job.id)
        .bind(job.lease_token)
        .fetch_one(&h.state.pool)
        .await
        .unwrap();
        assert!(failed);
        let row: (String, i32, i32) =
            sqlx::query_as("SELECT state,attempts,preparation_deferrals FROM jobs WHERE id=$1")
                .bind(job.id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        assert_eq!(row.1, ordinal);
        assert_eq!(row.2, 0);
        assert_eq!(row.0, if ordinal < 3 { "queued" } else { "failed" });
        if ordinal < 3 {
            tokio::time::sleep(Duration::from_millis(if ordinal == 1 {
                1100
            } else {
                2100
            }))
            .await;
        }
    }
    proxy.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_preparation_windowed_replacement_retries_pin_the_original_snapshot() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, mut scope, _) = fixture(&h, &owner).await;
    let snapshot = scope["snapshot_id"].clone();
    scope["snapshot_id"] = Value::Null;
    let (state, gate, proxy) = gated_state(&h).await;
    let mut reading = request(
        app(state),
        &owner,
        format!("{base}/graph/view"),
        json!({"scope":scope,"windowed":true}),
    );
    tokio::select! {
        _=gate.entered.notified()=>{},
        response=&mut reading=>panic!("Read finished before native verification: {}",response.unwrap().status()),
        _=tokio::time::sleep(Duration::from_secs(3))=>panic!("Read did not reach native verification"),
    }
    let replacement = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &owner,
        json!({"kind":"repository","snapshot_id":snapshot}),
    )
    .await;
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    // A newer repository head has no graph yet. Reprepare must keep the exact
    // immutable snapshot already selected by this request, not follow the head.
    combined::publish(
        &h,
        &owner,
        &base,
        "example.test/preparation/fixture",
        'b',
        exploration::facts(&[vec![1], vec![2], vec![]]),
    )
    .await;
    gate.resume.notify_one();
    let response = reading.await.unwrap();
    let status = response.status();
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["scope"]["snapshot_id"], snapshot);
    assert_eq!(body["generation"]["id"], replacement["id"]);
    assert_eq!(body["nodes"].as_array().unwrap().len(), 2);
    proxy.abort();
    cleanup(&h, brain).await;
    h.finish().await;
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

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_preparation_import_progress_does_not_take_the_brain_lock() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope, _) = fixture(&h, &owner).await;
    let generation = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &owner,
        json!({"kind":"repository","snapshot_id":scope["snapshot_id"]}),
    )
    .await;
    let id: Uuid = generation["id"].as_str().unwrap().parse().unwrap();
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.target_id, id);
    let (state, gate, proxy_task) = gated_state(&h).await;
    let observe = async {
        tokio::time::timeout(Duration::from_secs(3), gate.entered.notified())
            .await
            .unwrap();
        // A reader holds the shared Brain lock for the whole import. Progress
        // is job metadata; an exclusive-lock update per batch would wait here.
        let mut reader = db::actor_tx(&h.state.pool, job.actor_id)
            .await
            .ok()
            .unwrap();
        db::lock_brain(&mut reader, brain, false)
            .await
            .ok()
            .unwrap();
        gate.resume.notify_one();
        let advanced = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let progress: i16 = sqlx::query_scalar("SELECT progress FROM jobs WHERE id=$1")
                    .bind(job.id)
                    .fetch_one(&h.admin)
                    .await
                    .unwrap();
                if progress > 1 {
                    break progress;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;
        reader.rollback().await.unwrap();
        advanced
    };
    let (execution, advanced) = tokio::join!(graph::execute(&state, &job), observe);
    advanced.expect("Progress must not wait for the exclusive Brain lock");
    execution.unwrap();
    let ready: String = sqlx::query_scalar("SELECT state FROM graph_generations WHERE id=$1")
        .bind(id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(ready, "ready");
    proxy_task.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_preparation_stale_import_requeues_without_waiting_for_lease_expiry() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope, _) = fixture(&h, &owner).await;
    let generation = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &owner,
        json!({"kind":"repository","snapshot_id":scope["snapshot_id"]}),
    )
    .await;
    let id: Uuid = generation["id"].as_str().unwrap().parse().unwrap();
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.target_id, id);
    let (state, gate, proxy_task) = gated_state(&h).await;
    let mutation = async {
        tokio::time::timeout(Duration::from_secs(3), gate.entered.notified())
            .await
            .unwrap();
        // Actual canonical mutation while native setup is paused. The worker's
        // complete import must be rejected by its unchanged final epoch gate.
        let mut tx = db::actor_tx(&h.state.pool, job.actor_id)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1));
        db::require_writer(&mut tx, brain)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1));
        sqlx::query("INSERT INTO evidence_groups(id,brain_id,kind,name,created_by) VALUES($1,$2,'area','Import race',$3)")
            .bind(Uuid::new_v4()).bind(brain).bind(job.actor_id).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        gate.resume.notify_one();
    };
    let (execution, ()) = tokio::join!(graph::execute(&state, &job), mutation);
    execution.unwrap();
    let row: (String,String,Option<String>,bool,bool,i32) = sqlx::query_as(
        "SELECT g.state,j.state,j.error_code,j.lease_token IS NULL,j.not_before<=clock_timestamp()+interval '2 seconds',j.attempts FROM jobs j JOIN graph_generations g ON g.job_id=j.id WHERE j.id=$1")
        .bind(job.id).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        row,
        (
            "queued".into(),
            "queued".into(),
            Some("graph_preparation_changed".into()),
            true,
            true,
            1
        )
    );
    let rejected: bool =
        sqlx::query_scalar("SELECT recollect_fail_job($1,$2,'queued','graph_preparation_changed')")
            .bind(job.id)
            .bind(job.lease_token)
            .fetch_one(&h.state.pool)
            .await
            .unwrap();
    assert!(
        !rejected,
        "The old lease cannot change the queued job again"
    );
    tokio::time::sleep(Duration::from_millis(1100)).await;
    install_contention_gate(&h).await;
    let claimed = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    let before_native = gate.calls.load(Ordering::SeqCst);
    contend_before_native(&h, &state, &claimed).await;
    assert_eq!(gate.calls.load(Ordering::SeqCst), before_native);
    let deferred:(i32,i32,Option<Uuid>)=sqlx::query_as("SELECT j.attempts,j.preparation_deferrals,g.native_attempt_token FROM jobs j JOIN graph_generations g ON g.job_id=j.id WHERE j.id=$1")
        .bind(job.id).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        deferred,
        (1, 1, Some(job.lease_token)),
        "A prior native import remains charged; only the later no-I/O claim is refunded"
    );
    tokio::time::sleep(Duration::from_millis(2100)).await;
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let native=cypher(&h,"MATCH(g:RecollectGraphGeneration {brain:$brain,id:$generation}) RETURN g.import_attempt AS attempt",json!({"brain":brain,"generation":id})).await;
    assert_eq!(
        native,
        json!([[2]]),
        "Actual native attempt ordinals still advance after a refunded pre-import claim"
    );
    let ready: String = sqlx::query_scalar("SELECT state FROM graph_generations WHERE id=$1")
        .bind(id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(ready, "ready");
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    proxy_task.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_preparation_new_attempt_fences_delayed_native_batches_and_setup() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope, _) = fixture(&h, &owner).await;
    let queued = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &owner,
        json!({"kind":"repository","snapshot_id":scope["snapshot_id"]}),
    )
    .await;
    let generation: Uuid = queued["id"].as_str().unwrap().parse().unwrap();
    let first = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.target_id, generation);
    let (state, gate, proxy) = gated_state(&h).await;
    gate.nodes_only.store(true, Ordering::SeqCst);
    let mut old = Box::pin(graph::execute(&state, &first));
    tokio::select! {
        _=gate.entered.notified()=>{},
        result=&mut old=>panic!("Old import finished before actual node batch pause: {result:?}"),
        _=tokio::time::sleep(Duration::from_secs(3))=>panic!("Old import did not reach node batch"),
    }
    // Expire the actual durable lease while its native HTTP request is waiting.
    // The newer attempt is claimed normally, never by forging a lease token.
    sqlx::query("UPDATE jobs SET lease_until=clock_timestamp()-interval '1 second' WHERE id=$1 AND lease_token=$2")
        .bind(first.id).bind(first.lease_token).execute(&h.admin).await.unwrap();
    let second = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.id, first.id);
    assert_ne!(second.lease_token, first.lease_token);
    graph::execute(&state, &second).await.unwrap();
    let ready: String = sqlx::query_scalar("SELECT state FROM graph_generations WHERE id=$1")
        .bind(generation)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(ready, "ready");
    let bodies = gate.import_bodies.lock().unwrap().clone();
    let first_setup = bodies
        .iter()
        .find(|body| {
            body["statement"]
                .as_str()
                .unwrap()
                .contains("SET g.import_attempt")
        })
        .unwrap()
        .clone();
    let newest_nodes = bodies
        .iter()
        .rev()
        .find(|body| {
            body["statement"]
                .as_str()
                .unwrap()
                .contains("MERGE (n:RecollectGraphEntity")
        })
        .unwrap()
        .clone();
    // A delayed old setup cannot reset a newer generation. A replayed newest
    // node batch cannot write after the import was sealed and published ready.
    for body in [first_setup, newest_nodes] {
        let rows = cypher(
            &h,
            body["statement"].as_str().unwrap(),
            body["parameters"].clone(),
        )
        .await;
        assert_eq!(rows, json!([[0]]));
    }
    drop(old); // Model the cancelled worker future; the server received its I/O.
    let completed = gate.import_counts.lock().unwrap().len();
    gate.resume.notify_one();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if gate.import_counts.lock().unwrap().len() > completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        gate.import_counts.lock().unwrap().last(),
        Some(&Some(0)),
        "Late old HTTP request must be rejected by native attempt fencing"
    );
    let counts=cypher(&h,"MATCH (g:RecollectGraphGeneration {brain:$brain,id:$generation}) RETURN g.import_attempt AS attempt,g.import_open AS open,COUNT { (g)-[:RECOLLECT_GRAPH_MEMBER]->() } AS nodes",json!({"brain":brain,"generation":generation})).await;
    assert_eq!(counts, json!([[2, false, 2]]));
    let edges=cypher(&h,"MATCH ()-[r:RECOLLECT_GRAPH_EDGE {brain:$brain,generation:$generation}]->() RETURN count(r) AS count",json!({"brain":brain,"generation":generation})).await;
    assert_eq!(edges, json!([[1]]));
    let entities = cypher(
        &h,
        "MATCH(n:RecollectGraphEntity {brain:$brain}) RETURN count(n) AS count",
        json!({"brain":brain}),
    )
    .await;
    assert_eq!(
        entities,
        json!([[2]]),
        "Shared immutable entities must survive retry reset"
    );
    let page = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":scope,"windowed":true}),
    )
    .await;
    assert_eq!(page["nodes"].as_array().unwrap().len(), 2);
    let models: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(models, 0);
    proxy.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j; loopback model fixture only"]
async fn graph_preparation_10k_reads_during_capture_audits_and_actual_rebuilds() {
    let mut h = Harness::new().await;
    let (provider, provider_task) = crate::models::configure_provider(&mut h).await;
    provider.delay.store(20, Ordering::SeqCst);
    *provider.candidates.lock().unwrap() = json!({"claims":[{
        "subject":"Amber","predicate":"port","value":"8080","rationale":"Declared in the exact synthetic source.",
        "line_from":1,"line_to":1,"replaces_revision":null,"context_role":"convention"}],"retirements":[]});
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let proof_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/automatic-mapping");
    std::fs::create_dir_all(&proof_dir).unwrap();
    std::fs::write(proof_dir.join("graph-10k-mixed-fixture.json"),serde_json::to_vec_pretty(
        &json!({"database":h.database,"brain":brain,"purpose":"owned mixed graph/capture/audit/rebuild concurrency fixture"})).unwrap()).unwrap();
    let links: Vec<Vec<usize>> = (0..10_000)
        .map(|i| if i + 1 < 10_000 { vec![i + 1] } else { vec![] })
        .collect();
    let (repo, snapshot, _) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/preparation/mixed-10k",
        'c',
        exploration::facts(&links),
    )
    .await;
    let generation = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    assert_eq!(generation["node_count"], 10_000);
    assert_eq!(generation["edge_count"], 9_999);
    let scope =
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}});
    let center_id: Uuid=sqlx::query_scalar("SELECT id FROM repository_facts WHERE brain_id=$1 AND snapshot_id=$2 AND record->>'name'='node_5000'")
        .bind(brain).bind(snapshot.as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    let center = format!("repository_fact:{center_id}");
    let evidence=ok(&h,"POST",&format!("{base}/sources"),&owner,
        json!({"title":"Mixed audit evidence","media_type":"text/plain","retain_content":true,"content":"Amber.port = 8080\n"})).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    // Unreviewed canonical assertions require real verifier jobs; none is
    // pre-populated with a successful assessment or a forged provider receipt.
    for index in 0..16 {
        ok(
            &h,
            "POST",
            &format!("{base}/claims"),
            &owner,
            proposal(
                &evidence["version"]["id"],
                &format!("Amber audit {index}"),
                "8080",
            ),
        )
        .await;
    }
    crate::models::allow(&h, &owner, &base, |p| {
        p["autonomous_memory"] = json!(true);
        p["purposes"] = json!(["extraction", "synthesis"]);
        p["content_classes"] = json!(["document", "claim", "query", "support_excerpt"]);
    })
    .await;
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let first_audits =
        recollect_server::autonomous::support_discovery::prepare(&h.state, brain, actor)
            .await
            .ok()
            .unwrap()
            .unwrap();
    assert!(
        recollect_server::autonomous::support_discovery::publish(&h.state, first_audits)
            .await
            .ok()
            .unwrap()
            > 0,
        "The timed series must begin with actual verifier work available"
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let (heavy_state, native, proxy_task) = gated_state(&h).await;
    native.armed.store(false, Ordering::SeqCst);
    let wall_started: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let started = Instant::now();
    let reading = AtomicBool::new(true);
    let rebuilds_done = AtomicBool::new(false);
    let reads = async {
        let mut outcomes = Vec::new();
        // Sustain every request through actual import and ready publication.
        for wave in 0..600 {
            let read = |kind: &'static str, input: Value| {
                let h = &h;
                let base = &base;
                let owner = &owner;
                let started = &started;
                async move {
                    let began = Instant::now();
                    let offset = started.elapsed().as_secs_f64();
                    let (status, body, _) = h
                        .call("POST", &format!("{base}/graph/{kind}"), Some(owner), input)
                        .await;
                    json!({"wave":wave,"endpoint":kind,"status":status.as_u16(),"seconds":began.elapsed().as_secs_f64(),"started_seconds":offset,"finished_seconds":started.elapsed().as_secs_f64(),"code":body.get("code"),"nodes":body.get("nodes").and_then(Value::as_array).map(Vec::len)})
                }
            };
            let (overview, centered) = tokio::join!(
                read("view", json!({"scope":scope,"windowed":true})),
                read(
                    "explore",
                    json!({"scope":scope,"windowed":true,"center":center,"direction":"both","max_hops":1})
                )
            );
            outcomes.extend([overview, centered]);
            if wave >= 19 && rebuilds_done.load(Ordering::SeqCst) {
                break;
            }
            if started.elapsed() > Duration::from_secs(300) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        reading.store(false, Ordering::SeqCst);
        outcomes
    };
    let captures = async {
        let mut outcomes = Vec::new();
        for index in 0..8 {
            if !reading.load(Ordering::SeqCst) {
                break;
            }
            let began = Instant::now();
            let offset = started.elapsed().as_secs_f64();
            let (status,body,_)=h.call("POST",&format!("{base}/sources"),Some(&owner),
                json!({"title":format!("Mixed capture {index}"),"media_type":"text/plain","retain_content":true,"content":"Amber.port = 8080\n"})).await;
            let processed = if status == StatusCode::OK {
                worker::run_once(&h.state, "capture").await.map(Some)
            } else {
                Ok(None)
            };
            outcomes.push(json!({"status":status.as_u16(),"code":body.get("code"),"started_seconds":offset,"seconds":began.elapsed().as_secs_f64(),"capture_processed":processed.ok().flatten()}));
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        outcomes
    };
    let audits = async {
        let mut outcomes = Vec::new();
        for round in 0..16 {
            if !reading.load(Ordering::SeqCst) && round > 0 {
                break;
            }
            let began = Instant::now();
            let offset = started.elapsed().as_secs_f64();
            let mut jobs = Vec::new();
            for _ in 0..4 {
                let step_started = started.elapsed().as_secs_f64();
                let one = worker::run_once(&h.state, "model").await;
                jobs.push(json!({"started_seconds":step_started,"finished_seconds":started.elapsed().as_secs_f64(),"executed":one.as_ref().ok().copied()}));
                if !matches!(one, Ok(true)) {
                    break;
                }
            }
            let queued = recollect_server::autonomous::run_once(&h.state).await;
            let queued_count = queued.as_ref().ok().copied();
            let queued_error = queued.err().map(|e| e.1);
            outcomes.push(json!({"round":round,"started_seconds":offset,"seconds":began.elapsed().as_secs_f64(),"queued":queued_count,"error":queued_error,"worker_steps":jobs}));
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        outcomes
    };
    let rebuilds = async {
        let mut outcomes = Vec::new();
        for ordinal in 0..2 {
            let offset = started.elapsed().as_secs_f64();
            let (status, body, _) = h
                .call(
                    "POST",
                    &format!("{base}/graph/rebuild"),
                    Some(&owner),
                    json!({"kind":"repository","snapshot_id":snapshot}),
                )
                .await;
            if status != StatusCode::OK {
                outcomes.push(json!({"ordinal":ordinal,"status":status.as_u16(),"code":body.get("code"),"started_seconds":offset}));
                continue;
            }
            let id = body["id"].as_str().unwrap().parse::<Uuid>().unwrap();
            let mut attempts = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(180);
            while Instant::now() < deadline {
                let attempt = Instant::now();
                let executed = worker::run_once(&heavy_state, "heavy").await;
                let row: (String, Option<String>, String, Option<String>, i32, i32) = sqlx::query_as(
                    "SELECT g.state,g.error_code,j.state,j.error_code,j.attempts,j.preparation_deferrals FROM graph_generations g JOIN jobs j ON j.id=g.job_id WHERE g.brain_id=$1 AND g.id=$2",
                )
                .bind(brain)
                .bind(id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
                attempts.push(json!({"started_seconds":started.elapsed().as_secs_f64()-attempt.elapsed().as_secs_f64(),"seconds":attempt.elapsed().as_secs_f64(),"worker_step":executed.as_ref().ok().copied(),"worker_error":executed.err().map(|e|e.to_string()),"state":row.0,"code":row.1,"job_state":row.2,"job_code":row.3,"job_attempts":row.4,"preparation_deferrals":row.5}));
                if !matches!(row.0.as_str(), "queued" | "running") {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            outcomes.push(json!({"ordinal":ordinal,"status":status.as_u16(),"generation":id,"started_seconds":offset,"finished_seconds":started.elapsed().as_secs_f64(),"attempts":attempts}));
        }
        rebuilds_done.store(true, Ordering::SeqCst);
        outcomes
    };
    let (outcomes, captures, audits, rebuilds) = tokio::join!(reads, captures, audits, rebuilds);
    let last_read = outcomes
        .iter()
        .filter_map(|row| row["finished_seconds"].as_f64())
        .fold(0.0, f64::max);
    let native_import_offsets: Vec<f64> = native
        .imports
        .lock()
        .unwrap()
        .iter()
        .map(|at| at.duration_since(started).as_secs_f64())
        .collect();
    let import_windows: Vec<Value> = native.import_windows.lock().unwrap().iter().map(|(from,to)| {
        let from = from.duration_since(started).as_secs_f64();
        let to = to.duration_since(started).as_secs_f64();
        let overlapping = outcomes.iter().filter(|read|read["started_seconds"].as_f64().unwrap()<to && read["finished_seconds"].as_f64().unwrap()>from).count();
        json!({"started_seconds":from,"finished_seconds":to,"overlapping_read_requests":overlapping})
    }).collect();
    let audit_completions_during_reads:i64=sqlx::query_scalar("SELECT count(*) FROM memory_support_assessments WHERE brain_id=$1 AND source_stage_id IS NULL AND source_handover_stage_id IS NULL AND state='succeeded' AND finished_at>=$2 AND finished_at<=$2+($3::double precision*interval '1 second')")
        .bind(brain).bind(wall_started).bind(last_read).fetch_one(&h.admin).await.unwrap();
    let mut seconds: Vec<f64> = outcomes
        .iter()
        .map(|row| row["seconds"].as_f64().unwrap())
        .collect();
    seconds.sort_by(f64::total_cmp);
    let p95 = seconds[(seconds.len() * 95).div_ceil(100) - 1];
    let failures = outcomes.iter().filter(|row| row["status"] != 200).count();
    let audit_states:Vec<(String,i64)>=sqlx::query_as("SELECT state,count(*) FROM memory_support_assessments WHERE brain_id=$1 AND source_stage_id IS NULL AND source_handover_stage_id IS NULL GROUP BY state ORDER BY state")
        .bind(brain).fetch_all(&h.admin).await.unwrap();
    let cursor_steps:i64=sqlx::query_scalar("SELECT coalesce(sum(version),0)::bigint FROM autonomous_maintenance_cursors WHERE brain_id=$1")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    let provider_requests: i64 =
        sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let receipt = json!({"fixture":"10k chain plus actual capture/audit/rebuild work","database":h.database,"brain":brain,"nodes":10000,"edges":9999,"read_concurrency":2,"requested":outcomes.len(),"failures":failures,"p95_seconds_all_outcomes":p95,"model_requests_controlled_loopback_only":provider_requests,"provider_fixture_calls":provider.calls.load(Ordering::SeqCst),"paid_provider_requests":0,"reads":outcomes,"capture_writes":captures,"autonomous_rounds":audits,"audit_states":audit_states,"audit_completions_during_reads":audit_completions_during_reads,"native_import_started_seconds":native_import_offsets,"native_import_windows":import_windows,"cursor_steps":cursor_steps,"rebuilds":rebuilds});
    std::fs::write(
        proof_dir.join("graph-10k-mixed-concurrency.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    provider_task.abort();
    proxy_task.abort();
    assert!(
        outcomes.len() >= 40,
        "Every requested read remains in the denominator"
    );
    assert_eq!(
        failures, 0,
        "Mixed-workload read failures retained in graph-10k-mixed-concurrency.json"
    );
    assert!(
        p95 < 2.0,
        "Mixed-workload p95 {p95:.3}s exceeds existing populated-read target"
    );
    assert!(
        receipt["reads"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["nodes"]
                .as_u64()
                .is_some_and(|count| (1..=250).contains(&count))),
        "The populated exact fixture must remain nonempty and bounded during background work"
    );
    assert!(!receipt["capture_writes"].as_array().unwrap().is_empty());
    assert!(
        receipt["capture_writes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["status"] == 200 && row["capture_processed"] == true)
    );
    assert!(
        audit_states
            .iter()
            .any(|(state, count)| state == "succeeded" && *count > 0),
        "Actual verifier jobs must complete; scheduling alone is insufficient"
    );
    assert!(
        cursor_steps > 0,
        "Actual autonomous scheduling must advance durable fair-walk progress"
    );
    assert_eq!(receipt["rebuilds"].as_array().unwrap().len(), 2);
    assert!(
        receipt["rebuilds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["status"] == 200
                && row["attempts"].as_array().is_some_and(|attempts| attempts
                    .last()
                    .is_some_and(|last| last["state"] == "ready"))),
        "Actual rebuild execution must eventually recover to ready"
    );
    assert!(
        audit_completions_during_reads > 0,
        "At least one actual verifier must finish during the read series"
    );
    assert!(
        import_windows
            .iter()
            .any(|window| window["overlapping_read_requests"].as_u64().unwrap() > 0),
        "Actual native import must overlap the read series; queueing alone is insufficient"
    );
    cleanup(&h, brain).await;
    h.finish().await;
}
