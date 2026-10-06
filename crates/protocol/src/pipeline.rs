use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A bounded, read-only projection of canonical processing records.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PipelineFeed {
    pub brain_id: Uuid,
    pub observed_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub items: Vec<PipelineItem>,
    pub has_more: bool,
    pub graph: Option<PipelineGraph>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PipelineJob {
    pub id: Uuid,
    pub state: String,
    pub updated_at: DateTime<Utc>,
    pub lease_until: Option<DateTime<Utc>>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PipelineLearning {
    pub id: Uuid,
    pub state: String,
    pub job: PipelineJob,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub accepted: i32,
    pub proposed: i32,
    pub blocked: i32,
    pub conflicting: i32,
    pub reused: i32,
    pub revised: i32,
    pub retired: i32,
    pub claim_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PipelineItem {
    pub id: Uuid,
    pub capture_id: Option<Uuid>,
    pub binding_id: Option<Uuid>,
    pub source_id: Option<Uuid>,
    pub source_version_id: Option<Uuid>,
    pub actor_id: Uuid,
    pub contributor: String,
    pub device_id: Option<Uuid>,
    pub agent_name: Option<String>,
    pub host: Option<String>,
    pub host_session_id: Option<String>,
    pub agent_id: Option<String>,
    pub coverage: Vec<String>,
    pub kind: String,
    pub tool_name: Option<String>,
    pub title: String,
    pub received_at: DateTime<Utc>,
    pub activity_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub processing: String,
    pub processing_job: Option<PipelineJob>,
    pub learning: Option<PipelineLearning>,
    pub active: bool,
}

/// Graph work is Brain-wide; this deliberately has no source/run relationship.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PipelineGraph {
    pub id: Uuid,
    pub state: String,
    pub input_epoch: i64,
    pub current_epoch: i64,
    pub published_at: Option<DateTime<Utc>>,
    pub node_count: i64,
    pub edge_count: i64,
}
