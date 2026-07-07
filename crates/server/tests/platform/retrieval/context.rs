use super::super::review::proposal;
use super::*;

async fn processed(h: &Harness) {
    while worker::run_once(&h.state, "capture").await.unwrap() {}
}
async fn excerpt(h: &Harness, base: &str, owner: &Login, parent: &Value) -> Value {
    ok(h,"POST",&format!("{base}/excerpts"),owner,json!({"source_id":parent["id"],"version_id":parent["version"]["id"],"first_line":1,"last_line":1,"title":"Lineage excerpt"})).await
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn recall_context_lineage_preserves_retained_excerpts_all_supports_and_depth() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Source lineage recall"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let parent = ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Parent document","media_type":"text/plain","content":"Lineage retained explanation.\nREMOVED_PARENT_ONLY\n","retain_content":true,"retention_class":"raw_session"})).await;
    let child = excerpt(&h, &base, &owner, &parent).await;
    let nested = excerpt(&h, &base, &owner, &child).await;
    let independent = source(
        &h,
        &base,
        &owner,
        "Independent lineage document",
        "Lineage independent corroboration is not inferred.\n",
    )
    .await;
    let mut input = proposal(
        &nested["version"]["id"],
        "Lineage multi support claim",
        "retained",
    );
    input["content"]["supports"].as_array_mut().unwrap().push(json!({"kind":"source_version","id":independent["version"]["id"],"line_from":null,"line_to":null}));
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
    processed(&h).await;
    let query = json!({"query":"Lineage","context_bytes":32768,"limit":20});
    let all = recall(&h, &base, &owner, query.clone()).await;
    assert_eq!(all["context_selection"]["distinct_source_groups"], 2);
    assert_eq!(all["context_selection"]["unknown_lineage_items"], 0);
    assert!(has(&all, &claim["claim_id"]));
    assert!(has(&all, &nested["version"]["id"]));
    assert_eq!(
        all["context"]["items"].as_array().unwrap().len(),
        5,
        "Coverage preference retains neighboring depth"
    );
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '1000 days' WHERE id=$1",
    )
    .bind(
        parent["version"]["id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .execute(&h.admin)
    .await
    .unwrap();
    let expired = recall(&h, &base, &owner, query.clone()).await;
    assert!(!has(&expired, &parent["version"]["id"]));
    assert!(has(&expired, &nested["version"]["id"]));
    assert_eq!(expired["context_selection"]["distinct_source_groups"], 2);
    assert!(!expired.to_string().contains("REMOVED_PARENT_ONLY"));
    let exact = recall(&h,&base,&owner,json!({"exact":{"kind":"claim","id":claim["claim_id"]},"channels":["exact"],"context_bytes":32768})).await;
    assert_eq!(
        exact["context_selection"]["distinct_source_groups"], 2,
        "A multi-support claim covers every direct root"
    );
    // Corrupt optional metadata must not invent a new source, lose otherwise
    // retained evidence, or loop forever. Restore it before erasure proof.
    sqlx::query("UPDATE source_excerpts SET parent_version_id=version_id,parent_source_id=$2 WHERE version_id=$1")
        .bind(nested["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .bind(nested["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
    let unknown = recall(&h, &base, &owner, query.clone()).await;
    assert!(has(&unknown, &nested["version"]["id"]));
    assert_eq!(unknown["context_selection"]["unknown_lineage_items"], 2);
    assert_eq!(unknown["context_selection"]["distinct_source_groups"], 2);
    sqlx::query(
        "UPDATE source_excerpts SET parent_version_id=$2,parent_source_id=$3 WHERE version_id=$1",
    )
    .bind(
        nested["version"]["id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .bind(
        child["version"]["id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .bind(child["id"].as_str().unwrap().parse::<Uuid>().unwrap())
    .execute(&h.admin)
    .await
    .unwrap();
    let target = json!({"kind":"source","id":parent["id"]});
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
    let erased = recall(&h, &base, &owner, query).await;
    assert!(!has(&erased, &nested["version"]["id"]));
    assert!(!has(&erased, &child["version"]["id"]));
    assert!(has(&erased, &independent["version"]["id"]));
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn recall_context_preference_reserves_coverage_without_losing_small_matching_fragments() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Context admission recall"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let first = format!("Budgetneedle {}", "Budgetneedle padding ".repeat(300));
    let text = format!(
        "{}\nBudgetneedle short positive fragment.\n",
        &first[..4095]
    );
    let split = source(&h, &base, &owner, "Two fragments", &text).await;
    processed(&h).await;
    let narrow=recall(&h,&base,&owner,json!({"query":"Budgetneedle","channels":["exact","lexical"],"context_bytes":2048,"source_diversity":false})).await;
    assert!(has(&narrow, &split["version"]["id"]), "{narrow}");
    assert_eq!(
        narrow["context"]["items"][0]["provenance"][0]["byte_from"],
        4096
    );
    assert!(
        narrow["context"]["items"][0]["text"]
            .as_str()
            .unwrap()
            .contains("short positive")
    );
    let shared = source(
        &h,
        &base,
        &owner,
        "Shared support",
        "A retained declaration for coverage claims.",
    )
    .await;
    let independent = source(
        &h,
        &base,
        &owner,
        "Independent support",
        "Another retained declaration.",
    )
    .await;
    for i in 0..8 {
        let mut input = proposal(
            &shared["version"]["id"],
            &format!("Coverage coverage ranked {i}"),
            "candidate",
        );
        if i == 7 {
            input["content"]["subject"] = json!("Other eligible result");
            input["content"]["value"] = json!("Coverage");
            input["content"]["supports"][0]["id"] = independent["version"]["id"].clone();
        }
        let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
        ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":claim["id"],"action":"accept","reason":"Checked synthetic declared support"})).await;
    }
    processed(&h).await;
    let mut request = json!({"query":"Coverage","mode":"strict_accepted","context_bytes":32768,"limit":4,"source_diversity":false});
    let plain = recall(&h, &base, &owner, request.clone()).await;
    assert_eq!(
        plain["context_selection"]["distinct_source_groups"], 1,
        "{plain}"
    );
    request["source_diversity"] = json!(true);
    let diverse = recall(&h, &base, &owner, request).await;
    assert_eq!(
        diverse["context_selection"]["distinct_source_groups"], 2,
        "{diverse}"
    );
    assert_eq!(
        diverse["context"]["items"].as_array().unwrap().len(),
        4,
        "Remaining slots preserve depth"
    );
    h.finish().await;
}
