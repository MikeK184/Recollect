use crate::{
    AppState,
    auth::Auth,
    db,
    error::{Error, Result},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use recollect_protocol::{Job, ProcessingStatus};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct JobRow {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub target_id: Uuid,
    pub kind: String,
    pub lane: String,
    pub state: String,
    pub progress: i16,
    pub attempts: i32,
    pub max_attempts: i32,
    pub error_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
impl From<JobRow> for Job {
    fn from(r: JobRow) -> Self {
        Self {
            id: r.id,
            brain_id: r.brain_id,
            target_id: r.target_id,
            kind: r.kind,
            lane: r.lane,
            state: r.state,
            progress: r.progress,
            attempts: r.attempts,
            max_attempts: r.max_attempts,
            error_code: r.error_code,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub async fn capacity(tx: &mut Transaction<'_, Postgres>, brain: Uuid) -> Result<()> {
    db::lock_brain(tx, brain, true).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM jobs WHERE brain_id=$1 AND state IN ('queued','running')",
    )
    .bind(brain)
    .fetch_one(&mut **tx)
    .await?;
    if count >= 500 {
        return Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "queue_full",
            "This Brain has too much pending work. Wait for processing and retry.",
        ));
    }
    Ok(())
}

pub async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    brain: Uuid,
    audit: Uuid,
) -> Result<Uuid> {
    enqueue_work(
        tx,
        actor,
        brain,
        audit,
        brain,
        "brain.refresh",
        "interactive",
    )
    .await
}

pub async fn enqueue_work(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    brain: Uuid,
    audit: Uuid,
    target: Uuid,
    kind: &str,
    lane: &str,
) -> Result<Uuid> {
    capacity(tx, brain).await?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO jobs(id,brain_id,actor_id,audit_id,target_id,kind,lane) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(id).bind(brain).bind(actor).bind(audit).bind(target).bind(kind).bind(lane).execute(&mut **tx).await?;
    Ok(id)
}

#[utoipa::path(get,path="/api/brains/{id}/jobs",operation_id="listBrainJobs",params(("id"=Uuid,Path)),responses((status=200,body=Vec<Job>)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Job>>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, false).await?;
    let rows = sqlx::query_as::<_, JobRow>(
        "SELECT * FROM jobs WHERE brain_id=$1 ORDER BY created_at DESC,id LIMIT 100",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(get,path="/api/brains/{id}/processing",params(("id"=Uuid,Path)),responses((status=200,body=ProcessingStatus)))]
pub async fn processing(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<ProcessingStatus>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, false).await?;
    let (current,refreshed_at): (bool,Option<DateTime<Utc>>) = sqlx::query_as("SELECT coalesce(b.change_id=d.source_change,false),d.refreshed_at FROM brains b LEFT JOIN brain_directory d ON d.brain_id=b.id WHERE b.id=$1")
        .bind(id).fetch_one(&mut *tx).await?;
    let (pending,running,failed): (i64,i64,i64) = sqlx::query_as("SELECT count(*) FILTER (WHERE state IN ('queued','running')),count(*) FILTER (WHERE state='running'),count(*) FILTER (WHERE state IN ('failed','cancelled')) FROM jobs WHERE brain_id=$1")
        .bind(id).fetch_one(&mut *tx).await?;
    let status = if current {
        "current"
    } else if running > 0 {
        "running"
    } else if pending > 0 {
        "queued"
    } else if failed > 0 {
        "failed"
    } else {
        "missing"
    };
    tx.commit().await?;
    Ok(Json(ProcessingStatus {
        state: status.into(),
        refreshed_at,
        pending_jobs: pending,
    }))
}

#[utoipa::path(post,path="/api/brains/{id}/jobs/{job}/retry",params(("id"=Uuid,Path),("job"=Uuid,Path)),responses((status=200,body=Job),(status=409,body=recollect_protocol::ApiError)))]
pub async fn retry(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, job)): Path<(Uuid, Uuid)>,
) -> Result<Json<Job>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let old =
        sqlx::query_as::<_, JobRow>("SELECT * FROM jobs WHERE id=$1 AND brain_id=$2 FOR UPDATE")
            .bind(job)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(Error::missing)?;
    if !matches!(old.state.as_str(), "failed" | "cancelled") {
        return Err(Error(
            StatusCode::CONFLICT,
            "job_not_retryable",
            "Only failed or cancelled jobs can be retried.",
        ));
    }
    if matches!(
        old.kind.as_str(),
        "source.learn" | "handover.generate" | "semantic.generate"
    ) {
        return Err(Error(
            StatusCode::CONFLICT,
            "learning_retry_required",
            "Use the learning, handover or semantic controls to start a separately recorded model attempt.",
        ));
    }
    if old.kind == "graph.analyze" {
        return Err(Error(
            StatusCode::CONFLICT,
            "analysis_retry_required",
            "Queue a new analysis from the current graph selection; the old report keeps its original inputs.",
        ));
    }
    if !crate::retention::job_available(&mut tx, id, old.target_id, &old.kind).await? {
        return Err(crate::retention::unavailable());
    }
    if old.kind == "graph.project" {
        let generation = crate::graph::generation(&mut tx, id, old.target_id).await?;
        if !matches!(generation.state.as_str(), "failed" | "cancelled") {
            return Err(Error(
                StatusCode::CONFLICT,
                "graph_generation_obsolete",
                "This graph generation was replaced. Rebuild the current input instead.",
            ));
        }
    }
    capacity(&mut tx, id).await?;
    db::audit(
        &mut tx,
        auth.user.id,
        id,
        "job.retry",
        job,
        "retry_requested",
    )
    .await?;
    let row = sqlx::query_as::<_,JobRow>("UPDATE jobs SET actor_id=$3,device_id=recollect_device(),state='queued',attempts=0,progress=0,error_code=NULL,not_before=now(),lease_token=NULL,lease_until=NULL,updated_at=now() WHERE id=$1 AND brain_id=$2 RETURNING *")
        .bind(job).bind(id).bind(auth.user.id).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(row.into()))
}

#[utoipa::path(post,path="/api/brains/{id}/jobs/{job}/cancel",params(("id"=Uuid,Path),("job"=Uuid,Path)),responses((status=200,body=Job)))]
pub async fn cancel(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, job)): Path<(Uuid, Uuid)>,
) -> Result<Json<Job>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let old =
        sqlx::query_as::<_, JobRow>("SELECT * FROM jobs WHERE id=$1 AND brain_id=$2 FOR UPDATE")
            .bind(job)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(Error::missing)?;
    if !matches!(old.state.as_str(), "queued" | "running") {
        tx.commit().await?;
        return Ok(Json(old.into()));
    }
    let row = sqlx::query_as::<_,JobRow>("UPDATE jobs SET state='cancelled',error_code='cancelled_by_admin',lease_token=NULL,lease_until=NULL,updated_at=now() WHERE id=$1 AND brain_id=$2 RETURNING *")
        .bind(job).bind(id).fetch_one(&mut *tx).await?;
    db::audit(
        &mut tx,
        auth.user.id,
        id,
        "job.cancel",
        job,
        "cancelled_by_admin",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(row.into()))
}
