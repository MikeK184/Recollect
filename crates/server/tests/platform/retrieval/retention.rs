use super::*;

async fn accept(h: &Harness, base: &str, owner: &Login, claim: &Value) -> Value {
    ok(h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),owner,
        json!({"base_revision":claim["id"],"action":"accept","reason":"Accept the supported synthetic fixture."})).await["claims"][0]["revision"].clone()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn strict_handover_recall_rechecks_contributor_expiry_before_returning() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Recall deadline proof"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let source = source(
        &h,
        &base,
        &owner,
        "Retained configuration",
        "Quartz configuration 8080. Cobalt configuration blue.\n",
    )
    .await;
    let input = super::super::review::proposal(&source["version"]["id"], "Quartz", "8080");
    let original = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
    let contribution = accept(&h, &base, &owner, &original).await;
    let mut summary = super::super::review::proposal(
        &source["version"]["id"],
        "Summary",
        "The linked claim describes Quartz.",
    );
    summary["content"]["kind"] = json!("handover");
    summary["content"]["handover"] = json!({"completed":["Recorded the declaration."],"next_steps":["Inspect retained evidence."],"risks":["Declared configuration only."],"contributions":[contribution["id"]]});
    let summary = ok(&h, "POST", &format!("{base}/claims"), &owner, summary).await;
    let summary = accept(&h, &base, &owner, &summary).await;
    let control = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        super::super::review::proposal(&source["version"]["id"], "Cobalt", "blue"),
    )
    .await;
    let control = accept(&h, &base, &owner, &control).await;
    let policy = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut update = json!({"base_change":policy["change_id"],"policy":policy["policy"]});
    update["policy"]["claim_days"] = json!(1);
    ok(&h, "PUT", &format!("{base}/retention"), &owner, update).await;
    let request = json!({"query":"Summary OR Cobalt","mode":"strict_accepted"});
    let before = recall(&h, &base, &owner, request.clone()).await;
    assert!(has(&before, &summary["claim_id"]));
    assert!(has(&before, &control["claim_id"]));
    assert!(before["expires_at"].is_string());
    // Delay the final epoch read through an isolated fixture view. No production
    // test hook is needed; canonical reads complete before a slow database read
    // carries this request over the contributor's deadline.
    sqlx::raw_sql(
        "ALTER TABLE memory_epochs RENAME TO recall_epoch_fixture;
        CREATE VIEW memory_epochs WITH (security_invoker=true) AS
          SELECT e.* FROM recall_epoch_fixture e CROSS JOIN LATERAL (SELECT pg_sleep(1.2)) delay;
        GRANT SELECT ON memory_epochs TO recollect_app;",
    )
    .execute(&h.admin)
    .await
    .unwrap();
    sqlx::query("WITH origin AS (SELECT clock_timestamp()-interval '1 day'+interval '500 milliseconds' AS at),
        times AS (SELECT r.id,origin.at-CASE WHEN r.id=$2 THEN interval '0 seconds' ELSE interval '1 second' END AS at
          FROM claim_revisions r CROSS JOIN origin WHERE r.claim_id=$1)
        UPDATE claim_revisions r SET recorded_at=t.at,revision=jsonb_set(r.revision,'{recorded_at}',to_jsonb(t.at)) FROM times t WHERE r.id=t.id")
        .bind(contribution["claim_id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .bind(contribution["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .execute(&h.admin).await.unwrap();
    let crossed = recall(&h, &base, &owner, request).await;
    assert!(!has(&crossed, &summary["claim_id"]));
    assert!(has(&crossed, &control["claim_id"]));
    // The discarded handover's expired contribution cannot make the retained
    // positive control appear expired in the browser response.
    assert!(
        chrono::DateTime::parse_from_rfc3339(crossed["expires_at"].as_str().unwrap()).unwrap()
            > chrono::Utc::now()
    );
    assert!(
        crossed["coverage"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("content_expired_during_recall"))
    );
    for mode in ["investigation", "history"] {
        let qualified = recall(
            &h,
            &base,
            &owner,
            json!({"exact":{"kind":"claim","id":summary["claim_id"]},"mode":mode}),
        )
        .await;
        assert!(
            !has(&qualified, &summary["claim_id"]),
            "A reviewed parent cannot make an expired required contributor usable model context"
        );
    }
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn legacy_claims_and_expired_support_preserve_canonical_recall_eligibility() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Recall retention boundaries"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let raw = ok(&h, "POST", &format!("{base}/sources"), &owner,
        json!({"title":"Expiring raw support","media_type":"text/plain","content":"Amber configuration 8080.\n","retain_content":true,"retention_class":"raw_session"})).await;
    let raw_id: Uuid = raw["version"]["id"].as_str().unwrap().parse().unwrap();
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '3 days' WHERE id=$1",
    )
    .bind(raw_id)
    .execute(&h.admin)
    .await
    .unwrap();
    let proposal = super::super::review::proposal(&raw["version"]["id"], "Amber", "8080");
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, proposal).await;
    let claim_id: Uuid = claim["claim_id"].as_str().unwrap().parse().unwrap();
    // Original claim payloads predate the lifecycle field. Rust defaults them
    // to active; a SQL eligibility prefilter must preserve that same meaning.
    sqlx::query("UPDATE claim_revisions SET revision=(revision-'lifecycle') #- '{content,rationale}' WHERE claim_id=$1")
        .bind(claim_id).execute(&h.admin).await.unwrap();
    let exact = json!({"kind":"claim","id":claim["claim_id"]});
    // Legacy payload shape does not bypass the independent support gate.
    assert!(!has(
        &recall(&h, &base, &owner, json!({"exact":exact})).await,
        &claim["claim_id"]
    ));
    assert!(!has(
        &recall(
            &h,
            &base,
            &owner,
            json!({"exact":exact,"mode":"strict_accepted"})
        )
        .await,
        &claim["claim_id"]
    ));
    ok(&h,"POST",&format!("{base}/claims/{claim_id}/review"),&owner,
        json!({"base_revision":claim["id"],"action":"accept","reason":"Accept the retained synthetic declaration."})).await;
    sqlx::query("UPDATE claim_revisions SET revision=(revision-'lifecycle') #- '{content,rationale}' WHERE claim_id=$1")
        .bind(claim_id).execute(&h.admin).await.unwrap();
    assert!(has(
        &recall(
            &h,
            &base,
            &owner,
            json!({"exact":exact,"mode":"strict_accepted"})
        )
        .await,
        &claim["claim_id"]
    ));
    let independent = source(
        &h,
        &base,
        &owner,
        "Independent Cobalt evidence",
        "Cobalt configuration blue.\n",
    )
    .await;
    let control = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        super::super::review::proposal(&independent["version"]["id"], "Cobalt", "blue"),
    )
    .await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",control["claim_id"].as_str().unwrap()),&owner,
        json!({"base_revision":control["id"],"action":"accept","reason":"Independent retained positive control."})).await;
    let policy = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut update = json!({"base_change":policy["change_id"],"policy":policy["policy"]});
    update["policy"]["raw_session_days"] = json!(2);
    ok(&h, "PUT", &format!("{base}/retention"), &owner, update).await;
    for after_sweep in [false, true] {
        if after_sweep {
            while worker::run_once(&h.state, "recall-retention-proof")
                .await
                .unwrap()
            {}
        }
        let canonical = ok(
            &h,
            "GET",
            &format!("{base}/claims/{claim_id}"),
            &owner,
            Value::Null,
        )
        .await;
        assert_eq!(canonical["selected"]["eligibility"]["investigation"], true);
        assert_eq!(
            canonical["selected"]["eligibility"]["strict_accepted"],
            false
        );
        for mode in ["investigation", "history"] {
            let result = recall(&h, &base, &owner, json!({"exact":exact,"mode":mode})).await;
            assert!(
                has(&result, &claim["claim_id"]),
                "Retained claim before/after sweep {after_sweep}: {result}"
            );
            let item = &result["context"]["items"][0];
            assert_eq!(item["provenance"][0]["availability"], "expired");
            assert!(
                item["qualifications"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("retained_evidence_unavailable"))
            );
            assert!(
                !result["coverage"]["reasons"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("content_expired_during_recall"))
            );
            let raw_result = recall(
                &h,
                &base,
                &owner,
                json!({"exact":{"kind":"source_version","id":raw_id},"mode":mode}),
            )
            .await;
            assert!(
                raw_result["context"]["items"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
        let strict = recall(
            &h,
            &base,
            &owner,
            json!({"query":"Amber OR Cobalt","mode":"strict_accepted"}),
        )
        .await;
        assert!(!has(&strict, &claim["claim_id"]));
        assert!(has(&strict, &control["claim_id"]));
    }
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn investigation_deadlines_cover_live_evidence_without_expiring_independent_context() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Investigation display deadlines"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let raw = ok(&h,"POST",&format!("{base}/sources"),&owner,
        json!({"title":"Deadline raw","content":"Deadline service is declared blue.\n","media_type":"text/plain","retain_content":true,"retention_class":"raw_session"})).await;
    let raw_id: Uuid = raw["version"]["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE source_versions SET created_at=clock_timestamp()-interval '30 days'+interval '5 minutes' WHERE id=$1")
        .bind(raw_id).execute(&h.admin).await.unwrap();
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        super::super::review::proposal(&raw["version"]["id"], "Deadline claim", "blue"),
    )
    .await;
    let claim = accept(&h, &base, &owner, &claim).await;
    let control = source(
        &h,
        &base,
        &owner,
        "Deadline independent",
        "Deadline evidence with independent retention.\n",
    )
    .await;
    while worker::run_once(&h.state, "investigation-deadlines")
        .await
        .unwrap()
    {}
    let detail = ok(
        &h,
        "GET",
        &format!("{base}/claim-evidence/source_version/{raw_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert!(detail["text"].as_str().unwrap().contains("declared blue"));
    assert!(detail["expires_at"].is_string());
    let reference = json!({"kind":"claim","id":claim["claim_id"]});
    let recalled = recall(&h, &base, &owner, json!({"exact":reference})).await;
    assert!(has(&recalled, &claim["claim_id"]));
    assert_eq!(recalled["expires_at"], detail["expires_at"]);
    // An earlier deadline on matched but unselected evidence does not shorten
    // an independently retained exact result's display lifetime.
    let only_control = recall(&h,&base,&owner,json!({"query":"Deadline","exact":{"kind":"source_version","id":control["version"]["id"]},"limit":1})).await;
    assert!(has(&only_control, &control["version"]["id"]));
    assert!(only_control["expires_at"].is_null());
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '31 days' WHERE id=$1",
    )
    .bind(raw_id)
    .execute(&h.admin)
    .await
    .unwrap();
    let removed = ok(
        &h,
        "GET",
        &format!("{base}/claim-evidence/source_version/{raw_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(removed["evidence"]["availability"], "expired");
    assert!(removed["text"].is_null());
    assert!(
        chrono::DateTime::parse_from_rfc3339(removed["expires_at"].as_str().unwrap()).unwrap()
            < chrono::Utc::now()
    );
    let retained_claim = recall(&h, &base, &owner, json!({"exact":reference})).await;
    assert!(has(&retained_claim, &claim["claim_id"]));
    assert_eq!(
        retained_claim["context"]["items"][0]["provenance"][0]["availability"],
        "expired"
    );
    assert!(retained_claim["expires_at"].is_null());
    let fresh_control = recall(
        &h,
        &base,
        &owner,
        json!({"exact":{"kind":"source_version","id":control["version"]["id"]}}),
    )
    .await;
    assert!(has(&fresh_control, &control["version"]["id"]));
    h.finish().await;
}
