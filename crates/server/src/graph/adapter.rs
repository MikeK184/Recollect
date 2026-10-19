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
    // Generation-owned relationships are counted, verified and cleaned by their
    // `generation` property. Without an index each of those statements scans
    // every RECOLLECT_GRAPH_EDGE of every Brain (PROFILE: relationship type
    // scan). The composite index serves exact verification, which names the
    // generation and a batch of edge ids; a composite index is not used for the
    // `generation`-only counts and deletes, so both are needed.
    for (name, properties) in [
        ("recollect_graph_edge_generation", "r.generation"),
        ("recollect_graph_edge_generation_id", "r.generation, r.id"),
    ] {
        query(
            config,
            http,
            &format!("CREATE INDEX {name} IF NOT EXISTS FOR ()-[r:RECOLLECT_GRAPH_EDGE]-() ON ({properties})"),
            json!({}),
            &[],
        )
        .await?;
    }
    Ok(())
}
/// Native writes are fenced independently of a dropped Rust future. The job's
/// increasing durable attempt prevents an older delayed setup from reopening a
/// newer import; its exact token/deadline fence every subsequent batch.
pub(crate) struct ImportAttempt {
    pub number: i32,
    pub token: Uuid,
    pub deadline: chrono::DateTime<chrono::Utc>,
}
fn import_parameters(g: &GraphGeneration, attempt: &ImportAttempt) -> Value {
    json!({"brain":g.brain_id,"generation":g.id,"attempt":attempt.number,"token":attempt.token,"deadline":attempt.deadline})
}
pub(crate) async fn setup(
    state: &AppState,
    g: &GraphGeneration,
    attempt: &ImportAttempt,
) -> Result<()> {
    schema(&state.config, &state.http).await?;
    count(state,&format!("{GUARD} WITH guard WHERE NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:'generation:'+$generation}})}} AND datetime($deadline)>datetime.realtime()
      MERGE (g:RecollectGraphGeneration {{brain:$brain,id:$generation}})
      WITH g WHERE coalesce(g.import_attempt,0)<$attempt
      SET g.import_attempt=$attempt,g.import_token=$token,g.import_open=true RETURN count(g) AS count"),import_parameters(g,attempt),1).await?;
    // Never delete another generation or a shared entity. Reset in bounded
    // transactions, each rechecking the native attempt under the same guard.
    for members in [false, true] {
        for _ in 0..=501 {
            let body = if members {
                "MATCH (g)-[m:RECOLLECT_GRAPH_MEMBER {brain:$brain,generation:$generation}]->(n:RecollectGraphEntity {brain:$brain}) WITH m,n LIMIT 500 DELETE m WITH n FOREACH (x IN CASE WHEN NOT (n)--() THEN [n] ELSE [] END | DELETE x) RETURN count(n) AS count"
            } else {
                "MATCH ()-[r:RECOLLECT_GRAPH_EDGE {brain:$brain,generation:$generation}]->() WITH r LIMIT 500 DELETE r RETURN count(r) AS count"
            };
            let statement = format!(
                "{GUARD} MATCH (g:RecollectGraphGeneration {{brain:$brain,id:$generation,import_attempt:$attempt,import_token:$token}}) WHERE g.import_open AND datetime($deadline)>datetime.realtime() WITH g {body}"
            );
            let rows = query(
                &state.config,
                &state.http,
                &statement,
                import_parameters(g, attempt),
                &["count"],
            )
            .await?;
            match rows
                .first()
                .and_then(|row| row.first())
                .and_then(Value::as_u64)
            {
                Some(0) => break,
                Some(1..=500) => {}
                _ => {
                    return Err(failure(
                        "graph_response_invalid",
                        "Native reset did not return a bounded count.",
                    ));
                }
            }
        }
    }
    Ok(())
}
pub(crate) async fn nodes(
    state: &AppState,
    g: &GraphGeneration,
    attempt: &ImportAttempt,
    nodes: &[Entity],
) -> Result<()> {
    let keys: Vec<_> = nodes.iter().map(Entity::key).collect();
    let mut parameters = import_parameters(g, attempt);
    parameters["keys"] = json!(keys);
    count(state,&format!("{GUARD} MATCH (g:RecollectGraphGeneration {{brain:$brain,id:$generation,import_attempt:$attempt,import_token:$token}})
      WHERE g.import_open AND datetime($deadline)>datetime.realtime() AND NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:'generation:'+$generation}})}}
      UNWIND $keys AS key WITH g,key WHERE NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:key}})}}
      MERGE (n:RecollectGraphEntity {{brain:$brain,key:key}})
      MERGE (g)-[:RECOLLECT_GRAPH_MEMBER {{brain:$brain,generation:$generation}}]->(n) RETURN count(n) AS count"),parameters,keys.len()).await
}
pub(crate) async fn edges(
    state: &AppState,
    g: &GraphGeneration,
    attempt: &ImportAttempt,
    edges: &[GraphEdge],
) -> Result<()> {
    let mut parameters = import_parameters(g, attempt);
    parameters["edges"] = json!(edges);
    count(state,&format!("{GUARD} MATCH (g:RecollectGraphGeneration {{brain:$brain,id:$generation,import_attempt:$attempt,import_token:$token}})
      WHERE g.import_open AND datetime($deadline)>datetime.realtime() AND NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:'generation:'+$generation}})}}
      UNWIND $edges AS edge
      MATCH (g)-[:RECOLLECT_GRAPH_MEMBER]->(a:RecollectGraphEntity {{brain:$brain,key:edge.from}})
      MATCH (g)-[:RECOLLECT_GRAPH_MEMBER]->(b:RecollectGraphEntity {{brain:$brain,key:edge.to}})
      WHERE NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:edge.from}})}}
        AND NOT EXISTS {{MATCH (:RecollectGraphFence {{brain:$brain,key:edge.to}})}}
      MERGE (a)-[r:RECOLLECT_GRAPH_EDGE {{brain:$brain,generation:$generation,id:edge.id}}]->(b)
      SET r.family=edge.family,r.relation=edge.relation RETURN count(r) AS count"),parameters,edges.len()).await
}
pub(crate) async fn seal(
    state: &AppState,
    g: &GraphGeneration,
    attempt: &ImportAttempt,
) -> Result<()> {
    count(state,&format!("{GUARD} MATCH(g:RecollectGraphGeneration {{brain:$brain,id:$generation,import_attempt:$attempt,import_token:$token}})
      WHERE g.import_open AND datetime($deadline)>datetime.realtime() SET g.import_open=false RETURN count(g) AS count"),import_parameters(g,attempt),1).await
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
/// What PostgreSQL says a ready generation looks like natively: the lease token
/// of the import that sealed it and the membership/relationship counts verified
/// before readiness. A read compares this with Neo4j in one request; any
/// difference falls back to exact verification of the selected entities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Seal {
    pub id: Uuid,
    pub token: Option<Uuid>,
    pub nodes: i64,
    pub edges: i64,
}

/// One request per read counts every relationship of its generations by index
/// seek (about 0.5 microseconds each); very large combined reads split so each
/// statement stays far inside the three-second server bound.
const SEAL_EDGES: i64 = 1_000_000;
const SEAL: &str = "UNWIND $generations AS id OPTIONAL MATCH (g:RecollectGraphGeneration {brain:$brain,id:id}) \
    RETURN id, g.import_token AS token, g.import_open AS open, \
    CASE WHEN g IS NULL THEN -1 ELSE COUNT{(g)-[:RECOLLECT_GRAPH_MEMBER]->()} END AS members, \
    COUNT{()-[r:RECOLLECT_GRAPH_EDGE]->() WHERE r.generation=id AND r.brain=$brain} AS edges";

/// Pure comparison of the seal response with the canonical expectation. A
/// generation passes only when its sealing token matches, its import is closed
/// and both counts equal the stored ones. A missing generation (members -1), an
/// open import or a different token is a mismatch, not an error; only a response
/// that does not have the requested shape is invalid.
pub(crate) fn sealed(expected: &[Seal], rows: &[Vec<Value>]) -> Result<bool> {
    let invalid = || {
        failure(
            "graph_response_invalid",
            "The graph seal response was malformed.",
        )
    };
    let mut native = BTreeMap::new();
    for row in rows {
        let [id, token, open, members, edges] = row.as_slice() else {
            return Err(invalid());
        };
        let id = id.as_str().ok_or_else(invalid)?;
        let known = (
            token.as_str().map(str::to_owned),
            open.as_bool(),
            members.as_i64().ok_or_else(invalid)?,
            edges.as_i64().ok_or_else(invalid)?,
        );
        if !(token.is_null() || token.is_string()) || !(open.is_null() || open.is_boolean()) {
            return Err(invalid());
        }
        if native.insert(id, known).is_some() {
            return Err(invalid());
        }
    }
    if native.len() != expected.len() {
        return Err(invalid());
    }
    Ok(expected.iter().all(|e| {
        native
            .get(e.id.to_string().as_str())
            .is_some_and(|(token, open, members, edges)| {
                e.token
                    .is_some_and(|t| token.as_deref() == Some(t.to_string().as_str()))
                    && *open == Some(false)
                    && *members == e.nodes
                    && *edges == e.edges
            })
    }))
}

/// `Ok(false)` means "not provably intact": callers verify exactly or fail
/// closed. Transport and shape failures stay errors.
pub(crate) async fn seal_check(state: &AppState, brain: Uuid, expected: &[Seal]) -> Result<bool> {
    let mut groups: Vec<&[Seal]> = vec![];
    let (mut start, mut edges) = (0, 0);
    for (position, seal) in expected.iter().enumerate() {
        if position > start && edges + seal.edges > SEAL_EDGES {
            groups.push(&expected[start..position]);
            (start, edges) = (position, 0);
        }
        edges += seal.edges;
    }
    if start < expected.len() {
        groups.push(&expected[start..]);
    }
    for group in groups {
        let ids: Vec<_> = group.iter().map(|s| s.id).collect();
        let rows = query(
            &state.config,
            &state.http,
            SEAL,
            json!({"brain":brain,"generations":ids}),
            &["id", "token", "open", "members", "edges"],
        )
        .await?;
        if !sealed(group, &rows)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Exhaustive check of one ready generation against its complete frozen
/// descriptor, for periodic maintenance and never for a read. A sealed
/// generation must still carry its token with the stored counts; one published
/// before tokens were recorded (`token` None) has its counts compared with the
/// descriptor instead. `graph_import_mismatch` means damage; any other error is
/// an outage that proves nothing.
pub(crate) async fn verify_full(
    state: &AppState,
    g: &GraphGeneration,
    d: &Descriptor,
    token: Option<Uuid>,
) -> Result<()> {
    if token.is_some() {
        let seal = Seal {
            id: g.id,
            token,
            nodes: g.node_count,
            edges: g.edge_count,
        };
        if !seal_check(state, g.brain_id, std::slice::from_ref(&seal)).await? {
            return Err(failure(
                "graph_import_mismatch",
                "The imported graph did not match its canonical descriptors.",
            ));
        }
    } else {
        verify_generation(state, g).await?;
        verify_counts(state, g, d).await?;
    }
    for batch in d.nodes.chunks(500) {
        verify_nodes(state, g, batch).await?;
    }
    for batch in d.edges.chunks(500) {
        verify_edges(state, g, batch).await?;
    }
    Ok(())
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
    // COUNT{} reads the node's relationship degree instead of expanding it.
    count(state,"MATCH (g:RecollectGraphGeneration {brain:$brain,id:$generation}) RETURN COUNT{(g)-[:RECOLLECT_GRAPH_MEMBER]->()} AS count",json!({"brain":g.brain_id,"generation":g.id}),d.nodes.len()).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn seal(nodes: i64, edges: i64) -> Seal {
        Seal {
            id: Uuid::new_v4(),
            token: Some(Uuid::new_v4()),
            nodes,
            edges,
        }
    }
    /// The row a healthy sealed generation produces.
    fn healthy(s: &Seal) -> Vec<Value> {
        vec![
            json!(s.id),
            json!(s.token),
            json!(false),
            json!(s.nodes),
            json!(s.edges),
        ]
    }
    fn verdict(
        expected: &[Seal],
        rows: Vec<Vec<Value>>,
    ) -> std::result::Result<bool, &'static str> {
        sealed(expected, &rows).map_err(|e| e.1)
    }

    #[test]
    fn a_sealed_generation_with_its_stored_counts_passes_in_any_row_order() {
        let (a, b) = (seal(7, 6), seal(0, 0));
        assert_eq!(
            verdict(&[a.clone(), b.clone()], vec![healthy(&b), healthy(&a)]),
            Ok(true)
        );
    }

    #[test]
    fn any_difference_from_the_sealed_import_is_a_mismatch_not_an_error() {
        let s = seal(7, 6);
        let mut cases: Vec<(&str, Vec<Value>)> = vec![];
        let mut edit = |name, position: usize, value: Value| {
            let mut row = healthy(&s);
            row[position] = value;
            cases.push((name, row));
        };
        edit("another import's token", 1, json!(Uuid::new_v4()));
        edit("no token", 1, Value::Null);
        edit("import still open", 2, json!(true));
        edit("no open flag", 2, Value::Null);
        edit("a member erased", 3, json!(6));
        edit("an extra member", 3, json!(8));
        edit("a relationship deleted", 4, json!(5));
        edit("an extra relationship", 4, json!(7));
        edit("generation missing", 3, json!(-1));
        for (name, row) in cases {
            assert_eq!(
                verdict(std::slice::from_ref(&s), vec![row]),
                Ok(false),
                "{name}"
            );
        }
    }

    #[test]
    fn a_generation_without_a_recorded_token_can_never_be_sealed() {
        let unsealed = Seal {
            token: None,
            ..seal(1, 0)
        };
        let mut row = healthy(&unsealed);
        row[1] = json!(Uuid::new_v4());
        assert_eq!(verdict(&[unsealed], vec![row]), Ok(false));
    }

    #[test]
    fn one_unsealed_generation_fails_the_whole_read() {
        let (a, b) = (seal(3, 2), seal(4, 3));
        let mut damaged = healthy(&b);
        damaged[4] = json!(2);
        assert_eq!(
            verdict(&[a.clone(), b.clone()], vec![healthy(&a), damaged]),
            Ok(false)
        );
    }

    #[test]
    fn malformed_seal_responses_are_invalid_rather_than_trusted() {
        let s = seal(3, 2);
        let bad = |rows: Vec<Vec<Value>>| verdict(std::slice::from_ref(&s), rows);
        let invalid = Err("graph_response_invalid");
        assert_eq!(bad(vec![]), invalid, "a missing row");
        assert_eq!(
            bad(vec![healthy(&s), healthy(&s)]),
            invalid,
            "a repeated row"
        );
        assert_eq!(bad(vec![healthy(&s)[..4].to_vec()]), invalid, "a short row");
        let mut row = healthy(&s);
        row[3] = json!("3");
        assert_eq!(bad(vec![row]), invalid, "a count that is not a number");
        let mut row = healthy(&s);
        row[2] = json!("false");
        assert_eq!(
            bad(vec![row]),
            invalid,
            "an open flag that is not a boolean"
        );
        let mut row = healthy(&s);
        row[1] = json!(7);
        assert_eq!(bad(vec![row]), invalid, "a token that is not a string");
        let mut row = healthy(&s);
        row[0] = json!(Uuid::new_v4());
        assert_eq!(
            bad(vec![row]),
            Ok(false),
            "a row for a generation nobody asked for"
        );
    }
}
