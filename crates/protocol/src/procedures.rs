use crate::{ClaimEligibility, ClaimRevision, ScopeSelection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcedureObservation {
    pub result: String,
    pub observed_at: DateTime<Utc>,
    pub conditions: String,
    pub summary: String,
    pub support_ids: Vec<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcedureContent {
    pub conditions: String,
    pub steps: Vec<String>,
    pub expected_outcome: String,
    pub observations: Vec<ProcedureObservation>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HandoverContent {
    pub completed: Vec<String>,
    pub next_steps: Vec<String>,
    pub risks: Vec<String>,
    pub contributions: Vec<Uuid>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ClaimContribution {
    pub revision_id: Uuid,
    pub claim_id: Option<Uuid>,
    pub state: String,
    pub revision: Option<ClaimRevision>,
    pub eligibility: Option<ClaimEligibility>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HandoverInput {
    pub title: String,
    pub contributions: Vec<Uuid>,
    pub operation_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HandoverRun {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub title: String,
    pub contributions: Vec<Uuid>,
    pub selection: ScopeSelection,
    pub policy_id: Uuid,
    pub actor_id: Uuid,
    pub device_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub job_id: Uuid,
    pub state: String,
    pub error_code: Option<String>,
    pub request_id: Option<Uuid>,
    pub claim_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub automatic: bool,
    #[serde(default)]
    pub base_revision_id: Option<Uuid>,
    #[serde(default)]
    pub automatic_attempt: i32,
    #[serde(default)]
    pub retry_of: Option<Uuid>,
    pub finished_at: Option<DateTime<Utc>>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct HandoverPage {
    pub items: Vec<HandoverRun>,
    pub total: i64,
    pub offset: i64,
}
