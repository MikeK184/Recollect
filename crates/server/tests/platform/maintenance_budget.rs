use super::*;
use recollect_server::{autonomous, db, semantic};
use std::time::Duration;

async fn count(h: &Harness, table: &str, brain: Uuid) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE brain_id=$1"))
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}

/// Contract: [autonomous maintenance](../../../../../docs/contracts/memory-autonomous-maintenance.md)
/// requires unavailable work to stay visible "while other work proceeds". A Brain
/// whose admission lock is held elsewhere waits only its lock timeout, leaves no
/// partial rows, and cannot stop another Brain's discovery or its own later work.
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn a_brain_with_a_held_admission_lock_cannot_stall_semantic_or_autonomous_maintenance() {
    let (h, owner, first, _provider, server) = setup().await;
    let second = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Independent maintenance Brain"}),
    )
    .await;
    let second = format!("/api/brains/{}", second["id"].as_str().unwrap());
    for base in [&first, &second] {
        allow(&h, &owner, base, |p| {
            p["automatic_embedding"] = json!(true);
            p["autonomous_memory"] = json!(true);
            p["purposes"] = json!(["extraction", "synthesis", "embedding"]);
            p["content_classes"] = json!(["document", "claim", "query"]);
        })
        .await;
        source(
            &h,
            &owner,
            base,
            "Amber listens on port 8080 in the declared configuration.\n",
        )
        .await;
    }
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let (blocked, free) = (
        context(&h, &first).await.brain,
        context(&h, &second).await.brain,
    );
    let actor = context(&h, &first).await.actor;

    // The exact fair-admission lock that every Brain writer takes first.
    let mut hold = db::actor_tx(&h.state.pool, actor)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    db::lock_brain(&mut hold, blocked, true)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));

    let started = std::time::Instant::now();
    let queued = tokio::time::timeout(Duration::from_secs(30), semantic::run_once(&h.state))
        .await
        .expect("The held Brain is bounded by its lock timeout, not the pass")
        .unwrap_or_else(|e| panic!("Semantic maintenance: {}", e.1));
    assert!(queued >= 1, "The free Brain still queues its batches");
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "The held Brain must cost its lock timeout, not stall the pass"
    );
    assert!(count(&h, "semantic_entries", free).await > 0);
    assert!(count(&h, "semantic_batches", free).await > 0);
    for table in ["semantic_entries", "semantic_batches", "semantic_profiles"] {
        assert_eq!(
            count(&h, table, blocked).await,
            0,
            "{table}: the timed-out Brain must leave no partial rows"
        );
    }

    let queued = autonomous::run_once(&h.state)
        .await
        .unwrap_or_else(|e| panic!("Autonomous maintenance: {}", e.1));
    assert_eq!(queued, 1, "Only the free Brain's catch-up is queued");
    assert_eq!(count(&h, "learning_runs", free).await, 1);
    assert_eq!(count(&h, "learning_runs", blocked).await, 0);
    let cursors: i64 =
        sqlx::query_scalar("SELECT count(*) FROM autonomous_maintenance_cursors WHERE brain_id=$1")
            .bind(blocked)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        cursors, 0,
        "No scan progress may be recorded without publication"
    );

    // Release the lock: the same passes now serve the previously blocked Brain,
    // and neither pass duplicates the free Brain's work.
    hold.rollback().await.unwrap();
    semantic::run_once(&h.state)
        .await
        .unwrap_or_else(|e| panic!("Semantic maintenance: {}", e.1));
    assert!(count(&h, "semantic_entries", blocked).await > 0);
    assert_eq!(
        autonomous::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("Autonomous maintenance: {}", e.1)),
        1
    );
    assert_eq!(count(&h, "learning_runs", blocked).await, 1);
    assert_eq!(count(&h, "learning_runs", free).await, 1);
    server.abort();
    h.finish().await;
}
