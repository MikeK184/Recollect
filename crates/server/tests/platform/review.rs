use super::*;

#[path = "review/scoped_inspection.rs"]
mod scoped_inspection;

pub(super) async fn ok(h: &Harness, method: &str, path: &str, login: &Login, body: Value) -> Value {
    let (status, body, _) = h.call(method, path, Some(login), body).await;
    assert_eq!(status, StatusCode::OK, "{method} {path}: {body}");
    body
}
async fn source(h: &Harness, base: &str, login: &Login) -> Value {
    ok(h,"POST",&format!("{base}/sources"),login,json!({"title":"Synthetic review evidence","media_type":"text/plain","content":"Synthetic values for review proof.\n","retain_content":true,"source_uri":null,"group_ids":[],"observed_at":null})).await["version"]["id"].clone()
}
pub(super) fn proposal(version: &Value, subject: &str, value: &str) -> Value {
    json!({"base_revision":null,"operation_id":null,"content":{"kind":"claim","subject":subject,"predicate":"configuration","value":value,"rationale":"Synthetic review test.","selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"manifest_revision_id":null,"validity":{"kind":"interval","from":"2026-01-01T00:00:00Z","to":"2026-02-01T00:00:00Z","precision":"second"},"freshness":"current","operational":"declared","observed_at":null,"observation":"","supports":[{"kind":"source_version","id":version,"line_from":1,"line_to":1}]}})
}
fn url(base: &str, r: &Value) -> String {
    format!("{base}/claims/{}", r["claim_id"].as_str().unwrap())
}
fn action(r: &Value, action: &str) -> Value {
    json!({"base_revision":r["id"],"action":action,"reason":"Synthetic evidence reviewed by the signed-in account.","content":null,"revalidation_basis":null})
}
async fn reviewed(h: &Harness, base: &str, login: &Login, r: &Value, input: Value) -> Value {
    ok(h, "POST", &format!("{}/review", url(base, r)), login, input).await["claims"][0]["revision"]
        .clone()
}
async fn eligibility(h: &Harness, base: &str, login: &Login, r: &Value) -> Value {
    ok(h, "GET", &url(base, r), login, Value::Null).await["selected"]["eligibility"].clone()
}
async fn setup() -> (Harness, Login, Login, Uuid, String, Value) {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (writer_id, writer) = h.fixture_member().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic review proof"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        json!({"role":"writer"}),
    )
    .await;
    let version = source(&h, &base, &writer).await;
    (h, owner, writer, writer_id, base, version)
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn review_authority_durable_rules_revalidation_and_replay() {
    let (mut h, owner, writer, writer_id, base, version) = setup().await;
    let (reader_id, reader) = h.fixture_member().await;
    let (foreign_id, foreign) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    let input = proposal(&version, "Synthetic service", "first");
    let r = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &writer,
        input.clone(),
    )
    .await;
    let endpoint = format!("{}/review", url(&base, &r));
    let acceptance = action(&r, "accept");
    assert_eq!(
        h.call("POST", &endpoint, Some(&reader), acceptance.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert!(
        h.call("POST", &endpoint, Some(&foreign), acceptance.clone())
            .await
            .0
            .is_client_error()
    );
    let (_, token) = h.pair_device(&writer, "Review authority device").await;
    assert_eq!(
        h.bearer("POST", &endpoint, &token, acceptance.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let no_csrf = Login {
        cookie: writer.cookie.clone(),
        csrf: Uuid::new_v4().to_string(),
    };
    assert_eq!(
        h.call("POST", &endpoint, Some(&no_csrf), acceptance.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let mut forged = acceptance.clone();
    forged["reviewer_id"] = json!(writer_id);
    assert!(
        h.call("POST", &endpoint, Some(&writer), forged)
            .await
            .0
            .is_client_error()
    );
    let mut secret = acceptance.clone();
    secret["reason"] = json!(h.state.config.owner_password);
    assert_eq!(
        h.call("POST", &endpoint, Some(&writer), secret).await.0,
        StatusCode::BAD_REQUEST
    );
    let key = Uuid::new_v4().to_string();
    let (a, b) = tokio::join!(
        h.keyed(
            "POST",
            &endpoint,
            Some(&writer),
            acceptance.clone(),
            Some(&key)
        ),
        h.keyed(
            "POST",
            &endpoint,
            Some(&writer),
            acceptance.clone(),
            Some(&key)
        )
    );
    assert_eq!(a.0, StatusCode::OK, "{}", a.1);
    assert_eq!(a.1, b.1);
    let accepted = a.1["claims"][0]["revision"].clone();
    assert_eq!(accepted["reviewer_id"], json!(writer_id));
    assert_eq!(accepted["actor_id"], r["actor_id"]);
    assert_eq!(accepted["origin"], r["origin"]);
    assert_eq!(a.1["claims"][0]["eligibility"]["strict_accepted"], true);
    assert_eq!(a.1["claims"][0]["eligibility"]["strict_operational"], false);
    assert_eq!(
        h.call("POST", &endpoint, Some(&writer), acceptance.clone())
            .await
            .0,
        StatusCode::CONFLICT
    );
    let mut correction = action(&accepted, "correct");
    correction["content"] = accepted["content"].clone();
    correction["content"]["value"] = json!("replacement");
    let corrected = reviewed(&h, &base, &writer, &accepted, correction).await;
    assert_eq!(corrected["review"], "accepted");
    assert_eq!(
        eligibility(&h, &base, &reader, &corrected).await["strict_accepted"],
        true
    );
    let replay = h
        .keyed(
            "POST",
            &endpoint,
            Some(&writer),
            acceptance.clone(),
            Some(&key),
        )
        .await;
    assert_eq!(replay.1["decision"], a.1["decision"]);
    assert_eq!(
        replay.1["claims"][0]["eligibility"]["strict_accepted"],
        false
    );
    assert_eq!(replay.1["claims"][0]["eligibility"]["investigation"], false);
    let historical = ok(
        &h,
        "GET",
        &format!(
            "{}?knowledge_at={}",
            url(&base, &r),
            accepted["recorded_at"].as_str().unwrap()
        ),
        &reader,
        Value::Null,
    )
    .await;
    assert_eq!(historical["selected"]["revision"]["review"], "accepted");
    assert_eq!(
        historical["selected"]["eligibility"]["strict_accepted"],
        false
    );
    let fresh_evidence = source(&h, &base, &writer).await;
    let mut reentry = proposal(&fresh_evidence, "  SYNTHETIC   SERVICE  ", " first ");
    let blocked = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &writer,
        reentry.clone(),
    )
    .await;
    assert_eq!(blocked["admission"], "blocked_by_rule");
    assert_eq!(
        eligibility(&h, &base, &reader, &blocked).await["investigation"],
        false
    );
    // Explicitly unrelated periods and environments do not inherit the rejected assertion.
    reentry["content"]["validity"]["from"] = json!("2026-02-01T00:00:00Z");
    reentry["content"]["validity"]["to"] = json!("2026-03-01T00:00:00Z");
    let later = ok(&h, "POST", &format!("{base}/claims"), &writer, reentry).await;
    assert_eq!(later["admission"], "proposed");
    let later = reviewed(&h, &base, &writer, &later, action(&later, "accept")).await;
    assert_eq!(
        eligibility(&h, &base, &reader, &later).await["strict_accepted"],
        true
    );
    let mut changed = action(&blocked, "revalidate");
    changed["revalidation_basis"] = json!("changed_evidence");
    assert_eq!(
        h.call(
            "POST",
            &format!("{}/review", url(&base, &blocked)),
            Some(&writer),
            changed.clone()
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    changed["content"] = blocked["content"].clone();
    changed["content"]["supports"][0]["id"] = version.clone();
    let revalidated = reviewed(&h, &base, &writer, &blocked, changed).await;
    assert_eq!(revalidated["review"], "accepted");
    let history = ok(
        &h,
        "GET",
        &format!("{}/review", url(&base, &revalidated)),
        &reader,
        Value::Null,
    )
    .await;
    assert_eq!(history["exempted_rule_ids"].as_array().unwrap().len(), 1);
    // Revalidation does not settle its independent contradiction with the replacement.
    assert_eq!(
        eligibility(&h, &base, &reader, &revalidated).await["strict_accepted"],
        false
    );
    let duplicate = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &writer,
        input.clone(),
    )
    .await;
    assert_eq!(duplicate["admission"], "blocked_by_rule");
    let rejected = reviewed(
        &h,
        &base,
        &writer,
        &revalidated,
        action(&revalidated, "reject"),
    )
    .await;
    assert_eq!(rejected["review"], "rejected");
    assert_eq!(
        eligibility(&h, &base, &reader, &corrected).await["strict_accepted"],
        true
    );
    // Reconstructed application reads the durable rules, without any projection process.
    h.router = app(AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap());
    assert_eq!(
        eligibility(&h, &base, &reader, &duplicate).await["investigation"],
        false
    );
    let withdrawn = reviewed(
        &h,
        &base,
        &writer,
        &corrected,
        action(&corrected, "withdraw"),
    )
    .await;
    assert_eq!(withdrawn["review"], "accepted");
    assert_eq!(withdrawn["lifecycle"], "withdrawn");
    let mut replacement_input = input.clone();
    replacement_input["content"]["value"] = json!("replacement");
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/claims"),
            &writer,
            replacement_input
        )
        .await["admission"],
        "blocked_by_rule"
    );
    let restored = reviewed(
        &h,
        &base,
        &writer,
        &withdrawn,
        action(&withdrawn, "restore"),
    )
    .await;
    assert_eq!(
        eligibility(&h, &base, &reader, &restored).await["strict_accepted"],
        true
    );
    let brain_id: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let epoch: i64 = sqlx::query_scalar("SELECT epoch FROM memory_epochs WHERE brain_id=$1")
        .bind(brain_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(epoch > 10);
    let decisions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM memory_decisions WHERE brain_id=$1")
            .bind(brain_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_audit WHERE brain_id=$1 AND action='claim.review'",
    )
    .bind(brain_id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(decisions, audits);
    let mut tx = db::actor_tx(&h.state.pool, foreign_id).await.ok().unwrap();
    for table in [
        "memory_decisions",
        "memory_decision_claims",
        "assertion_rules",
        "memory_rule_exceptions",
        "memory_epochs",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
    tx.rollback().await.unwrap();
    let mut tx = db::actor_tx(&h.state.pool, writer_id).await.ok().unwrap();
    assert!(
        sqlx::query("UPDATE assertion_rules SET rule='{}' WHERE brain_id=$1")
            .bind(brain_id)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    assert!(
        h.call("GET", &endpoint, Some(&foreign), Value::Null)
            .await
            .0
            .is_client_error()
    );
    ok(&h, "PATCH", &base, &owner, json!({"archived":true})).await;
    assert!(
        h.keyed("POST", &endpoint, Some(&writer), acceptance, Some(&key))
            .await
            .0
            .is_client_error()
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn review_conflict_dispositions_and_atomic_stale_resolution() {
    let (h, _owner, writer, _, base, version) = setup().await;
    for disposition in ["keep_selected", "keep_both", "retract", "replace"] {
        let mut claims = Vec::new();
        for value in ["one", "two", "remaining"] {
            claims.push(
                ok(
                    &h,
                    "POST",
                    &format!("{base}/claims"),
                    &writer,
                    proposal(&version, disposition, value),
                )
                .await,
            );
        }
        let e = eligibility(&h, &base, &writer, &claims[0]).await;
        assert_eq!(e["conflicting_claim_ids"].as_array().unwrap().len(), 2);
        assert_eq!(
            h.call(
                "POST",
                &format!("{}/review", url(&base, &claims[0])),
                Some(&writer),
                action(&claims[0], "accept")
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
        let participants: Vec<Value> = claims
            .iter()
            .take(2)
            .map(|r| json!({"claim_id":r["claim_id"],"base_revision":r["id"],"content":null}))
            .collect();
        let mut body = json!({"disposition":disposition,"participants":participants,"selected_id":null,"replacement":null,"reason":"Resolve synthetic contradictory evidence."});
        if disposition == "keep_selected" {
            body["selected_id"] = claims[0]["claim_id"].clone();
        }
        if disposition == "keep_both" {
            for i in 0..2 {
                body["participants"][i]["content"] = claims[i]["content"].clone();
                body["participants"][i]["content"]["validity"] = if i == 0 {
                    json!({"kind":"interval","from":"2026-01-01T00:00:00Z","to":"2026-01-15T00:00:00Z","precision":"second"})
                } else {
                    json!({"kind":"interval","from":"2026-01-15T00:00:00Z","to":"2026-02-01T00:00:00Z","precision":"second"})
                };
            }
        }
        if disposition == "replace" {
            body["replacement"] =
                proposal(&version, disposition, "reviewed replacement")["content"].clone();
        }
        let endpoint = format!("{base}/claim-conflicts/resolve");
        let mut stale = body.clone();
        stale["participants"][1]["base_revision"] = json!(Uuid::new_v4());
        let before: i64 = sqlx::query_scalar("SELECT count(*) FROM memory_decisions")
            .fetch_one(&h.admin)
            .await
            .unwrap();
        assert_eq!(
            h.call("POST", &endpoint, Some(&writer), stale).await.0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            ok(&h, "GET", &url(&base, &claims[0]), &writer, Value::Null).await["current_revision"],
            claims[0]["id"]
        );
        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM memory_decisions")
            .fetch_one(&h.admin)
            .await
            .unwrap();
        assert_eq!(before, after);
        if disposition == "keep_both" {
            let mut overlap = body.clone();
            overlap["participants"][1]["content"]["validity"] =
                overlap["participants"][0]["content"]["validity"].clone();
            assert_eq!(
                h.call("POST", &endpoint, Some(&writer), overlap).await.0,
                StatusCode::BAD_REQUEST
            );
        }
        let key = Uuid::new_v4().to_string();
        let reply = h
            .keyed("POST", &endpoint, Some(&writer), body.clone(), Some(&key))
            .await;
        assert_eq!(reply.0, StatusCode::OK, "{disposition}: {}", reply.1);
        assert_eq!(
            h.keyed("POST", &endpoint, Some(&writer), body, Some(&key))
                .await
                .1,
            reply.1
        );
        let results = reply.1["claims"].as_array().unwrap();
        if disposition == "retract" {
            assert!(
                results
                    .iter()
                    .all(|r| r["revision"]["lifecycle"] == "withdrawn")
            );
        } else {
            let accepted: Vec<&Value> = results
                .iter()
                .filter(|r| r["revision"]["review"] == "accepted")
                .collect();
            assert_eq!(
                accepted.len(),
                if disposition == "keep_both" { 2 } else { 1 }
            );
            assert!(
                accepted
                    .iter()
                    .all(|r| r["eligibility"]["strict_accepted"] == false),
                "The remaining unselected sibling still conflicts."
            );
            reviewed(&h, &base, &writer, &claims[2], action(&claims[2], "reject")).await;
            for accepted in accepted {
                assert_eq!(
                    eligibility(&h, &base, &writer, &accepted["revision"]).await["strict_accepted"],
                    true
                );
            }
        }
    }
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn review_scope_boundaries_and_failed_audit_are_atomic() {
    let (h, _owner, writer, writer_id, base, version) = setup().await;
    let brain_id: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let mut environments = Vec::new();
    let mut repositories = Vec::new();
    for name in ["Development", "Production"] {
        environments.push(
            ok(
                &h,
                "POST",
                &format!("{base}/evidence/groups"),
                &writer,
                json!({"kind":"environment","name":name}),
            )
            .await["id"]
                .clone(),
        );
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,$3,$4)",
        )
        .bind(id)
        .bind(brain_id)
        .bind(format!("example.test/synthetic/{}", name.to_lowercase()))
        .bind(writer_id)
        .execute(&h.admin)
        .await
        .unwrap();
        repositories.push(id);
    }
    let mut input = proposal(&version, "Scoped assertion", "old value");
    input["content"]["selection"]["environment_id"] = environments[0].clone();
    input["content"]["selection"]["repository_ids"] = json!([repositories[0]]);
    let original = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &writer,
        input.clone(),
    )
    .await;
    reviewed(&h, &base, &writer, &original, action(&original, "reject")).await;
    for case in 0..7 {
        let mut next = input.clone();
        match case {
            0 => next["content"]["selection"]["environment_id"] = environments[1].clone(),
            1 => next["content"]["selection"]["repository_ids"] = json!([repositories[1]]),
            2 => {
                next["content"]["validity"] = json!({"kind":"point","from":"2026-02-01T12:30:00Z","to":null,"precision":"day"})
            }
            3 => {
                next["content"]["validity"] = json!({"kind":"point","from":"2026-01-15T12:30:00Z","to":null,"precision":"day"})
            }
            4 => {
                next["content"]["selection"] =
                    json!({"repository_ids":[],"area_ids":[],"environment_id":null});
                next["content"]["validity"] =
                    json!({"kind":"unknown","from":null,"to":null,"precision":"unknown"});
            }
            5 => next["content"]["value"] = json!("unrelated value"),
            _ => next["content"]["subject"] = json!("Unrelated subject"),
        }
        let r = ok(&h, "POST", &format!("{base}/claims"), &writer, next).await;
        assert_eq!(
            r["admission"],
            if matches!(case, 3 | 4) {
                "blocked_by_rule"
            } else {
                "proposed"
            },
            "Scope case {case}"
        );
    }
    // Inject a real transaction failure after revision/rule writes, then prove complete rollback.
    let endpoint = format!("{}/review", url(&base, &original));
    let old =
        ok(&h, "GET", &url(&base, &original), &writer, Value::Null).await["selected"]["revision"]
            .clone();
    let before:(i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM claim_revisions),(SELECT count(*) FROM memory_decisions),(SELECT count(*) FROM memory_rule_exceptions),(SELECT epoch FROM memory_epochs WHERE brain_id=$1)").bind(brain_id).fetch_one(&h.admin).await.unwrap();
    sqlx::query("ALTER TABLE mutation_audit ADD CONSTRAINT review_failure CHECK(action<>'claim.review') NOT VALID").execute(&h.admin).await.unwrap();
    let mut reconsider = action(&old, "revalidate");
    reconsider["revalidation_basis"] = json!("review_correction");
    let key = Uuid::new_v4().to_string();
    assert_eq!(
        h.keyed(
            "POST",
            &endpoint,
            Some(&writer),
            reconsider.clone(),
            Some(&key)
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let after:(i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM claim_revisions),(SELECT count(*) FROM memory_decisions),(SELECT count(*) FROM memory_rule_exceptions),(SELECT epoch FROM memory_epochs WHERE brain_id=$1)").bind(brain_id).fetch_one(&h.admin).await.unwrap();
    assert_eq!(before, after);
    assert_eq!(
        ok(&h, "GET", &url(&base, &old), &writer, Value::Null).await["current_revision"],
        old["id"]
    );
    sqlx::query("ALTER TABLE mutation_audit DROP CONSTRAINT review_failure")
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.keyed("POST", &endpoint, Some(&writer), reconsider, Some(&key))
            .await
            .0,
        StatusCode::OK
    );
    let before_epoch: i64 = sqlx::query_scalar("SELECT epoch FROM memory_epochs WHERE brain_id=$1")
        .bind(brain_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "DELETE",
            &format!(
                "{base}/evidence/groups/{}",
                environments[0].as_str().unwrap()
            ),
            Some(&writer),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let after_epoch: i64 = sqlx::query_scalar("SELECT epoch FROM memory_epochs WHERE brain_id=$1")
        .bind(brain_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(after_epoch, before_epoch + 1);
    assert!(
        eligibility(&h, &base, &writer, &original).await["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("scope_unavailable"))
    );
    h.finish().await;
}
