use super::*;

async fn source(h: &Harness, owner: &Login, base: &str, title: &str, content: &str) -> Value {
    let value = ok(
        h,
        "POST",
        &format!("{base}/sources"),
        owner,
        json!({"title":title,"media_type":"text/plain","content":content,"retain_content":true}),
    )
    .await;
    for _ in 0..30 {
        if !worker::run_once(&h.state, "capture").await.unwrap() {
            return value;
        }
    }
    panic!("Synthetic source processing did not drain");
}
async fn accepted(h: &Harness, owner: &Login, base: &str, claim: &Value) -> Value {
    ok(h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),owner,json!({"base_revision":claim["id"],"action":"accept","reason":"The synthetic evidence was checked."})).await["claims"][0]["revision"].clone()
}
fn contains(view: &Value, id: &Value) -> bool {
    view["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["evidence"]["revision_id"] == *id)
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_knowledge_preserves_typed_contributions_review_and_current_revisions() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let evidence = source(
        &h,
        &owner,
        &base,
        "Synthetic graph evidence",
        "The controlled service uses port 8080.\nThe operator recorded a successful probe.\n",
    )
    .await;
    let mut claims = vec![];
    for (subject, value) in [("Service port", "8080"), ("Service probe", "successful")] {
        let mut input = proposal(&evidence["version"]["id"], subject, value);
        input["content"]["operational"] = json!("verified");
        input["content"]["observed_at"] = json!("2026-01-15T00:00:00Z");
        input["content"]["observation"] = json!("A synthetic isolated probe observed this value.");
        let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
        claims.push(accepted(&h, &owner, &base, &claim).await);
    }
    let mut content = claims[0]["content"].clone();
    content["kind"] = json!("handover");
    content["subject"] = json!("Synthetic service handover");
    content["predicate"] = json!("handover");
    content["value"] = json!("The linked service evidence is ready for investigation.");
    content["operational"] = json!("declared");
    content["observed_at"] = Value::Null;
    content["observation"] = json!("");
    content["handover"] = json!({"completed":["Recorded two independent contributions."],"next_steps":["Inspect the evidence."],"risks":["Fixture evidence only."],"contributions":[claims[0]["id"],claims[1]["id"]]});
    let handover = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        json!({"content":content}),
    )
    .await;
    let handover = accepted(&h, &owner, &base, &handover).await;
    let generation = build(&h, &owner, &base, "knowledge", None).await;
    assert_eq!(generation["node_count"], 4);
    assert_eq!(generation["edge_count"], 5);
    let scope = json!({"kind":"knowledge","fact_at":"2026-01-15T00:00:00Z"});
    let view = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":scope}),
    )
    .await;
    assert_eq!(view["total_nodes"], 4);
    assert_eq!(view["total_edges"], 5);
    let input = json!({"scope":scope,"start":format!("claim:{}",claims[0]["id"].as_str().unwrap()),"end":format!("claim:{}",handover["id"].as_str().unwrap()),"direction":"outgoing","max_hops":2});
    let path = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        input.clone(),
    )
    .await;
    assert_eq!(path["edges"].as_array().unwrap().len(), 1);
    assert_eq!(path["edges"][0]["relation"], "contributed_to");
    assert_eq!(path["edges"][0]["evidence_id"], handover["id"]);
    let mut strict = scope.clone();
    strict["mode"] = json!("strict_accepted");
    let strict_view = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":strict}),
    )
    .await;
    assert_eq!(strict_view["total_nodes"], 3);
    assert_eq!(strict_view["total_edges"], 2);
    assert!(!contains(&strict_view, &evidence["version"]["id"]));
    strict["mode"] = json!("strict_operational");
    let operational = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":strict}),
    )
    .await;
    assert_eq!(operational["total_nodes"], 2);
    assert_eq!(operational["total_edges"], 0);
    let mut revised = claims[0]["content"].clone();
    revised["rationale"] = json!("Additional evidence recorded in a current revision.");
    let revision = ok(
        &h,
        "POST",
        &format!("{base}/claims/{}/review", claims[0]["claim_id"].as_str().unwrap()),
        &owner,
        json!({"base_revision":claims[0]["id"],"action":"revalidate","reason":"The same assertion was verified against retained evidence again.","revalidation_basis":"review_correction","content":revised}),
    )
    .await["claims"][0]["revision"].clone();
    let old_path = h
        .call("POST", &format!("{base}/graph/path"), Some(&owner), input)
        .await;
    assert_eq!(old_path.0, StatusCode::BAD_REQUEST);
    let stale = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":scope}),
    )
    .await;
    assert!(!contains(&stale, &claims[0]["id"]));
    assert!(!contains(&stale, &revision["id"]));
    assert!(
        stale["coverage"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("knowledge_generation_stale"))
    );
    let rebuilt = build(&h, &owner, &base, "knowledge", None).await;
    let current = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":scope}),
    )
    .await;
    assert!(contains(&current, &revision["id"]));
    assert!(!contains(&current, &claims[0]["id"]));
    let descriptor: sqlx::types::Json<Value> =
        sqlx::query_scalar("SELECT descriptor FROM graph_generations WHERE id=$1")
            .bind(rebuilt["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(
        !descriptor["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["relation"] == "contributed_to"
                && e["from"] == format!("claim:{}", revision["id"].as_str().unwrap())),
        "Revision ancestry cannot invent a current contribution"
    );
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_source_vertex_withholds_all_fragments_when_one_assertion_is_rejected() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let text = format!(
        "The controlled service port is 8080.\n{}",
        "Independent safe evidence line.\n".repeat(300)
    );
    let evidence = source(&h, &owner, &base, "Fragmented evidence", &text).await;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM source_chunks WHERE version_id=$1")
        .bind(
            evidence["version"]["id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap(),
        )
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(count > 1, "Fixture must have distinct fragments");
    let (foreign, _) = h.fixture_member().await;
    let mut isolated = recollect_server::db::actor_tx(&h.state.pool, foreign)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM source_chunks WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&mut *isolated)
            .await
            .unwrap(),
        0
    );
    isolated.rollback().await.unwrap();
    let rejected = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(
            &evidence["version"]["id"],
            "Controlled service port",
            "8080",
        ),
    )
    .await;
    let mut independent = proposal(
        &evidence["version"]["id"],
        "Independent safe evidence",
        "retained",
    );
    independent["content"]["supports"][0]["line_from"] = json!(250);
    independent["content"]["supports"][0]["line_to"] = json!(250);
    let independent = ok(&h, "POST", &format!("{base}/claims"), &owner, independent).await;
    build(&h, &owner, &base, "knowledge", None).await;
    let scope = json!({"kind":"knowledge"});
    let initial = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":scope}),
    )
    .await;
    assert!(contains(&initial, &evidence["version"]["id"]));
    ok(&h,"POST",&format!("{base}/claims/{}/review",rejected["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":rejected["id"],"action":"reject","reason":"The first fragment contains an invalid assertion."})).await;
    for _ in 0..2 {
        let recalled = ok(&h,"POST",&format!("{base}/recall"),&owner,
            json!({"exact":{"kind":"source_version","id":evidence["version"]["id"]},"channels":["exact","graph"],"context_bytes":8192})).await;
        assert_eq!(
            recalled["context"]["items"].as_array().unwrap().len(),
            1,
            "Safe fragment remains readable"
        );
        assert_eq!(
            recalled["graph"]["state"], "no_eligible_anchors",
            "A safe fragment cannot seed its withheld whole-source graph vertex"
        );
        assert!(recalled["context"]["items"][0]["graph_match"].is_null());
        let view = ok(
            &h,
            "POST",
            &format!("{base}/graph/view"),
            &owner,
            json!({"scope":scope}),
        )
        .await;
        assert!(!contains(&view, &evidence["version"]["id"]));
        assert!(contains(&view, &independent["id"]));
        assert_eq!(view["total_edges"], 0);
        build(&h, &owner, &base, "knowledge", None).await;
    }
    cleanup(&h, brain).await;
    h.finish().await;
}
