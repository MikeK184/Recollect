use super::*;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn operational_diagnostics_require_owner_and_count_only_accessible_brains() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (_, member) = h.fixture_member().await;
    let (status, own, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Owned operational control"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, other, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&member),
            json!({"name":"OTHER_BRAIN_CANARY"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        h.call("GET", "/api/operations", None, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call("GET", "/api/operations", Some(&member), Value::Null)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let (status, before, _) = h
        .call("GET", "/api/operations", Some(&owner), Value::Null)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(before["queue_scope"], "accessible_brains");
    assert_eq!(before["jobs_queued"], 1);
    assert!(!before.to_string().contains("CANARY"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE state='queued'")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        2
    );
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let (status, _, _) = h
        .call(
            "PUT",
            &format!(
                "/api/brains/{}/grants/{owner_id}",
                other["id"].as_str().unwrap()
            ),
            Some(&member),
            json!({"role":"reader"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let after = h
        .call("GET", "/api/operations", Some(&owner), Value::Null)
        .await
        .1;
    assert_eq!(
        after["jobs_queued"], 2,
        "an explicit grant changes visible operational counts"
    );
    sqlx::query("UPDATE jobs SET state='failed',error_code='fixture' WHERE brain_id=$1")
        .bind(own["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .execute(&h.admin)
        .await
        .unwrap();
    let after = h
        .call("GET", "/api/operations", Some(&owner), Value::Null)
        .await
        .1;
    assert_eq!(after["jobs_queued"], 1);
    assert_eq!(after["jobs_failed"], 1);
    h.finish().await;
}
