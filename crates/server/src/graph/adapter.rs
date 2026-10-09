use super::*;
use crate::{config::Config, privacy::Manifest};
use serde_json::Value;
use std::time::Duration;

pub(super) const GUARD: &str = "MERGE (guard:RecollectGraphBrain {brain:$brain}) SET guard.serial=coalesce(guard.serial,0)+1 WITH guard ";

pub(super) struct QueryOptions {
    pub seconds: u64,
    pub metadata: Value,
}

pub(crate) async fn query(
    config: &Config,
    http: &reqwest::Client,
    statement: &str,
    parameters: Value,
    fields: &[&str],
) -> Result<Vec<Vec<Value>>> {
    query_with(
        config,
        http,
        statement,
        parameters,
        fields,
        QueryOptions {
            seconds: 3,
            metadata: json!({}),
        },
    )
    .await
}
pub(super) async fn query_with(
    config: &Config,
    http: &reqwest::Client,
    statement: &str,
    parameters: Value,
    fields: &[&str],
    options: QueryOptions,
) -> Result<Vec<Vec<Value>>> {
    let mut response = http
        .post(format!(
            "{}/db/neo4j/query/v2",
            config.neo4j_url.trim_end_matches('/')
        ))
        .basic_auth(&config.neo4j_username, Some(&config.neo4j_password))
        .timeout(Duration::from_secs(options.seconds + 2))
        .json(&json!({"statement":statement,"parameters":parameters,"maxExecutionTime":options.seconds,"txMetadata":options.metadata}))
        .send()
        .await
        .map_err(|_| {
            failure(
                "graph_unavailable",
                "The graph service did not complete this request.",
            )
        })?;
    if !response.status().is_success() {
        return Err(failure(
            match response.status().as_u16() {
                401 | 403 => "graph_configuration_invalid",
                400..=499 if !matches!(response.status().as_u16(), 408 | 429) => {
                    "graph_query_invalid"
                }
                _ => "graph_unavailable",
            },
            "The graph service rejected this request.",
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        failure(
            "graph_response_incomplete",
            "The graph response was interrupted.",
        )
    })? {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err(failure(
                "graph_response_too_large",
                "The graph response exceeded its bounded envelope.",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let body: Value = serde_json::from_slice(&bytes).map_err(|_| {
        failure(
            "graph_response_invalid",
            "The graph response could not be decoded.",
        )
    })?;
    if let Some(errors) = body.get("errors") {
        let errors = errors.as_array().ok_or_else(|| {
            failure(
                "graph_response_invalid",
                "The graph error envelope was invalid.",
            )
        })?;
        if !errors.is_empty() {
            let code = if errors.iter().any(|e| {
                e["code"]
                    .as_str()
                    .is_some_and(|c| c.starts_with("Neo.ClientError.Security."))
            }) {
                "graph_configuration_invalid"
            } else if errors.iter().any(|e| {
                e["code"]
                    .as_str()
                    .is_some_and(|c| c.starts_with("Neo.ClientError."))
            }) {
                "graph_query_invalid"
            } else if errors.iter().all(|e| {
                e["code"].as_str().is_some_and(|c| {
                    c.starts_with("Neo.TransientError.")
                        || c == "Neo.DatabaseError.General.DatabaseUnavailable"
                })
            }) {
                "graph_backend_transient"
            } else {
                "graph_query_failed"
            };
            return Err(failure(
                code,
                "The graph query did not succeed. Its generation remains unpublished.",
            ));
        }
    }
    if body.pointer("/data/fields") != Some(&json!(fields)) {
        return Err(failure(
            "graph_response_invalid",
            "The graph response fields were incomplete.",
        ));
    }
    let rows: Vec<Vec<Value>> =
        serde_json::from_value(body.pointer("/data/values").cloned().unwrap_or(Value::Null))
            .map_err(|_| {
                failure(
                    "graph_response_invalid",
                    "The graph response rows were incomplete.",
                )
            })?;
    if rows.iter().any(|row| row.len() != fields.len()) {
        return Err(failure(
            "graph_response_invalid",
            "The graph returned a malformed row.",
        ));
    }
    Ok(rows)
}
async fn count(
    state: &AppState,
    statement: &str,
    parameters: Value,
    expected: usize,
) -> Result<()> {
    let rows = query(
        &state.config,
        &state.http,
        statement,
        parameters,
        &["count"],
    )
    .await?;
    if rows != vec![vec![json!(expected)]] {
        return Err(failure(
            "graph_import_mismatch",
            "The imported graph did not match its canonical descriptors.",
        ));
    }
    Ok(())
}
pub(super) async fn schema(config: &Config, http: &reqwest::Client) -> Result<()> {
    for (name, label, properties) in [
        ("recollect_graph_brain", "RecollectGraphBrain", "n.brain"),
        (
            "recollect_graph_entity",
            "RecollectGraphEntity",
            "(n.brain,n.key)",
        ),
        (
            "recollect_graph_generation",
            "RecollectGraphGeneration",
            "(n.brain,n.id)",
        ),
        (
            "recollect_graph_fence",
            "RecollectGraphFence",
            "(n.brain,n.key)",
        ),
    ] {
        query(config,http,&format!("CREATE CONSTRAINT {name} IF NOT EXISTS FOR (n:{label}) REQUIRE {properties} IS UNIQUE"),json!({}),&[]).await?;
    }
    Ok(())
}
pub(crate) async fn setup(state: &AppState, g: &GraphGeneration) -> Result<()> {
    schema(&state.config, &state.http).await?;
    count(state,&format!("{GUARD} WITH guard WHERE NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:'generation:'+$generation}})}} MERGE (g:RecollectGraphGeneration {{brain:$brain,id:$generation}}) RETURN count(g) AS count"),json!({"brain":g.brain_id,"generation":g.id}),1).await
}
pub(crate) async fn nodes(state: &AppState, g: &GraphGeneration, nodes: &[Entity]) -> Result<()> {
    let keys: Vec<_> = nodes.iter().map(Entity::key).collect();
    count(state,&format!("{GUARD} MATCH (g:RecollectGraphGeneration {{brain:$brain,id:$generation}})
      WHERE NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:'generation:'+$generation}})}}
      UNWIND $keys AS key WITH g,key WHERE NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:key}})}}
      MERGE (n:RecollectGraphEntity {{brain:$brain,key:key}})
      MERGE (g)-[:RECOLLECT_GRAPH_MEMBER {{brain:$brain,generation:$generation}}]->(n) RETURN count(n) AS count"),
      json!({"brain":g.brain_id,"generation":g.id,"keys":keys}),keys.len()).await
}
pub(crate) async fn edges(
    state: &AppState,
    g: &GraphGeneration,
    edges: &[GraphEdge],
) -> Result<()> {
    count(state,&format!("{GUARD} MATCH (g:RecollectGraphGeneration {{brain:$brain,id:$generation}})
      WHERE NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:'generation:'+$generation}})}}
      UNWIND $edges AS edge
      MATCH (g)-[:RECOLLECT_GRAPH_MEMBER]->(a:RecollectGraphEntity {{brain:$brain,key:edge.from}})
      MATCH (g)-[:RECOLLECT_GRAPH_MEMBER]->(b:RecollectGraphEntity {{brain:$brain,key:edge.to}})
      WHERE NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:edge.from}})}}
        AND NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:edge.to}})}}
      MERGE (a)-[r:RECOLLECT_GRAPH_EDGE {{brain:$brain,generation:$generation,id:edge.id}}]->(b)
      SET r.family=edge.family,r.relation=edge.relation RETURN count(r) AS count"),
      json!({"brain":g.brain_id,"generation":g.id,"edges":edges}),edges.len()).await
}
pub(crate) async fn verify_nodes(
    state: &AppState,
    g: &GraphGeneration,
    nodes: &[Entity],
) -> Result<()> {
    let keys: Vec<_> = nodes.iter().map(Entity::key).collect();
    count(state,"MATCH (:RecollectGraphGeneration {brain:$brain,id:$generation})-[r:RECOLLECT_GRAPH_MEMBER {brain:$brain,generation:$generation}]->(n:RecollectGraphEntity {brain:$brain}) WHERE n.key IN $keys RETURN count(DISTINCT n) AS count",json!({"brain":g.brain_id,"generation":g.id,"keys":keys}),keys.len()).await
}

