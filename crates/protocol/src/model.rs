use crate::ScopeSelection;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LiteralAcceptance {
    pub name: String,
    pub source_classes: Vec<String>,
    pub collection_ids: Vec<Uuid>,
    pub properties: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelPolicy {
    pub enabled: bool,
    pub provider: String,
    pub text_model: String,
    pub embedding_model: String,
    pub embedding_dimensions: i32,
    pub purposes: Vec<String>,
    pub content_classes: Vec<String>,
    pub max_input_bytes: i32,
    pub max_output_tokens: i32,
    pub daily_token_limit: i64,
    pub max_concurrent: i32,
    pub automatic_learning: bool,
    #[serde(default)]
    pub automatic_embedding: bool,
    #[serde(default)]
    pub autonomous_memory: bool,
    pub acceptance: Option<LiteralAcceptance>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelPolicyVersion {
    pub change_id: Uuid,
    pub brain_id: Uuid,
    pub policy: ModelPolicy,
    pub created_by: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InstalledModels {
    pub provider: String,
    pub endpoint: String,
    pub text_model: String,
    pub embedding_model: String,
    pub embedding_dimensions: i32,
    pub credentials_present: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelSettings {
    pub current: ModelPolicyVersion,
    pub installed: InstalledModels,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelPolicyUpdate {
    pub base_change: Uuid,
    pub policy: ModelPolicy,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelRequest {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub actor_id: Uuid,
    pub device_id: Option<Uuid>,
    pub operation_id: Uuid,
    pub policy_id: Uuid,
    pub purpose: String,
    pub model: String,
    pub returned_model: Option<String>,
    pub prompt_label: String,
    pub schema_label: String,
    pub state: String,
    pub error_code: Option<String>,
    pub reserved_tokens: i64,
    pub charged_tokens: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub dimensions: Option<i32>,
    pub suppressed: bool,
    #[serde(default)]
    pub detail_expired: bool,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelUsage {
    pub day: String,
    pub daily_limit: i64,
    pub charged_tokens: i64,
    pub remaining_tokens: i64,
    pub in_flight: i64,
    pub requests: Vec<ModelRequest>,
    pub total: i64,
    pub offset: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelCheckInput {
    pub operation_id: Uuid,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelCheckResult {
    pub requests: Vec<ModelRequest>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LearningInput {
    pub source_version_id: Uuid,
    pub selection: ScopeSelection,
    pub manifest_revision_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct LearningRun {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub source_version_id: Uuid,
    pub policy_id: Uuid,
    pub actor_id: Uuid,
    pub device_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub selection: ScopeSelection,
    pub manifest_revision_id: Option<Uuid>,
    pub automatic: bool,
    pub job_id: Uuid,
    pub state: String,
    pub error_code: Option<String>,
    pub request_id: Option<Uuid>,
    pub claim_ids: Vec<Uuid>,
    pub proposed: i32,
    pub accepted: i32,
    pub blocked: i32,
    pub conflicting: i32,
    pub reused: i32,
    #[serde(default)]
    pub revised: i32,
    #[serde(default)]
    pub retired: i32,
    #[serde(default)]
    pub reconciliation_inputs: Vec<Uuid>,
    #[serde(default)]
    pub automatic_attempt: i32,
    #[serde(default)]
    pub retry_of: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LearningRetryInput {
    pub operation_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct LearningPage {
    pub items: Vec<LearningRun>,
    pub total: i64,
    pub offset: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelDerivation {
    pub run_id: Uuid,
    pub request_id: Uuid,
    pub policy_id: Uuid,
    pub provider: String,
    pub requested_model: String,
    pub returned_model: String,
    pub prompt_label: String,
    pub schema_label: String,
}
