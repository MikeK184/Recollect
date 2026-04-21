use crate::{GraphGeneration, GraphNode, GraphSelection, Job, RecallCoverage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalyticsRequest {
    pub scope: GraphSelection,
    pub algorithm: String,
    pub direction: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsProvenance {
    pub generation: GraphGeneration,
    pub inputs: Vec<GraphGeneration>,
    pub memory_epoch: i64,
    pub coverage: RecallCoverage,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsReport {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub actor_id: Uuid,
    pub job_id: Uuid,
    pub algorithm: String,
    pub direction: String,
    pub state: String,
    pub selection: Option<GraphSelection>,
    pub provenance: Option<AnalyticsProvenance>,
    pub parameters: serde_json::Value,
    pub gds_version: Option<String>,
    pub analytics_epoch: i64,
    pub privacy_sequence: i64,
    pub node_count: usize,
    pub edge_count: usize,
    pub projected_edge_count: Option<usize>,
    pub estimated_bytes: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalyticsViewRequest {
    pub operation_id: Option<Uuid>,
    #[serde(default)]
    pub offset: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsResult {
    pub node: GraphNode,
    pub score: Option<f64>,
    pub group: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsView {
    pub report: AnalyticsReport,
    pub job: Job,
    pub cleanup_pending: bool,
    pub rows: Vec<AnalyticsResult>,
    pub total: usize,
    pub offset: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsStatus {
    pub reports: Vec<AnalyticsReport>,
    pub jobs: Vec<Job>,
    pub cleanup_pending: Vec<Uuid>,
    pub total: i64,
    pub offset: i64,
}
