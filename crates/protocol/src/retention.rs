use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RetentionPolicy {
    pub raw_session_days: i32,
    pub tool_output_days: i32,
    pub document_days: Option<i32>,
    pub support_excerpt_days: Option<i32>,
    pub allow_support_excerpts: bool,
    pub repository_days: Option<i32>,
    pub claim_days: Option<i32>,
    pub audit_days: i32,
    pub backup_days: i32,
}
impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            raw_session_days: 30,
            tool_output_days: 30,
            document_days: None,
            support_excerpt_days: None,
            allow_support_excerpts: true,
            repository_days: None,
            claim_days: None,
            audit_days: 365,
            backup_days: 7,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RetentionSettings {
    pub brain_id: Uuid,
    pub change_id: Uuid,
    pub policy: RetentionPolicy,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RetentionUpdate {
    pub base_change: Uuid,
    pub policy: RetentionPolicy,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExcerptInput {
    pub source_id: Uuid,
    pub version_id: Uuid,
    pub first_line: i32,
    pub last_line: i32,
    pub title: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ExcerptProvenance {
    pub source_id: Uuid,
    pub version_id: Uuid,
    pub first_line: i32,
    pub last_line: i32,
    pub captured_at: DateTime<Utc>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ErasureTarget {
    pub kind: String,
    pub id: Uuid,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ErasurePreview {
    pub brain_id: Uuid,
    pub target: ErasureTarget,
    pub eligibility_epoch: i64,
    pub source_versions: i64,
    pub claim_revisions: i64,
    pub snapshots: i64,
    pub manifest_revisions: i64,
    pub artifacts: i64,
    pub jobs: i64,
    #[serde(default)]
    pub model_input_fences: i64,
    #[serde(default)]
    pub model_claim_fences: i64,
    #[serde(default)]
    pub capture_event_fences: i64,
    pub independent_claim_revisions: i64,
    pub shared_sources: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ErasureInput {
    pub target: ErasureTarget,
    pub eligibility_epoch: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ErasureStatus {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub sequence: i64,
    pub target: ErasureTarget,
    pub cause: String,
    pub created_at: DateTime<Utc>,
    pub state: String,
    pub journaled: bool,
    pub pending_artifacts: i64,
    #[serde(default)]
    pub graph_pending: bool,
    pub error_code: Option<String>,
    pub local_copies: String,
    #[serde(default)]
    pub acknowledged_devices: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErasurePage {
    pub items: Vec<ErasureStatus>,
    pub total: i64,
    pub offset: i64,
    pub latest_sequence: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PublicationFence {
    pub repository_id: Uuid,
    pub revision: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PrivacyDeviceSync {
    pub brain_id: Uuid,
    pub policy: RetentionPolicy,
    pub sequence: i64,
    pub publication_fences: Vec<PublicationFence>,
    #[serde(default)]
    pub capture_event_fences: Vec<crate::CaptureFence>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PrivacyDeviceAck {
    pub sequence: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PrivacyDeviceReceipt {
    pub brain_id: Uuid,
    pub device_id: Uuid,
    pub sequence: i64,
    pub checked_at: DateTime<Utc>,
}
