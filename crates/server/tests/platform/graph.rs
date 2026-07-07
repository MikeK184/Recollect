use super::review::{ok, proposal};
use super::*;
use recollect_server::{graph, worker};

#[path = "graph_analytics.rs"]
mod analytics;
#[path = "graph_combined.rs"]
mod combined;
#[path = "graph_exploration.rs"]
mod exploration;
#[path = "graph_knowledge.rs"]
mod knowledge;
#[path = "graph_recovery.rs"]
pub(crate) mod recovery;
#[path = "graph_retrieval.rs"]
mod retrieval;
#[path = "graph_scope.rs"]
mod scope;

pub(super) async fn cypher(h: &Harness, statement: &str, parameters: Value) -> Value {
    let response = h
        .state
        .http
        .post(format!("{}/db/neo4j/query/v2", h.state.config.neo4j_url))
        .basic_auth(
            &h.state.config.neo4j_username,
            Some(&h.state.config.neo4j_password),
        )
        .json(&json!({"statement":statement,"parameters":parameters,"maxExecutionTime":3}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let response: Value = response.json().await.unwrap();
    assert!(
        response["errors"]
            .as_array()
            .is_none_or(|errors| errors.is_empty()),
        "Synthetic graph query failed: {}",
        response["errors"]
    );
    response["data"]["values"].clone()
}
pub(super) async fn cleanup(h: &Harness, brain: Uuid) {
    cypher(h,"MATCH (n) WHERE n.brain=$brain AND any(label IN labels(n) WHERE label IN ['RecollectGraphEntity','RecollectGraphGeneration','RecollectGraphBrain','RecollectGraphFence']) DETACH DELETE n",json!({"brain":brain})).await;
}
async fn base(h: &Harness, owner: &Login) -> (Uuid, String) {
    let brain = ok(
        h,
        "POST",
        "/api/brains",
        owner,
        json!({"name":"Synthetic graph proof"}),
    )
    .await;
    let id = brain["id"].as_str().unwrap().parse().unwrap();
    (id, format!("/api/brains/{id}"))
}
async fn materialized(
    h: &Harness,
    owner: &Login,
    base: &str,
    revision: char,
    existing: Option<Value>,
) -> (Value, Value, Vec<Value>) {
    let (_, token) = h.pair_device(owner, "Synthetic graph publication").await;
    let repo = if let Some(repo) = existing {
        repo
    } else {
        let (status,response)=h.bearer("POST",&format!("{base}/workspace/checkouts"),&token,json!({"workspace_root":"/fixture","complete":false,"notes":[],"checkouts":[{"local_path":"/fixture/repo","origin":"https://example.test/team/fixture.git","head":null,"branch":"main","dirty":false,"status":"available"}]})).await;
        assert_eq!(status, StatusCode::OK, "{response}");
        h.bearer("GET", &format!("{base}/workspace"), &token, Value::Null)
            .await
            .1["repositories"][0]["id"]
            .clone()
    };
    let operation = publication::begin(h, base, &token, &repo).await;
    let mut input = publication::input(&repo, &operation, &revision.to_string().repeat(40));
    input["files"][0]["content"] = Value::Null;
    input["settings"] = json!({"retained_files":[]});
    input["facts"] = json!([
        {"id":"a","kind":"function","name":"entry","file":"src/lib.rs","line":1,"relations":[{"kind":"calls","target_id":"b"},{"kind":"calls","target_id":"c"},{"kind":"calls","target_id":"duplicate"},{"kind":"calls","target":"missing"},{"kind":"future_relation","target_id":"d"}]},
        {"id":"b","kind":"function","name":"short bridge","file":"src/lib.rs","line":1,"relations":[{"kind":"calls","target_id":"d"}]},
        {"id":"c","kind":"function","name":"long bridge","file":"src/lib.rs","line":1,"relations":[{"kind":"calls","target_id":"e"},{"kind":"calls","target_id":"a"}]},
        {"id":"e","kind":"function","name":"long continuation","file":"src/lib.rs","line":1,"relations":[{"kind":"calls","target_id":"d"}]},
        {"id":"d","kind":"function","name":"destination","file":"src/lib.rs","line":1},
        {"id":"duplicate","kind":"function","name":"duplicate first","file":"src/lib.rs","line":1},
        {"id":"duplicate","kind":"function","name":"duplicate second","file":"src/lib.rs","line":2}
    ]);
    input["receipt"]["fact_count"] = json!(7);
    for fact in input["facts"].as_array_mut().unwrap() {
        if let Some(relations) = fact.get_mut("relations").and_then(Value::as_array_mut) {
            for relation in relations {
                if relation.get("target").is_none() {
                    relation["target"] = relation["target_id"].clone();
                }
            }
        }
    }
    let (status, response) = h
        .bearer(
            "POST",
            &format!("{base}/repositories/{}/snapshots", repo.as_str().unwrap()),
            &token,
            input,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{response}");
    let snapshot = response["snapshot"]["id"].clone();
    assert!(snapshot.is_string(), "{response}");
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let records = ok(
        h,
        "GET",
        &format!(
            "{base}/repository-snapshots/{}/facts",
            snapshot.as_str().unwrap()
        ),
        owner,
        Value::Null,
    )
    .await;
    let facts = records["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].clone())
        .collect();
    (repo, snapshot, facts)
}
pub(super) async fn build(
    h: &Harness,
    owner: &Login,
    base: &str,
    kind: &str,
    snapshot: Option<&Value>,
) -> Value {
    let g = ok(
        h,
        "POST",
        &format!("{base}/graph/rebuild"),
        owner,
        json!({"kind":kind,"snapshot_id":snapshot}),
    )
    .await;
    // Autonomous discovery can have queued other exact inputs first. Run the
    // bounded lane until this requested generation completes, preserving FIFO.
    let mut status = Value::Null;
    for _ in 0..20 {
        if !worker::run_once(&h.state, "heavy").await.unwrap() {
            break;
        }
        status = ok(h, "GET", &format!("{base}/graph"), owner, Value::Null).await;
        if status["generations"].as_array().unwrap().iter().any(|row| {
            row["id"] == g["id"] && row["state"] != "queued" && row["state"] != "running"
        }) {
            break;
        }
    }
    let current = status["generations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == g["id"])
        .unwrap();
    assert_eq!(current["state"], "ready", "{status}");
    current.clone()
}
fn path_input(scope: &Value, facts: &[Value]) -> Value {
    json!({"scope":scope,"start":format!("repository_fact:{}",facts[0].as_str().unwrap()),"end":format!("repository_fact:{}",facts[4].as_str().unwrap()),"direction":"outgoing","max_hops":6})
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_structural_publication_prefilter_rejection_and_rebuild() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, facts) = materialized(&h, &owner, &base, 'a', None).await;
    let scope =
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}});
    let missing = h
        .call(
            "POST",
            &format!("{base}/graph/view"),
            Some(&owner),
            json!({"scope":scope}),
        )
        .await;
    assert_eq!(missing.1["code"], "graph_projection_missing");
    let first = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    assert_eq!(first["node_count"], 7);
    assert_eq!(first["edge_count"], 6);
    assert_eq!(first["unresolved"], 1);
    assert_eq!(first["ambiguous"], 1);
    assert_eq!(first["unsupported"], 1);
    let view = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":scope}),
    )
    .await;
    assert_eq!(view["total_nodes"], 7);
    assert_eq!(view["total_edges"], 6);
    let input = path_input(&scope, &facts);
    let baseline = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        input.clone(),
    )
    .await;
    assert_eq!(baseline["edges"].as_array().unwrap().len(), 2);
    assert_eq!(baseline["nodes"][1]["evidence"]["id"], facts[1]);
    let mut reversed = input.clone();
    reversed["direction"] = json!("incoming");
    assert_eq!(
        ok(&h, "POST", &format!("{base}/graph/path"), &owner, reversed).await["status"],
        "no_path_within_bound"
    );
    let mut short = input.clone();
    short["max_hops"] = json!(1);
    assert_eq!(
        ok(&h, "POST", &format!("{base}/graph/path"), &owner, short).await["status"],
        "no_path_within_bound"
    );
    // A real rejected-value decision removes the intermediate source fact from
    // graph admission. The old Neo4j route remains physically present here.
    let mut claim = proposal(&facts[1], "Bridge configuration", "unsupported route");
    claim["content"]["selection"] = scope["selection"].clone();
    claim["content"]["supports"][0]["kind"] = json!("repository_fact");
    claim["content"]["supports"][0]["line_from"] = Value::Null;
    claim["content"]["supports"][0]["line_to"] = Value::Null;
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, claim).await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":claim["id"],"action":"reject","reason":"The supporting bridge assertion is rejected by this fixture."})).await;
    let alternative = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        input.clone(),
    )
    .await;
    assert_eq!(alternative["edges"].as_array().unwrap().len(), 3);
    assert_eq!(alternative["nodes"][1]["evidence"]["id"], facts[2]);
    assert_eq!(alternative["nodes"][2]["evidence"]["id"], facts[3]);
    let second = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    assert_ne!(first["id"], second["id"]);
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            input.clone()
        )
        .await["edges"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let owner_id: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    graph::maintain_brain(&h.state, brain, owner_id)
        .await
        .unwrap_or_else(|e| panic!("{}: {}", e.1, e.2));
    let old = cypher(
        &h,
        "MATCH (g:RecollectGraphGeneration {brain:$brain,id:$id}) RETURN count(g)",
        json!({"brain":brain,"id":first["id"]}),
    )
    .await;
    assert_eq!(old, json!([[0]]));
    let current = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        input.clone(),
    )
    .await;
    assert_eq!(current["view"]["generation"]["id"], second["id"]);
    cypher(&h,"MATCH ()-[r:RECOLLECT_GRAPH_EDGE {brain:$brain,generation:$generation,id:$id}]->() DELETE r RETURN count(r)",json!({"brain":brain,"generation":second["id"],"id":current["edges"][0]["id"]})).await;
    let damaged = h
        .call(
            "POST",
            &format!("{base}/graph/path"),
            Some(&owner),
            input.clone(),
        )
        .await;
    assert_eq!(damaged.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        damaged.1["code"], "graph_import_mismatch",
        "A missing physical relationship cannot become a no-path claim"
    );
    let repaired = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let path = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        input.clone(),
    )
    .await;
    assert_eq!(path["view"]["generation"]["id"], repaired["id"]);
    assert_eq!(path["edges"].as_array().unwrap().len(), 3);
    let mut same = input;
    same["end"] = same["start"].clone();
    let zero = ok(&h, "POST", &format!("{base}/graph/path"), &owner, same).await;
    assert_eq!(zero["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(zero["edges"].as_array().unwrap().len(), 0);
    cleanup(&h, brain).await;
    h.finish().await;
}
