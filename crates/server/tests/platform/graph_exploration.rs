use super::recovery::{Rule, proxy};
use super::*;

pub(super) fn facts(links: &[Vec<usize>]) -> Value {
    json!(links.iter().enumerate().map(|(i, links)| json!({
        "id":format!("n{i}"),"kind":"function","name":format!("node_{i}"),"file":"main.rs","line":1,
        "relations":links.iter().map(|j|json!({"kind":"calls","target_id":format!("n{j}"),"target":format!("n{j}")})).collect::<Vec<_>>()
    })).collect::<Vec<_>>())
}
fn request(scope: &Value, center: Option<&str>, direction: &str, hops: usize) -> Value {
    json!({"scope":scope,"center":center,"direction":direction,"max_hops":hops})
}
fn distances(response: &Value) -> std::collections::BTreeMap<String, usize> {
    response["reach"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let name = response["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["key"] == r["key"])
                .unwrap()["evidence"]["label"]
                .as_str()
                .unwrap()
                .to_owned();
            (name, r["edges"].as_array().unwrap().len())
        })
        .collect()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_exploration_native_reachability_rejects_excluded_hubs_and_damaged_projection() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, records) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/explore/native",
        'a',
        facts(&[
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
    let scope = json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]},"relations":["calls"]});
    let endpoint = format!("{base}/graph/explore");
    let root = combined::entity(&records, "node_0");
    let isolate = combined::entity(&records, "node_5");
    let overview = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        request(&scope, None, "outgoing", 3),
    )
    .await;
    assert_eq!(overview["nodes"].as_array().unwrap().len(), 6);
    assert_eq!(overview["edges"].as_array().unwrap().len(), 8);
    assert_eq!(overview["reach"], json!([]));
    assert_eq!(
        overview["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["from"] == e["to"])
            .count(),
        1
    );
    assert_eq!(
        overview["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["from"] == root && e["to"] == combined::entity(&records, "node_2"))
            .count(),
        2
    );
    let out = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        request(&scope, Some(&root), "outgoing", 3),
    )
    .await;
    assert_eq!(
        distances(&out),
        std::collections::BTreeMap::from([
            ("node_0".into(), 0),
            ("node_1".into(), 1),
            ("node_2".into(), 1),
            ("node_3".into(), 2),
            ("node_4".into(), 2)
        ])
    );
    let incoming = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        request(&scope, Some(&root), "incoming", 3),
    )
    .await;
    assert_eq!(
        distances(&incoming),
        std::collections::BTreeMap::from([("node_0".into(), 0), ("node_2".into(), 1)])
    );
    let alone = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        request(&scope, Some(&isolate), "both", 8),
    )
    .await;
    assert_eq!(alone["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(alone["edges"], json!([]));

    let hub = &records
        .iter()
        .find(|r| r["record"]["name"] == "node_1")
        .unwrap()["id"];
    let mut claim = proposal(hub, "Excluded shorter bridge", "unsupported route");
    claim["content"]["selection"] = scope["selection"].clone();
    claim["content"]["supports"] =
        json!([{"kind":"repository_fact","id":hub,"line_from":null,"line_to":null}]);
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, claim).await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,
        json!({"base_revision":claim["id"],"action":"reject","reason":"Excluded synthetic hub; eligible alternative must remain."})).await;
    for direction in ["outgoing", "both"] {
        let r = ok(
            &h,
            "POST",
            &endpoint,
            &owner,
            request(&scope, Some(&root), direction, 3),
        )
        .await;
        assert_eq!(
            distances(&r),
            std::collections::BTreeMap::from([
                ("node_0".into(), 0),
                ("node_2".into(), 1),
                ("node_3".into(), 3),
                ("node_4".into(), 2)
            ])
        );
        assert_eq!(
            r["view"]["generation"]["id"], generation["id"],
            "Qualification precedes rebuild"
        );
    }
    let short = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        request(&scope, Some(&root), "outgoing", 2),
    )
    .await;
    assert_eq!(
        distances(&short),
        std::collections::BTreeMap::from([
            ("node_0".into(), 0),
            ("node_2".into(), 1),
            ("node_4".into(), 2)
        ])
    );
    let (member, reader) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{member}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    let readable = ok(
        &h,
        "POST",
        &endpoint,
        &reader,
        request(&scope, Some(&root), "outgoing", 2),
    )
    .await;
    assert_eq!(distances(&readable), distances(&short));
    ok(
        &h,
        "DELETE",
        &format!("{base}/grants/{member}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &endpoint,
            Some(&reader),
            request(&scope, Some(&root), "outgoing", 2)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    cypher(
        &h,
        "MATCH ()-[r:RECOLLECT_GRAPH_EDGE {brain:$brain,id:$id}]->() DELETE r",
        json!({"brain":brain,"id":short["edges"][0]["id"]}),
    )
    .await;
    let broken = h
        .call(
            "POST",
            &endpoint,
            Some(&owner),
            request(&scope, Some(&isolate), "outgoing", 2),
        )
        .await;
    assert_eq!(
        broken.1["code"], "graph_import_mismatch",
        "A damaged complete selection must not look like an isolated center"
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_exploration_display_limits_refuse_complete_overflow_and_allow_smaller_positive_controls()
 {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (_, base) = base(&h, &owner).await;
    let mut star = vec![vec![]; 501];
    star[0] = (1..501).collect();
    let (repo, snapshot, records) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/explore/large",
        'b',
        facts(&star),
    )
    .await;
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let scope = json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]},"relations":["calls"]});
    let endpoint = format!("{base}/graph/explore");
    let window = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        json!({"scope":scope,"center":null,"direction":"outgoing","max_hops":1,"windowed":true}),
    )
    .await;
    assert_eq!(window["nodes"].as_array().unwrap().len(), 250);
    assert!(
        window["view"]["coverage"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("graph_window"))
    );
    let root = combined::entity(&records, "node_0");
    let first = root.as_str();
    let neighborhood = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        json!({"scope":scope,"center":first,"direction":"both","max_hops":1,"windowed":true}),
    )
    .await;
    assert_eq!(neighborhood["nodes"].as_array().unwrap().len(), 250);
    assert_eq!(neighborhood["edges"].as_array().unwrap().len(), 249);
    assert!(
        neighborhood["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["key"] == first)
    );
    let page = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":scope,"offset":100,"windowed":true}),
    )
    .await;
    assert_eq!(page["nodes"].as_array().unwrap().len(), 100);
    assert_eq!(page["next_offset"], 200);
    for center in [None, Some(root.as_str())] {
        let refused = h
            .call(
                "POST",
                &endpoint,
                Some(&owner),
                request(&scope, center, "outgoing", 1),
            )
            .await;
        assert_eq!(refused.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(refused.1["code"], "graph_display_too_large");
        assert!(refused.1.get("nodes").is_none());
    }
    let narrow = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        request(&scope, Some(&root), "incoming", 8),
    )
    .await;
    assert_eq!(narrow["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(narrow["view"]["total_nodes"], 501);
    let dense = (0..46)
        .map(|i| (0..46).filter(|j| *j != i).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let (repo, snapshot, records) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/explore/dense",
        'c',
        facts(&dense),
    )
    .await;
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let scope = json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]},"relations":["calls"]});
    let root = combined::entity(&records, "node_0");
    for center in [None, Some(root.as_str())] {
        let refused = h
            .call(
                "POST",
                &endpoint,
                Some(&owner),
                request(&scope, center, "outgoing", 1),
            )
            .await;
        assert_eq!(
            refused.1["code"], "graph_display_too_large",
            "2070 induced edges cannot be truncated"
        );
    }
    let mut narrower = scope.clone();
    narrower["relations"] = json!(["imports"]);
    let admitted = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        request(&narrower, None, "outgoing", 1),
    )
    .await;
    assert_eq!(admitted["nodes"].as_array().unwrap().len(), 46);
    assert_eq!(admitted["edges"], json!([]));
    // Exercise the actual maximum admitted display, separately from overflow.
    let bounded = (0..500)
        .map(|i| (1..=4).map(|j| (i + j) % 500).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let (repo, snapshot, _) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/explore/bounded",
        'e',
        facts(&bounded),
    )
    .await;
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let scope = json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]},"relations":["calls"]});
    let started = std::time::Instant::now();
    let admitted = ok(
        &h,
        "POST",
        &endpoint,
        &owner,
        request(&scope, None, "outgoing", 1),
    )
    .await;
    assert_eq!(admitted["nodes"].as_array().unwrap().len(), 500);
    assert_eq!(admitted["edges"].as_array().unwrap().len(), 2000);
    eprintln!(
        "Complete 500-node/2000-edge exploration: {:?}",
        started.elapsed()
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_exploration_paired_scope_and_malformed_witnesses_fail_closed() {
    let mut h = Harness::new().await;
    let owner = h.login().await;
    let (_, base) = base(&h, &owner).await;
    let (repo, snapshot, records) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/explore/device",
        'd',
        facts(&[vec![1], vec![]]),
    )
    .await;
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let scope = json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo],"area_ids":[],"environment_id":null}});
    let endpoint = format!("{base}/graph/explore");
    let start = combined::entity(&records, "node_0");
    let end = combined::entity(&records, "node_1");
    let (_, token) = h.pair_device(&owner, "Scoped exploration").await;
    let task = scope::device(
        &h,
        &token,
        &format!("{base}/workspace/tasks"),
        json!({"label":"Explore selected repository","selection":scope["selection"]}),
    )
    .await;
    let operation = scope::device(
        &h,
        &token,
        &format!(
            "{base}/workspace/tasks/{}/operations",
            task["task"]["id"].as_str().unwrap()
        ),
        json!({"kind":"retrieval"}),
    )
    .await;
    assert!(
        h.bearer(
            "POST",
            &endpoint,
            &token,
            request(&scope, Some(&start), "outgoing", 1)
        )
        .await
        .0
        .is_client_error()
    );
    let mut scoped = scope.clone();
    scoped["operation_id"] = operation["id"].clone();
    let current = scope::device(
        &h,
        &token,
        &endpoint,
        request(&scoped, Some(&start), "outgoing", 1),
    )
    .await;
    assert_eq!(current["nodes"].as_array().unwrap().len(), 2);
    let mut wide = scoped.clone();
    wide["selection"]["repository_ids"] = json!([]);
    assert!(
        h.bearer(
            "POST",
            &endpoint,
            &token,
            request(&wide, Some(&start), "outgoing", 1)
        )
        .await
        .0
        .is_client_error()
    );
    let (p, proxied, server) = proxy(&h).await;
    h.router = app(proxied);
    for rows in [
        json!([[end, [start, end], [Uuid::new_v4()]]]),
        json!([[end, [end, start], [current["edges"][0]["id"]]]]),
        json!([
            [end, [start, end], [current["edges"][0]["id"]]],
            [end, [start, end], [current["edges"][0]["id"]]]
        ]),
    ] {
        p.arm(Rule {
            contains: "MATCH SHORTEST 1",
            pause: false,
            before: true,
            response: Some((
                StatusCode::OK,
                json!({"data":{"fields":["key","nodes","edges"],"values":rows}}),
            )),
        });
        let refused = h
            .call(
                "POST",
                &endpoint,
                Some(&owner),
                request(&scope, Some(&start), "outgoing", 1),
            )
            .await;
        assert_eq!(refused.1["code"], "graph_response_invalid");
        assert!(refused.1.get("nodes").is_none());
    }
    assert_eq!(
        ok(
            &h,
            "POST",
            &endpoint,
            &owner,
            request(&scope, Some(&start), "outgoing", 1)
        )
        .await["nodes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    server.abort();
    h.finish().await;
}
