use super::*;
use recollect_server::semantic;
#[path = "semantic_recovery.rs"]
mod recovery;

async fn status(h: &Harness, owner: &Login, base: &str) -> Value {
    ok(h, "GET", &format!("{base}/semantic"), owner, Value::Null).await
}

async fn discover(h: &Harness, cx: gateway::Context) -> usize {
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    semantic::maintain_brain(&h.state, cx.brain, cx.actor)
        .await
        .unwrap_or_else(|e| panic!("Semantic maintenance: {}", e.1))
}

async fn enable(h: &Harness, owner: &Login, base: &str) {
    allow(h, owner, base, |p| {
        p["automatic_embedding"] = json!(true);
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_discovery_cohorts_preserve_current_eligibility_and_all_state_identity() {
    let (h, owner, base, provider, server) = setup().await;
    let cx = context(&h, &base).await;
    let mut claims = Vec::new();
    let mut versions = Vec::new();
    for index in 0..8 {
        let evidence = source(&h, &owner, &base, "The service uses port 8080.\n").await;
        while worker::run_once(&h.state, "capture").await.unwrap() {}
        versions.push(
            evidence["version"]["id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap(),
        );
        let claim = ok(
            &h,
            "POST",
            &format!("{base}/claims"),
            &owner,
            review::proposal(
                &evidence["version"]["id"],
                &format!("Discovery service {index}"),
                "8080",
            ),
        )
        .await;
        let accepted = ok(&h, "POST", &format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),
            &owner,json!({"base_revision":claim["id"],"action":"accept","reason":"Checked synthetic source."})).await;
        claims.push(accepted["claims"][0]["revision"].clone());
    }
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let mut unchecked = Vec::new();
    for index in 0..101 {
        let claim = ok(
            &h,
            "POST",
            &format!("{base}/claims"),
            &owner,
            review::proposal(
                &json!(versions[7]),
                &format!("Unchecked prefix {index}"),
                "8080",
            ),
        )
        .await;
        unchecked.push(claim["id"].as_str().unwrap().parse::<Uuid>().unwrap());
    }
    // Give the unavailable cohort an earlier discovery order. Taking the first
    // 100 identities before support qualification would hide the valid claim.
    sqlx::query("UPDATE claim_revisions SET recorded_at=clock_timestamp()-interval '1 minute',
        revision=jsonb_set(revision,'{recorded_at}',to_jsonb(clock_timestamp()-interval '1 minute')) WHERE id=ANY($1)")
        .bind(&unchecked).execute(&h.admin).await.unwrap();
    enable(&h, &owner, &base).await;
    let profile = ok(
        &h,
        "POST",
        &format!("{base}/semantic/reindex"),
        &owner,
        json!({"base_profile":null}),
    )
    .await;
    let profile_id: Uuid = profile["id"].as_str().unwrap().parse().unwrap();
    for (index, state) in [
        "pending", "queued", "running", "ready", "blocked", "failed", "removed",
    ]
    .iter()
    .enumerate()
    {
        sqlx::query("INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id,state,embedding)
            SELECT $1,$2,$3,'source_chunk',c.id,c.id,c.version_id,$5,
              CASE WHEN $5='ready' THEN array_fill(1::real,ARRAY[3072])::vector ELSE NULL END
            FROM source_chunks c WHERE c.brain_id=$2 AND c.version_id=$4")
            .bind(Uuid::new_v4()).bind(cx.brain).bind(profile_id).bind(versions[index]).bind(state)
            .execute(&h.admin).await.unwrap();
        let revision: Uuid = claims[index]["id"].as_str().unwrap().parse().unwrap();
        sqlx::query("INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,claim_revision_id,state,embedding)
            VALUES($1,$2,$3,'claim_revision',$4,$4,$5,
              CASE WHEN $5='ready' THEN array_fill(1::real,ARRAY[3072])::vector ELSE NULL END)")
            .bind(Uuid::new_v4()).bind(cx.brain).bind(profile_id).bind(revision).bind(state)
            .execute(&h.admin).await.unwrap();
    }
    // The old query is retained as a counterfactual, rather than recreating the
    // new cohort logic in assertions. Both run through the actual app role/RLS.
    let old_sql = include_str!("../fixtures/semantic_candidates_before_discovery.sql");
    let new_sql = include_str!("../../src/semantic_candidates.sql");
    let old_query = format!(
        "SELECT kind,input_id FROM ({old_sql}) c WHERE NOT EXISTS(
        SELECT 1 FROM semantic_entries e WHERE e.brain_id=$1 AND e.profile_id=$3::uuid
          AND e.kind=c.kind AND e.input_id=c.input_id) ORDER BY created_at,kind,input_id LIMIT 100"
    );
    let new_query = format!(
        "SELECT kind,input_id FROM ({new_sql}) c ORDER BY created_at,kind,input_id LIMIT 100"
    );
    let classes = vec!["document", "claim", "repository"];
    let mut tx = db::actor_tx(&h.state.pool, cx.actor).await.ok().unwrap();
    for (selected_profile, expected) in [
        (None, 16),
        (Some(profile_id), 2),
        (Some(Uuid::new_v4()), 16),
    ] {
        let old: Vec<(String, Uuid)> = sqlx::query_as(&old_query)
            .bind(cx.brain)
            .bind(&classes)
            .bind(selected_profile)
            .fetch_all(&mut *tx)
            .await
            .unwrap();
        let new: Vec<(String, Uuid)> = sqlx::query_as(&new_query)
            .bind(cx.brain)
            .bind(&classes)
            .bind(selected_profile)
            .fetch_all(&mut *tx)
            .await
            .unwrap();
        assert_eq!(
            new, old,
            "The exact eligible canonical identities must match"
        );
        assert_eq!(new.len(), expected);
        assert!(
            !new.iter().any(|(_, revision)| unchecked.contains(revision)),
            "Browser submission alone cannot qualify a claim for embedding"
        );
    }
    tx.rollback().await.unwrap();
    // A new current revision is a new identity; the old represented revision
    // must not suppress it, while rejection still excludes a missing input.
    ok(&h, "POST", &format!("{base}/claims/{}/review",claims[0]["claim_id"].as_str().unwrap()),
        &owner, json!({"base_revision":claims[0]["id"],"action":"revalidate","reason":"New current identity.","revalidation_basis":"review_correction","content":claims[0]["content"]})).await;
    ok(&h, "POST", &format!("{base}/claims/{}/review",claims[7]["claim_id"].as_str().unwrap()),
        &owner, json!({"base_revision":claims[7]["id"],"action":"reject","reason":"Negative eligibility control."})).await;
    let mut tx = db::actor_tx(&h.state.pool, cx.actor).await.ok().unwrap();
    let old: Vec<(String, Uuid)> = sqlx::query_as(&old_query)
        .bind(cx.brain)
        .bind(&classes)
        .bind(profile_id)
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    let new: Vec<(String, Uuid)> = sqlx::query_as(&new_query)
        .bind(cx.brain)
        .bind(&classes)
        .bind(profile_id)
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    assert_eq!(new, old);
    assert_eq!(
        new.len(),
        2,
        "One missing chunk and one newly accepted revision"
    );
    tx.rollback().await.unwrap();
    let (foreign, _) = h.fixture_member().await;
    let mut tx = db::actor_tx(&h.state.pool, foreign).await.ok().unwrap();
    let hidden: Vec<(String, Uuid)> = sqlx::query_as(&new_query)
        .bind(cx.brain)
        .bind(&classes)
        .bind(None::<Uuid>)
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    assert!(hidden.is_empty(), "Missing profile cannot bypass Brain RLS");
    tx.rollback().await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let requests: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        requests, 0,
        "Cohort qualification does not need a model judge or call"
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_automatic_batches_reindex_and_canonical_new_inputs() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    p.mode.store(6, Ordering::SeqCst);
    for i in 0..23 {
        let text = if i < 3 {
            format!("Service {i}: {}", "q".repeat(2950))
        } else {
            format!("Independent component {i} uses its declared endpoint.")
        };
        source(&h, &owner, &base, &text).await;
    }
    assert_eq!(
        discover(&h, cx).await,
        0,
        "Migration cannot enable transmission"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    enable(&h, &owner, &base).await;
    let queued = semantic::run_once(&h.state)
        .await
        .unwrap_or_else(|e| panic!("Semantic discovery: {}", e.1));
    assert!(
        queued >= 2,
        "Both byte and item limits require bounded batches"
    );
    let before = status(&h, &owner, &base).await;
    assert_eq!(before["counts"]["queued"], 23);
    let profile = before["profile"]["id"].clone();
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let ready = status(&h, &owner, &base).await;
    assert_eq!(
        ready["counts"]["ready"], 23,
        "Actual worker must publish every permitted input"
    );
    assert_eq!(ready["state"], "ready");
    assert_eq!(p.calls.load(Ordering::SeqCst), queued);
    for body in p.bodies.lock().unwrap().iter() {
        let inputs = body["input"].as_array().unwrap();
        assert!(inputs.len() <= 20);
        assert!(
            inputs
                .iter()
                .map(|v| v.as_str().unwrap().len())
                .sum::<usize>()
                <= 8000
        );
    }
    assert_eq!(
        discover(&h, cx).await,
        0,
        "Completed entries cannot be silently embedded again"
    );
    let extra = source(
        &h,
        &owner,
        &base,
        "A newly captured independent service uses port 9191.",
    )
    .await;
    assert_eq!(discover(&h, cx).await, 1);
    model_job(&h).await;
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 24);
    // Removing standing capture/index permission preserves the saved projection
    // and prevents newly created evidence from causing additional paid work.
    allow(&h, &owner, &base, |p| {
        p["automatic_embedding"] = json!(false);
    })
    .await;
    source(
        &h,
        &owner,
        &base,
        "Pending until approved once; no human per-memory queue.",
    )
    .await;
    assert_eq!(discover(&h, cx).await, 0);
    let calls = p.calls.load(Ordering::SeqCst);
    enable(&h, &owner, &base).await;
    let reindexed = ok(
        &h,
        "POST",
        &format!("{base}/semantic/reindex"),
        &owner,
        json!({"base_profile":profile}),
    )
    .await;
    assert_ne!(reindexed["id"], profile);
    assert_eq!(
        status(&h, &owner, &base).await["counts"]["ready"],
        0,
        "A new generation cannot expose old vectors"
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        calls,
        "Reindex queues; it does not bypass the worker"
    );
    discover(&h, cx).await;
    let obsolete: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM semantic_entries WHERE profile_id=$1 AND embedding IS NOT NULL",
    )
    .bind(profile.as_str().unwrap().parse::<Uuid>().unwrap())
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(obsolete, 0);
    while worker::run_once(&h.state, "model").await.unwrap() {}
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 25);
    assert!(extra["version"]["id"].is_string());
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_retries_are_separate_bounded_and_budget_blocks_resume() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    source(
        &h,
        &owner,
        &base,
        "Retry proof: the cache belongs to service Amber.",
    )
    .await;
    assert_eq!(discover(&h, cx).await, 1);
    p.mode.store(1, Ordering::SeqCst);
    model_job(&h).await;
    let first = status(&h, &owner, &base).await;
    assert_eq!(first["batches"][0]["state"], "failed");
    assert_eq!(first["batches"][0]["error_code"], "provider_rate_limited");
    assert_eq!(discover(&h, cx).await, 0);
    let mut last = first["batches"][0]["id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    for (attempt, minutes) in [(1, 6), (2, 31)] {
        sqlx::query("UPDATE semantic_batches SET finished_at=clock_timestamp()-make_interval(mins=>$2) WHERE id=$1")
            .bind(last).bind(minutes).execute(&h.admin).await.unwrap();
        assert_eq!(discover(&h, cx).await, 1);
        model_job(&h).await;
        let after = status(&h, &owner, &base).await;
        assert_eq!(after["batches"][0]["automatic_attempt"], attempt);
        assert_eq!(after["batches"][0]["retry_of"], last.to_string());
        last = after["batches"][0]["id"].as_str().unwrap().parse().unwrap();
    }
    sqlx::query(
        "UPDATE semantic_batches SET finished_at=clock_timestamp()-interval '1 day' WHERE id=$1",
    )
    .bind(last)
    .execute(&h.admin)
    .await
    .unwrap();
    assert_eq!(discover(&h, cx).await, 0);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        3,
        "Only two confirmed transient replacements are automatic"
    );
    p.mode.store(0, Ordering::SeqCst);
    let retry_path = format!("{base}/semantic/batches/{last}/retry");
    let (code, new, _) = h
        .keyed(
            "POST",
            &retry_path,
            Some(&owner),
            Value::Null,
            Some("semantic-retry-proof"),
        )
        .await;
    assert_eq!(code, StatusCode::OK, "{new}");
    let (code, replayed, _) = h
        .keyed(
            "POST",
            &retry_path,
            Some(&owner),
            Value::Null,
            Some("semantic-retry-proof"),
        )
        .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(new["id"], replayed["id"]);
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 1);
    // A blocked pre-admission batch has no paid attempt and can resume when the
    // current standing budget changes. Old failures remain independently visible.
    allow(&h, &owner, &base, |p| {
        p["automatic_embedding"] = json!(true);
        p["daily_token_limit"] = json!(1000);
    })
    .await;
    source(
        &h,
        &owner,
        &base,
        "Budget resume proof: service Beryl keeps its own source.",
    )
    .await;
    assert_eq!(discover(&h, cx).await, 1);
    model_job(&h).await;
    let blocked = status(&h, &owner, &base).await;
    assert_eq!(blocked["batches"][0]["state"], "blocked");
    assert_eq!(
        blocked["batches"][0]["error_code"],
        "model_budget_exhausted"
    );
    assert!(blocked["batches"][0]["request_id"].is_null());
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    allow(&h, &owner, &base, |p| {
        p["automatic_embedding"] = json!(true);
        p["daily_token_limit"] = json!(100000);
    })
    .await;
    sqlx::query("UPDATE semantic_entries SET updated_at=clock_timestamp()-interval '2 minutes' WHERE state='blocked'")
        .execute(&h.admin).await.unwrap();
    assert_eq!(discover(&h, cx).await, 1);
    model_job(&h).await;
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 2);
    assert_eq!(p.calls.load(Ordering::SeqCst), 5);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_lost_paid_response_and_erasure_cannot_publish_or_resend() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    source(
        &h,
        &owner,
        &base,
        "A retained independent source survives an interrupted query.",
    )
    .await;
    assert_eq!(discover(&h, cx).await, 1);
    p.delay.store(1000, Ordering::SeqCst);
    let state = h.state.clone();
    let task = tokio::spawn(async move { worker::run_once(&state, "model").await });
    p.entered.notified().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    sqlx::query("UPDATE jobs SET lease_until=clock_timestamp()-interval '1 second' WHERE kind='semantic.generate' AND state='running'")
        .execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE model_requests SET deadline=clock_timestamp()-interval '1 second' WHERE state='running'")
        .execute(&h.admin).await.unwrap();
    model_job(&h).await;
    let interrupted = status(&h, &owner, &base).await;
    assert_eq!(interrupted["batches"][0]["state"], "failed");
    assert_eq!(
        interrupted["batches"][0]["error_code"],
        "model_attempt_recorded"
    );
    let uncertain: i64 =
        sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE state='uncertain'")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(uncertain, 1);
    assert_eq!(discover(&h, cx).await, 0);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        1,
        "Reclaimed job cannot repeat an admitted request"
    );
    let id = interrupted["batches"][0]["id"].as_str().unwrap();
    ok(
        &h,
        "POST",
        &format!("{base}/semantic/batches/{id}/retry"),
        &owner,
        Value::Null,
    )
    .await;
    p.delay.store(0, Ordering::SeqCst);
    model_job(&h).await;
    p.entered.notified().await;
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 1);
    let erased = source(
        &h,
        &owner,
        &base,
        "Erase this newly captured input while the provider is working.",
    )
    .await;
    let remaining = source(
        &h,
        &owner,
        &base,
        "Independent retry input from the partially removed batch.",
    )
    .await;
    assert_eq!(discover(&h, cx).await, 1);
    p.delay.store(150, Ordering::SeqCst);
    let state = h.state.clone();
    let task = tokio::spawn(async move { worker::run_once(&state, "model").await });
    p.entered.notified().await;
    let target = json!({"kind":"source","id":erased["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    task.await.unwrap().unwrap();
    let after = status(&h, &owner, &base).await;
    assert_eq!(
        after["counts"]["ready"], 1,
        "The independent vector remains useful"
    );
    assert_eq!(after["counts"]["removed"], 1);
    let forbidden:i64=sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE source_version_id=$1 AND embedding IS NOT NULL")
        .bind(erased["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    assert_eq!(forbidden, 0);
    assert_eq!(discover(&h, cx).await, 0);
    assert_eq!(p.calls.load(Ordering::SeqCst), 3);
    let interrupted_batch = &after["batches"][0];
    assert_eq!(interrupted_batch["can_retry"], true);
    let retry = ok(
        &h,
        "POST",
        &format!(
            "{base}/semantic/batches/{}/retry",
            interrupted_batch["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        retry["input_count"], 1,
        "Only the surviving canonical input enters the explicit new attempt"
    );
    p.delay.store(0, Ordering::SeqCst);
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 4);
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 2);
    let (retained,removed):(i64,i64)=sqlx::query_as("SELECT count(*) FILTER(WHERE source_version_id=$1 AND embedding IS NOT NULL),count(*) FILTER(WHERE source_version_id=$2 AND embedding IS NOT NULL) FROM semantic_entries")
        .bind(remaining["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .bind(erased["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    assert_eq!((retained, removed), (1, 0));
    let newest = status(&h, &owner, &base).await;
    assert_eq!(
        newest["batches"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["id"] == interrupted_batch["id"])
            .unwrap()["can_retry"],
        false
    );
    server.abort();
    h.finish().await;
}

fn embed(entries: &[Uuid]) -> gateway::Invocation {
    gateway::Invocation {
        operation: Uuid::new_v4(),
        purpose: "embedding".into(),
        inputs: entries
            .iter()
            .map(|id| gateway::InputRef {
                kind: "semantic_entry".into(),
                id: *id,
            })
            .collect(),
        query: None,
        instructions: String::new(),
        prompt_label: "engineering-text-1".into(),
        schema_label: "embedding-3072-1".into(),
        format: gateway::Format::Embedding,
        work_lease: None,
        metadata_replay: false,
        expected_json: None,
    }
}

fn query(text: &str, channels: &[&str]) -> Value {
    let mut input = json!({"query":text,"channels":channels,"context_bytes":8192});
    if channels.contains(&"semantic") {
        input["semantic_request_id"] = json!(Uuid::new_v4());
    }
    input
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_exact_scan_continues_after_more_than_one_hundred_unavailable_inputs() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    p.mode.store(5, Ordering::SeqCst);
    let control = source(
        &h,
        &owner,
        &base,
        "NEAR_VECTOR: Independent retained engineering evidence.",
    )
    .await;
    let mut unavailable = Vec::new();
    for i in 0..101 {
        let source = source(
            &h,
            &owner,
            &base,
            &format!("High scoring fixture {i}: this artifact will become unreadable."),
        )
        .await;
        unavailable.push(
            source["version"]["id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap(),
        );
    }
    while discover(&h, cx).await > 0 {
        while worker::run_once(&h.state, "model").await.unwrap() {}
    }
    assert_eq!(status(&h, &owner, &base).await["counts"]["ready"], 102);
    let files: Vec<Uuid> =
        sqlx::query_scalar("SELECT artifact_id FROM source_versions WHERE id=ANY($1)")
            .bind(&unavailable)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    for artifact in files {
        tokio::fs::write(
            recollect_server::artifacts::path(&h.state.config.artifact_dir, cx.brain, artifact),
            "Changed fixture bytes",
        )
        .await
        .unwrap();
    }
    let result = recall(
        &h,
        &owner,
        &base,
        query("Find retained engineering evidence", &["semantic"]),
    )
    .await;
    assert_eq!(result["context"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["context"]["items"][0]["id"],
        control["version"]["id"]
    );
    assert_eq!(result["coverage"]["withheld"], 101);
    assert_eq!(result["coverage"]["unavailable"], 101);
    assert_eq!(result["coverage"]["examined"], 102);
    assert_eq!(result["semantic"]["scoped_entries"], 102);
    let calls = p.calls.load(Ordering::SeqCst);
    let mut strict = query("Find retained engineering evidence", &["semantic"]);
    strict["mode"] = json!("strict_accepted");
    let strict = recall(&h, &owner, &base, strict).await;
    assert!(strict["context"]["items"].as_array().unwrap().is_empty());
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        calls,
        "No strict raw-evidence query is transmitted"
    );
    server.abort();
    h.finish().await;
}

async fn recall(h: &Harness, owner: &Login, base: &str, input: Value) -> Value {
    let result = ok(h, "POST", &format!("{base}/recall"), owner, input).await;
    assert_eq!(
        serde_json::to_vec(&result["context"]).unwrap().len(),
        result["context_bytes"].as_u64().unwrap() as usize
    );
    assert!(result["context_bytes"].as_u64().unwrap() <= 8192);
    result
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_recall_uses_exact_cosine_scope_and_independent_channel_ranks() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    p.mode.store(5, Ordering::SeqCst);
    // No compatible inputs means no paid query, even though embedding is approved.
    let empty = recall(&h, &owner, &base, query("previous runs", &["semantic"])).await;
    assert_eq!(empty["semantic"]["state"], "missing");
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    let collection = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"collection","name":"Semantic math corpus"}),
    )
    .await;
    let exact = source(
        &h,
        &owner,
        &base,
        "Operational executions are retained in the audit journal.",
    )
    .await;
    let near = source(
        &h,
        &owner,
        &base,
        "NEAR_VECTOR: The previous runs view also exposes execution metadata.",
    )
    .await;
    let distractor = source(
        &h,
        &owner,
        &base,
        "DISTRACTOR_VECTOR: Tea and coffee are available in the kitchen.",
    )
    .await;
    let outside = source(
        &h,
        &owner,
        &base,
        "OUTSIDE_SCOPE_CANARY: previous runs must never enter the selected collection.",
    )
    .await;
    for source in [&exact, &near, &distractor] {
        ok(
            &h,
            "PUT",
            &format!("{base}/sources/{}/groups", source["id"].as_str().unwrap()),
            &owner,
            json!({"group_ids":[collection["id"]]}),
        )
        .await;
    }
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let before = p.calls.load(Ordering::SeqCst);
    let mut lexical = query("previous runs", &["exact", "lexical"]);
    lexical["collection_id"] = collection["id"].clone();
    let lexical = recall(&h, &owner, &base, lexical).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), before);
    assert_eq!(lexical["context"]["items"][0]["id"], near["version"]["id"]);
    let mut semantic_input = query("previous runs", &["semantic"]);
    semantic_input["collection_id"] = collection["id"].clone();
    semantic_input["semantic_min_similarity"] = json!(0.5);
    let semantic = recall(&h, &owner, &base, semantic_input.clone()).await;
    let items = semantic["context"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], exact["version"]["id"]);
    assert_eq!(items[1]["id"], near["version"]["id"]);
    // Independent dot-product and Euclidean norm, not the database operator.
    let expected = (1.0f64 * 3.0 + 0.0 * 4.0) / ((1.0f64 + 0.0).sqrt() * (9.0f64 + 16.0).sqrt());
    assert!((items[1]["semantic_similarity"].as_f64().unwrap() - expected).abs() < 1e-6);
    assert_eq!(semantic["semantic"]["scoped_entries"], 3);
    assert!(!semantic.to_string().contains("OUTSIDE_SCOPE_CANARY"));
    assert!(
        items[0]["qualifications"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "semantic_similarity_not_truth")
    );
    let (code, replay, _) = h
        .call(
            "POST",
            &format!("{base}/recall"),
            Some(&owner),
            semantic_input,
        )
        .await;
    assert_eq!(code, StatusCode::CONFLICT, "{replay}");
    assert_eq!(replay["code"], "model_attempt_recorded");
    assert_eq!(p.calls.load(Ordering::SeqCst), before + 1);
    let mut combined = query("previous runs", &["exact", "lexical", "semantic"]);
    combined["collection_id"] = collection["id"].clone();
    combined["semantic_min_similarity"] = json!(0.5);
    let combined = recall(&h, &owner, &base, combined).await;
    assert_eq!(combined["algorithm"], "identity-priority-rrf-k60");
    assert_eq!(
        combined["context"]["items"][0]["id"], near["version"]["id"],
        "The independently lexical and semantic match has both ranks"
    );
    assert_eq!(
        combined["context"]["items"][0]["channels"],
        json!(["lexical", "semantic"])
    );
    let mut exact_id = query("previous runs", &["exact", "semantic"]);
    exact_id["exact"] = json!({"kind":"source_version","id":distractor["version"]["id"]});
    exact_id["collection_id"] = collection["id"].clone();
    let exact_id = recall(&h, &owner, &base, exact_id).await;
    assert_eq!(
        exact_id["context"]["items"][0]["id"], distractor["version"]["id"],
        "Explicit identity stays first even when its cosine is lower"
    );
    assert_ne!(outside["version"]["id"], exact["version"]["id"]);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_chunk_gateway_uses_exact_bytes_parent_dependencies_and_privacy_fences() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    let defaults = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(defaults["current"]["policy"]["automatic_embedding"], false);
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(true);
    })
    .await;
    let text = format!(
        "FIRST_SELECTED\n{}MIDDLE_NEVER_EMBED{}\nLAST_SELECTED\n",
        "a".repeat(5000),
        "b".repeat(4000)
    );
    let source = source(&h, &owner, &base, &text).await;
    let version = source["version"]["id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let chunks: Vec<(Uuid, i32)> =
        sqlx::query_as("SELECT id,ordinal FROM source_chunks WHERE version_id=$1 ORDER BY ordinal")
            .bind(version)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert_eq!(chunks.len(), 3);
    let profile = Uuid::new_v4();
    sqlx::query("INSERT INTO semantic_profiles(id,brain_id,provider,model,dimensions,representation,created_by,policy_id) VALUES($1,$2,'openai','text-embedding-3-large',3072,'engineering-text-1',$3,(SELECT policy_id FROM model_policy_heads WHERE brain_id=$2))")
        .bind(profile).bind(cx.brain).bind(cx.actor).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO semantic_heads(brain_id,profile_id) VALUES($1,$2)")
        .bind(cx.brain)
        .bind(profile)
        .execute(&h.admin)
        .await
        .unwrap();
    let mut entries = Vec::new();
    for chunk in [chunks[0].0, chunks[2].0] {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id) VALUES($1,$2,$3,'source_chunk',$4,$4,$5)")
            .bind(id).bind(cx.brain).bind(profile).bind(chunk).bind(version).execute(&h.admin).await.unwrap();
        entries.push(id);
    }
    let request = embed(&entries);
    let operation = request.operation;
    let result = gateway::invoke(&h.state, cx, request)
        .await
        .unwrap_or_else(|e| panic!("Gateway failed: {}", e.1));
    let Some(gateway::Output::Embeddings(vectors)) = result.output else {
        panic!("Expected two complete embedding vectors");
    };
    assert_eq!(vectors.len(), 2);
    assert!(
        vectors
            .iter()
            .all(|v| v.len() == 3072 && v.iter().all(|n| n.is_finite()))
    );
    let calls = p.bodies.lock().unwrap().clone();
    let sent = calls[0]["input"].as_array().unwrap();
    assert_eq!(sent.len(), 2);
    assert!(sent[0].as_str().unwrap().contains("FIRST_SELECTED"));
    assert!(sent[1].as_str().unwrap().contains("LAST_SELECTED"));
    assert!(
        !sent
            .iter()
            .any(|value| value.as_str().unwrap().contains("MIDDLE_NEVER_EMBED"))
    );
    assert!(sent.iter().all(|value| {
        value
            .as_str()
            .unwrap()
            .starts_with("Title: Synthetic model evidence\nRole: source_document\nText:\n")
    }));
    let links: Vec<(String, Uuid)> = sqlx::query_as(
        "SELECT kind,input_id FROM model_request_inputs WHERE request_id=$1 ORDER BY kind,input_id",
    )
    .bind(result.request.id)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        links.len(),
        3,
        "Two chunk entries share exactly one parent source dependency"
    );
    assert!(links.contains(&("source_version".into(), version)));
    let mut replay = embed(&entries);
    replay.operation = operation;
    assert_eq!(
        gateway::invoke(&h.state, cx, replay).await.err().unwrap().1,
        "model_attempt_recorded"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    // A corrupt processing copy cannot replace the original artifact bytes.
    sqlx::query("UPDATE source_chunks SET content='TAMPERED_CHUNK' WHERE id=$1")
        .bind(chunks[0].0)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        gateway::invoke(&h.state, cx, embed(&entries[..1]))
            .await
            .err()
            .unwrap()
            .1,
        "semantic_source_span_unavailable"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    // The unaffected chunk remains a usable independent input.
    let good = gateway::invoke(&h.state, cx, embed(&entries[1..]))
        .await
        .unwrap_or_else(|e| panic!("Independent gateway input failed: {}", e.1));
    assert_eq!(good.request.state, "succeeded");
    let encoded = serde_json::to_string(&vectors[1]).unwrap();
    sqlx::query("UPDATE semantic_entries SET state='ready',embedding=$2::text::vector,request_id=$3 WHERE id=$1")
        .bind(entries[1]).bind(encoded).bind(good.request.id).execute(&h.admin).await.unwrap();
    let target = json!({"kind":"source","id":source["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let cleared: i64 = sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE brain_id=$1 AND state='removed' AND embedding IS NULL")
        .bind(cx.brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(cleared, 2);
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT suppressed FROM model_requests WHERE id=$1")
            .bind(good.request.id)
            .fetch_one(&h.admin)
            .await
            .unwrap()
    );
    assert_eq!(
        gateway::invoke(&h.state, cx, embed(&entries[1..]))
            .await
            .err()
            .unwrap()
            .1,
        "content_removed"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_budget_block_waits_for_reset_or_a_policy_change() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    p.mode.store(6, Ordering::SeqCst);
    enable(&h, &owner, &base).await;
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(true);
        policy["daily_token_limit"] = json!(1000);
    })
    .await;
    source(&h, &owner, &base, "Synthetic budget scheduling source.").await;
    assert_eq!(discover(&h, cx).await, 1);
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    let blocked = status(&h, &owner, &base).await;
    assert!(blocked["counts"]["blocked"].as_i64().unwrap() > 0);
    sqlx::query("UPDATE semantic_entries SET updated_at=clock_timestamp()-interval '2 minutes'")
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        discover(&h, cx).await,
        0,
        "Same-day budget rejection must not loop every minute"
    );
    sqlx::query("UPDATE semantic_entries SET updated_at=clock_timestamp()-interval '1 day'")
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        discover(&h, cx).await,
        1,
        "A new UTC day permits a new check"
    );
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    sqlx::query("UPDATE semantic_entries SET updated_at=clock_timestamp()-interval '2 minutes'")
        .execute(&h.admin)
        .await
        .unwrap();
    allow(&h, &owner, &base, |policy| {
        policy["automatic_embedding"] = json!(true);
        policy["daily_token_limit"] = json!(100000);
    })
    .await;
    assert_eq!(
        discover(&h, cx).await,
        1,
        "A changed policy permits a new check"
    );
    model_job(&h).await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    assert!(
        status(&h, &owner, &base).await["counts"]["ready"]
            .as_i64()
            .unwrap()
            > 0
    );
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; local HTTP embedding fixture only"]
async fn semantic_windows_survive_lexical_alternatives_and_keep_span_scores() {
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    p.mode.store(5, Ordering::SeqCst);
    let text = (0..6)
        .map(|i| format!("marker window {i}: {}\n", "q".repeat(3950)))
        .collect::<String>();
    let original = source(&h, &owner, &base, &text).await;
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let mut input = query("marker", &["lexical", "semantic"]);
    input["context_bytes"] = json!(16384);
    input["limit"] = json!(3);
    let recalled = ok(&h, "POST", &format!("{base}/recall"), &owner, input.clone()).await;
    let items = recalled["context"]["items"].as_array().unwrap();
    assert!(items.len() >= 2, "{recalled}");
    assert!(recalled["context_bytes"].as_u64().unwrap() <= 16384);
    let mut spans = Vec::new();
    for item in items {
        assert_eq!(item["id"], original["version"]["id"]);
        assert!(
            item["channels"]
                .as_array()
                .unwrap()
                .contains(&json!("semantic"))
        );
        assert!((item["semantic_similarity"].as_f64().unwrap() - 1.0).abs() < 1e-6);
        let provenance = &item["provenance"][0];
        let from = provenance["byte_from"].as_u64().unwrap() as usize;
        let to = provenance["byte_to"].as_u64().unwrap() as usize;
        assert_eq!(&text[from..to], item["text"].as_str().unwrap());
        assert!(spans.iter().all(|(a, b)| to <= *a || from >= *b));
        spans.push((from, to));
    }
    // A scored primary cannot silently become a different lexical span when
    // its complete attribution exceeds the caller's tight budget.
    input["semantic_request_id"] = json!(Uuid::new_v4());
    input["context_bytes"] = json!(2048);
    let tight = ok(&h, "POST", &format!("{base}/recall"), &owner, input).await;
    assert!(
        tight["context"]["items"].as_array().unwrap().is_empty(),
        "{tight}"
    );
    server.abort();
    h.finish().await;
}
