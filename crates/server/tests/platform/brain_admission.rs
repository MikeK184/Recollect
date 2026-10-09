use super::*;
use std::time::Duration;

fn checked<T>(result: recollect_server::error::Result<T>) -> T {
    result.unwrap_or_else(|error| panic!("{}", error.1))
}

async fn waiting(h: &Harness, pid: i32) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT cardinality(pg_blocking_pids($1))>0")
                .bind(pid)
                .fetch_one(&h.admin)
                .await
                .unwrap();
            if blocked {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the selected transaction must reach its lock wait");
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn brain_admission_waiting_writer_precedes_new_readers_without_blocking_other_brains() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let mut brains = vec![];
    for name in ["Contended Brain", "Independent Brain"] {
        let (status, value, _) = h
            .call("POST", "/api/brains", Some(&owner), json!({"name":name}))
            .await;
        assert_eq!(status, StatusCode::OK);
        brains.push(value["id"].as_str().unwrap().parse::<Uuid>().unwrap());
    }
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brains[0])
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let mut first = checked(db::actor_tx(&h.state.pool, actor).await);
    checked(db::lock_brain(&mut first, brains[0], false).await);
    // Existing readers can overlap before any writer is waiting.
    let mut second = checked(db::actor_tx(&h.state.pool, actor).await);
    checked(
        tokio::time::timeout(
            Duration::from_secs(1),
            db::lock_brain(&mut second, brains[0], false),
        )
        .await
        .unwrap(),
    );
    second.commit().await.unwrap();

    let brain = brains[0];
    let pool = h.state.pool.clone();
    let (pid_tx, pid_rx) = tokio::sync::oneshot::channel();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let write = tokio::spawn(async move {
        let mut writer = checked(db::actor_tx(&pool, actor).await);
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        pid_tx.send(pid).unwrap();
        checked(db::lock_brain(&mut writer, brain, true).await);
        ready_tx.send(()).unwrap();
        release_rx.await.unwrap();
        writer.rollback().await.unwrap();
    });
    let writer_pid = pid_rx.await.unwrap();
    waiting(&h, writer_pid).await;

    let pool = h.state.pool.clone();
    let (pid_tx, pid_rx) = tokio::sync::oneshot::channel();
    let mut read = tokio::spawn(async move {
        let mut late = checked(db::actor_tx(&pool, actor).await);
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *late)
            .await
            .unwrap();
        pid_tx.send(pid).unwrap();
        checked(db::lock_brain(&mut late, brain, false).await);
        late.commit().await.unwrap();
    });
    let late_pid = pid_rx.await.unwrap();
    tokio::select! {
        _ = waiting(&h, late_pid) => {},
        result = &mut read => {
            result.unwrap();
            first.rollback().await.unwrap();
            release_tx.send(()).unwrap();
            write.await.unwrap();
            h.finish().await;
            panic!("a new reader bypassed an already waiting Brain writer");
        }
    }
    // Admission is per Brain, including while another Brain has both waiters.
    let mut other = checked(db::actor_tx(&h.state.pool, actor).await);
    checked(
        tokio::time::timeout(
            Duration::from_secs(1),
            db::lock_brain(&mut other, brains[1], true),
        )
        .await
        .unwrap(),
    );
    other.commit().await.unwrap();
    first.commit().await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), ready_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !read.is_finished(),
        "the new reader must wait for the writer"
    );
    // Rollback releases admission just as commit does; no lock survives in a pool.
    release_tx.send(()).unwrap();
    write.await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), read)
        .await
        .unwrap()
        .unwrap();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn brain_preparation_snapshot_does_not_block_writes_and_cannot_publish_mutations() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (_, value, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Snapshot concurrency fixture"}),
        )
        .await;
    let brain: Uuid = value["id"].as_str().unwrap().parse().unwrap();
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let mut snapshot = checked(db::preparation_tx(&h.state.pool, actor, None).await);
    checked(db::require_role(&mut snapshot, brain, false).await);
    let epoch: i64 = sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut *snapshot)
        .await
        .unwrap();
    let start = std::time::Instant::now();
    tokio::time::timeout(Duration::from_secs(1), async {
        let mut writer = checked(db::actor_tx(&h.state.pool, actor).await);
        checked(db::require_writer(&mut writer, brain).await);
        sqlx::query("INSERT INTO evidence_groups(id,brain_id,kind,name,created_by) VALUES($1,$2,'area','Concurrent fixture', $3)")
            .bind(Uuid::new_v4()).bind(brain).bind(actor).execute(&mut *writer).await.unwrap();
        writer.commit().await.unwrap();
    }).await.expect("provisional preparation must not hold Brain/account write locks");
    let unchanged: i64 = sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut *snapshot)
        .await
        .unwrap();
    assert_eq!(
        epoch, unchanged,
        "all preparation reads must use one consistent snapshot"
    );
    snapshot.commit().await.unwrap();
    let mut current = checked(db::actor_tx(&h.state.pool, actor).await);
    checked(db::require_role(&mut current, brain, false).await);
    let changed: i64 = sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut *current)
        .await
        .unwrap();
    assert!(changed > epoch);
    let flag: Option<String> =
        sqlx::query_scalar("SELECT nullif(current_setting('recollect.preparation',true),'')")
            .fetch_one(&mut *current)
            .await
            .unwrap();
    assert_ne!(
        flag.as_deref(),
        Some("on"),
        "local preparation state must not leak through the pool"
    );
    current.commit().await.unwrap();

    let mut snapshot = checked(db::preparation_tx(&h.state.pool, actor, None).await);
    let write = sqlx::query("UPDATE brains SET archived=true WHERE id=$1")
        .bind(brain)
        .execute(&mut *snapshot)
        .await
        .unwrap_err();
    assert_eq!(
        write.as_database_error().unwrap().code().as_deref(),
        Some("25006")
    );
    snapshot.rollback().await.unwrap();
    let mut snapshot = checked(db::preparation_tx(&h.state.pool, actor, None).await);
    assert!(db::lock_brain(&mut snapshot, brain, true).await.is_err());
    snapshot.rollback().await.unwrap();
    let mut write = checked(db::actor_tx(&h.state.pool, actor).await);
    sqlx::query("SET LOCAL recollect.preparation='on'")
        .execute(&mut *write)
        .await
        .unwrap();
    assert!(
        db::lock_brain(&mut write, brain, false).await.is_err(),
        "a flag in a mutable transaction cannot bypass admission"
    );
    write.rollback().await.unwrap();
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    eprintln!(
        "Snapshot/writer concurrency proof: writer committed in {} ms; model requests=0",
        start.elapsed().as_millis()
    );
    h.finish().await;
}