pub(super) async fn neighbor_keys(
    state: &AppState,
    projections: &[(GraphGeneration, Descriptor)],
    relations: &[String],
    center: &str,
    direction: &str,
    limit: usize,
) -> Result<BTreeSet<String>> {
    let generations: Vec<_> = projections.iter().map(|(g, _)| g.id).collect();
    let edge_ids: Vec<_> = projections
        .iter()
        .flat_map(|(_, d)| &d.edges)
        .filter(|e| relations.is_empty() || relations.contains(&e.relation))
        .map(|e| e.id)
        .collect();
    let pattern = match direction {
        "outgoing" => "(a)-[r:RECOLLECT_GRAPH_EDGE]->(b)",
        "incoming" => "(a)<-[r:RECOLLECT_GRAPH_EDGE]-(b)",
        _ => "(a)-[r:RECOLLECT_GRAPH_EDGE]-(b)",
    };
    let rows = query(&state.config, &state.http, &format!(
        "MATCH (a:RecollectGraphEntity {{brain:$brain,key:$center}}) MATCH {pattern} \
         WHERE b.brain=$brain AND r.brain=$brain AND r.generation IN $generations AND r.id IN $edges \
         RETURN DISTINCT b.key AS key ORDER BY key LIMIT $limit"),
        json!({"brain": projections[0].0.brain_id,"center":center,"generations":generations,"edges":edge_ids,"limit":limit}), &["key"]).await?;
    rows.into_iter()
        .map(|row| {
            row[0].as_str().map(str::to_owned).ok_or_else(|| {
                failure(
                    "graph_response_invalid",
                    "Native neighbor identities are invalid.",
                )
            })
        })
        .collect()
}
pub(crate) async fn verify_generation(state: &AppState, g: &GraphGeneration) -> Result<()> {
    count(
        state,
        "MATCH (g:RecollectGraphGeneration {brain:$brain,id:$generation}) RETURN count(g) AS count",
        json!({"brain":g.brain_id,"generation":g.id}),
        1,
    )
    .await
}
pub(crate) async fn verify_edges(
    state: &AppState,
    g: &GraphGeneration,
    edges: &[GraphEdge],
) -> Result<()> {
    let ids: Vec<_> = edges.iter().map(|e| e.id).collect();
    let rows=query(&state.config,&state.http,"MATCH (a:RecollectGraphEntity {brain:$brain})-[r:RECOLLECT_GRAPH_EDGE {brain:$brain,generation:$generation}]->(b:RecollectGraphEntity {brain:$brain}) WHERE r.id IN $ids RETURN r.id AS id,a.key AS from,b.key AS to,r.family AS family,r.relation AS relation ORDER BY id",json!({"brain":g.brain_id,"generation":g.id,"ids":ids}),&["id","from","to","family","relation"]).await?;
    let mut expected: Vec<_> = edges
        .iter()
        .map(|e| {
            vec![
                json!(e.id),
                json!(e.from),
                json!(e.to),
                json!(e.family),
                json!(e.relation),
            ]
        })
        .collect();
    expected.sort_by_key(|r| r[0].as_str().unwrap().to_owned());
    if rows != expected {
        return Err(failure(
            "graph_import_mismatch",
            "The imported relationships differ from their canonical descriptors.",
        ));
    }
    Ok(())
}
pub(crate) async fn verify_counts(
    state: &AppState,
    g: &GraphGeneration,
    d: &Descriptor,
) -> Result<()> {
    count(state,"MATCH (:RecollectGraphGeneration {brain:$brain,id:$generation})-[r:RECOLLECT_GRAPH_MEMBER]->() RETURN count(r) AS count",json!({"brain":g.brain_id,"generation":g.id}),d.nodes.len()).await?;
    count(state,"MATCH ()-[r:RECOLLECT_GRAPH_EDGE {brain:$brain,generation:$generation}]->() RETURN count(r) AS count",json!({"brain":g.brain_id,"generation":g.id}),d.edges.len()).await
}

