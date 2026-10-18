//! Shared, explicitly published agent work projection. Peer content is untrusted data.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CoordinationIntent {
    pub repository_id: Uuid,
    pub path: String,
    pub mode: String,
    pub phase: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CoordinationSessionInput {
    pub id: Uuid,
    pub task_id: Uuid,
    pub expected_scope: Uuid,
    pub expected_revision: Option<i64>,
    #[serde(default)]
    pub context_repository_ids: Vec<Uuid>,
    pub host_kind: String,
    pub label: String,
    pub goal: String,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub intents: Vec<CoordinationIntent>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CoordinationSession {
    pub id: Uuid,
    pub revision: i64,
    pub account_id: Uuid,
    pub account_name: String,
    pub device_id: Uuid,
    pub device_name: String,
    pub host_kind: String,
    pub label: String,
    pub goal: String,
    pub topics: Vec<String>,
    pub intents: Vec<CoordinationIntent>,
    pub environment_id: Option<Uuid>,
    #[serde(default)]
    pub environment_name: Option<String>,
    pub repository_ids: Vec<Uuid>,
    pub area_ids: Vec<Uuid>,
    pub status: String,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CoordinationOverlap {
    pub id: Uuid,
    pub peer: CoordinationSession,
    pub reason: String,
    pub paths: Vec<String>,
    pub tokens: Vec<String>,
    pub basis: Vec<String>,
    pub own_revision: i64,
    pub peer_revision: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CoordinationAnnouncement {
    pub session: CoordinationSession,
    pub overlaps: Vec<CoordinationOverlap>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CoordinationPeers {
    pub sessions: Vec<CoordinationSession>,
    pub overlaps: Vec<CoordinationOverlap>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CoordinationHeartbeat {
    pub expected_revision: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CoordinationMessageInput {
    pub id: Uuid,
    pub from_session_id: Uuid,
    pub to_session_id: Uuid,
    pub kind: String,
    pub text: String,
    pub reply_to: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CoordinationMessage {
    pub id: Uuid,
    pub from_session_id: Uuid,
    pub to_session_id: Uuid,
    pub kind: String,
    pub text: String,
    pub reply_to: Option<Uuid>,
    pub sender: CoordinationSession,
    pub recipient: CoordinationSession,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CoordinationPeerClosed {
    pub peer: CoordinationSession,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum CoordinationPayload {
    Overlap(CoordinationOverlap),
    Message(CoordinationMessage),
    PeerClosed(CoordinationPeerClosed),
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CoordinationEvent {
    pub sequence: i64,
    pub id: Uuid,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub payload: CoordinationPayload,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CoordinationInbox {
    pub events: Vec<CoordinationEvent>,
    pub next_cursor: i64,
    pub gap: bool,
    pub peers: Vec<CoordinationSession>,
}
