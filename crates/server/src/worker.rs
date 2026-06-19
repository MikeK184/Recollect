use crate::{AppState, db};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClaimedJob {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub actor_id: Uuid,
    pub target_id: Uuid,
    pub kind: String,
    pub lease_token: Uuid,
    pub device_id: Option<Uuid>,
}
#[derive(Debug, PartialEq)]
pub enum Failure {
    Database,
    Revoked,
    Unsupported,
    Missing,
    LostLease,
    Storage,
}
impl From<sqlx::Error> for Failure {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}
impl From<crate::error::Error> for Failure {
    fn from(error: crate::error::Error) -> Self {
        if error.0 == axum::http::StatusCode::UNAUTHORIZED {
            Self::Revoked
        } else {
            Self::Database
        }
    }
}

pub async fn claim(pool: &PgPool, lane: &str) -> Result<Option<ClaimedJob>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id,brain_id,actor_id,target_id,kind,lease_token,device_id FROM recollect_claim_job($1,$2)",
    )
    .bind(lane)
    .bind(Uuid::new_v4())
    .fetch_optional(pool)
    .await
}
pub async fn renew(pool: &PgPool, job: &ClaimedJob) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT recollect_renew_job($1,$2)")
        .bind(job.id)
        .bind(job.lease_token)
        .fetch_one(pool)
        .await
}

/// Keep publication polled while renewal waits for a job row held by that same
/// work. All outbound-I/O workers share this lease boundary.
pub fn with_lease<T>(
    pool: &PgPool,
    job: &ClaimedJob,
    work: impl std::future::Future<Output = T>,
) -> impl std::future::Future<Output = Result<T, Failure>> {
    // Allocate before constructing the heartbeat future. Otherwise nested
    // worker state machines copy the entire outbound job onto their stack.
    let mut work = Box::pin(work);
    async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.tick().await;
        loop {
            tokio::select! {
                result=&mut work=>return Ok(result),
                _=interval.tick()=>{
                    tokio::select! {
                        biased;
                        result=&mut work=>return Ok(result),
                        renewed=renew(pool,job)=>{
                            if !renewed? {return Err(Failure::LostLease);}
                        }
                    }
                }
            }
        }
    }
}
pub async fn fail(pool: &PgPool, job: &ClaimedJob, failure: Failure) -> Result<bool, sqlx::Error> {
    let (outcome, reason) = match failure {
        Failure::Database => ("queued", "database_error"),
        Failure::Revoked => ("cancelled", "permission_revoked"),
        Failure::Unsupported => ("failed", "unsupported_kind"),
        Failure::Missing => ("cancelled", "input_missing"),
        Failure::LostLease => return Ok(false),
        Failure::Storage => ("queued", "artifact_unavailable"),
    };
    sqlx::query_scalar("SELECT recollect_fail_job($1,$2,$3,$4)")
        .bind(job.id)
        .bind(job.lease_token)
        .bind(outcome)
        .bind(reason)
        .fetch_one(pool)
        .await
}

