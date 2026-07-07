use super::*;
use recollect_agent::{Client, StoredDevice, workspace_cli};

async fn recall(h: &Harness, owner: &Login, base: &str, input: Value) -> Value {
    let result = ok(h, "POST", &format!("{base}/recall"), owner, input.clone()).await;
    assert_eq!(
        serde_json::to_vec(&result["context"]).unwrap().len(),
        result["context_bytes"].as_u64().unwrap() as usize
    );
    assert!(
        result["context_bytes"].as_u64().unwrap()
            <= input["context_bytes"].as_u64().unwrap_or(8192)
    );
    result
}
fn names(response: &Value) -> std::collections::BTreeSet<String> {
    response["context"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["label"].as_str().unwrap().into())
        .collect()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_recall_candidate_cap_reports_partial_coverage_without_path_multiplicity_votes() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let mut links = vec![vec![]; 108];
    links[0] = (1..108).collect();
    links[0].extend([1, 1, 0]);
    let (repo, snapshot, _) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/recall/cap",
        'a',
        exploration::facts(&links),
    )
    .await;
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let result=recall(&h,&owner,&base,json!({"query":"node_0","channels":["exact","graph"],
        "selection":{"repository_ids":[repo]},"graph":{"kind":"repository","max_hops":1,"direction":"outgoing"},"limit":4,"context_bytes":32768})).await;
    assert_eq!(result["graph"]["candidates"], 100);
    assert_eq!(result["graph"]["anchors"].as_array().unwrap().len(), 1);
    assert_eq!(result["context"]["items"].as_array().unwrap().len(), 4);
    assert!(
        result["coverage"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("graph_candidate_limit"))
    );
    let items = result["context"]["items"].as_array().unwrap();
    assert_eq!(
        items
            .iter()
            .map(|i| i["id"].as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    for item in items.iter().skip(1) {
        assert_eq!(item["graph_match"]["edges"].as_array().unwrap().len(), 1);
    }
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_recall_fuses_native_relationships_and_preserves_scope_budget_and_failures() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, records) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/recall/native",
        'a',
        exploration::facts(&[
            vec![1, 2, 2],
            vec![3],
            vec![4, 0],
            vec![],
            vec![3, 4],
            vec![],
        ]),
    )
    .await;
    let generation = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let input = json!({"query":"node_0", "selection":{"repository_ids":[repo]}, "channels":["exact","lexical","graph"],
        "graph":{"kind":"repository","direction":"outgoing","max_hops":2,"relations":["calls"]}, "limit":20,"context_bytes":32768});
    let mut baseline = input.clone();
    baseline["channels"] = json!(["exact", "lexical"]);
    baseline.as_object_mut().unwrap().remove("graph");
    let simple = recall(&h, &owner, &base, baseline).await;
    assert_eq!(names(&simple), ["node_0".into()].into());
    let fused = recall(&h, &owner, &base, input.clone()).await;
    assert_eq!(
        names(&fused),
        [
            "node_0".into(),
            "node_1".into(),
            "node_2".into(),
            "node_3".into(),
            "node_4".into()
        ]
        .into()
    );
    assert_eq!(fused["context"]["items"][0]["label"], "node_0");
    assert!(
        fused["context"]["items"][0]["graph_match"].is_null(),
        "Anchors receive no graph boost"
    );
    assert_eq!(fused["graph"]["candidates"], 4);
    assert_eq!(fused["graph"]["view"]["generation"]["id"], generation["id"]);
    assert_eq!(
        fused["context_selection"]["distinct_source_groups"], 1,
        "Neighboring facts from one file preserve useful depth"
    );
    for item in fused["context"]["items"].as_array().unwrap().iter().skip(1) {
        let witness = &item["graph_match"];
        assert!(witness["edges"].as_array().unwrap().len() <= 2);
        assert_eq!(witness["nodes"][0]["label"], "node_0");
        assert_eq!(
            witness["nodes"].as_array().unwrap().last().unwrap()["id"],
            item["id"]
        );
        assert_eq!(item["channels"], json!(["graph"]));
        assert_eq!(item["provenance"][0]["snapshot_id"], snapshot);
        assert!(
            item["qualifications"]
                .as_array()
                .unwrap()
                .contains(&json!("graph_proximity_not_truth"))
        );
    }
    let mut incoming = input.clone();
    incoming["graph"]["direction"] = json!("incoming");
    assert_eq!(
        names(&recall(&h, &owner, &base, incoming).await),
        ["node_0".into(), "node_2".into()].into()
    );
    let mut one = input.clone();
    one["graph"]["max_hops"] = json!(1);
    assert_eq!(
        names(&recall(&h, &owner, &base, one).await),
        ["node_0".into(), "node_1".into(), "node_2".into()].into()
    );
    let mut small = input.clone();
    small["context_bytes"] = json!(2048);
    let budget = recall(&h, &owner, &base, small).await;
    assert_eq!(names(&budget), ["node_0".into()].into());
    assert!(
        budget["coverage"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("context_budget"))
    );
    let mut no_anchor = input.clone();
    no_anchor["query"] = json!("NoSuchAnchor");
    assert_eq!(
        recall(&h, &owner, &base, no_anchor).await["graph"]["state"],
        "no_eligible_anchors"
    );
    let mut strict = input.clone();
    strict["mode"] = json!("strict_accepted");
    let strict = recall(&h, &owner, &base, strict).await;
    assert_eq!(strict["graph"]["state"], "no_eligible_anchors");
    assert!(names(&strict).is_empty());
    for change in [
        json!({"knowledge_at":chrono::Utc::now()}),
        json!({"mode":"history"}),
    ] {
        let mut invalid = input.clone();
        invalid
            .as_object_mut()
            .unwrap()
            .extend(change.as_object().unwrap().clone());
        invalid["channels"] = json!(["exact", "semantic", "graph"]);
        invalid["semantic_request_id"] = json!(Uuid::new_v4());
        let (code, error, _) = h
            .call("POST", &format!("{base}/recall"), Some(&owner), invalid)
            .await;
        assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error["code"], "graph_history_unavailable");
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM model_requests WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );

    // Real native client uses the original immutable operation even after the
    // mutable task default changes. Its graph options cannot replace scope.
    let (device_id, token) = h.pair_device(&owner, "Graph recall native proof").await;
    let task = scope::device(
        &h,
        &token,
        &format!("{base}/workspace/tasks"),
        json!({"label":"Graph context","selection":input["selection"]}),
    )
    .await;
    let task_path = format!(
        "{base}/workspace/tasks/{}",
        task["task"]["id"].as_str().unwrap()
    );
    let operation = scope::device(
        &h,
        &token,
        &format!("{task_path}/operations"),
        json!({"kind":"context"}),
    )
    .await;
    let (code, _) = h
        .bearer(
            "PUT",
            &format!("{task_path}/scope"),
            &token,
            json!({"base_scope":task["task"]["scope"]["id"],"selection":{}}),
        )
        .await;
    assert_eq!(code, StatusCode::OK);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let api = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = Client::new(&endpoint).unwrap();
    let device = StoredDevice {
        endpoint,
        device_id,
        token: token.parse().unwrap(),
    };
    let result = workspace_cli::run(
        &client,
        &device,
        "scope",
        &[
            "recall".into(),
            brain.to_string(),
            operation["id"].as_str().unwrap().into(),
            "node_0".into(),
            "--channels".into(),
            "exact,lexical,graph".into(),
            "--graph-kind".into(),
            "repository".into(),
            "--graph-direction".into(),
            "outgoing".into(),
            "--graph-hops".into(),
            "1".into(),
            "--graph-relations".into(),
            "calls".into(),
            "--source-diversity".into(),
            "false".into(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(result["scope_id"], operation["scope"]["id"]);
    assert_eq!(result["selection"]["repository_ids"], json!([repo]));
    assert_eq!(
        names(&result),
        ["node_0".into(), "node_1".into(), "node_2".into()].into()
    );
    assert_eq!(result["context_selection"]["source_diversity"], false);
    let mut forged = input.clone();
    forged["operation_id"] = operation["id"].clone();
    forged["selection"] = json!({});
    forged["graph"]["kind"] = json!("knowledge");
    assert_eq!(
        h.bearer("POST", &format!("{base}/recall"), &token, forged)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    api.abort();

    // Withhold the shorter hub canonically and retain a qualified alternative.
    let hub = records
        .iter()
        .find(|r| r["record"]["name"] == "node_1")
        .unwrap()["id"]
        .clone();
    let mut proposal = proposal(&hub, "Excluded hub", "not applicable");
    proposal["content"]["selection"] = input["selection"].clone();
    proposal["content"]["supports"] =
        json!([{"kind":"repository_fact","id":hub,"line_from":null,"line_to":null}]);
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, proposal).await;
    ok(
        &h,
        "POST",
        &format!(
            "{base}/claims/{}/review",
            claim["claim_id"].as_str().unwrap()
        ),
        &owner,
        json!({"base_revision":claim["id"],"action":"reject","reason":"Synthetic invalid bridge"}),
    )
    .await;
    let mut longer = input.clone();
    longer["graph"]["max_hops"] = json!(3);
    let corrected = recall(&h, &owner, &base, longer).await;
    assert!(!names(&corrected).contains("node_1"));
    let target = corrected["context"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["label"] == "node_3")
        .unwrap();
    assert_eq!(target["graph_match"]["edges"].as_array().unwrap().len(), 3);
    cypher(
        &h,
        "MATCH (n:RecollectGraphEntity {brain:$brain,key:$key}) DETACH DELETE n",
        json!({"brain":brain,"key":combined::entity(&records,"node_4")}),
    )
    .await;
    let (code, error, _) = h
        .call("POST", &format!("{base}/recall"), Some(&owner), input)
        .await;
    assert!(!code.is_success());
    assert_eq!(error["code"], "graph_import_mismatch");
    cleanup(&h, brain).await;
    h.finish().await;
}
