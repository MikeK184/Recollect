use crate::ScopeSelection;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FactValidity {
    pub kind: String,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub precision: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimSupport {
    pub kind: String,
    pub id: Uuid,
    pub line_from: Option<i32>,
    pub line_to: Option<i32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimContent {
    pub kind: String,
    pub subject: String,
    pub predicate: String,
    pub value: String,
    #[serde(default)]
    pub rationale: String,
    pub selection: ScopeSelection,
    pub manifest_revision_id: Option<Uuid>,
    pub validity: FactValidity,
    pub freshness: String,
    pub operational: String,
    pub observed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub observation: String,
    pub supports: Vec<ClaimSupport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedure: Option<crate::ProcedureContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handover: Option<crate::HandoverContent>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimInput {
    pub base_revision: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub content: ClaimContent,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ClaimRevision {
    pub id: Uuid,
    pub claim_id: Uuid,
    pub brain_id: Uuid,
    pub actor_id: Uuid,
    pub actor_name: String,
    pub device_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub origin: String,
    pub recorded_at: DateTime<Utc>,
    pub content: ClaimContent,
    pub review: String,
    pub reviewer_id: Option<Uuid>,
    pub acceptance_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivation: Option<crate::ModelDerivation>,
    #[serde(default = "active_lifecycle")]
    pub lifecycle: String,
    pub review_decision_id: Option<Uuid>,
    #[serde(default = "proposed_admission")]
    pub admission: String,
}
fn active_lifecycle() -> String {
    "active".into()
}
fn proposed_admission() -> String {
    "proposed".into()
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ClaimEligibility {
    pub investigation: bool,
    pub strict_accepted: bool,
    pub strict_operational: bool,
    pub effective_freshness: String,
    pub reasons: Vec<String>,
    #[serde(default)]
    pub rule_ids: Vec<Uuid>,
    #[serde(default)]
    pub conflicting_claim_ids: Vec<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ClaimEvidenceChoice {
    pub kind: String,
    pub id: Uuid,
    pub label: String,
    pub source_id: Option<Uuid>,
    pub repository_id: Option<Uuid>,
    pub snapshot_id: Option<Uuid>,
    pub revision: Option<String>,
    pub reference: Option<String>,
    pub observed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub availability: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ClaimEvidenceDetail {
    pub evidence: ClaimEvidenceChoice,
    pub data: serde_json::Value,
    pub text: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ClaimEvidencePage {
    pub items: Vec<ClaimEvidenceChoice>,
    pub total: i64,
    pub offset: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ClaimView {
    pub revision: ClaimRevision,
    pub knowledge_until: Option<DateTime<Utc>>,
    pub eligibility: ClaimEligibility,
    pub evidence: Vec<ClaimEvidenceChoice>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributions: Vec<crate::ClaimContribution>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ClaimPage {
    pub items: Vec<ClaimView>,
    /// Earliest canonical retention deadline for content in this bounded page.
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    pub total_candidates: i64,
    pub offset: i64,
    pub next_offset: Option<i64>,
    pub knowledge_at: DateTime<Utc>,
    #[serde(default)]
    pub unavailable: Vec<UnavailableClaim>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ClaimDetail {
    pub selected: Option<ClaimView>,
    pub current_revision: Uuid,
    pub history: Vec<ClaimRevision>,
    pub total: i64,
    pub offset: i64,
    pub knowledge_at: DateTime<Utc>,
    pub selection_state: String,
    pub unavailable_history: Vec<UnavailableRevision>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UnavailableRevision {
    pub id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub state: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UnavailableClaim {
    pub claim_id: Uuid,
    pub revision: UnavailableRevision,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewInput {
    pub base_revision: Uuid,
    pub action: String,
    pub reason: String,
    pub content: Option<ClaimContent>,
    pub revalidation_basis: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConflictParticipant {
    pub claim_id: Uuid,
    pub base_revision: Uuid,
    pub content: Option<ClaimContent>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConflictResolution {
    pub disposition: String,
    pub participants: Vec<ConflictParticipant>,
    pub selected_id: Option<Uuid>,
    pub replacement: Option<ClaimContent>,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ReviewTransition {
    pub claim_id: Uuid,
    pub before_revision: Option<Uuid>,
    pub after_revision: Uuid,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ReviewDecision {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub actor_id: Uuid,
    pub actor_name: String,
    pub action: String,
    pub reason: String,
    pub revalidation_basis: Option<String>,
    pub transitions: Vec<ReviewTransition>,
    pub recorded_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AssertionRule {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub claim_id: Uuid,
    pub revision_id: Uuid,
    pub decision_id: Uuid,
    pub kind: String,
    pub content: ClaimContent,
    pub created_at: DateTime<Utc>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ReviewOutcome {
    pub decision: ReviewDecision,
    pub claims: Vec<ClaimView>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ReviewHistory {
    pub decisions: Vec<ReviewDecision>,
    pub rules: Vec<AssertionRule>,
    pub exempted_rule_ids: Vec<Uuid>,
    pub conflicts: Vec<ClaimRevision>,
    pub total: i64,
    pub offset: i64,
}
