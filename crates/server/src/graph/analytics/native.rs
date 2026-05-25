use super::*;
use crate::config::Config;
use journal::Entry;
use std::time::Duration;

fn invalid() -> Error {
    failure(
        "analytics_result_invalid",
        "The graph engine returned an incomplete or invalid analytical result.",
    )
}
pub(super) async fn control(
    config: &Config,
    http: &reqwest::Client,
    q: &str,
    p: Value,
    f: &[&str],
) -> Result<Vec<Vec<Value>>> {
    adapter::query(config, http, q, p, f).await
}
pub(super) async fn terminate(config: &Config, http: &reqwest::Client, e: &Entry) -> Result<bool> {
    let rows=control(config,http,"SHOW TRANSACTIONS YIELD transactionId,metaData WHERE metaData.recollect_installation=$installation AND metaData.recollect_analytics_attempt=$attempt RETURN transactionId",json!({"installation":e.installation_id,"attempt":e.id}),&["transactionId"]).await?;
    if rows.is_empty() {
        return Ok(true);
    }
    if rows.len() > 10
        || rows
            .iter()
            .any(|r| r[0].as_str().is_none_or(|s| s.len() > 100))
    {
        return Err(invalid());
    }
    let ids: Vec<_> = rows.into_iter().map(|r| r[0].clone()).collect();
    control(
        config,
        http,
        "TERMINATE TRANSACTIONS $ids YIELD transactionId,message RETURN transactionId",
        json!({"ids":ids}),
        &["transactionId"],
    )
    .await?;
    Ok(false)
}
pub(super) async fn cleanup(config: &Config, http: &reqwest::Client, e: &Entry) -> Result<()> {
    // Free a transaction holding the projection guard, then close creation
    // under that same guard. Repeat termination after fencing delayed starts.
    terminate(config, http, e).await?;
    adapter::schema(config, http).await?;
    let rows = control(
        config,
        http,
        &format!(
            "{} MERGE (f:RecollectGraphFence {{brain:$brain,key:$key}}) RETURN count(f) AS count",
            adapter::GUARD
        ),
        json!({"brain":e.brain_id,"key":e.fence()}),
        &["count"],
    )
    .await?;
    if rows != vec![vec![json!(1)]] {
        return Err(invalid());
    }
    let mut stopped = false;
    for _ in 0..10 {
        if terminate(config, http, e).await? {
            stopped = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    if !stopped {
        return Err(failure(
            "analytics_cleanup_pending",
            "The analytical transaction is still stopping. Cleanup will retry.",
        ));
    }
    let graph = e.graph();
    let dropped = control(
        config,
        http,
        "CALL gds.graph.drop($graph,false) YIELD graphName RETURN graphName",
        json!({"graph":graph}),
        &["graphName"],
    )
    .await?;
    if !dropped.is_empty() && dropped != vec![vec![json!(graph)]] {
        return Err(invalid());
    }
    let rows = control(
        config,
        http,
        "CALL gds.graph.list() YIELD graphName WHERE graphName=$graph RETURN graphName",
        json!({"graph":graph}),
        &["graphName"],
    )
    .await?;
    if !rows.is_empty() {
        return Err(failure(
            "analytics_cleanup_pending",
            "The analytical projection is still present. Cleanup will retry.",
        ));
    }
    Ok(())
}

pub(super) struct Output {
    pub scores: Vec<Score>,
    pub version: String,
    pub estimate: i64,
    pub projected_edges: usize,
}
async fn estimate(state: &AppState, q: &str, p: Value) -> Result<u64> {
    let rows = control(&state.config, &state.http, q, p, &["bytesMax"]).await?;
    let bytes = rows
        .first()
        .filter(|_| rows.len() == 1)
        .and_then(|r| r[0].as_u64())
        .ok_or_else(invalid)?;
    if bytes > MEMORY {
        return Err(Error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "analytics_memory_limit",
            "The native memory estimate exceeds the 64 MiB analysis budget. Select a smaller graph.",
        ));
    }
    Ok(bytes)
}
pub(super) async fn calculate(
    state: &AppState,
    r: &AnalyticsReport,
    d: &Frozen,
    e: &Entry,
) -> Result<Output> {
    adapter::schema(&state.config, &state.http).await?;
    let version = control(
        &state.config,
        &state.http,
        "RETURN gds.version() AS version",
        json!({}),
        &["version"],
    )
    .await?;
    let version = version
        .first()
        .filter(|_| version.len() == 1)
        .and_then(|r| r[0].as_str())
        .filter(|s| !s.is_empty() && s.len() <= 64)
        .ok_or_else(invalid)?
        .to_owned();
    let projected_edges = if r.direction == "both" {
        d.edges.len() * 2
    } else {
        d.edges.len()
    };
    let projection_bytes=estimate(state,"CALL gds.graph.project.estimate('*','*',{nodeCount:$nodes,relationshipCount:$edges}) YIELD bytesMax RETURN bytesMax",json!({"nodes":d.nodes.len(),"edges":projected_edges})).await?;
    let ids: BTreeMap<_, _> = d
        .nodes
        .iter()
        .enumerate()
        .map(|(i, k)| (k.as_str(), i))
        .collect();
    let mut rows: Vec<Value> = d
        .nodes
        .iter()
        .enumerate()
        .map(|(i, _)| json!([i, null]))
        .collect();
    for edge in &d.edges {
        let from = *ids.get(edge.from.as_str()).ok_or_else(invalid)?;
        let to = *ids.get(edge.to.as_str()).ok_or_else(invalid)?;
        rows.push(if r.direction == "incoming" {
            json!([to, from])
        } else {
            json!([from, to])
        });
    }
    let graph = e.graph();
    let statement=format!("CYPHER 25 {} CALL (*) {{
      WHEN datetime.realtime()<datetime($deadline) AND NOT EXISTS {{ MATCH (:RecollectGraphFence {{brain:$brain,key:$fence}}) }} THEN {{
       UNWIND $rows AS row WITH gds.graph.project($graph,row[0],row[1],{{}},$config) AS g
       RETURN g.graphName AS graph,g.nodeCount AS nodes,g.relationshipCount AS edges
      }} ELSE {{ RETURN null AS graph,0 AS nodes,0 AS edges }}
     }} RETURN graph,nodes,edges",adapter::GUARD);
    let result=adapter::query_with(&state.config,&state.http,&statement,json!({"brain":e.brain_id,"fence":e.fence(),"deadline":e.deadline,"graph":graph,"rows":rows,"config":{"readConcurrency":1,"undirectedRelationshipTypes":if r.direction=="both"{vec!["*"]}else{vec![]}}}),&["graph","nodes","edges"],adapter::QueryOptions{seconds:10,metadata:e.metadata()}).await?;
    if result
        != vec![vec![
            json!(graph),
            json!(d.nodes.len()),
            json!(projected_edges),
        ]]
    {
        return Err(failure(
            "analytics_projection_closed",
            "This analytical attempt was closed or its input projection did not match.",
        ));
    }
    let procedure = match r.algorithm.as_str() {
        "pagerank" => "gds.pageRank.stream",
        "leiden" => "gds.leiden.stream",
        "wcc" => "gds.wcc.stream",
        _ => return Err(invalid()),
    };
    let mut config = r.parameters.clone();
    config["jobId"] = json!(e.id);
    let algo_bytes = estimate(
        state,
        &format!("CALL {procedure}.estimate($graph,$config) YIELD bytesMax RETURN bytesMax"),
        json!({"graph":graph,"config":config}),
    )
    .await?;
    let bytes = projection_bytes
        .checked_add(algo_bytes)
        .filter(|&b| b <= MEMORY)
        .ok_or(Error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "analytics_memory_limit",
            "The combined native memory estimate exceeds the 64 MiB analysis budget.",
        ))?;
    let value = match r.algorithm.as_str() {
        "pagerank" => "score",
        "leiden" => "communityId",
        _ => "componentId",
    };
    let output=adapter::query_with(&state.config,&state.http,&format!("CALL {procedure}($graph,$config) YIELD nodeId,{value} RETURN nodeId,{value} AS value ORDER BY nodeId"),json!({"graph":graph,"config":config}),&["nodeId","value"],adapter::QueryOptions{seconds:90,metadata:e.metadata()}).await?;
    if output.len() != d.nodes.len() {
        return Err(invalid());
    }
    let mut seen = BTreeSet::new();
    let mut scores = Vec::with_capacity(output.len());
    for row in output {
        let index = row[0]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|&i| i < d.nodes.len())
            .ok_or_else(invalid)?;
        if !seen.insert(index) {
            return Err(invalid());
        }
        let (score, group) = if r.algorithm == "pagerank" {
            (
                Some(
                    row[1]
                        .as_f64()
                        .filter(|v| v.is_finite() && *v >= 0.)
                        .ok_or_else(invalid)?,
                ),
                None,
            )
        } else {
            (
                None,
                Some(row[1].as_i64().filter(|v| *v >= 0).ok_or_else(invalid)?),
            )
        };
        scores.push(Score {
            key: d.nodes[index].clone(),
            score,
            group,
        });
    }
    scores.sort_by(|a, b| match (a.score, b.score) {
        (Some(a_score), Some(b_score)) => b_score.total_cmp(&a_score).then(a.key.cmp(&b.key)),
        _ => a.group.cmp(&b.group).then(a.key.cmp(&b.key)),
    });
    Ok(Output {
        scores,
        version,
        estimate: bytes as i64,
        projected_edges,
    })
}
