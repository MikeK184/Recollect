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
    /// The Brain's selected text model. An empty value means none is selected,
    /// which is valid only while `enabled` is false.
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
    /// The operator's explicit text-model override; absent when the
    /// installation supplies none and each Brain chooses its own.
    #[serde(default)]
    pub text_model: Option<String>,
    pub embedding_model: String,
    pub embedding_dimensions: i32,
    pub credentials_present: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelSettings {
    pub current: ModelPolicyVersion,
    pub installed: InstalledModels,
    #[serde(default)]
    pub providers: Vec<InstalledModels>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelPolicyUpdate {
    pub base_change: Uuid,
    pub policy: ModelPolicy,
    #[serde(default)]
    pub rebuild_embeddings: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CatalogueModel {
    pub id: String,
    pub kind: String,
    pub available: Option<bool>,
    pub selectable: bool,
    pub max_dimensions: Option<i32>,
    pub input_usd_per_million: Option<f64>,
    pub cached_input_usd_per_million: Option<f64>,
    pub output_usd_per_million: Option<f64>,
    pub pricing_tier: String,
    pub checked_on: String,
    pub source_url: String,
    pub pricing_stale: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelCatalogue {
    pub models: Vec<CatalogueModel>,
    pub observed_at: Option<DateTime<Utc>>,
    pub stale: bool,
    pub error_code: Option<String>,
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
    #[serde(default = "legacy_request_provider")]
    pub provider: String,
    pub cost_usd: Option<f64>,
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
fn legacy_request_provider() -> String {
    "openai".into()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_models_read_a_receipt_with_or_without_a_text_model() {
        let installed = |text_model: Option<&str>| {
            let mut value = serde_json::json!({
                "provider":"openai", "endpoint":"https://api.openai.com/v1",
                "embedding_model":"text-embedding-3-large",
                "embedding_dimensions":3072, "credentials_present":false
            });
            if let Some(model) = text_model {
                value["text_model"] = model.into();
            }
            serde_json::from_value::<InstalledModels>(value).unwrap()
        };
        // Receipts saved before the model became optional name their model.
        assert_eq!(
            installed(Some("gpt-5.6-luna")).text_model.as_deref(),
            Some("gpt-5.6-luna")
        );
        assert_eq!(installed(None).text_model, None);
    }

    #[test]
    fn pre_provider_request_receipt_remains_openai_with_unknown_cost() {
        let legacy = serde_json::json!({
            "id":Uuid::nil(), "brain_id":Uuid::nil(), "actor_id":Uuid::nil(),
            "operation_id":Uuid::nil(), "policy_id":Uuid::nil(),
            "purpose":"answering", "model":"gpt-5.6-luna",
            "prompt_label":"brain-answer-1", "schema_label":"brain-answer-1",
            "state":"succeeded", "reserved_tokens":100, "charged_tokens":42,
            "suppressed":false, "created_at":"2026-10-07T00:00:00Z"
        });
        let receipt: ModelRequest = serde_json::from_value(legacy).unwrap();
        assert_eq!(receipt.provider, "openai");
        assert_eq!(receipt.cost_usd, None);
        assert_eq!(receipt.charged_tokens, 42);
    }
}
