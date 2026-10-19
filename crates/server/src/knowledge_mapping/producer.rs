//! Retain validated output until a release-qualified producer can use it.
//! This module never invokes a provider or changes a standing model policy.
use super::{SourceCandidates, production, source_candidates, storage};
use crate::db::admission::{self, BrainAccess, Role};
use crate::{
    AppState, db,
    error::{Error, Result},
    graph, learning,
    memory_evidence::Tx,
    model_gateway, model_policy,
    worker::ClaimedJob,
};
use chrono::{DateTime, Duration, Utc};
use recollect_protocol::{LearningRun, ModelRequest};
use serde_json::Value;
use sqlx::types::Json;
use uuid::Uuid;

const STAGE_CAPACITY: i64 = 1000;

pub(super) async fn retain(
    tx: &mut Tx<'_>,
    run: &LearningRun,
    request: &ModelRequest,
    context: &Value,
    candidates: &SourceCandidates,
    source_deadline: Option<DateTime<Utc>>,
) -> Result<bool> {
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM knowledge_mapping_stages WHERE brain_id=$1 AND run_id=$2)",
    )
    .bind(run.brain_id)
    .bind(run.id)
    .fetch_one(&mut **tx)
    .await?;
    if !existing {
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM knowledge_mapping_stages WHERE brain_id=$1")
                .bind(run.brain_id)
                .fetch_one(&mut **tx)
                .await?;
        if count >= STAGE_CAPACITY {
            return Ok(false);
        }
    }
    let audit: Option<DateTime<Utc>> = sqlx::query_scalar("SELECT recollect_retention_deadline(brain_id,'audit',created_at) FROM model_requests WHERE brain_id=$1 AND id=$2 AND state='succeeded' AND NOT suppressed AND NOT detail_expired")
        .bind(run.brain_id).bind(request.id).fetch_optional(&mut **tx).await?.ok_or_else(crate::retention::unavailable)?;
    let expires = [source_deadline, audit, Some(Utc::now() + Duration::days(7))]
        .into_iter()
        .flatten()
        .min()
        .unwrap();
    if expires <= Utc::now() {
        return Ok(false);
    }
    let payload = serde_json::to_value(candidates)
        .map_err(|_| Error::invalid("Invalid retained mapping stage."))?;
    sqlx::query("INSERT INTO knowledge_mapping_stages(run_id,brain_id,request_id,source_version_id,context,payload,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(run_id) DO NOTHING")
        .bind(run.id).bind(run.brain_id).bind(request.id).bind(run.source_version_id).bind(Json(context)).bind(Json(&payload)).bind(expires).execute(&mut **tx).await?;
    let same: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM knowledge_mapping_stages WHERE brain_id=$1 AND run_id=$2 AND request_id=$3 AND source_version_id=$4 AND context=$5 AND payload=$6)")
        .bind(run.brain_id).bind(run.id).bind(request.id).bind(run.source_version_id).bind(Json(context)).bind(Json(&payload)).fetch_one(&mut **tx).await?;
    if !same {
        return Err(Error::invalid("The retained mapping identity changed."));
    }
    Ok(true)
}

#[derive(Clone, sqlx::FromRow)]
struct Stage {
    request_id: Uuid,
    source_version_id: Uuid,
    context: Json<Value>,
    payload: Json<Value>,
    expires_at: DateTime<Utc>,
}

async fn current(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
    id: Uuid,
) -> Result<Option<(Stage, LearningRun, ModelRequest)>> {
    let stage: Option<Stage>=sqlx::query_as("SELECT request_id,source_version_id,context,payload,expires_at FROM knowledge_mapping_stages WHERE brain_id=$1 AND run_id=$2 AND expires_at>clock_timestamp()")
        .bind(brain).bind(id).fetch_optional(&mut **tx).await?;
    let Some(stage) = stage else { return Ok(None) };
    let run = learning::row(tx, brain, id).await?;
    let request = model_policy::request(tx, brain, stage.request_id).await?;
    let exact: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM learning_runs r JOIN model_requests q ON q.brain_id=r.brain_id AND q.id=r.request_id JOIN model_policy_heads h ON h.brain_id=r.brain_id AND h.policy_id=r.policy_id JOIN model_policies p ON p.brain_id=h.brain_id AND p.id=h.policy_id WHERE r.brain_id=$1 AND r.id=$2 AND r.actor_id=$3 AND r.device_id IS NOT DISTINCT FROM $4 AND r.state='succeeded' AND r.source_version_id=$5 AND q.id=$6 AND q.operation_id=r.id AND q.actor_id=r.actor_id AND q.device_id IS NOT DISTINCT FROM r.device_id AND q.policy_id=r.policy_id AND q.purpose='extraction' AND q.state='succeeded' AND NOT q.suppressed AND NOT q.detail_expired AND (recollect_retention_deadline(q.brain_id,'audit',q.created_at) IS NULL OR recollect_retention_deadline(q.brain_id,'audit',q.created_at)>clock_timestamp()) AND p.policy->>'enabled'='true' AND p.policy->'purposes' ? 'extraction')")
        .bind(brain).bind(id).bind(actor).bind(device).bind(stage.source_version_id).bind(stage.request_id).fetch_one(&mut **tx).await?;
    if !exact {
        return Ok(None);
    }
    Ok(Some((stage, run, request)))
}

