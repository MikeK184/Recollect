use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct LatencyBucket {
    /// Inclusive response-header latency bound; None is the final +infinity bucket.
    pub upper_bound_ms: Option<u64>,
    pub count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HttpMetrics {
    pub instance_id: Uuid,
    pub uptime_seconds: u64,
    pub requests_started: u64,
    pub in_flight: u64,
    pub interrupted: u64,
    /// Indices 1 through 5 represent HTTP 1xx through 5xx; 0 is other statuses.
    pub responses_by_status_class: Vec<u64>,
    pub response_header_latency_ms: Vec<LatencyBucket>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OperationalStatus {
    pub http: HttpMetrics,
    pub database_pool_size: u32,
    pub database_pool_idle: usize,
    /// Current caller's accessible Brains only; this is not a global queue census.
    pub queue_scope: String,
    pub jobs_queued: i64,
    pub jobs_running: i64,
    pub jobs_failed: i64,
}
