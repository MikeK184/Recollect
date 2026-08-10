//! Durable sanitized completion receipts, never execution requests. Retrying this
//! store may upload a receipt, but cannot initiate another provider call.
use crate::{Result, RuntimeError};
use recollect_protocol::McpCompletion;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;

const MAX_PENDING: i64 = 64;
const MAX_RECEIPT: usize = 300 * 1024;
const APPLICATION_ID: i64 = 0x524d4350;
static BLOCKING: OnceLock<Arc<Semaphore>> = OnceLock::new();

pub struct Outbox {
    path: PathBuf,
    owner: String,
}
fn unavailable(_: impl std::fmt::Debug) -> RuntimeError {
    RuntimeError("receipt_storage_unavailable")
}
fn private(path: &Path, directory: bool) -> Result<()> {
    if !path.exists() {
        if directory {
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(path).map_err(unavailable)?;
        } else {
            let mut options = fs::OpenOptions::new();
            options.create_new(true).write(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(path) {
                Ok(file) => file.sync_all().map_err(unavailable)?,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(unavailable(error)),
            }
        }
    }
    let metadata = fs::symlink_metadata(path).map_err(unavailable)?;
    if metadata.file_type().is_symlink()
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
    {
        return Err(RuntimeError("receipt_storage_invalid"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(RuntimeError("receipt_storage_not_private"));
        }
    }
    Ok(())
}
fn seconds(now: SystemTime) -> Result<i64> {
    now.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .ok_or(RuntimeError("receipt_clock_invalid"))
}
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    let permit = BLOCKING
        .get_or_init(|| Arc::new(Semaphore::new(4)))
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| RuntimeError("receipt_storage_unavailable"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await
    .map_err(unavailable)?
}
fn connect(path: &Path, owner: &str) -> Result<Connection> {
    private(path, false)?;
    let mut db = Connection::open(path).map_err(unavailable)?;
    db.busy_timeout(Duration::from_millis(500))
        .map_err(unavailable)?;
    let application: i64 = db
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .map_err(unavailable)?;
    let tables: i64 = db
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table'",
            [],
            |row| row.get(0),
        )
        .map_err(unavailable)?;
    if application != APPLICATION_ID && !(application == 0 && tables == 0) {
        return Err(RuntimeError("receipt_owner_mismatch"));
    }
    if application == APPLICATION_ID {
        let recorded: String = db
            .query_row("SELECT owner FROM owner WHERE id=1", [], |r| r.get(0))
            .map_err(unavailable)?;
        if recorded != owner {
            return Err(RuntimeError("receipt_owner_mismatch"));
        }
    }
    db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA secure_delete=ON; PRAGMA temp_store=MEMORY;").map_err(unavailable)?;
    if application == APPLICATION_ID {
        return Ok(db);
    }
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(unavailable)?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS owner(id INTEGER PRIMARY KEY CHECK(id=1),owner TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS receipts(id TEXT PRIMARY KEY,body TEXT NOT NULL CHECK(length(body)<=307200),expires_at INTEGER NOT NULL,rejected INTEGER NOT NULL DEFAULT 0);").map_err(unavailable)?;
    tx.execute(
        "INSERT OR IGNORE INTO owner(id,owner) VALUES(1,?1)",
        [owner],
    )
    .map_err(unavailable)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)
        .map_err(unavailable)?;
    let recorded: String = tx
        .query_row("SELECT owner FROM owner WHERE id=1", [], |r| r.get(0))
        .map_err(unavailable)?;
    if recorded != owner {
        return Err(RuntimeError("receipt_owner_mismatch"));
    }
    tx.commit().map_err(unavailable)?;
    Ok(db)
}
impl Outbox {
    /// Operator-only relocation of an already owned receipt store. This changes
    /// its endpoint identity, never receipt bodies or execution requests. The
    /// caller must hold the installation's recovery barrier; ordinary open still
    /// rejects a foreign owner. Repeating a completed relocation is harmless.
    pub async fn relocate(
        directory: PathBuf,
        old_owner: String,
        new_owner: String,
    ) -> Result<Self> {
        for owner in [&old_owner, &new_owner] {
            if owner.is_empty() || owner.len() > 2048 || owner.chars().any(char::is_control) {
                return Err(RuntimeError("receipt_owner_invalid"));
            }
        }
        let path = directory.join("receipts.sqlite");
        let opened = path.clone();
        let identity = new_owner.clone();
        blocking(move || {
            private(&directory, true)?;
            if !opened.exists() {
                connect(&opened, &identity)?;
                return Ok(());
            }
            let mut db = match connect(&opened, &old_owner) {
                Ok(db) => db,
                Err(RuntimeError("receipt_owner_mismatch")) => {
                    connect(&opened, &identity)?;
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(unavailable)?;
            tx.execute(
                "UPDATE owner SET owner=?1 WHERE id=1 AND owner=?2",
                params![identity, old_owner],
            )
            .map_err(unavailable)?;
            tx.commit().map_err(unavailable)
        })
        .await?;
        Ok(Self {
            path,
            owner: new_owner,
        })
    }

    pub async fn open(directory: PathBuf, owner: String) -> Result<Self> {
        if owner.is_empty() || owner.len() > 2048 || owner.chars().any(char::is_control) {
            return Err(RuntimeError("receipt_owner_invalid"));
        }
        let path = directory.join("receipts.sqlite");
        let opened = path.clone();
        let identity = owner.clone();
        blocking(move || {
            private(&directory, true)?;
            connect(&opened, &identity)?;
            Ok(())
        })
        .await?;
        Ok(Self { path, owner })
    }
    pub async fn store(&self, completion: &McpCompletion, now: SystemTime) -> Result<()> {
        let body = serde_json::to_string(completion).map_err(unavailable)?;
        if body.len() > MAX_RECEIPT {
            return Err(RuntimeError("receipt_too_large"));
        }
        let id = completion.attempt.call_id.to_string();
        let path = self.path.clone();
        let owner = self.owner.clone();
        let now = seconds(now)?;
        blocking(move || {
            let mut db = connect(&path, &owner)?;
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(unavailable)?;
            tx.execute("DELETE FROM receipts WHERE expires_at<=?1", [now])
                .map_err(unavailable)?;
            let old: Option<String> = tx
                .query_row("SELECT body FROM receipts WHERE id=?1", [&id], |r| r.get(0))
                .optional()
                .map_err(unavailable)?;
            if let Some(old) = old {
                if old != body {
                    return Err(RuntimeError("receipt_conflict"));
                }
            } else {
                let pending: i64 = tx
                    .query_row("SELECT count(*) FROM receipts", [], |r| r.get(0))
                    .map_err(unavailable)?;
                if pending >= MAX_PENDING {
                    return Err(RuntimeError("receipt_capacity"));
                }
                tx.execute(
                    "INSERT INTO receipts(id,body,expires_at) VALUES(?1,?2,?3)",
                    params![id, body, now + 3600],
                )
                .map_err(unavailable)?;
            }
            tx.commit().map_err(unavailable)
        })
        .await
    }
    pub async fn pending(&self, now: SystemTime) -> Result<Vec<McpCompletion>> {
        let path = self.path.clone();
        let owner = self.owner.clone();
        let now = seconds(now)?;
        blocking(move || {
            let mut db = connect(&path, &owner)?;
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(unavailable)?;
            tx.execute("DELETE FROM receipts WHERE expires_at<=?1", [now])
                .map_err(unavailable)?;
            let mut result = Vec::new();
            {
                let mut statement = tx
                    .prepare("SELECT body FROM receipts WHERE rejected=0 ORDER BY expires_at,id LIMIT 65")
                    .map_err(unavailable)?;
                let rows = statement
                    .query_map([], |r| r.get::<_, String>(0))
                    .map_err(unavailable)?;
                for row in rows {
                    let body = row.map_err(unavailable)?;
                    if body.len() > MAX_RECEIPT || result.len() >= MAX_PENDING as usize {
                        return Err(RuntimeError("receipt_storage_invalid"));
                    }
                    result.push(
                        serde_json::from_str(&body)
                            .map_err(|_| RuntimeError("receipt_storage_invalid"))?,
                    );
                }
            }
            tx.commit().map_err(unavailable)?;
            Ok(result)
        })
        .await
    }
    /// Include quarantined receipts: deletion applies to their bodies too.
    pub async fn identities(&self, now: SystemTime) -> Result<Vec<uuid::Uuid>> {
        let path = self.path.clone();
        let owner = self.owner.clone();
        let now = seconds(now)?;
        blocking(move || {
            let mut db = connect(&path, &owner)?;
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(unavailable)?;
            tx.execute("DELETE FROM receipts WHERE expires_at<=?1", [now])
                .map_err(unavailable)?;
            let ids = {
                let mut query = tx
                    .prepare("SELECT id FROM receipts ORDER BY id LIMIT 65")
                    .map_err(unavailable)?;
                let ids = query
                    .query_map([], |r| r.get::<_, String>(0))
                    .map_err(unavailable)?
                    .map(|v| {
                        v.map_err(unavailable)
                            .and_then(|v| uuid::Uuid::parse_str(&v).map_err(unavailable))
                    })
                    .collect::<Result<Vec<_>>>()?;
                if ids.len() > MAX_PENDING as usize {
                    return Err(RuntimeError("receipt_storage_invalid"));
                }
                ids
            };
            tx.commit().map_err(unavailable)?;
            Ok(ids)
        })
        .await
    }
    pub async fn remove(&self, ids: Vec<uuid::Uuid>) -> Result<()> {
        if ids.len() > MAX_PENDING as usize {
            return Err(RuntimeError("receipt_storage_invalid"));
        }
        let path = self.path.clone();
        let owner = self.owner.clone();
        blocking(move || {
            let mut db = connect(&path, &owner)?;
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(unavailable)?;
            for id in ids {
                tx.execute("DELETE FROM receipts WHERE id=?1", [id.to_string()])
                    .map_err(unavailable)?;
            }
            tx.commit().map_err(unavailable)
        })
        .await
    }
    pub async fn acknowledge(&self, completion: &McpCompletion) -> Result<()> {
        let body = serde_json::to_string(completion).map_err(unavailable)?;
        let id = completion.attempt.call_id.to_string();
        let path = self.path.clone();
        let owner = self.owner.clone();
        blocking(move || {
            connect(&path, &owner)?
                .execute(
                    "DELETE FROM receipts WHERE id=?1 AND body=?2",
                    params![id, body],
                )
                .map_err(unavailable)?;
            Ok(())
        })
        .await
    }
    pub async fn quarantine(&self, completion: &McpCompletion) -> Result<()> {
        let body = serde_json::to_string(completion).map_err(unavailable)?;
        let id = completion.attempt.call_id.to_string();
        let path = self.path.clone();
        let owner = self.owner.clone();
        blocking(move || {
            connect(&path, &owner)?
                .execute(
                    "UPDATE receipts SET rejected=1 WHERE id=?1 AND body=?2",
                    params![id, body],
                )
                .map_err(unavailable)?;
            Ok(())
        })
        .await
    }
    pub async fn count(&self, now: SystemTime) -> Result<usize> {
        let path = self.path.clone();
        let owner = self.owner.clone();
        let now = seconds(now)?;
        blocking(move || {
            let mut db = connect(&path, &owner)?;
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(unavailable)?;
            tx.execute("DELETE FROM receipts WHERE expires_at<=?1", [now])
                .map_err(unavailable)?;
            let count: i64 = tx
                .query_row("SELECT count(*) FROM receipts", [], |r| r.get(0))
                .map_err(unavailable)?;
            tx.commit().map_err(unavailable)?;
            Ok(count as usize)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use recollect_protocol::McpAttempt;
    use uuid::Uuid;
    #[tokio::test]
    async fn receipt_restart_identity_expiry_conflict_and_atomic_capacity() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache/mcp-outbox-proof")
            .join(Uuid::new_v4().to_string());
        let now = SystemTime::now();
        let owner = Uuid::new_v4().to_string();
        let outbox = Outbox::open(path.clone(), owner.clone()).await.unwrap();
        let receipt = McpCompletion {
            attempt: McpAttempt {
                epoch: Uuid::new_v4(),
                brain_id: Uuid::new_v4(),
                call_id: Uuid::new_v4(),
                attempt_token: Uuid::new_v4(),
            },
            state: "succeeded".into(),
            code: "connector_response".into(),
            result: Some(
                serde_json::json!({"content":[],"structuredContent":{"receipt":"sanitized fixture"}}),
            ),
        };
        outbox.store(&receipt, now).await.unwrap();
        drop(outbox);
        assert!(matches!(
            Outbox::open(path.clone(), "different identity".into()).await,
            Err(RuntimeError("receipt_owner_mismatch"))
        ));
        let outbox = Arc::new(Outbox::open(path.clone(), owner).await.unwrap());
        assert_eq!(outbox.pending(now).await.unwrap().len(), 1);
        outbox
            .store(&receipt, now + Duration::from_secs(3599))
            .await
            .unwrap();
        let mut changed = receipt.clone();
        changed.code = "changed_receipt".into();
        assert_eq!(
            outbox.store(&changed, now).await.unwrap_err(),
            RuntimeError("receipt_conflict")
        );
        outbox.acknowledge(&changed).await.unwrap();
        assert_eq!(outbox.pending(now).await.unwrap().len(), 1);
        for _ in 1..63 {
            let mut next = receipt.clone();
            next.attempt.call_id = Uuid::new_v4();
            outbox.store(&next, now).await.unwrap();
        }
        let mut a = receipt.clone();
        a.attempt.call_id = Uuid::new_v4();
        let mut b = receipt.clone();
        b.attempt.call_id = Uuid::new_v4();
        let (a, b) = tokio::join!(outbox.store(&a, now), outbox.store(&b, now));
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        assert_eq!(outbox.pending(now).await.unwrap().len(), 64);
        outbox.quarantine(&receipt).await.unwrap();
        assert_eq!(outbox.pending(now).await.unwrap().len(), 63);
        assert_eq!(
            outbox.count(now).await.unwrap(),
            64,
            "fenced receipts still consume their retention capacity"
        );
        assert!(
            outbox
                .pending(now + Duration::from_secs(3600))
                .await
                .unwrap()
                .is_empty(),
            "replaying a receipt must not extend retention"
        );
        drop(outbox);
        fs::remove_dir_all(path).unwrap();
    }
}