async fn promote(
    state: &AppState,
    brain: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
    id: Uuid,
) -> Result<bool> {
    let mut tx = admission::brain_preparation_tx(
        &state.pool,
        actor,
        device,
        brain,
        BrainAccess::READ.with_role(Role::Writer),
    )
    .await?
    .tx;
    sqlx::query("SET LOCAL statement_timeout='750ms'")
        .execute(&mut *tx)
        .await?;
    let Some((stage, run, request)) = current(&mut tx, brain, actor, device, id).await? else {
        return Ok(false);
    };
    let epoch = graph::preparation_epoch(&mut tx, brain).await?;
    let policy = model_policy::current(state, &mut tx, brain).await?;
    if !model_policy::matches_installation(state, &policy.policy) {
        return Ok(false);
    }
    let source = model_gateway::source(
        state,
        &mut tx,
        brain,
        stage.source_version_id,
        policy.policy.max_input_bytes as usize,
    )
    .await?;
    let context =
        storage::context(&mut tx, brain, stage.source_version_id, source.source_id).await?;
    let deadline = storage::input_gate(&mut tx, brain, stage.source_version_id).await?;
    let candidate_bytes = serde_json::to_vec(&stage.payload.0)
        .map_err(|_| Error::invalid("Invalid retained mapping stage."))?;
    let candidates = source_candidates(stage.source_version_id, &source.text, &candidate_bytes)
        .map_err(|failure| {
            tracing::warn!(brain_id=%brain, version_id=%stage.source_version_id, failure=failure.code(), "Retained mapping stage no longer validates");
            Error::invalid("Invalid retained mapping source.")
        })?;
    tx.commit().await?;
    let mut tx = admission::brain_tx_as(
        &state.pool,
        actor,
        device,
        brain,
        BrainAccess::WRITE
            .with_statement_timeout(std::time::Duration::from_millis(750))
            .with_lock_timeout(std::time::Duration::from_millis(250)),
    )
    .await?
    .tx;
    graph::preparation_unchanged(&mut tx, brain, epoch).await?;
    let Some((fresh, _, _)) = current(&mut tx, brain, actor, device, id).await? else {
        return Ok(false);
    };
    if fresh.request_id != stage.request_id
        || fresh.payload.0 != stage.payload.0
        || fresh.context.0 != stage.context.0
    {
        return Ok(false);
    }
    if context != stage.context.0 || deadline.is_some_and(|at| at <= Utc::now()) {
        sqlx::query("DELETE FROM knowledge_mapping_stages WHERE brain_id=$1 AND run_id=$2")
            .bind(brain)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(false);
    }
    let _: Option<Uuid> = sqlx::query_scalar("SELECT recollect_mapping_bootstrap($1)")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    production::stage_learning(
        &mut tx,
        &run,
        &request,
        &source,
        production::PreparedMapping::Validated {
            candidates,
            source: source.source_id,
            context,
            deadline: Some(
                [Some(stage.expires_at), deadline]
                    .into_iter()
                    .flatten()
                    .min()
                    .unwrap(),
            ),
        },
    )
    .await?;
    let staged: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM knowledge_mapping_coverage WHERE brain_id=$1 AND run_id=$2 AND state='staged')").bind(brain).bind(id).fetch_one(&mut *tx).await?;
    if !staged {
        tx.commit().await?;
        return Ok(false);
    }
    let audit = db::audit(
        &mut tx,
        actor,
        brain,
        "knowledge.mapping.resume",
        id,
        "retained_validated_output",
    )
    .await?;
    let placeholder = ClaimedJob {
        id: Uuid::nil(),
        brain_id: brain,
        actor_id: actor,
        target_id: id,
        kind: "knowledge.map".into(),
        lease_token: Uuid::nil(),
        device_id: device,
    };
    production::enqueue_learning(&mut tx, &placeholder, &run, audit).await?;
    sqlx::query("DELETE FROM knowledge_mapping_stages WHERE brain_id=$1 AND run_id=$2")
        .bind(brain)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let publication_deadline = [Some(stage.expires_at), deadline]
        .into_iter()
        .flatten()
        .min()
        .unwrap();
    let fresh_deadline: bool = sqlx::query_scalar("SELECT $1>clock_timestamp() AND EXISTS(SELECT 1 FROM accounts a WHERE a.id=$2 AND a.enabled AND (a.auth_kind='local' OR a.membership_until>clock_timestamp())) AND ($3::uuid IS NULL OR EXISTS(SELECT 1 FROM devices d WHERE d.id=$3 AND d.account_id=$2 AND d.claimed AND d.revoked_at IS NULL AND d.expires_at>clock_timestamp()))")
        .bind(publication_deadline).bind(actor).bind(device)
        .fetch_one(&mut *tx)
        .await?;
    if !fresh_deadline {
        return Err(crate::retention::unavailable());
    }
    tx.commit().await?;
    Ok(true)
}

/// Bounded maintenance of already authorized results, independent of whether
/// new autonomous extraction is enabled. No body is returned by routing SQL.
pub async fn run_once(state: &AppState) -> Result<usize> {
    sqlx::query("SELECT recollect_mapping_stage_maintain()")
        .execute(&state.pool)
        .await?;
    let routes: Vec<(Uuid, Uuid, Option<Uuid>, Uuid)> = sqlx::query_as(
        "SELECT brain_id,actor_id,device_id,run_id FROM recollect_mapping_stage_routes()",
    )
    .fetch_all(&state.pool)
    .await?;
    let mut promoted = 0;
    for (brain, actor, device, id) in routes {
        match tokio::time::timeout(
            std::time::Duration::from_millis(1500),
            promote(state, brain, actor, device, id),
        )
        .await
        {
            Ok(Ok(true)) => promoted += 1,
            Ok(Ok(false)) => {}
            Ok(Err(error)) => {
                tracing::warn!(brain_id=%brain,code=error.1,"Retained mapping is waiting")
            }
            Err(_) => {
                tracing::warn!(brain_id=%brain,"Retained mapping preparation exceeded its bound")
            }
        }
    }
    Ok(promoted)
}
