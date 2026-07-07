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
