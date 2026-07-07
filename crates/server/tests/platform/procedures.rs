use super::*;

fn procedure(version: &Value, title: &str, outcome: Option<&str>) -> Value {
    let mut input = review::proposal(
        version,
        title,
        "Inspect the synthetic service and record its result.",
    );
    let content = &mut input["content"];
    content["kind"] = json!("procedure");
    content["operational"] = json!("verified");
    content["observed_at"] = json!("2026-09-14T12:00:00Z");
    content["observation"] = json!("The contributor recorded a synthetic trial.");
    content["procedure"] = json!({
        "conditions":"Use the isolated synthetic service.",
        "steps":["Inspect the service status.", "Record the observed port."],
        "expected_outcome":"The service reports port 8080.",
        "observations":outcome.map(|result|vec![json!({"result":result,"observed_at":"2026-09-14T12:00:00Z",
            "conditions":"Isolated local fixture.","summary":"The fixture reported the recorded result.","support_ids":[version]})]).unwrap_or_default()
    });
    input
}
async fn accept(h: &Harness, owner: &Login, base: &str, r: &Value) -> Value {
    ok(h,"POST",&format!("{base}/claims/{}/review",r["claim_id"].as_str().unwrap()),owner,
        json!({"base_revision":r["id"],"action":"accept","reason":"Checked the synthetic source and attributed observations.","content":null,"revalidation_basis":null})).await["claims"][0]["revision"].clone()
}
async fn detail(h: &Harness, owner: &Login, base: &str, r: &Value) -> Value {
    ok(
        h,
        "GET",
        &format!("{base}/claims/{}", r["claim_id"].as_str().unwrap()),
        owner,
        Value::Null,
    )
    .await
}
fn handover(rows: &[Value], title: &str) -> Value {
    let mut content = rows[0]["content"].clone();
    content["kind"] = json!("handover");
    content["subject"] = json!(title);
    content["predicate"] = json!("handover");
    content["value"] = json!("The linked records summarize the synthetic investigation.");
    content["operational"] = json!("declared");
    content["observed_at"] = Value::Null;
    content["observation"] = json!("");
    content["procedure"] = Value::Null;
    let mut repositories = vec![];
    let mut areas = vec![];
    let mut supports = vec![];
    for row in rows {
        repositories.extend(
            row["content"]["selection"]["repository_ids"]
                .as_array()
                .unwrap()
                .clone(),
        );
        areas.extend(
            row["content"]["selection"]["area_ids"]
                .as_array()
                .unwrap()
                .clone(),
        );
        for support in row["content"]["supports"].as_array().unwrap() {
            if !supports.contains(support) {
                supports.push(support.clone());
            }
        }
    }
    repositories.sort_by_key(Value::to_string);
    repositories.dedup();
    areas.sort_by_key(Value::to_string);
    areas.dedup();
    content["selection"]["repository_ids"] = json!(repositories);
    content["selection"]["area_ids"] = json!(areas);
    content["supports"] = json!(supports);
    content["handover"] = json!({"completed":["Recorded the synthetic fixture."],"next_steps":["Inspect the linked evidence."],"risks":["This is local synthetic evidence."],"contributions":rows.iter().map(|r|r["id"].clone()).collect::<Vec<_>>()});
    json!({"base_revision":null,"operation_id":null,"content":content})
}
async fn generated_runs(h: &Harness, owner: &Login, base: &str) -> Value {
    ok(h, "GET", &format!("{base}/handovers"), owner, Value::Null).await
}
fn generated_input(r: &Value) -> Value {
    json!({"title":"Synthetic engineering handover","contributions":[r["id"]],"operation_id":null})
}
async fn erase(h: &Harness, owner: &Login, base: &str, target: Value) -> Value {
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
    preview
}
fn draft() -> Value {
    json!({"summary":"The synthetic source records the selected service port.","completed":["Recorded the configuration declaration."],
        "next_steps":["Verify the service before changing deployment."],"risks":["The declaration alone does not establish runtime health."]})
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn procedures_and_handovers_preserve_outcomes_scope_review_and_erasure() {
    let (h, owner, base, p, server) = setup().await;
    let source = source(
        &h,
        &owner,
        &base,
        "The synthetic fixture reported its expected port.\n",
    )
    .await;
    let version = &source["version"]["id"];
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let environment = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Synthetic test"}),
    )
    .await["id"]
        .clone();
    let mut rows = vec![];
    for (index, outcome) in [Some("success"), Some("failure"), None]
        .into_iter()
        .enumerate()
    {
        let repo = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,$3,$4)",
        )
        .bind(repo)
        .bind(brain)
        .bind(format!("example.test/fixture/repo{index}"))
        .bind(actor)
        .execute(&h.admin)
        .await
        .unwrap();
        let mut input = procedure(version, &format!("Procedure {index}"), outcome);
        input["content"]["selection"] =
            json!({"repository_ids":[repo],"area_ids":[],"environment_id":environment});
        let r = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
        let r = accept(&h, &owner, &base, &r).await;
        let view = detail(&h, &owner, &base, &r).await;
        assert_eq!(view["selected"]["eligibility"]["strict_accepted"], true);
        assert_eq!(
            view["selected"]["eligibility"]["strict_operational"],
            index == 0
        );
        rows.push(r);
    }
    let input = handover(&rows[..2], "Multi-repository handover");
    let mut wrong = input.clone();
    wrong["content"]["selection"]["repository_ids"] = json!([]);
    assert!(
        h.call("POST", &format!("{base}/claims"), Some(&owner), wrong)
            .await
            .0
            .is_client_error()
    );
    let r = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
    let r = accept(&h, &owner, &base, &r).await;
    let view = detail(&h, &owner, &base, &r).await;
    assert_eq!(
        view["selected"]["contributions"].as_array().unwrap().len(),
        2
    );
    assert_eq!(view["selected"]["eligibility"]["strict_accepted"], true);
    assert_eq!(view["selected"]["eligibility"]["strict_operational"], false);
    let only = ok(
        &h,
        "GET",
        &format!("{base}/claims?kind=procedure"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(only["items"].as_array().unwrap().len(), 3);
    let mut recursive = handover(std::slice::from_ref(&r), "Recursive handover");
    recursive["content"]["procedure"] = Value::Null;
    assert!(
        h.call("POST", &format!("{base}/claims"), Some(&owner), recursive)
            .await
            .0
            .is_client_error()
    );
    let (_, foreign) = h.fixture_member().await;
    assert_eq!(
        h.call(
            "GET",
            &format!("{base}/claims/{}", r["claim_id"].as_str().unwrap()),
            Some(&foreign),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let withdrawn = ok(&h,"POST",&format!("{base}/claims/{}/review",rows[0]["claim_id"].as_str().unwrap()),&owner,
        json!({"base_revision":rows[0]["id"],"action":"withdraw","reason":"The synthetic procedure is no longer current.","content":null,"revalidation_basis":null})).await;
    assert!(!withdrawn["claims"].as_array().unwrap().is_empty());
    let view = detail(&h, &owner, &base, &r).await;
    assert_eq!(view["selected"]["eligibility"]["investigation"], false);
    let preview = erase(
        &h,
        &owner,
        &base,
        json!({"kind":"claim","id":rows[0]["claim_id"]}),
    )
    .await;
    assert!(preview["claim_revisions"].as_i64().unwrap() >= 4);
    assert_eq!(
        detail(&h, &owner, &base, &r).await["selection_state"],
        "erased"
    );
    assert_eq!(
        detail(&h, &owner, &base, &rows[2]).await["selected"]["revision"]["content"]["subject"],
        "Procedure 2"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn handover_generation_policy_replay_and_stale_input() {
    let (mut h, owner, base, p, server) = setup().await;
    let source = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let r = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&source["version"]["id"], "Amber", "8080"),
    )
    .await;
    let input = generated_input(&r);
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/handovers"),
            Some(&owner),
            input.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["synthesis"]);
        policy["content_classes"] = json!(["claim", "query"]);
    })
    .await;
    *p.candidates.lock().unwrap() = draft();
    let key = Uuid::new_v4().to_string();
    let queued = h
        .keyed(
            "POST",
            &format!("{base}/handovers"),
            Some(&owner),
            input.clone(),
            Some(&key),
        )
        .await;
    assert_eq!(queued.0, StatusCode::OK, "{}", queued.1);
    let replay = h
        .keyed(
            "POST",
            &format!("{base}/handovers"),
            Some(&owner),
            input.clone(),
            Some(&key),
        )
        .await;
    assert_eq!(queued.1, replay.1);
    h.state = AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap();
    h.router = app(h.state.clone());
    model_job(&h).await;
    let run = generated_runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(run["state"], "succeeded", "{run}");
    let detail = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", run["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        detail["selected"]["revision"]["origin"],
        "model_synthesized"
    );
    assert_eq!(detail["selected"]["revision"]["review"], "proposed");
    assert_eq!(detail["selected"]["revision"]["reviewer_id"], Value::Null);
    assert_eq!(
        detail["selected"]["eligibility"]["effective_freshness"],
        "current"
    );
    assert_eq!(detail["selected"]["eligibility"]["strict_accepted"], false);
    assert_eq!(
        detail["selected"]["contributions"][0]["revision_id"],
        r["id"]
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    let replay = h
        .keyed(
            "POST",
            &format!("{base}/handovers"),
            Some(&owner),
            input.clone(),
            Some(&key),
        )
        .await;
    assert_eq!(replay.1, queued.1);
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM claims")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    p.delay.store(250, Ordering::SeqCst);
    ok(
        &h,
        "POST",
        &format!("{base}/handovers"),
        &owner,
        input.clone(),
    )
    .await;
    let state = h.state.clone();
    let work = tokio::spawn(async move { worker::run_once(&state, "model").await.unwrap() });
    wait_calls(&p, 2).await;
    let mut changed = review::proposal(&source["version"]["id"], "Amber", "9090");
    changed["base_revision"] = r["id"].clone();
    ok(
        &h,
        "PUT",
        &format!("{base}/claims/{}", r["claim_id"].as_str().unwrap()),
        &owner,
        changed,
    )
    .await;
    assert!(work.await.unwrap());
    let failed = generated_runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(failed["state"], "failed");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM claims")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        before
    );
    assert!(
        h.call(
            "POST",
            &format!("{base}/handovers/{}/retry", failed["id"].as_str().unwrap()),
            Some(&owner),
            json!({"operation_id":null})
        )
        .await
        .0
        .is_client_error()
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn handover_erasure_replays_into_pre_handover_database() {
    use recollect_server::privacy_journal;
    let (mut h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["synthesis"]);
        policy["content_classes"] = json!(["claim", "query"]);
    })
    .await;
    let source_a = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let source_b = source(&h, &owner, &base, "Birch.port = 9090\n").await;
    let r = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&source_a["version"]["id"], "Amber", "8080"),
    )
    .await;
    let unrelated = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&source_b["version"]["id"], "Birch", "9090"),
    )
    .await;
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable handover restore fixture: {backup}");
    h.state.pool.close().await;
    h.admin.close().await;
    sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'")).execute(&h.root).await.unwrap();
    let mut admin_url = reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
    admin_url.set_path(&h.database);
    h.admin = db::pool(admin_url.as_str()).await.unwrap();
    h.state = AppState::new(
        db::pool(&h.state.config.database_url).await.unwrap(),
        (*h.state.config).clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    admin_url.set_path(&backup);
    let backup_admin = db::pool(admin_url.as_str()).await.unwrap();
    let mut backup_config = (*h.state.config).clone();
    let mut url = reqwest::Url::parse(&backup_config.database_url).unwrap();
    url.set_path(&backup);
    backup_config.database_url = url.to_string();
    let backup_state = AppState::new(
        db::pool(&backup_config.database_url).await.unwrap(),
        backup_config.clone(),
    )
    .unwrap();
    let record = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        handover(std::slice::from_ref(&r), "Erased synthetic handover"),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/handovers"),
        &owner,
        generated_input(&r),
    )
    .await;
    let preview = erase(
        &h,
        &owner,
        &base,
        json!({"kind":"claim","id":record["claim_id"]}),
    )
    .await;
    assert_eq!(preview["model_claim_fences"], 1);
    assert_eq!(preview["source_versions"], 0);
    assert_eq!(preview["jobs"], 1);
    assert_eq!(
        generated_runs(&h, &owner, &base).await["items"][0]["state"],
        "removed"
    );
    assert_eq!(
        detail(&h, &owner, &base, &r).await["selected"]["revision"]["content"]["subject"],
        "Amber"
    );
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/handovers"),
            Some(&owner),
            generated_input(&r)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    privacy_journal::run_once(&h.state).await.unwrap();
    assert!(
        privacy_journal::barrier(&backup_state.pool, &backup_config)
            .await
            .is_err()
    );
    privacy_journal::reconcile(&backup_admin, &backup_config)
        .await
        .unwrap();
    privacy_journal::barrier(&backup_state.pool, &backup_config)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM claims")
            .fetch_one(&backup_admin)
            .await
            .unwrap(),
        2
    );
    let live = h.state.clone();
    h.state = backup_state.clone();
    h.router = app(h.state.clone());
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/handovers"),
            Some(&owner),
            generated_input(&r)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    *p.candidates.lock().unwrap() = draft();
    ok(
        &h,
        "POST",
        &format!("{base}/handovers"),
        &owner,
        generated_input(&unrelated),
    )
    .await;
    model_job(&h).await;
    let run = generated_runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(run["state"], "succeeded");
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    // Seven-day maintenance must not delete a job still referenced by durable history.
    sqlx::query("UPDATE jobs SET updated_at=clock_timestamp()-interval '8 days' WHERE id IN(SELECT job_id FROM handover_runs)")
        .execute(&backup_admin).await.unwrap();
    assert!(worker::run_once(&h.state, "interactive").await.unwrap());
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM jobs WHERE id IN(SELECT job_id FROM handover_runs)"
        )
        .fetch_one(&backup_admin)
        .await
        .unwrap(),
        1
    );
    sqlx::query("UPDATE handover_runs SET created_at=clock_timestamp()-interval '400 days'")
        .execute(&backup_admin)
        .await
        .unwrap();
    privacy_journal::run_once(&h.state).await.unwrap();
    assert_eq!(
        generated_runs(&h, &owner, &base).await["items"][0]["title"],
        ""
    );
    h.state = live;
    h.router = app(h.state.clone());
    backup_state.pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn handover_failure_explicit_retry_and_publication_rollback() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["synthesis"]);
        policy["content_classes"] = json!(["claim", "query"]);
    })
    .await;
    let source = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let r = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        review::proposal(&source["version"]["id"], "Amber", "8080"),
    )
    .await;
    p.mode.store(1, Ordering::SeqCst);
    ok(
        &h,
        "POST",
        &format!("{base}/handovers"),
        &owner,
        generated_input(&r),
    )
    .await;
    model_job(&h).await;
    let failed = generated_runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(failed["state"], "failed");
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    assert!(!worker::run_once(&h.state, "model").await.unwrap());
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/jobs/{}/retry", failed["job_id"].as_str().unwrap()),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    p.mode.store(0, Ordering::SeqCst);
    *p.candidates.lock().unwrap() = draft();
    ok(
        &h,
        "POST",
        &format!("{base}/handovers/{}/retry", failed["id"].as_str().unwrap()),
        &owner,
        json!({"operation_id":null}),
    )
    .await;
    model_job(&h).await;
    assert_eq!(
        generated_runs(&h, &owner, &base).await["items"][0]["state"],
        "succeeded"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM claims")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE mutation_audit ADD CONSTRAINT handover_fault CHECK(action <> 'handover.publish') NOT VALID").execute(&h.admin).await.unwrap();
    ok(
        &h,
        "POST",
        &format!("{base}/handovers"),
        &owner,
        generated_input(&r),
    )
    .await;
    model_job(&h).await;
    assert_eq!(
        generated_runs(&h, &owner, &base).await["items"][0]["state"],
        "failed"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM claims")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        before
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    sqlx::query("ALTER TABLE mutation_audit DROP CONSTRAINT handover_fault")
        .execute(&h.admin)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn handover_device_scope_foreign_environment_revocation_and_expiry() {
    use recollect_server::privacy_journal;
    let (h, owner, base, p, server) = setup().await;
    let (writer_id, writer) = h.fixture_member().await;
    let (reader_id, reader) = h.fixture_member().await;
    for (id, role) in [(writer_id, "writer"), (reader_id, "reader")] {
        ok(
            &h,
            "PUT",
            &format!("{base}/grants/{id}"),
            &owner,
            json!({"role":role}),
        )
        .await;
    }
    allow(&h, &owner, &base, |policy| {
        policy["purposes"] = json!(["synthesis"]);
        policy["content_classes"] = json!(["claim", "query"]);
    })
    .await;
    let source = source(&h, &writer, &base, "Amber.port = 8080\n").await;
    let r = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &writer,
        review::proposal(&source["version"]["id"], "Amber", "8080"),
    )
    .await;
    let mut input = generated_input(&r);
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/handovers"),
            Some(&reader),
            input.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, token) = h.pair_device(&writer, "Synthetic handover device").await;
    assert_eq!(
        h.bearer("POST", &format!("{base}/handovers"), &token, input.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let task = h
        .bearer(
            "POST",
            &format!("{base}/workspace/tasks"),
            &token,
            json!({"label":"Scoped handover","selection":r["content"]["selection"]}),
        )
        .await;
    assert_eq!(task.0, StatusCode::OK);
    let op = h
        .bearer(
            "POST",
            &format!(
                "{base}/workspace/tasks/{}/operations",
                task.1["task"]["id"].as_str().unwrap()
            ),
            &token,
            json!({"kind":"write"}),
        )
        .await;
    assert_eq!(op.0, StatusCode::OK);
    input["operation_id"] = op.1["id"].clone();
    *p.candidates.lock().unwrap() = draft();
    assert_eq!(
        h.bearer("POST", &format!("{base}/handovers"), &token, input.clone())
            .await
            .0,
        StatusCode::OK
    );
    model_job(&h).await;
    let completed = generated_runs(&h, &owner, &base).await["items"][0].clone();
    assert_eq!(completed["state"], "succeeded");
    let generated = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", completed["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await["selected"]["revision"]
        .clone();
    assert_eq!(generated["device_id"], completed["device_id"]);
    assert!(!generated["device_id"].is_null());
    let other = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Foreign handover evidence"}),
    )
    .await;
    let other_base = format!("/api/brains/{}", other["id"].as_str().unwrap());
    let other_source = super::source(&h, &owner, &other_base, "Foreign.port = 7777\n").await;
    let foreign = ok(
        &h,
        "POST",
        &format!("{other_base}/claims"),
        &owner,
        review::proposal(&other_source["version"]["id"], "Foreign", "7777"),
    )
    .await;
    assert!(
        h.call(
            "POST",
            &format!("{base}/handovers"),
            Some(&owner),
            generated_input(&foreign)
        )
        .await
        .0
        .is_client_error()
    );
    let env = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Different test environment"}),
    )
    .await["id"]
        .clone();
    let mut different = review::proposal(&source["version"]["id"], "Different environment", "8080");
    different["content"]["selection"]["environment_id"] = env;
    let different = ok(&h, "POST", &format!("{base}/claims"), &owner, different).await;
    assert!(h.call("POST",&format!("{base}/handovers"),Some(&owner),json!({"title":"Wrong combination","contributions":[r["id"],different["id"]],"operation_id":null})).await.0.is_client_error());
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    p.delay.store(250, Ordering::SeqCst);
    assert_eq!(
        h.bearer("POST", &format!("{base}/handovers"), &token, input)
            .await
            .0,
        StatusCode::OK
    );
    let state = h.state.clone();
    let work = tokio::spawn(async move { worker::run_once(&state, "model").await.unwrap() });
    wait_calls(&p, 2).await;
    ok(
        &h,
        "DELETE",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert!(work.await.unwrap());
    assert_eq!(
        generated_runs(&h, &owner, &base).await["items"][0]["state"],
        "failed"
    );
    let policy = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut new_policy = policy["policy"].clone();
    new_policy["claim_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":policy["change_id"],"policy":new_policy}),
    )
    .await;
    sqlx::query(
        "UPDATE claim_revisions SET recorded_at=clock_timestamp()-interval '2 days' WHERE id=$1",
    )
    .bind(r["id"].as_str().unwrap().parse::<Uuid>().unwrap())
    .execute(&h.admin)
    .await
    .unwrap();
    let view = detail(&h, &owner, &base, &generated).await;
    assert_eq!(view["selected"]["contributions"][0]["state"], "expired");
    assert_eq!(
        view["selected"]["contributions"][0]["revision"],
        Value::Null
    );
    assert_eq!(view["selected"]["eligibility"]["strict_accepted"], false);
    assert_eq!(
        view["selected"]["eligibility"]["effective_freshness"],
        "needs_verification"
    );
    assert!(
        h.call(
            "POST",
            &format!("{base}/handovers"),
            Some(&owner),
            generated_input(&r)
        )
        .await
        .0
        .is_client_error()
    );
    privacy_journal::run_once(&h.state).await.unwrap();
    let history = generated_runs(&h, &owner, &base).await;
    let preserved = history["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["id"] == completed["id"])
        .unwrap();
    assert_eq!(preserved["state"], "succeeded");
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}
