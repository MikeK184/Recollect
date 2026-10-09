use super::*;
use crate::{
    db,
    worker::{self, ClaimedJob, Failure},
};

async fn lease(tx: &mut Tx<'_>, job: &ClaimedJob) -> Result<()> {
    let valid:Option<bool>=sqlx::query_scalar("SELECT lease_until>clock_timestamp() FROM jobs WHERE id=$1 AND brain_id=$2 AND state='running' AND lease_token=$3 FOR UPDATE")
        .bind(job.id).bind(job.brain_id).bind(job.lease_token).fetch_optional(&mut **tx).await?;
    if valid != Some(true) {
        return Err(failure(
            "graph_lease_lost",
            "This graph worker no longer owns publication.",
        ));
    }
    Ok(())
}
async fn ready<'a>(state: &'a AppState, job: &ClaimedJob) -> Result<(Tx<'a>, GraphGeneration)> {
    let mut tx = db::device_tx(&state.pool, job.actor_id, job.device_id).await?;
    db::require_writer(&mut tx, job.brain_id).await?;
    lease(&mut tx, job).await?;
    let g = generation(&mut tx, job.brain_id, job.target_id).await?;
    if g.job_id != job.id || !matches!(g.state.as_str(), "queued" | "running") {
        return Err(failure(
            "graph_generation_changed",
            "This generation no longer accepts publication.",
        ));
    }
    descriptor::available(&mut tx, &g).await?;
    Ok((tx, g))
}
async fn prepare<'a>(state: &'a AppState, job: &ClaimedJob) -> Result<(Tx<'a>, GraphGeneration)> {
    let mut tx = db::preparation_tx(&state.pool, job.actor_id, job.device_id).await?;
    let role = db::require_role(&mut tx, job.brain_id, false).await?;
    if !matches!(role.as_str(), "writer" | "admin") {
        return Err(Error::forbidden());
    }
    let archived: bool = sqlx::query_scalar("SELECT archived FROM brains WHERE id=$1")
        .bind(job.brain_id)
        .fetch_one(&mut *tx)
        .await?;
    if archived {
        return Err(failure(
            "graph_generation_changed",
            "This Brain is archived.",
        ));
    }
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM jobs WHERE id=$1 AND brain_id=$2 AND state='running' AND lease_token=$3 AND lease_until>clock_timestamp())",
    )
    .bind(job.id)
    .bind(job.brain_id)
    .bind(job.lease_token)
    .fetch_one(&mut *tx)
    .await?;
    if !active {
        return Err(failure(
            "graph_lease_lost",
            "This graph worker no longer owns preparation.",
        ));
    }
    let g = generation(&mut tx, job.brain_id, job.target_id).await?;
    if g.job_id != job.id || !matches!(g.state.as_str(), "queued" | "running") {
        return Err(failure(
            "graph_generation_changed",
            "This generation no longer accepts preparation.",
        ));
    }
    descriptor::available(&mut tx, &g).await?;
    Ok((tx, g))
}
async fn progress(state: &AppState, job: &ClaimedJob, percent: i32) -> Result<()> {
    let mut tx = db::device_tx(&state.pool, job.actor_id, job.device_id).await?;
    db::require_writer(&mut tx, job.brain_id).await?;
    lease(&mut tx, job).await?;
    sqlx::query(
        "UPDATE jobs SET progress=$3,updated_at=clock_timestamp() WHERE id=$1 AND lease_token=$2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(percent)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
async fn run(state: &AppState, job: &ClaimedJob) -> Result<()> {
    let (mut tx, g) = prepare(state, job).await?;
    let saved: Option<Json<Descriptor>> =
        sqlx::query_scalar("SELECT descriptor FROM graph_generations WHERE brain_id=$1 AND id=$2")
            .bind(job.brain_id)
            .bind(g.id)
            .fetch_one(&mut *tx)
            .await?;
    let current = preparation_epoch(&mut tx, job.brain_id).await?;
    let d = match saved {
        Some(d) if d.preparation_epoch == Some(current) => d.0,
        _ => descriptor::build(&mut tx, &g).await?,
    };
    descriptor::retained(&mut tx, &d).await?;
    tx.commit().await?;
    let (mut tx, g) = ready(state, job).await?;
    preparation_unchanged(&mut tx, job.brain_id, current).await?;
    descriptor::retained(&mut tx, &d).await?;
    sqlx::query("UPDATE graph_generations SET descriptor=$3,node_count=$4,edge_count=$5,unresolved=$6,ambiguous=$7,unsupported=$8,state='running',error_code=NULL,cleaned_at=NULL WHERE brain_id=$1 AND id=$2")
        .bind(job.brain_id).bind(g.id).bind(Json(&d)).bind(d.nodes.len() as i64).bind(d.edges.len() as i64)
        .bind(d.unresolved).bind(d.ambiguous).bind(d.unsupported).execute(&mut *tx).await?;
    tx.commit().await?;
    adapter::setup(state, &g).await?;
    let batches = 2 * (d.nodes.len().div_ceil(500) + d.edges.len().div_ceil(500));
    let mut done = 0;
    for batch in d.nodes.chunks(500) {
        adapter::nodes(state, &g, batch).await?;
        done += 1;
        progress(state, job, 5 + (90 * done / batches) as i32).await?;
    }
    for batch in d.edges.chunks(500) {
        adapter::edges(state, &g, batch).await?;
        done += 1;
        progress(state, job, 5 + (90 * done / batches) as i32).await?;
    }
    for batch in d.nodes.chunks(500) {
        adapter::verify_nodes(state, &g, batch).await?;
        done += 1;
        progress(state, job, 5 + (90 * done / batches) as i32).await?;
    }
    for batch in d.edges.chunks(500) {
        adapter::verify_edges(state, &g, batch).await?;
        done += 1;
        progress(state, job, 5 + (90 * done / batches) as i32).await?;
    }
    adapter::verify_counts(state, &g, &d).await?;
    let (mut tx, g) = ready(state, job).await?;
    preparation_unchanged(&mut tx, job.brain_id, current).await?;
    // Memory epoch and snapshot retention are current under the same Brain lock
    // that publishes the generation; import success alone cannot establish it.
    sqlx::query("UPDATE graph_generations SET state='superseded' WHERE brain_id=$1 AND kind=$2 AND snapshot_id IS NOT DISTINCT FROM $3 AND state IN ('ready','failed','cancelled') AND id<>$4 AND input_snapshot_ids=$5")
        .bind(job.brain_id).bind(&g.kind).bind(g.snapshot_id).bind(g.id).bind(&g.input_snapshot_ids).execute(&mut *tx).await?;
    sqlx::query("UPDATE graph_generations SET state='ready',published_at=clock_timestamp(),error_code=NULL WHERE brain_id=$1 AND id=$2")
        .bind(job.brain_id).bind(g.id).execute(&mut *tx).await?;
    let lease_until: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT lease_until FROM jobs WHERE id=$1")
            .bind(job.id)
            .fetch_one(&mut *tx)
            .await?;
    let done=sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE id=$1 AND lease_token=$2 AND state='running' AND lease_until>clock_timestamp()")
        .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
    if done.rows_affected() != 1 {
        return Err(failure(
            "graph_lease_lost",
            "This graph worker lost its publication lease.",
        ));
    }
    db::audit(
        &mut tx,
        job.actor_id,
        job.brain_id,
        "graph.publish",
        g.id,
        "ready",
    )
    .await?;
    // Triggers and publication writes can cross a wall-clock deadline while
    // holding the canonical locks. Check after them, not just before I/O.
    descriptor::available(&mut tx, &g).await?;
    descriptor::retained(&mut tx, &d).await?;
    let owns: bool = sqlx::query_scalar("SELECT $1>clock_timestamp()")
        .bind(lease_until)
        .fetch_one(&mut *tx)
        .await?;
    if !owns {
        return Err(failure(
            "graph_lease_lost",
            "This graph worker lost its publication lease.",
        ));
    }
    tx.commit().await?;
    Ok(())
}
pub async fn execute(state: &AppState, job: &ClaimedJob) -> std::result::Result<(), Failure> {
    let work = async {
        tokio::time::timeout(std::time::Duration::from_secs(120), run(state, job))
            .await
            .map_err(|_| {
                failure(
                    "graph_attempt_timeout",
                    "This graph import attempt exceeded its time limit.",
                )
            })?
    };
    let result = worker::with_lease(&state.pool, job, work).await?;
    if let Err(error) = result {
        if error.1 == "graph_lease_lost" {
            return Err(Failure::LostLease);
        }
        let (outcome, reason) = if matches!(
            error.0,
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED | StatusCode::NOT_FOUND
        ) {
            ("cancelled", "permission_revoked")
        } else if matches!(
            error.1,
            "graph_input_changed"
                | "graph_input_unavailable"
                | "graph_input_expired"
                | "graph_input_too_large"
                | "graph_generation_changed"
        ) {
            ("cancelled", error.1)
        } else if matches!(
            error.1,
            "graph_configuration_invalid" | "graph_query_invalid"
        ) {
            ("failed", error.1)
        } else {
            (
                "queued",
                if error.1.starts_with("graph_") {
                    error.1
                } else {
                    "database_error"
                },
            )
        };
        // Reuse the durable queue's bounded backoff and lease fencing, retaining
        // only a static error code. Backend payloads are never stored or logged.
        sqlx::query("SELECT recollect_fail_job($1,$2,$3,$4)")
            .bind(job.id)
            .bind(job.lease_token)
            .bind(outcome)
            .bind(reason)
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}
