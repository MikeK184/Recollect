use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SemanticProfile {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub provider: String,
    pub model: String,
    pub dimensions: i32,
    pub representation: String,
    pub created_by: Uuid,
    pub policy_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SemanticBatch {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub profile_id: Uuid,
    pub actor_id: Uuid,
    pub policy_id: Uuid,
    pub job_id: Uuid,
    pub request_id: Option<Uuid>,
    pub state: String,
    pub input_count: i32,
    pub error_code: Option<String>,
    pub retry_of: Option<Uuid>,
    pub automatic_attempt: i32,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub can_retry: bool,
}

#[derive(Default, Serialize, Deserialize, ToSchema)]
pub struct SemanticCounts {
    pub pending: i64,
    pub queued: i64,
    pub running: i64,
    pub ready: i64,
    pub blocked: i64,
    pub failed: i64,
    pub removed: i64,
    pub truncated: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SemanticStatus {
    pub enabled: bool,
    pub state: String,
    pub profile: Option<SemanticProfile>,
    pub counts: SemanticCounts,
    pub coverage: Vec<String>,
    pub batches: Vec<SemanticBatch>,
    pub total_batches: i64,
    pub offset: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticReindex {
    pub base_profile: Option<Uuid>,
}
