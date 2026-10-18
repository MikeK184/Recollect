//! Provisional preparation and short, independently committed maintenance work.
//! Cursors store only scheduling progress; all publication uses fresh authority.
use crate::{
    AppState, db,
    error::{Error, Result},
    graph,
    memory_evidence::Tx,
    model_policy, session_digests,
};
use recollect_protocol::ModelPolicyVersion;
use std::{
    future::Future,
    time::{Duration, Instant},
};
use uuid::Uuid;

const SCAN: i64 = 16;
const PREPARE_TIME: Duration = Duration::from_secs(5);
const PUBLISH_TIME: Duration = Duration::from_millis(1500);

pub(crate) fn enabled(state: &AppState, p: &ModelPolicyVersion, actor: Uuid) -> bool {
    p.created_by == Some(actor)
        && p.policy.enabled
        && p.policy.autonomous_memory
        && model_policy::matches_installation(state, &p.policy)
}

fn deferred() -> Error {
    Error(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "maintenance_deferred",
        "Autonomous maintenance is waiting for a bounded retry.",
    )
}

/// A cancelled complete transaction rolls back; it is never empty coverage.
pub(crate) async fn bounded<T>(future: impl Future<Output = Result<T>>) -> Result<T> {
    tokio::time::timeout(PUBLISH_TIME, future)
        .await
        .map_err(|_| deferred())?
}

pub(crate) struct Cursor {
    pub brain: Uuid,
    pub actor: Uuid,
    pub policy: Uuid,
    kind: &'static str,
    last: Option<Uuid>,
    version: i64,
    pub next: Option<Uuid>,
    pub epoch: i64,
}

impl Cursor {
    pub(crate) async fn snapshot<'a>(
        state: &'a AppState,
        brain: Uuid,
        actor: Uuid,
        kind: &'static str,
    ) -> Result<Option<(Tx<'a>, ModelPolicyVersion, Self)>> {
        let mut tx = db::preparation_tx(&state.pool, actor, None).await?;
        sqlx::query("SET LOCAL statement_timeout='1s'")
            .execute(&mut *tx)
            .await?;
        if !matches!(
            db::require_role(&mut tx, brain, false).await?.as_str(),
            "writer" | "admin"
        ) {
            return Ok(None);
        }
        let p = model_policy::current(state, &mut tx, brain).await?;
        if !enabled(state, &p, actor) {
            return Ok(None);
        }
        let cursor: Option<(Option<Uuid>, i64)> = sqlx::query_as(
            "SELECT last_id,version FROM autonomous_maintenance_cursors WHERE brain_id=$1 AND policy_id=$2 AND kind=$3")
            .bind(brain).bind(p.change_id).bind(kind).fetch_optional(&mut *tx).await?;
        let (last, version) = cursor.unwrap_or((None, 0));
        let epoch = graph::preparation_epoch(&mut tx, brain).await?;
        let cursor = Self {
            brain,
            actor,
            policy: p.change_id,
            kind,
            last,
            version,
            next: None,
            epoch,
        };
        Ok(Some((tx, p, cursor)))
    }

    /// Identity windows are resolved before any expensive qualification. Wrap
    /// within the same bounded pass, excluding identities already examined.
    pub(crate) async fn batch(&self, tx: &mut Tx<'_>) -> Result<Vec<Uuid>> {
        let relation = match self.kind {
            "source" => "source_versions",
            "digest_discovery" => "capture_events",
            "digest" => "session_digest_partitions",
            "handover" => "claim_revisions",
            _ => unreachable!("Internal maintenance cursor kind"),
        };
        let statement = format!(
            "SELECT id FROM {relation} WHERE brain_id=$1 AND ($2::uuid IS NULL OR id>$2) AND ($3::uuid IS NULL OR id<=$3) ORDER BY id LIMIT $4"
        );
        let mut ids: Vec<Uuid> = sqlx::query_scalar(&statement)
            .bind(self.brain)
            .bind(self.last)
            .bind(None::<Uuid>)
            .bind(SCAN)
            .fetch_all(&mut **tx)
            .await?;
        if self.last.is_some() && ids.len() < SCAN as usize {
            let rest: Vec<Uuid> = sqlx::query_scalar(&statement)
                .bind(self.brain)
                .bind(None::<Uuid>)
                .bind(self.last)
                .bind(SCAN - ids.len() as i64)
                .fetch_all(&mut **tx)
                .await?;
            ids.extend(rest);
        }
        Ok(ids)
    }

    pub(crate) async fn writer<'a>(
        &self,
        state: &'a AppState,
        epoch: bool,
    ) -> Result<Option<(Tx<'a>, ModelPolicyVersion)>> {
        let mut tx = db::actor_tx(&state.pool, self.actor).await?;
        sqlx::query("SET LOCAL statement_timeout='750ms'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL lock_timeout='250ms'")
            .execute(&mut *tx)
            .await?;
        let started = Instant::now();
        db::require_writer(&mut tx, self.brain).await?;
        tracing::debug!(brain_id=%self.brain,kind=self.kind,lock_wait_ms=started.elapsed().as_millis(),
            "Autonomous maintenance writer admitted");
        let p = model_policy::current(state, &mut tx, self.brain).await?;
        if p.change_id != self.policy
            || !enabled(state, &p, self.actor)
            || (epoch && graph::preparation_epoch(&mut tx, self.brain).await? != self.epoch)
        {
            return Ok(None);
        }
        Ok(Some((tx, p)))
    }

    pub(crate) async fn advance_in(&self, tx: &mut Tx<'_>) -> Result<bool> {
        let changed = if self.version == 0 {
            sqlx::query("INSERT INTO autonomous_maintenance_cursors(brain_id,policy_id,kind,last_id,version) VALUES($1,$2,$3,$4,1) ON CONFLICT DO NOTHING")
                .bind(self.brain).bind(self.policy).bind(self.kind).bind(self.next).execute(&mut **tx).await?.rows_affected()
        } else {
            sqlx::query("UPDATE autonomous_maintenance_cursors SET last_id=$4,version=version+1 WHERE brain_id=$1 AND policy_id=$2 AND kind=$3 AND version=$5")
                .bind(self.brain).bind(self.policy).bind(self.kind).bind(self.next).bind(self.version).execute(&mut **tx).await?.rows_affected()
        };
        Ok(changed == 1)
    }

    pub(crate) async fn advance(&self, state: &AppState) -> Result<bool> {
        bounded(async {
            let Some((mut tx, _)) = self.writer(state, false).await? else {
                return Ok(false);
            };
            let changed = self.advance_in(&mut tx).await?;
            tx.commit().await?;
            Ok(changed)
        })
        .await
    }
}

