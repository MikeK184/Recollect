//! Force the source report to expire after the writer's first gate, inside the
//! actual audit statement. The sequence witnesses that the audit was reached
//! even though the command transaction must roll back.
use super::*;

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; controlled audit delay, zero provider calls"]
async fn organization_source_override_rechecks_deadline_after_audit() {
    let (h, owner, base, provider, server) = setup().await;
    let (brain, _, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    let report = organization_hybrid::calculate(&h, &h.state, brain).await;
    let page = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"}}),
    )
    .await;
    let topic = &page["topics"][0];
    let topic_id: Uuid = topic["id"].as_str().unwrap().parse().unwrap();
    sqlx::raw_sql("CREATE SEQUENCE fixture_source_audit_entered; CREATE FUNCTION fixture_source_audit_delay() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN IF NEW.action='organization.topic' THEN PERFORM nextval('fixture_source_audit_entered'); PERFORM pg_sleep(0.6); END IF; RETURN NEW; END $$; CREATE TRIGGER fixture_source_audit_delay BEFORE INSERT ON mutation_audit FOR EACH ROW EXECUTE FUNCTION fixture_source_audit_delay()")
        .execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE analytics_reports SET expires_at=clock_timestamp()+interval '500 milliseconds' WHERE id=$1")
        .bind(report).execute(&h.admin).await.unwrap();
    let response = h
        .keyed(
            "PUT",
            &format!("{base}/organization/topics/{topic_id}"),
            Some(&owner),
            json!({"base_change_id":topic["change_id"],"change":{"action":"pin","pinned":true}}),
            Some("source-audit-expiry"),
        )
        .await;
    assert_eq!(response.0, StatusCode::CONFLICT, "{}", response.1);
    assert_eq!(response.1["code"], "organization_changed");
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT is_called FROM fixture_source_audit_entered")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "The source deadline must cross after preparation, at the actual audit"
    );
    let unchanged: (bool, Uuid) =
        sqlx::query_as("SELECT pinned,change_id FROM knowledge_topics WHERE id=$1")
            .bind(topic_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(!unchanged.0);
    assert_eq!(
        unchanged.1.to_string(),
        topic["change_id"].as_str().unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM command_receipts WHERE key='source-audit-expiry'"
        )
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0,
        "An expired command cannot leave a successful replay receipt"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM mutation_audit WHERE action='organization.topic' AND target_id=$1"
        )
        .bind(topic_id)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    server.abort();
    h.finish().await;
}
