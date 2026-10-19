//! Lock, statement-count and race proof for the typed Brain admission
//! (slice `platform-brain-admission`). The golden status matrix lives in
//! `admission_matrix.rs`; this file proves what a status cannot: which Brain
//! lock an admission holds, how many statements it costs after authentication,
//! and that role and archive state are read after the lock wait.
use super::*;
use recollect_server::db::admission::{self, Archived, BrainAccess, Role, Transport};
use std::{
    collections::BTreeSet,
    fmt::Debug,
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing_subscriber::{Layer, layer::SubscriberExt};

const NAMESPACE: i64 = 73241029;

fn archived_probe() -> recollect_server::error::Error {
    recollect_server::error::Error(StatusCode::CONFLICT, "probe_archived", "probe")
}

struct Fixture {
    owner: Uuid,
    brain: Uuid,
    key: i32,
}

async fn fixture(h: &Harness) -> Fixture {
    let owner = h.login().await;
    let (status, brain, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Admission locks"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let (owner, key): (Uuid, i32) =
        sqlx::query_as("SELECT owner_id,admission_key FROM brains WHERE id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    Fixture { owner, brain, key }
}

/// What a transaction holds on the Brain, observed from another connection.
#[derive(Debug, PartialEq)]
struct Held {
    advisory: BTreeSet<String>,
    /// A competing shared advisory lock could be taken at once.
    shared_free: bool,
    exclusive_free: bool,
    /// A competing `FOR SHARE NOWAIT` / `FOR UPDATE NOWAIT` row lock succeeded.
    row_share_free: bool,
    row_update_free: bool,
}

async fn observe(h: &Harness, pid: i32, f: &Fixture) -> Held {
    let advisory: Vec<String> = sqlx::query_scalar(&format!(
        "SELECT mode FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND granted AND classid::bigint={NAMESPACE} AND objid::bigint=$2 ORDER BY mode"
    ))
    .bind(pid)
    .bind(f.key as i64)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    let advisory_free = |exclusive: bool| {
        let function = if exclusive {
            "pg_try_advisory_xact_lock"
        } else {
            "pg_try_advisory_xact_lock_shared"
        };
        let pool = h.admin.clone();
        let key = f.key;
        async move {
            let mut probe = pool.begin().await.unwrap();
            let free: bool = sqlx::query_scalar(&format!("SELECT {function}({NAMESPACE}::int,$1)"))
                .bind(key)
                .fetch_one(&mut *probe)
                .await
                .unwrap();
            probe.rollback().await.unwrap();
            free
        }
    };
    let row_free = |clause: &'static str| {
        let pool = h.admin.clone();
        let brain = f.brain;
        async move {
            let mut probe = pool.begin().await.unwrap();
            let result = sqlx::query(&format!(
                "SELECT id FROM brains WHERE id=$1 FOR {clause} NOWAIT"
            ))
            .bind(brain)
            .fetch_one(&mut *probe)
            .await;
            let _ = probe.rollback().await;
            result.is_ok()
        }
    };
    Held {
        advisory: advisory.into_iter().collect(),
        shared_free: advisory_free(false).await,
        exclusive_free: advisory_free(true).await,
        row_share_free: row_free("SHARE").await,
        row_update_free: row_free("UPDATE").await,
    }
}

fn strongest(modes: &BTreeSet<String>) -> &'static str {
    if modes.contains("ExclusiveLock") {
        "ExclusiveLock"
    } else if modes.contains("ShareLock") {
        "ShareLock"
    } else {
        "none"
    }
}

async fn pid_of(tx: &mut admission::Tx<'_>) -> i32 {
    sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn admission_holds_the_same_brain_lock_as_the_helpers_it_replaces() {
    let h = Harness::new().await;
    let f = fixture(&h).await;
    let owned_pool = h.state.pool.clone();
    let pool = &owned_pool;

    // Reads: the queued shared lock and a row share lock, nothing stronger.
    let mut old = db::actor_tx(pool, f.owner).await.unwrap();
    db::lock_brain(&mut old, f.brain, false).await.unwrap();
    db::require_role(&mut old, f.brain, false).await.unwrap();
    let old_read = observe(&h, pid_of(&mut old).await, &f).await;
    old.rollback().await.unwrap();
    let mut new = admission::brain_tx_as(pool, f.owner, None, f.brain, BrainAccess::READ)
        .await
        .unwrap();
    let new_read = observe(&h, pid_of(&mut new.tx).await, &f).await;
    new.tx.rollback().await.unwrap();
    eprintln!(
        "ADMISSION_LOCKS read old={:?} new={:?}",
        old_read.advisory, new_read.advisory
    );
    assert_eq!(strongest(&old_read.advisory), "ShareLock");
    assert_eq!(new_read.advisory, BTreeSet::from(["ShareLock".to_owned()]));
    assert!(new_read.shared_free && !new_read.exclusive_free);
    assert!(new_read.row_share_free && !new_read.row_update_free);
    assert_eq!(
        (
            new_read.shared_free,
            new_read.exclusive_free,
            new_read.row_share_free,
            new_read.row_update_free
        ),
        (
            old_read.shared_free,
            old_read.exclusive_free,
            old_read.row_share_free,
            old_read.row_update_free
        ),
        "A read admission excludes exactly what the old helpers excluded"
    );

    // Writes: `require_writer` took the exclusive lock, then re-took the shared
    // one through `require_role`; the single admission keeps the exclusive lock
    // and drops only the redundant shared hold.
    let mut old = db::actor_tx(pool, f.owner).await.unwrap();
    db::require_writer(&mut old, f.brain).await.unwrap();
    let old_write = observe(&h, pid_of(&mut old).await, &f).await;
    old.rollback().await.unwrap();
    let mut new = admission::brain_tx_as(pool, f.owner, None, f.brain, BrainAccess::WRITE)
        .await
        .unwrap();
    let new_write = observe(&h, pid_of(&mut new.tx).await, &f).await;
    new.tx.rollback().await.unwrap();
    eprintln!(
        "ADMISSION_LOCKS write old={:?} new={:?}",
        old_write.advisory, new_write.advisory
    );
    assert_eq!(strongest(&old_write.advisory), "ExclusiveLock");
    assert_eq!(
        new_write.advisory,
        BTreeSet::from(["ExclusiveLock".to_owned()])
    );
    assert!(!new_write.shared_free && !new_write.exclusive_free);
    assert!(!new_write.row_share_free && !new_write.row_update_free);
    assert_eq!(
        (
            new_write.shared_free,
            new_write.exclusive_free,
            new_write.row_share_free,
            new_write.row_update_free
        ),
        (
            old_write.shared_free,
            old_write.exclusive_free,
            old_write.row_share_free,
            old_write.row_update_free
        ),
        "A write admission excludes exactly what require_writer excluded"
    );
    assert!(old_write.advisory.is_superset(&new_write.advisory));

    // Administrator access (`require_role(.., true)`): exclusive, no extra hold.
    let mut old = db::actor_tx(pool, f.owner).await.unwrap();
    db::require_role(&mut old, f.brain, true).await.unwrap();
    let old_admin = observe(&h, pid_of(&mut old).await, &f).await;
    old.rollback().await.unwrap();
    let mut new = admission::brain_tx_as(
        pool,
        f.owner,
        None,
        f.brain,
        BrainAccess::admin(Archived::Allow),
    )
    .await
    .unwrap();
    assert_eq!(new.role, Role::Admin);
    let new_admin = observe(&h, pid_of(&mut new.tx).await, &f).await;
    new.tx.rollback().await.unwrap();
    assert_eq!(new_admin, old_admin);
    h.finish().await;
}

#[derive(Clone, Default)]
struct Statements(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> Layer<S> for Statements {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        // sqlx puts a short statement in `summary` and leaves `db.statement`
        // empty; a long or multi-line one fills `db.statement`.
        struct Text {
            summary: String,
            statement: String,
        }
        impl tracing::field::Visit for Text {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn Debug) {
                let value = format!("{value:?}");
                match field.name() {
                    "summary" => self.summary = value,
                    "db.statement" => self.statement = value,
                    _ => {}
                }
            }
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                match field.name() {
                    "summary" => self.summary = value.to_owned(),
                    "db.statement" => self.statement = value.to_owned(),
                    _ => {}
                }
            }
        }
        if event.metadata().target() == "sqlx::query" {
            let mut text = Text {
                summary: String::new(),
                statement: String::new(),
            };
            event.record(&mut text);
            let statement = if text.statement.trim().is_empty() {
                text.summary
            } else {
                text.statement
            };
            self.0.lock().unwrap().push(statement.trim().to_owned());
        }
    }
}

