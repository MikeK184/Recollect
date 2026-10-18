use super::*;
use recollect_server::worker;
use review::ok;

async fn scope(h: &Harness, owner: &Login, base: &str) -> (Value, String) {
    let task = ok(h,"POST",&format!("{base}/workspace/tasks"),owner,json!({"label":"Synthetic utility task","workspace_id":null,"selection":{"repository_ids":[],"area_ids":[],"environment_id":null}})).await;
    let task_url = format!(
        "{base}/workspace/tasks/{}",
        task["task"]["id"].as_str().unwrap()
    );
    let op = ok(
        h,
        "POST",
        &format!("{task_url}/operations"),
        owner,
        json!({"kind":"retrieval"}),
    )
    .await;
    (
        json!({"kind":"knowledge","operation_id":op["id"],"selection":op["scope"]["selection"],"mode":"investigation","relations":[]}),
        task_url,
    )
}

pub(super) async fn seed_for_version(
    h: &Harness,
    owner: &Login,
    base: &str,
    version: Uuid,
) -> Value {
    seed_for_input(h, owner, base, "source_version", version).await
}

async fn seed_for_input(
    h: &Harness,
    owner: &Login,
    base: &str,
    kind: &str,
    version: Uuid,
) -> Value {
    let (scope, _) = scope(h, owner, base).await;
    let input = json!({"scope":scope,"query_context_id":Uuid::new_v4(),"input_kind":kind,"revision_id":version});
    let delivery = h
        .keyed(
            "POST",
            &format!("{base}/retrieval/feedback/context"),
            Some(owner),
            input,
            Some(&Uuid::new_v4().to_string()),
        )
        .await;
    assert_eq!(delivery.0, StatusCode::OK, "{}", delivery.1);
    let command = json!({"context_id":delivery.1["feedback"]["context_id"],"base_change_id":null,"vote":"helpful"});
    let result = h
        .keyed(
            "POST",
            &format!("{base}/retrieval/feedback"),
            Some(owner),
            command,
            Some(&Uuid::new_v4().to_string()),
        )
        .await;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    result.1
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn utility_feedback_has_real_delivery_attribution_versioning_and_scope_invalidation() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let b = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic bounded utility"}),
    )
    .await;
    let base = format!("/api/brains/{}", b["id"].as_str().unwrap());
    let source=ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Synthetic feedback evidence","media_type":"text/plain","content":"Vault powers the synthetic application.","retain_content":true})).await;
    for _ in 0..20 {
        if !worker::run_once(&h.state, "capture").await.unwrap() {
            break;
        }
    }
    let (scope, task_url) = scope(&h, &owner, &base).await;
    let input = json!({"scope":scope,"query_context_id":Uuid::new_v4(),"input_kind":"source_version","revision_id":source["version"]["id"]});
    let context_url = format!("{base}/retrieval/feedback/context");
    let feedback_url = format!("{base}/retrieval/feedback");
    let delivery = h
        .keyed(
            "POST",
            &context_url,
            Some(&owner),
            input.clone(),
            Some("delivery-1"),
        )
        .await;
    assert_eq!(delivery.0, StatusCode::OK, "{}", delivery.1);
    assert_eq!(
        delivery.1["node"]["key"],
        format!(
            "source_version:{}",
            source["version"]["id"].as_str().unwrap()
        )
    );
    let repeat = h
        .keyed(
            "POST",
            &context_url,
            Some(&owner),
            input.clone(),
            Some("delivery-2"),
        )
        .await;
    assert_eq!(repeat.0, StatusCode::OK, "{}", repeat.1);
    assert_eq!(repeat.1["feedback"], delivery.1["feedback"]);
    let context = delivery.1["feedback"]["context_id"].clone();
    let vote = json!({"context_id":context,"base_change_id":null,"vote":"helpful"});
    let first = h
        .keyed(
            "POST",
            &feedback_url,
            Some(&owner),
            vote.clone(),
            Some("vote-1"),
        )
        .await;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    assert_eq!(
        h.keyed(
            "POST",
            &feedback_url,
            Some(&owner),
            vote.clone(),
            Some("vote-1")
        )
        .await
        .1,
        first.1
    );
    assert_eq!(
        h.keyed(
            "POST",
            &feedback_url,
            Some(&owner),
            vote.clone(),
            Some("vote-stale")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let same = json!({"context_id":context,"base_change_id":first.1["change_id"],"vote":"helpful"});
    assert_eq!(
        h.keyed(
            "POST",
            &feedback_url,
            Some(&owner),
            same,
            Some("vote-no-extra-reward")
        )
        .await
        .1,
        first.1
    );
    let revision =
        json!({"context_id":context,"base_change_id":first.1["change_id"],"vote":"unhelpful"});
    let revised = h
        .keyed(
            "POST",
            &feedback_url,
            Some(&owner),
            revision,
            Some("vote-2"),
        )
        .await;
    assert_eq!(revised.0, StatusCode::OK, "{}", revised.1);
    assert_ne!(revised.1["change_id"], first.1["change_id"]);
    assert_eq!(
        h.keyed(
            "POST",
            &feedback_url,
            Some(&owner),
            vote.clone(),
            Some("vote-1")
        )
        .await
        .0,
        StatusCode::CONFLICT,
        "Old idempotent result cannot overwrite/reward the new head"
    );
    let changes: i64 = sqlx::query_scalar("SELECT count(*) FROM retrieval_feedback_changes")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(changes, 2);
    let (reader_id, reader) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    assert!(
        h.keyed(
            "POST",
            &feedback_url,
            Some(&reader),
            vote.clone(),
            Some("reader-vote")
        )
        .await
        .0
        .is_client_error()
    );
    let (_, token) = h.pair_device(&owner, "Different feedback device").await;
    assert!(
        h.bearer("POST", &feedback_url, &token, vote.clone())
            .await
            .0
            .is_client_error()
    );
    let mut wrong = input.clone();
    wrong["revision_id"] = json!(Uuid::new_v4());
    assert!(
        h.keyed(
            "POST",
            &context_url,
            Some(&owner),
            wrong,
            Some("missing-input")
        )
        .await
        .0
        .is_client_error()
    );
    let receipts: Vec<Value> = sqlx::query_scalar(
        "SELECT response FROM command_receipts WHERE operation LIKE 'retrieval.feedback.%'",
    )
    .fetch_all(&h.admin)
    .await
    .unwrap();
    assert!(
        receipts
            .iter()
            .all(|v| !v.to_string().contains("Vault powers") && v.get("node").is_none())
    );
    // A new scope snapshot, even with the same selected IDs, withdraws the
    // exact task-bound delivery. No stale reward/receipt survives the change.
    let current_task = ok(&h, "GET", &task_url, &owner, Value::Null).await;
    ok(&h,"PUT",&format!("{task_url}/scope"),&owner,json!({"base_scope":current_task["task"]["scope"]["id"],"selection":{"repository_ids":[],"area_ids":[],"environment_id":null}})).await;
    let remaining:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM retrieval_feedback_contexts),(SELECT count(*) FROM retrieval_feedback_changes)").fetch_one(&h.admin).await.unwrap();
    assert_eq!(remaining, (0, 0));
    assert!(
        h.keyed(
            "POST",
            &feedback_url,
            Some(&owner),
            vote,
            Some("vote-after-scope-change")
        )
        .await
        .0
        .is_client_error()
    );
    let invalid:i64=sqlx::query_scalar("SELECT count(*) FROM command_receipts WHERE operation LIKE 'retrieval.feedback.%' AND invalidated AND input='{}' AND response IS NULL").fetch_one(&h.admin).await.unwrap();
    assert!(invalid >= 4);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn utility_feedback_claim_revisions_never_survive_correction_or_supported_source_erasure() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let b = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic exact claim utility"}),
    )
    .await;
    let brain: Uuid = b["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain}");
    let source=ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Synthetic claim support","media_type":"text/plain","content":"Vault has synthetic configuration.","retain_content":true})).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let mut proposed = review::proposal(&source["version"]["id"], "Synthetic subject", "first");
    proposed["content"]["validity"] =
        json!({"kind":"unknown","from":null,"to":null,"precision":"unknown"});
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, proposed).await;
    let review_url = format!(
        "{base}/claims/{}/review",
        claim["claim_id"].as_str().unwrap()
    );
    let accepted=ok(&h,"POST",&review_url,&owner,json!({"base_revision":claim["id"],"action":"accept","reason":"Synthetic explicit owner review.","content":null,"revalidation_basis":null})).await["claims"][0]["revision"].clone();
    let original = seed_for_input(
        &h,
        &owner,
        &base,
        "claim",
        accepted["id"].as_str().unwrap().parse().unwrap(),
    )
    .await;
    let mut corrected = accepted["content"].clone();
    corrected["value"] = json!("second");
    let changed=ok(&h,"POST",&review_url,&owner,json!({"base_revision":accepted["id"],"action":"correct","reason":"Synthetic supported correction.","content":corrected,"revalidation_basis":null})).await["claims"][0]["revision"].clone();
    let old: i64 =
        sqlx::query_scalar("SELECT count(*) FROM retrieval_feedback_contexts WHERE id=$1")
            .bind(
                original["context_id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
            )
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(old, 0);
    let (mut selected, _) = scope(&h, &owner, &base).await;
    selected["mode"] = json!("history");
    let historical = json!({"scope":selected,"query_context_id":Uuid::new_v4(),"input_kind":"claim","revision_id":accepted["id"]});
    assert!(
        h.keyed(
            "POST",
            &format!("{base}/retrieval/feedback/context"),
            Some(&owner),
            historical,
            Some("old-corrected-claim")
        )
        .await
        .0
        .is_client_error()
    );
    seed_for_input(
        &h,
        &owner,
        &base,
        "claim",
        changed["id"].as_str().unwrap().parse().unwrap(),
    )
    .await;
    let target = json!({"kind":"source","id":source["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    assert_eq!(preview["retrieval_feedback_contexts"], 1);
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let left:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM retrieval_feedback_contexts WHERE brain_id=$1),(SELECT count(*) FROM retrieval_feedback_changes WHERE brain_id=$1)").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(left, (0, 0));
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn utility_feedback_rejects_corrected_history_and_preserves_independent_excerpt_expiry() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let b = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Synthetic utility lifecycle"}),
    )
    .await;
    let brain: Uuid = b["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain}");
    let old=ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Synthetic correction","media_type":"text/plain","content":"Vault uses a synthetic application.","retain_content":true})).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let vote = seed_for_version(
        &h,
        &owner,
        &base,
        old["version"]["id"].as_str().unwrap().parse().unwrap(),
    )
    .await;
    let new=ok(&h,"POST",&format!("{base}/sources/{}/versions",old["id"].as_str().unwrap()),&owner,json!({"title":"Synthetic corrected version","media_type":"text/plain","content":"Vault uses the corrected synthetic application.","retain_content":true})).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let (mut selected, _) = scope(&h, &owner, &base).await;
    selected["mode"] = json!("history");
    let old_request = json!({"scope":selected,"query_context_id":Uuid::new_v4(),"input_kind":"source_version","revision_id":old["version"]["id"]});
    assert!(
        h.keyed(
            "POST",
            &format!("{base}/retrieval/feedback/context"),
            Some(&owner),
            old_request,
            Some("corrected-history")
        )
        .await
        .0
        .is_client_error()
    );
    let prior: i64 =
        sqlx::query_scalar("SELECT count(*) FROM retrieval_feedback_contexts WHERE id=$1")
            .bind(
                vote["context_id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
            )
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        prior, 0,
        "A corrected source cannot retain its old utility head"
    );
    let excerpt=ok(&h,"POST",&format!("{base}/excerpts"),&owner,json!({"source_id":new["id"],"version_id":new["version"]["id"],"first_line":1,"last_line":1,"title":"Independent synthetic excerpt"})).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let parent = seed_for_version(
        &h,
        &owner,
        &base,
        new["version"]["id"].as_str().unwrap().parse().unwrap(),
    )
    .await;
    let retained = seed_for_version(
        &h,
        &owner,
        &base,
        excerpt["version"]["id"].as_str().unwrap().parse().unwrap(),
    )
    .await;
    // Real canonical expiry, not direct context deletion or a forged manifest.
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["document_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '2 days' WHERE id=$1",
    )
    .bind(
        new["version"]["id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .execute(&h.admin)
    .await
    .unwrap();
    let expired: Option<Uuid> = sqlx::query_scalar("SELECT recollect_expire_one()")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(expired.is_some());
    let contexts: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM retrieval_feedback_contexts WHERE brain_id=$1 ORDER BY id",
    )
    .bind(brain)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        contexts,
        vec![
            retained["context_id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap()
        ],
        "Natural parent expiry removes only dependent parent utility"
    );
    assert_ne!(parent["context_id"], retained["context_id"]);
    h.finish().await;
}
