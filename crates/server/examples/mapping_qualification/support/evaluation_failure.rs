//! Exact failed-experiment cleanup shared by the guarded tool and native proof.
use anyhow::{Context, Result, ensure};
use recollect_server::{AppState, auth::Auth};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;
pub async fn cancel_failed(
    state: &AppState,
    admin: &PgPool,
    auth: &Auth,
    brain: Uuid,
    report: Uuid,
    job: Uuid,
) -> Result<Value> {
    auth.require_browser()
        .map_err(|e| anyhow::anyhow!("Browser evaluator required ({})", e.1))?;
    // An analytics error can safely requeue its job. Experiments are one-shot:
    // close its exact native attempt and cancel that exact owned job so a later
    // tuple cannot consume the predecessor. Never retry or drain another job.
    let snapshot:Option<(String,Uuid,Uuid,Uuid,String)>=sqlx::query_as("SELECT j.state,j.brain_id,j.actor_id,j.target_id,j.kind FROM jobs j JOIN analytics_reports r ON r.brain_id=j.brain_id AND r.job_id=j.id WHERE j.id=$1 AND r.id=$2 AND r.purpose='organization_shadow'").bind(job).bind(report).fetch_optional(admin).await?;
    let (before, actual_brain, actor, target, kind) =
        snapshot.context("Exact failed shadow job missing")?;
    ensure!(
        actual_brain == brain
            && actor == auth.user.id
            && target == report
            && kind == "graph.analyze",
        "Failed job ownership changed; no cancellation admitted"
    );
    recollect_server::graph::analytics::reconcile(&state.config, &state.pool).await?;
    let pending:i64=sqlx::query_scalar("SELECT count(*) FROM analytics_attempts WHERE brain_id=$1 AND report_id=$2 AND cleaned_at IS NULL").bind(brain).bind(report).fetch_one(admin).await?;
    ensure!(
        pending == 0,
        "Exact failed shadow attempt still owns native scratch; stop before another tuple"
    );
    if matches!(before.as_str(), "queued" | "running") {
        let _ = recollect_server::jobs::cancel(
            axum::extract::State(state.clone()),
            auth.clone(),
            axum::extract::Path((brain, job)),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Owned job cancellation failed ({})", e.1))?;
    }
    let after: String = sqlx::query_scalar("SELECT state FROM jobs WHERE brain_id=$1 AND id=$2")
        .bind(brain)
        .bind(job)
        .fetch_one(admin)
        .await?;
    ensure!(
        !matches!(after.as_str(), "queued" | "running"),
        "Failed predecessor remains claimable"
    );
    Ok(
        json!({"brain_id":brain,"report_id":report,"job_id":job,"actor_id":actor,"kind":kind,"before":before,"after":after,"pending_native_attempts":pending,"retries":0}),
    )
}
