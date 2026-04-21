use crate::{McpDefinitionManifest, ScopeSnapshot};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

fn default_timeout() -> u32 {
    300
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpPrivateRunner {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub device_id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub eligible: bool,
    pub available: bool,
    pub lease_until: Option<DateTime<Utc>>,
    pub revision: Uuid,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpPrivateRunnerInput {
    pub name: String,
    pub device_id: Uuid,
    pub enabled: bool,
    pub base_revision: Option<Uuid>,
}

#[derive(Clone, Debug, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct McpRunnerSelection {
    pub private_runner: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpCallInput {
    pub request_id: Uuid,
    pub profile_id: Uuid,
    pub connection_id: Uuid,
    pub tool_name: String,
    pub arguments: Value,
    pub environment_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub client_session_id: Uuid,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpCall {
    pub id: Uuid,
    pub request_id: Uuid,
    pub brain_id: Uuid,
    pub actor_id: Uuid,
    pub device_id: Option<Uuid>,
    pub profile_id: Uuid,
    pub connection_id: Uuid,
    pub tool_name: String,
    pub environment_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub scope: Option<ScopeSnapshot>,
    pub client_session_id: Uuid,
    pub runner_reference: String,
    pub instance_id: Option<Uuid>,
    pub state: String,
    pub code: Option<String>,
    pub cancel_requested: bool,
    pub timeout_seconds: u32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub dispatched_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub payload_expires_at: Option<DateTime<Utc>>,
    pub output_access: String,
    pub result: Option<Value>,
    pub reconciles_call_id: Option<Uuid>,
    pub resolutions: Vec<McpResolution>,
    pub observations: Vec<McpObservation>,
    pub capture_disposition: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpObservation {
    pub id: Uuid,
    pub call_id: Uuid,
    pub stage: String,
    pub outcome: String,
    pub state: String,
    pub code: Option<String>,
    pub captured_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub source_id: Option<Uuid>,
    pub source_version_id: Option<Uuid>,
    pub coverage: Vec<String>,
    pub attempts: i32,
    pub next_attempt_at: Option<DateTime<Utc>>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpObservationPage {
    pub items: Vec<McpObservation>,
    pub total: i64,
    pub offset: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpObservationStatus {
    pub pending: i64,
    pub errors: i64,
    pub published: i64,
    pub filtered: i64,
    pub skipped: i64,
    pub unknown: i64,
    pub last_publication: Option<DateTime<Utc>>,
    pub oldest_pending: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpReceiptCheck {
    pub call_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpReceiptRemovals {
    pub call_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpCallPage {
    pub calls: Vec<McpCall>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpReconcileInput {
    pub request_id: Uuid,
    pub client_session_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpResolveInput {
    pub request_id: Uuid,
    pub outcome: String,
    pub explanation: String,
    pub source_version_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpResolution {
    pub id: Uuid,
    pub kind: String,
    pub outcome: String,
    pub actor_id: Uuid,
    pub receipt_call_id: Option<Uuid>,
    pub source_version_id: Option<Uuid>,
    pub explanation: Option<String>,
    pub explanation_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRunnerEpoch {
    pub epoch: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpRunnerLease {
    pub runner_reference: String,
    pub epoch: Uuid,
    pub lease_until: DateTime<Utc>,
    pub cancel_calls: Vec<Uuid>,
    pub drain_instances: Vec<Uuid>,
    pub released_sessions: Vec<McpReleasedSession>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpReleasedSession {
    pub brain_id: Uuid,
    pub actor_id: Uuid,
    pub client_session_id: Uuid,
}

// This envelope crosses only the authenticated executor boundary. It is never
// included in a model-facing tool descriptor or ordinary call metadata response.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpExecutionPlan {
    pub call_id: Uuid,
    pub brain_id: Uuid,
    pub actor_id: Uuid,
    pub device_id: Option<Uuid>,
    pub profile_id: Uuid,
    pub connection_id: Uuid,
    pub client_session_id: Uuid,
    pub runner_reference: String,
    pub runner_epoch: Uuid,
    pub attempt_token: Uuid,
    pub lease_until: DateTime<Utc>,
    pub connection_revision: Uuid,
    pub definition_revision: DateTime<Utc>,
    pub definition: McpDefinitionManifest,
    pub target: String,
    pub configuration: Value,
    pub credential_alias: Option<String>,
    pub tool_name: String,
    pub arguments: Value,
    pub timeout_seconds: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpClaim {
    pub plan: Option<McpExecutionPlan>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpAttempt {
    pub epoch: Uuid,
    pub brain_id: Uuid,
    pub call_id: Uuid,
    pub attempt_token: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpStart {
    pub attempt: McpAttempt,
    pub instance_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpStartPermit {
    pub dispatch: bool,
    pub state: String,
    pub code: Option<String>,
    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpCompletion {
    pub attempt: McpAttempt,
    pub state: String,
    pub code: String,
    pub result: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpInstanceUpdate {
    pub attempt: McpAttempt,
    pub instance_id: Uuid,
    pub credential_generation: Uuid,
    pub state: String,
    pub active_calls: u32,
    pub idle_seconds: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpInstance {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub connection_id: Uuid,
    pub actor_id: Uuid,
    pub device_id: Option<Uuid>,
    pub client_session_id: Uuid,
    pub runner_reference: String,
    pub state: String,
    pub active_calls: i32,
    pub idle_seconds: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpRuntimeStatus {
    pub instances: Vec<McpInstance>,
    pub runners: Vec<McpRunnerStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpRunnerStatus {
    pub runner_reference: String,
    pub available: bool,
    pub lease_until: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpSessionRelease {
    pub client_session_id: Uuid,
}
