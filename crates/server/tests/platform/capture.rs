use super::*;
use chrono::{Duration, Utc};
use recollect_protocol::*;
use recollect_server::{autonomous, privacy_journal};

#[path = "capture_reconciliation.rs"]
mod reconciliation;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn native_setup_uploader_recovers_lost_ack_and_syncs_erasure_before_replay() {
    use recollect_agent::{
        Client, StoredDevice,
        capture::{Inbox, profile_root},
        capture_cli, capture_delivery,
        capture_setup::{self, SetupOptions},
        publication,
    };
    use std::sync::atomic::AtomicBool;
    let (h, owner, base, p, provider_server) = setup().await;
    capture_policy(&h, &owner, &base, true).await;
    let (device, token) = h.pair_device(&owner, "Automatic uploader fixture").await;
    let lose_response = Arc::new(AtomicBool::new(true));
    let lost = lose_response.clone();
    let router = h.router.clone().layer(axum::middleware::from_fn(
        move |request: Request<Body>, next: axum::middleware::Next| {
            let lost = lost.clone();
            async move {
                let capture = request.method() == axum::http::Method::POST
                    && request.uri().path().ends_with("/capture/events");
                let response = next.run(request).await;
                if capture
                    && response.status() == StatusCode::OK
                    && lost.swap(false, Ordering::SeqCst)
                {
                    // Canonical commit succeeded; the caller never sees its receipt.
                    axum::response::Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .body(Body::from("synthetic-untrusted-error-body-never-forward"))
                        .unwrap()
                } else {
                    response
                }
            }
        },
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let api_server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = Client::new(&endpoint).unwrap();
    let stored = StoredDevice {
        endpoint: endpoint.clone(),
        device_id: device,
        token: token.parse().unwrap(),
    };
    let root = publication::project_root()
        .join(".cache")
        .join(format!("capture-uploader-{}", Uuid::new_v4()));
    let work = root.join("workspace");
    let output = root.join("generated hooks");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir(&output).unwrap();
    std::fs::write(
        output.join("unrelated.txt"),
        "Preserve existing workspace configuration",
    )
    .unwrap();
    let brain = Uuid::parse_str(base.rsplit('/').next().unwrap()).unwrap();
    let prepared = capture_setup::prepare(
        &client,
        &stored,
        SetupOptions {
            host: "codex".into(),
            host_version: "0.154.0".into(),
            directory: work,
            brain: Some(brain),
            task: None,
            agent_id: None,
            output: Some(output.clone()),
            executable: publication::project_root().join("target/debug/recollect-agent"),
            device_profile: "unused-uploader-fixture".into(),
            evidence_root: root.join("evidence"),
        },
    )
    .await
    .unwrap();
    assert_eq!(prepared.state, "configured_only");
    assert!(prepared.capture_enabled);
    assert_eq!(
        std::fs::read_to_string(output.join("unrelated.txt")).unwrap(),
        "Preserve existing workspace configuration"
    );
    let setup = capture_cli::read_setup(&prepared.setup_file).await.unwrap();
    assert_eq!(setup.brain_id, brain);
    assert!(setup.created_task);
    let (renewed_path, setup) = capture_setup::refresh_version(
        &client,
        &stored,
        &prepared.setup_file,
        &setup,
        "0.154.1",
        &publication::project_root().join("target/debug/recollect-agent"),
    )
    .await
    .unwrap();
    assert_ne!(setup.binding_id, prepared.binding.id);
    let renewed = device_ok(
        &h,
        "GET",
        &format!("{base}/capture/bindings"),
        &token,
        Value::Null,
    )
    .await;
    assert_eq!(renewed.as_array().unwrap().len(), 2);
    assert_eq!(
        renewed[0]["operation"]["id"],
        json!(prepared.binding.operation.id)
    );
    let same = capture_setup::refresh_version(
        &client,
        &stored,
        &prepared.setup_file,
        &capture_cli::read_setup(&prepared.setup_file).await.unwrap(),
        "0.154.1",
        &publication::project_root().join("target/debug/recollect-agent"),
    )
    .await
    .unwrap();
    assert_eq!(same.0, renewed_path);
    assert_eq!(same.1.binding_id, setup.binding_id);
    assert!(
        prepared
            .binding
            .operation
            .scope
            .selection
            .area_ids
            .is_empty()
    );
    let configuration = std::fs::read_to_string(&prepared.setup_file).unwrap();
    assert!(!configuration.contains(&token));
    let configured = ok(
        &h,
        "GET",
        &format!("{base}/capture/devices"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(configured["total"], 1);
    assert!(configured["items"][0]["reported_at"].is_null());
    assert!(configured["items"][0]["last_publication"].is_null());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for file in [&prepared.setup_file, &prepared.hooks_file] {
            assert_eq!(
                std::fs::metadata(file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    let mut inbox = Inbox::open(
        &profile_root(&setup.evidence_root, device),
        &endpoint,
        device,
    )
    .unwrap();
    let raw = |turn: &str, text: &str| {
        serde_json::to_vec(&json!({"hook_event_name":"UserPromptSubmit","session_id":"uploader-session","turn_id":turn,"prompt":text})).unwrap()
    };
    let first = inbox
        .capture(
            setup.binding_id,
            &raw("first", "AUTOMATIC_UPLOAD_SENTINEL"),
            &[],
            Utc::now(),
        )
        .unwrap();
    let initial = capture_delivery::run_once(&client, &stored, &setup.evidence_root, Some(brain))
        .await
        .unwrap();
    assert!(initial.connected);
    assert_eq!(initial.published, 0);
    assert_eq!(initial.inbox.pending, 1);
    assert!(
        initial
            .failures
            .iter()
            .any(|(_, c)| *c == "server_unavailable")
    );
    assert!(
        !serde_json::to_string(&initial)
            .unwrap()
            .contains("synthetic-untrusted-error-body-never-forward")
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM capture_events WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let partial = ok(
        &h,
        "GET",
        &format!("{base}/capture/devices"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(partial["items"][0]["report"]["pending"], 1);
    assert_eq!(partial["items"][0]["report"]["issue"], "server_unavailable");
    assert!(
        partial["items"][0]["last_publication"].is_string(),
        "Server receipt is independent of the companion's lost acknowledgment"
    );
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let recovered = capture_delivery::run_once(&client, &stored, &setup.evidence_root, Some(brain))
        .await
        .unwrap();
    assert_eq!(recovered.published, 1);
    assert_eq!(recovered.inbox.pending, 0);
    assert!(recovered.failures.is_empty());
    let connected = ok(
        &h,
        "GET",
        &format!("{base}/capture/devices"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(connected["items"][0]["report"]["pending"], 0);
    assert!(connected["items"][0]["report"]["issue"].is_null());
    let (invalid, _) = h.bearer("POST",&format!("{base}/capture/devices"),&token,
        json!({"pending":0,"denied":0,"device_gap_count":0,"issue":"arbitrary-raw-error-must-not-be-stored"})).await;
    assert_eq!(invalid, StatusCode::BAD_REQUEST);
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/capture/devices"),
            Some(&owner),
            json!({"pending":0,"denied":0,"device_gap_count":0,"issue":null})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM capture_events WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(first)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(count, 1, "Retry reused the existing canonical capture");
    let erased_id = inbox
        .capture(
            setup.binding_id,
            &raw("erased", "UPLOADER_ERASURE_SENTINEL"),
            &[],
            Utc::now(),
        )
        .unwrap();
    lose_response.store(true, Ordering::SeqCst);
    let lost = capture_delivery::run_once(&client, &stored, &setup.evidence_root, Some(brain))
        .await
        .unwrap();
    assert_eq!(lost.inbox.pending, 1);
    let source: Uuid = sqlx::query_scalar("SELECT source_id FROM capture_events WHERE id=$1")
        .bind(erased_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    erase(&h, &owner, &base, &json!(source)).await;
    let independent = inbox
        .capture(
            setup.binding_id,
            &raw("independent", "INDEPENDENT_UPLOADER_SENTINEL"),
            &[],
            Utc::now(),
        )
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let synchronized =
        capture_delivery::run_once(&client, &stored, &setup.evidence_root, Some(brain))
            .await
            .unwrap();
    assert_eq!(synchronized.published, 1);
    assert_eq!(synchronized.inbox.pending, 0);
    assert_eq!(synchronized.inbox.removed, 1);
    let states: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id,state FROM capture_events WHERE id=ANY($1) ORDER BY id")
            .bind(vec![erased_id, independent])
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert!(states.contains(&(erased_id, "removed".into())));
    assert!(states.contains(&(independent, "accepted".into())));
    process(&h).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        0,
        "Uploader never calls models directly"
    );
    inbox
        .capture(
            setup.binding_id,
            &raw("offline", "OFFLINE_EXPIRY_SENTINEL"),
            &[],
            Utc::now() - Duration::days(31),
        )
        .unwrap();
    api_server.abort();
    let _ = api_server.await;
    let offline_client = Client::new(&endpoint).unwrap();
    let offline =
        capture_delivery::run_once(&offline_client, &stored, &setup.evidence_root, Some(brain))
            .await
            .unwrap();
    assert!(!offline.connected);
    assert_eq!(offline.inbox.pending, 0);
    assert_eq!(offline.inbox.removed, 2);
    let bytes =
        std::fs::read(profile_root(&setup.evidence_root, device).join("inbox.sqlite")).unwrap();
    for marker in [
        "UPLOADER_ERASURE_SENTINEL",
        "OFFLINE_EXPIRY_SENTINEL",
        "AUTOMATIC_UPLOAD_SENTINEL",
        "INDEPENDENT_UPLOADER_SENTINEL",
    ] {
        assert!(!bytes.windows(marker.len()).any(|w| w == marker.as_bytes()));
    }
    provider_server.abort();
    drop(inbox);
    std::fs::remove_dir_all(root).unwrap();
    h.finish().await;
}

async fn device_ok(h: &Harness, method: &str, path: &str, token: &str, body: Value) -> Value {
    let (status, result) = h.bearer(method, path, token, body).await;
    assert_eq!(status, StatusCode::OK, "{method} {path}: {result}");
    result
}
async fn capture_policy(h: &Harness, owner: &Login, base: &str, enabled: bool) -> Value {
    let current = ok(
        h,
        "GET",
        &format!("{base}/capture/policy"),
        owner,
        Value::Null,
    )
    .await;
    let mut policy = current["policy"].clone();
    policy["enabled"] = json!(enabled);
    ok(
        h,
        "PUT",
        &format!("{base}/capture/policy"),
        owner,
        json!({"base_change":current["change_id"],"policy":policy}),
    )
    .await
}
async fn binding(h: &Harness, base: &str, token: &str, selection: Value) -> Value {
    let task = device_ok(
        h,
        "POST",
        &format!("{base}/workspace/tasks"),
        token,
        json!({"label":"Controlled capture task","selection":selection}),
    )
    .await;
    let operation = device_ok(
        h,
        "POST",
        &format!(
            "{base}/workspace/tasks/{}/operations",
            task["task"]["id"].as_str().unwrap()
        ),
        token,
        json!({"kind":"capture"}),
    )
    .await;
    device_ok(h,"POST",&format!("{base}/capture/bindings"),token,json!({"id":Uuid::new_v4(),"operation_id":operation["id"],"host":"codex","host_version":"0.154.0"})).await
}
fn event(binding: &Value, turn: &str, content: &str) -> Value {
    json!({"id":Uuid::new_v4(),"binding_id":binding["id"],"event":{
        "host_event":"UserPromptSubmit","host_session_id":"synthetic-session","turn_id":turn,"agent_id":null,
        "tool_use_id":null,"tool_name":null,"kind":"prompt","outcome":"reported","content":content,
        "coverage":["partial_host_coverage"],"captured_at":Utc::now()}})
}
async fn process(h: &Harness) {
    while worker::run_once(&h.state, "capture").await.unwrap() {}
}
async fn erase(h: &Harness, owner: &Login, base: &str, source: &Value) -> Value {
    let target = json!({"kind":"source","id":source});
    let preview = ok(
        h,
        "POST",
        &format!("{base}/erasures/preview"),
        owner,
        target.clone(),
    )
    .await;
    assert_eq!(preview["capture_event_fences"], 1);
    ok(
        h,
        "POST",
        &format!("{base}/erasures"),
        owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn capture_admission_replay_original_scope_and_standing_learning() {
    let (h, owner, base, p, server) = setup().await;
    let policy_url = format!("{base}/capture/policy");
    let publish_url = format!("{base}/capture/events");
    let original = ok(&h, "GET", &policy_url, &owner, Value::Null).await;
    assert_eq!(original["policy"]["enabled"], false);
    let (writer_id, writer) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        json!({"role":"writer"}),
    )
    .await;
    let (device, token) = h.pair_device(&writer, "Capture writer").await;
    let (_, other_token) = h.pair_device(&owner, "Different companion").await;
    let area = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Synthetic service"}),
    )
    .await;
    let selection = json!({"repository_ids":[],"area_ids":[area["id"]],"environment_id":null});
    let bound = binding(&h, &base, &token, selection.clone()).await;
    let input = event(
        &bound,
        "turn-original",
        "Amber listens on port 8080.\nsynthetic-provider-credential\nAPI_KEY=synthetic-assignment\n",
    );
    assert_eq!(
        h.bearer("POST", &publish_url, &token, input.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let enabled = capture_policy(&h, &owner, &base, true).await;
    assert_eq!(
        h.call(
            "PUT",
            &policy_url,
            Some(&writer),
            json!({"base_change":enabled["change_id"],"policy":enabled["policy"]})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.bearer(
            "PUT",
            &policy_url,
            &token,
            json!({"base_change":enabled["change_id"],"policy":enabled["policy"]})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call(
            "PUT",
            &policy_url,
            Some(&owner),
            json!({"base_change":original["change_id"],"policy":enabled["policy"]})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.call("POST", &publish_url, Some(&owner), input.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.bearer("POST", &publish_url, &other_token, input.clone())
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let receipt = device_ok(&h, "POST", &publish_url, &token, input.clone()).await;
    assert_eq!(receipt["state"], "accepted");
    assert_eq!(receipt["event_id"], input["id"]);
    assert!(!receipt["source_version_id"].is_null());
    let source_version: Uuid = receipt["source_version_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let (artifact,bytes,created,class,origin): (Uuid,i32,chrono::DateTime<Utc>,String,String) = sqlx::query_as("SELECT artifact_id,byte_length,created_at,retention_class,source_uri FROM source_versions WHERE id=$1")
        .bind(source_version).fetch_one(&h.admin).await.unwrap();
    assert_eq!(class, "raw_session");
    assert_eq!(
        origin,
        format!("recollect:capture:{}", input["id"].as_str().unwrap())
    );
    let captured: chrono::DateTime<Utc> =
        serde_json::from_value(input["event"]["captured_at"].clone()).unwrap();
    assert!((created - captured).num_microseconds().unwrap().abs() < 2);
    let text = recollect_server::artifacts::read(
        &h.state.config.artifact_dir,
        Uuid::parse_str(base.rsplit('/').next().unwrap()).unwrap(),
        artifact,
        bytes,
    )
    .await
    .unwrap();
    assert!(text.contains("8080"));
    assert!(!text.contains("synthetic-provider-credential"));
    assert!(!text.contains("synthetic-assignment"));
    let row: Value = sqlx::query_scalar("SELECT to_jsonb(e) FROM capture_events e WHERE id=$1")
        .bind(Uuid::parse_str(input["id"].as_str().unwrap()).unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(row["metadata"]["content"].is_null());
    assert!(
        !row.to_string().contains("8080"),
        "Text belongs to the canonical source"
    );
    assert_eq!(
        device_ok(&h, "POST", &publish_url, &token, input.clone()).await,
        receipt,
        "Lost response retry"
    );
    let mut host_retry = input.clone();
    host_retry["id"] = json!(Uuid::new_v4());
    host_retry["event"]["captured_at"] = json!(Utc::now());
    assert_eq!(
        device_ok(&h, "POST", &publish_url, &token, host_retry).await,
        receipt
    );
    let mut changed = input.clone();
    changed["event"]["content"] = json!("Different text under the same identity");
    assert_eq!(
        h.bearer("POST", &publish_url, &token, changed).await.0,
        StatusCode::CONFLICT
    );
    let task = bound["operation"]["task_id"].as_str().unwrap();
    device_ok(&h,"PUT",&format!("{base}/workspace/tasks/{task}/scope"),&token,json!({"base_scope":bound["operation"]["scope"]["id"],"selection":{"repository_ids":[],"area_ids":[],"environment_id":null}})).await;
    device_ok(
        &h,
        "POST",
        &format!("{base}/workspace/tasks/{task}/close"),
        &token,
        Value::Null,
    )
    .await;
    assert_eq!(h.bearer("POST",&format!("{base}/capture/bindings"),&token,json!({"id":Uuid::new_v4(),"operation_id":bound["operation"]["id"],"host":"codex","host_version":"0.154.0"})).await.0,StatusCode::CONFLICT);
    process(&h).await;
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        0,
        "Capture permission does not allow model transmission"
    );
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
        policy["content_classes"] = json!(["raw_session", "claim", "query"]);
    })
    .await;
    *p.candidates.lock().unwrap() =
        json!({"claims":[candidate("Amber","port","8080",1)],"retirements":[]});
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    model_job(&h).await;
    let learned = runs(&h, &owner, &base).await;
    assert_eq!(learned["items"][0]["state"], "succeeded", "{learned}");
    assert_eq!(learned["items"][0]["selection"], selection);
    assert_eq!(learned["items"][0]["accepted"], 1);
    let claim = ok(
        &h,
        "GET",
        &format!(
            "{base}/claims/{}",
            learned["items"][0]["claim_ids"][0].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        claim["selected"]["revision"]["content"]["selection"],
        selection
    );
    assert!(claim["selected"]["revision"]["reviewer_id"].is_null());
    assert!(claim["selected"]["revision"]["review_decision_id"].is_null());
    // Updating this source keeps its original captured scope through automatic
    // revision; a new source-version UUID must not silently mean Brain-wide.
    ok(&h,"POST",&format!("{base}/sources/{}/versions",receipt["source_id"].as_str().unwrap()),&owner,
        json!({"base_version":receipt["source_version_id"],"title":"Updated captured declaration","media_type":"text/plain","retain_content":true,"content":"Amber now listens on port 9090, replacing its earlier port 8080 declaration.\n"})).await;
    process(&h).await;
    let mut replacement = candidate("Amber", "port", "9090", 1);
    replacement["replaces_revision"] = claim["selected"]["revision"]["id"].clone();
    *p.candidates.lock().unwrap() = json!({"claims":[replacement],"retirements":[]});
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap(),
        1
    );
    model_job(&h).await;
    let revised = runs(&h, &owner, &base).await;
    assert_eq!(revised["items"][0]["state"], "succeeded", "{revised}");
    assert_eq!(revised["items"][0]["selection"], selection);
    assert_eq!(revised["items"][0]["revised"], 1);
    assert_eq!(
        revised["items"][0]["claim_ids"][0],
        learned["items"][0]["claim_ids"][0]
    );
    let activity = ok(&h, "GET", &publish_url, &owner, Value::Null).await;
    assert_eq!(activity["total"], 1);
    assert_eq!(activity["items"][0]["selection"], selection);
    assert_eq!(activity["items"][0]["source_available"], true);
    let mut policy = enabled["policy"].clone();
    policy["enabled"] = json!(false);
    policy["excluded_content"] = json!(["Amber"]);
    ok(
        &h,
        "PUT",
        &policy_url,
        &owner,
        json!({"base_change":enabled["change_id"],"policy":policy}),
    )
    .await;
    assert_eq!(
        device_ok(&h, "POST", &publish_url, &token, input.clone()).await,
        receipt,
        "Policy changes cannot break committed receipt replay"
    );
    assert_eq!(
        h.bearer(
            "POST",
            &publish_url,
            &token,
            event(&bound, "disabled-turn", "Never retained")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    capture_policy(&h, &owner, &base, true).await;
    let marker = event(&bound, "late-turn", "Independent scoped content");
    let late = device_ok(&h, "POST", &publish_url, &token, marker).await;
    assert_eq!(
        late["state"], "accepted",
        "Closed task can drain a previous binding"
    );
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{base}/capture/devices"),
            &token,
            json!({"pending":0,"denied":1,"device_gap_count":0,"issue":"capture_denied"})
        )
        .await
        .0,
        StatusCode::NO_CONTENT,
        "A paired reader can report its own denied upload, while evidence publication remains forbidden"
    );
    assert_eq!(
        h.bearer(
            "POST",
            &publish_url,
            &token,
            event(&bound, "reader-turn", "Denied writer")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        device_ok(&h, "GET", &policy_url, &token, Value::Null).await["policy"]["enabled"],
        true
    );
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        json!({"role":"writer"}),
    )
    .await;
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/devices/{device}"),
            Some(&writer),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.bearer("POST", &publish_url, &token, input).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn capture_erasure_expiry_native_sync_and_journal_replay() {
    use recollect_agent::{
        Client, StoredDevice,
        capture::{CachedCaptureBinding, Inbox, profile_root},
        privacy as native,
    };
    let (h, owner, base, p, provider_server) = setup().await;
    capture_policy(&h, &owner, &base, true).await;
    let (device, token) = h.pair_device(&owner, "Capture privacy companion").await;
    let bound = binding(
        &h,
        &base,
        &token,
        json!({"repository_ids":[],"area_ids":[],"environment_id":null}),
    )
    .await;
    let brain: Uuid = bound["brain_id"].as_str().unwrap().parse().unwrap();
    let publish_url = format!("{base}/capture/events");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let api_server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = Client::new(&endpoint).unwrap();
    let stored = StoredDevice {
        endpoint: endpoint.clone(),
        device_id: device,
        token: token.parse().unwrap(),
    };
    let root = recollect_agent::publication::project_root()
        .join(".cache")
        .join(format!("capture-privacy-{}", Uuid::new_v4()));
    let inbox_root = profile_root(&root, device);
    let policy = CapturePolicy {
        enabled: true,
        ..Default::default()
    };
    let mut inbox = Inbox::open(&inbox_root, &endpoint, device).unwrap();
    let cached = CachedCaptureBinding {
        binding: serde_json::from_value(bound.clone()).unwrap(),
        policy: policy.clone(),
        retention: RetentionPolicy::default(),
        synchronized_at: Utc::now(),
        agent_id: None,
    };
    inbox.remember(&cached).unwrap();
    let raw = json!({"hook_event_name":"UserPromptSubmit","session_id":"privacy-session","turn_id":"erased-turn","prompt":"CAPTURE_ERASURE_SENTINEL"});
    let erased_id = inbox
        .capture(
            cached.binding.id,
            &serde_json::to_vec(&raw).unwrap(),
            &[],
            Utc::now(),
        )
        .unwrap();
    let pending = inbox.pending(Utc::now()).unwrap().remove(0);
    let accepted = device_ok(
        &h,
        "POST",
        &publish_url,
        &token,
        serde_json::to_value(&pending).unwrap(),
    )
    .await;
    // Lost server response leaves the native body pending until privacy sync.
    assert_eq!(inbox.status().unwrap().pending, 1);
    let independent_id = inbox.capture(cached.binding.id,&serde_json::to_vec(&json!({"hook_event_name":"UserPromptSubmit","session_id":"privacy-session","turn_id":"independent-turn","prompt":"INDEPENDENT_CAPTURE_SENTINEL"})).unwrap(),&[],Utc::now()).unwrap();
    let surviving = device_ok(
        &h,
        "POST",
        &publish_url,
        &token,
        event(&bound, "server-independent", "Independent central source"),
    )
    .await;
    let erased = erase(&h, &owner, &base, &accepted["source_id"]).await;
    assert_eq!(
        device_ok(
            &h,
            "POST",
            &publish_url,
            &token,
            serde_json::to_value(&pending).unwrap()
        )
        .await["state"],
        "removed"
    );
    let cleanup = native::synchronize(&client, &stored, brain, &root)
        .await
        .unwrap();
    assert!(cleanup.acknowledged);
    assert_eq!(cleanup.removed, 1);
    assert_eq!(
        inbox
            .pending(Utc::now())
            .unwrap()
            .iter()
            .map(|e| e.id)
            .collect::<Vec<_>>(),
        vec![independent_id]
    );
    inbox
        .acknowledge(&serde_json::from_value(accepted.clone()).unwrap())
        .unwrap();
    assert_eq!(
        inbox.status().unwrap().removed,
        1,
        "Late success cannot undo deletion"
    );
    assert!(
        inbox
            .capture(
                cached.binding.id,
                &serde_json::to_vec(&raw).unwrap(),
                &[],
                Utc::now()
            )
            .is_err()
    );
    privacy_journal::maintain(&h.state).await.unwrap();
    let request: Uuid = erased["id"].as_str().unwrap().parse().unwrap();
    let manifest: Value = sqlx::query_scalar("SELECT manifest FROM privacy_requests WHERE id=$1")
        .bind(request)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        manifest["capture_event_fences"][0]["event_id"],
        json!(erased_id)
    );
    assert!(!manifest.to_string().contains("SENTINEL"));
    let metadata: (Option<Value>, Option<Value>) =
        sqlx::query_as("SELECT metadata,admission_policy FROM capture_events WHERE id=$1")
            .bind(erased_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(metadata, (None, None));
    let bytes = std::fs::read(inbox_root.join("inbox.sqlite")).unwrap();
    assert!(
        !bytes
            .windows(b"CAPTURE_ERASURE_SENTINEL".len())
            .any(|w| w == b"CAPTURE_ERASURE_SENTINEL")
    );
    assert!(
        bytes
            .windows(b"INDEPENDENT_CAPTURE_SENTINEL".len())
            .any(|w| w == b"INDEPENDENT_CAPTURE_SENTINEL")
    );
    // A real older database image predates the next captured event/binding.
    h.state.pool.close().await;
    h.admin.close().await;
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable capture backup database: {backup}");
    sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'")).execute(&h.root).await.unwrap();
    let mut admin_url = reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
    admin_url.set_path(&h.database);
    let mut h = Harness {
        state: AppState::new(
            db::pool(&h.state.config.database_url).await.unwrap(),
            (*h.state.config).clone(),
        )
        .unwrap(),
        admin: db::pool(admin_url.as_str()).await.unwrap(),
        ..h
    };
    h.router = app(h.state.clone());
    let fresh_input = json!({"id":Uuid::new_v4(),"operation_id":bound["operation"]["id"],"host":"codex","host_version":"0.154.0"});
    let fresh = device_ok(
        &h,
        "POST",
        &format!("{base}/capture/bindings"),
        &token,
        fresh_input.clone(),
    )
    .await;
    let future_input = event(&fresh, "future-erased", "FUTURE_CAPTURE_SENTINEL");
    let future = device_ok(&h, "POST", &publish_url, &token, future_input.clone()).await;
    erase(&h, &owner, &base, &future["source_id"]).await;
    privacy_journal::maintain(&h.state).await.unwrap();
    admin_url.set_path(&backup);
    let older_admin = db::pool(admin_url.as_str()).await.unwrap();
    let mut config = (*h.state.config).clone();
    let mut app_url = reqwest::Url::parse(&config.database_url).unwrap();
    app_url.set_path(&backup);
    config.database_url = app_url.to_string();
    privacy_journal::reconcile(&older_admin, &config)
        .await
        .unwrap();
    let older = AppState::new(db::pool(&config.database_url).await.unwrap(), config).unwrap();
    let fences: i64 =
        sqlx::query_scalar("SELECT count(*) FROM privacy_capture_fences WHERE event_id=$1")
            .bind(Uuid::parse_str(future_input["id"].as_str().unwrap()).unwrap())
            .fetch_one(&older_admin)
            .await
            .unwrap();
    assert_eq!(
        fences, 1,
        "Journal must preserve identities absent from the backup"
    );
    // Recreate the original binding through normal device admission. The
    // journal fence must survive even though this binding was absent on replay.
    let rebound = app(older.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("{base}/capture/bindings"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(fresh_input.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rebound.status(), StatusCode::OK);
    let response = app(older.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&publish_url)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(future_input.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["state"], "removed");
    assert!(value["source_id"].is_null());
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM capture_events WHERE id=$1")
        .bind(Uuid::parse_str(future_input["id"].as_str().unwrap()).unwrap())
        .fetch_one(&older_admin)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    older.pool.close().await;
    older_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup} WITH (FORCE)"))
        .execute(&h.root)
        .await
        .unwrap();
    // Expired arrivals never create artifacts, even after an offline delay.
    let bound_id: Uuid = bound["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE capture_bindings SET created_at=now()-interval '40 days' WHERE id=$1")
        .bind(bound_id)
        .execute(&h.admin)
        .await
        .unwrap();
    let mut expired = event(&bound, "offline-expired", "EXPIRED_ARRIVAL_SENTINEL");
    expired["event"]["captured_at"] = json!(Utc::now() - Duration::days(31));
    let expired_receipt = device_ok(&h, "POST", &publish_url, &token, expired.clone()).await;
    assert_eq!(expired_receipt["state"], "expired");
    assert!(expired_receipt["source_id"].is_null());
    let due: Value = sqlx::query_scalar("SELECT to_jsonb(e) FROM capture_events e WHERE id=$1")
        .bind(Uuid::parse_str(expired["id"].as_str().unwrap()).unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(due["metadata"].is_null());
    assert!(!due.to_string().contains("EXPIRED_ARRIVAL_SENTINEL"));
    let mut marker = event(&bound, "old-marker", "Lifecycle content must never persist");
    marker["event"]["host_event"] = json!("SessionEnd");
    marker["event"]["kind"] = json!("lifecycle");
    marker["event"]["captured_at"] = json!(Utc::now() - Duration::days(2));
    let marker_receipt = device_ok(&h, "POST", &publish_url, &token, marker.clone()).await;
    assert!(marker_receipt["source_id"].is_null());
    let mut tool = event(
        &bound,
        "old-tool",
        "{\"input\":{\"command\":\"inspect synthetic service\"},\"output\":{\"stdout\":\"TOOL_EXPIRY_SENTINEL\"}}",
    );
    tool["event"]["host_event"] = json!("PostToolUse");
    tool["event"]["kind"] = json!("tool_result");
    tool["event"]["tool_use_id"] = json!("old-tool-call");
    tool["event"]["tool_name"] = json!("Bash");
    tool["event"]["captured_at"] = json!(Utc::now() - Duration::days(2));
    let tool_receipt = device_ok(&h, "POST", &publish_url, &token, tool.clone()).await;
    assert!(!tool_receipt["source_id"].is_null());
    let settings = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut retention = settings["policy"].clone();
    retention["raw_session_days"] = json!(1);
    retention["tool_output_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":settings["change_id"],"policy":retention}),
    )
    .await;
    privacy_journal::run_once(&h.state).await.unwrap();
    for payload in [marker, tool] {
        let receipt = device_ok(&h, "POST", &publish_url, &token, payload).await;
        assert_eq!(receipt["state"], "expired");
        let clean: bool = sqlx::query_scalar(
            "SELECT metadata IS NULL AND admission_policy IS NULL FROM capture_events WHERE id=$1",
        )
        .bind(Uuid::parse_str(receipt["event_id"].as_str().unwrap()).unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap();
        assert!(clean);
    }
    let activity = ok(&h, "GET", &publish_url, &owner, Value::Null).await;
    assert!(
        activity["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["receipt"]["event_id"] == surviving["event_id"]
                && e["source_available"] == true)
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    api_server.abort();
    provider_server.abort();
    drop(inbox);
    std::fs::remove_dir_all(root).unwrap();
    h.finish().await;
}
