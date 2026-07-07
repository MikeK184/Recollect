use super::*;
use axum::{Json, extract::State, routing::post};
use std::sync::{Arc, Mutex};
use tokio::{sync::Notify, task::JoinHandle};

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_combined_current_retention_covers_hidden_inputs_and_policy_extension() {
    let mut h = Harness::new().await;
    let owner = h.login().await;
    let (_, base) = base(&h, &owner).await;
    let (a,sa,fa)=combined::publish(&h,&owner,&base,"example.test/a/core",'a',json!([
        combined::directory(json!([{"kind":"declares","target":"destination","target_id":"destination"}])),
        {"id":"destination","kind":"symbol","name":"destination","file":"main.tf","line":1}
    ])).await;
    let (b, sb, _) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/b/core",
        'b',
        json!([combined::directory(json!([]))]),
    )
    .await;
    for snapshot in [&sa, &sb] {
        build(&h, &owner, &base, "repository", Some(snapshot)).await;
    }
    let env = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Retention inputs"}),
    )
    .await;
    let manifest=combined::manifest(&h,&owner,&base,&env["id"],"Required inputs",json!([{"repository_id":a,"snapshot_id":sa,"revision":"a".repeat(40),"config_paths":[]},{"repository_id":b,"snapshot_id":sb,"revision":"b".repeat(40),"config_paths":[]}])).await;
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["repository_days"] = json!(1);
    let retention = ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    let snapshot: Uuid = sb.as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE repository_snapshots SET created_at=clock_timestamp()-interval '1 day'+interval '4 seconds' WHERE id=$1").bind(snapshot).execute(&h.admin).await.unwrap();
    let generation = combined::combined_build(&h, &owner, &base, &manifest["id"]).await;
    policy["repository_days"] = json!(2);
    let retention = ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(4100)).await;
    let input = json!({"scope":{"kind":"combined","manifest_revision_id":manifest["id"],"selection":{"repository_ids":[a],"environment_id":env["id"]}},"start":combined::entity(&fa,"."),"end":combined::entity(&fa,"destination"),"direction":"outgoing","max_hops":1});
    let path = format!("{base}/graph/path");
    let extended = ok(&h, "POST", &path, &owner, input.clone()).await;
    assert_eq!(extended["status"], "path");
    assert_eq!(
        extended["view"]["generation"]["id"], generation["id"],
        "Current retained data does not require rebuilding an obsolete attempt deadline"
    );
    let exploration =
        json!({"scope":input["scope"],"center":input["start"],"direction":"outgoing","max_hops":1});
    let explored = ok(
        &h,
        "POST",
        &format!("{base}/graph/explore"),
        &owner,
        exploration.clone(),
    )
    .await;
    assert_eq!(explored["nodes"].as_array().unwrap().len(), 2);
    assert!(explored["expires_at"].is_string());
    let recall_input = json!({"exact":{"kind":"repository_fact","id":fa.iter().find(|f|f["record"]["name"]==".").unwrap()["id"]},
        "channels":["exact","graph"],"selection":input["scope"]["selection"],"manifest_revision_id":manifest["id"],
        "graph":{"kind":"combined","direction":"outgoing","max_hops":1},"context_bytes":32768});
    let recalled = ok(
        &h,
        "POST",
        &format!("{base}/recall"),
        &owner,
        recall_input.clone(),
    )
    .await;
    assert_eq!(
        recalled["graph"]["view"]["generation"]["id"],
        generation["id"]
    );
    assert_eq!(recalled["graph"]["candidates"], 1);
    sqlx::query("UPDATE repository_snapshots SET created_at=clock_timestamp()-interval '1 day'+interval '3 seconds' WHERE id=$1").bind(snapshot).execute(&h.admin).await.unwrap();
    policy["repository_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    let (p, proxied, server) = proxy(&h).await;
    h.router = app(proxied);
    for (path, input) in [
        (path, input),
        (format!("{base}/graph/explore"), exploration),
        (format!("{base}/recall"), recall_input),
    ] {
        sqlx::query("UPDATE repository_snapshots SET created_at=clock_timestamp()-interval '1 day'+interval '3 seconds' WHERE id=$1").bind(snapshot).execute(&h.admin).await.unwrap();
        p.arm(Rule {
            contains: "MATCH SHORTEST 1",
            pause: true,
            before: true,
            response: None,
        });
        let expire = async {
            p.entered().await;
            tokio::time::sleep(std::time::Duration::from_millis(3100)).await;
            p.release.notify_one();
        };
        let ((status, response, _), ()) =
            tokio::join!(h.call("POST", &path, Some(&owner), input), expire);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response["code"], "graph_retention_changed",
            "A required manifest input is still required when hidden by repository selection"
        );
    }
    server.abort();
    h.finish().await;
}