/// Only journaled canonical closures call this function. Its identity fences
/// serialize with imports so a delayed worker cannot resurrect a removed key.
pub async fn erase(config: &Config, brain: Uuid, m: &Manifest) -> Result<()> {
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| failure("graph_unavailable", "Graph cleanup is unavailable."))?;
    let keys: Vec<String> = [
        ("source_version", &m.source_versions),
        ("claim", &m.claim_revisions),
        ("repository_fact", &m.facts),
        ("manifest_revision", &m.manifest_revisions),
    ]
    .into_iter()
    .flat_map(|(kind, ids)| ids.iter().map(move |id| key(kind, *id)))
    .collect();
    if !keys.is_empty() {
        schema(config, &http).await?;
    }
    for batch in keys.chunks(500) {
        let rows = query(
            config,
            &http,
            &format!(
                "{GUARD} UNWIND $keys AS key
          MERGE (:RecollectGraphFence {{brain:$brain,key:key}})
          WITH key OPTIONAL MATCH (n:RecollectGraphEntity {{brain:$brain,key:key}}) DETACH DELETE n
          RETURN count(key) AS count"
            ),
            json!({"brain":brain,"keys":batch}),
            &["count"],
        )
        .await?;
        if rows != vec![vec![json!(batch.len())]] {
            return Err(failure(
                "graph_import_mismatch",
                "Graph cleanup could not validate its identity closure.",
            ));
        }
    }
    Ok(())
}

