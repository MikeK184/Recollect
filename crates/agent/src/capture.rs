//! Private, sanitized local capture inbox. These methods never call a provider
//! or server; a separate uploader owns current remote authorization and receipts.
use anyhow::{Result, anyhow, ensure};
use chrono::{DateTime, Duration, Utc};
use recollect_protocol::*;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration as Wait,
};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct CachedCaptureBinding {
    pub binding: CaptureBinding,
    pub policy: CapturePolicy,
    pub retention: RetentionPolicy,
    pub synchronized_at: DateTime<Utc>,
    pub agent_id: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct InboxStatus {
    pub pending: i64,
    pub pending_bytes: i64,
    pub delivered: i64,
    pub denied: i64,
    pub removed: i64,
    pub gaps: Vec<(String, i64)>,
}
pub struct Inbox {
    db: Connection,
    device: Uuid,
    max_pending: i64,
    max_bytes: i64,
}
fn private(path: &Path, directory: bool) -> Result<()> {
    if path.exists() {
        let metadata = fs::symlink_metadata(path)?;
        ensure!(
            !metadata.file_type().is_symlink() && metadata.is_dir() == directory,
            "Capture storage must be an owned regular file or directory."
        );
    } else if directory {
        fs::create_dir_all(path)?;
    } else {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path)?.sync_all()?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
        )?;
    }
    Ok(())
}
fn binding(tx: &Transaction<'_>, id: Uuid) -> Result<CachedCaptureBinding> {
    let body: String = tx
        .query_row(
            "SELECT body FROM bindings WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("capture_binding_missing"))?;
    serde_json::from_str(&body).map_err(|_| anyhow!("capture_binding_unreadable"))
}
fn gap(tx: &Transaction<'_>, code: &str) -> Result<()> {
    tx.execute(
        "INSERT INTO gaps(code,total) VALUES(?1,1) ON CONFLICT(code) DO UPDATE SET total=total+1",
        [code],
    )?;
    Ok(())
}
fn clear(tx: &Transaction<'_>, id: &str, state: &str) -> Result<()> {
    tx.execute(
        "UPDATE events SET body=NULL,bytes=0,state=?2,error_code=NULL WHERE id=?1",
        params![id, state],
    )?;
    Ok(())
}
pub fn profile_root(evidence_root: &Path, device: Uuid) -> PathBuf {
    evidence_root.join(".capture").join(device.to_string())
}
impl Inbox {
    pub fn open(root: &Path, endpoint: &str, device: Uuid) -> Result<Self> {
        let path = root.join("inbox.sqlite");
        if let Ok(meta) = fs::symlink_metadata(&path) {
            ensure!(
                meta.is_file() && !meta.file_type().is_symlink(),
                "invalid_capture_storage"
            );
            if meta.len() > 0 {
                let existing =
                    Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
                let matches: bool = existing
                    .query_row(
                        "SELECT endpoint=?1 AND device=?2 FROM profile WHERE id=1",
                        params![endpoint, device.to_string()],
                        |r| r.get(0),
                    )
                    .map_err(|_| anyhow!("capture_profile_mismatch"))?;
                ensure!(matches, "capture_profile_mismatch");
            }
        }
        private(root, true)?;
        // Another hook may create the file between the check and create_new.
        if let Err(error) = private(&path, false) {
            if !path.is_file() {
                return Err(error);
            }
            private(&path, false)?;
        }
        let db = Connection::open(&path)?;
        db.busy_timeout(Wait::from_millis(250))?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA secure_delete=ON; PRAGMA temp_store=MEMORY;
            CREATE TABLE IF NOT EXISTS profile(id INTEGER PRIMARY KEY CHECK(id=1),endpoint TEXT NOT NULL,device TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS bindings(id TEXT PRIMARY KEY,brain TEXT NOT NULL,body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS launch_defaults(id TEXT PRIMARY KEY,original TEXT NOT NULL REFERENCES bindings(id),current TEXT NOT NULL REFERENCES bindings(id));
            CREATE TABLE IF NOT EXISTS turns(host TEXT NOT NULL,session TEXT NOT NULL,turn TEXT NOT NULL,agent TEXT NOT NULL,binding TEXT NOT NULL REFERENCES bindings(id),PRIMARY KEY(host,session,turn,agent));
            CREATE TABLE IF NOT EXISTS tools(host TEXT NOT NULL,session TEXT NOT NULL,tool TEXT NOT NULL,agent TEXT NOT NULL,binding TEXT NOT NULL REFERENCES bindings(id),PRIMARY KEY(host,session,tool,agent));
            CREATE TABLE IF NOT EXISTS events(id TEXT PRIMARY KEY,binding TEXT NOT NULL REFERENCES bindings(id),native_key TEXT,body TEXT,receipt TEXT,state TEXT NOT NULL,bytes INTEGER NOT NULL,created_at INTEGER NOT NULL,expires_at INTEGER NOT NULL,attempts INTEGER NOT NULL DEFAULT 0,retry_at INTEGER NOT NULL DEFAULT 0,error_code TEXT,UNIQUE(binding,native_key));
            CREATE TABLE IF NOT EXISTS fences(brain TEXT NOT NULL,event TEXT NOT NULL,binding TEXT NOT NULL,native_key TEXT,PRIMARY KEY(brain,event),UNIQUE(brain,binding,native_key));
            CREATE TABLE IF NOT EXISTS gaps(code TEXT PRIMARY KEY,total INTEGER NOT NULL);")?;
        db.execute(
            "INSERT OR IGNORE INTO profile(id,endpoint,device) VALUES(1,?1,?2)",
            params![endpoint, device.to_string()],
        )?;
        let matches: bool = db.query_row(
            "SELECT endpoint=?1 AND device=?2 FROM profile WHERE id=1",
            params![endpoint, device.to_string()],
            |r| r.get(0),
        )?;
        ensure!(matches, "capture_profile_mismatch");
        Ok(Self {
            db,
            device,
            max_pending: 5000,
            max_bytes: 64 * 1024 * 1024,
        })
    }
    pub fn remember(&mut self, saved: &CachedCaptureBinding) -> Result<()> {
        validate_capture_policy(&saved.policy).map_err(|s| anyhow!(s))?;
        let b = &saved.binding;
        ensure!(
            b.device_id == self.device
                && b.operation.device_id == Some(self.device)
                && b.brain_id == b.operation.brain_id
                && b.operation.kind == "capture"
                && b.operation.scope_valid
                && b.operation.scope.brain_id == b.brain_id,
            "invalid_capture_binding"
        );
        ensure!(
            saved.agent_id.as_deref().is_none_or(capture_identity),
            "invalid_capture_identity"
        );
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = tx
            .query_row(
                "SELECT body FROM bindings WHERE id=?1",
                [b.id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            let old: CachedCaptureBinding = serde_json::from_str(&existing)?;
            ensure!(
                serde_json::to_value(&old.binding)? == serde_json::to_value(b)?
                    && old.agent_id == saved.agent_id,
                "capture_binding_changed"
            );
            ensure!(
                saved.synchronized_at >= old.synchronized_at,
                "capture_policy_moved_backwards"
            );
        }
        tx.execute("INSERT INTO bindings(id,brain,body) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET body=excluded.body",
            params![b.id.to_string(),b.brain_id.to_string(),serde_json::to_string(saved)?])?;
        tx.execute("UPDATE events SET body=NULL,bytes=0,state='expired',error_code=NULL WHERE binding=?1 AND body IS NOT NULL AND expires_at<=?2",params![b.id.to_string(),saved.synchronized_at.timestamp()])?;
        tx.execute("UPDATE events SET expires_at=created_at+86400*CASE WHEN json_extract(body,'$.event.kind')='tool_result' THEN ?2 ELSE ?3 END WHERE binding=?1 AND body IS NOT NULL",
            params![b.id.to_string(),saved.retention.tool_output_days,saved.retention.raw_session_days])?;
        tx.commit()?;
        Ok(())
    }
    pub fn record_gap(&mut self, code: &str) -> Result<()> {
        let tx = self.db.transaction()?;
        // Callers may pass error codes, never arbitrary host/error text.
        let code = if code.len() <= 80 && code.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            code
        } else {
            "capture_failed"
        };
        gap(&tx, code)?;
        tx.commit()?;
        Ok(())
    }
    pub fn capture(
        &mut self,
        default_binding: Uuid,
        raw: &[u8],
        secrets: &[String],
        now: DateTime<Utc>,
    ) -> Result<Uuid> {
        self.capture_in_launch(default_binding, None, raw, secrets, now)
    }
    pub fn start_launch(&mut self, launch: Uuid, original: Uuid) -> Result<()> {
        ensure!(!launch.is_nil(), "invalid_capture_launch");
        self.db.execute(
            "INSERT INTO launch_defaults(id,original,current) VALUES(?1,?2,?2)",
            params![launch.to_string(), original.to_string()],
        )?;
        Ok(())
    }
    pub fn launch_default(&self, launch: Uuid, original: Uuid) -> Result<Uuid> {
        let current: String = self
            .db
            .query_row(
                "SELECT current FROM launch_defaults WHERE id=?1 AND original=?2",
                params![launch.to_string(), original.to_string()],
                |r| r.get(0),
            )
            .map_err(|_| anyhow!("capture_launch_missing"))?;
        Ok(current.parse()?)
    }
    /// Move one launch's future default only. Immutable setup, turn and tool rows
    /// are never rewritten, and competing publishers must inspect a lost CAS.
    pub fn advance_launch(
        &mut self,
        launch: Uuid,
        original: Uuid,
        expected: Uuid,
        next: Uuid,
    ) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let before = binding(&tx, original)?;
        let after = binding(&tx, next)?;
        ensure!(
            before.binding.brain_id == after.binding.brain_id
                && before.binding.device_id == after.binding.device_id
                && before.binding.operation.task_id == after.binding.operation.task_id
                && before.binding.operation.actor_id == after.binding.operation.actor_id
                && before.binding.host == after.binding.host
                && before.binding.host_version == after.binding.host_version
                && before.agent_id == after.agent_id,
            "capture_launch_scope_mismatch"
        );
        ensure!(
            tx.execute(
                "UPDATE launch_defaults SET current=?4 WHERE id=?1 AND original=?2 AND current=?3",
                params![
                    launch.to_string(),
                    original.to_string(),
                    expected.to_string(),
                    next.to_string()
                ]
            )? == 1,
            "capture_launch_changed"
        );
        tx.commit()?;
        Ok(())
    }
    pub fn capture_in_launch(
        &mut self,
        original: Uuid,
        launch: Option<Uuid>,
        raw: &[u8],
        secrets: &[String],
        now: DateTime<Utc>,
    ) -> Result<Uuid> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let default_binding = match launch {
            Some(launch) => tx
                .query_row(
                    "SELECT current FROM launch_defaults WHERE id=?1 AND original=?2",
                    params![launch.to_string(), original.to_string()],
                    |r| r.get::<_, String>(0),
                )
                .map_err(|_| anyhow!("capture_launch_missing"))?
                .parse()?,
            None => original,
        };
        let cached = binding(&tx, default_binding)?;
        if now - cached.synchronized_at > Duration::hours(24) {
            gap(&tx, "capture_policy_stale")?;
            tx.commit()?;
            return Err(anyhow!("capture_policy_stale"));
        }
        let mut event =
            match normalize_capture_hook(&cached.binding.host, raw, &cached.policy, secrets, now) {
                Ok(event) => event,
                Err(code) => {
                    gap(&tx, code)?;
                    tx.commit()?;
                    return Err(anyhow!(code));
                }
            };
        let host = &cached.binding.host;
        let agent = event.agent_id.as_deref().unwrap_or("");
        let matching_agent = cached.agent_id.as_deref().unwrap_or("") == agent;
        let session = &event.host_session_id;
        if event.host_event == "UserPromptSubmit"
            && matching_agent
            && let Some(turn) = &event.turn_id
        {
            tx.execute("INSERT OR IGNORE INTO turns(host,session,turn,agent,binding) VALUES(?1,?2,?3,?4,?5)",params![host,session,turn,agent,default_binding.to_string()])?;
        }
        let tool_binding = if event.host_event == "PostToolUse"
            || event.host_event == "PostToolUseFailure"
        {
            if let Some(tool) = &event.tool_use_id {
                tx.query_row("SELECT binding FROM tools WHERE host=?1 AND session=?2 AND tool=?3 AND agent=?4",params![host,session,tool,agent],|r|r.get::<_,String>(0)).optional()?
            } else {
                None
            }
        } else {
            None
        };
        let turn_binding = if let Some(turn) = &event.turn_id {
            tx.query_row(
                "SELECT binding FROM turns WHERE host=?1 AND session=?2 AND turn=?3 AND agent=?4",
                params![host, session, turn, agent],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        } else {
            None
        };
        if tool_binding.is_some() && turn_binding.is_some() && tool_binding != turn_binding {
            event.coverage.push("delayed_tool_completion".into());
        }
        let known = tool_binding.or(turn_binding);
        let routed = known
            .as_ref()
            .map(|s| s.parse::<Uuid>())
            .transpose()?
            .unwrap_or(default_binding);
        if event.kind != "lifecycle" && known.is_none() {
            event.content = None;
            if !event.coverage.iter().any(|s| s == "ambiguous_attribution") {
                event.coverage.push("ambiguous_attribution".into());
            }
        }
        let actual = binding(&tx, routed)?;
        // A delayed event uses its original scope but must still satisfy both
        // currently cached policies, including tighter limits/exclusions.
        if routed != default_binding {
            ensure!(
                actual.binding.host == *host && actual.binding.device_id == self.device,
                "capture_route_mismatch"
            );
            if now - actual.synchronized_at > Duration::hours(24) || !actual.policy.enabled {
                gap(&tx, "capture_policy_stale")?;
                tx.commit()?;
                return Err(anyhow!("capture_policy_stale"));
            }
            let scoped = normalize_capture_hook(host, raw, &actual.policy, secrets, now)
                .map_err(|s| anyhow!(s))?;
            if scoped.content.is_none() {
                event.content = None;
            }
            let max = actual.policy.max_event_bytes as usize;
            if let Some(content) = &mut event.content
                && content.len() > max
            {
                let mut n = max;
                while !content.is_char_boundary(n) {
                    n -= 1;
                }
                content.truncate(n);
                event.coverage.push("truncated".into());
            }
            for flag in scoped.coverage {
                if !event.coverage.contains(&flag) {
                    event.coverage.push(flag);
                }
            }
        }
        if event.host_event == "PreToolUse"
            && known.is_some()
            && let Some(tool) = &event.tool_use_id
        {
            tx.execute("INSERT OR IGNORE INTO tools(host,session,tool,agent,binding) VALUES(?1,?2,?3,?4,?5)",params![host,session,tool,agent,routed.to_string()])?;
        }
        let native_key = capture_native_key(host, &event);
        if native_key.is_none() {
            event.coverage.push("host_deduplication_unavailable".into());
        }
        let fenced: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM fences WHERE binding=?1 AND native_key=?2)",
            params![routed.to_string(), native_key],
            |r| r.get(0),
        )?;
        if fenced {
            gap(&tx, "capture_event_removed")?;
            tx.commit()?;
            return Err(anyhow!("capture_event_removed"));
        }
        if let Some(key) = &native_key
            && let Some((id, old)) = tx
                .query_row(
                    "SELECT id,body FROM events WHERE binding=?1 AND native_key=?2",
                    params![routed.to_string(), key],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
                )
                .optional()?
        {
            if let Some(body) = old {
                let mut old: CaptureEventInput = serde_json::from_str(&body)?;
                old.event.captured_at = event.captured_at;
                if old.event != event {
                    gap(&tx, "capture_identity_conflict")?;
                    tx.commit()?;
                    return Err(anyhow!("capture_identity_conflict"));
                }
            }
            tx.commit()?;
            return Ok(id.parse()?);
        }
        let (count, bytes): (i64, i64) = tx.query_row(
            "SELECT count(*),coalesce(sum(bytes),0) FROM events WHERE body IS NOT NULL",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let size = event.content.as_ref().map_or(0, |s| s.len()) as i64;
        if count >= self.max_pending || bytes + size > self.max_bytes {
            gap(&tx, "capture_inbox_full")?;
            tx.commit()?;
            return Err(anyhow!("capture_inbox_full"));
        }
        let days = if event.kind == "tool_result" {
            actual.retention.tool_output_days
        } else {
            actual.retention.raw_session_days
        };
        let id = Uuid::new_v4();
        let input = CaptureEventInput {
            id,
            binding_id: routed,
            event,
        };
        tx.execute("INSERT INTO events(id,binding,native_key,body,state,bytes,created_at,expires_at) VALUES(?1,?2,?3,?4,'pending',?5,?6,?7)",params![id.to_string(),routed.to_string(),native_key,serde_json::to_string(&input)?,size,now.timestamp(),(now+Duration::days(i64::from(days))).timestamp()])?;
        tx.commit()?;
        Ok(id)
    }
    pub fn pending(&mut self, now: DateTime<Utc>) -> Result<Vec<CaptureEventInput>> {
        self.expire(now)?;
        let mut statement=self.db.prepare("SELECT body FROM events WHERE state='pending' AND retry_at<=?1 ORDER BY created_at,id LIMIT 20")?;
        let rows = statement.query_map([now.timestamp()], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn cached_bindings(&self) -> Result<Vec<CachedCaptureBinding>> {
        let mut statement = self
            .db
            .prepare("SELECT body FROM bindings ORDER BY brain,id")?;
        let rows = statement.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn cached_binding(&self, id: Uuid) -> Result<CachedCaptureBinding> {
        let body: String = self
            .db
            .query_row(
                "SELECT body FROM bindings WHERE id=?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("capture_binding_missing"))?;
        serde_json::from_str(&body).map_err(|_| anyhow!("capture_binding_unreadable"))
    }
    pub fn retry_permissions(&mut self, brain: Uuid, now: DateTime<Utc>) -> Result<()> {
        self.db.execute("UPDATE events SET state='pending' WHERE binding IN(SELECT id FROM bindings WHERE brain=?1) AND state='denied' AND body IS NOT NULL AND retry_at<=?2 AND error_code='capture_denied'",
            params![brain.to_string(),now.timestamp()])?;
        Ok(())
    }
    pub fn acknowledge(&mut self, receipt: &CaptureReceipt) -> Result<()> {
        self.acknowledge_delivery(receipt.event_id, receipt)
    }
    pub fn acknowledge_delivery(&mut self, local_id: Uuid, receipt: &CaptureReceipt) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (matches, state): (bool, String) = tx.query_row(
            "SELECT binding=?2,state FROM events WHERE id=?1",
            params![local_id.to_string(), receipt.binding_id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        ensure!(matches, "capture_receipt_mismatch");
        if matches!(state.as_str(), "removed" | "expired") {
            tx.commit()?;
            return Ok(());
        }
        ensure!(
            matches!(receipt.state.as_str(), "accepted" | "expired" | "removed"),
            "invalid_capture_receipt"
        );
        clear(
            &tx,
            &local_id.to_string(),
            if receipt.state == "accepted" {
                "delivered"
            } else {
                &receipt.state
            },
        )?;
        tx.execute(
            "UPDATE events SET receipt=?2 WHERE id=?1",
            params![local_id.to_string(), serde_json::to_string(receipt)?],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn expire(&mut self, now: DateTime<Utc>) -> Result<usize> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count=tx.execute("UPDATE events SET body=NULL,bytes=0,state='expired',error_code=NULL WHERE body IS NOT NULL AND expires_at<=?1",[now.timestamp()])?;
        tx.commit()?;
        Ok(count)
    }
    pub fn erase_events(&mut self, ids: &[Uuid]) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        for id in ids {
            clear(&tx, &id.to_string(), "removed")?;
        }
        tx.commit()?;
        Ok(())
    }
    /// Shared privacy cleanup runs before the device acknowledges its position.
    /// Persist unknown fences too, so replay cannot resurrect an older inbox.
    pub fn apply_privacy(&mut self, sync: &PrivacyDeviceSync, now: DateTime<Utc>) -> Result<usize> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        for f in &sync.capture_event_fences {
            ensure!(
                !f.event_id.is_nil()
                    && !f.binding_id.is_nil()
                    && f.native_key.as_ref().is_none_or(|key| {
                        key.len() <= 1200
                            && serde_json::from_str::<Vec<serde_json::Value>>(key)
                                .ok()
                                .is_some_and(|parts| {
                                    parts.len() == 6
                                        && matches!(
                                            parts[0].as_str(),
                                            Some("codex" | "claude_code")
                                        )
                                        && parts.iter().skip(1).all(|p| {
                                            p.is_null()
                                                || p.as_str().is_some_and(|s| {
                                                    s.is_empty() || capture_identity(s)
                                                })
                                        })
                                })
                    }),
                "invalid_capture_fence"
            );
            tx.execute(
                "INSERT OR IGNORE INTO fences(brain,event,binding,native_key) VALUES(?1,?2,?3,?4)",
                params![
                    sync.brain_id.to_string(),
                    f.event_id.to_string(),
                    f.binding_id.to_string(),
                    f.native_key
                ],
            )?;
        }
        let mut count = tx.execute("UPDATE events SET body=NULL,bytes=0,state='removed',error_code=NULL WHERE binding IN(SELECT id FROM bindings WHERE brain=?1) AND EXISTS(SELECT 1 FROM fences f WHERE f.brain=?1 AND (f.event=events.id OR (f.binding=events.binding AND f.native_key=events.native_key) OR f.event=json_extract(events.receipt,'$.event_id'))) AND state<>'removed'",[sync.brain_id.to_string()])?;
        // Clear already-expired bodies before considering a longer new policy.
        count += tx.execute("UPDATE events SET body=NULL,bytes=0,state='expired',error_code=NULL WHERE binding IN(SELECT id FROM bindings WHERE brain=?1) AND body IS NOT NULL AND expires_at<=?2",params![sync.brain_id.to_string(),now.timestamp()])?;
        tx.execute("UPDATE events SET expires_at=created_at+86400*CASE WHEN json_extract(body,'$.event.kind')='tool_result' THEN ?2 ELSE ?3 END WHERE binding IN(SELECT id FROM bindings WHERE brain=?1) AND body IS NOT NULL",
            params![sync.brain_id.to_string(),sync.policy.tool_output_days,sync.policy.raw_session_days])?;
        count += tx.execute("UPDATE events SET body=NULL,bytes=0,state='expired',error_code=NULL WHERE binding IN(SELECT id FROM bindings WHERE brain=?1) AND body IS NOT NULL AND expires_at<=?2",params![sync.brain_id.to_string(),now.timestamp()])?;
        let bindings: Vec<(String, String)> = {
            let mut statement = tx.prepare("SELECT id,body FROM bindings WHERE brain=?1")?;
            statement
                .query_map([sync.brain_id.to_string()], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<std::result::Result<_, _>>()?
        };
        for (id, body) in bindings {
            let mut saved: CachedCaptureBinding = serde_json::from_str(&body)?;
            saved.retention = sync.policy.clone();
            tx.execute(
                "UPDATE bindings SET body=?2 WHERE id=?1",
                params![id, serde_json::to_string(&saved)?],
            )?;
        }
        tx.commit()?;
        Ok(count)
    }
    pub fn failed_delivery(
        &mut self,
        id: Uuid,
        code: &str,
        retryable: bool,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let code = match code {
            "transport_unavailable"
            | "server_unavailable"
            | "capture_denied"
            | "capture_identity_conflict"
            | "capture_binding_missing" => code,
            _ => "capture_delivery_failed",
        };
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let attempts: i64 = tx.query_row(
            "SELECT attempts FROM events WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )?;
        let delay = if retryable {
            (1_i64 << attempts.clamp(0, 6)).min(60)
        } else {
            60
        };
        tx.execute("UPDATE events SET attempts=attempts+1,retry_at=?2,error_code=?3,state=?4 WHERE id=?1 AND state='pending'",
            params![id.to_string(),(now+Duration::seconds(delay)).timestamp(),code,if retryable {"pending"} else {"denied"}])?;
        tx.commit()?;
        Ok(())
    }
    pub fn status(&self) -> Result<InboxStatus> {
        let (pending,pending_bytes,delivered,removed,denied)=self.db.query_row("SELECT coalesce(sum(state='pending'),0),coalesce(sum(bytes),0),coalesce(sum(state='delivered'),0),coalesce(sum(state IN ('removed','expired')),0),coalesce(sum(state='denied'),0) FROM events",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        let mut statement = self
            .db
            .prepare("SELECT code,total FROM gaps ORDER BY code")?;
        let gaps = statement
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(InboxStatus {
            pending,
            pending_bytes,
            delivered,
            denied,
            removed,
            gaps,
        })
    }
    pub fn brain_report(&self, brain: Uuid) -> Result<CaptureDeviceReport> {
        let (pending,denied) = self.db.query_row("SELECT coalesce(sum(e.state='pending'),0),coalesce(sum(e.state='denied'),0) FROM events e JOIN bindings b ON b.id=e.binding WHERE b.brain=?1",
            [brain.to_string()], |r| Ok((r.get(0)?,r.get(1)?)))?;
        let gaps: i64 = self
            .db
            .query_row("SELECT coalesce(sum(total),0) FROM gaps", [], |r| r.get(0))?;
        Ok(CaptureDeviceReport {
            pending,
            denied,
            device_gap_count: gaps.min(1_000_000_000_000),
            issue: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::path::PathBuf;
    const ENDPOINT: &str = "http://127.0.0.1:8787";
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.cache/native-capture-tests")
                    .join(Uuid::new_v4().to_string()),
            )
        }
        fn assert_absent(&self, text: &str) {
            for f in fs::read_dir(&self.0).unwrap() {
                let bytes = fs::read(f.unwrap().path()).unwrap();
                assert!(
                    !bytes.windows(text.len()).any(|b| b == text.as_bytes()),
                    "Removed content survived in a controlled inbox file"
                );
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if self.0.exists() {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }
    fn saved(device: Uuid, now: DateTime<Utc>) -> CachedCaptureBinding {
        let brain = Uuid::new_v4();
        let task = Uuid::new_v4();
        CachedCaptureBinding {
            binding: CaptureBinding {
                id: Uuid::new_v4(),
                brain_id: brain,
                device_id: device,
                host: "codex".into(),
                host_version: "0.154.0".into(),
                created_at: now,
                operation: OperationBinding {
                    id: Uuid::new_v4(),
                    brain_id: brain,
                    task_id: task,
                    kind: "capture".into(),
                    actor_id: Uuid::new_v4(),
                    device_id: Some(device),
                    scope_valid: true,
                    created_at: now,
                    scope: ScopeSnapshot {
                        id: Uuid::new_v4(),
                        task_id: task,
                        brain_id: brain,
                        selection: ScopeSelection::default(),
                        repositories: vec![],
                        areas: vec![],
                        environment: None,
                        created_at: now,
                    },
                },
            },
            policy: CapturePolicy {
                enabled: true,
                ..Default::default()
            },
            retention: RetentionPolicy::default(),
            synchronized_at: now,
            agent_id: None,
        }
    }
    fn prompt(turn: &str, text: &str) -> Value {
        json!({"hook_event_name":"UserPromptSubmit","session_id":"root-session","turn_id":turn,"prompt":text})
    }
    fn capture(db: &mut Inbox, b: Uuid, payload: &Value, now: DateTime<Utc>) -> Uuid {
        db.capture(b, &serde_json::to_vec(payload).unwrap(), &[], now)
            .unwrap()
    }
    #[test]
    fn inbox_pins_concurrent_scope_replays_sanitized_events_and_rejects_wrong_profile() {
        let fixture = Fixture::new();
        let now = Utc::now();
        let device = Uuid::new_v4();
        let mut inbox = Inbox::open(&fixture.0, ENDPOINT, device).unwrap();
        let a = saved(device, now);
        let mut b = a.clone();
        b.binding.id = Uuid::new_v4();
        b.binding.operation.id = Uuid::new_v4();
        b.binding.operation.scope.id = Uuid::new_v4();
        b.binding.operation.scope.selection.environment_id = Some(Uuid::new_v4());
        inbox.remember(&a).unwrap();
        inbox.remember(&b).unwrap();
        let first = capture(
            &mut inbox,
            a.binding.id,
            &prompt("turn-a", "port=8080\nAPI_KEY=fixture-private-credential"),
            now,
        );
        let repeated = capture(
            &mut inbox,
            b.binding.id,
            &prompt("turn-a", "port=8080\nAPI_KEY=fixture-private-credential"),
            now + Duration::seconds(1),
        );
        assert_eq!(first, repeated);
        let start = json!({"hook_event_name":"PreToolUse","session_id":"root-session","turn_id":"turn-a","tool_name":"Bash","tool_use_id":"old-tool","tool_input":{"command":"inspect"}});
        capture(&mut inbox, a.binding.id, &start, now);
        capture(
            &mut inbox,
            b.binding.id,
            &prompt("turn-b", "independent observation"),
            now,
        );
        let stop = json!({"hook_event_name":"PostToolUse","session_id":"root-session","turn_id":"turn-b","tool_name":"Bash","tool_use_id":"old-tool","tool_input":{"command":"inspect"},"tool_response":{"exit_code":1,"stdout":"declared port is unavailable"}});
        let late = capture(&mut inbox, b.binding.id, &stop, now);
        let ambiguous = json!({"hook_event_name":"Stop","session_id":"root-session","turn_id":"unknown-child","last_assistant_message":"unattributed text must not be stored"});
        let gap = capture(&mut inbox, b.binding.id, &ambiguous, now);
        let recalled = json!({"hook_event_name":"PostToolUse","session_id":"root-session","turn_id":"turn-b",
            "tool_name":"Bash","tool_use_id":"own-recall","tool_input":{"command":"'/local/with spaces/recollect-agent' scope recall brain operation query"},
            "tool_response":{"stdout":"OWN_RECALL_MUST_NEVER_REACH_SQLITE"}});
        let excluded = capture(&mut inbox, b.binding.id, &recalled, now);
        let events = inbox.pending(now).unwrap();
        let excluded = events.iter().find(|e| e.id == excluded).unwrap();
        assert!(excluded.event.content.is_none());
        assert!(
            excluded
                .event
                .coverage
                .contains(&"excluded_tool_content".into())
        );
        fixture.assert_absent("OWN_RECALL_MUST_NEVER_REACH_SQLITE");
        assert_eq!(
            events.iter().find(|e| e.id == late).unwrap().binding_id,
            a.binding.id
        );
        assert_eq!(
            events.iter().find(|e| e.id == late).unwrap().event.outcome,
            "failed"
        );
        assert!(
            events
                .iter()
                .find(|e| e.id == gap)
                .unwrap()
                .event
                .content
                .is_none()
        );
        assert!(
            inbox
                .capture(
                    a.binding.id,
                    &serde_json::to_vec(&prompt("turn-a", "different content")).unwrap(),
                    &[],
                    now
                )
                .is_err()
        );
        assert_eq!(inbox.status().unwrap().pending, 6);
        drop(inbox);
        fixture.assert_absent("fixture-private-credential");
        fixture.assert_absent("unattributed text must not be stored");
        let before = fs::read(fixture.0.join("inbox.sqlite")).unwrap();
        assert!(Inbox::open(&fixture.0, ENDPOINT, Uuid::new_v4()).is_err());
        assert_eq!(fs::read(fixture.0.join("inbox.sqlite")).unwrap(), before);
        let mut resumed = Inbox::open(&fixture.0, ENDPOINT, device).unwrap();
        assert_eq!(resumed.pending(now).unwrap().len(), 6);
    }
    #[test]
    fn managed_launch_scope_switch_is_durable_and_preserves_pins_and_other_launches() {
        let fixture = Fixture::new();
        let now = Utc::now();
        let device = Uuid::new_v4();
        let mut inbox = Inbox::open(&fixture.0, ENDPOINT, device).unwrap();
        let a = saved(device, now);
        let mut b = a.clone();
        b.binding.id = Uuid::new_v4();
        b.binding.operation.id = Uuid::new_v4();
        b.binding.operation.scope.id = Uuid::new_v4();
        let mut child = b.clone();
        child.binding.id = Uuid::new_v4();
        child.binding.operation.task_id = Uuid::new_v4();
        child.binding.operation.scope.task_id = child.binding.operation.task_id;
        for saved in [&a, &b, &child] {
            inbox.remember(saved).unwrap();
        }
        let launch = Uuid::new_v4();
        let other = Uuid::new_v4();
        inbox.start_launch(launch, a.binding.id).unwrap();
        inbox.start_launch(other, a.binding.id).unwrap();
        let record = |inbox: &mut Inbox, payload: Value| {
            inbox
                .capture_in_launch(
                    a.binding.id,
                    Some(launch),
                    &serde_json::to_vec(&payload).unwrap(),
                    &[],
                    now,
                )
                .unwrap()
        };
        let old = record(&mut inbox, prompt("old", "scope A observation"));
        record(
            &mut inbox,
            json!({"hook_event_name":"PreToolUse","session_id":"root-session",
            "turn_id":"old","tool_use_id":"delayed","tool_name":"Bash","tool_input":{"command":"inspect"}}),
        );
        inbox
            .advance_launch(launch, a.binding.id, a.binding.id, b.binding.id)
            .unwrap();
        assert!(
            inbox
                .advance_launch(launch, a.binding.id, a.binding.id, a.binding.id)
                .is_err(),
            "a late publisher cannot overwrite a new pointer"
        );
        assert!(
            inbox
                .advance_launch(launch, a.binding.id, b.binding.id, child.binding.id)
                .is_err(),
            "a child is never the parent's future default"
        );
        assert_eq!(
            inbox.launch_default(other, a.binding.id).unwrap(),
            a.binding.id
        );
        drop(inbox);
        let mut inbox = Inbox::open(&fixture.0, ENDPOINT, device).unwrap();
        assert_eq!(
            inbox.launch_default(launch, a.binding.id).unwrap(),
            b.binding.id
        );
        assert_eq!(
            record(&mut inbox, prompt("old", "scope A observation")),
            old
        );
        let new = record(&mut inbox, prompt("new", "scope B observation"));
        let late = record(
            &mut inbox,
            json!({"hook_event_name":"PostToolUse","session_id":"root-session",
            "turn_id":"new","tool_use_id":"delayed","tool_name":"Bash","tool_response":{"stdout":"delayed observation"}}),
        );
        let ambiguous = record(
            &mut inbox,
            json!({"hook_event_name":"Stop","session_id":"root-session",
            "turn_id":"unbound-child","agent_id":"child","last_assistant_message":"must stay unattributed"}),
        );
        let pending = inbox.pending(now).unwrap();
        assert_eq!(
            pending.iter().find(|e| e.id == new).unwrap().binding_id,
            b.binding.id
        );
        assert_eq!(
            pending.iter().find(|e| e.id == late).unwrap().binding_id,
            a.binding.id
        );
        assert!(
            pending
                .iter()
                .find(|e| e.id == ambiguous)
                .unwrap()
                .event
                .content
                .is_none()
        );
        assert_eq!(
            inbox
                .cached_binding(a.binding.id)
                .unwrap()
                .binding
                .operation
                .scope
                .id,
            a.binding.operation.scope.id
        );
        fixture.assert_absent("must stay unattributed");
    }
    #[test]
    fn inbox_concurrent_hooks_acknowledgment_expiry_and_erasure_remove_payload_bytes() {
        let fixture = Fixture::new();
        let now = Utc::now();
        let device = Uuid::new_v4();
        let config = saved(device, now);
        let mut inbox = Inbox::open(&fixture.0, ENDPOINT, device).unwrap();
        inbox.remember(&config).unwrap();
        drop(inbox);
        let mut threads = vec![];
        for n in 0..6 {
            let root = fixture.0.clone();
            let binding = config.binding.id;
            threads.push(std::thread::spawn(move || {
                let mut db = Inbox::open(&root, ENDPOINT, device).unwrap();
                capture(
                    &mut db,
                    binding,
                    &prompt(
                        &format!("t{n}"),
                        &format!("retained-raw-event-{n}-long-fixture-text"),
                    ),
                    now,
                )
            }));
        }
        let ids: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        let mut inbox = Inbox::open(&fixture.0, ENDPOINT, device).unwrap();
        assert_eq!(inbox.status().unwrap().pending, 6);
        inbox
            .acknowledge(&CaptureReceipt {
                event_id: ids[0],
                binding_id: config.binding.id,
                state: "accepted".into(),
                source_id: Some(Uuid::new_v4()),
                source_version_id: Some(Uuid::new_v4()),
                expires_at: Some(now + Duration::days(30)),
                received_at: now,
            })
            .unwrap();
        inbox.erase_events(&[ids[1]]).unwrap();
        inbox
            .acknowledge(&CaptureReceipt {
                event_id: ids[1],
                binding_id: config.binding.id,
                state: "accepted".into(),
                source_id: Some(Uuid::new_v4()),
                source_version_id: Some(Uuid::new_v4()),
                expires_at: Some(now + Duration::days(30)),
                received_at: now,
            })
            .unwrap();
        assert_eq!(inbox.pending(now).unwrap().len(), 4);
        let mut tighter = config.clone();
        tighter.retention.raw_session_days = 1;
        tighter.synchronized_at = now + Duration::hours(1);
        inbox.remember(&tighter).unwrap();
        assert!(inbox.pending(now + Duration::days(2)).unwrap().is_empty());
        assert_eq!(inbox.status().unwrap().delivered, 1);
        assert_eq!(inbox.status().unwrap().removed, 5);
        drop(inbox);
        for n in 0..6 {
            fixture.assert_absent(&format!("retained-raw-event-{n}-long-fixture-text"));
        }
        assert!(!fixture.0.join("inbox.sqlite-wal").exists());
        assert!(!fixture.0.join("inbox.sqlite-journal").exists());
    }
    #[test]
    fn inbox_backpressure_retry_and_stale_policy_preserve_independent_events() {
        let fixture = Fixture::new();
        let now = Utc::now();
        let device = Uuid::new_v4();
        let config = saved(device, now);
        let mut inbox = Inbox::open(&fixture.0, ENDPOINT, device).unwrap();
        inbox.remember(&config).unwrap();
        inbox.max_pending = 2;
        let first = capture(
            &mut inbox,
            config.binding.id,
            &prompt("t1", "first preserved"),
            now,
        );
        let second = capture(
            &mut inbox,
            config.binding.id,
            &prompt("t2", "independent preserved"),
            now,
        );
        assert!(
            inbox
                .capture(
                    config.binding.id,
                    &serde_json::to_vec(&prompt("t3", "must not replace queued content")).unwrap(),
                    &[],
                    now
                )
                .is_err()
        );
        inbox
            .failed_delivery(first, "transport_unavailable", true, now)
            .unwrap();
        let ready = inbox.pending(now).unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, second);
        assert_eq!(inbox.pending(now + Duration::seconds(2)).unwrap().len(), 2);
        assert!(
            inbox
                .capture(
                    config.binding.id,
                    &serde_json::to_vec(&prompt("t4", "stale permission must not retain this"))
                        .unwrap(),
                    &[],
                    now + Duration::hours(25)
                )
                .is_err()
        );
        assert_eq!(inbox.status().unwrap().pending, 2);
        drop(inbox);
        fixture.assert_absent("must not replace queued content");
        fixture.assert_absent("stale permission must not retain this");
    }
}
