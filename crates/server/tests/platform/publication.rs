use super::*;
use recollect_server::{artifacts, worker};

pub(super) async fn begin(h: &Harness, base: &str, token: &str, repo: &Value) -> Value {
    let task=h.bearer("POST",&format!("{base}/workspace/tasks"),token,json!({"label":"Publication proof","selection":{"repository_ids":[repo],"area_ids":[],"environment_id":null}})).await;
    assert_eq!(task.0, StatusCode::OK, "{}", task.1);
    let op = h
        .bearer(
            "POST",
            &format!(
                "{base}/workspace/tasks/{}/operations",
                task.1["task"]["id"].as_str().unwrap()
            ),
            token,
            json!({"kind":"capture"}),
        )
        .await;
    assert_eq!(op.0, StatusCode::OK, "{}", op.1);
    op.1
}
pub(super) fn input(repo: &Value, op: &Value, revision: &str) -> Value {
    json!({"publication_id":Uuid::new_v4(),"operation_id":op["id"],"origin":"example.test/team/fixture","revision":revision,"branch":"main","dirty":true,"captured_at":"2026-09-14T12:00:00Z","adapter":"enola-committed","adapter_build":"fixture","extractor_version":"fixture","settings":{"retained_files":["src/lib.rs"]},
    "files":[{"path":"src/lib.rs","object_id":"b".repeat(40),"mode":"100644","size":23,"status":"materialized","extraction":"facts_emitted","content":"pub fn committed() {}\n\n"},{"path":"deploy/app.yaml","object_id":"c".repeat(40),"mode":"100644","size":12,"status":"materialized","extraction":"no_facts","content":null}],
    "facts":[{"id":"same-upstream-id","repo":repo,"kind":"function","name":"committed","file":"src/lib.rs","line":1,"relations":[{"kind":"calls","target":"missing"}]},{"id":"same-upstream-id","repo":repo,"kind":"future_kind","name":"opaque retained record","file":"src/lib.rs","props":{"unknown":true}}],"insights":[],"receipt":{"enola_version":"fixture","extractor_version":"fixture","fact_count":2,"quality":{"parse_errors":0}}})
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn repository_publication_authority_deduplication_fencing_and_manifests() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (writer_id, writer) = h.fixture_member().await;
    let (reader_id, reader) = h.fixture_member().await;
    let brain = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Publication proof"}),
        )
        .await
        .1["id"]
        .clone();
    let brain_id: Uuid = brain.as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{}", brain.as_str().unwrap());
    for (id, role) in [(writer_id, "writer"), (reader_id, "reader")] {
        assert_eq!(
            h.call(
                "PUT",
                &format!("{base}/grants/{id}"),
                Some(&owner),
                json!({"role":role})
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let (device, token) = h.pair_device(&writer, "Publication writer").await;
    let (_, other_token) = h.pair_device(&owner, "Second contributor").await;
    let refresh = json!({"workspace_root":"/fixture","complete":false,"notes":[],"checkouts":[{"local_path":"/fixture/repo","origin":"https://example.test/team/fixture.git","head":null,"branch":"main","dirty":true,"status":"available"}]});
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{base}/workspace/checkouts"),
            &token,
            refresh
        )
        .await
        .0,
        StatusCode::OK
    );
    let repo = h
        .bearer("GET", &format!("{base}/workspace"), &token, Value::Null)
        .await
        .1["repositories"][0]["id"]
        .clone();
    let op = begin(&h, &base, &token, &repo).await;
    let first_revision = "a".repeat(40);
    let mut payload = input(&repo, &op, &first_revision);
    assert_eq!(payload["files"][0]["content"].as_str().unwrap().len(), 23);
    let url = format!("{base}/repositories/{}/snapshots", repo.as_str().unwrap());
    assert_eq!(
        h.bearer("POST", &url, &token, payload.clone()).await.0,
        StatusCode::FORBIDDEN
    );
    for denied in [&reader, &writer] {
        assert_eq!(
            h.call(
                "PUT",
                &format!("{base}/repositories/policy"),
                Some(denied),
                json!({"allow_file_content":true})
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/repositories/policy"),
            Some(&owner),
            json!({"allow_file_content":true})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call("POST", &url, Some(&writer), payload.clone()).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.bearer("POST", &url, &other_token, payload.clone())
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let mut malformed = payload.clone();
    malformed["facts"][0]["file"] = json!("../outside");
    assert_eq!(
        h.bearer("POST", &url, &token, malformed).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut malformed = payload.clone();
    malformed["insights"] = json!([{"evidence":[{"file":"/private/outside"}]}]);
    assert_eq!(
        h.bearer("POST", &url, &token, malformed).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut malformed = payload.clone();
    malformed["facts"][0]["line"] = json!(0);
    assert_eq!(
        h.bearer("POST", &url, &token, malformed).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut leaked = payload.clone();
    leaked["files"][0]["content"] = json!(h.state.config.owner_password);
    leaked["files"][0]["size"] = json!(h.state.config.owner_password.len());
    assert_eq!(
        h.bearer("POST", &url, &token, leaked).await.0,
        StatusCode::BAD_REQUEST
    );
    let (a, b) = tokio::join!(
        h.bearer("POST", &url, &token, payload.clone()),
        h.bearer("POST", &url, &token, payload.clone())
    );
    assert_eq!(a.0, StatusCode::OK, "{}", a.1);
    assert_eq!(b.0, StatusCode::OK, "{}", b.1);
    assert_eq!(a.1, b.1);
    let snapshot = a.1["snapshot"]["id"].as_str().unwrap().to_owned();
    let snapshot_id: Uuid = snapshot.parse().unwrap();
    let detail_url = format!("{base}/repository-snapshots/{snapshot}");
    assert_eq!(a.1["snapshot"]["processing"], "queued");
    let mut changed = payload.clone();
    changed["dirty"] = json!(false);
    assert_eq!(
        h.bearer("POST", &url, &token, changed).await.0,
        StatusCode::CONFLICT
    );
    let mut changed = payload.clone();
    changed["publication_id"] = json!(Uuid::new_v4());
    changed["dirty"] = json!(false);
    assert_eq!(
        h.bearer("POST", &url, &token, changed).await.0,
        StatusCode::CONFLICT
    );
    let op2 = begin(&h, &base, &other_token, &repo).await;
    let mut second_contributor = payload.clone();
    second_contributor["operation_id"] = op2["id"].clone();
    second_contributor["publication_id"] = json!(Uuid::new_v4());
    second_contributor["dirty"] = json!(false);
    let reused = h
        .bearer("POST", &url, &other_token, second_contributor.clone())
        .await;
    assert_eq!(reused.0, StatusCode::OK, "{}", reused.1);
    assert_eq!(reused.1["reused"], true);
    assert_eq!(reused.1["snapshot"]["id"], snapshot);
    let detail = h.call("GET", &detail_url, Some(&reader), Value::Null).await;
    assert_eq!(detail.0, StatusCode::OK);
    assert_eq!(detail.1["contributor_total"], 2);
    second_contributor["publication_id"] = json!(Uuid::new_v4());
    second_contributor["facts"][0]["name"] = json!("conflicting artifact");
    assert_eq!(
        h.bearer("POST", &url, &other_token, second_contributor)
            .await
            .0,
        StatusCode::CONFLICT
    );
    let lease = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE jobs SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(lease.id)
        .execute(&h.admin)
        .await
        .unwrap();
    let replacement = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        worker::execute(&h.state, &lease).await,
        Err(worker::Failure::LostLease)
    );
    worker::execute(&h.state, &replacement).await.unwrap();
    let facts = h
        .call(
            "GET",
            &format!("{detail_url}/facts"),
            Some(&reader),
            Value::Null,
        )
        .await
        .1;
    assert_eq!(facts["processing"], "ready");
    assert_eq!(facts["items"].as_array().unwrap().len(), 2);
    assert_ne!(facts["items"][0]["id"], facts["items"][1]["id"]);
    assert_eq!(
        h.call(
            "POST",
            &format!("{detail_url}/process"),
            Some(&writer),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let repeated = h
        .call(
            "GET",
            &format!("{detail_url}/facts"),
            Some(&reader),
            Value::Null,
        )
        .await
        .1;
    assert_eq!(facts, repeated);
    let files = h
        .call(
            "GET",
            &format!("{detail_url}/files"),
            Some(&reader),
            Value::Null,
        )
        .await
        .1;
    let retained = files["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == "src/lib.rs")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let content_url = format!("{detail_url}/files/{retained}");
    assert_eq!(
        h.call("GET", &content_url, Some(&reader), Value::Null)
            .await
            .1["content"],
        payload["files"][0]["content"]
    );
    assert_eq!(files["items"][0]["availability"], "not_retained");
    let op3 = begin(&h, &base, &token, &repo).await;
    payload["operation_id"] = op3["id"].clone();
    payload["publication_id"] = json!(Uuid::new_v4());
    payload["revision"] = json!("d".repeat(40));
    let next = h.bearer("POST", &url, &token, payload.clone()).await;
    assert_eq!(next.0, StatusCode::OK, "{}", next.1);
    assert_ne!(next.1["snapshot"]["id"], snapshot);
    let mut envs = Vec::new();
    for name in ["Production", "Development"] {
        envs.push(
            h.call(
                "POST",
                &format!("{base}/evidence/groups"),
                Some(&owner),
                json!({"kind":"environment","name":name}),
            )
            .await
            .1["id"]
                .clone(),
        );
    }
    let mut manifest = json!({"name":"Selected revisions","environment_id":envs[0],"kind":"desired","entries":[{"repository_id":repo,"revision":first_revision,"snapshot_id":snapshot,"config_paths":["deploy/app.yaml"]}],"base_revision":null,"operation_id":null,"observed_at":null,"observation_reference":null,"notes":"Desired configuration"});
    let created = h
        .call(
            "POST",
            &format!("{base}/revision-manifests"),
            Some(&writer),
            manifest.clone(),
        )
        .await;
    assert_eq!(created.0, StatusCode::OK, "{}", created.1);
    let manifest_url = format!(
        "{base}/revision-manifests/{}",
        created.1["manifest_id"].as_str().unwrap()
    );
    manifest["environment_id"] = envs[1].clone();
    manifest["kind"] = json!("committed");
    manifest["entries"][0]["revision"] = payload["revision"].clone();
    manifest["entries"][0]["snapshot_id"] = next.1["snapshot"]["id"].clone();
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/revision-manifests"),
            Some(&writer),
            manifest.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    manifest["environment_id"] = envs[0].clone();
    manifest["base_revision"] = created.1["id"].clone();
    manifest["kind"] = json!("observed_deployed");
    assert_eq!(
        h.call("PUT", &manifest_url, Some(&writer), manifest.clone())
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    manifest["observed_at"] = json!("2026-09-14T12:20:00Z");
    manifest["observation_reference"] = json!("https://example.test/deployment/fixture");
    let updated = h
        .call("PUT", &manifest_url, Some(&writer), manifest.clone())
        .await;
    assert_eq!(updated.0, StatusCode::OK, "{}", updated.1);
    assert_eq!(
        h.call("PUT", &manifest_url, Some(&writer), manifest.clone())
            .await
            .0,
        StatusCode::CONFLICT
    );
    let history = h
        .call("GET", &manifest_url, Some(&reader), Value::Null)
        .await
        .1;
    assert_eq!(history["total"], 2);
    assert_eq!(history["history"][1]["kind"], "desired");
    assert_eq!(history["current"]["kind"], "observed_deployed");
    let historic = h
        .call(
            "GET",
            &format!(
                "{manifest_url}/revisions/{}",
                created.1["id"].as_str().unwrap()
            ),
            Some(&reader),
            Value::Null,
        )
        .await
        .1;
    assert_eq!(historic["entries"][0]["snapshot_id"], snapshot);
    let missing_selection = json!({"name":"Unpublished revision","environment_id":envs[1],"kind":"committed","entries":[{"repository_id":repo,"revision":"e".repeat(40),"snapshot_id":null,"config_paths":[]}],"base_revision":null,"operation_id":null,"observed_at":null,"observation_reference":null,"notes":"Explicit unavailable snapshot"});
    let missing = h
        .call(
            "POST",
            &format!("{base}/revision-manifests"),
            Some(&writer),
            missing_selection,
        )
        .await;
    assert_eq!(missing.0, StatusCode::OK, "{}", missing.1);
    assert!(missing.1["entries"][0]["snapshot_id"].is_null());
    assert_eq!(
        h.call(
            "DELETE",
            &format!("{base}/evidence/groups/{}", envs[0].as_str().unwrap()),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let foreign = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Unrelated Brain"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{foreign}/repository-snapshots/{snapshot}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/revision-manifests"),
            Some(&reader),
            manifest.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Revocation fences queued work, while accepted artifacts remain readable by current Brain members.
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/devices/{device}"),
            Some(&writer),
            Value::Null,
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.bearer("POST", &url, &token, payload.clone()).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    assert_eq!(
        h.call(
            "GET",
            &format!(
                "{base}/repository-snapshots/{}",
                next.1["snapshot"]["id"].as_str().unwrap()
            ),
            Some(&reader),
            Value::Null
        )
        .await
        .1["snapshot"]["processing"],
        "cancelled"
    );
    assert!(
        h.call("GET", &content_url, Some(&reader), Value::Null)
            .await
            .1["content"]
            .is_string()
    );
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/repositories/policy"),
            Some(&owner),
            json!({"allow_file_content":false})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert!(
        h.call("GET", &content_url, Some(&reader), Value::Null)
            .await
            .1["content"]
            .is_string()
    );
    let artifact_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM repository_artifacts WHERE snapshot_id=$1 AND kind='facts'",
    )
    .bind(snapshot_id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let artifact_path = artifacts::path(&h.state.config.artifact_dir, brain_id, artifact_id);
    let saved = std::fs::read(&artifact_path).unwrap();
    std::fs::remove_file(&artifact_path).unwrap();
    assert_eq!(
        h.call(
            "GET",
            &format!("{detail_url}/artifacts/facts"),
            Some(&reader),
            Value::Null
        )
        .await
        .1["availability"],
        "missing"
    );
    assert_eq!(
        h.call(
            "POST",
            &format!("{detail_url}/process"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        worker::execute(&h.state, &job).await,
        Err(worker::Failure::Storage)
    );
    std::fs::write(artifact_path, saved).unwrap();
    worker::execute(&h.state, &job).await.unwrap();
    let mut rls = db::actor_tx(&h.state.pool, reader_id).await.ok().unwrap();
    assert!(
        sqlx::query("UPDATE repository_snapshots SET revision='changed' WHERE id=$1")
            .bind(snapshot_id)
            .execute(&mut *rls)
            .await
            .is_err()
    );
    rls.rollback().await.unwrap();
    h.call(
        "DELETE",
        &format!("{base}/grants/{reader_id}"),
        Some(&owner),
        Value::Null,
    )
    .await;
    assert_eq!(
        h.call("GET", &detail_url, Some(&reader), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let mut rls = db::actor_tx(&h.state.pool, reader_id).await.ok().unwrap();
    let visible: i64 = sqlx::query_scalar("SELECT count(*) FROM repository_snapshots")
        .fetch_one(&mut *rls)
        .await
        .unwrap();
    assert_eq!(visible, 0);
    rls.rollback().await.unwrap();
    h.finish().await;
}
