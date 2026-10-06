use super::*;
use chrono::DateTime;

async fn feed(h: &Harness, owner: &Login, base: &str) -> Value {
    ok(h, "GET", &format!("{base}/pipeline"), owner, Value::Null).await
}
fn input<'a>(feed: &'a Value, version: &Value) -> &'a Value {
    feed["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["source_version_id"] == *version)
        .unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run with --ignored"]
async fn pipeline_exact_origins_jobs_outcomes_access_and_retention() {
    let (h, owner, base, provider, server) = setup().await;
    capture_policy(&h, &owner, &base, true).await;
    allow(&h, &owner, &base, |_| {}).await;
    *provider.candidates.lock().unwrap() = json!({"claims":[]});
    let (writer_id, writer) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        json!({"role":"writer"}),
    )
    .await;
    let (first_device, first_token) = h
        .pair_with(&writer, "First coding host", Some("codex"), Some("plugin"))
        .await;
    let (second_device, second_token) = h
        .pair_with(&owner, "Second coding host", Some("codex"), Some("plugin"))
        .await;
    let selection = json!({"repository_ids":[],"area_ids":[],"environment_id":null});
    let first_binding = binding(&h, &base, &first_token, selection.clone()).await;
    let second_binding = binding(&h, &base, &second_token, selection).await;
    sqlx::query("UPDATE capture_bindings SET created_at=now()-interval '3 days' WHERE id=$1")
        .bind(Uuid::parse_str(first_binding["id"].as_str().unwrap()).unwrap())
        .execute(&h.admin)
        .await
        .unwrap();
    let mut first_event = event(&first_binding, "first", "First pipeline source.");
    first_event["event"]["agent_id"] = json!("worker-one");
    first_event["event"]["host_session_id"] = json!("first-session");
    // Delayed host upload must still sort by receipt, not old host time.
    first_event["event"]["captured_at"] = json!(Utc::now() - Duration::days(2));
    let first = device_ok(
        &h,
        "POST",
        &format!("{base}/capture/events"),
        &first_token,
        first_event,
    )
    .await;
    let mut second_event = event(&second_binding, "second", "Second pipeline source.");
    second_event["event"]["agent_id"] = json!("worker-two");
    let second = device_ok(
        &h,
        "POST",
        &format!("{base}/capture/events"),
        &second_token,
        second_event,
    )
    .await;
    let first_version = first["source_version_id"].clone();
    let second_version = second["source_version_id"].clone();
    let snapshot = feed(&h, &owner, &base).await;
    assert_eq!(snapshot["items"].as_array().unwrap().len(), 2);
    let a = input(&snapshot, &first_version);
    let b = input(&snapshot, &second_version);
    assert_eq!(a["device_id"], json!(first_device));
    assert_eq!(a["actor_id"], json!(writer_id));
    assert_eq!(a["agent_id"], "worker-one");
    assert_eq!(a["host_session_id"], "first-session");
    assert_eq!(a["binding_id"], first_binding["id"]);
    assert_eq!(b["device_id"], json!(second_device));
    assert_ne!(a["processing_job"]["id"], b["processing_job"]["id"]);
    assert!(a["learning"].is_null());
    assert!(
        DateTime::parse_from_rfc3339(a["received_at"].as_str().unwrap()).unwrap()
            > Utc::now() - Duration::minutes(1)
    );
    let first_job: Uuid = serde_json::from_value(a["processing_job"]["id"].clone()).unwrap();
    let second_job: Uuid = serde_json::from_value(b["processing_job"]["id"].clone()).unwrap();
    sqlx::query("UPDATE jobs SET state='running',lease_until=now()+interval '1 minute',updated_at=clock_timestamp() WHERE id=ANY($1)").bind(vec![first_job,second_job]).execute(&h.admin).await.unwrap();
    let running = feed(&h, &owner, &base).await;
    assert!(
        running["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["processing_job"]["state"] == "running" && i["active"] == true)
    );
    sqlx::query("UPDATE jobs SET state='queued',lease_until=NULL WHERE id=ANY($1)")
        .bind(vec![first_job, second_job])
        .execute(&h.admin)
        .await
        .unwrap();
    process(&h).await;
    // Actual provider/worker path succeeds with no memory output; contributor stays writer.
    let run=ok(&h,"POST",&format!("{base}/learning"),&owner,json!({"source_version_id":first_version,"selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"manifest_revision_id":null,"operation_id":null})).await;
    provider.delay.store(600, Ordering::SeqCst);
    let calls_before = provider.calls.load(Ordering::SeqCst);
    let working_state = h.state.clone();
    let worker_task =
        tokio::spawn(async move { worker::run_once(&working_state, "model").await.unwrap() });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while provider.calls.load(Ordering::SeqCst) == calls_before {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let in_flight = feed(&h, &owner, &base).await;
    assert_eq!(
        input(&in_flight, &first_version)["learning"]["state"],
        "running"
    );
    assert_eq!(
        input(&in_flight, &first_version)["learning"]["job"]["state"],
        "running"
    );
    assert_eq!(
        input(&in_flight, &first_version)["actor_id"],
        json!(writer_id)
    );
    assert!(worker_task.await.unwrap());
    provider.delay.store(0, Ordering::SeqCst);
    let completed = feed(&h, &owner, &base).await;
    let result = input(&completed, &first_version);
    assert_eq!(result["learning"]["id"], run["id"]);
    assert_eq!(result["learning"]["state"], "succeeded");
    assert_eq!(result["learning"]["claim_ids"], json!([]));
    assert_eq!(result["actor_id"], json!(writer_id));
    assert_eq!(result["learning"]["accepted"], 0);
    assert!(input(&completed, &second_version)["learning"].is_null());
    // Older job retry beats a newer terminal job for the SAME version.
    let newer = Uuid::new_v4();
    sqlx::query("INSERT INTO jobs(id,brain_id,actor_id,audit_id,target_id,kind,lane,state,created_at) SELECT $2,brain_id,actor_id,audit_id,target_id,kind,lane,'succeeded',clock_timestamp() FROM jobs WHERE id=$1").bind(first_job).bind(newer).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE jobs SET state='failed',error_code='fixture_failure' WHERE id=$1")
        .bind(first_job)
        .execute(&h.admin)
        .await
        .unwrap();
    ok(
        &h,
        "POST",
        &format!("{base}/jobs/{first_job}/retry"),
        &owner,
        Value::Null,
    )
    .await;
    let retry = feed(&h, &owner, &base).await;
    assert_eq!(
        input(&retry, &first_version)["processing_job"]["id"],
        json!(first_job)
    );
    assert_eq!(
        input(&retry, &first_version)["processing_job"]["state"],
        "queued"
    );
    // Unrelated Brain job is never learning, including for manual imports.
    let manual = source(&h, &owner, &base, "Manual source proof.").await;
    let manual_feed = feed(&h, &owner, &base).await;
    let m = input(&manual_feed, &manual["version"]["id"]);
    assert!(m["capture_id"].is_null() && m["device_id"].is_null() && m["learning"].is_null());
    // Published source-free lifecycle events remain explicit and have no jobs.
    let mut lifecycle = event(&second_binding, "lifecycle", "");
    lifecycle["event"]["kind"] = json!("lifecycle");
    lifecycle["event"]["host_event"] = json!("SessionStart");
    lifecycle["event"]["content"] = Value::Null;
    let lifecycle = device_ok(
        &h,
        "POST",
        &format!("{base}/capture/events"),
        &second_token,
        lifecycle,
    )
    .await;
    let snapshot = feed(&h, &owner, &base).await;
    let row = snapshot["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == lifecycle["event_id"])
        .unwrap();
    assert!(
        row["source_version_id"].is_null()
            && row["processing_job"].is_null()
            && row["learning"].is_null()
    );
    // A late finish, not run creation order, defines the last terminal outcome.
    let older_run: Uuid = serde_json::from_value(run["id"].clone()).unwrap();
    let newer_run = Uuid::new_v4();
    let newer_job = Uuid::new_v4();
    sqlx::query("INSERT INTO jobs(id,brain_id,actor_id,audit_id,target_id,kind,lane,state) SELECT $2,brain_id,actor_id,audit_id,$3,kind,lane,'succeeded' FROM jobs WHERE id=$1").bind(Uuid::parse_str(run["job_id"].as_str().unwrap()).unwrap()).bind(newer_job).bind(newer_run).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO learning_runs(id,brain_id,source_version_id,policy_id,actor_id,selection,job_id,state,created_at,finished_at) SELECT $2,brain_id,source_version_id,policy_id,actor_id,selection,$3,'succeeded',clock_timestamp(),clock_timestamp()-interval '1 second' FROM learning_runs WHERE id=$1").bind(older_run).bind(newer_run).bind(newer_job).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE learning_runs SET finished_at=clock_timestamp() WHERE id=$1")
        .bind(older_run)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        input(&feed(&h, &owner, &base).await, &first_version)["learning"]["id"],
        run["id"]
    );
    // Member sees published contribution, but not another account's device alias.
    let shared = feed(&h, &writer, &base).await;
    assert!(input(&shared, &second_version)["agent_name"].is_null());
    assert_eq!(input(&shared, &second_version)["host"], "codex");
    let (_, outsider) = h.fixture_member().await;
    assert_eq!(
        h.call(
            "GET",
            &format!("{base}/pipeline"),
            Some(&outsider),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    ok(
        &h,
        "DELETE",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        h.call(
            "GET",
            &format!("{base}/pipeline"),
            Some(&writer),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // A bounded list keeps old active inputs ahead of newer completed inputs.
    for _ in 0..32 {
        source(&h, &owner, &base, "Recent bounded-window source.").await;
    }
    sqlx::query("UPDATE jobs SET state='succeeded',updated_at=clock_timestamp() WHERE brain_id=$1 AND kind='source.process' AND id<>$2").bind(Uuid::parse_str(base.rsplit('/').next().unwrap()).unwrap()).bind(first_job).execute(&h.admin).await.unwrap();
    let bounded = feed(&h, &owner, &base).await;
    assert_eq!(bounded["items"].as_array().unwrap().len(), 30);
    assert_eq!(bounded["has_more"], true);
    assert_eq!(bounded["items"][0]["source_version_id"], first_version);
    let observed = DateTime::parse_from_rfc3339(bounded["observed_at"].as_str().unwrap()).unwrap();
    let valid = DateTime::parse_from_rfc3339(bounded["valid_until"].as_str().unwrap()).unwrap();
    assert!(valid - observed <= Duration::seconds(6));
    // Retention removal excludes the source even if its processing job is active.
    let version: Uuid = serde_json::from_value(first_version.clone()).unwrap();
    sqlx::query("UPDATE source_versions SET created_at=now()-interval '400 days' WHERE id=$1")
        .bind(version)
        .execute(&h.admin)
        .await
        .unwrap();
    let expired = feed(&h, &owner, &base).await;
    assert!(
        expired["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["source_version_id"] != first_version)
    );
    let version: Uuid = serde_json::from_value(second_version.clone()).unwrap();
    sqlx::query("UPDATE source_versions SET privacy_state='erased' WHERE id=$1")
        .bind(version)
        .execute(&h.admin)
        .await
        .unwrap();
    let erased = feed(&h, &owner, &base).await;
    assert!(
        erased["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["source_version_id"] != second_version)
    );
    server.abort();
    h.finish().await;
}
