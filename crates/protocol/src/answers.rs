use crate::{RecallItem, RecallRequest, RecallResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AnswerRequest {
    pub request_id: Uuid,
    pub question: String,
    #[serde(default)]
    pub recall: RecallRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AnswerStatement {
    pub text: String,
    pub citation_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AnswerText {
    pub summary: String,
    pub statements: Vec<AnswerStatement>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AnswerCitation {
    pub id: String,
    pub evidence: RecallItem,
}

/// Only the initial POST may contain ephemeral payload. Status/replay returns
/// the same shape with answer, citations and recall absent or empty.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AnswerResponse {
    pub request_id: Uuid,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<AnswerText>,
    pub citations: Vec<AnswerCitation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recall: Option<RecallResponse>,
    pub model_request_id: Option<Uuid>,
    pub failure_code: Option<String>,
    pub memory_epoch: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,
    pub provider_may_have_run: bool,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}