pub(crate) struct Rule {
    pub contains: &'static str,
    pub pause: bool,
    pub before: bool,
    pub response: Option<(StatusCode, Value)>,
}
pub(crate) struct Proxy {
    state: AppState,
    rule: Mutex<Option<Rule>>,
    entered: Notify,
    pub release: Notify,
    pub writes: Mutex<Vec<Value>>,
}
impl Proxy {
    pub(crate) fn arm(&self, rule: Rule) {
        *self.rule.lock().unwrap() = Some(rule);
    }
    pub(crate) async fn entered(&self) {
        tokio::time::timeout(std::time::Duration::from_secs(10), self.entered.notified())
            .await
            .expect("The controlled graph operation must reach its pause");
    }
}
async fn forward(
    State(p): State<Arc<Proxy>>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let statement = body["statement"].as_str().unwrap_or("");
    if statement.contains("MERGE") {
        p.writes.lock().unwrap().push(body.clone());
    }
    let rule = {
        let mut rule = p.rule.lock().unwrap();
        if rule
            .as_ref()
            .is_some_and(|r| statement.contains(r.contains))
        {
            rule.take()
        } else {
            None
        }
    };
    if rule.as_ref().is_some_and(|r| r.pause && r.before) {
        p.entered.notify_one();
        p.release.notified().await;
    }
    let response = if let Some(response) = rule
        .as_ref()
        .filter(|r| r.before)
        .and_then(|r| r.response.clone())
    {
        response
    } else {
        let response = p
            .state
            .http
            .post(format!("{}/db/neo4j/query/v2", p.state.config.neo4j_url))
            .basic_auth(
                &p.state.config.neo4j_username,
                Some(&p.state.config.neo4j_password),
            )
            .json(&body)
            .send()
            .await
            .unwrap();
        (response.status(), response.json().await.unwrap())
    };
    if rule.as_ref().is_some_and(|r| r.pause && !r.before) {
        p.entered.notify_one();
        p.release.notified().await;
    }
    let response = rule
        .filter(|r| !r.before)
        .and_then(|r| r.response)
        .unwrap_or(response);
    (response.0, Json(response.1))
}
pub(crate) async fn proxy(h: &Harness) -> (Arc<Proxy>, AppState, JoinHandle<()>) {
    let p = Arc::new(Proxy {
        state: h.state.clone(),
        rule: Mutex::new(None),
        entered: Notify::new(),
        release: Notify::new(),
        writes: Mutex::new(vec![]),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = (*h.state.config).clone();
    config.neo4j_url = format!("http://{}", listener.local_addr().unwrap());
    let mut state = h.state.clone();
    state.config = Arc::new(config);
    let router = Router::new()
        .route("/db/neo4j/query/v2", post(forward))
        .with_state(p.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (p, state, server)
}
async fn queued(h: &Harness, owner: &Login, base: &str, snapshot: &Value) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/graph/rebuild"),
        owner,
        json!({"kind":"repository","snapshot_id":snapshot}),
    )
    .await
}
async fn state(h: &Harness, generation: &Value) -> Value {
    let value: sqlx::types::Json<Value> =
        sqlx::query_scalar("SELECT to_jsonb(g) FROM graph_generations g WHERE id=$1")
            .bind(generation["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    value.0
}
async fn runnable(h: &Harness, generation: &Value) {
    sqlx::query("UPDATE jobs SET not_before=clock_timestamp()-interval '1 second' WHERE id=$1")
        .bind(
            generation["job_id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap(),
        )
        .execute(&h.admin)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_partial_import_retries_frozen_descriptors_and_fences_old_generation() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, facts) = materialized(&h, &owner, &base, 'a', None).await;
    let (p, proxied, server) = proxy(&h).await;
    p.arm(Rule {
        contains: "MERGE (n:RecollectGraphEntity",
        pause: true,
        before: false,
        response: Some((
            StatusCode::ACCEPTED,
            json!({"data":{"fields":[],"values":[]}}),
        )),
    });
    let generation = queued(&h, &owner, &base, &snapshot).await;
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    let work = tokio::spawn(async move { worker::execute(&proxied, &job).await });
    p.entered().await;
    let frozen = state(&h, &generation).await;
    assert_eq!(frozen["state"], "running");
    let scope =
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}});
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/graph/view"),
            Some(&owner),
            json!({"scope":scope})
        )
        .await
        .1["code"],
        "graph_projection_missing"
    );
    assert_eq!(cypher(&h,"MATCH (g:RecollectGraphGeneration {brain:$brain,id:$generation})-[:RECOLLECT_GRAPH_MEMBER]->(n) RETURN count(n)",json!({"brain":brain,"generation":generation["id"]})).await,json!([[7]]));
    p.release.notify_one();
    work.await.unwrap().unwrap();
    let failed = state(&h, &generation).await;
    assert_eq!(failed["state"], "queued");
    assert_eq!(failed["error_code"], "graph_response_invalid");
    runnable(&h, &generation).await;
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let recovered = state(&h, &generation).await;
    assert_eq!(recovered["state"], "ready");
    assert_eq!(recovered["descriptor"], frozen["descriptor"]);
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            path_input(&scope, &facts)
        )
        .await["edges"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let replacement = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let actor: Uuid = recovered["actor_id"].as_str().unwrap().parse().unwrap();
    graph::maintain_brain(&h.state, brain, actor)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let writes = p.writes.lock().unwrap().clone();
    for body in writes
        .iter()
        .filter(|body| !body["parameters"]["generation"].is_null())
    {
        assert_eq!(
            cypher(
                &h,
                body["statement"].as_str().unwrap(),
                body["parameters"].clone()
            )
            .await,
            json!([[0]]),
            "An old setup or node write must remain fenced"
        );
    }
    assert_eq!(
        cypher(
            &h,
            "MATCH (g:RecollectGraphGeneration {brain:$brain,id:$generation}) RETURN count(g)",
            json!({"brain":brain,"generation":generation["id"]})
        )
        .await,
        json!([[0]])
    );
    assert_eq!(
        cypher(
            &h,
            "MATCH (n:RecollectGraphEntity {brain:$brain}) RETURN count(n)",
            json!({"brain":brain})
        )
        .await,
        json!([[7]]),
        "Shared evidence still belongs to the replacement"
    );
    let retry = h
        .call(
            "POST",
            &format!(
                "{base}/jobs/{}/retry",
                generation["job_id"].as_str().unwrap()
            ),
            Some(&owner),
            Value::Null,
        )
        .await;
    assert_eq!(retry.0, StatusCode::CONFLICT);
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            path_input(&scope, &facts)
        )
        .await["view"]["generation"]["id"],
        replacement["id"]
    );
    server.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_backend_errors_remain_unpublished_and_preserve_retry_classification() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (_, snapshot, _) = materialized(&h, &owner, &base, 'b', None).await;
    let (p, proxied, server) = proxy(&h).await;
    for (response, error, state_name) in [
        (
            json!({"errors":[{"code":"Neo.TransientError.Transaction.DeadlockDetected","message":"PRIVATE_BACKEND_PAYLOAD"}]}),
            "graph_backend_transient",
            "queued",
        ),
        (
            json!({"errors":[{"code":"Neo.ClientError.Statement.SyntaxError","message":"PRIVATE_BACKEND_PAYLOAD"}]}),
            "graph_query_invalid",
            "failed",
        ),
        (
            json!({"data":{"fields":["wrong"],"values":[[1]]}}),
            "graph_response_invalid",
            "queued",
        ),
        (
            json!({"data":{"fields":["count"],"values":[[999]]}}),
            "graph_import_mismatch",
            "queued",
        ),
    ] {
        p.arm(Rule {
            contains: "MERGE (g:RecollectGraphGeneration",
            pause: false,
            before: false,
            response: Some((StatusCode::ACCEPTED, response)),
        });
        let generation = queued(&h, &owner, &base, &snapshot).await;
        assert!(worker::run_once(&proxied, "heavy").await.unwrap());
        let failed = state(&h, &generation).await;
        assert_eq!(failed["state"], state_name, "{failed}");
        assert_eq!(failed["error_code"], error);
        assert!(!failed.to_string().contains("PRIVATE_BACKEND_PAYLOAD"));
        if state_name == "queued" {
            ok(
                &h,
                "POST",
                &format!(
                    "{base}/jobs/{}/cancel",
                    generation["job_id"].as_str().unwrap()
                ),
                &owner,
                Value::Null,
            )
            .await;
        }
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM graph_generations WHERE brain_id=$1 AND state='ready'"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    server.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_lost_lease_cancellation_and_writer_revocation_cannot_publish() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, facts) = materialized(&h, &owner, &base, 'c', None).await;
    let original = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let (p, proxied, server) = proxy(&h).await;
    let selection =
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}});
    // An expired owner can finish an already-dispatched batch, but cannot
    // publish over the replacement lease's successful generation.
    p.arm(Rule {
        contains: "UNWIND $edges AS edge",
        pause: true,
        before: true,
        response: None,
    });
    let generation = queued(&h, &owner, &base, &snapshot).await;
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    let (old, state_copy) = (job.clone(), proxied.clone());
    let work = tokio::spawn(async move { worker::execute(&state_copy, &old).await });
    p.entered().await;
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            path_input(&selection, &facts)
        )
        .await["view"]["generation"]["id"],
        original["id"]
    );
    sqlx::query("UPDATE jobs SET lease_until=clock_timestamp()-interval '1 second' WHERE id=$1")
        .bind(job.id)
        .execute(&h.admin)
        .await
        .unwrap();
    let replacement = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replacement.id, job.id);
    assert_ne!(replacement.lease_token, job.lease_token);
    worker::execute(&h.state, &replacement).await.unwrap();
    p.release.notify_one();
    assert_eq!(work.await.unwrap(), Err(worker::Failure::LostLease));
    assert_eq!(state(&h, &generation).await["state"], "ready");
    let publishes:i64=sqlx::query_scalar("SELECT count(*) FROM mutation_audit WHERE brain_id=$1 AND target_id=$2 AND action='graph.publish'")
        .bind(brain).bind(job.target_id).fetch_one(&h.admin).await.unwrap();
    assert_eq!(publishes, 1);

    p.arm(Rule {
        contains: "UNWIND $edges AS edge",
        pause: true,
        before: true,
        response: None,
    });
    let cancelled = queued(&h, &owner, &base, &snapshot).await;
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    let state_copy = proxied.clone();
    let work = tokio::spawn(async move { worker::execute(&state_copy, &job).await });
    p.entered().await;
    ok(
        &h,
        "POST",
        &format!(
            "{base}/jobs/{}/cancel",
            cancelled["job_id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    p.release.notify_one();
    assert_eq!(work.await.unwrap(), Err(worker::Failure::LostLease));
    assert_eq!(state(&h, &cancelled).await["state"], "cancelled");
    ok(
        &h,
        "POST",
        &format!(
            "{base}/jobs/{}/retry",
            cancelled["job_id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    assert_eq!(state(&h, &cancelled).await["state"], "ready");

    let (writer_id, writer) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        json!({"role":"writer"}),
    )
    .await;
    p.arm(Rule {
        contains: "UNWIND $edges AS edge",
        pause: true,
        before: true,
        response: None,
    });
    let revoked = queued(&h, &writer, &base, &snapshot).await;
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    let work = tokio::spawn(async move { worker::execute(&proxied, &job).await });
    p.entered().await;
    ok(
        &h,
        "DELETE",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        Value::Null,
    )
    .await;
    p.release.notify_one();
    work.await.unwrap().unwrap();
    let revoked = state(&h, &revoked).await;
    assert_eq!(revoked["state"], "cancelled");
    assert_eq!(revoked["error_code"], "permission_revoked");
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            path_input(&selection, &facts)
        )
        .await["view"]["generation"]["id"],
        cancelled["id"]
    );
    server.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_erasure_blocks_late_import_and_replays_physical_cleanup_after_outage() {
    use recollect_server::privacy_journal;
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, facts) = materialized(&h, &owner, &base, 'd', None).await;
    let (p, proxied, server) = proxy(&h).await;
    p.arm(Rule {
        contains: "MERGE (n:RecollectGraphEntity",
        pause: true,
        before: true,
        response: None,
    });
    let generation = queued(&h, &owner, &base, &snapshot).await;
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    let work = tokio::spawn(async move { worker::execute(&proxied, &job).await });
    p.entered().await;
    let target = json!({"kind":"snapshot","id":snapshot});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    let erased = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let id: Uuid = erased["id"].as_str().unwrap().parse().unwrap();
    let mut offline = h.state.clone();
    let mut config = (*offline.config).clone();
    config.neo4j_url = "http://127.0.0.1:1".into();
    offline.config = Arc::new(config);
    privacy_journal::maintain(&offline).await.unwrap();
    let pending: (String, bool, Option<String>) =
        sqlx::query_as("SELECT state,graph_pending,error_code FROM privacy_requests WHERE id=$1")
            .bind(id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        pending,
        ("error".into(), true, Some("graph_unavailable".into()))
    );
    let selection =
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}});
    let removed = h
        .call(
            "POST",
            &format!("{base}/graph/path"),
            Some(&owner),
            path_input(&selection, &facts),
        )
        .await;
    assert_eq!(removed.0, StatusCode::GONE);
    privacy_journal::maintain(&h.state).await.unwrap();
    let completed: (String, bool) =
        sqlx::query_as("SELECT state,graph_pending FROM privacy_requests WHERE id=$1")
            .bind(id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(completed, ("complete".into(), false));
    p.release.notify_one();
    work.await.unwrap().unwrap();
    assert_ne!(state(&h, &generation).await["state"], "ready");
    assert_eq!(
        cypher(
            &h,
            "MATCH (n:RecollectGraphEntity {brain:$brain}) RETURN count(n)",
            json!({"brain":brain})
        )
        .await,
        json!([[0]])
    );
    let status = ok(&h, "GET", &format!("{base}/graph"), &owner, Value::Null).await;
    assert_eq!(status["generations"][0]["state"], "removed");
    // Simulate restoring this test's own old Neo4j identities. Journal startup
    // replay removes them again even though PostgreSQL says erasure complete.
    let key = format!("repository_fact:{}", facts[0].as_str().unwrap());
    cypher(
        &h,
        "MERGE (n:RecollectGraphEntity {brain:$brain,key:$key}) RETURN count(n)",
        json!({"brain":brain,"key":key}),
    )
    .await;
    privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    assert_eq!(
        cypher(
            &h,
            "MATCH (n:RecollectGraphEntity {brain:$brain}) RETURN count(n)",
            json!({"brain":brain})
        )
        .await,
        json!([[0]])
    );
    let actor: Uuid = generation["actor_id"].as_str().unwrap().parse().unwrap();
    graph::maintain_brain(&h.state, brain, actor)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(state(&h, &generation).await["state"], "removed");
    server.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_publication_crossing_retention_deadline_rolls_back_and_renews_without_deadlock() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (_, snapshot, _) = materialized(&h, &owner, &base, 'e', None).await;
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["repository_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    let generation = queued(&h, &owner, &base, &snapshot).await;
    sqlx::query("CREATE FUNCTION graph_publication_pause() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.state='ready' THEN PERFORM pg_sleep(6.2); END IF; RETURN NEW; END $$").execute(&h.admin).await.unwrap();
    sqlx::query("CREATE TRIGGER graph_publication_pause BEFORE UPDATE OF state ON graph_generations FOR EACH ROW EXECUTE FUNCTION graph_publication_pause()").execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE repository_snapshots SET created_at=clock_timestamp()-interval '1 day'+interval '3 seconds' WHERE id=$1").bind(snapshot.as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(15),
        worker::run_once(&h.state, "heavy"),
    )
    .await
    .expect("Publication must remain polled during renewal")
    .unwrap();
    let failed = state(&h, &generation).await;
    assert_eq!(failed["state"], "cancelled");
    assert_eq!(failed["error_code"], "graph_input_unavailable");
    assert!(failed["published_at"].is_null());
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM mutation_audit WHERE brain_id=$1 AND action='graph.publish'"
        )
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_http_timeout_cannot_publish_a_delayed_import() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (_, snapshot, _) = materialized(&h, &owner, &base, 'a', None).await;
    let (p, proxied, server) = proxy(&h).await;
    p.arm(Rule {
        contains: "MERGE (n:RecollectGraphEntity",
        pause: true,
        before: true,
        response: None,
    });
    let generation = queued(&h, &owner, &base, &snapshot).await;
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    let work = tokio::spawn(async move { worker::execute(&proxied, &job).await });
    p.entered().await;
    tokio::time::timeout(std::time::Duration::from_secs(8), work)
        .await
        .expect("The five-second HTTP bound must finish this attempt")
        .unwrap()
        .unwrap();
    let failed = state(&h, &generation).await;
    assert_eq!(failed["state"], "queued");
    assert_eq!(failed["error_code"], "graph_unavailable");
    assert!(failed["published_at"].is_null());
    p.release.notify_one();
    runnable(&h, &generation).await;
    worker::run_once(&h.state, "heavy").await.unwrap();
    assert_eq!(state(&h, &generation).await["state"], "ready");
    server.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_path_checks_expiry_after_query_and_maintenance_rebuilds_without_epoch_edit() {
    let mut h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let source=ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Expiring graph evidence","content":"Synthetic retained support.\n","media_type":"text/plain","retain_content":true})).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(
            &source["version"]["id"],
            "Independent assertion",
            "retained",
        ),
    )
    .await;
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["document_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    sqlx::query("UPDATE source_versions SET created_at=clock_timestamp()-interval '1 day'+interval '3 seconds' WHERE id=$1")
        .bind(source["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
    let generation = build(&h, &owner, &base, "knowledge", None).await;
    let (p, proxied, server) = proxy(&h).await;
    h.router = app(proxied);
    p.arm(Rule {
        contains: "MATCH SHORTEST 1",
        pause: true,
        before: true,
        response: None,
    });
    let input = json!({"scope":{"kind":"knowledge"},"start":format!("claim:{}",claim["id"].as_str().unwrap()),"end":format!("source_version:{}",source["version"]["id"].as_str().unwrap()),"direction":"outgoing","max_hops":2});
    let path = format!("{base}/graph/path");
    let request = h.call("POST", &path, Some(&owner), input);
    let expire = async {
        p.entered().await;
        tokio::time::sleep(std::time::Duration::from_millis(3100)).await;
        p.release.notify_one();
    };
    let ((status, response, _), ()) = tokio::join!(request, expire);
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response["code"], "graph_retention_changed");
    let actor: Uuid = generation["actor_id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        graph::maintain_brain(&h.state, brain, actor)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        1
    );
    worker::run_once(&h.state, "heavy").await.unwrap();
    let current = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"}}),
    )
    .await;
    assert_ne!(current["generation"]["id"], generation["id"]);
    assert_eq!(
        current["generation"]["input_epoch"],
        generation["input_epoch"]
    );
    assert_eq!(current["total_nodes"], 1);
    assert_eq!(current["total_edges"], 0);
    assert_eq!(current["generation"]["unresolved"], 1);
    assert_eq!(
        graph::maintain_brain(&h.state, brain, actor)
            .await
            .unwrap_or_else(|e| panic!("{}", e.1)),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM model_requests WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    server.abort();
    cleanup(&h, brain).await;
    h.finish().await;
}