pub async fn execute(state: &AppState, job: &ClaimedJob) -> Result<(), Failure> {
    if job.kind == "source.learn" {
        return crate::learning::execute(state, job).await;
    }
    if job.kind == "handover.generate" {
        return crate::handovers::execute(state, job).await;
    }
    if job.kind == "semantic.generate" {
        return crate::semantic::execute(state, job).await;
    }
    if job.kind == "graph.project" {
        return crate::graph::execute(state, job).await;
    }
    if job.kind == "graph.analyze" {
        return crate::graph::analytics::execute(state, job).await;
    }
    let pool = &state.pool;
    let mut tx = db::device_tx(pool, job.actor_id, job.device_id).await?;
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM accounts WHERE id=$1 FOR SHARE")
            .bind(job.actor_id)
            .fetch_optional(&mut *tx)
            .await?;
    let role: Option<String> = sqlx::query_scalar("SELECT recollect_role($1)")
        .bind(job.brain_id)
        .fetch_one(&mut *tx)
        .await?;
    if enabled != Some(true) || !matches!(role.as_deref(), Some("writer" | "admin")) {
        return Err(Failure::Revoked);
    }
    // Materialization changes canonical graph eligibility and its Brain epoch.
    // Acquire the write lock before processing; two capture workers must not
    // both hold shared Brain locks and then attempt to upgrade in a trigger.
    db::lock_brain(
        &mut tx,
        job.brain_id,
        matches!(job.kind.as_str(), "source.process" | "repository.process"),
    )
    .await?;
    let valid: Option<bool> = sqlx::query_scalar("SELECT lease_until>clock_timestamp() FROM jobs WHERE id=$1 AND brain_id=$2 AND state='running' AND lease_token=$3 FOR UPDATE")
        .bind(job.id).bind(job.brain_id).bind(job.lease_token).fetch_optional(&mut *tx).await?;
    if valid != Some(true) {
        return Err(Failure::LostLease);
    }
    let (action, disposition) = match job.kind.as_str() {
        "brain.refresh" => {
            if job.target_id != job.brain_id {
                return Err(Failure::Missing);
            }
            // Read the current canonical state, never a stale payload supplied by an old job.
            sqlx::query("INSERT INTO brain_directory(brain_id,name,description,archived,source_change) SELECT id,name,description,archived,change_id FROM brains WHERE id=$1 ON CONFLICT(brain_id) DO UPDATE SET name=excluded.name,description=excluded.description,archived=excluded.archived,source_change=excluded.source_change,refreshed_at=now()")
        .bind(job.brain_id).execute(&mut *tx).await?;
            ("projection.refresh", "current")
        }
        "source.process" => (
            "source.process",
            crate::evidence::project(state, &mut tx, job.brain_id, job.target_id).await?,
        ),
        "repository.process" => (
            "repository.process",
            crate::publication::project(state, &mut tx, job).await?,
        ),
        _ => return Err(Failure::Unsupported),
    };
    if !crate::retention::job_available(&mut tx, job.brain_id, job.target_id, &job.kind).await? {
        return Err(Failure::Missing);
    }
    if job.kind == "source.process" {
        crate::learning::automatic(state, &mut tx, job.brain_id, job.target_id).await?;
    }
    // Check the lease again at publication. Losing it rolls back the projection too.
    let finished = sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,updated_at=now() WHERE id=$1 AND lease_token=$2 AND state='running' AND lease_until>clock_timestamp() AND recollect_role(brain_id) IN ('writer','admin')")
        .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
    if finished.rows_affected() != 1 {
        return Err(Failure::LostLease);
    }
    db::audit(
        &mut tx,
        job.actor_id,
        job.brain_id,
        action,
        job.target_id,
        disposition,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn run_once(state: &AppState, lane: &str) -> Result<bool, sqlx::Error> {
    let Some(job) = claim(&state.pool, lane).await? else {
        return Ok(false);
    };
    if let Err(failure) = execute(state, &job).await {
        tracing::warn!(job_id=%job.id,kind=?failure,"Job did not publish");
        fail(&state.pool, &job, failure).await?;
    }
    Ok(true)
}

pub async fn run(state: AppState) -> anyhow::Result<()> {
    let (stop, signal) = tokio::sync::watch::channel(false);
    let mut tasks = tokio::task::JoinSet::new();
    {
        let state = state.clone();
        let mut signal = signal.clone();
        tasks.spawn(async move {
            loop {
                if *signal.borrow() { break; }
                if crate::mcp::runtime::observations::run_once(&state).await.is_err() {
                    tracing::warn!("Managed observation publication will retry");
                }
                tokio::select! { _=signal.changed()=>{}, _=tokio::time::sleep(Duration::from_secs(1))=>{} }
            }
        });
    }
    {
        let state = state.clone();
        let mut signal = signal.clone();
        tasks.spawn(async move {
            loop {
                if *signal.borrow() { break; }
                if crate::autonomous::run_once(&state).await.is_err() {
                    tracing::warn!("Autonomous maintenance will retry");
                }
                if crate::semantic::run_once(&state).await.is_err() {
                    tracing::warn!("Semantic maintenance will retry");
                }
                if crate::graph::run_once(&state).await.is_err() {
                    tracing::warn!("Graph maintenance will retry");
                }
                if crate::graph::analytics::run_once(&state).await.is_err() {
                    tracing::warn!("Analytical cleanup will retry");
                }
                tokio::select! { _=signal.changed()=>{}, _=tokio::time::sleep(Duration::from_secs(10))=>{} }
            }
        });
    }
    {
        let state = state.clone();
        let mut signal = signal.clone();
        tasks.spawn(async move {
            loop {
                if *signal.borrow() {break;}
                if crate::privacy_journal::run_once(&state).await.is_err() {tracing::warn!("Privacy maintenance is pending; retrying");}
                tokio::select!{_=signal.changed()=>{},_=tokio::time::sleep(Duration::from_secs(1))=>{}}
            }
        });
    }
    for lane in [
        "interactive",
        "interactive",
        "capture",
        "capture",
        "model",
        "heavy",
    ] {
        let state = state.clone();
        let mut signal = signal.clone();
        tasks.spawn(async move {
            loop {
                if *signal.borrow() { break; }
                let worked = match run_once(&state,lane).await {
                    Ok(worked) => worked,
                    Err(_) => { tracing::warn!(lane,"Worker database call failed; retrying"); false }
                };
                if !worked {
                    tokio::select! { _=signal.changed()=>{}, _=tokio::time::sleep(Duration::from_millis(500))=>{} }
                }
            }
        });
    }
    tracing::info!("Durable worker started with bounded lanes");
    crate::shutdown::requested().await?;
    stop.send(true)?;
    while tasks.join_next().await.is_some() {}
    Ok(())
}