pub(crate) async fn cleanup(state: &AppState, brain: Uuid, generation: Uuid) -> Result<bool> {
    // Only this exact generation's relationships and now-unreferenced entities
    // can be removed. Other graphs and reference checkouts are not targets.
    schema(&state.config, &state.http).await?;
    count(state,&format!("{GUARD} MERGE (f:RecollectGraphFence {{brain:$brain,key:'generation:'+$generation}}) RETURN count(f) AS count"),json!({"brain":brain,"generation":generation}),1).await?;
    let mut edges_done = false;
    for _ in 0..10 {
        let statement = if !edges_done {
            format!(
                "{GUARD} MATCH ()-[r:RECOLLECT_GRAPH_EDGE {{brain:$brain,generation:$generation}}]->() WITH r LIMIT 500 DELETE r RETURN count(r) AS count"
            )
        } else {
            format!("{GUARD} MATCH (g:RecollectGraphGeneration {{brain:$brain,id:$generation}})-[m:RECOLLECT_GRAPH_MEMBER]->(n:RecollectGraphEntity {{brain:$brain}})
          WITH m,n LIMIT 500 DELETE m WITH n
          FOREACH (x IN CASE WHEN NOT (n)--() THEN [n] ELSE [] END | DELETE x)
          RETURN count(n) AS count")
        };
        let rows = query(
            &state.config,
            &state.http,
            &statement,
            json!({"brain":brain,"generation":generation}),
            &["count"],
        )
        .await?;
        if rows.len() != 1 {
            return Err(failure(
                "graph_response_invalid",
                "Graph cleanup returned an invalid count.",
            ));
        }
        match rows.first().and_then(|r| r.first()).and_then(Value::as_u64) {
            Some(0) if edges_done => {
                query(&state.config,&state.http,&format!("{GUARD} MATCH (g:RecollectGraphGeneration {{brain:$brain,id:$generation}}) DELETE g RETURN count(g) AS count"),json!({"brain":brain,"generation":generation}),&["count"]).await?;
                return Ok(true);
            }
            Some(0) => edges_done = true,
            Some(1..=500) => {}
            _ => {
                return Err(failure(
                    "graph_response_invalid",
                    "Graph cleanup returned an invalid count.",
                ));
            }
        }
    }
    Ok(false)
}

pub(crate) async fn path(
    state: &AppState,
    g: &GraphGeneration,
    generations: &[Uuid],
    request: &GraphPathRequest,
    nodes: &[String],
    edges: &[GraphEdge],
) -> Result<Option<(Vec<String>, Vec<Uuid>)>> {
    let pattern = match request.direction.as_str() {
        "outgoing" => format!(
            "(start)-[:RECOLLECT_GRAPH_EDGE]->{{1,{}}}(finish)",
            request.max_hops
        ),
        "incoming" => format!(
            "(start)<-[:RECOLLECT_GRAPH_EDGE]-{{1,{}}}(finish)",
            request.max_hops
        ),
        "both" => format!(
            "(start)-[:RECOLLECT_GRAPH_EDGE]-{{1,{}}}(finish)",
            request.max_hops
        ),
        _ => {
            return Err(Error::invalid(
                "Choose outgoing, incoming or both directions.",
            ));
        }
    };
    let statement=format!("MATCH (start:RecollectGraphEntity {{brain:$brain,key:$start}}),(finish:RecollectGraphEntity {{brain:$brain,key:$end}})
      MATCH SHORTEST 1 (p = {pattern}
        WHERE all(n IN nodes(p) WHERE n:RecollectGraphEntity AND n.brain=$brain AND n.key IN $nodes)
          AND all(r IN relationships(p) WHERE r.brain=$brain AND r.generation IN $generations AND r.id IN $edges))
      RETURN [n IN nodes(p) | n.key] AS nodes,[r IN relationships(p) | r.id] AS edges");
    let rows=query(&state.config,&state.http,&statement,json!({"brain":g.brain_id,"generations":generations,"start":request.start,"end":request.end,"nodes":nodes,"edges":edges.iter().map(|e|e.id).collect::<Vec<_>>()}),&["nodes","edges"]).await?;
    if rows.is_empty() {
        return Ok(None);
    }
    if rows.len() != 1 {
        return Err(failure(
            "graph_response_invalid",
            "The path response was ambiguous.",
        ));
    }
    let nodes: Vec<String> = serde_json::from_value(rows[0][0].clone()).map_err(|_| {
        failure(
            "graph_response_invalid",
            "The path nodes could not be read.",
        )
    })?;
    let edges: Vec<Uuid> = serde_json::from_value(rows[0][1].clone()).map_err(|_| {
        failure(
            "graph_response_invalid",
            "The path edges could not be read.",
        )
    })?;
    Ok(Some((nodes, edges)))
}

pub(super) async fn reach(
    state: &AppState,
    selected: &super::read::Selected,
    request: &GraphExploreRequest,
    limit: usize,
) -> Result<Vec<GraphReach>> {
    // Direction and hop count were validated before constructing this syntax;
    // all entity IDs and canonical membership sets remain parameters.
    let pattern = match request.direction.as_str() {
        "outgoing" => format!(
            "(start)-[:RECOLLECT_GRAPH_EDGE]->{{1,{}}}(finish)",
            request.max_hops
        ),
        "incoming" => format!(
            "(start)<-[:RECOLLECT_GRAPH_EDGE]-{{1,{}}}(finish)",
            request.max_hops
        ),
        _ => format!(
            "(start)-[:RECOLLECT_GRAPH_EDGE]-{{1,{}}}(finish)",
            request.max_hops
        ),
    };
    let statement = format!("MATCH (start:RecollectGraphEntity {{brain:$brain,key:$start}}),(finish:RecollectGraphEntity {{brain:$brain}})
        WHERE finish.key IN $nodes AND finish.key<>$start
        MATCH SHORTEST 1 (p = {pattern}
          WHERE all(n IN nodes(p) WHERE n:RecollectGraphEntity AND n.brain=$brain AND n.key IN $nodes)
            AND all(r IN relationships(p) WHERE r.brain=$brain AND r.generation IN $generations AND r.id IN $edges))
        RETURN finish.key AS key,[n IN nodes(p) | n.key] AS nodes,[r IN relationships(p) | r.id] AS edges
        ORDER BY size(edges),key LIMIT $limit");
    let rows = query(
        &state.config,
        &state.http,
        &statement,
        json!({
            "brain": selected.view.brain_id, "start": request.center,
            "limit": limit,
            "generations": selected.projections.iter().map(|(g,_)| g.id).collect::<Vec<_>>(),
            "nodes": selected.nodes.keys().collect::<Vec<_>>(),
            "edges": selected.edges.iter().map(|e| e.id).collect::<Vec<_>>()
        }),
        &["key", "nodes", "edges"],
    )
    .await?;
    rows.into_iter()
        .map(|row| {
            serde_json::from_value(json!({"key": row[0], "nodes": row[1], "edges": row[2]}))
                .map_err(|_| {
                    failure(
                        "graph_response_invalid",
                        "Native reachability could not be read.",
                    )
                })
        })
        .collect()
}
