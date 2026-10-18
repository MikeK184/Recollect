use super::*;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn managed_skills_catalogue_capacity_is_explicit_and_counts_disabled() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (status, brain, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Skills capacity"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let id = brain["id"].as_str().unwrap();
    let path = format!("/api/brains/{id}/skills");
    let body = |number: usize| json!({"name":format!("guide-{number}"),"description":"Shared engineering instructions.","kind":"instructions","files":[{"path":"AGENTS.md","content":"Use exact documentation and existing tool permissions."}],"environment_id":null,"profile_ids":[],"tags":[],"enabled":number!=0,"expected_version":null});
    for i in 0..63 {
        let result = h.call("POST", &path, Some(&owner), body(i)).await;
        assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    }
    let (one, two) = tokio::join!(
        h.call("POST", &path, Some(&owner), body(63)),
        h.call("POST", &path, Some(&owner), body(64))
    );
    assert!(matches!(
        (one.0, two.0),
        (StatusCode::OK, StatusCode::TOO_MANY_REQUESTS)
            | (StatusCode::TOO_MANY_REQUESTS, StatusCode::OK)
    ));
    let shown = h.call("GET", &path, Some(&owner), Value::Null).await;
    assert_eq!(shown.0, StatusCode::OK);
    assert_eq!(shown.1.as_array().unwrap().len(), 63);
    let all = h
        .call(
            "GET",
            &format!("{path}?include_disabled=true"),
            Some(&owner),
            Value::Null,
        )
        .await;
    assert_eq!(all.0, StatusCode::OK);
    assert_eq!(all.1.as_array().unwrap().len(), 64);
    assert!(serde_json::to_vec(&all.1).unwrap().len() < 1024 * 1024 - 1024);
    h.finish().await;
}
