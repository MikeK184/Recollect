use super::*;
use recollect_server::{artifacts, worker};

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn collections_history_artifacts_scope_and_fenced_processing() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (writer_id, writer) = h.fixture_member().await;
    let (_, reader) = h.fixture_member().await;
    let brain = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Evidence"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let brain_id: Uuid = brain.parse().unwrap();
    let base = format!("/api/brains/{brain}");
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/grants/{writer_id}"),
            Some(&owner),
            json!({"role":"writer"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let foreign_brain = h
        .call(
            "POST",
            "/api/brains",
            Some(&reader),
            json!({"name":"Foreign evidence"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let foreign_base = format!("/api/brains/{foreign_brain}");
    let mut groups = vec![];
    for (kind, name) in [
        ("collection", "Runbooks"),
        ("collection", "Shared notes"),
        ("area", "Vault"),
        ("environment", "Production"),
    ] {
        let result = h
            .call(
                "POST",
                &format!("{base}/evidence/groups"),
                Some(&writer),
                json!({"kind":kind,"name":name}),
            )
            .await;
        assert_eq!(result.0, StatusCode::OK);
        groups.push(result.1["id"].as_str().unwrap().to_owned());
    }
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/evidence/groups"),
            Some(&writer),
            json!({"kind":"collection","name":"RUNBOOKS"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let foreign_group = h
        .call(
            "POST",
            &format!("{foreign_base}/evidence/groups"),
            Some(&reader),
            json!({"kind":"collection","name":"Canary"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let text = "First Ω line\nSecond engineering line\n".repeat(3000);
    let input = json!({"title":"Vault evidence","media_type":"text/markdown","content":text,"retain_content":true,"source_uri":"https://docs.example.test/vault","group_ids":groups,"observed_at":null});
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/sources"),
            Some(&reader),
            input.clone()
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let mut invalid = input.clone();
    invalid["group_ids"] = json!([foreign_group]);
    assert_eq!(
        h.call("POST", &format!("{base}/sources"), Some(&writer), invalid)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let created = h
        .keyed(
            "POST",
            &format!("{base}/sources"),
            Some(&writer),
            input.clone(),
            Some("import-evidence"),
        )
        .await;
    assert_eq!(
        created.0,
        StatusCode::OK,
        "Large text import must use the upload route's body limit"
    );
    let source = created.1["id"].as_str().unwrap().to_owned();
    let old = created.1["version"]["id"].as_str().unwrap().to_owned();
    assert_eq!(created.1["created_by"], writer_id.to_string());
    assert_eq!(created.1["version"]["availability"], "retained");
    let replay = h
        .keyed(
            "POST",
            &format!("{base}/sources"),
            Some(&writer),
            input.clone(),
            Some("import-evidence"),
        )
        .await;
    assert_eq!(
        replay.0,
        StatusCode::OK,
        "Writers can replay their still-authorized command"
    );
    assert_eq!(replay.1, created.1);
    let mut different = input.clone();
    different["content"] = json!("Different text");
    assert_eq!(
        h.keyed(
            "POST",
            &format!("{base}/sources"),
            Some(&writer),
            different,
            Some("import-evidence")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let source_path = format!("{base}/sources/{source}");
    let identity = h
        .call("GET", &source_path, Some(&writer), Value::Null)
        .await;
    assert_eq!(identity.0, StatusCode::OK);
    assert_eq!(identity.1["id"], source);
    assert_eq!(
        h.call("GET", &source_path, Some(&reader), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let old_path = format!("{source_path}/versions/{old}");
    assert_eq!(
        h.call("GET", &old_path, Some(&writer), Value::Null).await.1["content"],
        text
    );
    assert_eq!(
        h.call("GET", &old_path, Some(&reader), Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    let foreign_source=h.call("POST",&format!("{foreign_base}/sources"),Some(&reader),json!({"title":"Canary","media_type":"text/plain","content":"FOREIGN EVIDENCE CANARY","retain_content":true})).await.1;
    let mut scoped = db::actor_tx(&h.state.pool, writer_id)
        .await
        .unwrap_or_else(|_| panic!("writer transaction"));
    let canary_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM source_versions WHERE brain_id=$1")
            .bind(foreign_brain.parse::<Uuid>().unwrap())
            .fetch_one(&mut *scoped)
            .await
            .unwrap();
    assert_eq!(canary_count, 0, "RLS independently excludes another Brain");
    scoped.rollback().await.unwrap();
    let foreign_url = format!(
        "{base}/sources/{}/versions/{}",
        foreign_source["id"].as_str().unwrap(),
        foreign_source["version"]["id"].as_str().unwrap()
    );
    assert_eq!(
        h.call("GET", &foreign_url, Some(&writer), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    // Claim the old version, then publish a newer canonical version before processing it.
    let old_job = worker::claim(&h.state.pool, "capture")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(old_job.target_id, old.parse::<Uuid>().unwrap());
    let update = json!({"title":"Vault evidence updated","media_type":"text/markdown","content":"Production has separate authority.\nObserved state remains explicit.\n","retain_content":true,"base_version":old});
    let updated = h
        .call(
            "POST",
            &format!("{source_path}/versions"),
            Some(&writer),
            update.clone(),
        )
        .await;
    assert_eq!(updated.0, StatusCode::OK);
    assert_eq!(updated.1["version_count"], 2);
    let current = updated.1["version"]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        h.call(
            "POST",
            &format!("{source_path}/versions"),
            Some(&writer),
            update.clone()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    worker::execute(&h.state, &old_job).await.unwrap();
    let catalogue = h
        .call(
            "GET",
            &format!(
                "{base}/evidence?collection={}&area={}&environment={}",
                groups[0], groups[2], groups[3]
            ),
            Some(&writer),
            Value::Null,
        )
        .await;
    assert_eq!(catalogue.0, StatusCode::OK);
    assert_eq!(catalogue.1["total"], 1);
    assert_eq!(catalogue.1["sources"][0]["version"]["id"], current);
    assert_eq!(catalogue.1["sources"][0]["version"]["processing"], "queued");
    let old_content = h.call("GET", &old_path, Some(&writer), Value::Null).await.1;
    assert_eq!(old_content["content"], text);
    assert!(old_content["spans"].as_array().unwrap().len() > 20);
    let mut assembled = String::new();
    for span in old_content["spans"].as_array().unwrap() {
        let start = span["byte_start"].as_u64().unwrap() as usize;
        let end = span["byte_end"].as_u64().unwrap() as usize;
        assert!(end - start <= 4096);
        assembled.push_str(&text[start..end]);
    }
    assert_eq!(
        assembled, text,
        "UTF-8 spans reproduce the exact retained bytes"
    );
    // Drain remaining capture work through actual handlers, including the permitted foreign control.
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let current_path = format!("{source_path}/versions/{current}");
    let current_content = h
        .call("GET", &current_path, Some(&writer), Value::Null)
        .await
        .1;
    assert_eq!(current_content["version"]["processing"], "ready");
    let stable_spans = current_content["spans"].clone();
    h.call(
        "POST",
        &format!("{source_path}/process"),
        Some(&writer),
        Value::Null,
    )
    .await;
    worker::run_once(&h.state, "capture").await.unwrap();
    assert_eq!(
        h.call("GET", &current_path, Some(&writer), Value::Null)
            .await
            .1["spans"],
        stable_spans
    );
    // Removing membership or an entire collection cannot erase shared evidence.
    h.call(
        "PUT",
        &format!("{source_path}/groups"),
        Some(&writer),
        json!({"group_ids":[groups[1],groups[2],groups[3]]}),
    )
    .await;
    assert_eq!(
        h.call(
            "GET",
            &format!("{base}/evidence?collection={}", groups[0]),
            Some(&writer),
            Value::Null
        )
        .await
        .1["total"],
        0
    );
    assert_eq!(
        h.call(
            "DELETE",
            &format!("{base}/evidence/groups/{}", groups[1]),
            Some(&writer),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.call(
            "GET",
            &format!(
                "{base}/evidence?area={}&environment={}",
                groups[2], groups[3]
            ),
            Some(&writer),
            Value::Null
        )
        .await
        .1["total"],
        1
    );
    assert_eq!(
        h.call("GET", &old_path, Some(&writer), Value::Null).await.1["content"],
        text
    );
    // A missing file hides support spans and clears the old chunk projection on reprocessing.
    let artifact: Uuid = sqlx::query_scalar("SELECT artifact_id FROM source_versions WHERE id=$1")
        .bind(current.parse::<Uuid>().unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let artifact_path = artifacts::path(&h.state.config.artifact_dir, brain_id, artifact);
    let saved_path = artifact_path.with_extension("test-saved");
    std::fs::rename(&artifact_path, &saved_path).unwrap();
    let missing = h
        .call("GET", &current_path, Some(&writer), Value::Null)
        .await
        .1;
    assert_eq!(missing["version"]["availability"], "missing");
    assert!(missing["content"].is_null());
    assert_eq!(missing["spans"], json!([]));
    h.call(
        "POST",
        &format!("{source_path}/process"),
        Some(&writer),
        Value::Null,
    )
    .await;
    worker::run_once(&h.state, "capture").await.unwrap();
    assert_eq!(
        h.call("GET", &current_path, Some(&writer), Value::Null)
            .await
            .1["version"]["processing"],
        "missing"
    );
    std::fs::rename(&saved_path, &artifact_path).unwrap();
    h.call(
        "POST",
        &format!("{source_path}/process"),
        Some(&writer),
        Value::Null,
    )
    .await;
    worker::run_once(&h.state, "capture").await.unwrap();
    assert_eq!(
        h.call("GET", &current_path, Some(&writer), Value::Null)
            .await
            .1["content"],
        update["content"]
    );
    // Capture policy denies new payloads without erasing already permitted history.
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/evidence/policy"),
            Some(&writer),
            json!({"allow_document_content":false})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    h.call(
        "PUT",
        &format!("{base}/evidence/policy"),
        Some(&owner),
        json!({"allow_document_content":false}),
    )
    .await;
    let mut capture_attempt = input.clone();
    capture_attempt["group_ids"] = json!([]);
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/sources"),
            Some(&writer),
            capture_attempt
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call("GET", &old_path, Some(&writer), Value::Null).await.1["content"],
        text
    );
    let reference=h.call("POST",&format!("{base}/sources"),Some(&writer),json!({"title":"Controlled external reference","media_type":"text/plain","source_uri":"https://example.test/controlled"})).await;
    assert_eq!(reference.0, StatusCode::OK);
    assert_eq!(reference.1["version"]["availability"], "reference_only");
    h.call(
        "PUT",
        &format!("{base}/evidence/policy"),
        Some(&owner),
        json!({"allow_document_content":true}),
    )
    .await;
    let counts = || async {
        sqlx::query_as::<_,(i64,i64,i64)>("SELECT (SELECT count(*) FROM sources),(SELECT count(*) FROM mutation_audit),(SELECT count(*) FROM jobs)").fetch_one(&h.admin).await.unwrap()
    };
    let before = counts().await;
    let secret_input = json!({"title":"Rejected credential import","media_type":"text/plain","content":h.state.config.owner_password,"retain_content":true});
    let rejected_secret = h
        .keyed(
            "POST",
            &format!("{base}/sources"),
            Some(&writer),
            secret_input,
            Some("blocked-secret"),
        )
        .await;
    assert_eq!(rejected_secret.0, StatusCode::BAD_REQUEST);
    assert!(
        !rejected_secret
            .1
            .to_string()
            .contains(&h.state.config.owner_password)
    );
    let receipt_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM command_receipts WHERE key='blocked-secret'")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(receipt_count, 0);
    assert_eq!(counts().await, before);
    let blocked = std::path::Path::new(&h.state.config.artifact_dir).join("test-blocked-volume");
    std::fs::write(&blocked, b"owned test obstacle").unwrap();
    let mut config = (*h.state.config).clone();
    config.artifact_dir = blocked.to_string_lossy().into_owned();
    let broken_state = AppState::new(h.state.pool.clone(), config).unwrap();
    let broken = app(broken_state.clone());
    let response=broken.oneshot(Request::builder().method("POST").uri(format!("{base}/sources")).header("content-type","application/json").header("cookie",&writer.cookie).header("x-csrf-token",&writer.csrf).body(Body::from(json!({"title":"Must roll back","media_type":"text/plain","content":"No accepted artifact","retain_content":true}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        counts().await,
        before,
        "Failed file persistence rolls back canonical metadata/audit/jobs"
    );
    let oversized = json!({"title":"Oversized","media_type":"text/plain","content":"a".repeat(artifacts::MAX_TEXT+1),"retain_content":true});
    assert_eq!(
        h.call("POST", &format!("{base}/sources"), Some(&writer), oversized)
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(counts().await, before);
    h.call(
        "POST",
        &format!("{source_path}/process"),
        Some(&writer),
        Value::Null,
    )
    .await;
    let storage_job = worker::claim(&h.state.pool, "capture")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        worker::execute(&broken_state, &storage_job).await,
        Err(worker::Failure::Storage)
    );
    assert!(
        worker::fail(&h.state.pool, &storage_job, worker::Failure::Storage)
            .await
            .unwrap()
    );
    let storage_error: String = sqlx::query_scalar("SELECT error_code FROM jobs WHERE id=$1")
        .bind(storage_job.id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(storage_error, "artifact_unavailable");
    sqlx::query("UPDATE jobs SET not_before=now() WHERE id=$1")
        .bind(storage_job.id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(worker::run_once(&h.state, "capture").await.unwrap());
    // Revoking a contributing device prevents its queued source from publishing.
    let (device, token) = h.pair_device(&writer, "Evidence companion").await;
    let submitted=h.bearer("POST",&format!("{base}/sources"),&token,json!({"title":"Companion evidence","media_type":"text/plain","content":"Contributor-scoped pending work","retain_content":true})).await;
    assert_eq!(submitted.0, StatusCode::OK);
    assert_eq!(submitted.1["version"]["device_id"], device.to_string());
    let queued = worker::claim(&h.state.pool, "capture")
        .await
        .unwrap()
        .unwrap();
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
        worker::execute(&h.state, &queued).await,
        Err(worker::Failure::Revoked)
    );
    worker::fail(&h.state.pool, &queued, worker::Failure::Revoked)
        .await
        .unwrap();
    // A reader can inspect the retained versions but cannot change them or replay a write.
    h.call(
        "PUT",
        &format!("{base}/grants/{writer_id}"),
        Some(&owner),
        json!({"role":"reader"}),
    )
    .await;
    assert_eq!(
        h.call("GET", &old_path, Some(&writer), Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        h.keyed(
            "POST",
            &format!("{base}/sources"),
            Some(&writer),
            input,
            Some("import-evidence")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    h.call("PATCH", &base, Some(&owner), json!({"archived":true}))
        .await;
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/evidence/groups"),
            Some(&owner),
            json!({"kind":"area","name":"Archived write"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    h.finish().await;
}
