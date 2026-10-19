//! Periodic exhaustive verification of ready generations.
//!
//! Reads compare each generation's sealing token and counts (one request) and
//! verify the relationships they return, so out-of-band damage to anything a
//! read relies on is seen at once. This pass closes the remaining gap: it fully
//! re-verifies every node and relationship of one ready generation per Brain per
//! maintenance pass once its last verification is older than 24 hours, or was
//! cleared by a read that saw damage. It never holds a Brain lock across native
//! I/O. Damage marks only that exact generation; the normal discovery step queues
//! its single replacement under the existing 500-job cap and dedupe.
use super::*;
use crate::db;

/// What one verification attempt found.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Checked {
    /// No ready generation is due.
    Idle,
    /// Every descriptor node and relationship is present; the stamp advanced.
    Verified,
    /// The generation no longer matches its descriptor and is marked damaged.
    Damaged,
}

/// Oldest stamp first; a cleared stamp (a read saw damage) precedes all.
const DUE: &str = "SELECT to_jsonb(g)-'descriptor',native_attempt_token,native_verified_at FROM graph_generations g
    WHERE brain_id=$1 AND state='ready' AND error_code IS NULL
      AND (native_verified_at IS NULL OR native_verified_at<clock_timestamp()-interval '24 hours')
    ORDER BY native_verified_at NULLS FIRST,created_at,id LIMIT 1";

type Due = (
    Json<GraphGeneration>,
    Option<Uuid>,
    Option<chrono::DateTime<chrono::Utc>>,
);

pub(super) async fn verify_one(state: &AppState, brain: Uuid, actor: Uuid) -> Result<Checked> {
    let mut tx = db::actor_tx(&state.pool, actor).await?;
    let due: Option<Due> = sqlx::query_as(DUE)
        .bind(brain)
        .fetch_optional(&mut *tx)
        .await?;
    let Some((Json(g), token, stamp)) = due else {
        return Ok(Checked::Idle);
    };
    let descriptor: Option<Json<Descriptor>> =
        sqlx::query_scalar("SELECT descriptor FROM graph_generations WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(g.id)
            .fetch_one(&mut *tx)
            .await?;
    // No transaction while talking to Neo4j.
    tx.commit().await?;
    let Some(Json(descriptor)) = descriptor else {
        return Ok(Checked::Idle);
    };
    match adapter::verify_full(state, &g, &descriptor, token).await {
        Ok(()) => {
            stamped(state, actor, &g, token, stamp).await?;
            Ok(Checked::Verified)
        }
        Err(error) if error.1 == "graph_import_mismatch" => {
            if damaged(state, actor, &g, token).await? {
                tracing::warn!(brain_id=%brain,generation_id=%g.id,kind=%g.kind,
                    "Native graph generation failed full verification; its replacement is queued by discovery");
            }
            Ok(Checked::Damaged)
        }
        // An outage proves nothing either way. Leave the stamp and retry later.
        Err(error) => Err(error),
    }
}

/// Advance the stamp only if this exact sealed import is still the ready one and
/// nothing changed the stamp meanwhile: a replaced generation, a re-imported
/// token, or a read that cleared the stamp during verification must not be
/// papered over by a verification that began before it.
async fn stamped(
    state: &AppState,
    actor: Uuid,
    g: &GraphGeneration,
    token: Option<Uuid>,
    seen: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<bool> {
    let mut tx = db::actor_tx(&state.pool, actor).await?;
    sqlx::query("SET LOCAL lock_timeout='2s'")
        .execute(&mut *tx)
        .await?;
    let changed = sqlx::query("UPDATE graph_generations SET native_verified_at=clock_timestamp() WHERE brain_id=$1 AND id=$2 AND state='ready' AND native_attempt_token IS NOT DISTINCT FROM $3 AND native_verified_at IS NOT DISTINCT FROM $4")
        .bind(g.brain_id).bind(g.id).bind(token).bind(seen)
        .execute(&mut *tx).await?.rows_affected();
    tx.commit().await?;
    Ok(changed == 1)
}

/// Record damage on the ready generation. The marker excludes it from further
/// deep checks (no repeated verification storm) and from the "a generation
/// already exists" discovery predicates, so discovery queues one replacement; a
/// queued or running replacement, or any failed one, satisfies those predicates
/// again. Reads keep failing closed on it until the replacement is ready.
async fn damaged(
    state: &AppState,
    actor: Uuid,
    g: &GraphGeneration,
    token: Option<Uuid>,
) -> Result<bool> {
    let mut tx = db::actor_tx(&state.pool, actor).await?;
    sqlx::query("SET LOCAL lock_timeout='2s'")
        .execute(&mut *tx)
        .await?;
    let changed = sqlx::query("UPDATE graph_generations SET error_code='graph_import_mismatch' WHERE brain_id=$1 AND id=$2 AND state='ready' AND error_code IS NULL AND native_attempt_token IS NOT DISTINCT FROM $3")
        .bind(g.brain_id).bind(g.id).bind(token)
        .execute(&mut *tx).await?.rows_affected();
    tx.commit().await?;
    Ok(changed == 1)
}
