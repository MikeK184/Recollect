use super::*;
use crate::worker::{self, Failure};
use std::time::Duration;

async fn ready<'a>(
    state: &'a AppState,
    job: &ClaimedJob,
) -> Result<(Tx<'a>, AnalyticsReport, Auth)> {
    let mut tx = db::device_tx(&state.pool, job.actor_id, job.device_id).await?;
    db::require_writer(&mut tx, job.brain_id).await?;
    lease(&mut tx, job).await?;
    let r = report(&mut tx, job.brain_id, job.target_id).await?;
    if r.job_id != job.id || r.state != "running" {
        return Err(changed());
    }
    let auth = job_auth(&mut tx, job).await?;
    Ok((tx, r, auth))
}
async fn progress(state: &AppState, job: &ClaimedJob, entry: &journal::Entry) -> Result<()> {
    let live: bool = sqlx::query_scalar("SELECT recollect_analytics_attempt_live($1,$2)")
        .bind(job.id)
        .bind(job.lease_token)
        .fetch_one(&state.pool)
        .await?;
    if !live {
        return Err(failure(
            "analytics_lease_lost",
            "This analytical job was cancelled, revoked or lost its inputs.",
        ));
    }
    let rows = native::control(
        &state.config,
        &state.http,
        "CALL gds.listProgress() YIELD jobId,progress WHERE jobId=$job RETURN progress",
        json!({"job":entry.id}),
        &["progress"],
    )
    .await?;
    let percent = rows
        .iter()
        .filter_map(|r| r[0].as_str())
        .filter_map(|s| s.strip_suffix('%'))
        .filter_map(|s| s.parse::<i16>().ok())
        .filter(|n| (0..=100).contains(n))
        .min()
        .map_or(15, |n| 20 + n * 60 / 100);
    let mut tx = db::device_tx(&state.pool, job.actor_id, job.device_id).await?;
    db::require_writer(&mut tx, job.brain_id).await?;
    lease(&mut tx, job).await?;
    sqlx::query("UPDATE jobs SET progress=greatest(progress,$3),updated_at=clock_timestamp() WHERE id=$1 AND lease_token=$2")
      .bind(job.id).bind(job.lease_token).bind(percent).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn run(state: &AppState, job: &ClaimedJob, armed: &mut Option<journal::Entry>) -> Result<()> {
    // A failed predecessor cannot consume native memory behind a new lease.
    journal::reconcile(&state.config, &state.pool)
        .await
        .map_err(|_| {
            failure(
                "analytics_cleanup_pending",
                "Previous analytical scratch cleanup must finish before allocation.",
            )
        })?;
    let (mut tx, r, auth) = ready(state, job).await?;
    let operation = r.selection.as_ref().and_then(|s| s.operation_id);
    current(state, &mut tx, &auth, &r, operation).await?;
    let d = descriptor(&mut tx, job.brain_id, r.id).await?;
    let e = journal::arm(state, &mut tx, job).await?;
    *armed = Some(e.clone());
    tx.commit().await?;
    let mut calculation = Box::pin(native::calculate(state, &r, &d, &e));
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.tick().await;
    let result = loop {
        tokio::select! {
            result=&mut calculation=>break result?,
            _=interval.tick()=>{
                // Keep native work polled while the status call takes locks.
                tokio::select! {
                    biased;
                    result=&mut calculation=>break result?,
                    checked=progress(state,job,&e)=>checked?,
                }
            }
        }
    };
    // Success includes exact native eviction, then a fresh canonical read.
    journal::close(&state.config, &state.pool, &state.http, &e).await?;
    *armed = None;
    let (mut tx, r, auth) = ready(state, job).await?;
    let graph = current(state, &mut tx, &auth, &r, operation).await?;
    sqlx::query("UPDATE analytics_reports SET state='ready',scores=$3,gds_version=$4,estimated_bytes=$5,projected_edge_count=$6,published_at=clock_timestamp(),error_code=NULL WHERE brain_id=$1 AND id=$2")
      .bind(job.brain_id).bind(r.id).bind(Json(&result.scores)).bind(result.version).bind(result.estimate).bind(result.projected_edges as i32).execute(&mut *tx).await?;
    let lease_until: DateTime<Utc> = sqlx::query_scalar("SELECT lease_until FROM jobs WHERE id=$1")
        .bind(job.id)
        .fetch_one(&mut *tx)
        .await?;
    let changed_job=sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,error_code=NULL,updated_at=clock_timestamp() WHERE id=$1 AND state='running' AND lease_token=$2 AND lease_until>clock_timestamp()")
      .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
    if changed_job.rows_affected() != 1 {
        return Err(failure(
            "analytics_lease_lost",
            "This analytical worker lost its publication lease.",
        ));
    }
    db::audit(
        &mut tx,
        job.actor_id,
        job.brain_id,
        "graph.analysis.publish",
        r.id,
        "ready",
    )
    .await?;
    // Publication triggers/audit can cross a clock boundary under these locks.
    if analytics_epoch(&mut tx, job.brain_id).await? != r.analytics_epoch {
        return Err(changed());
    }
    deadline(
        &mut tx,
        [graph.deadline, r.expires_at, Some(lease_until)]
            .into_iter()
            .flatten()
            .min(),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
pub async fn execute(state: &AppState, job: &ClaimedJob) -> std::result::Result<(), Failure> {
    let mut armed = None;
    let work = async {
        tokio::time::timeout(Duration::from_secs(120), run(state, job, &mut armed))
            .await
            .map_err(|_| {
                failure(
                    "analytics_attempt_timeout",
                    "This analysis exceeded its 120-second attempt limit.",
                )
            })?
    };
    let result = worker::with_lease(&state.pool, job, work).await;
    // Dropping an HTTP future does not terminate native work. Retain ownership
    // and cleanup on every exit, including a lost heartbeat or outer timeout.
    let cleanup = if let Some(e) = armed {
        journal::close(&state.config, &state.pool, &state.http, &e).await
    } else {
        Ok(())
    };
    let error = match result {
        Ok(Ok(())) => return cleanup.map_err(Into::into),
        Ok(Err(e)) => e,
        Err(e) => {
            if cleanup.is_err() {
                tracing::warn!(job_id=%job.id,"Analytical cleanup will retry from its journal");
            }
            return Err(e);
        }
    };
    let error = cleanup.err().unwrap_or(error);
    if error.1 == "analytics_lease_lost" {
        return Err(Failure::LostLease);
    }
    let outcome = if matches!(
        error.0,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
    ) || error.1 == "analytics_inputs_changed"
    {
        "cancelled"
    } else if error.0 == StatusCode::UNPROCESSABLE_ENTITY
        || matches!(
            error.1,
            "graph_query_invalid"
                | "graph_configuration_invalid"
                | "analytics_result_invalid"
                | "analytics_projection_closed"
        )
    {
        "failed"
    } else {
        "queued"
    };
    sqlx::query("SELECT recollect_fail_analytics_job($1,$2,$3,$4)")
        .bind(job.id)
        .bind(job.lease_token)
        .bind(outcome)
        .bind(error.1)
        .execute(&state.pool)
        .await?;
    Ok(())
}