pub struct PreparedDigest {
    cursor: Cursor,
    partition: Option<session_digests::PreparedPartition>,
}

impl PreparedDigest {
    /// Diagnostic count only; a prepared page still has no publication authority.
    pub fn page_count(&self) -> Option<usize> {
        self.partition.as_ref().map(session_digests::pages)
    }
}

/// The complete partition/page computation takes no exclusive Brain lock.
pub async fn prepare_digest(
    state: &AppState,
    brain: Uuid,
    actor: Uuid,
) -> Result<Option<PreparedDigest>> {
    let Some((mut tx, policy, mut cursor)) =
        Cursor::snapshot(state, brain, actor, "digest").await?
    else {
        return Ok(None);
    };
    let ids = cursor.batch(&mut tx).await?;
    tx.commit().await?;
    let mut prepared = None;
    let started = Instant::now();
    // A fresh read-only snapshot for each candidate permits statement cancellation
    // without poisoning the whole walk or accepting an incomplete partition.
    for id in ids {
        if started.elapsed() >= PREPARE_TIME {
            break;
        }
        cursor.next = Some(id);
        let result = tokio::time::timeout(PREPARE_TIME, async {
            let mut tx = db::preparation_tx(&state.pool, actor, None).await?;
            sqlx::query("SET LOCAL statement_timeout='1s'")
                .execute(&mut *tx)
                .await?;
            db::require_role(&mut tx, brain, false).await?;
            let current = model_policy::current(state, &mut tx, brain).await?;
            if current.change_id != policy.change_id || !enabled(state, &current, actor) {
                return Ok(None);
            }
            let epoch = graph::preparation_epoch(&mut tx, brain).await?;
            let value =
                session_digests::prepare_partition(state, &mut tx, brain, &current, id).await?;
            tx.commit().await?;
            Ok::<_, Error>(value.map(|value| (epoch, value)))
        })
        .await;
        match result {
            Ok(Ok(Some((epoch, value)))) => {
                cursor.epoch = epoch;
                prepared = Some(value);
                break;
            }
            Ok(Ok(None)) => {}
            _ => tracing::debug!(brain_id=%brain, "Digest partition remains unresolved"),
        }
    }
    Ok(Some(PreparedDigest {
        cursor,
        partition: prepared,
    }))
}

