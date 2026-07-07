use super::*;
use chrono::{DateTime, Utc};
use recollect_protocol::{ClaimRevision, FactValidity};
use recollect_server::{
    artifacts,
    memory_policy::{self as policy, Assessment, TimeMatch},
};

fn proposal(version: &Value) -> Value {
    json!({"base_revision":null,"operation_id":null,"content":{
        "kind":"claim","subject":"Vault production","predicate":"auth method","value":"JWT",
        "rationale":"Recorded from the selected evidence, awaiting review.",
        "selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"manifest_revision_id":null,
        "validity":{"kind":"interval","from":"2026-01-01T00:00:00Z","to":"2026-02-01T00:00:00Z","precision":"second"},
        "freshness":"current","operational":"declared","observed_at":null,"observation":"",
        "supports":[{"kind":"source_version","id":version,"line_from":1,"line_to":1}]}})
}
fn parse(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}
fn assessment() -> Assessment {
    Assessment {
        scope_valid: true,
        support_available: true,
        evidence_changed: false,
        evidence_missing: false,
        fact_time: TimeMatch::Matches,
    }
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn claim_manifests_preserve_independent_environment_applicability() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (writer_id, writer) = h.fixture_member().await;
    let (_, foreign) = h.fixture_member().await;
    let brain = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Claim manifests"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let brain_id: Uuid = brain.parse().unwrap();
    let base = format!("/api/brains/{brain}");
    h.call(
        "PUT",
        &format!("{base}/grants/{writer_id}"),
        Some(&owner),
        json!({"role":"writer"}),
    )
    .await;
    let mut repos = Vec::new();
    let mut snapshots = Vec::new();
    let mut facts = Vec::new();
    // Canonical publication fixtures; the publication suite separately proves admission/extraction.
    for i in 0..2 {
        let repo = Uuid::new_v4();
        let snapshot = Uuid::new_v4();
        let fact = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,$3,$4)",
        )
        .bind(repo)
        .bind(brain_id)
        .bind(format!("example.test/team/repo{i}"))
        .bind(writer_id)
        .execute(&h.admin)
        .await
        .unwrap();
        sqlx::query("INSERT INTO repository_snapshots(id,brain_id,repository_id,revision,adapter,adapter_build,extractor_version,settings,coverage,file_count,fact_count,retained_file_count,created_by) VALUES($1,$2,$3,$4,'fixture','fixture','fixture','{}','{}',1,1,0,$5)").bind(snapshot).bind(brain_id).bind(repo).bind("a".repeat(40)).bind(writer_id).execute(&h.admin).await.unwrap();
        sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) VALUES($1,$2,$3,0,$4)").bind(fact).bind(brain_id).bind(snapshot).bind(json!({"kind":"function","name":format!("fixture{i}"),"file":"main.rs","line":1})).execute(&h.admin).await.unwrap();
        repos.push(repo);
        snapshots.push(snapshot);
        facts.push(fact);
    }
    let mut claim_ids = Vec::new();
    let mut manifests = Vec::new();
    let mut inputs = Vec::new();
    for environment in ["Production", "Development"] {
        let env = h
            .call(
                "POST",
                &format!("{base}/evidence/groups"),
                Some(&writer),
                json!({"kind":"environment","name":environment}),
            )
            .await
            .1["id"]
            .clone();
        let input = json!({"name":format!("{environment} selection"),"environment_id":env,"kind":"committed","entries":[{"repository_id":repos[0],"revision":"a".repeat(40),"snapshot_id":snapshots[0],"config_paths":[]},{"repository_id":repos[1],"revision":"a".repeat(40),"snapshot_id":snapshots[1],"config_paths":[]}],"notes":"","base_revision":null,"operation_id":null});
        let manifest = h
            .call(
                "POST",
                &format!("{base}/revision-manifests"),
                Some(&writer),
                input.clone(),
            )
            .await;
        assert_eq!(manifest.0, StatusCode::OK, "{}", manifest.1);
        let mut p = proposal(&json!(facts[0]));
        p["content"]["selection"] =
            json!({"repository_ids":[repos[0]],"area_ids":[],"environment_id":env});
        p["content"]["manifest_revision_id"] = manifest.1["id"].clone();
        p["content"]["supports"] = json!([{"kind":"repository_fact","id":facts[0],"line_from":null,"line_to":null},{"kind":"manifest_revision","id":manifest.1["id"],"line_from":null,"line_to":null}]);
        let claim = h
            .call("POST", &format!("{base}/claims"), Some(&writer), p.clone())
            .await;
        assert_eq!(claim.0, StatusCode::OK, "{}", claim.1);
        let mut wrong = p.clone();
        wrong["content"]["supports"][0]["id"] = json!(facts[1]);
        assert_eq!(
            h.call("POST", &format!("{base}/claims"), Some(&writer), wrong)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
        let mut wrong = p;
        wrong["content"]["selection"]["environment_id"] = Value::Null;
        assert_eq!(
            h.call("POST", &format!("{base}/claims"), Some(&writer), wrong)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
        claim_ids.push(claim.1);
        manifests.push(manifest.1);
        inputs.push(input);
    }
    // Changing an unrelated repository or notes preserves Production's selected claim evidence.
    let mut update = inputs[0].clone();
    update["base_revision"] = manifests[0]["id"].clone();
    update["notes"] = json!("An unrelated repo advanced");
    update["entries"][1]["revision"] = json!("b".repeat(40));
    update["entries"][1]["snapshot_id"] = Value::Null;
    let update = h
        .call(
            "PUT",
            &format!(
                "{base}/revision-manifests/{}",
                manifests[0]["manifest_id"].as_str().unwrap()
            ),
            Some(&writer),
            update,
        )
        .await;
    assert_eq!(update.0, StatusCode::OK, "{}", update.1);
    // Development changes the actual supporting selection.
    let mut update = inputs[1].clone();
    update["base_revision"] = manifests[1]["id"].clone();
    update["entries"][0]["revision"] = json!("c".repeat(40));
    update["entries"][0]["snapshot_id"] = Value::Null;
    assert_eq!(
        h.call(
            "PUT",
            &format!(
                "{base}/revision-manifests/{}",
                manifests[1]["manifest_id"].as_str().unwrap()
            ),
            Some(&writer),
            update
        )
        .await
        .0,
        StatusCode::OK
    );
    for (i, expected) in ["current", "needs_verification"].into_iter().enumerate() {
        let url = format!(
            "{base}/claims/{}",
            claim_ids[i]["claim_id"].as_str().unwrap()
        );
        let current = h.call("GET", &url, Some(&writer), Value::Null).await.1;
        assert_eq!(
            current["selected"]["eligibility"]["effective_freshness"], expected,
            "{current}"
        );
        let before = h
            .call(
                "GET",
                &format!(
                    "{url}?knowledge_at={}",
                    claim_ids[i]["recorded_at"].as_str().unwrap()
                ),
                Some(&writer),
                Value::Null,
            )
            .await
            .1;
        assert_eq!(
            before["selected"]["eligibility"]["effective_freshness"],
            "current"
        );
    }
    let catalogue = h
        .call(
            "GET",
            &format!(
                "{base}/claim-evidence?kind=repository_fact&repository_id={}",
                repos[0]
            ),
            Some(&writer),
            Value::Null,
        )
        .await
        .1;
    assert_eq!(catalogue["total"], 1);
    assert_eq!(catalogue["items"][0]["id"], json!(facts[0]));
    assert_eq!(
        h.call(
            "GET",
            &format!("{base}/claim-evidence/repository_fact/{}", facts[0]),
            Some(&writer),
            Value::Null
        )
        .await
        .1["data"]["file"],
        "main.rs"
    );
    let foreign_brain = h
        .call(
            "POST",
            "/api/brains",
            Some(&foreign),
            json!({"name":"Foreign scope"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let foreign_evidence=h.call("POST",&format!("/api/brains/{foreign_brain}/sources"),Some(&foreign),json!({"title":"Foreign evidence","media_type":"text/plain","retain_content":false,"source_uri":"https://example.test/foreign","group_ids":[]})).await.1;
    let mut wrong = proposal(&foreign_evidence["version"]["id"]);
    wrong["content"]["supports"][0]["line_from"] = Value::Null;
    wrong["content"]["supports"][0]["line_to"] = Value::Null;
    assert_eq!(
        h.call("POST", &format!("{base}/claims"), Some(&writer), wrong)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "GET",
            &format!(
                "{base}/claim-evidence/source_version/{}",
                foreign_evidence["version"]["id"].as_str().unwrap()
            ),
            Some(&writer),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("{base}/claims"),
            Some(&foreign),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // Reconstruct the application and read the persisted history using the same authenticated session.
    let restarted = app(AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap());
    let response = restarted
        .oneshot(
            Request::builder()
                .uri(format!("{base}/claims"))
                .header("cookie", &writer.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn claims_temporal_history_eligibility_scope_and_evidence_changes() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (writer_id, writer) = h.fixture_member().await;
    let (reader_id, reader) = h.fixture_member().await;
    let brain = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Memory proof"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let brain_id: Uuid = brain.parse().unwrap();
    let base = format!("/api/brains/{brain}");
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
    let source_input = json!({"title":"Dated observation","media_type":"text/plain","content":"January uses JWT.\nFebruary uses OIDC.\n","retain_content":true,"source_uri":"https://docs.example.test/observations","group_ids":[],"observed_at":"2026-01-15T12:00:00Z"});
    let source = h
        .call(
            "POST",
            &format!("{base}/sources"),
            Some(&writer),
            source_input.clone(),
        )
        .await;
    assert_eq!(source.0, StatusCode::OK, "{}", source.1);
    let source = source.1;
    let version = &source["version"]["id"];
    let mut input = proposal(version);
    let url = format!("{base}/claims");
    assert_eq!(
        h.call("POST", &url, Some(&reader), input.clone()).await.0,
        StatusCode::FORBIDDEN
    );
    for field in ["review", "reviewer_id", "acceptance_policy"] {
        let mut forged = input.clone();
        forged[field] = json!("accepted");
        assert!(
            h.call("POST", &url, Some(&writer), forged)
                .await
                .0
                .is_client_error()
        );
    }
    let mut invalid = input.clone();
    invalid["content"]["supports"][0]["line_to"] = json!(50);
    assert_eq!(
        h.call("POST", &url, Some(&writer), invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut invalid = input.clone();
    invalid["content"]["value"] = json!(h.state.config.owner_password);
    assert_eq!(
        h.call("POST", &url, Some(&writer), invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    let key = Uuid::new_v4().to_string();
    let (a, b) = tokio::join!(
        h.keyed("POST", &url, Some(&writer), input.clone(), Some(&key)),
        h.keyed("POST", &url, Some(&writer), input.clone(), Some(&key))
    );
    assert_eq!(a.0, StatusCode::OK, "{}", a.1);
    assert_eq!(b.1, a.1);
    let first = a.1;
    assert_eq!(first["review"], "proposed");
    assert_eq!(first["origin"], "browser_authored");
    assert!(first["reviewer_id"].is_null());
    let claim = first["claim_id"].as_str().unwrap();
    let detail_url = format!("{url}/{claim}");
    let detail = h.call("GET", &detail_url, Some(&reader), Value::Null).await;
    assert_eq!(detail.0, StatusCode::OK, "{}", detail.1);
    assert_eq!(detail.1["selected"]["eligibility"]["investigation"], true);
    assert_eq!(
        detail.1["selected"]["eligibility"]["strict_accepted"],
        false
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("{url}?knowledge_at=2026-01-01T00:00:00Z"),
            Some(&reader),
            Value::Null
        )
        .await
        .1["items"],
        json!([])
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("{url}?fact_at=2026-01-15T00:00:00Z"),
            Some(&reader),
            Value::Null
        )
        .await
        .1["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("{url}?fact_at=2026-02-01T00:00:00Z"),
            Some(&reader),
            Value::Null
        )
        .await
        .1["items"],
        json!([])
    );
    input["base_revision"] = first["id"].clone();
    input["content"]["value"] = json!("OIDC");
    let mut stale = input.clone();
    stale["content"]["value"] = json!("Different competing edit");
    let (a, b) = tokio::join!(
        h.call("PUT", &detail_url, Some(&writer), input.clone()),
        h.call("PUT", &detail_url, Some(&writer), stale)
    );
    assert!(
        (a.0 == StatusCode::OK && b.0 == StatusCode::CONFLICT)
            || (b.0 == StatusCode::OK && a.0 == StatusCode::CONFLICT)
    );
    let second = if a.0 == StatusCode::OK { a.1 } else { b.1 };
    assert!(
        parse(second["recorded_at"].as_str().unwrap())
            > parse(first["recorded_at"].as_str().unwrap())
    );
    let historical = h
        .call(
            "GET",
            &format!(
                "{url}?knowledge_at={}&fact_at=2026-01-15T00:00:00Z",
                first["recorded_at"].as_str().unwrap()
            ),
            Some(&reader),
            Value::Null,
        )
        .await;
    assert_eq!(historical.0, StatusCode::OK, "{}", historical.1);
    assert_eq!(
        historical.1["items"][0]["revision"]["content"]["value"],
        "JWT"
    );
    assert_eq!(
        historical.1["items"][0]["knowledge_until"],
        second["recorded_at"]
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("{url}?fact_at=2026-01-15T00:00:00Z"),
            Some(&reader),
            Value::Null
        )
        .await
        .1["items"][0]["revision"]["content"]["value"],
        second["content"]["value"]
    );
    let mut r: ClaimRevision = serde_json::from_value(first.clone()).unwrap();
    r.content.operational = "verified".into();
    r.content.observed_at = Some(parse("2026-01-15T00:00:00Z"));
    r.content.observation = "Observed fixture response".into();
    assert!(
        !policy::eligibility(&r, assessment()).strict_operational,
        "Authored assessment cannot create acceptance"
    );
    r.review = "accepted".into();
    assert!(
        !policy::eligibility(&r, assessment()).strict_accepted,
        "An accepted label without reviewer or policy attribution is insufficient"
    );
    r.reviewer_id = Some(writer_id);
    assert!(
        policy::eligibility(&r, assessment()).strict_operational,
        "Positive accepted eligibility control"
    );
    let mut changed = assessment();
    changed.evidence_changed = true;
    assert!(!policy::eligibility(&r, changed).strict_accepted);
    let point = FactValidity {
        kind: "point".into(),
        from: Some(parse("2026-01-15T12:34:10Z")),
        to: None,
        precision: "minute".into(),
    };
    assert_eq!(
        policy::fact_match(&point, Some(parse("2026-01-15T12:34:59Z"))),
        TimeMatch::Matches
    );
    assert_eq!(
        policy::fact_match(&point, Some(parse("2026-01-15T12:35:00Z"))),
        TimeMatch::Outside
    );
    let open = FactValidity {
        kind: "interval".into(),
        from: Some(parse("2026-01-01T00:00:00Z")),
        to: None,
        precision: "second".into(),
    };
    assert_eq!(
        policy::fact_match(&open, Some(parse("2026-01-15T00:00:00Z"))),
        TimeMatch::Unknown
    );
    let unknown = FactValidity {
        kind: "unknown".into(),
        from: None,
        to: None,
        precision: "unknown".into(),
    };
    assert_eq!(
        policy::fact_match(&unknown, Some(parse("2026-01-15T00:00:00Z"))),
        TimeMatch::Unknown
    );
    // Supporting document updates qualify current knowledge, but preserve the earlier knowledge view.
    let mut next_source = source_input.clone();
    next_source["base_version"] = version.clone();
    next_source["content"] = json!("Later document evidence.\n");
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/sources/{}/versions", source["id"].as_str().unwrap()),
            Some(&writer),
            next_source
        )
        .await
        .0,
        StatusCode::OK
    );
    let detail = h
        .call("GET", &detail_url, Some(&reader), Value::Null)
        .await
        .1;
    assert_eq!(
        detail["selected"]["eligibility"]["effective_freshness"],
        "needs_verification"
    );
    let historical = h
        .call(
            "GET",
            &format!(
                "{detail_url}?knowledge_at={}",
                first["recorded_at"].as_str().unwrap()
            ),
            Some(&reader),
            Value::Null,
        )
        .await
        .1;
    assert_eq!(
        historical["selected"]["eligibility"]["effective_freshness"],
        "current"
    );
    let evidence_url = format!(
        "{base}/claim-evidence/source_version/{}",
        version.as_str().unwrap()
    );
    let artifact: Uuid = sqlx::query_scalar("SELECT artifact_id FROM source_versions WHERE id=$1")
        .bind(version.as_str().unwrap().parse::<Uuid>().unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let path = artifacts::path(&h.state.config.artifact_dir, brain_id, artifact);
    let bytes = tokio::fs::read(&path).await.unwrap();
    tokio::fs::write(&path, vec![0xff; bytes.len()])
        .await
        .unwrap();
    let unreadable = h
        .call("GET", &detail_url, Some(&reader), Value::Null)
        .await
        .1;
    assert_eq!(
        unreadable["selected"]["evidence"][0]["availability"],
        "unreadable"
    );
    tokio::fs::remove_file(&path).await.unwrap();
    assert_eq!(
        h.call("GET", &evidence_url, Some(&reader), Value::Null)
            .await
            .1["evidence"]["availability"],
        "unavailable"
    );
    let absent = h
        .call(
            "GET",
            &format!(
                "{detail_url}?knowledge_at={}",
                first["recorded_at"].as_str().unwrap()
            ),
            Some(&reader),
            Value::Null,
        )
        .await
        .1;
    assert_eq!(
        absent["selected"]["eligibility"]["effective_freshness"],
        "needs_verification"
    );
    tokio::fs::write(&path, bytes).await.unwrap();
    let evidence = h
        .call("GET", &evidence_url, Some(&reader), Value::Null)
        .await
        .1;
    assert!(
        evidence["text"]
            .as_str()
            .unwrap()
            .contains("January uses JWT")
    );
    let mut reference = source_input.clone();
    reference["retain_content"] = json!(false);
    reference["content"] = Value::Null;
    let reference = h
        .call("POST", &format!("{base}/sources"), Some(&writer), reference)
        .await
        .1;
    let mut reference_claim = proposal(&reference["version"]["id"]);
    reference_claim["content"]["supports"][0]["line_from"] = Value::Null;
    reference_claim["content"]["supports"][0]["line_to"] = Value::Null;
    let reference_claim = h.call("POST", &url, Some(&writer), reference_claim).await;
    assert_eq!(reference_claim.0, StatusCode::OK, "{}", reference_claim.1);
    let ref_detail = h
        .call(
            "GET",
            &format!("{url}/{}", reference_claim.1["claim_id"].as_str().unwrap()),
            Some(&reader),
            Value::Null,
        )
        .await
        .1;
    assert_eq!(
        ref_detail["selected"]["evidence"][0]["availability"],
        "reference_only"
    );
    // Paired authoring preserves the original write operation and rejects omitted/wrong scope.
    let (device, token) = h.pair_device(&writer, "Memory fixture").await;
    let task=h.bearer("POST",&format!("{base}/workspace/tasks"),&token,json!({"label":"Claims","selection":{"repository_ids":[],"area_ids":[],"environment_id":null}})).await.1;
    let op = h
        .bearer(
            "POST",
            &format!(
                "{base}/workspace/tasks/{}/operations",
                task["task"]["id"].as_str().unwrap()
            ),
            &token,
            json!({"kind":"write"}),
        )
        .await
        .1;
    let mut device_input = proposal(version);
    assert_eq!(
        h.bearer("POST", &url, &token, device_input.clone()).await.0,
        StatusCode::FORBIDDEN
    );
    device_input["operation_id"] = op["id"].clone();
    let area = h
        .call(
            "POST",
            &format!("{base}/evidence/groups"),
            Some(&writer),
            json!({"kind":"area","name":"Separate operation scope"}),
        )
        .await
        .1["id"]
        .clone();
    let mut wrong_scope = device_input.clone();
    wrong_scope["content"]["selection"]["area_ids"] = json!([area]);
    assert_eq!(
        h.bearer("POST", &url, &token, wrong_scope).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call("POST", &url, Some(&writer), device_input.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let native = h.bearer("POST", &url, &token, device_input.clone()).await;
    assert_eq!(native.0, StatusCode::OK, "{}", native.1);
    assert_eq!(native.1["origin"], "device_authored");
    assert_eq!(native.1["device_id"], json!(device));
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
        h.bearer("POST", &url, &token, device_input).await.0,
        StatusCode::UNAUTHORIZED
    );
    // Immutable evidence/claim state is protected by the application role as well as API checks.
    let mut tx = db::actor_tx(&h.state.pool, writer_id).await.ok().unwrap();
    assert!(
        sqlx::query("UPDATE claim_revisions SET revision='{}' WHERE brain_id=$1")
            .bind(brain_id)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        h.call(
            "DELETE",
            &format!("{base}/grants/{writer_id}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    assert!(
        h.keyed("POST", &url, Some(&writer), proposal(version), Some(&key))
            .await
            .0
            .is_client_error()
    );
    assert_eq!(
        h.call("GET", &detail_url, Some(&writer), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call("GET", &evidence_url, Some(&writer), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let mut tx = db::actor_tx(&h.state.pool, writer_id).await.ok().unwrap();
    let hidden: i64 = sqlx::query_scalar("SELECT count(*) FROM claim_revisions WHERE brain_id=$1")
        .bind(brain_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(hidden, 0);
    tx.commit().await.unwrap();
    assert_eq!(
        h.call("PATCH", &base, Some(&owner), json!({"archived":true}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call("POST", &url, Some(&owner), proposal(version))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM claim_revisions WHERE brain_id=$1),(SELECT count(*) FROM mutation_audit WHERE brain_id=$1 AND action='claim.propose'),(SELECT count(*) FROM jobs j JOIN mutation_audit a ON a.id=j.audit_id WHERE a.brain_id=$1 AND a.action='claim.propose')").bind(brain_id).fetch_one(&h.admin).await.unwrap();
    assert_eq!(counts, (4, 4, 4));
    h.finish().await;
}
