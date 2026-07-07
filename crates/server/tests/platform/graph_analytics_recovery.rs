use super::super::recovery::{Rule, proxy};
use super::*;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_cancellation_terminates_running_native_work_through_owned_cleanup() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope) = setup(&h, &owner).await;
    let (p, proxied, server) = proxy(&h).await;
    let r = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    p.arm(Rule {
        contains: "gds.pageRank.stream($graph",
        pause: true,
        before: true,
        response: None,
    });
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    let work = tokio::spawn(async move { worker::execute(&proxied, &job).await });
    p.entered().await;
    let (attempt,installation):(Uuid,Uuid)=sqlx::query_as("SELECT a.id,i.id FROM analytics_attempts a CROSS JOIN privacy_installation i WHERE report_id=$1")
        .bind(r["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    let graph = format!(
        "recollect_analytics_{}_{}",
        installation.simple(),
        attempt.simple()
    );
    // Keep the real attempt ownership and exact scratch name, but replace its
    // tiny projection with a bounded 10k-node/50k-edge native stress fixture.
    // The fixed product recipe is unchanged; cleanup is the behavior under test.
    cypher(
        &h,
        "CALL gds.graph.drop($graph) YIELD graphName RETURN graphName",
        json!({"graph":graph}),
    )
    .await;
    let edges: Vec<_> = (0..10000)
        .flat_map(|i| (0..5).map(move |j| [i, (i * 37 + 13 + j * 59) % 10000]))
        .collect();
    assert_eq!(cypher(&h,"UNWIND $rows AS row WITH gds.graph.project($graph,row[0],row[1],{},{readConcurrency:1}) AS g RETURN g.nodeCount,g.relationshipCount",json!({"graph":graph,"rows":edges})).await,json!([[10000,50000]]));
    let native = tokio::spawn({
        let state = h.state.clone();
        async move {
            state.http.post(format!("{}/db/neo4j/query/v2",state.config.neo4j_url))
                .basic_auth(&state.config.neo4j_username,Some(&state.config.neo4j_password))
                .timeout(std::time::Duration::from_secs(32))
                .json(&json!({"statement":"CALL gds.pageRank.stream($graph,{concurrency:1,maxIterations:1000000,tolerance:0.0,jobId:$job}) YIELD nodeId,score RETURN count(*) AS nodes",
                    "parameters":{"graph":graph,"job":attempt},"maxExecutionTime":30,
                    "txMetadata":{"recollect_installation":installation,"recollect_analytics_attempt":attempt}}))
                .send().await.unwrap().json::<Value>().await.unwrap()
        }
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let progress = cypher(
                &h,
                "CALL gds.listProgress() YIELD jobId WHERE jobId=$job RETURN jobId",
                json!({"job":attempt}),
            )
            .await;
            if !progress.as_array().unwrap().is_empty() {
                break;
            }
            assert!(
                !native.is_finished(),
                "Native GDS must still be computing before cancellation"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("The fixture must observe running native GDS work");
    ok(
        &h,
        "POST",
        &format!("{base}/jobs/{}/cancel", r["job_id"].as_str().unwrap()),
        &owner,
        json!({}),
    )
    .await;
    // Native termination is acknowledged before the engine has necessarily
    // retired its transaction. Exercise the documented eventual cleanup retry.
    let cleanup_started = std::time::Instant::now();
    let mut cleanup_retries = 0;
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            match recollect_server::graph::analytics::reconcile(&h.state.config, &h.state.pool)
                .await
            {
                Ok(_) => break,
                Err(_) => cleanup_retries += 1,
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    })
    .await
    .expect("Owned native cleanup must eventually confirm absence");
    eprintln!(
        "Native cleanup confirmed after {:?}, {cleanup_retries} retries",
        cleanup_started.elapsed()
    );
    let response = native.await.unwrap();
    assert!(
        response["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e.to_string().to_lowercase().contains("terminat")),
        "Product cleanup must terminate the observed native transaction"
    );
    assert_eq!(cypher(&h,"SHOW TRANSACTIONS YIELD transactionId,metaData WHERE metaData.recollect_installation=$installation AND metaData.recollect_analytics_attempt=$attempt RETURN transactionId",json!({"installation":installation,"attempt":attempt})).await,json!([]));
    absent(&h, brain).await;
    p.release.notify_one();
    let _ = work.await.unwrap();
    let report = read(&h, &owner, &base, &r["id"]).await;
    assert_eq!(report["job"]["state"], "cancelled");
    assert_eq!(report["report"]["state"], "cancelled");
    assert_eq!(report["rows"], json!([]));
    assert_eq!(report["cleanup_pending"], false);
    server.abort();
    h.finish().await;
}

pub(super) async fn setup(h: &Harness, owner: &Login) -> (Uuid, String, Value) {
    let (brain, base) = base(h, owner).await;
    let (repo, snapshot, _) = combined::publish(
        h,
        owner,
        &base,
        "example.test/analytics/recovery",
        'a',
        fixture(&[&[1], &[2], &[0], &[]]),
    )
    .await;
    build(h, owner, &base, "repository", Some(&snapshot)).await;
    (
        brain,
        base,
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]},"relations":["calls"]}),
    )
}
async fn absent(h: &Harness, brain: Uuid) {
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM analytics_attempts WHERE brain_id=$1")
        .bind(brain)
        .fetch_all(&h.admin)
        .await
        .unwrap();
    let installation: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let names: Vec<_> = ids
        .iter()
        .map(|id| {
            format!(
                "recollect_analytics_{}_{}",
                installation.simple(),
                id.simple()
            )
        })
        .collect();
    assert_eq!(
        cypher(
            h,
            "CALL gds.graph.list() YIELD graphName WHERE graphName IN $names RETURN graphName",
            json!({"names":names})
        )
        .await,
        json!([])
    );
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_native_estimate_and_malformed_result_refuse_publication() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope) = setup(&h, &owner).await;
    let (p, state, server) = proxy(&h).await;
    for (contains, response, code) in [
        (
            "gds.graph.project.estimate",
            json!({"data":{"fields":["bytesMax"],"values":[[67108865]]}}),
            "analytics_memory_limit",
        ),
        (
            "gds.pageRank.stream($graph",
            json!({"data":{"fields":["nodeId","value"],"values":[[0,1.0],[0,1.0],[2,1.0],[3,1.0]]}}),
            "analytics_result_invalid",
        ),
    ] {
        let r = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
        p.arm(Rule {
            contains,
            pause: false,
            before: contains.contains("estimate"),
            response: Some((StatusCode::ACCEPTED, response)),
        });
        assert!(worker::run_once(&state, "heavy").await.unwrap());
        let result = read(&h, &owner, &base, &r["id"]).await;
        assert_eq!(result["report"]["state"], "failed", "{result}");
        assert_eq!(result["report"]["error_code"], code);
        assert_eq!(result["rows"], json!([]));
        absent(&h, brain).await;
    }
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_cancel_and_erasure_fence_a_delayed_native_creation() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope) = setup(&h, &owner).await;
    let (p, state, server) = proxy(&h).await;
    for erase in [false, true] {
        let r = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
        p.arm(Rule {
            contains: "WITH gds.graph.project($graph",
            pause: true,
            before: true,
            response: None,
        });
        let cancel = async {
            p.entered().await;
            let payload = p
                .writes
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|v| {
                    v["statement"]
                        .as_str()
                        .is_some_and(|s| s.contains("WITH gds.graph.project($graph"))
                })
                .unwrap()
                .clone();
            if erase {
                let target = json!({"kind":"snapshot","id":scope["snapshot_id"]});
                let preview = ok(
                    &h,
                    "POST",
                    &format!("{base}/erasures/preview"),
                    &owner,
                    target.clone(),
                )
                .await;
                ok(
                    &h,
                    "POST",
                    &format!("{base}/erasures"),
                    &owner,
                    json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
                )
                .await;
                recollect_server::privacy_journal::maintain(&h.state)
                    .await
                    .unwrap();
            } else {
                ok(
                    &h,
                    "POST",
                    &format!("{base}/jobs/{}/cancel", r["job_id"].as_str().unwrap()),
                    &owner,
                    json!({}),
                )
                .await;
                recollect_server::graph::analytics::reconcile(&h.state.config, &h.state.pool)
                    .await
                    .unwrap();
            }
            p.release.notify_one();
            payload
        };
        let (worker_result, payload) = tokio::join!(worker::run_once(&state, "heavy"), cancel);
        assert!(worker_result.unwrap());
        let result = read(&h, &owner, &base, &r["id"]).await;
        assert_ne!(result["report"]["state"], "ready");
        assert_eq!(result["rows"], json!([]));
        absent(&h, brain).await;
        // An exact delayed retry after successful cleanup remains fenced.
        let replay = cypher(
            &h,
            payload["statement"].as_str().unwrap(),
            payload["parameters"].clone(),
        )
        .await;
        assert_eq!(replay, json!([[null, 0, 0]]));
        absent(&h, brain).await;
    }
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_interrupted_attempt_and_late_reply_cannot_publish_after_lease_replacement()
{
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope) = setup(&h, &owner).await;
    for interrupt in [true, false] {
        let (p, proxied, server) = proxy(&h).await;
        let r = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
        let rid: Uuid = r["id"].as_str().unwrap().parse().unwrap();
        p.arm(Rule {
            contains: "gds.pageRank.stream($graph",
            pause: true,
            before: false,
            response: None,
        });
        let original = worker::claim(&h.state.pool, "heavy")
            .await
            .unwrap()
            .unwrap();
        let jid = original.id;
        let lease = original.lease_token;
        let work = tokio::spawn(async move { worker::execute(&proxied, &original).await });
        p.entered().await;
        let (attempt,installation):(Uuid,Uuid)=sqlx::query_as("SELECT a.id,i.id FROM analytics_attempts a CROSS JOIN privacy_installation i WHERE report_id=$1")
            .bind(rid).fetch_one(&h.admin).await.unwrap();
        let graph = format!(
            "recollect_analytics_{}_{}",
            installation.simple(),
            attempt.simple()
        );
        assert_eq!(
            cypher(
                &h,
                "CALL gds.graph.list() YIELD graphName WHERE graphName=$graph RETURN graphName",
                json!({"graph":graph})
            )
            .await,
            json!([[graph]])
        );
        let staged: bool = sqlx::query_scalar(
            "SELECT state='running' AND scores IS NULL FROM analytics_reports WHERE id=$1",
        )
        .bind(rid)
        .fetch_one(&h.admin)
        .await
        .unwrap();
        assert!(
            staged,
            "A completed native calculation is not yet a published report"
        );
        if interrupt {
            // Abrupt task interruption deliberately skips the normal cleanup
            // path; recovery must rely on durable attempt ownership.
            work.abort();
        }
        sqlx::query(
            "UPDATE jobs SET lease_until=clock_timestamp()-interval '1 second' WHERE id=$1",
        )
        .bind(jid)
        .execute(&h.admin)
        .await
        .unwrap();
        let replacement = worker::claim(&h.state.pool, "heavy")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(replacement.id, jid);
        assert_ne!(replacement.lease_token, lease);
        worker::execute(&h.state, &replacement).await.unwrap();
        let ready = read(&h, &owner, &base, &r["id"]).await;
        assert_eq!(ready["report"]["state"], "ready");
        assert_eq!(ready["rows"].as_array().unwrap().len(), 4);
        let ordered:bool=sqlx::query_scalar("SELECT old.cleaned_at<=new.created_at FROM analytics_attempts old JOIN analytics_attempts new ON new.report_id=old.report_id AND new.id<>old.id WHERE old.id=$1")
            .bind(attempt).fetch_one(&h.admin).await.unwrap();
        assert!(
            ordered,
            "Old scratch is confirmed absent before replacement allocation"
        );
        p.release.notify_one();
        let old = work.await;
        if interrupt {
            assert!(old.unwrap_err().is_cancelled());
        } else {
            assert_eq!(old.unwrap(), Err(worker::Failure::LostLease));
        }
        let published:i64=sqlx::query_scalar("SELECT count(*) FROM mutation_audit WHERE brain_id=$1 AND action='graph.analysis.publish' AND target_id=$2")
            .bind(brain).bind(rid).fetch_one(&h.admin).await.unwrap();
        assert_eq!(
            published, 1,
            "A late native result must never republish under its replaced lease"
        );
        assert_eq!(
            read(&h, &owner, &base, &r["id"]).await["rows"],
            ready["rows"]
        );
        absent(&h, brain).await;
        server.abort();
    }
    h.finish().await;
}
