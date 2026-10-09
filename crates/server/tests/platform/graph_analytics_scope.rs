use super::*;

async fn source(h: &Harness, owner: &Login, base: &str, name: &str) -> Value {
    ok(h,"POST",&format!("{base}/sources"),owner,json!({"title":name,"media_type":"text/plain","content":"Synthetic retained evidence for analytics.\n","retain_content":true})).await
}
async fn epochs(h: &Harness, brain: Uuid) -> (i64, i64) {
    sqlx::query_as("SELECT coalesce((SELECT epoch FROM memory_epochs WHERE brain_id=$1),0),analytics_epoch FROM brains WHERE id=$1").bind(brain).fetch_one(&h.admin).await.unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_materialization_and_withheld_conflict_expiry_invalidate_complete_inputs() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let source_a = source(&h, &owner, &base, "Source A").await;
    let source_b = source(&h, &owner, &base, "Source B").await;
    // Retain an independent eligible control while Source A is incomplete.
    let (repo, snapshot, _) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/analytics/control",
        'a',
        fixture(&[&[]]),
    )
    .await;
    let fact = ok(
        &h,
        "GET",
        &format!(
            "{base}/repository-snapshots/{}/facts",
            snapshot.as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await["items"][0]["id"]
        .clone();
    let mut control = proposal(&fact, "Independent control", "available");
    control["content"]["selection"] =
        json!({"repository_ids":[repo],"area_ids":[],"environment_id":null});
    control["content"]["supports"] =
        json!([{"kind":"repository_fact","id":fact,"line_from":null,"line_to":null}]);
    let control = ok(&h, "POST", &format!("{base}/claims"), &owner, control).await;
    let control=ok(&h,"POST",&format!("{base}/claims/{}/review",control["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":control["id"],"action":"accept","reason":"Independent accepted control for strict analytical scope."})).await["claims"][0]["revision"].clone();
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(&source_a["version"]["id"], "Service port", "8080"),
    )
    .await;
    // Source-support-3 requires explicit authority for this manually authored
    // assertion. This fixture tests incomplete source materialization, not
    // whether an unassessed proposal can enter the knowledge projection.
    let claim = ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,
        json!({"base_revision":claim["id"],"action":"accept","reason":"Explicit fixture authority for retained but unprocessed source evidence."}))
        .await["claims"][0]["revision"].clone();
    build(&h, &owner, &base, "knowledge", None).await;
    let scope = json!({"kind":"knowledge","relations":["supported_by"]});
    let r = queue(&h, &owner, &base, &scope, "wcc", "both").await;
    let r = completed(&h, &owner, &base, &r).await;
    assert_eq!(
        r["total"], 3,
        "Qualified claims remain eligible while their unprocessed source vertex is withheld"
    );
    assert!(
        r["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["node"]["evidence"]["kind"] != "source_version")
    );
    let before = epochs(&h, brain).await;
    let one = worker::claim(&h.state.pool, "capture")
        .await
        .unwrap()
        .unwrap();
    let two = worker::claim(&h.state.pool, "capture")
        .await
        .unwrap()
        .unwrap();
    let results = tokio::join!(
        worker::execute(&h.state, &one),
        worker::execute(&h.state, &two)
    );
    results.0.unwrap();
    results.1.unwrap();
    let after = epochs(&h, brain).await;
    assert_eq!(
        before.0, after.0,
        "Materialization does not change autonomous memory scheduling"
    );
    assert!(after.1 > before.1);
    assert_eq!(
        read(&h, &owner, &base, &r["report"]["id"]).await["report"]["state"],
        "stale"
    );
    let fresh = queue(&h, &owner, &base, &scope, "wcc", "both").await;
    let fresh = completed(&h, &owner, &base, &fresh).await;
    assert_eq!(fresh["total"], 4);
    let accepted = claim.clone();
    let conflict = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(&source_b["version"]["id"], "Service port", "9090"),
    )
    .await;
    // Keep an explicitly reviewed contradictory peer eligible. Ordinary
    // acceptance refuses an overlapping conflict; review correction records
    // the deliberate fixture authority without bypassing production handlers.
    let conflict = ok(&h,"POST",&format!("{base}/claims/{}/review",conflict["claim_id"].as_str().unwrap()),&owner,
        json!({"base_revision":conflict["id"],"action":"revalidate","revalidation_basis":"review_correction","reason":"Retain the contradictory synthetic peer to test clock-based invalidation."}))
        .await["claims"][0]["revision"].clone();
    let settings = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = settings["policy"].clone();
    policy["claim_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":settings["change_id"],"policy":policy}),
    )
    .await;
    build(&h, &owner, &base, "knowledge", None).await;
    sqlx::query("UPDATE claim_revisions SET recorded_at=clock_timestamp()-interval '1 day'+interval '4 seconds' WHERE id=$1")
      .bind(conflict["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
    let strict = json!({"kind":"knowledge","relations":["supported_by"],"mode":"strict_accepted"});
    let r = queue(&h, &owner, &base, &strict, "wcc", "both").await;
    let r = completed(&h, &owner, &base, &r).await;
    assert!(
        r["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["node"]["evidence"]["revision_id"] == control["id"])
    );
    assert!(
        r["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["node"]["evidence"]["revision_id"] != accepted["id"])
    );
    let before = epochs(&h, brain).await;
    tokio::time::sleep(std::time::Duration::from_millis(4100)).await;
    assert_eq!(
        before,
        epochs(&h, brain).await,
        "Clock expiry is not a database mutation"
    );
    assert_eq!(
        read(&h, &owner, &base, &r["report"]["id"]).await["report"]["state"],
        "stale"
    );
    let current = queue(&h, &owner, &base, &strict, "wcc", "both").await;
    let current = completed(&h, &owner, &base, &current).await;
    assert!(
        current["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["node"]["evidence"]["revision_id"] == accepted["id"]),
        "The expired withheld peer now admits the accepted claim"
    );
    assert!(current["total"].as_i64().unwrap() > r["total"].as_i64().unwrap());
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_large_input_and_native_paired_scope_are_bounded() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, _) = combined::publish(
        &h,
        &owner,
        &base,
        "example.test/analytics/large",
        'f',
        fixture(&[&[]]),
    )
    .await;
    let snapshot_id: Uuid = snapshot.as_str().unwrap().parse().unwrap();
    sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) SELECT gen_random_uuid(),$1,$2,n,jsonb_build_object('id','bulk-'||n,'kind','function','name','bulk_'||n,'file','main.rs','line',1) FROM generate_series(1,6000) n")
      .bind(brain).bind(snapshot_id).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE repository_snapshots SET fact_count=6001 WHERE id=$1")
        .bind(snapshot_id)
        .execute(&h.admin)
        .await
        .unwrap();
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let scope = json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo],"area_ids":[],"environment_id":null},"relations":["calls"]});
    let refusal = h
        .call(
            "POST",
            &format!("{base}/graph/view"),
            Some(&owner),
            json!({"scope":scope}),
        )
        .await;
    assert_eq!(refusal.1["code"], "graph_scope_too_large");
    let started = std::time::Instant::now();
    let r = queue(&h, &owner, &base, &scope, "wcc", "both").await;
    let r = completed(&h, &owner, &base, &r).await;
    assert_eq!(r["total"], 6001);
    assert_eq!(r["rows"].as_array().unwrap().len(), 100);
    eprintln!(
        "Bounded 6001-vertex analysis including queue/compute/read: {:?}",
        started.elapsed()
    );
    // Every numeric score can depend on contributors absent from this page.
    let (member, member_session) = h.fixture_member().await;
    assert_eq!(
        h.call(
            "POST",
            &format!(
                "{base}/graph/analytics/{}/view",
                r["report"]["id"].as_str().unwrap()
            ),
            Some(&member_session),
            json!({"offset":0})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{member}"),
        &owner,
        json!({"role":"writer"}),
    )
    .await;
    let (_, token) = h
        .pair_device(&member_session, "Analytical scope proof")
        .await;
    let task = h
        .bearer(
            "POST",
            &format!("{base}/workspace/tasks"),
            &token,
            json!({"label":"Analyze exact snapshot","selection":scope["selection"]}),
        )
        .await;
    assert_eq!(task.0, StatusCode::OK, "{task:?}");
    let task_id = task.1["task"]["id"].as_str().unwrap();
    let operation = h
        .bearer(
            "POST",
            &format!("{base}/workspace/tasks/{task_id}/operations"),
            &token,
            json!({"kind":"retrieval"}),
        )
        .await;
    assert_eq!(operation.0, StatusCode::OK, "{operation:?}");
    let read_url = format!(
        "{base}/graph/analytics/{}/view",
        r["report"]["id"].as_str().unwrap()
    );
    assert_eq!(
        h.bearer("POST", &read_url, &token, json!({"offset":0}))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let view = h
        .bearer(
            "POST",
            &read_url,
            &token,
            json!({"offset":100,"operation_id":operation.1["id"]}),
        )
        .await;
    assert_eq!(view.0, StatusCode::OK, "{view:?}");
    assert_eq!(view.1["total"], 6001);
    assert_eq!(view.1["rows"].as_array().unwrap().len(), 100);
    let mut paired_scope = scope.clone();
    paired_scope["operation_id"] = operation.1["id"].clone();
    let queued = h
        .bearer(
            "POST",
            &format!("{base}/graph/analytics"),
            &token,
            json!({"scope":paired_scope,"algorithm":"wcc","direction":"both"}),
        )
        .await;
    assert_eq!(queued.0, StatusCode::OK, "{queued:?}");
    let queued = completed(&h, &owner, &base, &queued.1).await;
    assert_eq!(queued["total"], 6001);
    // Exceed the candidate bound without changing the endpoint into a prefix.
    sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) SELECT gen_random_uuid(),$1,$2,n,jsonb_build_object('id','bulk-'||n,'kind','function','name','bulk_'||n,'file','main.rs','line',1) FROM generate_series(6001,10000) n")
      .bind(brain).bind(snapshot_id).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE repository_snapshots SET fact_count=10001 WHERE id=$1")
        .bind(snapshot_id)
        .execute(&h.admin)
        .await
        .unwrap();
    build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    let refusal = h
        .call(
            "POST",
            &format!("{base}/graph/analytics"),
            Some(&owner),
            json!({"scope":scope,"algorithm":"wcc","direction":"both"}),
        )
        .await;
    assert_eq!(
        refusal.1["code"], "analytics_scope_too_large",
        "{refusal:?}"
    );
    assert_eq!(
        read(&h, &owner, &base, &r["report"]["id"]).await["rows"],
        json!([])
    );
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    h.finish().await;
}