/// Statements sent while the future runs. The runtime is current-thread, so the
/// thread-local subscriber sees every statement the future issues.
async fn statements<T>(work: impl std::future::Future<Output = T>) -> (T, Vec<String>) {
    let seen = Statements::default();
    let guard = tracing::subscriber::set_default(tracing_subscriber::registry().with(seen.clone()));
    let value = work.await;
    drop(guard);
    // Transaction control is not an admission cost.
    let list = seen
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|s| !matches!(s.as_str(), "BEGIN" | "COMMIT" | "ROLLBACK"))
        .cloned()
        .collect();
    (value, list)
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn admission_costs_one_statement_after_authentication() {
    let h = Harness::new().await;
    let f = fixture(&h).await;
    let owned_pool = h.state.pool.clone();
    let pool = &owned_pool;
    // Warm one connection so statement preparation does not blur the count.
    sqlx::query("SELECT 1").execute(pool).await.unwrap();
    let (_, auth_only) = statements(async {
        let tx = db::actor_tx(pool, f.owner).await.unwrap();
        tx.rollback().await.unwrap();
    })
    .await;
    let mut report = vec![format!("authentication alone: {}", auth_only.len())];
    for (name, access) in [
        ("READ", BrainAccess::READ),
        ("WRITE", BrainAccess::WRITE),
        ("admin", BrainAccess::admin(Archived::Allow)),
        (
            "admin and archive check",
            BrainAccess::admin(Archived::Reject(archived_probe)),
        ),
    ] {
        let (_, new) = statements(async {
            let admitted = admission::brain_tx_as(pool, f.owner, None, f.brain, access)
                .await
                .unwrap();
            admitted.tx.rollback().await.unwrap();
        })
        .await;
        assert_eq!(
            new.len(),
            auth_only.len() + 1,
            "{name} admission must be one statement: {new:?}"
        );
        assert!(
            new.last().unwrap().contains("recollect_admit_brain"),
            "{new:?}"
        );
        report.push(format!("{name}: {}", new.len()));
    }
    // The statements the replaced helpers sent after authentication.
    let (_, old_write) = statements(async {
        let mut tx = db::actor_tx(pool, f.owner).await.unwrap();
        db::require_writer(&mut tx, f.brain).await.unwrap();
        tx.rollback().await.unwrap();
    })
    .await;
    let (_, old_read) = statements(async {
        let mut tx = db::actor_tx(pool, f.owner).await.unwrap();
        db::lock_brain(&mut tx, f.brain, false).await.unwrap();
        db::require_role(&mut tx, f.brain, false).await.unwrap();
        tx.rollback().await.unwrap();
    })
    .await;
    assert_eq!(old_write.len() - auth_only.len(), 4);
    assert_eq!(old_read.len() - auth_only.len(), 3);
    report.push(format!(
        "replaced require_writer: {}, replaced lock_brain+require_role: {}",
        old_write.len(),
        old_read.len()
    ));
    eprintln!("ADMISSION_STATEMENTS {}", report.join("; "));
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn admission_reads_role_and_archive_after_the_lock_wait_and_never_locks_for_an_invisible_brain()
 {
    let h = Harness::new().await;
    let f = fixture(&h).await;
    let (writer, _) = h.fixture_member().await;
    let (stranger, _) = h.fixture_member().await;
    let owner = h.login().await;
    let (status, _, _) = h
        .call(
            "PUT",
            &format!("/api/brains/{}/grants/{writer}", f.brain),
            Some(&owner),
            json!({"role":"writer"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let pool = h.state.pool.clone();

    // A Brain the caller cannot see is 404 without waiting for a held lock.
    let mut holder = db::actor_tx(&pool, f.owner).await.unwrap();
    db::lock_brain(&mut holder, f.brain, true).await.unwrap();
    let hidden = tokio::time::timeout(
        Duration::from_secs(2),
        admission::brain_tx_as(&pool, stranger, None, f.brain, BrainAccess::WRITE),
    )
    .await
    .expect("an invisible Brain must not queue for its lock");
    assert_eq!(hidden.err().unwrap().0, StatusCode::NOT_FOUND);
    holder.rollback().await.unwrap();

    // The role and the archive flag are those committed while this admission
    // waited: an administrator archives the Brain and revokes the grant behind
    // two queued writers, who must see neither the old role nor an open Brain.
    for revoke in [false, true] {
        let mut holder = db::actor_tx(&pool, f.owner).await.unwrap();
        db::lock_brain(&mut holder, f.brain, true).await.unwrap();
        let waiting_pool = pool.clone();
        let brain = f.brain;
        let queued = tokio::spawn(async move {
            admission::brain_tx_as(
                &waiting_pool,
                writer,
                None,
                brain,
                BrainAccess::WRITE.with_archived(Archived::Reject(archived_probe)),
            )
            .await
            .map(|admitted| admitted.role)
        });
        // Wait for any backend to queue behind the holder.
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let waiting: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM pg_locks WHERE locktype='advisory' AND NOT granted AND classid::bigint=$1 AND objid::bigint=$2",
                )
                .bind(NAMESPACE)
                .bind(f.key as i64)
                .fetch_one(&h.admin)
                .await
                .unwrap();
                if waiting > 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the writer must queue behind the holder");
        if revoke {
            sqlx::query("DELETE FROM brain_grants WHERE brain_id=$1 AND account_id=$2")
                .bind(f.brain)
                .bind(writer)
                .execute(&mut *holder)
                .await
                .unwrap();
        } else {
            sqlx::query("UPDATE brains SET archived=true WHERE id=$1")
                .bind(f.brain)
                .execute(&mut *holder)
                .await
                .unwrap();
        }
        holder.commit().await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), queued)
            .await
            .unwrap()
            .unwrap();
        let (status, code) = result
            .err()
            .map(|e| (e.0, e.1))
            .expect("the committed change must refuse the queued admission");
        if revoke {
            assert_eq!((status, code), (StatusCode::NOT_FOUND, "not_found"));
        } else {
            assert_eq!((status, code), (StatusCode::CONFLICT, "probe_archived"));
        }
    }
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn admission_variants_for_workers_and_preparation_snapshots() {
    let h = Harness::new().await;
    let f = fixture(&h).await;
    let (reader, _) = h.fixture_member().await;
    let owner = h.login().await;
    let (status, _, _) = h
        .call(
            "PUT",
            &format!("/api/brains/{}/grants/{reader}", f.brain),
            Some(&owner),
            json!({"role":"reader"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let owned_pool = h.state.pool.clone();
    let pool = &owned_pool;

    // Worker variant: stored actor, no device, role reported.
    let admitted = admission::brain_tx_as(pool, reader, None, f.brain, BrainAccess::READ)
        .await
        .unwrap();
    assert_eq!((admitted.role, admitted.archived), (Role::Reader, false));
    admitted.tx.rollback().await.unwrap();
    let denied = admission::brain_tx_as(pool, reader, None, f.brain, BrainAccess::WRITE).await;
    assert_eq!(denied.err().unwrap().1, "forbidden");
    // A worker cannot satisfy a browser-only or device-only surface it lacks.
    let device = BrainAccess::READ.with_transport(Transport::Device);
    assert_eq!(
        admission::brain_tx_as(pool, reader, None, f.brain, device)
            .await
            .err()
            .unwrap()
            .1,
        admission::device_required().1
    );

    // Preparation variant: read-only REPEATABLE READ, never a lock, role checked.
    let mut prepared =
        admission::brain_preparation_tx(pool, reader, None, f.brain, BrainAccess::READ)
            .await
            .unwrap();
    assert_eq!(prepared.role, Role::Reader);
    let pid = pid_of(&mut prepared.tx).await;
    assert!(
        observe(&h, pid, &f).await.advisory.is_empty(),
        "A preparation snapshot never takes the Brain lock"
    );
    let write = sqlx::query("UPDATE brains SET description=description WHERE id=$1")
        .bind(f.brain)
        .execute(&mut *prepared.tx)
        .await;
    assert!(write.is_err(), "A preparation snapshot is read-only");
    drop(prepared);
    // The write floor still applies to a preparation snapshot, and the lock
    // mode requested by a write surface is ignored rather than rejected.
    let denied =
        admission::brain_preparation_tx(pool, reader, None, f.brain, BrainAccess::WRITE).await;
    assert_eq!(denied.err().unwrap().1, "forbidden");
    let owner_read = admission::brain_preparation_tx(
        pool,
        f.owner,
        None,
        f.brain,
        BrainAccess::admin(Archived::Allow),
    )
    .await
    .unwrap();
    assert_eq!(owner_read.role, Role::Admin);
    drop(owner_read);
    let missing =
        admission::brain_preparation_tx(pool, reader, None, Uuid::new_v4(), BrainAccess::READ)
            .await;
    assert_eq!(missing.err().unwrap().0, StatusCode::NOT_FOUND);
    h.finish().await;
}
