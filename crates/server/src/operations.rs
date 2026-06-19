//! Fixed operational dimensions; request/source/credential payloads never enter metrics.
use crate::{
    AppState,
    auth::Auth,
    error::{Error, Result},
};
use axum::{
    Json,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use recollect_protocol::{HttpMetrics, LatencyBucket, OperationalStatus};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use uuid::Uuid;

const LATENCY_MS: [u64; 8] = [5, 25, 100, 500, 2_000, 10_000, 60_000, u64::MAX];

pub struct Metrics {
    instance: Uuid,
    started: Instant,
    requests: AtomicU64,
    in_flight: AtomicU64,
    interrupted: AtomicU64,
    statuses: [AtomicU64; 6],
    latency: [AtomicU64; 8],
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            instance: Uuid::new_v4(),
            started: Instant::now(),
            requests: AtomicU64::new(0),
            in_flight: AtomicU64::new(0),
            interrupted: AtomicU64::new(0),
            statuses: std::array::from_fn(|_| AtomicU64::new(0)),
            latency: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }
}
impl Metrics {
    pub fn snapshot(&self) -> HttpMetrics {
        HttpMetrics {
            instance_id: self.instance,
            uptime_seconds: self.started.elapsed().as_secs(),
            requests_started: self.requests.load(Ordering::Relaxed),
            in_flight: self.in_flight.load(Ordering::Relaxed),
            interrupted: self.interrupted.load(Ordering::Relaxed),
            responses_by_status_class: self
                .statuses
                .iter()
                .map(|v| v.load(Ordering::Relaxed))
                .collect(),
            response_header_latency_ms: LATENCY_MS
                .iter()
                .zip(&self.latency)
                .map(|(limit, count)| LatencyBucket {
                    upper_bound_ms: (*limit != u64::MAX).then_some(*limit),
                    count: count.load(Ordering::Relaxed),
                })
                .collect(),
        }
    }
}

struct RequestGuard(Arc<Metrics>, bool);
impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::Relaxed);
        if !self.1 {
            self.0.interrupted.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub async fn record(State(metrics): State<Arc<Metrics>>, request: Request, next: Next) -> Response {
    metrics.requests.fetch_add(1, Ordering::Relaxed);
    metrics.in_flight.fetch_add(1, Ordering::Relaxed);
    let mut guard = RequestGuard(metrics.clone(), false);
    let started = Instant::now();
    let response = next.run(request).await;
    let elapsed = started.elapsed();
    let status = response.status().as_u16();
    let class = match status / 100 {
        1..=5 => usize::from(status / 100),
        _ => 0,
    };
    metrics.statuses[class].fetch_add(1, Ordering::Relaxed);
    for (limit, count) in LATENCY_MS.iter().zip(&metrics.latency) {
        if elapsed <= Duration::from_millis(*limit) {
            count.fetch_add(1, Ordering::Relaxed);
        }
    }
    guard.1 = true;
    tracing::info!(
        status,
        elapsed_micros = elapsed.as_micros() as u64,
        "HTTP response headers sent"
    );
    response
}

#[utoipa::path(get,path="/api/operations",operation_id="operationalStatus",responses((status=200,body=OperationalStatus)))]
pub async fn status(State(state): State<AppState>, auth: Auth) -> Result<Json<OperationalStatus>> {
    if !auth.user.installation_owner {
        return Err(Error::forbidden());
    }
    let mut tx = auth.tx(&state.pool).await?;
    sqlx::query("SET LOCAL statement_timeout='5s'")
        .execute(&mut *tx)
        .await?;
    let (queued, running, failed): (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER(WHERE state='queued'),count(*) FILTER(WHERE state='running'),count(*) FILTER(WHERE state='failed') FROM jobs")
        .fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(OperationalStatus {
        http: state.metrics.snapshot(),
        database_pool_size: state.pool.size(),
        database_pool_idle: state.pool.num_idle(),
        queue_scope: "accessible_brains".into(),
        jobs_queued: queued,
        jobs_running: running,
        jobs_failed: failed,
    }))
}
