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
async fn graph_deadlines_preserve_complete_lineage_and_reviewed_expired_support() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    // Differential control against the accepted original canonical function,
    // in this disposable database only. The caller uses the real app role/RLS.
    let old = include_str!("../../migrations/037_memory_revision_support.sql")
        .split("CREATE FUNCTION recollect_memory_deadline")
        .nth(1)
        .unwrap()
        .split("CREATE FUNCTION recollect_support_audit_target")
        .next()
        .unwrap();
    sqlx::raw_sql(&format!(
        "CREATE FUNCTION recollect_fixture_old_deadline{old}"
    ))
    .execute(&h.admin)
    .await
    .unwrap();
    let old_support = include_str!("../../migrations/043_support_predicate_short_circuit.sql")
        .split("CREATE OR REPLACE FUNCTION recollect_pre_digest_supported")
        .nth(1)
        .unwrap()
        .split("CREATE OR REPLACE FUNCTION recollect_memory_supported")
        .next()
        .unwrap();
    sqlx::raw_sql(&format!(
        "CREATE FUNCTION recollect_fixture_old_support{old_support}"
    ))
    .execute(&h.admin)
    .await
    .unwrap();
    let evidence = source(
        &h,
        &owner,
        &base,
        "Deadline lineage",
        "The service uses port 8080.\n",
    )
    .await;
    let version: Uuid = evidence["version"]["id"].as_str().unwrap().parse().unwrap();
    let mut originals = vec![];
    for subject in ["Service port", "Independent deadline control"] {
        let claim = ok(
            &h,
            "POST",
            &format!("{base}/claims"),
            &owner,
            proposal(&evidence["version"]["id"], subject, "8080"),
        )
        .await;
        originals.push(accepted(&h, &owner, &base, &claim).await);
    }
    let mut content = originals[0]["content"].clone();
    content["kind"] = json!("handover");
    content["subject"] = json!("Exact deadline contributors");
    content["predicate"] = json!("handover");
    content["handover"] = json!({"completed":["Retain both exact contributors."],"next_steps":["Inspect current evidence."],"risks":["Synthetic only."],"contributions":[originals[0]["id"],originals[1]["id"]]});
    let root = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        json!({"content":content}),
    )
    .await;
    let root = accepted(&h, &owner, &base, &root).await;
    let mut content = originals[0]["content"].clone();
    content["rationale"] = json!("A new current revision retains the original assertion.");
    let revised = ok(&h, "POST", &format!("{base}/claims/{}/review",originals[0]["claim_id"].as_str().unwrap()), &owner,
        json!({"base_revision":originals[0]["id"],"action":"revalidate","reason":"Exact current-head deadline control.","revalidation_basis":"review_correction","content":content})).await["claims"][0]["revision"].clone();
    let policy = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut settings = policy["policy"].clone();
    settings["document_days"] = json!(1);
    settings["claim_days"] = json!(30);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":policy["change_id"],"policy":settings}),
    )
    .await;
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let root_id: Uuid = root["id"].as_str().unwrap().parse().unwrap();
    let check =
        async |expired: bool| {
            let mut tx = db::actor_tx(&h.state.pool, actor)
                .await
                .unwrap_or_else(|e| panic!("{}", e.1));
            let dependencies: Vec<Uuid> =
                sqlx::query_scalar("SELECT revision_id FROM recollect_memory_dependencies($1,$2)")
                    .bind(brain)
                    .bind(root_id)
                    .fetch_all(&mut *tx)
                    .await
                    .unwrap();
            assert_eq!(dependencies.len(), 4);
            for revision in [&root, &originals[0], &originals[1], &revised] {
                let id: Uuid = revision["id"].as_str().unwrap().parse().unwrap();
                assert!(dependencies.contains(&id));
                let (supported,original_supported):(bool,bool)=sqlx::query_as(
                "SELECT recollect_pre_digest_supported($1,$2),recollect_fixture_old_support($1,$2)")
                .bind(brain).bind(id).fetch_one(&mut *tx).await.unwrap();
                assert_eq!(
                    supported, original_supported,
                    "Canonical support meaning changed"
                );
                let (current, original): (
                    Option<chrono::DateTime<chrono::Utc>>,
                    Option<chrono::DateTime<chrono::Utc>>,
                ) = sqlx::query_as(
                    "SELECT recollect_memory_deadline($1,$2),recollect_fixture_old_deadline($1,$2)",
                )
                .bind(brain)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
                assert_eq!(current, original, "Complete canonical deadline changed");
                let deadline = current.expect("Finite claim retention is required");
                if expired {
                    assert!(deadline > chrono::Utc::now() + chrono::Duration::days(29));
                } else {
                    assert!(deadline < chrono::Utc::now() + chrono::Duration::days(2));
                }
            }
            tx.rollback().await.unwrap();
        };
    check(false).await;
    sqlx::query("UPDATE source_versions SET created_at=clock_timestamp()-interval '2 days' WHERE brain_id=$1 AND id=$2")
        .bind(brain).bind(version).execute(&h.admin).await.unwrap();
    check(true).await;
    let (foreign, _) = h.fixture_member().await;
    let mut tx = db::actor_tx(&h.state.pool, foreign)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let value: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT recollect_memory_deadline($1,$2)")
            .bind(brain)
            .bind(root_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert!(
        value.is_none(),
        "Foreign Brain deadlines must remain hidden by invoker RLS"
    );
    tx.rollback().await.unwrap();
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    h.finish().await;
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
    let windowed_stale = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":scope,"windowed":true}),
    )
    .await;
    assert!(
        !contains(&windowed_stale, &claims[0]["id"]),
        "Narrowing must not resurrect an old descriptor revision"
    );
    assert!(!contains(&windowed_stale, &revision["id"]));
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
    let unchecked = ok(
        &h,
        "POST",
        &format!("{base}/graph/view"),
        &owner,
        json!({"scope":scope,"windowed":true}),
    )
    .await;
    assert!(
        !contains(&unchecked, &evidence["version"]["id"]),
        "Unchecked assertions cannot qualify the whole source"
    );
    let rejected = accepted(&h, &owner, &base, &rejected).await;
    let independent = accepted(&h, &owner, &base, &independent).await;
    build(&h, &owner, &base, "knowledge", None).await;
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
        let raw = recalled["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["kind"] == "source_version")
            .collect::<Vec<_>>();
        assert!(
            !raw.is_empty(),
            "Safe fragments remain readable alongside accepted independent memory"
        );
        assert!(
            raw.iter()
                .all(|item| !item["text"].as_str().unwrap().contains("8080")),
            "No packed source window may expose the rejected assertion"
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
        let window = ok(
            &h,
            "POST",
            &format!("{base}/graph/view"),
            &owner,
            json!({"scope":scope,"windowed":true}),
        )
        .await;
        assert!(
            !contains(&window, &evidence["version"]["id"]),
            "One unsafe fragment withholds the whole source in a requested window too"
        );
        assert!(contains(&window, &independent["id"]));
        build(&h, &owner, &base, "knowledge", None).await;
    }
    cleanup(&h, brain).await;
    h.finish().await;
}