pub struct PreparedHandovers {
    cursor: Cursor,
    candidates: Vec<crate::handovers::PreparedRefresh>,
}

pub async fn prepare_handovers(
    state: &AppState,
    brain: Uuid,
    actor: Uuid,
) -> Result<Option<PreparedHandovers>> {
    let Some((mut tx, policy, mut cursor)) =
        Cursor::snapshot(state, brain, actor, "handover").await?
    else {
        return Ok(None);
    };
    let ids = cursor.batch(&mut tx).await?;
    tx.commit().await?;
    let mut candidates = Vec::new();
    let started = Instant::now();
    for id in ids {
        if started.elapsed() >= PREPARE_TIME || candidates.len() == 5 {
            break;
        }
        cursor.next = Some(id);
        let result = tokio::time::timeout(PREPARE_TIME, async {
            let mut tx = db::preparation_tx(&state.pool, actor, None).await?;
            sqlx::query("SET LOCAL statement_timeout='1s'")
                .execute(&mut *tx)
                .await?;
            db::require_role(&mut tx, brain, false).await?;
            let p = model_policy::current(state, &mut tx, brain).await?;
            if p.change_id != policy.change_id || !enabled(state, &p, actor) {
                return Ok(None);
            }
            let epoch = graph::preparation_epoch(&mut tx, brain).await?;
            let value =
                crate::handovers::prepare_refresh(state, &mut tx, brain, actor, &p, id).await?;
            tx.commit().await?;
            Ok::<_, Error>(value.map(|value| (epoch, value)))
        })
        .await;
        match result {
            Ok(Ok(Some((epoch, value)))) => {
                // Each separately prepared candidate is self-contained. If the
                // input epoch changes, retain only the new consistent cohort.
                if cursor.epoch != epoch {
                    candidates.clear();
                    cursor.epoch = epoch;
                }
                candidates.push(value);
            }
            Ok(Ok(None)) => {}
            _ => tracing::debug!(brain_id=%brain,"Handover refresh remains unresolved"),
        }
    }
    Ok(Some(PreparedHandovers { cursor, candidates }))
}

pub async fn publish_handovers(
    state: &AppState,
    prepared: PreparedHandovers,
    limit: usize,
) -> Result<usize> {
    let cursor = &prepared.cursor;
    if !cursor.advance(state).await? {
        return Ok(0);
    }
    let mut queued = 0;
    for candidate in prepared.candidates.into_iter().take(limit) {
        let outcome = bounded(async {
            let started = Instant::now();
            let Some((mut tx, policy)) = cursor.writer(state, true).await? else { return Ok(0); };
            let count = crate::handovers::publish_refresh(state, &mut tx, cursor.brain, cursor.actor, &policy, &candidate).await?;
            tx.commit().await?;
            tracing::debug!(brain_id=%cursor.brain,transaction_ms=started.elapsed().as_millis(),"Handover refresh publication completed");
            Ok(count)
        }).await;
        match outcome {
            Ok(count) => queued += count,
            Err(error) => {
                tracing::debug!(brain_id=%cursor.brain,code=error.1,"Handover refresh publication deferred")
            }
        }
    }
    Ok(queued)
}

