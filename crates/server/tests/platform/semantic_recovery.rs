use super::*;
use crate::review::proposal;
use recollect_server::{db, privacy_journal};

// Positive graph/embedding fixtures require actual server-attributed human
// review under the source-support contract. Browser submission alone is a
// proposal, and cannot supply usable graph anchors or claim vectors.
async fn reviewed_proposal(h: &Harness, owner: &Login, base: &str, input: Value) -> Value {
    let claim = ok(h, "POST", &format!("{base}/claims"), owner, input).await;
    ok(h, "POST", &format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()), owner,
        json!({"base_revision":claim["id"],"action":"accept","reason":"Checked fixture evidence for this positive control."})).await["claims"][0]["revision"].clone()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_recall_requalifies_after_embedding_and_freezes_new_projection_knowledge() {
    let (h, owner, base, p, provider) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    let victim = source(&h, &owner, &base, "GRAPH_QUERY_ERASED_PAYLOAD").await;
    let control = source(&h, &owner, &base, "Independent live graph query control.").await;
    for (source, name) in [(&victim, "Victim claim"), (&control, "Control claim")] {
        reviewed_proposal(
            &h,
            &owner,
            &base,
            proposal(&source["version"]["id"], name, "retained declaration"),
        )
        .await;
    }
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    p.entered.notified().await;
    crate::graph::build(&h, &owner, &base, "knowledge", None).await;
    p.delay.store(3000, Ordering::SeqCst);
    let input = query("Find retained engineering evidence", &["semantic", "graph"]);
    let url = format!("{base}/recall");
    let change = async {
        p.entered.notified().await;
        erase(
            &h,
            &owner,
            &base,
            json!({"kind":"source","id":victim["id"]}),
        )
        .await;
        let late = reviewed_proposal(
            &h,
            &owner,
            &base,
            proposal(
                &control["version"]["id"],
                "LATE_GRAPH_QUERY_KNOWLEDGE",
                "not known at query admission",
            ),
        )
        .await;
        let projection = crate::graph::build(&h, &owner, &base, "knowledge", None).await;
        (late, projection)
    };
    let ((code, result, _), (late, projection)) =
        tokio::join!(h.call("POST", &url, Some(&owner), input), change);
    assert_eq!(code, StatusCode::OK, "{result}");
    assert_eq!(result["context"]["items"].as_array().unwrap().len(), 2);
    assert!(
        result["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == control["version"]["id"])
    );
    assert!(!result.to_string().contains("GRAPH_QUERY_ERASED_PAYLOAD"));
    assert!(!result.to_string().contains("LATE_GRAPH_QUERY_KNOWLEDGE"));
    assert!(
        !result["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == late["claim_id"])
    );
    assert_eq!(
        result["graph"]["view"]["generation"]["id"],
        projection["id"]
    );
    assert_eq!(
        result["graph"]["anchors"].as_array().unwrap().len(),
        2,
        "Both surviving supported representations are qualified graph anchors"
    );
    assert_eq!(
        result["graph"]["candidates"], 0,
        "Qualified anchors get no graph vote and the late claim cannot enter the frozen query"
    );
    p.delay.store(0, Ordering::SeqCst);
    let current = ok(
        &h,
        "POST",
        &url,
        &owner,
        query("Find retained engineering evidence", &["semantic", "graph"]),
    )
    .await;
    assert!(
        current["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == late["claim_id"]),
        "A new query must see newly known evidence"
    );
    provider.abort();
    crate::graph::cleanup(&h, cx.brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_recall_budget_covers_both_provider_sides_without_replaying_a_charged_query() {
    use crate::graph::recovery::{Rule, proxy};
    let (mut h, owner, base, p, provider) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    let evidence = source(&h, &owner, &base, "Graph budget retained evidence.").await;
    reviewed_proposal(
        &h,
        &owner,
        &base,
        proposal(
            &evidence["version"]["id"],
            "Budget anchor",
            "linked evidence",
        ),
    )
    .await;
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    crate::graph::build(&h, &owner, &base, "knowledge", None).await;
    let (proxy, proxied, graph_server) = proxy(&h).await;
    h.router = app(proxied);
    let arm = || {
        proxy.arm(Rule {
            contains: "MATCH",
            pause: true,
            before: true,
            response: None,
        })
    };
    arm();
    // Each native HTTP call is individually inside its five-second transport
    // bound. Neither verification phase alone reaches the 15-second aggregate.
    p.delay.store(2500, Ordering::SeqCst);
    let input = query("Budget anchor", &["exact", "semantic", "graph"]);
    let path = format!("{base}/recall");
    let calls = p.calls.load(Ordering::SeqCst);
    let started = std::time::Instant::now();
    let delays = async {
        for ordinal in 0..6 {
            proxy.entered().await;
            tokio::time::sleep(std::time::Duration::from_millis(2600)).await;
            if ordinal < 5 {
                arm();
            }
            proxy.release.notify_one();
        }
    };
    let ((code, result, _), ()) =
        tokio::join!(h.call("POST", &path, Some(&owner), input.clone()), delays);
    assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE, "{result}");
    assert_eq!(result["code"], "graph_timeout");
    assert!(result["context"].is_null());
    assert!(
        started.elapsed() >= std::time::Duration::from_secs(17),
        "Provider time is excluded from graph work time"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), calls + 1);
    let (_, replay, _) = h.call("POST", &path, Some(&owner), input).await;
    assert_eq!(replay["code"], "model_attempt_recorded");
    assert_eq!(p.calls.load(Ordering::SeqCst), calls + 1);
    p.delay.store(0, Ordering::SeqCst);
    let control = ok(
        &h,
        "POST",
        &path,
        &owner,
        query("Budget anchor", &["exact", "semantic", "graph"]),
    )
    .await;
    assert!(!control["context"]["items"].as_array().unwrap().is_empty());
    assert_eq!(p.calls.load(Ordering::SeqCst), calls + 2);
    graph_server.abort();
    provider.abort();
    crate::graph::cleanup(&h, cx.brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn semantic_fusion_keeps_the_score_bearing_fragment_and_exact_priority() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    p.mode.store(5, Ordering::SeqCst);
    let first = format!(
        "DISTRACTOR_VECTOR LEXICAL_ONLY dispatch guide. {}",
        "padding ".repeat(600)
    );
    let text = format!(
        "{}\nNEAR_VECTOR SEMANTIC_ONLY recover failed callback delivery.\n",
        &first[..4095]
    );
    let split = source(&h, &owner, &base, &text).await;
    let control = source(&h, &owner, &base, "Independent highest cosine control.").await;
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 3);
    let lexical = recall(
        &h,
        &owner,
        &base,
        query("LEXICAL_ONLY", &["exact", "lexical"]),
    )
    .await;
    assert_eq!(
        lexical["context"]["items"][0]["provenance"][0]["byte_from"],
        0
    );
    assert!(
        lexical["context"]["items"][0]["text"]
            .as_str()
            .unwrap()
            .contains("LEXICAL_ONLY")
    );
    for channels in [vec!["semantic"], vec!["exact", "lexical", "semantic"]] {
        let result = recall(&h, &owner, &base, query("LEXICAL_ONLY", &channels)).await;
        let item = result["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == split["version"]["id"])
            .unwrap();
        assert!(item["text"].as_str().unwrap().contains("SEMANTIC_ONLY"));
        assert!(!item["text"].as_str().unwrap().contains("LEXICAL_ONLY"));
        assert_eq!(item["provenance"][0]["byte_from"], 4096);
        assert_eq!(item["provenance"][0]["byte_to"], text.len());
        assert!((item["semantic_similarity"].as_f64().unwrap() - 0.6).abs() < 1e-6);
        if channels.len() > 1 {
            assert!(
                item["qualifications"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("lexical_match_in_another_fragment"))
            );
            assert!(
                item["channels"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("lexical"))
            );
        } else {
            assert_eq!(
                result["context"]["items"][0]["id"],
                control["version"]["id"]
            );
        }
    }
    let mut explicit = query("LEXICAL_ONLY", &["exact", "lexical", "semantic"]);
    explicit["exact"] = json!({"kind":"source_version","id":split["version"]["id"]});
    let result = recall(&h, &owner, &base, explicit).await;
    assert_eq!(result["context"]["items"][0]["id"], split["version"]["id"]);
    assert_eq!(
        result["context"]["items"][0]["provenance"][0]["byte_from"],
        4096
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    // Three exact anchors keep the scored source outside the anchor set. It
    // must get one graph vote while retaining its second, score-bearing span.
    for predicate in ["handling", "recovery", "dispatch"] {
        let mut input = proposal(&split["version"]["id"], "LEXICAL_ONLY", "qualified anchor");
        input["content"]["predicate"] = json!(predicate);
        reviewed_proposal(&h, &owner, &base, input).await;
    }
    crate::graph::build(&h, &owner, &base, "knowledge", None).await;
    let mut graph_query = query("LEXICAL_ONLY", &["exact", "lexical", "semantic", "graph"]);
    graph_query["context_bytes"] = json!(32768);
    let result = ok(&h, "POST", &format!("{base}/recall"), &owner, graph_query).await;
    let item = result["context"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == split["version"]["id"])
        .unwrap();
    assert_eq!(item["provenance"][0]["byte_from"], 4096);
    assert!(item["text"].as_str().unwrap().contains("SEMANTIC_ONLY"));
    assert_eq!(item["graph_match"]["edges"].as_array().unwrap().len(), 1);
    assert_eq!(
        item["channels"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| **c == "graph")
            .count(),
        1
    );
    assert!((item["semantic_similarity"].as_f64().unwrap() - 0.6).abs() < 1e-6);
    assert_eq!(result["graph"]["anchors"].as_array().unwrap().len(), 3);
    assert_eq!(
        result["graph"]["candidates"], 1,
        "Three anchor paths provide one graph contribution for their shared target"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 5);
    crate::graph::cypher(&h,"MATCH (n:RecollectGraphEntity {brain:$brain,key:$key}) DETACH DELETE n",
        json!({"brain":cx.brain,"key":format!("source_version:{}",split["version"]["id"].as_str().unwrap())})).await;
    let (code, error, _) = h
        .call(
            "POST",
            &format!("{base}/recall"),
            Some(&owner),
            query("LEXICAL_ONLY", &["exact", "lexical", "semantic", "graph"]),
        )
        .await;
    assert!(!code.is_success());
    assert_eq!(error["code"], "graph_import_mismatch");
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        5,
        "Physical preflight must refuse before another query charge"
    );
    crate::graph::cleanup(&h, cx.brain).await;
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_handover_publication_checks_contributor_deadline_without_inheriting_raw_ttl() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(true);
        policy["content_classes"] = json!(["document", "query", "claim"]);
    })
    .await;
    let evidence = source(
        &h,
        &owner,
        &base,
        "Quartz port 8080 is a synthetic declaration.\n",
    )
    .await;
    let contribution = accepted(&h, &owner, &base, &evidence, "Quartz", "8080").await;
    let control = source(
        &h,
        &owner,
        &base,
        "Independent handover publication control.",
    )
    .await;
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let mut body = review::proposal(
        &evidence["version"]["id"],
        "Summary",
        "Quartz has a declared endpoint.",
    );
    body["content"]["kind"] = json!("handover");
    body["content"]["handover"] = json!({"completed":["Recorded the declaration."],"next_steps":["Inspect supporting evidence."],"risks":["No deployment observation."],"contributions":[contribution["id"]]});
    let summary = reviewed_proposal(&h, &owner, &base, body).await;
    discover(&h, cx).await;
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["claim_days"] = json!(1);
    policy["document_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    let summary_id = summary["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    sqlx::query(&format!("CREATE FUNCTION semantic_contributor_pause() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.claim_revision_id='{summary_id}'::uuid AND NEW.state='ready' THEN PERFORM pg_sleep(2.2); END IF; RETURN NEW; END $$"))
        .execute(&h.admin).await.unwrap();
    sqlx::query("CREATE TRIGGER semantic_contributor_pause BEFORE UPDATE OF embedding ON semantic_entries FOR EACH ROW EXECUTE FUNCTION semantic_contributor_pause()")
        .execute(&h.admin).await.unwrap();
    sqlx::query("WITH origin AS (SELECT clock_timestamp()-interval '1 day'+interval '1 second' AS at),
        times AS (SELECT r.id,origin.at-CASE WHEN r.id=$2 THEN interval '0 seconds' ELSE interval '1 second' END AS at
          FROM claim_revisions r CROSS JOIN origin WHERE r.claim_id=$1)
        UPDATE claim_revisions r SET recorded_at=t.at,revision=jsonb_set(r.revision,'{recorded_at}',to_jsonb(t.at)) FROM times t WHERE r.id=t.id")
        .bind(contribution["claim_id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .bind(contribution["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
    model_job(&h).await;
    let result = status(&h, &owner, &base).await;
    assert_eq!(result["batches"][0]["error_code"], "semantic_input_expired");
    let present:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM semantic_entries WHERE claim_revision_id=$1 AND embedding IS NOT NULL)")
        .bind(summary_id).fetch_one(&h.admin).await.unwrap();
    assert!(
        !present,
        "An expired required contributor prevents atomic publication"
    );

    // An ordinary independently retained claim does not inherit raw support TTL.
    // Queue it while its support is live, expire only that raw support, and prove
    // a separate batch still publishes a qualified, useful claim representation.
    let independent = accepted(
        &h,
        &owner,
        &base,
        &evidence,
        "Independent declaration",
        "8080",
    )
    .await;
    discover(&h, cx).await;
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '2 days' WHERE id=$1",
    )
    .bind(
        evidence["version"]["id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .execute(&h.admin)
    .await
    .unwrap();
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let present:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM semantic_entries WHERE claim_revision_id=$1 AND state='ready' AND embedding IS NOT NULL)")
        .bind(independent["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    assert!(
        present,
        "Ordinary expired support is not a handover contributor deadline"
    );
    let result = recall(
        &h,
        &owner,
        &base,
        query("Retained declarations", &["semantic"]),
    )
    .await;
    let items = result["context"]["items"].as_array().unwrap();
    assert!(items.iter().any(|i| i["id"] == independent["claim_id"]));
    assert!(items.iter().any(|i| i["id"] == control["version"]["id"]));
    assert!(p.calls.load(Ordering::SeqCst) >= 4);
    server.abort();
    h.finish().await;
}

async fn erase(h: &Harness, owner: &Login, base: &str, target: Value) {
    let preview = ok(
        h,
        "POST",
        &format!("{base}/erasures/preview"),
        owner,
        target.clone(),
    )
    .await;
    ok(
        h,
        "POST",
        &format!("{base}/erasures"),
        owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_query_rechecks_erasure_and_profile_and_never_silently_falls_back() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    let victim = source(
        &h,
        &owner,
        &base,
        "QUERY_ERASURE_CANARY cannot survive the query boundary.",
    )
    .await;
    let control = source(&h, &owner, &base, "Independent retained query control.").await;
    discover(&h, cx).await;
    model_job(&h).await;
    p.entered.notified().await;
    p.delay.store(200, Ordering::SeqCst);
    let input = query("Query the retained evidence", &["semantic"]);
    let path = format!("{base}/recall");
    let pending = h.call("POST", &path, Some(&owner), input);
    let remove = async {
        p.entered.notified().await;
        erase(
            &h,
            &owner,
            &base,
            json!({"kind":"source","id":victim["id"]}),
        )
        .await;
    };
    let ((code, result, _), ()) = tokio::join!(pending, remove);
    assert_eq!(code, StatusCode::OK, "{result}");
    assert_eq!(result["context"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["context"]["items"][0]["id"],
        control["version"]["id"]
    );
    assert!(!result.to_string().contains("QUERY_ERASURE_CANARY"));
    let old_profile = status(&h, &owner, &base).await["profile"]["id"].clone();
    let input = query(
        "Independent retained query control",
        &["exact", "lexical", "semantic"],
    );
    let pending = h.call("POST", &path, Some(&owner), input.clone());
    let rebuild = async {
        p.entered.notified().await;
        ok(
            &h,
            "POST",
            &format!("{base}/semantic/reindex"),
            &owner,
            json!({"base_profile":old_profile}),
        )
        .await;
    };
    let ((code, result, _), ()) = tokio::join!(pending, rebuild);
    assert_eq!(code, StatusCode::CONFLICT, "{result}");
    assert_eq!(result["code"], "semantic_profile_changed");
    assert!(
        result["context"].is_null(),
        "A failed semantic channel cannot masquerade as successful lexical recall"
    );
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 0);
    let calls = p.calls.load(Ordering::SeqCst);
    let (code, replay, _) = h.call("POST", &path, Some(&owner), input).await;
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(
        replay["code"], "model_attempt_recorded",
        "Even an empty rebuilt index preserves the admitted query's outcome"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), calls);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_current_manifests_preserve_selected_repository_revision_and_privacy() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    let repository = Uuid::new_v4();
    sqlx::query("INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,'example.test/semantic/synthetic',$3)")
        .bind(repository).bind(cx.brain).bind(cx.actor).execute(&h.admin).await.unwrap();
    let mut snapshots = Vec::new();
    let mut facts = Vec::new();
    for (i, name) in ["production_callback", "development_callback"]
        .into_iter()
        .enumerate()
    {
        let snapshot = Uuid::new_v4();
        let fact = Uuid::new_v4();
        sqlx::query("INSERT INTO repository_snapshots(id,brain_id,repository_id,revision,adapter,adapter_build,extractor_version,settings,coverage,file_count,fact_count,retained_file_count,created_by)
            VALUES($1,$2,$3,$4,'fixture','fixture','fixture','{}','{}',1,1,0,$5)")
            .bind(snapshot).bind(cx.brain).bind(repository).bind(if i==0 {"a".repeat(40)} else {"b".repeat(40)}).bind(cx.actor).execute(&h.admin).await.unwrap();
        sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) VALUES($1,$2,$3,0,$4)")
            .bind(fact).bind(cx.brain).bind(snapshot).bind(json!({"kind":"function","name":name,"file":"src/callback.rs","line":17})).execute(&h.admin).await.unwrap();
        snapshots.push(snapshot);
        facts.push(fact);
    }
    let environment = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Production"}),
    )
    .await;
    let manifest=ok(&h,"POST",&format!("{base}/revision-manifests"),&owner,
        json!({"name":"Current production selection","environment_id":environment["id"],"kind":"committed","entries":[{"repository_id":repository,"revision":"a".repeat(40),"snapshot_id":snapshots[0],"config_paths":[]}],"notes":"","base_revision":null,"operation_id":null})).await;
    assert_eq!(
        discover(&h, cx).await,
        0,
        "Repository content is not permitted by the original policy"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(true);
        policy["content_classes"]
            .as_array_mut()
            .unwrap()
            .push(json!("repository"));
    })
    .await;
    assert_eq!(discover(&h, cx).await, 1);
    model_job(&h).await;
    assert_eq!(
        status(&h, &owner, &base).await["counts"]["ready"],
        3,
        "Both current repository contexts and the manifest receive representations"
    );
    let mut input = query("Find the callback declaration", &["semantic"]);
    input["selection"] =
        json!({"repository_ids":[repository],"area_ids":[],"environment_id":environment["id"]});
    input["manifest_revision_id"] = manifest["id"].clone();
    let selected = recall(&h, &owner, &base, input.clone()).await;
    let items = selected["context"]["items"].as_array().unwrap();
    assert!(items.iter().any(|i| i["id"] == facts[0].to_string()));
    assert!(!items.iter().any(|i| i["id"] == facts[1].to_string()));
    let fact = items
        .iter()
        .find(|i| i["id"] == facts[0].to_string())
        .unwrap();
    assert_eq!(
        fact["provenance"][0]["snapshot_id"],
        snapshots[0].to_string()
    );
    assert!(
        fact["qualifications"]
            .as_array()
            .unwrap()
            .iter()
            .any(|q| q == "committed_structure_not_deployment_proof")
    );
    // These claims have the same repository/environment scope. One cites a
    // different snapshot; another selects different manifest config paths.
    // Both must be excluded before counting vectors, not just after ranking.
    let alternate=ok(&h,"POST",&format!("{base}/revision-manifests"),&owner,
        json!({"name":"Other configuration paths","environment_id":environment["id"],"kind":"committed","entries":[{"repository_id":repository,"revision":"a".repeat(40),"snapshot_id":snapshots[0],"config_paths":["other.tf"]}],"notes":"","base_revision":null,"operation_id":null})).await;
    let mut claims = Vec::new();
    for (i, subject) in [
        "Selected declaration",
        "Wrong snapshot declaration",
        "Wrong manifest declaration",
    ]
    .into_iter()
    .enumerate()
    {
        let mut body = review::proposal(&json!(facts[0]), subject, "callback");
        body["content"]["selection"] = input["selection"].clone();
        body["content"]["supports"] = json!([{"kind":"repository_fact","id":facts[usize::from(i==1)],"line_from":null,"line_to":null}]);
        if i == 2 {
            body["content"]["manifest_revision_id"] = alternate["id"].clone();
        }
        let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, body).await;
        let accepted = ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,
            json!({"base_revision":claim["id"],"action":"accept","reason":"Accept synthetic structural declaration."})).await["claims"][0]["revision"].clone();
        claims.push(accepted);
    }
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(true);
        policy["content_classes"] = json!(["document", "query", "repository", "claim"]);
    })
    .await;
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let vectors:i64=sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE claim_revision_id=ANY($1) AND embedding IS NOT NULL")
        .bind(claims.iter().map(|r|r["id"].as_str().unwrap().parse::<Uuid>().unwrap()).collect::<Vec<_>>())
        .fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        vectors, 3,
        "All three actual vectors exist before selecting a manifest"
    );
    let mut strict = input.clone();
    strict["semantic_request_id"] = json!(Uuid::new_v4());
    strict["mode"] = json!("strict_accepted");
    let selected = recall(&h, &owner, &base, strict).await;
    assert_eq!(
        selected["semantic"]["scoped_entries"], 1,
        "Incompatible claim vectors cannot influence scope admission or distance"
    );
    assert_eq!(selected["context"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        selected["context"]["items"][0]["revision_id"],
        claims[0]["id"]
    );
    let mut latest = query("Find the callback declaration", &["exact", "semantic"]);
    latest["exact"] = json!({"kind":"repository_fact","id":facts[1]});
    latest["selection"] = json!({"repository_ids":[repository]});
    let latest = recall(&h, &owner, &base, latest).await;
    assert!(
        latest["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == facts[1].to_string())
    );
    assert!(
        !latest["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == facts[0].to_string())
    );
    erase(
        &h,
        &owner,
        &base,
        json!({"kind":"snapshot","id":snapshots[0]}),
    )
    .await;
    let vectors:i64=sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE (fact_id=$1 OR manifest_revision_id=$2) AND embedding IS NOT NULL")
        .bind(facts[0]).bind(manifest["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        vectors, 0,
        "Snapshot erasure also invalidates its manifest representation"
    );
    let mut latest = query("Find the callback declaration", &["exact", "semantic"]);
    latest["exact"] = json!({"kind":"repository_fact","id":facts[1]});
    latest["selection"] = json!({"repository_ids":[repository]});
    let remaining = recall(&h, &owner, &base, latest).await;
    assert!(
        remaining["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == facts[1].to_string())
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_publication_keeps_polling_across_renewal_and_rechecks_expiry() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    let control = source(
        &h,
        &owner,
        &base,
        "Independent retained vector survives the failed publication.",
    )
    .await;
    discover(&h, cx).await;
    model_job(&h).await;
    let expiring = source(
        &h,
        &owner,
        &base,
        "The publication must discard this soon-expired representation.",
    )
    .await;
    discover(&h, cx).await;
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["document_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    let version = expiring["version"]["id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    sqlx::query(&format!("CREATE FUNCTION semantic_publication_pause() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.source_version_id='{version}'::uuid AND NEW.state='ready' THEN PERFORM pg_sleep(6.2); END IF; RETURN NEW; END $$"))
        .execute(&h.admin).await.unwrap();
    sqlx::query("CREATE TRIGGER semantic_publication_pause BEFORE UPDATE OF embedding ON semantic_entries FOR EACH ROW EXECUTE FUNCTION semantic_publication_pause()")
        .execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE source_versions SET created_at=clock_timestamp()-interval '1 day'+interval '2 seconds' WHERE id=$1")
        .bind(version).execute(&h.admin).await.unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(15),
        worker::run_once(&h.state, "model"),
    )
    .await
    .expect("Publication must keep polling while renewal waits on its job row")
    .unwrap();
    let result = status(&h, &owner, &base).await;
    assert_eq!(result["counts"]["ready"], 1);
    assert_eq!(result["counts"]["failed"], 1);
    assert_eq!(result["batches"][0]["error_code"], "semantic_input_expired");
    assert!(result["batches"][0]["request_id"].is_string());
    let payload:i64=sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE source_version_id=$1 AND embedding IS NOT NULL")
        .bind(version).fetch_one(&h.admin).await.unwrap();
    assert_eq!(payload, 0);
    privacy_journal::run_once(&h.state).await.unwrap();
    assert_eq!(status(&h, &owner, &base).await["counts"]["removed"], 1);
    let recall = recall(
        &h,
        &owner,
        &base,
        query("Retained independent evidence", &["semantic"]),
    )
    .await;
    assert_eq!(
        recall["context"]["items"][0]["id"],
        control["version"]["id"]
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_gateway_correlates_batch_indices_and_rejects_invalid_vectors() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    let near = source(
        &h,
        &owner,
        &base,
        "NEAR_VECTOR is the first declared source.",
    )
    .await;
    let other = source(
        &h,
        &owner,
        &base,
        "DISTRACTOR_VECTOR is the independent second source.",
    )
    .await;
    discover(&h, cx).await;
    let mut ids = Vec::new();
    for source in [near, other] {
        let id: Uuid =
            sqlx::query_scalar("SELECT id FROM semantic_entries WHERE source_version_id=$1")
                .bind(
                    source["version"]["id"]
                        .as_str()
                        .unwrap()
                        .parse::<Uuid>()
                        .unwrap(),
                )
                .fetch_one(&h.admin)
                .await
                .unwrap();
        ids.push(id);
    }
    p.mode.store(7, Ordering::SeqCst);
    let output = gateway::invoke(&h.state, cx, embed(&ids))
        .await
        .map_err(|e| e.1)
        .unwrap();
    let Some(gateway::Output::Embeddings(vectors)) = output.output else {
        panic!("Expected correctly correlated vectors")
    };
    assert_eq!(&vectors[0][..2], &[3.0, 4.0]);
    assert_eq!(&vectors[1][..2], &[0.0, 1.0]);
    for mode in [4, 8, 9, 10] {
        p.mode.store(mode, Ordering::SeqCst);
        let selected = if mode == 4 { &ids[..1] } else { &ids[..] };
        let error = gateway::invoke(&h.state, cx, embed(selected))
            .await
            .err()
            .unwrap();
        assert_eq!(
            error.1, "provider_shape",
            "Wrong dimensions/model, zero norm and duplicate indices are unusable"
        );
    }
    let ready: i64 =
        sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE state='ready'")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        ready, 0,
        "The gateway cannot publish a projection by itself"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 5);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_large_scope_refuses_before_charging_and_narrow_scope_remains_useful() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    let large = source(
        &h,
        &owner,
        &base,
        "Adversarial repeated projection fragment.",
    )
    .await;
    let control = source(
        &h,
        &owner,
        &base,
        "A separate selected source remains usable.",
    )
    .await;
    let collection = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"collection","name":"Narrow semantic scope"}),
    )
    .await;
    ok(
        &h,
        "PUT",
        &format!("{base}/sources/{}/groups", control["id"].as_str().unwrap()),
        &owner,
        json!({"group_ids":[collection["id"]]}),
    )
    .await;
    discover(&h, cx).await;
    model_job(&h).await;
    let version = large["version"]["id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    // Adversarial internal projection, with exact retained spans and valid
    // canonical parents. It must not turn the scoped scan into a partial answer.
    sqlx::query("INSERT INTO source_chunks(id,brain_id,version_id,ordinal,byte_start,byte_end,line_start,line_end,content)
        SELECT gen_random_uuid(),c.brain_id,c.version_id,n,c.byte_start,c.byte_end,c.line_start,c.line_end,c.content
        FROM source_chunks c CROSS JOIN generate_series(1,5000) n WHERE c.version_id=$1 AND c.ordinal=0")
        .bind(version).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id,state,embedding)
        SELECT gen_random_uuid(),e.brain_id,e.profile_id,'source_chunk',c.id,c.id,c.version_id,'ready',e.embedding
        FROM source_chunks c CROSS JOIN semantic_entries e WHERE c.version_id=$1 AND c.ordinal>0
        AND e.source_version_id=$1 AND e.state='ready'")
        .bind(version).execute(&h.admin).await.unwrap();
    let before = p.calls.load(Ordering::SeqCst);
    let (code, body, _) = h
        .call(
            "POST",
            &format!("{base}/recall"),
            Some(&owner),
            query("Engineering source", &["semantic"]),
        )
        .await;
    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "semantic_scope_too_large");
    assert_eq!(p.calls.load(Ordering::SeqCst), before);
    let mut input = query("Engineering source", &["semantic"]);
    input["collection_id"] = collection["id"].clone();
    let result = recall(&h, &owner, &base, input).await;
    assert_eq!(result["semantic"]["scoped_entries"], 1);
    assert_eq!(
        result["context"]["items"][0]["id"],
        control["version"]["id"]
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), before + 1);
    server.abort();
    h.finish().await;
}

async fn accepted(
    h: &Harness,
    owner: &Login,
    base: &str,
    source: &Value,
    subject: &str,
    value: &str,
) -> Value {
    let mut input = review::proposal(&source["version"]["id"], subject, value);
    input["content"]["predicate"] = json!("port");
    input["content"]["validity"] =
        json!({"kind":"unknown","from":null,"to":null,"precision":"unknown"});
    let claim = ok(h, "POST", &format!("{base}/claims"), owner, input).await;
    ok(h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),owner,
        json!({"base_revision":claim["id"],"action":"accept","reason":"Accept the synthetic declared source evidence."})).await["claims"][0]["revision"].clone()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn native_semantic_recall_keeps_its_operation_scope_and_uses_one_query_attempt() {
    use recollect_agent::{Client, StoredDevice, workspace_cli};
    let (h, owner, base, p, provider) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    let a = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Semantic task A"}),
    )
    .await;
    let b = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Semantic task B"}),
    )
    .await;
    let source_a = source(
        &h,
        &owner,
        &base,
        "Native semantic evidence belongs to task A.",
    )
    .await;
    let source_b = source(
        &h,
        &owner,
        &base,
        "Native semantic evidence belongs to task B.",
    )
    .await;
    for (source, area) in [(&source_a, &a), (&source_b, &b)] {
        ok(
            &h,
            "PUT",
            &format!("{base}/sources/{}/groups", source["id"].as_str().unwrap()),
            &owner,
            json!({"group_ids":[area["id"]]}),
        )
        .await;
    }
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let (device_id, token) = h
        .pair_device(&owner, "Native semantic scoped fixture")
        .await;
    let scope_a = json!({"repository_ids":[],"area_ids":[a["id"]],"environment_id":null});
    let scope_b = json!({"repository_ids":[],"area_ids":[b["id"]],"environment_id":null});
    let (code, task) = h
        .bearer(
            "POST",
            &format!("{base}/workspace/tasks"),
            &token,
            json!({"label":"Native semantic task","selection":scope_a}),
        )
        .await;
    assert_eq!(code, StatusCode::OK, "{task}");
    let task_path = format!(
        "{base}/workspace/tasks/{}",
        task["task"]["id"].as_str().unwrap()
    );
    let (code, operation) = h
        .bearer(
            "POST",
            &format!("{task_path}/operations"),
            &token,
            json!({"kind":"context"}),
        )
        .await;
    assert_eq!(code, StatusCode::OK, "{operation}");
    let (code, changed) = h
        .bearer(
            "PUT",
            &format!("{task_path}/scope"),
            &token,
            json!({"base_scope":task["task"]["scope"]["id"],"selection":scope_b}),
        )
        .await;
    assert_eq!(code, StatusCode::OK, "{changed}");
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
    let arguments = vec![
        "recall".into(),
        cx.brain.to_string(),
        operation["id"].as_str().unwrap().into(),
        "Recall the engineering evidence".into(),
        "--channels".into(),
        "semantic".into(),
    ];
    let calls = p.calls.load(Ordering::SeqCst);
    let first = workspace_cli::run(&client, &device, "scope", &arguments)
        .await
        .unwrap();
    assert_eq!(p.calls.load(Ordering::SeqCst), calls + 1);
    assert_eq!(first["scope_id"], operation["scope"]["id"]);
    assert_eq!(first["selection"], scope_a);
    assert_eq!(first["context"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        first["context"]["items"][0]["id"],
        source_a["version"]["id"]
    );
    let second = workspace_cli::run(&client, &device, "scope", &arguments)
        .await
        .unwrap();
    assert_ne!(
        first["semantic"]["model_request_id"], second["semantic"]["model_request_id"],
        "A second explicit native invocation is a new query attempt"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), calls + 2);
    let mut forged = query("Recall evidence", &["semantic"]);
    forged["operation_id"] = operation["id"].clone();
    forged["selection"] = scope_b;
    assert_eq!(
        h.bearer("POST", &format!("{base}/recall"), &token, forged)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), calls + 2);
    // The statement-scoped RLS Brain sets retain read/write distinctions and
    // observe revoked access on the next statement; there is no session cache.
    let (reader_id, reader) = h.fixture_member().await;
    let mut tx = db::actor_tx(&h.state.pool, reader_id)
        .await
        .map_err(|e| e.1)
        .unwrap();
    let hidden: i64 = sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE brain_id=$1")
        .bind(cx.brain)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(hidden, 0);
    tx.rollback().await.unwrap();
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    assert_eq!(status(&h, &reader, &base).await["counts"]["ready"], 2);
    let mut tx = db::actor_tx(&h.state.pool, reader_id)
        .await
        .map_err(|e| e.1)
        .unwrap();
    let changed =
        sqlx::query("UPDATE semantic_entries SET state='removed',embedding=NULL WHERE brain_id=$1")
            .bind(cx.brain)
            .execute(&mut *tx)
            .await
            .unwrap();
    assert_eq!(
        changed.rows_affected(),
        0,
        "Reader access cannot modify the semantic projection"
    );
    tx.commit().await.unwrap();
    ok(
        &h,
        "DELETE",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        Value::Null,
    )
    .await;
    let mut tx = db::actor_tx(&h.state.pool, reader_id)
        .await
        .map_err(|e| e.1)
        .unwrap();
    let revoked: i64 =
        sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE brain_id=$1")
            .bind(cx.brain)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(revoked, 0);
    tx.rollback().await.unwrap();
    api.abort();
    provider.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_correction_raw_copy_reindex_and_historical_strict_recall_share_authority() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(true);
        policy["content_classes"]
            .as_array_mut()
            .unwrap()
            .push(json!("claim"));
    })
    .await;
    let old = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let independent = source(&h, &owner, &base, "Beryl.port = 9222\n").await;
    let original = accepted(&h, &owner, &base, &old, "Amber", "8080").await;
    let control = accepted(&h, &owner, &base, &independent, "Beryl", "9222").await;
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let known = chrono::Utc::now();
    let mut strict = query("Find the declared service port", &["semantic"]);
    strict["mode"] = json!("strict_accepted");
    let before = recall(&h, &owner, &base, strict.clone()).await;
    assert!(
        before["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["revision_id"] == original["id"])
    );
    let current = source(&h, &owner, &base, "Amber.port = 9090\n").await;
    let mut content = original["content"].clone();
    content["value"] = json!("9090");
    content["supports"] =
        json!([{"kind":"source_version","id":current["version"]["id"],"line_from":1,"line_to":1}]);
    let corrected=ok(&h,"POST",&format!("{base}/claims/{}/review",original["claim_id"].as_str().unwrap()),&owner,
        json!({"base_revision":original["id"],"action":"correct","reason":"The new source corrects the declared port.","content":content})).await["claims"][0]["revision"].clone();
    let inspection = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", original["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert!(
        inspection["history"]
            .as_array()
            .unwrap()
            .iter()
            .any(|revision| revision["id"] == original["id"]
                && revision["content"]["value"] == "8080"),
        "The prior assertion remains inspectable through canonical history"
    );
    let copied = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    for rebuild in [false, true] {
        if rebuild {
            let profile = status(&h, &owner, &base).await["profile"]["id"].clone();
            ok(
                &h,
                "POST",
                &format!("{base}/semantic/reindex"),
                &owner,
                json!({"base_profile":profile}),
            )
            .await;
            discover(&h, cx).await;
            while worker::run_once(&h.state, "model").await.unwrap() {}
        }
        let result = recall(
            &h,
            &owner,
            &base,
            query("Find service port evidence", &["semantic"]),
        )
        .await;
        let items = result["context"]["items"].as_array().unwrap();
        assert!(
            items.iter().any(|i| i["revision_id"] == corrected["id"]),
            "The correction remains useful after reindex={rebuild}"
        );
        assert!(items.iter().any(|i| i["id"] == control["claim_id"]));
        assert!(
            !items
                .iter()
                .any(|i| i["id"] == old["version"]["id"] || i["id"] == copied["version"]["id"])
        );
        assert!(!result["context"].to_string().contains("8080"));
        assert!(result["coverage"]["withheld"].as_u64().unwrap() >= 2);
        let mut current_strict = query("Find service port evidence", &["semantic"]);
        current_strict["mode"] = json!("strict_accepted");
        let result = recall(&h, &owner, &base, current_strict).await;
        assert!(
            result["context"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["revision_id"] == corrected["id"])
        );
        let retained:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM semantic_entries e JOIN semantic_heads h ON h.profile_id=e.profile_id AND h.brain_id=e.brain_id WHERE e.claim_revision_id=$1 AND e.state='ready' AND e.embedding IS NOT NULL)")
            .bind(original["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
        assert_eq!(
            retained, !rebuild,
            "The pre-rebuild history check must exercise an actually retained historical vector"
        );
        for mode in ["strict_accepted", "history"] {
            let mut historical = query("Find the declared service port", &["semantic"]);
            historical["mode"] = json!(mode);
            historical["knowledge_at"] = json!(known);
            let historical = recall(&h, &owner, &base, historical).await;
            let items = historical["context"]["items"].as_array().unwrap();
            assert!(items.iter().any(|i| i["id"] == control["claim_id"]));
            let old = items.iter().find(|i| i["revision_id"] == original["id"]);
            assert!(
                old.is_none(),
                "Model-facing recall applies current support/correction even to retained historical vectors"
            );
        }
    }
    strict["semantic_request_id"] = json!(Uuid::new_v4());
    strict["knowledge_at"] = json!(known);
    let historical = recall(&h, &owner, &base, strict).await;
    assert!(!historical["context"].to_string().contains("8080"));
    assert!(
        historical["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == control["claim_id"])
    );
    assert!(p.calls.load(Ordering::SeqCst) > 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_older_populated_restore_clears_vectors_and_pending_batches() {
    let (mut h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    let victim = source(&h, &owner, &base, "ERASED_SEMANTIC_CANARY first revision.").await;
    let control = source(
        &h,
        &owner,
        &base,
        "Independent surviving evidence for restore validation.",
    )
    .await;
    assert_eq!(discover(&h, cx).await, 1);
    model_job(&h).await;
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 2);
    ok(&h,"POST",&format!("{base}/sources/{}/versions",victim["id"].as_str().unwrap()),&owner,
        json!({"base_version":victim["version"]["id"],"title":"Pending victim revision","media_type":"text/plain","retain_content":true,"content":"ERASED_SEMANTIC_CANARY newer pending revision."})).await;
    assert_eq!(discover(&h, cx).await, 1);
    assert_eq!(status(&h, &owner, &base).await["counts"]["queued"], 1);
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable semantic restore fixture: {backup}");
    h.state.pool.close().await;
    h.admin.close().await;
    sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable semantic restore test created by crates/server/tests/platform/semantic_recovery.rs'"))
        .execute(&h.root).await.unwrap();
    let mut url = reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
    url.set_path(&h.database);
    h.admin = db::pool(url.as_str()).await.unwrap();
    h.state = AppState::new(
        db::pool(&h.state.config.database_url).await.unwrap(),
        (*h.state.config).clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    url.set_path(&backup);
    let backup_admin = db::pool(url.as_str()).await.unwrap();
    let mut config = (*h.state.config).clone();
    let mut app_url = reqwest::Url::parse(&config.database_url).unwrap();
    app_url.set_path(&backup);
    config.database_url = app_url.to_string();
    let restored = AppState::new(
        db::pool(&config.database_url).await.unwrap(),
        config.clone(),
    )
    .unwrap();
    let saved: i64 =
        sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE embedding IS NOT NULL")
            .fetch_one(&backup_admin)
            .await
            .unwrap();
    assert_eq!(
        saved, 2,
        "The older database really contains the old vectors"
    );
    erase(
        &h,
        &owner,
        &base,
        json!({"kind":"source","id":victim["id"]}),
    )
    .await;
    privacy_journal::run_once(&h.state).await.unwrap();
    assert!(
        privacy_journal::barrier(&restored.pool, &config)
            .await
            .is_err()
    );
    privacy_journal::reconcile(&backup_admin, &config)
        .await
        .unwrap();
    privacy_journal::barrier(&restored.pool, &config)
        .await
        .unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE embedding IS NOT NULL")
            .fetch_one(&backup_admin)
            .await
            .unwrap();
    assert_eq!(
        remaining, 1,
        "Journal replay clears an already-populated older vector projection"
    );
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM semantic_batches WHERE state IN ('queued','running')",
    )
    .fetch_one(&backup_admin)
    .await
    .unwrap();
    assert_eq!(
        pending, 0,
        "The older pending model batch cannot resurrect its input"
    );
    assert!(!worker::run_once(&restored, "model").await.unwrap());
    assert_eq!(
        semantic::maintain_brain(&restored, cx.brain, cx.actor)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Restore and reconciliation do not call a model"
    );
    let original_state = h.state.clone();
    h.state = restored.clone();
    h.router = app(restored.clone());
    let result = recall(
        &h,
        &owner,
        &base,
        query("Find the surviving evidence", &["semantic"]),
    )
    .await;
    assert_eq!(result["context"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["context"]["items"][0]["id"],
        control["version"]["id"]
    );
    assert!(!result.to_string().contains("ERASED_SEMANTIC_CANARY"));
    h.state = original_state;
    h.router = app(h.state.clone());
    restored.pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_policy_change_during_generation_discards_paid_output_and_preserves_controls() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    source(
        &h,
        &owner,
        &base,
        "An independent permitted vector remains ready.",
    )
    .await;
    discover(&h, cx).await;
    model_job(&h).await;
    p.entered.notified().await;
    source(
        &h,
        &owner,
        &base,
        "A pending representation will lose transmission permission.",
    )
    .await;
    discover(&h, cx).await;
    p.delay.store(250, Ordering::SeqCst);
    let state = h.state.clone();
    let task = tokio::spawn(async move { worker::run_once(&state, "model").await });
    p.entered.notified().await;
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(false);
    })
    .await;
    task.await.unwrap().unwrap();
    let failed = status(&h, &owner, &base).await;
    assert_eq!(failed["counts"]["ready"], 1);
    assert_eq!(failed["counts"]["failed"], 1);
    assert_eq!(failed["batches"][0]["error_code"], "model_policy_changed");
    assert!(failed["batches"][0]["request_id"].is_string());
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT suppressed FROM model_requests WHERE id=$1")
            .bind(
                failed["batches"][0]["request_id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap()
            )
            .fetch_one(&h.admin)
            .await
            .unwrap()
    );
    enable(&h, &owner, &base).await;
    assert_eq!(
        discover(&h, cx).await,
        0,
        "Changing policy cannot silently resend the charged batch"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    p.delay.store(0, Ordering::SeqCst);
    let proof = recall(
        &h,
        &owner,
        &base,
        query("An independent vector", &["semantic"]),
    )
    .await;
    assert_eq!(proof["context"]["items"].as_array().unwrap().len(), 1);
    let before = p.calls.load(Ordering::SeqCst);
    let (_, outsider) = h.fixture_member().await;
    let (code, _, _) = h
        .call(
            "POST",
            &format!("{base}/recall"),
            Some(&outsider),
            query("forbidden", &["semantic"]),
        )
        .await;
    assert_eq!(code, StatusCode::NOT_FOUND);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        before,
        "Denied recall is checked before transmission"
    );
    server.abort();
    h.finish().await;
}
