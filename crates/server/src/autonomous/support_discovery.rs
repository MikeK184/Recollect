//! Bounded provisional discovery. Fresh complete qualification, not the saved
//! cursor or a prepared Boolean, authorizes an audit's existing queue operation.
use crate::{
    AppState, db, error::Result, memory_evidence::Tx, memory_support, memory_support_audit,
    model_policy,
};
use recollect_protocol::ModelPolicyVersion;
use std::time::{Duration, Instant};
use uuid::Uuid;

const SCAN: i64 = 16;
const ADMISSIONS: usize = 4;
const PREPARATION_BUDGET: Duration = Duration::from_secs(2);
const PUBLICATION_BUDGET: Duration = Duration::from_secs(1);

pub struct Prepared {
    brain: Uuid,
    actor: Uuid,
    policy: Uuid,
    cursor_version: i64,
    next: Option<Uuid>,
    eligible: Vec<Uuid>,
}

fn enabled(state: &AppState, p: &ModelPolicyVersion, actor: Uuid) -> bool {
    p.created_by == Some(actor)
        && p.policy.enabled
        && p.policy.autonomous_memory
        && p.policy.purposes.iter().any(|v| v == "extraction")
        && p.policy.content_classes.iter().any(|v| v == "claim")
        && model_policy::matches_installation(state, &p.policy)
}

/// Statement cancellation is an unresolved candidate, never a verdict. A
/// savepoint lets the next walk make progress without accepting partial results.
async fn qualifies(
    tx: &mut Tx<'_>,
    brain: Uuid,
    policy: Uuid,
    revision: Uuid,
    timeout: &str,
) -> Result<bool> {
    sqlx::query("SAVEPOINT support_candidate")
        .execute(&mut **tx)
        .await?;
    sqlx::query("SELECT set_config('statement_timeout',$1,true)")
        .bind(timeout)
        .execute(&mut **tx)
        .await?;
    let result: std::result::Result<Option<Uuid>, sqlx::Error> =
        sqlx::query_scalar(include_str!("../memory_support_audit/qualify.sql"))
            .bind(brain)
            .bind(policy)
            .bind(memory_support::VERSION)
            .bind(revision)
            .fetch_optional(&mut **tx)
            .await;
    // Restore the enclosing transaction's timeout even on success. SELECT-only
    // qualification has no state to retain from this savepoint.
    sqlx::query("ROLLBACK TO SAVEPOINT support_candidate")
        .execute(&mut **tx)
        .await?;
    sqlx::query("RELEASE SAVEPOINT support_candidate")
        .execute(&mut **tx)
        .await?;
    match result {
        Ok(id) => Ok(id.is_some()),
        Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("57014") => {
            tracing::debug!(brain_id=%brain, "Support discovery qualification deferred after timeout");
            Ok(false)
        }
        Err(error) => Err(error.into()),
    }
}

