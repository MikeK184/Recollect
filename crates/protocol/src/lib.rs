use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

mod workspace;
pub use workspace::*;
mod publication;
pub use publication::*;
mod memory;
pub use memory::*;
mod procedures;
pub use procedures::*;
mod retention;
pub use retention::*;
mod capture;
pub use capture::*;
mod retrieval;
pub use retrieval::*;
mod answers;
pub use answers::*;
mod semantic;
pub use semantic::*;
mod graph;
pub use graph::*;
mod analytics;
pub use analytics::*;
mod module_source;
pub use module_source::*;
mod mcp;
pub use mcp::*;
mod mcp_runtime;
pub use mcp_runtime::*;
mod operations;
pub use operations::*;
mod pipeline;
pub use pipeline::*;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub installation_owner: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SessionInfo {
    pub user: User,
    pub csrf_token: String,
    pub device_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Brain {
    #[serde(default)]
    pub icon_revision: Option<Uuid>,
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: String,
    pub archived: bool,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateBrain {
    pub name: String,
    #[serde(default)]
    pub managed_memory: bool,
    #[serde(default)]
    pub description: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateBrain {
    pub name: Option<String>,
    pub description: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuditEvent {
    pub id: Uuid,
    pub actor_id: Uuid,
    pub action: String,
    pub target_id: Uuid,
    pub disposition: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DependencyStatus {
    pub name: String,
    pub connected: bool,
    pub detail: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ServiceStatus {
    pub ready: bool,
    pub dependencies: Vec<DependencyStatus>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Job {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub target_id: Uuid,
    pub kind: String,
    pub lane: String,
    pub state: String,
    pub progress: i16,
    pub attempts: i32,
    pub max_attempts: i32,
    pub error_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProcessingStatus {
    pub state: String,
    pub refreshed_at: Option<DateTime<Utc>>,
    pub pending_jobs: i64,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    pub enabled: bool,
    pub installation_owner: bool,
    pub auth_kind: String,
    pub oidc_subject: Option<String>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct Invitation {
    pub id: Uuid,
    pub account_id: Uuid,
    pub username: String,
    pub expires_at: DateTime<Utc>,
    pub state: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct Team {
    pub accounts: Vec<Account>,
    pub invitations: Vec<Invitation>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct InviteRequest {
    pub username: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EnrollmentLink {
    pub id: Uuid,
    pub token: Uuid,
    pub username: String,
    pub expires_at: DateTime<Utc>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EnrollRequest {
    pub token: Uuid,
    pub password: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccountStatus {
    pub enabled: bool,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct OidcAccountRequest {
    pub username: String,
    pub subject: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuthOptions {
    pub oidc_configured: bool,
    pub membership_minutes: i32,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GrantRequest {
    pub role: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DirectGrantRequest {
    pub username: String,
    pub role: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GroupGrantRequest {
    pub group_name: String,
    pub role: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct OwnerRequest {
    pub account_id: Uuid,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EffectiveAccess {
    pub account: Account,
    pub owner: bool,
    pub direct_role: Option<String>,
    pub groups: Vec<String>,
    pub effective_role: Option<String>,
    pub membership_until: Option<DateTime<Utc>>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GroupGrant {
    pub id: Uuid,
    pub group_name: String,
    pub role: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct BrainAccess {
    pub members: Vec<EffectiveAccess>,
    pub group_grants: Vec<GroupGrant>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessChange {
    pub effective_role: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct Device {
    pub id: Uuid,
    pub name: String,
    pub claimed: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration: Option<String>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PairingRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct BrainAgent {
    pub device_id: Uuid,
    pub can_revoke: bool,
    pub name: String,
    pub host_kind: Option<String>,
    pub integration: String,
    pub claimed: bool,
    pub active: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub last_used_on_brain_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct BrainAgentGroup {
    pub user_name: String,
    pub agents: Vec<BrainAgent>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct BrainAgentRoster {
    pub groups: Vec<BrainAgentGroup>,
    pub hidden_count: i64,
}
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentBrainUsage {
    pub icon_revision: Option<Uuid>,
    pub brain_id: Uuid,
    pub name: String,
    pub last_used_at: DateTime<Utc>,
}
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountAgent {
    pub device_id: Uuid,
    pub name: String,
    pub host_kind: Option<String>,
    pub integration: String,
    pub claimed: bool,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub brains: Vec<AgentBrainUsage>,
}
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountAgentGroup {
    pub user_name: String,
    pub agents: Vec<AccountAgent>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccountAgentRoster {
    pub groups: Vec<AccountAgentGroup>,
    pub hidden_count: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PairingStart {
    pub device_code: Uuid,
    pub user_code: String,
    pub verification_url: String,
    pub expires_at: DateTime<Utc>,
    pub interval_seconds: u32,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PairingView {
    pub name: String,
    pub user_code: String,
    pub state: String,
    pub expires_at: DateTime<Utc>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PairingDecision {
    pub approve: bool,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PairingCode {
    pub device_code: Uuid,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PairingPoll {
    pub state: String,
    pub device: Option<Device>,
    pub token: Option<Uuid>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct EvidenceGroup {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub source_count: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateEvidenceGroup {
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateEvidenceGroup {
    pub name: String,
    #[serde(default)]
    pub description: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EvidencePolicy {
    pub allow_document_content: bool,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SourceInput {
    pub title: String,
    pub media_type: String,
    pub source_uri: Option<String>,
    pub observed_at: Option<DateTime<Utc>>,
    pub content: Option<String>,
    #[serde(default)]
    pub retain_content: bool,
    #[serde(default)]
    pub group_ids: Vec<Uuid>,
    pub base_version: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_class: Option<String>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SourceGroups {
    pub group_ids: Vec<Uuid>,
}
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceVersion {
    pub id: Uuid,
    pub source_id: Uuid,
    pub brain_id: Uuid,
    pub title: String,
    pub media_type: String,
    pub source_uri: Option<String>,
    pub observed_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub contributor: String,
    pub device_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    /// Server knowledge time; captured evidence can have an earlier created_at.
    pub recorded_at: DateTime<Utc>,
    pub retained: bool,
    pub byte_length: i32,
    pub processing: String,
    pub availability: String,
    #[serde(default = "document_class")]
    pub retention_class: String,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default = "active_privacy")]
    pub privacy_state: String,
    pub excerpt: Option<ExcerptProvenance>,
}
fn document_class() -> String {
    "document".into()
}
fn active_privacy() -> String {
    "active".into()
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SourceSummary {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub version: SourceVersion,
    pub version_count: i64,
    pub group_ids: Vec<Uuid>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EvidenceCatalogue {
    pub groups: Vec<EvidenceGroup>,
    pub sources: Vec<SourceSummary>,
    pub policy: EvidencePolicy,
    pub total: i64,
    pub offset: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SourceHistory {
    pub versions: Vec<SourceVersion>,
    pub total: i64,
    pub offset: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SourceSpan {
    pub id: Uuid,
    pub ordinal: i32,
    pub byte_start: i32,
    pub byte_end: i32,
    pub line_start: i32,
    pub line_end: i32,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SourceContent {
    pub version: SourceVersion,
    pub content: Option<String>,
    pub spans: Vec<SourceSpan>,
}
mod model;
pub use model::*;
