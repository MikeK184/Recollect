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

/// Claims last this long; `recollect_claim_job` and `recollect_renew_job` grant
/// 20 seconds from the moment they run.
const LEASE: Duration = Duration::from_secs(20);
const HEARTBEAT: Duration = Duration::from_secs(5);

/// Keep publication polled while renewal waits for a job row held by that same
/// work. All outbound-I/O workers share this lease boundary.
pub fn with_lease<T>(
    pool: &PgPool,
    job: &ClaimedJob,
    work: impl std::future::Future<Output = T>,
) -> impl std::future::Future<Output = Result<T, Failure>> {
    // Allocate before constructing the heartbeat future. Otherwise nested
    // worker state machines copy the entire outbound job onto their stack.
    let work = Box::pin(work);
    async move { hold_lease(work, || renew(pool, job), LEASE, HEARTBEAT).await }
}

/// Run `work` while renewing its lease every `heartbeat`. The lease is lost
/// only when renewal reports it (`false`) or when no renewal has succeeded for
/// a whole `lease`; a renewal that merely errors (pool exhaustion, statement
/// timeout) is retried at the next heartbeat so a busy database does not
/// discard in-flight work.
async fn hold_lease<T, E, R>(
    mut work: impl std::future::Future<Output = T> + Unpin,
    mut renew: impl FnMut() -> R,
    lease: Duration,
    heartbeat: Duration,
) -> Result<T, Failure>
where
    R: std::future::Future<Output = Result<bool, E>>,
    E: std::fmt::Display,
{
    let mut interval = tokio::time::interval(heartbeat);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval.tick().await;
    // Lower bound of the database lease: each grant starts after we asked for it.
    let mut expires = tokio::time::Instant::now() + lease;
    loop {
        tokio::select! {
            result=&mut work=>return Ok(result),
            _=interval.tick()=>{
                let asked = tokio::time::Instant::now();
                tokio::select! {
                    biased;
                    result=&mut work=>return Ok(result),
                    renewed=tokio::time::timeout_at(expires,renew())=>match renewed {
                        Ok(Ok(true))=>expires=asked+lease,
                        Ok(Ok(false))|Err(_)=>return Err(Failure::LostLease),
                        Ok(Err(error))=>{
                            tracing::warn!(%error,"Lease renewal failed; retrying");
                            if tokio::time::Instant::now()>=expires {return Err(Failure::LostLease);}
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
    if job.kind == "claim.support" {
        return crate::memory_support_audit::execute(state, job).await;
    }
    if job.kind == "source.learn" {
        return crate::learning::execute(state, job).await;
    }
    if job.kind == "knowledge.map" {
        return crate::knowledge_mapping::production::execute(state, job).await;
    }
    if job.kind == "knowledge.fields" {
        return crate::knowledge_mapping::fields::execute(state, job).await;
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
    // Projection runs under the exclusive Brain lock and can outlast one lease.
    // Heartbeat while it runs; losing the lease drops the work and its transaction.
    with_lease(&state.pool, job, project(state, job)).await?
}

async fn project(state: &AppState, job: &ClaimedJob) -> Result<(), Failure> {
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
    // No row lock: it would block the heartbeat for the whole projection. The
    // publication UPDATE below re-checks token and lease atomically.
    let valid: Option<bool> = sqlx::query_scalar("SELECT lease_until>clock_timestamp() FROM jobs WHERE id=$1 AND brain_id=$2 AND state='running' AND lease_token=$3")
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
    if job.kind == "source.learn"
        && crate::support_excerpts::reconcile_artifacts(state, Some(job.target_id))
            .await
            .is_err()
    {
        tracing::warn!(job_id=%job.id,"Pending excerpt artifacts await maintenance");
    }
    Ok(true)
}

/// Time one Brain may spend in a maintenance pass before the pass moves on, so a
/// stuck Brain cannot starve the rest.
pub const BRAIN_BUDGET: Duration = Duration::from_secs(30);
/// Ceiling for a whole subsystem pass across every Brain.
const PASS_BUDGET: Duration = Duration::from_secs(120);
const MAINTENANCE_EVERY: Duration = Duration::from_secs(10);
/// Brains remembered per subsystem before the order is forgotten and rebuilt.
const ATTEMPT_MEMORY: usize = 4096;

/// Order in which this process last started each Brain, per subsystem. It is
/// only a fairness hint for scheduling: nothing durable depends on it, a restart
/// falls back to the database's own order, and it is cleared when it grows.
#[derive(Default)]
struct Attempts {
    next: u64,
    last: std::collections::HashMap<Uuid, u64>,
}
impl Attempts {
    /// Never-attempted Brains first, then least recently attempted. Stable, so
    /// ties keep the database's order.
    fn order(&self, brains: &mut [(Uuid, Uuid)]) {
        brains.sort_by_key(|(brain, _)| self.last.get(brain).copied());
    }
    fn record(&mut self, brain: Uuid) {
        if self.last.len() >= ATTEMPT_MEMORY && !self.last.contains_key(&brain) {
            self.last.clear();
        }
        self.next += 1;
        self.last.insert(brain, self.next);
    }
}
static ATTEMPTS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<&'static str, Attempts>>,
> = std::sync::LazyLock::new(Default::default);
fn attempts<T>(subsystem: &'static str, f: impl FnOnce(&mut Attempts) -> T) -> T {
    let mut all = ATTEMPTS.lock().unwrap_or_else(|e| e.into_inner());
    f(all.entry(subsystem).or_default())
}

/// Run one maintenance subsystem's work for every Brain in turn, so one Brain
/// that cannot make progress never starves the rest
/// ([autonomous maintenance contract](../../../docs/contracts/memory-autonomous-maintenance.md):
/// archived or unavailable Brains stay visible "while other work proceeds").
///
/// * Each Brain gets [`BRAIN_BUDGET`]; on expiry its work future is dropped.
///   That is safe: dropping an sqlx 0.8 `Transaction` queues a ROLLBACK, the pool
///   settles the connection (`ping`) before it is reused, and PostgreSQL then
///   releases that transaction's locks. Nothing is half-committed, because every
///   unit of work either commits as one transaction or leaves no rows. (Native
///   graph I/O already in flight is idempotent and lease/attempt-fenced.)
/// * No new Brain starts once `PASS_BUDGET - BRAIN_BUDGET` has elapsed, so the
///   caller's whole-pass limit is not what cancels the last Brain mid-work.
///   Skipped Brains are logged and keep their place for the next pass.
/// * Brains run in least-recently-attempted order. The attempt is recorded
///   before the work starts so a pass cancelled mid-work still counts, and a
///   Brain that timed out is recorded again after the pass so it goes to the
///   back instead of leading every pass.
///
/// Returns the sum of the units of work reported by Brains that completed.
pub(crate) async fn each_brain<F, Fut>(
    subsystem: &'static str,
    mut brains: Vec<(Uuid, Uuid)>,
    mut work: F,
) -> usize
where
    F: FnMut(Uuid, Uuid) -> Fut,
    Fut: std::future::Future<Output = crate::error::Result<usize>>,
{
    attempts(subsystem, |a| a.order(&mut brains));
    let pass = tokio::time::Instant::now();
    let mut count = 0;
    let mut timed_out = vec![];
    for (position, &(brain, actor)) in brains.iter().enumerate() {
        if pass.elapsed() >= PASS_BUDGET - BRAIN_BUDGET {
            for &(skipped, _) in &brains[position..] {
                tracing::info!(subsystem, brain_id=%skipped, outcome="skipped", "Maintenance pass budget reached before this Brain");
            }
            break;
        }
        attempts(subsystem, |a| a.record(brain));
        let started = tokio::time::Instant::now();
        let outcome = tokio::time::timeout(BRAIN_BUDGET, work(brain, actor)).await;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        match outcome {
            Ok(Ok(done)) => {
                count += done;
                tracing::debug!(subsystem, brain_id=%brain, outcome="ok", elapsed_ms, "Maintenance completed");
            }
            Ok(Err(error)) => {
                tracing::warn!(subsystem, brain_id=%brain, outcome="error", code=error.1, elapsed_ms, "Maintenance is waiting");
            }
            Err(_) => {
                timed_out.push(brain);
                tracing::warn!(subsystem, brain_id=%brain, outcome="timeout", code="brain_budget", elapsed_ms, "Maintenance exceeded its Brain budget");
            }
        }
    }
    attempts(subsystem, |a| {
        timed_out.into_iter().for_each(|b| a.record(b))
    });
    count
}

/// Delay between polls of one lane: starts at `base`, doubles while claims
/// return nothing up to `cap`, and resets as soon as a job is claimed.
#[derive(Debug, Clone, Copy)]
struct Backoff {
    base: Duration,
    cap: Duration,
    next: Duration,
}
impl Backoff {
    fn new(base: Duration, cap: Duration) -> Self {
        Self {
            base,
            cap,
            next: base,
        }
    }
    fn idle(&mut self) -> Duration {
        let wait = self.next;
        self.next = (self.next * 2).min(self.cap);
        wait
    }
    fn reset(&mut self) {
        self.next = self.base;
    }
}

/// Remove expired receipts and long-finished jobs; claiming no longer does.
pub async fn cleanup(pool: &PgPool) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar("SELECT recollect_cleanup_expired()")
        .fetch_one(pool)
        .await
}

/// Run one maintenance subsystem on its own task and interval. `pass` reports
/// whether it failed; a pass that overruns `PASS_BUDGET` is cancelled and retried.
fn periodic<F, Fut>(
    tasks: &mut tokio::task::JoinSet<()>,
    state: &AppState,
    mut signal: tokio::sync::watch::Receiver<bool>,
    name: &'static str,
    every: Duration,
    pass: F,
) where
    F: Fn(AppState) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = bool> + Send + 'static,
{
    let state = state.clone();
    tasks.spawn(async move {
        loop {
            if *signal.borrow() {
                break;
            }
            match tokio::time::timeout(PASS_BUDGET, pass(state.clone())).await {
                Ok(false) => {}
                Ok(true) => tracing::warn!(name, "Maintenance pass failed; it will retry"),
                Err(_) => {
                    tracing::warn!(name, "Maintenance pass exceeded its budget; it will retry")
                }
            }
            tokio::select! { _=signal.changed()=>{}, _=tokio::time::sleep(every)=>{} }
        }
    });
}

pub async fn run(state: AppState) -> anyhow::Result<()> {
    let (stop, signal) = tokio::sync::watch::channel(false);
    let mut tasks = tokio::task::JoinSet::new();
    let second = Duration::from_secs(1);
    periodic(
        &mut tasks,
        &state,
        signal.clone(),
        "observations",
        second,
        |state| async move {
            crate::mcp::runtime::observations::run_once(&state)
                .await
                .is_err()
        },
    );
    periodic(
        &mut tasks,
        &state,
        signal.clone(),
        "privacy",
        second,
        |state| async move { crate::privacy_journal::run_once(&state).await.is_err() },
    );
    periodic(
        &mut tasks,
        &state,
        signal.clone(),
        "autonomous",
        MAINTENANCE_EVERY,
        |state| async move { crate::autonomous::run_once(&state).await.is_err() },
    );
    periodic(
        &mut tasks,
        &state,
        signal.clone(),
        "semantic",
        MAINTENANCE_EVERY,
        |state| async move { crate::semantic::run_once(&state).await.is_err() },
    );
    periodic(
        &mut tasks,
        &state,
        signal.clone(),
        "retained mapping",
        MAINTENANCE_EVERY,
        |state| async move {
            crate::knowledge_mapping::producer::run_once(&state)
                .await
                .is_err()
        },
    );
    periodic(
        &mut tasks,
        &state,
        signal.clone(),
        "graph",
        MAINTENANCE_EVERY,
        |state| async move { crate::graph::run_once(&state).await.is_err() },
    );
    periodic(
        &mut tasks,
        &state,
        signal.clone(),
        "canonical fields",
        MAINTENANCE_EVERY,
        |state| async move {
            crate::knowledge_mapping::fields::run_once(&state)
                .await
                .is_err()
        },
    );
    periodic(
        &mut tasks,
        &state,
        signal.clone(),
        "analytics",
        MAINTENANCE_EVERY,
        |state| async move { crate::graph::analytics::run_once(&state).await.is_err() },
    );
    periodic(
        &mut tasks,
        &state,
        signal.clone(),
        "cleanup",
        MAINTENANCE_EVERY,
        |state| async move { cleanup(&state.pool).await.is_err() },
    );
    for (lane, cap) in [
        ("interactive", 2),
        ("interactive", 2),
        ("capture", 2),
        ("capture", 2),
        ("model", 5),
        ("heavy", 5),
    ] {
        let state = state.clone();
        let mut signal = signal.clone();
        tasks.spawn(async move {
            let mut backoff = Backoff::new(Duration::from_millis(500), Duration::from_secs(cap));
            loop {
                if *signal.borrow() {
                    break;
                }
                let worked = match run_once(&state, lane).await {
                    Ok(worked) => worked,
                    Err(_) => {
                        tracing::warn!(lane, "Worker database call failed; retrying");
                        false
                    }
                };
                if worked {
                    backoff.reset();
                } else {
                    let wait = backoff.idle();
                    tokio::select! { _=signal.changed()=>{}, _=tokio::time::sleep(wait)=>{} }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const LEASE: Duration = Duration::from_millis(120);
    const BEAT: Duration = Duration::from_millis(20);

    async fn long_work() -> &'static str {
        tokio::time::sleep(Duration::from_millis(500)).await;
        "done"
    }

    #[tokio::test]
    async fn work_longer_than_the_lease_survives_while_renewals_succeed() {
        let renewals = AtomicUsize::new(0);
        let result = hold_lease(
            Box::pin(long_work()),
            || {
                renewals.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Ok::<_, String>(true))
            },
            LEASE,
            BEAT,
        )
        .await;
        assert_eq!(result, Ok("done"));
        assert!(renewals.load(Ordering::SeqCst) >= 10);
    }

    #[tokio::test]
    async fn a_refused_renewal_loses_the_lease() {
        let result = hold_lease(
            Box::pin(long_work()),
            || std::future::ready(Ok::<_, String>(false)),
            LEASE,
            BEAT,
        )
        .await;
        assert_eq!(result, Err(Failure::LostLease));
    }

    #[tokio::test]
    async fn transient_renewal_errors_keep_the_work_running() {
        // Fail the first few renewals (a 100 ms outage inside a 120 ms lease), then recover.
        let calls = AtomicUsize::new(0);
        let result = hold_lease(
            Box::pin(long_work()),
            || {
                let call = calls.fetch_add(1, Ordering::SeqCst);
                std::future::ready(if call < 4 {
                    Err("pool timed out".to_string())
                } else {
                    Ok(true)
                })
            },
            LEASE,
            BEAT,
        )
        .await;
        assert_eq!(result, Ok("done"));
    }

    #[tokio::test]
    async fn persistent_renewal_errors_end_at_lease_expiry() {
        let started = tokio::time::Instant::now();
        let result = hold_lease(
            Box::pin(long_work()),
            || std::future::ready(Err::<bool, _>("pool timed out".to_string())),
            LEASE,
            BEAT,
        )
        .await;
        assert_eq!(result, Err(Failure::LostLease));
        assert!(started.elapsed() >= LEASE);
        assert!(started.elapsed() < Duration::from_millis(400));
    }

    #[tokio::test]
    async fn a_renewal_that_hangs_cannot_outlive_the_lease() {
        let result = hold_lease(
            Box::pin(long_work()),
            std::future::pending::<Result<bool, String>>,
            LEASE,
            BEAT,
        )
        .await;
        assert_eq!(result, Err(Failure::LostLease));
    }

    fn brains(count: usize) -> Vec<(Uuid, Uuid)> {
        (0..count)
            .map(|_| (Uuid::new_v4(), Uuid::new_v4()))
            .collect()
    }
    type Log = std::sync::Arc<std::sync::Mutex<Vec<Uuid>>>;
    /// Work that records its Brain, then behaves as `behave` says.
    async fn visit(
        subsystem: &'static str,
        brains: &[(Uuid, Uuid)],
        log: &Log,
        behave: impl Fn(Uuid) -> Option<Duration> + Copy,
        failing: Option<Uuid>,
    ) -> usize {
        each_brain(subsystem, brains.to_vec(), |brain, _| {
            let log = log.clone();
            async move {
                log.lock().unwrap().push(brain);
                match behave(brain) {
                    Some(wait) => tokio::time::sleep(wait).await,
                    None => std::future::pending::<()>().await,
                }
                if failing == Some(brain) {
                    return Err(crate::error::Error::missing());
                }
                Ok(1)
            }
        })
        .await
    }
    fn took(log: &Log) -> Vec<Uuid> {
        std::mem::take(&mut *log.lock().unwrap())
    }

    #[tokio::test(start_paused = true)]
    async fn a_stuck_brain_times_out_and_the_others_still_run() {
        let brains = brains(3);
        let (stuck, log) = (brains[0].0, Log::default());
        let started = tokio::time::Instant::now();
        let done = visit(
            "test-stuck",
            &brains,
            &log,
            |b| (b != stuck).then_some(Duration::ZERO),
            None,
        )
        .await;
        assert_eq!(done, 2, "Only the two healthy Brains complete work");
        assert_eq!(took(&log), brains.iter().map(|b| b.0).collect::<Vec<_>>());
        assert_eq!(
            started.elapsed(),
            BRAIN_BUDGET,
            "The stuck Brain costs exactly its budget"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_error_in_one_brain_does_not_stop_the_rest() {
        let brains = brains(3);
        let log = Log::default();
        let done = visit(
            "test-error",
            &brains,
            &log,
            |_| Some(Duration::ZERO),
            Some(brains[1].0),
        )
        .await;
        assert_eq!(done, 2);
        assert_eq!(took(&log).len(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn a_brain_that_timed_out_is_ordered_last_on_the_next_pass() {
        let brains = brains(3);
        let (stuck, log) = (brains[0].0, Log::default());
        visit(
            "test-order",
            &brains,
            &log,
            |b| (b != stuck).then_some(Duration::ZERO),
            None,
        )
        .await;
        took(&log);
        // Healthy now: the previously stuck Brain no longer leads the pass.
        let done = visit("test-order", &brains, &log, |_| Some(Duration::ZERO), None).await;
        assert_eq!(done, 3);
        assert_eq!(
            took(&log),
            [brains[1].0, brains[2].0, brains[0].0],
            "A Brain that exhausted its budget goes to the back, not the front"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn the_pass_stops_starting_brains_at_its_deadline_and_resumes_with_the_skipped() {
        let ids: Vec<Uuid> = {
            let list = brains(6);
            list.iter().map(|b| b.0).collect()
        };
        let brains: Vec<_> = ids.iter().map(|&b| (b, Uuid::nil())).collect();
        let log = Log::default();
        let slow = |_| Some(Duration::from_secs(25));
        let started = tokio::time::Instant::now();
        // Starts at 0, 25, 50 and 75 s. At 100 s the 90 s limit
        // (PASS_BUDGET - BRAIN_BUDGET) has passed, so the last two never start.
        let done = visit("test-deadline", &brains, &log, slow, None).await;
        assert_eq!(done, 4);
        assert_eq!(took(&log), ids[..4]);
        assert_eq!(started.elapsed(), Duration::from_secs(100));
        // The skipped Brains were never attempted, so they lead the next pass.
        let done = visit("test-deadline", &brains, &log, slow, None).await;
        assert_eq!(done, 4);
        assert_eq!(took(&log), [ids[4], ids[5], ids[0], ids[1]]);
    }

    #[test]
    fn attempt_memory_is_bounded_and_unattempted_brains_lead() {
        let mut attempts = Attempts::default();
        let known = Uuid::new_v4();
        attempts.record(known);
        let fresh = Uuid::new_v4();
        let mut order = [(known, Uuid::nil()), (fresh, Uuid::nil())];
        attempts.order(&mut order);
        assert_eq!(order[0].0, fresh);
        for _ in 0..ATTEMPT_MEMORY {
            attempts.record(Uuid::new_v4());
        }
        assert!(
            attempts.last.len() <= ATTEMPT_MEMORY,
            "The attempt map must stay bounded"
        );
    }

    #[test]
    fn idle_lanes_back_off_and_reset_on_a_claim() {
        let mut backoff = Backoff::new(Duration::from_millis(500), Duration::from_secs(5));
        let waits: Vec<_> = (0..6).map(|_| backoff.idle()).collect();
        assert_eq!(
            waits,
            [500, 1000, 2000, 4000, 5000, 5000].map(Duration::from_millis)
        );
        backoff.reset();
        assert_eq!(backoff.idle(), Duration::from_millis(500));
    }
}