/// Library entry point shared by the scheduler and native race controls. This
/// performs no mutation, reservation, model request or Brain admission lock.
pub async fn prepare(state: &AppState, brain: Uuid, actor: Uuid) -> Result<Option<Prepared>> {
    let mut tx = db::preparation_tx(&state.pool, actor, None).await?;
    if !matches!(
        db::require_role(&mut tx, brain, false).await?.as_str(),
        "writer" | "admin"
    ) {
        return Ok(None);
    }
    let archived: bool = sqlx::query_scalar("SELECT archived FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    let p = model_policy::current(state, &mut tx, brain).await?;
    if archived || !enabled(state, &p, actor) {
        return Ok(None);
    }
    let cursor: Option<(Option<Uuid>, i64)> = sqlx::query_as(
        "SELECT last_revision_id,version FROM memory_support_discovery
         WHERE brain_id=$1 AND policy_id=$2 AND verifier_version=$3",
    )
    .bind(brain)
    .bind(p.change_id)
    .bind(memory_support::VERSION)
    .fetch_optional(&mut *tx)
    .await?;
    let (last, cursor_version) = cursor.unwrap_or((None, 0));
    let mut batch: Vec<Uuid> =
        sqlx::query_scalar(include_str!("../memory_support_audit/batch.sql"))
            .bind(brain)
            .bind(p.change_id)
            .bind(memory_support::VERSION)
            .bind(last)
            .bind(SCAN)
            .bind(None::<Uuid>)
            .fetch_all(&mut *tx)
            .await?;
    // Wrap within remaining examination capacity. The upper bound prevents
    // duplicate identities in this pass, including a concurrently inserted head.
    if last.is_some() && batch.len() < SCAN as usize {
        let tail: Vec<Uuid> = sqlx::query_scalar(include_str!("../memory_support_audit/batch.sql"))
            .bind(brain)
            .bind(p.change_id)
            .bind(memory_support::VERSION)
            .bind(None::<Uuid>)
            .bind(SCAN - batch.len() as i64)
            .bind(last)
            .fetch_all(&mut *tx)
            .await?;
        batch.extend(tail);
    }
    let mut prepared = Prepared {
        brain,
        actor,
        policy: p.change_id,
        cursor_version,
        next: None,
        eligible: vec![],
    };
    let started = Instant::now();
    for revision in batch {
        if started.elapsed() >= PREPARATION_BUDGET || prepared.eligible.len() == ADMISSIONS {
            break;
        }
        if qualifies(&mut tx, brain, p.change_id, revision, "1s").await? {
            prepared.eligible.push(revision);
        }
        prepared.next = Some(revision);
    }
    tx.commit().await?;
    Ok(Some(prepared))
}

/// Commit progress independently of other maintenance. Canonical changes during
/// preparation are safe because every admitted revision is fully checked again.
pub async fn publish(state: &AppState, prepared: Prepared) -> Result<usize> {
    let mut tx = db::actor_tx(&state.pool, prepared.actor).await?;
    db::require_writer(&mut tx, prepared.brain).await?;
    let p = model_policy::current(state, &mut tx, prepared.brain).await?;
    if p.change_id != prepared.policy || !enabled(state, &p, prepared.actor) {
        return Ok(0);
    }
    let advanced = sqlx::query(
        "INSERT INTO memory_support_discovery(brain_id,policy_id,verifier_version,last_revision_id,version)
         SELECT $1,$2,$3,$4,1 WHERE $5=0
         ON CONFLICT(brain_id,policy_id,verifier_version) DO NOTHING",
    ).bind(prepared.brain).bind(prepared.policy).bind(memory_support::VERSION)
        .bind(prepared.next).bind(prepared.cursor_version).execute(&mut *tx).await?;
    let changed = if prepared.cursor_version == 0 {
        advanced.rows_affected() == 1
    } else {
        sqlx::query(
            "UPDATE memory_support_discovery SET last_revision_id=$4,version=version+1
             WHERE brain_id=$1 AND policy_id=$2 AND verifier_version=$3 AND version=$5",
        )
        .bind(prepared.brain)
        .bind(prepared.policy)
        .bind(memory_support::VERSION)
        .bind(prepared.next)
        .bind(prepared.cursor_version)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1
    };
    if !changed {
        return Ok(0);
    }
    let started = Instant::now();
    let mut qualified = vec![];
    for revision in prepared.eligible {
        if started.elapsed() >= PUBLICATION_BUDGET {
            break;
        }
        if qualifies(&mut tx, prepared.brain, prepared.policy, revision, "750ms").await? {
            qualified.push(revision);
        }
    }
    let count = memory_support_audit::enqueue(
        &mut tx,
        prepared.brain,
        prepared.actor,
        &p,
        ADMISSIONS,
        &qualified,
    )
    .await?;
    tx.commit().await?;
    Ok(count)
}

pub(crate) async fn schedule(state: &AppState, brain: Uuid, actor: Uuid) -> Result<usize> {
    match prepare(state, brain, actor).await? {
        Some(prepared) => publish(state, prepared).await,
        None => Ok(0),
    }
}
