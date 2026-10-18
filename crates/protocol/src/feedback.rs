use crate::{GraphNode, GraphSelection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A delivery context is issued only for one canonically qualified input under
/// an owned retrieval operation. It contains no duplicate query/evidence body.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FeedbackContextRequest {
    pub scope: GraphSelection,
    pub query_context_id: Uuid,
    pub input_kind: String,
    pub revision_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UtilityVote {
    Helpful,
    Unhelpful,
    Reused,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FeedbackCommand {
    pub context_id: Uuid,
    pub base_change_id: Option<Uuid>,
    pub vote: UtilityVote,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FeedbackResult {
    pub context_id: Uuid,
    pub task_id: Uuid,
    pub query_context_id: Uuid,
    pub change_id: Option<Uuid>,
    pub vote: Option<UtilityVote>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FeedbackContext {
    pub feedback: FeedbackResult,
    pub node: GraphNode,
}
