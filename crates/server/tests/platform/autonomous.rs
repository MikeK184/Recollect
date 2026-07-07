use super::*;
use recollect_server::autonomous;
use recollect_server::privacy_journal;

async fn enable(h: &Harness, owner: &Login, base: &str) {
    allow(h, owner, base, |p| {
        p["autonomous_memory"] = json!(true);
        p["purposes"] = json!(["extraction", "synthesis"]);
        p["content_classes"] = json!(["document", "claim", "query"]);
    })
    .await;
}
fn extracted(value: &str, replaces: Value) -> Value {
    let mut c = candidate("Amber", "port", value, 1);
    c["replaces_revision"] = replaces;
    json!({"claims":[c],"retirements":[]})
}
fn handover(value: &str) -> Value {
    json!({"summary":format!("Amber declares port {value}."),"completed":["Read the supplied configuration."],"next_steps":["Confirm runtime if needed."],"risks":["Declared configuration is not runtime proof."]})
}
async fn detail(h: &Harness, owner: &Login, base: &str, id: &Value) -> Value {
    ok(
        h,
        "GET",
        &format!("{base}/claims/{}", id.as_str().unwrap()),
        owner,
        Value::Null,
    )
    .await
}
async fn change(h: &Harness, owner: &Login, base: &str, source: &Value, text: &str) -> Value {
    ok(h,"POST",&format!("{base}/sources/{}/versions",source["id"].as_str().unwrap()),owner,
       json!({"base_version":source["version"]["id"],"title":"Updated synthetic evidence","media_type":"text/plain","retain_content":true,"content":text})).await
}
async fn process(h: &Harness) {
    while worker::run_once(&h.state, "capture").await.unwrap() {}
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn autonomous_maintenance_at_identity_capacity_and_atomic_new_claim_failure() {
    let (h, owner, base, p, server) = setup().await;
    enable(&h, &owner, &base).await;
    let original = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    *p.candidates.lock().unwrap() = extracted("8080", Value::Null);
    learn(&h, &owner, &base, &original).await;
    model_job(&h).await;
    let initial = runs(&h, &owner, &base).await;
    assert_eq!(initial["items"][0]["state"], "succeeded", "{initial}");
    let claim = initial["items"][0]["claim_ids"][0].clone();
    let first = detail(&h, &owner, &base, &claim).await["selected"]["revision"].clone();

    // Populate the capacity boundary with valid, independently identified
    // revisions/support rows. A separate source keeps these fixtures out of
    // Amber's reconciliation lineage; no thousands of model calls are needed.
    let filler = source(&h, &owner, &base, "Independent capacity fixtures.\n").await;
    let mut template = first.clone();
    template["origin"] = json!("browser_authored");
    template["review"] = json!("proposed");
    template["admission"] = json!("proposed");
    template["acceptance_policy"] = Value::Null;
    template["derivation"] = Value::Null;
    template["content"]["supports"][0]["id"] = filler["version"]["id"].clone();
    let brain = Uuid::parse_str(base.rsplit('/').next().unwrap()).unwrap();
    let actor = Uuid::parse_str(first["actor_id"].as_str().unwrap()).unwrap();
    sqlx::query("WITH fixtures AS MATERIALIZED (
        SELECT gen_random_uuid() claim,gen_random_uuid() revision,n FROM generate_series(1,4999) n
      ), identities AS (
        INSERT INTO claims(id,brain_id,created_by,current_revision)
        SELECT claim,$1,$2,revision FROM fixtures RETURNING id
      ), revisions AS (
        INSERT INTO claim_revisions(id,claim_id,brain_id,recorded_at,revision,subject_key,predicate_key,value_key)
        SELECT f.revision,f.claim,$1,($3->>'recorded_at')::timestamptz,
          jsonb_set(jsonb_set(jsonb_set($3,'{id}',to_jsonb(f.revision)),
            '{claim_id}',to_jsonb(f.claim)),'{content,subject}',to_jsonb('Fixture '||f.n)),
          'fixture '||f.n,'port','8080'
        FROM fixtures f JOIN identities i ON i.id=f.claim RETURNING id,brain_id
      ) INSERT INTO claim_supports(revision_id,brain_id,ordinal,source_version_id)
        SELECT id,brain_id,0,$4 FROM revisions")
        .bind(brain).bind(actor).bind(sqlx::types::Json(&template))
        .bind(Uuid::parse_str(filler["version"]["id"].as_str().unwrap()).unwrap())
        .execute(&h.admin).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM claims WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(count, 5000);

    let updated = change(
        &h,
        &owner,
        &base,
        &original,
        "Amber.port = 9090; replaces 8080.\n",
    )
    .await;
    *p.candidates.lock().unwrap() = extracted("9090", first["id"].clone());
    learn(&h, &owner, &base, &updated).await;
    model_job(&h).await;
    let revised = runs(&h, &owner, &base).await;
    assert_eq!(revised["items"][0]["state"], "succeeded", "{revised}");
    assert_eq!(revised["items"][0]["revised"], 1);
    assert_eq!(revised["items"][0]["claim_ids"][0], claim);
    let current = detail(&h, &owner, &base, &claim).await["selected"]["revision"].clone();
    assert_eq!(current["content"]["value"], "9090");

    *p.candidates.lock().unwrap() = extracted("9090", Value::Null);
    learn(&h, &owner, &base, &updated).await;
    model_job(&h).await;
    let reused = runs(&h, &owner, &base).await;
    assert_eq!(reused["items"][0]["state"], "succeeded", "{reused}");
    assert_eq!(reused["items"][0]["reused"], 1);

    let changed = change(
        &h,
        &owner,
        &base,
        &updated,
        "Amber.port = 7070; replaces 9090.\nQuartz.port = 5050\n",
    )
    .await;
    let mut mixed = extracted("7070", current["id"].clone());
    let mut new_claim = candidate("Quartz", "port", "5050", 2);
    new_claim["replaces_revision"] = Value::Null;
    mixed["claims"]
        .as_array_mut()
        .unwrap()
        .push(new_claim.clone());
    *p.candidates.lock().unwrap() = mixed;
    learn(&h, &owner, &base, &changed).await;
    model_job(&h).await;
    let failed = runs(&h, &owner, &base).await;
    assert_eq!(failed["items"][0]["state"], "failed", "{failed}");
    assert_eq!(
        detail(&h, &owner, &base, &claim).await["selected"]["revision"]["id"],
        current["id"],
        "A later capacity failure must roll back the earlier replacement"
    );
    *p.candidates.lock().unwrap() = json!({"claims":[new_claim],"retirements":[]});
    learn(&h, &owner, &base, &changed).await;
    model_job(&h).await;
    assert_eq!(runs(&h, &owner, &base).await["items"][0]["state"], "failed");

    let removed = change(
        &h,
        &owner,
        &base,
        &changed,
        "Amber's listener was removed; port 9090 is obsolete.\n",
    )
    .await;
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[{
        "revision_id":current["id"],"reason":"The source explicitly removes the listener.","line_from":1,"line_to":1}]});
    learn(&h, &owner, &base, &removed).await;
    model_job(&h).await;
    let retired = runs(&h, &owner, &base).await;
    assert_eq!(retired["items"][0]["state"], "succeeded", "{retired}");
    assert_eq!(retired["items"][0]["retired"], 1);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM claims WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(count, 5000);
    let revisions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM claim_revisions WHERE claim_id=$1")
            .bind(Uuid::parse_str(claim.as_str().unwrap()).unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        revisions, 3,
        "Only original, replacement and retirement committed"
    );
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_audit WHERE brain_id=$1 AND action='claim.learn'",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        audits, 2,
        "The mixed batch must not retain its rolled-back claim audit"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn autonomous_catchup_revision_handover_retirement_and_erasure() {
    let (h, owner, base, p, server) = setup().await;
    let original = source(
        &h,
        &owner,
        &base,
        "Amber listens on port 8080 in the declared configuration.\n",
    )
    .await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    enable(&h, &owner, &base).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );
    *p.candidates.lock().unwrap() = extracted("8080", Value::Null);
    let restarted = AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap();
    assert!(worker::run_once(&restarted, "model").await.unwrap());
    let learned = runs(&h, &owner, &base).await;
    assert_eq!(learned["items"][0]["state"], "succeeded", "{learned}");
    assert_eq!(learned["items"][0]["accepted"], 1);
    let claim = learned["items"][0]["claim_ids"][0].clone();
    let before = detail(&h, &owner, &base, &claim).await;
    let first = &before["selected"]["revision"];
    assert_eq!(first["review"], "accepted");
    assert!(first["reviewer_id"].is_null());
    assert!(first["review_decision_id"].is_null());
    assert_eq!(before["selected"]["eligibility"]["strict_accepted"], true);
    assert_eq!(
        before["selected"]["eligibility"]["strict_operational"],
        false
    );
    assert!(
        first["acceptance_policy"]
            .as_str()
            .unwrap()
            .starts_with("autonomous-evidence@")
    );
    *p.candidates.lock().unwrap() = handover("8080");
    ok(
        &h,
        "POST",
        &format!("{base}/handovers"),
        &owner,
        json!({"title":"Amber handover","contributions":[first["id"]],"operation_id":null}),
    )
    .await;
    model_job(&h).await;
    let handovers = ok(&h, "GET", &format!("{base}/handovers"), &owner, Value::Null).await;
    let handover_id = handovers["items"][0]["claim_id"].clone();
    let initial_handover = detail(&h, &owner, &base, &handover_id).await;
    assert_eq!(
        initial_handover["selected"]["eligibility"]["strict_accepted"],
        true
    );
    let updated = change(
        &h,
        &owner,
        &base,
        &original,
        "Amber now listens on port 9090; this replaces the previous port 8080 configuration.\n",
    )
    .await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    *p.candidates.lock().unwrap() = extracted("9090", first["id"].clone());
    model_job(&h).await;
    let revised = runs(&h, &owner, &base).await;
    assert_eq!(revised["items"][0]["state"], "succeeded", "{revised}");
    assert_eq!(revised["items"][0]["revised"], 1);
    assert_eq!(revised["items"][0]["claim_ids"][0], claim);
    let after = detail(&h, &owner, &base, &claim).await;
    assert_eq!(after["selected"]["revision"]["content"]["value"], "9090");
    assert_eq!(after["selected"]["eligibility"]["strict_accepted"], true);
    assert_ne!(after["selected"]["revision"]["id"], first["id"]);
    assert_eq!(
        detail(&h, &owner, &base, &handover_id).await["selected"]["eligibility"]["strict_accepted"],
        false
    );
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    *p.candidates.lock().unwrap() = handover("9090");
    model_job(&h).await;
    let refreshed = detail(&h, &owner, &base, &handover_id).await;
    assert_eq!(refreshed["selected"]["revision"]["claim_id"], handover_id);
    assert_eq!(
        refreshed["selected"]["revision"]["content"]["value"],
        "Amber declares port 9090."
    );
    assert_eq!(
        refreshed["selected"]["eligibility"]["strict_accepted"],
        true
    );
    assert!(refreshed["selected"]["revision"]["reviewer_id"].is_null());
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );
    let _retired_source=change(&h,&owner,&base,&updated,"Amber's network listener was removed; the previous port 9090 declaration is explicitly obsolete.\n").await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    *p.candidates.lock().unwrap() = json!({"claims":[],"retirements":[{
        "revision_id":after["selected"]["revision"]["id"],"reason":"The new source explicitly removes the listener.","line_from":1,"line_to":1}]});
    model_job(&h).await;
    let retired = runs(&h, &owner, &base).await;
    assert_eq!(retired["items"][0]["state"], "succeeded", "{retired}");
    assert_eq!(retired["items"][0]["retired"], 1);
    let forgotten = detail(&h, &owner, &base, &claim).await;
    assert_eq!(
        forgotten["selected"]["revision"]["content"]["freshness"],
        "superseded"
    );
    assert_eq!(
        forgotten["selected"]["eligibility"]["strict_accepted"],
        false
    );
    assert_eq!(
        detail(&h, &owner, &base, &handover_id).await["selected"]["eligibility"]["investigation"],
        false
    );
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 5);
    let decisions: i64 = sqlx::query_scalar("SELECT count(*) FROM memory_decisions")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        decisions, 0,
        "The full loop must not manufacture human review decisions"
    );
    let independent = source(&h, &owner, &base, "Independent retained evidence.\n").await;
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        json!({"kind":"claim","id":claim}),
    )
    .await;
    let erase = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":{"kind":"claim","id":claim},"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert!(erase.is_object());
    assert!(detail(&h, &owner, &base, &claim).await["selected"].is_null());
    assert!(detail(&h, &owner, &base, &handover_id).await["selected"].is_null());
    assert_eq!(
        h.call(
            "GET",
            &format!(
                "{base}/sources/{}/versions/{}",
                independent["id"].as_str().unwrap(),
                independent["version"]["id"].as_str().unwrap()
            ),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn automatic_retries_are_bounded_and_human_overrides_survive_reconciliation() {
    let (h, owner, base, p, server) = setup().await;
    enable(&h, &owner, &base).await;
    let original = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    p.mode.store(1, Ordering::SeqCst);
    model_job(&h).await;
    assert_eq!(
        runs(&h, &owner, &base).await["items"][0]["error_code"],
        "provider_rate_limited"
    );
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );
    for attempt in 1..=2 {
        sqlx::query("UPDATE learning_runs SET finished_at=clock_timestamp()-interval '1 hour' WHERE state='failed'")
            .execute(&h.admin).await.unwrap();
        assert_eq!(
            autonomous::run_once(&h.state)
                .await
                .map_err(|e| e.1)
                .unwrap(),
            1
        );
        let queued = runs(&h, &owner, &base).await;
        assert_eq!(queued["items"][0]["automatic_attempt"], attempt);
        assert!(queued["items"][0]["retry_of"].is_string());
        assert_eq!(
            autonomous::run_once(&h.state)
                .await
                .map_err(|e| e.1)
                .unwrap(),
            0
        );
        model_job(&h).await;
    }
    sqlx::query("UPDATE learning_runs SET finished_at=clock_timestamp()-interval '1 hour' WHERE state='failed'")
        .execute(&h.admin).await.unwrap();
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    p.mode.store(0, Ordering::SeqCst);
    let updated = change(
        &h,
        &owner,
        &base,
        &original,
        "Amber.port = 8080\nAn additional observation confirms the declaration.\n",
    )
    .await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    *p.candidates.lock().unwrap() = extracted("8080", Value::Null);
    model_job(&h).await;
    let claim = runs(&h, &owner, &base).await["items"][0]["claim_ids"][0].clone();
    let before = detail(&h, &owner, &base, &claim).await;
    let r = &before["selected"]["revision"];
    let reviewed=ok(&h,"POST",&format!("{base}/claims/{}/review",claim.as_str().unwrap()),&owner,
        json!({"base_revision":r["id"],"action":"reject","reason":"Synthetic human override is authoritative.","content":null,"revalidation_basis":null})).await;
    let human = reviewed["claims"][0]["revision"].clone();
    assert!(human["reviewer_id"].is_string());
    let repeated = change(
        &h,
        &owner,
        &base,
        &updated,
        "Amber.port = 8080\nThe same rejected claim is repeated by this source.\n",
    )
    .await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    model_job(&h).await;
    let blocked = runs(&h, &owner, &base).await;
    assert_eq!(blocked["items"][0]["blocked"], 1, "{blocked}");
    assert_eq!(blocked["items"][0]["accepted"], 0);
    assert_eq!(
        detail(&h, &owner, &base, &claim).await["selected"]["revision"]["id"],
        human["id"]
    );
    let _forged = change(&h, &owner, &base, &repeated, "Amber.port = 9090\n").await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    *p.candidates.lock().unwrap() = extracted("9090", human["id"].clone());
    model_job(&h).await;
    assert_eq!(runs(&h, &owner, &base).await["items"][0]["state"], "failed");
    assert_eq!(
        detail(&h, &owner, &base, &claim).await["selected"]["revision"]["id"],
        human["id"]
    );
    let _ambiguity = source(
        &h,
        &owner,
        &base,
        "Cedar.port = 1111\nCedar.port = 2222\nBirch.port = 3333\n",
    )
    .await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    *p.candidates.lock().unwrap() = json!({"claims":[candidate("Cedar","port","1111",1),
        candidate("Cedar","port","2222",2),candidate("Birch","port","3333",3)],"retirements":[]});
    model_job(&h).await;
    let result = runs(&h, &owner, &base).await;
    assert_eq!(result["items"][0]["state"], "succeeded", "{result}");
    assert_eq!(result["items"][0]["conflicting"], 2);
    assert_eq!(result["items"][0]["accepted"], 1);
    for id in result["items"][0]["claim_ids"].as_array().unwrap() {
        let d = detail(&h, &owner, &base, id).await;
        let r = &d["selected"]["revision"];
        if r["content"]["subject"] == "Cedar" {
            assert_eq!(r["admission"], "uncertain_evidence");
            assert_eq!(d["selected"]["eligibility"]["strict_accepted"], false);
        } else {
            assert_eq!(d["selected"]["eligibility"]["strict_accepted"], true);
        }
    }
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );
    let _procedure=source(&h,&owner,&base,
        "Local procedure: read the configuration, then record its port. Expected result: a recorded declaration.\n").await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    *p.candidates.lock().unwrap() = json!({"claims":[{
        "kind":"procedure","subject":"Inspect configured port","predicate":"procedure","value":"Record the configured port.",
        "rationale":"The source supplies the steps and expected result.","line_from":1,"line_to":1,"replaces_revision":null,
        "procedure":{"conditions":"Local configuration is available.","steps":["Read the configuration.","Record its port."],
            "expected_outcome":"A recorded declaration."}}],"retirements":[]});
    model_job(&h).await;
    let generated = runs(&h, &owner, &base).await;
    assert_eq!(generated["items"][0]["state"], "succeeded", "{generated}");
    let procedure = detail(&h, &owner, &base, &generated["items"][0]["claim_ids"][0]).await;
    assert_eq!(
        procedure["selected"]["revision"]["content"]["kind"],
        "procedure"
    );
    assert_eq!(
        procedure["selected"]["revision"]["content"]["procedure"]["steps"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        procedure["selected"]["revision"]["content"]["procedure"]["observations"],
        json!([])
    );
    assert_eq!(
        procedure["selected"]["eligibility"]["strict_accepted"],
        true
    );
    assert_eq!(
        procedure["selected"]["eligibility"]["strict_operational"],
        false
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn autonomous_inflight_erasure_expiry_and_older_restore_preserve_independent_evidence() {
    let (mut h, owner, base, p, server) = setup().await;
    enable(&h, &owner, &base).await;
    let original = source(&h, &owner, &base, "Amber.port = 8080\n").await;
    let independent = source(&h, &owner, &base, "Birch.port = 3030\n").await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        2
    );
    for _ in 0..2 {
        let job = worker::claim(&h.state.pool, "model")
            .await
            .unwrap()
            .unwrap();
        let version: Uuid =
            sqlx::query_scalar("SELECT source_version_id FROM learning_runs WHERE id=$1")
                .bind(job.target_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        *p.candidates.lock().unwrap() =
            if version.to_string() == original["version"]["id"].as_str().unwrap() {
                extracted("8080", Value::Null)
            } else {
                json!({"claims":[candidate("Birch","port","3030",1)],"retirements":[]})
            };
        worker::execute(&h.state, &job).await.unwrap();
    }
    let claim = runs(&h, &owner, &base).await["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source_version_id"] == original["version"]["id"])
        .unwrap()["claim_ids"][0]
        .clone();
    let first = detail(&h, &owner, &base, &claim).await["selected"]["revision"].clone();
    let unrelated = runs(&h, &owner, &base).await["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source_version_id"] == independent["version"]["id"])
        .unwrap()["claim_ids"][0]
        .clone();
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable autonomous restore fixture: {backup}");
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
    let new = change(
        &h,
        &owner,
        &base,
        &original,
        "Amber.port = 9090\nThis supersedes the previous port declaration.\n",
    )
    .await;
    process(&h).await;
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    *p.candidates.lock().unwrap() = extracted("9090", first["id"].clone());
    p.delay.store(450, Ordering::SeqCst);
    let job = worker::claim(&h.state.pool, "model")
        .await
        .unwrap()
        .unwrap();
    let state = h.state.clone();
    let inflight = tokio::spawn(async move { worker::execute(&state, &job).await });
    wait_calls(&p, 3).await;
    let target = json!({"kind":"claim","id":claim});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    assert_eq!(
        preview["model_input_fences"], 2,
        "Both admitted reconciliation sources must be fenced"
    );
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    inflight.await.unwrap().unwrap();
    assert_eq!(
        runs(&h, &owner, &base).await["items"][0]["state"],
        "removed"
    );
    assert!(detail(&h, &owner, &base, &claim).await["selected"].is_null());
    assert_eq!(
        detail(&h, &owner, &base, &unrelated).await["selected"]["eligibility"]["strict_accepted"],
        true
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM claim_revisions WHERE value_key='9090'")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        0
    );
    assert_eq!(h.call("POST",&format!("{base}/learning"),Some(&owner),
        json!({"source_version_id":new["version"]["id"],"selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"manifest_revision_id":null,"operation_id":null})).await.0,StatusCode::CONFLICT);
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["claim_days"] = json!(1);
    let unrelated_id = Uuid::parse_str(unrelated.as_str().unwrap()).unwrap();
    sqlx::query("UPDATE claim_revisions SET recorded_at=clock_timestamp()-interval '2 days' WHERE claim_id=$1")
        .bind(unrelated_id).execute(&h.admin).await.unwrap();
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    assert!(detail(&h, &owner, &base, &unrelated).await["selected"].is_null());
    privacy_journal::run_once(&h.state).await.unwrap();
    let expired: (String, Value) =
        sqlx::query_as("SELECT privacy_state,revision FROM claim_revisions WHERE claim_id=$1")
            .bind(unrelated_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(expired.0, "expired");
    assert!(
        expired.1["content"].is_null(),
        "Physical expiry removes the retained assertion body"
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        3,
        "Expiry must not require a model or human action per record"
    );
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
    let new_id = Uuid::parse_str(new["version"]["id"].as_str().unwrap()).unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM model_input_fences WHERE source_version_id=$1"
        )
        .bind(new_id)
        .fetch_one(&backup_admin)
        .await
        .unwrap(),
        1,
        "Replay fences a source created after the backup"
    );
    p.delay.store(0, Ordering::SeqCst);
    let ctx = context(&h, &base).await;
    let positive = gateway::invoke(
        &backup_state,
        ctx,
        gateway::extraction(
            Uuid::new_v4(),
            Uuid::parse_str(independent["version"]["id"].as_str().unwrap()).unwrap(),
        ),
    )
    .await
    .map_err(|e| e.1)
    .unwrap();
    assert_eq!(positive.request.state, "succeeded");
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    backup_state.pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup}"))
        .execute(&h.root)
        .await
        .unwrap();
    server.abort();
    h.finish().await;
}