pub async fn publish_digest(
    state: &AppState,
    prepared: PreparedDigest,
    limit: usize,
) -> Result<usize> {
    let cursor = &prepared.cursor;
    if !cursor.advance(state).await? {
        return Ok(0);
    }
    let Some(partition) = prepared.partition else {
        return Ok(0);
    };
    let mut queued = 0;
    let mut terminal = false;
    for page in 0..session_digests::pages(&partition) {
        let result = bounded(async {
            let started = Instant::now();
            let Some((mut tx, policy)) = cursor.writer(state, true).await? else { return Ok(session_digests::PageOutcome::Deferred); };
            let outcome = session_digests::publish_page(state, &mut tx, cursor.brain, cursor.actor,
                &policy, &partition, page, queued < limit).await?;
            tx.commit().await?;
            tracing::debug!(brain_id=%cursor.brain,transaction_ms=started.elapsed().as_millis(), "Digest page publication completed");
            Ok(outcome)
        }).await;
        match result {
            Ok(session_digests::PageOutcome::Resolved(count, stopped)) => {
                queued += count;
                terminal |= stopped;
            }
            Ok(session_digests::PageOutcome::Deferred) => return Ok(queued),
            Err(error) => {
                tracing::debug!(brain_id=%cursor.brain,code=error.1,"Digest page publication deferred");
                return Ok(queued);
            }
        }
    }
    bounded(async {
        let Some((mut tx, _)) = cursor.writer(state, true).await? else {
            return Ok(());
        };
        session_digests::finish_partition(
            &mut tx,
            cursor.brain,
            cursor.policy,
            &partition,
            terminal,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    })
    .await?;
    Ok(queued)
}

pub(crate) async fn discover_digests(state: &AppState, brain: Uuid, actor: Uuid) -> Result<()> {
    let Some((mut tx, _, mut cursor)) =
        Cursor::snapshot(state, brain, actor, "digest_discovery").await?
    else {
        return Ok(());
    };
    let ids = cursor.batch(&mut tx).await?;
    cursor.next = ids.last().copied();
    tx.commit().await?;
    bounded(async {
        let Some((mut tx, _)) = cursor.writer(state, false).await? else {
            return Ok(());
        };
        if !cursor.advance_in(&mut tx).await? {
            return Ok(());
        }
        if !ids.is_empty() {
            sqlx::query("SELECT recollect_digest_discover_events($1,$2)")
                .bind(brain)
                .bind(ids)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    })
    .await
}

pub(crate) async fn sources(state: &AppState, brain: Uuid, actor: Uuid) -> Result<usize> {
    let Some((mut tx, policy, mut cursor)) =
        Cursor::snapshot(state, brain, actor, "source").await?
    else {
        return Ok(0);
    };
    let ids = cursor.batch(&mut tx).await?;
    let mut inputs = Vec::new();
    for id in ids {
        cursor.next = Some(id);
        if source_eligible(&mut tx, brain, id, &policy).await? {
            inputs.push((
                id,
                crate::capture::source_selection(&mut tx, brain, id).await?,
            ));
        }
    }
    tx.commit().await?;
    if !cursor.advance(state).await? {
        return Ok(0);
    }
    let mut queued = 0;
    for (id, selection) in inputs.into_iter().take(10) {
        let outcome = bounded(async {
            let Some((mut tx, p)) = cursor.writer(state, true).await? else {
                return Ok(0);
            };
            if !source_eligible(&mut tx, brain, id, &p).await? {
                return Ok(0);
            }
            crate::jobs::capacity(&mut tx, brain).await?;
            crate::learning::enqueue(
                &mut tx,
                brain,
                actor,
                None,
                &recollect_protocol::LearningInput {
                    source_version_id: id,
                    selection,
                    manifest_revision_id: None,
                    operation_id: None,
                },
                p.change_id,
                crate::learning::QueueMode::Automatic,
            )
            .await?;
            tx.commit().await?;
            Ok(1)
        })
        .await;
        match outcome {
            Ok(count) => queued += count,
            Err(error) => {
                tracing::debug!(brain_id=%brain,code=error.1,"Source maintenance deferred")
            }
        }
    }
    Ok(queued)
}

async fn source_eligible(
    tx: &mut Tx<'_>,
    brain: Uuid,
    id: Uuid,
    policy: &ModelPolicyVersion,
) -> Result<bool> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM source_versions v JOIN sources s ON s.current_version=v.id AND s.brain_id=v.brain_id
        WHERE v.brain_id=$1 AND v.id=$2 AND v.processing='ready' AND v.artifact_id IS NOT NULL
        AND v.retention_class=ANY($3) AND v.byte_length<=$4
        AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
        AND NOT EXISTS(SELECT 1 FROM automatic_support_excerpts a WHERE a.brain_id=v.brain_id AND a.version_id=v.id)
        AND NOT recollect_source_model_fenced(v.brain_id,v.id)
        AND NOT EXISTS(SELECT 1 FROM learning_runs l WHERE l.brain_id=v.brain_id AND l.source_version_id=v.id AND l.policy_id=$5 AND l.automatic))")
        .bind(brain).bind(id).bind(&policy.policy.content_classes).bind(policy.policy.max_input_bytes).bind(policy.change_id)
        .fetch_one(&mut **tx).await.map_err(Into::into)
}
