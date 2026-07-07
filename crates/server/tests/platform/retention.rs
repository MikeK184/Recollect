use super::review::{ok, proposal};
use super::*;
use recollect_server::privacy_journal;

async fn bearer_ok(h: &Harness, method: &str, path: &str, token: &str, body: Value) -> Value {
    let reply = h.bearer(method, path, token, body).await;
    assert_eq!(reply.0, StatusCode::OK, "{method} {path}: {}", reply.1);
    reply.1
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn snapshot_manifest_erasure_and_native_privacy_sync_are_scoped() {
    use recollect_agent::{Client, StoredDevice, privacy as native, publication as local};
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic native erasure proof"}),
    )
    .await;
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain_id}");
    let (device_id, token) = h.pair_device(&owner, "Privacy companion").await;
    bearer_ok(&h,"POST",&format!("{base}/workspace/checkouts"),&token,json!({"workspace_root":"/fixture","complete":false,"notes":[],"checkouts":[{"local_path":"/fixture/repo","origin":"https://example.test/team/fixture.git","head":null,"branch":"main","dirty":true,"status":"available"}]})).await;
    let repo=bearer_ok(&h,"GET",&format!("{base}/workspace"),&token,Value::Null).await["repositories"][0]["id"].clone();
    let repo_id: Uuid = repo.as_str().unwrap().parse().unwrap();
    ok(
        &h,
        "PUT",
        &format!("{base}/repositories/policy"),
        &owner,
        json!({"allow_file_content":true}),
    )
    .await;
    let op = super::publication::begin(&h, &base, &token, &repo).await;
    let payload = super::publication::input(&repo, &op, &"a".repeat(40));
    let url = format!("{base}/repositories/{repo_id}/snapshots");
    let published = bearer_ok(&h, "POST", &url, &token, payload.clone()).await;
    let sid = published["snapshot"]["id"].as_str().unwrap();
    let op2 = super::publication::begin(&h, &base, &token, &repo).await;
    let mut second = super::publication::input(&repo, &op2, &"b".repeat(40));
    // This bundle is the retained control when a one-day offline TTL is set
    // below. Its age must not depend on the calendar date of a static fixture.
    second["captured_at"] = json!(chrono::Utc::now().to_rfc3339());
    let independent = bearer_ok(&h, "POST", &url, &token, second.clone()).await;
    while worker::run_once(&h.state, "heavy").await.unwrap() {}
    let fact = ok(
        &h,
        "GET",
        &format!("{base}/repository-snapshots/{sid}/facts"),
        &owner,
        Value::Null,
    )
    .await["items"][0]["id"]
        .clone();
    let env = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Synthetic environment"}),
    )
    .await["id"]
        .clone();
    let manifest=ok(&h,"POST",&format!("{base}/revision-manifests"),&owner,json!({"name":"Manifest with removed snapshot","environment_id":env,"kind":"committed","entries":[{"repository_id":repo,"revision":"a".repeat(40),"snapshot_id":sid,"config_paths":["deploy/removed.yaml"]}],"notes":"Controlled synthetic manifest note","base_revision":null,"operation_id":null})).await;
    let mut p = proposal(&fact, "Snapshot dependent", "static declaration");
    p["content"]["selection"] = json!({"repository_ids":[repo],"area_ids":[],"environment_id":env});
    p["content"]["supports"] =
        json!([{"kind":"repository_fact","id":fact,"line_from":null,"line_to":null}]);
    p["content"]["manifest_revision_id"] = manifest["id"].clone();
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, p).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = Client::new(&endpoint).unwrap();
    let device = StoredDevice {
        endpoint: endpoint.clone(),
        device_id,
        token: token.parse().unwrap(),
    };
    let root = local::project_root()
        .join(".cache")
        .join(format!("privacy-bundles-{}", Uuid::new_v4()));
    let make_bundle = |input: Value, brain, device| local::PreparedPublication {
        endpoint: endpoint.clone(),
        device_id: device,
        brain_id: brain,
        repository_id: repo_id,
        input: serde_json::from_value(input).unwrap(),
    };
    let mut erased_payload = payload.clone();
    erased_payload["publication_id"] = json!(Uuid::new_v4());
    let erased = make_bundle(erased_payload, brain_id, device_id);
    let erased_path = local::save_bundle(&root, &erased).await.unwrap();
    let mut surviving_payload = second.clone();
    surviving_payload["publication_id"] = json!(Uuid::new_v4());
    let surviving = make_bundle(surviving_payload, brain_id, device_id);
    let surviving_path = local::save_bundle(&root, &surviving).await.unwrap();
    let mut foreign_payload = payload.clone();
    foreign_payload["publication_id"] = json!(Uuid::new_v4());
    let foreign_bundle = make_bundle(foreign_payload, Uuid::new_v4(), device_id);
    let foreign_path = local::save_bundle(&root, &foreign_bundle).await.unwrap();
    let mut other_device_payload = payload.clone();
    other_device_payload["publication_id"] = json!(Uuid::new_v4());
    let other_device = make_bundle(other_device_payload, brain_id, Uuid::new_v4());
    let other_path = local::save_bundle(&root, &other_device).await.unwrap();
    let before = native::synchronize(&client, &device, brain_id, &root)
        .await
        .unwrap();
    assert_eq!(before.removed, 0);
    let target = json!({"kind":"snapshot","id":sid});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    assert_eq!(preview["snapshots"], 1);
    assert_eq!(preview["manifest_revisions"], 1);
    assert_eq!(preview["claim_revisions"], 1);
    let erased_request = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    privacy_journal::maintain(&h.state).await.unwrap();
    assert_eq!(
        h.call(
            "GET",
            &format!("{base}/repository-snapshots/{sid}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::GONE
    );
    assert_eq!(
        h.call(
            "GET",
            &format!(
                "{base}/revision-manifests/{}",
                manifest["manifest_id"].as_str().unwrap()
            ),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::GONE
    );
    let view = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", claim["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(view["selection_state"], "erased");
    let mut replay = payload.clone();
    replay["publication_id"] = json!(Uuid::new_v4());
    replay["adapter_build"] = json!("different adapter after erase");
    assert_eq!(
        h.bearer("POST", &url, &token, replay).await.0,
        StatusCode::GONE,
        "New adapter and publication IDs cannot bypass an erased commit"
    );
    ok(
        &h,
        "GET",
        &format!(
            "{base}/repository-snapshots/{}",
            independent["snapshot"]["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    let after = native::synchronize(&client, &device, brain_id, &root)
        .await
        .unwrap();
    assert_eq!(after.removed, 1);
    assert_eq!(after.retained, 1);
    assert!(after.acknowledged);
    assert!(!erased_path.exists());
    assert!(surviving_path.exists());
    assert!(foreign_path.exists());
    assert!(other_path.exists());
    let status = ok(&h, "GET", &format!("{base}/erasures"), &owner, Value::Null).await;
    assert_eq!(status["items"][0]["acknowledged_devices"], 1);
    assert_eq!(after.sequence, erased_request["sequence"].as_i64().unwrap());
    let too_far = h
        .bearer(
            "POST",
            &format!("{base}/privacy-sync"),
            &token,
            json!({"sequence":after.sequence+1}),
        )
        .await;
    assert_eq!(too_far.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        h.call(
            "GET",
            &format!("{base}/privacy-sync"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Known local expiry remains enforceable without a network connection.
    let mut cached = native::cached(&root, &endpoint, device_id, brain_id)
        .await
        .unwrap()
        .unwrap();
    cached.sync.policy.repository_days = Some(1);
    native::save(&root, &cached).await.unwrap();
    let mut old_payload = second;
    old_payload["publication_id"] = json!(Uuid::new_v4());
    old_payload["captured_at"] =
        json!((chrono::Utc::now() - chrono::Duration::days(2)).to_rfc3339());
    let old = make_bundle(old_payload, brain_id, device_id);
    let old_path = local::save_bundle(&root, &old).await.unwrap();
    server.abort();
    let _ = server.await;
    let offline = native::cleanup_known(&client, &device, brain_id, &root)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(offline.removed, 1);
    assert!(!offline.acknowledged);
    assert!(!old_path.exists());
    assert!(surviving_path.exists());
    assert!(foreign_path.exists());
    assert!(other_path.exists());
    tokio::fs::remove_dir_all(root).await.unwrap();
    h.finish().await;
}
use recollect_server::{artifacts, worker};

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn erasure_storage_retry_revoked_initiator_and_older_database_restore() {
    let mut h = Harness::new().await;
    let owner = h.login().await;
    let (admin_id, initiator) = h.fixture_member().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic restore proof"}),
    )
    .await;
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain_id}");
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{admin_id}"),
        &owner,
        json!({"role":"admin"}),
    )
    .await;
    let a = test_source(&h, &base, &owner, "Synthetic erased backup content").await;
    let version: Uuid = a["version"]["id"].as_str().unwrap().parse().unwrap();
    let artifact: Uuid = sqlx::query_scalar("SELECT artifact_id FROM source_versions WHERE id=$1")
        .bind(version)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let artifact_path = artifacts::path(&h.state.config.artifact_dir, brain_id, artifact);
    let original_bytes = std::fs::read(&artifact_path).unwrap();
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(&a["version"]["id"], "Restored claim", "private value"),
    )
    .await;
    privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    // Make an actual PostgreSQL copy from the state before erasure.
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable older-database fixture: {backup}");
    h.state.pool.close().await;
    h.admin.close().await;
    sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'")).execute(&h.root).await.unwrap();
    let admin_url = std::env::var("DATABASE_ADMIN_URL").unwrap();
    let mut url = reqwest::Url::parse(&admin_url).unwrap();
    url.set_path(&h.database);
    h.admin = db::pool(url.as_str()).await.unwrap();
    h.state = AppState::new(
        db::pool(&h.state.config.database_url).await.unwrap(),
        h.state.config.as_ref().clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    url.set_path(&backup);
    let backup_admin = db::pool(url.as_str()).await.unwrap();
    let mut backup_config = h.state.config.as_ref().clone();
    let mut url = reqwest::Url::parse(&backup_config.database_url).unwrap();
    url.set_path(&backup);
    backup_config.database_url = url.to_string();
    backup_config.artifact_dir = format!(".cache/{backup}-artifacts");
    artifacts::write(
        &backup_config.artifact_dir,
        brain_id,
        artifact,
        std::str::from_utf8(&original_bytes).unwrap(),
    )
    .await
    .unwrap();
    let backup_pool = db::pool(&backup_config.database_url).await.unwrap();
    let target = json!({"kind":"source","id":a["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &initiator,
        target.clone(),
    )
    .await;
    let request = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &initiator,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    ok(
        &h,
        "DELETE",
        &format!("{base}/grants/{admin_id}"),
        &owner,
        Value::Null,
    )
    .await;
    let installation: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let dir = std::path::Path::new(&h.state.config.erasure_journal).join(installation.to_string());
    let missing = dir.with_extension("unavailable");
    std::fs::rename(&dir, &missing).unwrap();
    assert!(
        privacy_journal::barrier(&h.state.pool, &h.state.config)
            .await
            .is_err()
    );
    privacy_journal::maintain(&h.state).await.unwrap();
    let status = ok(&h, "GET", &format!("{base}/erasures"), &owner, Value::Null).await;
    assert_eq!(status["items"][0]["error_code"], "journal_unavailable");
    assert_eq!(status["items"][0]["journaled"], false);
    assert!(
        artifact_path.exists(),
        "A failed export cannot be reported complete"
    );
    std::fs::rename(&missing, &dir).unwrap();
    let held = artifact_path.with_extension("held");
    std::fs::rename(&artifact_path, &held).unwrap();
    std::fs::create_dir(&artifact_path).unwrap();
    privacy_journal::maintain(&h.state).await.unwrap();
    let status = ok(&h, "GET", &format!("{base}/erasures"), &owner, Value::Null).await;
    assert_eq!(status["items"][0]["error_code"], "artifact_unavailable");
    assert_eq!(status["items"][0]["journaled"], true);
    assert_eq!(status["items"][0]["pending_artifacts"], 1);
    let journal =
        std::fs::read_to_string(dir.join(format!("{}.json", request["id"].as_str().unwrap())))
            .unwrap();
    assert!(!journal.contains("Synthetic erased backup content"));
    assert!(!journal.contains("private value"));
    assert!(!journal.contains(&h.state.config.artifact_dir));
    std::fs::remove_dir(&artifact_path).unwrap();
    std::fs::rename(&held, &artifact_path).unwrap();
    let status = ok(
        &h,
        "POST",
        &format!("{base}/erasures/{}/retry", request["id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(status["state"], "complete");
    assert_eq!(status["pending_artifacts"], 0);
    assert!(!artifact_path.exists());
    assert_eq!(status["local_copies"], "device_check_in_required");
    assert!(
        privacy_journal::barrier(&backup_pool, &backup_config)
            .await
            .is_err(),
        "Older database cannot serve before journal replay"
    );
    privacy_journal::reconcile(&backup_admin, &backup_config)
        .await
        .unwrap();
    privacy_journal::barrier(&backup_pool, &backup_config)
        .await
        .unwrap();
    let restored: Value = sqlx::query_scalar("SELECT revision FROM claim_revisions WHERE id=$1")
        .bind(claim["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .fetch_one(&backup_admin)
        .await
        .unwrap();
    assert_eq!(restored, json!({}));
    assert!(!artifacts::path(&backup_config.artifact_dir, brain_id, artifact).exists());
    let restored: String =
        sqlx::query_scalar("SELECT privacy_state FROM source_versions WHERE id=$1")
            .bind(version)
            .fetch_one(&backup_admin)
            .await
            .unwrap();
    assert_eq!(restored, "erased");
    // Even a complete DB plus a reintroduced artifact copy must reconcile at startup.
    artifacts::write(
        &h.state.config.artifact_dir,
        brain_id,
        artifact,
        std::str::from_utf8(&original_bytes).unwrap(),
    )
    .await
    .unwrap();
    privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    assert!(!artifact_path.exists());
    let path = dir.join(format!("{}.json", request["id"].as_str().unwrap()));
    let held = path.with_extension("held");
    std::fs::rename(&path, &held).unwrap();
    assert!(
        privacy_journal::barrier(&h.state.pool, &h.state.config)
            .await
            .is_err(),
        "Missing acknowledged entry fails closed"
    );
    std::fs::rename(&held, &path).unwrap();
    backup_pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup} WITH (FORCE)"))
        .execute(&h.root)
        .await
        .unwrap();
    std::fs::remove_dir_all(&backup_config.artifact_dir).unwrap();
    h.finish().await;
}

async fn test_source(h: &Harness, base: &str, owner: &Login, title: &str) -> Value {
    ok(h,"POST",&format!("{base}/sources"),owner,json!({"title":title,"media_type":"text/plain","content":format!("{title}\n"),"retain_content":true})).await
}
async fn revise(h: &Harness, base: &str, owner: &Login, old: &Value, version: &Value) -> Value {
    let mut input = proposal(
        version,
        old["content"]["subject"].as_str().unwrap(),
        old["content"]["value"].as_str().unwrap(),
    );
    input["base_revision"] = old["id"].clone();
    ok(
        h,
        "PUT",
        &format!("{base}/claims/{}", old["claim_id"].as_str().unwrap()),
        owner,
        input,
    )
    .await
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn erasure_closure_preserves_independent_revisions_and_history_gaps() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (reader_id, reader) = h.fixture_member().await;
    let (_, foreign) = h.fixture_member().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic erasure proof"}),
    )
    .await;
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain_id}");
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    let a = test_source(&h, &base, &owner, "Removed synthetic evidence").await;
    let b = test_source(&h, &base, &owner, "Independent synthetic evidence").await;
    let a_id = a["id"].as_str().unwrap();
    let av = &a["version"]["id"];
    let bv = &b["version"]["id"];
    let excerpt=ok(&h,"POST",&format!("{base}/excerpts"),&owner,json!({"source_id":a["id"],"version_id":av,"first_line":1,"last_line":1,"title":"Explicit excerpt of removed evidence"})).await;
    let old = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(bv, "Current gap", "alpha"),
    )
    .await;
    let gap = revise(&h, &base, &owner, &old, av).await;
    let dependent = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(&excerpt["version"]["id"], "Excerpt dependent", "beta"),
    )
    .await;
    let first = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(av, "Independent latest", "gamma"),
    )
    .await;
    let independent = revise(&h, &base, &owner, &first, bv).await;
    let rejected = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(av, "Rejected controlled value", "private synthetic value"),
    )
    .await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",rejected["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":rejected["id"],"action":"reject","reason":"Sensitive synthetic review text removed with its source.","content":null,"revalidation_basis":null})).await;
    let mut co = proposal(av, "Two co-supports", "delta");
    co["content"]["supports"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"source_version","id":bv,"line_from":1,"line_to":1}));
    let co = ok(&h, "POST", &format!("{base}/claims"), &owner, co).await;
    let target = json!({"kind":"source","id":a["id"]});
    let preview_url = format!("{base}/erasures/preview");
    assert_eq!(
        h.call("POST", &preview_url, Some(&reader), target.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert!(
        h.call("POST", &preview_url, Some(&foreign), target.clone())
            .await
            .0
            .is_client_error()
    );
    let preview = ok(&h, "POST", &preview_url, &owner, target.clone()).await;
    assert_eq!(preview["source_versions"], 2);
    assert_eq!(preview["independent_claim_revisions"], 2);
    // A mutation after preview changes the closure counter; confirmation cannot silently rebase.
    test_source(&h, &base, &owner, "Unrelated later source").await;
    let stale = json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]});
    assert_eq!(
        h.call("POST", &format!("{base}/erasures"), Some(&owner), stale)
            .await
            .0,
        StatusCode::CONFLICT
    );
    let preview = ok(&h, "POST", &preview_url, &owner, target.clone()).await;
    let input = json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]});
    let leased = worker::claim(&h.state.pool, "capture")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        leased.target_id,
        av.as_str().unwrap().parse::<Uuid>().unwrap()
    );
    sqlx::query("ALTER TABLE mutation_audit ADD CONSTRAINT erasure_audit_fault CHECK(action<>'memory.erase') NOT VALID").execute(&h.admin).await.unwrap();
    let failed = h
        .keyed(
            "POST",
            &format!("{base}/erasures"),
            Some(&owner),
            input.clone(),
            Some("erase-synthetic-source"),
        )
        .await;
    assert_eq!(failed.0, StatusCode::SERVICE_UNAVAILABLE);
    let (requests,active):(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM privacy_requests WHERE brain_id=$1),(SELECT count(*) FROM source_versions WHERE brain_id=$1 AND privacy_state='active')").bind(brain_id).fetch_one(&h.admin).await.unwrap();
    assert_eq!(requests, 0);
    assert!(active >= 3, "A failed audit must roll back all tombstones");
    sqlx::query("ALTER TABLE mutation_audit DROP CONSTRAINT erasure_audit_fault")
        .execute(&h.admin)
        .await
        .unwrap();
    let accepted = h
        .keyed(
            "POST",
            &format!("{base}/erasures"),
            Some(&owner),
            input.clone(),
            Some("erase-synthetic-source"),
        )
        .await;
    assert_eq!(accepted.0, StatusCode::OK, "{}", accepted.1);
    assert_eq!(accepted.1["state"], "pending");
    assert_eq!(accepted.1["journaled"], false);
    assert_eq!(
        worker::execute(&h.state, &leased).await.unwrap_err(),
        worker::Failure::LostLease
    );
    let replay = h
        .keyed(
            "POST",
            &format!("{base}/erasures"),
            Some(&owner),
            input,
            Some("erase-synthetic-source"),
        )
        .await;
    assert_eq!(replay.0, StatusCode::OK);
    assert_eq!(replay.1["id"], accepted.1["id"]);
    let erased = ok(
        &h,
        "GET",
        &format!("{base}/sources/{a_id}/versions/{}", av.as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(erased["version"]["privacy_state"], "erased");
    assert!(erased["content"].is_null());
    let url = format!("{base}/claims/{}", gap["claim_id"].as_str().unwrap());
    let view = ok(&h, "GET", &url, &owner, Value::Null).await;
    assert_eq!(view["current_revision"], gap["id"]);
    assert_eq!(view["selection_state"], "erased");
    assert!(view["selected"].is_null());
    assert_eq!(view["history"].as_array().unwrap().len(), 1);
    assert_eq!(view["unavailable_history"].as_array().unwrap().len(), 1);
    let at = old["recorded_at"].as_str().unwrap();
    let view = ok(
        &h,
        "GET",
        &format!("{url}?knowledge_at={at}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        view["selected"]["revision"]["id"], old["id"],
        "Independent historical interval remains inspectable"
    );
    for mode in ["investigation", "history"] {
        let latest = ok(
            &h,
            "POST",
            &format!("{base}/recall"),
            &owner,
            json!({"exact":{"kind":"claim","id":gap["claim_id"]},"mode":mode}),
        )
        .await;
        assert_eq!(
            latest["context"]["items"],
            json!([]),
            "Erased latest selection must not resurrect old claim text"
        );
        let historical = ok(
            &h,
            "POST",
            &format!("{base}/recall"),
            &owner,
            json!({"exact":{"kind":"claim","id":gap["claim_id"]},"knowledge_at":at,"mode":mode}),
        )
        .await;
        assert_eq!(historical["context"]["items"][0]["revision_id"], old["id"]);
        let control = ok(
            &h,
            "POST",
            &format!("{base}/recall"),
            &owner,
            json!({"exact":{"kind":"claim","id":independent["claim_id"]},"mode":mode}),
        )
        .await;
        assert_eq!(
            control["context"]["items"][0]["revision_id"],
            independent["id"]
        );
    }
    let list = ok(
        &h,
        "GET",
        &format!("{base}/claims?mode=history"),
        &owner,
        Value::Null,
    )
    .await;
    assert!(
        !list["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["revision"]["claim_id"] == gap["claim_id"])
    );
    assert!(
        list["unavailable"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["claim_id"] == gap["claim_id"])
    );
    let view = ok(
        &h,
        "GET",
        &format!(
            "{base}/claims/{}",
            independent["claim_id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(view["selected"]["revision"]["id"], independent["id"]);
    assert_eq!(view["selected"]["evidence"][0]["availability"], "retained");
    for r in [&dependent, &co, &rejected] {
        let view = ok(
            &h,
            "GET",
            &format!("{base}/claims/{}", r["claim_id"].as_str().unwrap()),
            &owner,
            Value::Null,
        )
        .await;
        assert_eq!(view["selection_state"], "erased");
    }
    let payloads: Vec<Value> =
        sqlx::query_scalar("SELECT rule FROM assertion_rules WHERE brain_id=$1")
            .bind(brain_id)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert!(!payloads.is_empty());
    assert!(payloads.iter().all(|v| *v == json!({})));
    let decisions: Vec<Value> =
        sqlx::query_scalar("SELECT decision FROM memory_decisions WHERE brain_id=$1")
            .bind(brain_id)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert!(decisions.iter().all(|v| v["reason"] == ""));
    let requests = ok(&h, "GET", &format!("{base}/erasures"), &reader, Value::Null).await;
    assert_eq!(requests["total"], 1);
    assert_eq!(requests["items"][0]["pending_artifacts"], 2);
    assert!(
        h.call(
            "GET",
            &format!("{base}/erasures"),
            Some(&foreign),
            Value::Null
        )
        .await
        .0
        .is_client_error()
    );
    assert_eq!(h.call("POST",&format!("{base}/sources/{a_id}/versions"),Some(&owner),json!({"title":"Replay","media_type":"text/plain","content":"Recreated old evidence.","retain_content":true})).await.0,StatusCode::GONE);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn retention_deadlines_excerpt_independence_and_policy_authority() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (reader_id, reader) = h.fixture_member().await;
    let (_, foreign) = h.fixture_member().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic retention proof"}),
    )
    .await;
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain_id}");
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    let policy_url = format!("{base}/retention");
    let settings = ok(&h, "GET", &policy_url, &reader, Value::Null).await;
    assert_eq!(settings["policy"]["raw_session_days"], 30);
    assert!(settings["policy"]["document_days"].is_null());
    let input = json!({"title":"Synthetic raw session","media_type":"text/plain","content":"Raw first line.\nRetained short evidence.\nUnselected raw third line.\n","retain_content":true,"retention_class":"raw_session"});
    let raw = h
        .keyed(
            "POST",
            &format!("{base}/sources"),
            Some(&owner),
            input.clone(),
            Some("retention-raw-receipt"),
        )
        .await;
    assert_eq!(raw.0, StatusCode::OK, "{}", raw.1);
    let raw = raw.1;
    let raw_id = raw["id"].as_str().unwrap();
    let version = raw["version"]["id"].clone();
    assert_eq!(raw["version"]["retention_class"], "raw_session");
    assert!(raw["version"]["expires_at"].is_string());
    let excerpt_input = json!({"source_id":raw_id,"version_id":version,"first_line":2,"last_line":2,"title":"Permitted evidence excerpt"});
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/excerpts"),
            Some(&reader),
            excerpt_input.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let excerpt = ok(
        &h,
        "POST",
        &format!("{base}/excerpts"),
        &owner,
        excerpt_input.clone(),
    )
    .await;
    assert_eq!(excerpt["version"]["retention_class"], "support_excerpt");
    assert_eq!(excerpt["version"]["excerpt"]["first_line"], 2);
    let document=ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Independent document","media_type":"text/plain","content":"Independent retained evidence.\n","retain_content":true})).await;
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(&version, "Raw supported service", "alpha"),
    )
    .await;
    let excerpt_claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(
            &excerpt["version"]["id"],
            "Excerpt supported service",
            "beta",
        ),
    )
    .await;
    let mut change_class = input.clone();
    change_class["retention_class"] = json!("document");
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/sources/{raw_id}/versions"),
            Some(&owner),
            change_class
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut update = json!({"base_change":settings["change_id"],"policy":settings["policy"]});
    update["policy"]["raw_session_days"] = json!(2);
    assert_eq!(
        h.call("PUT", &policy_url, Some(&reader), update.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert!(
        h.call("PUT", &policy_url, Some(&foreign), update.clone())
            .await
            .0
            .is_client_error()
    );
    let (_, device) = h.pair_device(&owner, "Retention device").await;
    assert_eq!(
        h.bearer("PUT", &policy_url, &device, update.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let changed = ok(&h, "PUT", &policy_url, &owner, update.clone()).await;
    assert_ne!(changed["change_id"], settings["change_id"]);
    assert_eq!(
        h.call("PUT", &policy_url, Some(&owner), update).await.0,
        StatusCode::CONFLICT
    );
    let replay = h
        .keyed(
            "POST",
            &format!("{base}/sources"),
            Some(&owner),
            input,
            Some("retention-raw-receipt"),
        )
        .await;
    assert_eq!(replay.0, StatusCode::GONE);
    assert_eq!(replay.1["code"], "command_invalidated");
    let version_id: Uuid = version.as_str().unwrap().parse().unwrap();
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '3 days' WHERE id=$1",
    )
    .bind(version_id)
    .execute(&h.admin)
    .await
    .unwrap();
    let artifact: Uuid = sqlx::query_scalar("SELECT artifact_id FROM source_versions WHERE id=$1")
        .bind(version_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(
        artifacts::path(&h.state.config.artifact_dir, brain_id, artifact).exists(),
        "Deadline enforcement must precede physical cleanup"
    );
    let expired = ok(
        &h,
        "GET",
        &format!("{base}/sources/{raw_id}/versions/{version_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(expired["version"]["privacy_state"], "expired", "{expired}");
    assert!(!expired.to_string().contains("Raw first line"));
    assert!(!expired.to_string().contains("Synthetic raw session"));
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/excerpts"),
            Some(&owner),
            excerpt_input
        )
        .await
        .0,
        StatusCode::GONE
    );
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/claims"),
            Some(&owner),
            proposal(&version, "Late invalid proposal", "gamma")
        )
        .await
        .0,
        StatusCode::GONE
    );
    let evidence = ok(
        &h,
        "GET",
        &format!("{base}/claim-evidence/source_version/{version_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(evidence["evidence"]["availability"], "expired");
    assert!(evidence["text"].is_null());
    let retained = ok(
        &h,
        "GET",
        &format!(
            "{base}/claim-evidence/source_version/{}",
            excerpt["version"]["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(retained["text"], "Retained short evidence.\n");
    assert_eq!(retained["evidence"]["availability"], "retained");
    let retained = ok(
        &h,
        "GET",
        &format!(
            "{base}/claim-evidence/source_version/{}",
            document["version"]["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(retained["evidence"]["availability"], "retained");
    let view = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", claim["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(view["selection_state"], "active");
    assert_eq!(view["selected"]["evidence"][0]["availability"], "expired");
    assert_eq!(view["selected"]["eligibility"]["strict_accepted"], false);
    let view = ok(
        &h,
        "GET",
        &format!(
            "{base}/claims/{}",
            excerpt_claim["claim_id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(view["selected"]["evidence"][0]["availability"], "retained");
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let state: String =
        sqlx::query_scalar("SELECT state FROM jobs WHERE target_id=$1 AND kind='source.process'")
            .bind(version_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(state, "cancelled");
    let chunks: i64 = sqlx::query_scalar("SELECT count(*) FROM source_chunks WHERE version_id=$1")
        .bind(version_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(chunks, 0);
    privacy_journal::run_once(&h.state).await.unwrap();
    assert!(!artifacts::path(&h.state.config.artifact_dir, brain_id, artifact).exists());
    let expired = ok(
        &h,
        "GET",
        &format!("{base}/claim-evidence/source_version/{version_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(expired["evidence"]["availability"], "expired");
    let settings = ok(&h, "GET", &policy_url, &owner, Value::Null).await;
    let mut update = json!({"base_change":settings["change_id"],"policy":settings["policy"]});
    update["policy"]["raw_session_days"] = json!(30);
    update["policy"]["claim_days"] = json!(1);
    update["policy"]["allow_support_excerpts"] = json!(false);
    ok(&h, "PUT", &policy_url, &owner, update).await;
    let expired = ok(
        &h,
        "GET",
        &format!("{base}/claim-evidence/source_version/{version_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        expired["evidence"]["availability"], "expired",
        "Extending a policy cannot revive removed text"
    );
    sqlx::query(
        "UPDATE claim_revisions SET recorded_at=clock_timestamp()-interval '2 days' WHERE id=$1",
    )
    .bind(claim["id"].as_str().unwrap().parse::<Uuid>().unwrap())
    .execute(&h.admin)
    .await
    .unwrap();
    privacy_journal::run_once(&h.state).await.unwrap();
    let view = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", claim["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(view["selection_state"], "expired");
    let retained = ok(
        &h,
        "GET",
        &format!(
            "{base}/claim-evidence/source_version/{}",
            excerpt["version"]["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        retained["evidence"]["availability"], "retained",
        "Claim expiry does not erase independent excerpts"
    );
    assert_eq!(h.call("POST",&format!("{base}/excerpts"),Some(&owner),json!({"source_id":document["id"],"version_id":document["version"]["id"],"first_line":1,"last_line":1,"title":"Not permitted"})).await.0,StatusCode::FORBIDDEN);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn collection_claim_and_manifest_erasure_preserve_unrelated_inputs() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic erasure target controls"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let collection = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"collection","name":"Erased collection selection"}),
    )
    .await;
    let shared = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"collection","name":"Shared collection selection"}),
    )
    .await;
    let a = test_source(&h, &base, &owner, "Shared source erased everywhere").await;
    let b = test_source(&h, &base, &owner, "Independent source remains retained").await;
    ok(
        &h,
        "PUT",
        &format!("{base}/sources/{}/groups", a["id"].as_str().unwrap()),
        &owner,
        json!({"group_ids":[collection["id"],shared["id"]]}),
    )
    .await;
    ok(
        &h,
        "PUT",
        &format!("{base}/sources/{}/groups", b["id"].as_str().unwrap()),
        &owner,
        json!({"group_ids":[shared["id"]]}),
    )
    .await;
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        proposal(
            &b["version"]["id"],
            "Individually erased claim",
            "retained evidence",
        ),
    )
    .await;
    let mut revision = proposal(
        &b["version"]["id"],
        "Individually erased claim",
        "New retained interpretation",
    );
    revision["base_revision"] = claim["id"].clone();
    let other = ok(
        &h,
        "PUT",
        &format!("{base}/claims/{}", claim["claim_id"].as_str().unwrap()),
        &owner,
        revision,
    )
    .await;
    let target = json!({"kind":"claim","id":claim["claim_id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    assert_eq!(preview["claim_revisions"], 2);
    assert_eq!(preview["source_versions"], 0);
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let view = ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", other["claim_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(view["unavailable_history"].as_array().unwrap().len(), 2);
    let evidence = ok(
        &h,
        "GET",
        &format!(
            "{base}/claim-evidence/source_version/{}",
            b["version"]["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(evidence["evidence"]["availability"], "retained");
    // A reference-only manifest proves erasure does not require a materialized snapshot.
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let repo = Uuid::new_v4();
    sqlx::query("INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,'example.test/retention/fixture',$3)").bind(repo).bind(brain_id).bind(owner_id).execute(&h.admin).await.unwrap();
    let env = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Manifest cleanup environment"}),
    )
    .await;
    let mut input = json!({"name":"Explicitly removed manifest","environment_id":env["id"],"kind":"desired","entries":[{"repository_id":repo,"revision":"a".repeat(40),"snapshot_id":null,"config_paths":[]}],"notes":"Controlled note","base_revision":null,"operation_id":null});
    let manifest = ok(
        &h,
        "POST",
        &format!("{base}/revision-manifests"),
        &owner,
        input.clone(),
    )
    .await;
    input["base_revision"] = manifest["id"].clone();
    input["notes"] = json!("Second controlled note");
    ok(
        &h,
        "PUT",
        &format!(
            "{base}/revision-manifests/{}",
            manifest["manifest_id"].as_str().unwrap()
        ),
        &owner,
        input,
    )
    .await;
    let target = json!({"kind":"manifest","id":manifest["manifest_id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    assert_eq!(preview["manifest_revisions"], 2);
    assert_eq!(preview["snapshots"], 0);
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert_eq!(
        h.call(
            "GET",
            &format!(
                "{base}/revision-manifests/{}/revisions/{}",
                manifest["manifest_id"].as_str().unwrap(),
                manifest["id"].as_str().unwrap()
            ),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::GONE
    );
    ok(&h, "PATCH", &base, &owner, json!({"archived":true})).await;
    let target = json!({"kind":"collection","id":collection["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    assert_eq!(preview["shared_sources"], true);
    assert_eq!(preview["source_versions"], 1);
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    privacy_journal::maintain(&h.state).await.unwrap();
    let evidence = ok(
        &h,
        "GET",
        &format!(
            "{base}/claim-evidence/source_version/{}",
            a["version"]["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(evidence["evidence"]["availability"], "erased");
    let evidence = ok(
        &h,
        "GET",
        &format!(
            "{base}/claim-evidence/source_version/{}",
            b["version"]["id"].as_str().unwrap()
        ),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(evidence["evidence"]["availability"], "retained");
    let (foreign_id, _) = h.fixture_member().await;
    let mut scoped = db::actor_tx(&h.state.pool, foreign_id).await.ok().unwrap();
    let visible: i64 = sqlx::query_scalar("SELECT count(*) FROM privacy_requests")
        .fetch_one(&mut *scoped)
        .await
        .unwrap();
    assert_eq!(visible, 0);
    assert!(
        sqlx::query("SELECT recollect_privacy_preview($1,'source',$2)")
            .bind(brain_id)
            .bind(a["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .execute(&mut *scoped)
            .await
            .is_err()
    );
    scoped.rollback().await.unwrap();
    h.finish().await;
}
