use super::*;
#[path = "graph_analytics_journal.rs"]
mod journal;
#[path = "graph_analytics_reads.rs"]
mod reads;
#[path = "graph_analytics_recovery.rs"]
mod recovery_tests;
#[path = "graph_analytics_scope.rs"]
mod scope;

fn fixture(edges: &[&[usize]]) -> Value {
    json!(edges.iter().enumerate().map(|(i,edges)|json!({"id":format!("n{i}"),"kind":"function","name":format!("node_{i}"),"file":"main.rs","line":1,"relations":edges.iter().map(|target|json!({"kind":"calls","target_id":format!("n{target}"),"target":format!("n{target}")})).collect::<Vec<_>>()})).collect::<Vec<_>>())
}
async fn queue(
    h: &Harness,
    owner: &Login,
    base: &str,
    scope: &Value,
    algorithm: &str,
    direction: &str,
) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/graph/analytics"),
        owner,
        json!({"scope":scope,"algorithm":algorithm,"direction":direction}),
    )
    .await
}
async fn read(h: &Harness, owner: &Login, base: &str, id: &Value) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/graph/analytics/{}/view", id.as_str().unwrap()),
        owner,
        json!({"offset":0}),
    )
    .await
}
async fn completed(h: &Harness, owner: &Login, base: &str, r: &Value) -> Value {
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let result = read(h, owner, base, &r["id"]).await;
    assert_eq!(result["report"]["state"], "ready", "{result}");
    assert_eq!(result["cleanup_pending"], false);
    result
}
fn score(result: &Value, label: &str) -> f64 {
    result["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["node"]["evidence"]["label"] == label)
        .unwrap()["score"]
        .as_f64()
        .unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_native_recipes_direction_parallel_edges_and_isolates() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, _) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/analytics/cycle",
        'c',
        fixture(&[&[1], &[2], &[0], &[]]),
    )
    .await;
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let scope = json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]},"relations":["calls"]});
    let rank = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    assert!(
        read(&h, &owner, &base, &rank["id"]).await["rows"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let rank = completed(&h, &owner, &base, &rank).await;
    assert_eq!(rank["report"]["gds_version"], "2026.08.1");
    let expected = 1.0_f64 - 0.85_f64.powi(50);
    for name in ["node_0", "node_1", "node_2"] {
        assert!((score(&rank, name) - expected).abs() < 1e-9);
    }
    assert!((score(&rank, "node_3") - 0.15).abs() < 1e-9);
    assert_eq!(rank["report"]["node_count"], 4);
    assert_eq!(rank["report"]["projected_edge_count"], 3);
    for algorithm in ["leiden", "wcc"] {
        let r = queue(&h, &owner, &base, &scope, algorithm, "both").await;
        let r = completed(&h, &owner, &base, &r).await;
        let group = |name: &str| {
            r["rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["node"]["evidence"]["label"] == name)
                .unwrap()["group"]
                .clone()
        };
        assert_eq!(group("node_0"), group("node_1"));
        assert_eq!(group("node_1"), group("node_2"));
        assert_ne!(group("node_0"), group("node_3"));
        assert_eq!(r["report"]["projected_edge_count"], 6);
    }
    let (repo, snapshot, _) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/analytics/parallel",
        'd',
        fixture(&[&[1, 1, 0], &[], &[]]),
    )
    .await;
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let scope = json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]},"relations":["calls"]});
    let outgoing = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    let outgoing = completed(&h, &owner, &base, &outgoing).await;
    let incoming = queue(&h, &owner, &base, &scope, "pagerank", "incoming").await;
    let incoming = completed(&h, &owner, &base, &incoming).await;
    assert!(score(&outgoing, "node_1") > score(&outgoing, "node_0"));
    assert!(score(&incoming, "node_0") > score(&incoming, "node_1"));
    let both = queue(&h, &owner, &base, &scope, "wcc", "both").await;
    let both = completed(&h, &owner, &base, &both).await;
    assert_eq!(both["report"]["edge_count"], 3);
    assert_eq!(both["report"]["projected_edge_count"], 6);
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    let installation: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(cypher(&h,"CALL gds.graph.list() YIELD graphName WHERE graphName STARTS WITH $prefix RETURN graphName",json!({"prefix":format!("recollect_analytics_{}_",installation.simple())})).await,json!([]));
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_correction_invalidates_all_scores_and_erasure_scrubs_hidden_contributors()
{
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, facts) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/analytics/corrections",
        'e',
        fixture(&[&[1], &[2], &[0], &[1]]),
    )
    .await;
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let scope = json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]},"relations":["calls"]});
    let mut proposal = proposal(&facts[1]["id"], "Hub assertion", "unsupported hub");
    proposal["content"]["selection"] = scope["selection"].clone();
    proposal["content"]["supports"][0]["kind"] = json!("repository_fact");
    proposal["content"]["supports"][0]["line_from"] = Value::Null;
    proposal["content"]["supports"][0]["line_to"] = Value::Null;
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, proposal).await;
    let r = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    let before = completed(&h, &owner, &base, &r).await;
    let pending = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":claim["id"],"action":"reject","reason":"Synthetic rejected hub must not affect survivors."})).await;
    let stale = read(&h, &owner, &base, &r["id"]).await;
    assert_eq!(stale["report"]["state"], "stale");
    assert_eq!(stale["rows"], json!([]));
    assert_eq!(
        read(&h, &owner, &base, &pending["id"]).await["job"]["state"],
        "cancelled"
    );
    let saved: bool =
        sqlx::query_scalar("SELECT scores IS NULL FROM analytics_reports WHERE id=$1")
            .bind(r["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(saved);
    let fresh = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    let fresh = completed(&h, &owner, &base, &fresh).await;
    assert_eq!(fresh["total"], 3);
    assert!(score(&before, "node_0") > score(&fresh, "node_0"));
    assert!(
        fresh["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["node"]["evidence"]["label"] != "node_1")
    );
    assert_eq!(cypher(&h,"MATCH (n:RecollectGraphEntity {brain:$brain,key:$key}) RETURN count(n)",json!({"brain":brain,"key":format!("repository_fact:{}",facts[1]["id"].as_str().unwrap())})).await,json!([[1]]),"The canonical exclusion must precede physical graph cleanup");
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        json!({"kind":"snapshot","id":snapshot}),
    )
    .await;
    let erased = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":{"kind":"snapshot","id":snapshot},"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let removed = read(&h, &owner, &base, &fresh["report"]["id"]).await;
    assert_eq!(removed["report"]["state"], "removed");
    assert_eq!(removed["rows"], json!([]));
    assert!(removed["report"]["selection"].is_null());
    recollect_server::privacy_journal::run_once(&h.state)
        .await
        .unwrap();
    let done = ok(&h, "GET", &format!("{base}/erasures"), &owner, Value::Null).await;
    assert!(done.to_string().contains(erased["id"].as_str().unwrap()));
    h.finish().await;
}
