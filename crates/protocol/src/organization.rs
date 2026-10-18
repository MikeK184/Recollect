//! Qualified semantic navigation, separate from accepted claims and graph paths.
use crate::{GraphNode, GraphSelection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OrganizationCenter {
    Topic { id: Uuid },
    Entity { id: Uuid },
}

/// A keyset in one bounded detail stream. Source windows advance only after
/// both card streams and all three detail phases have been exhausted.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum OrganizationRowCursor {
    Mentions {
        after_id: Option<Uuid>,
    },
    Memberships {
        after_topic: Option<Uuid>,
        after_entity: Option<Uuid>,
        after_input: Option<Uuid>,
    },
    Connections {
        after_id: Option<Uuid>,
    },
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationCursor {
    pub after_source: Option<Uuid>,
    pub card_offset: usize,
    pub row_cursor: OrganizationRowCursor,
    pub epoch: i64,
    pub scope: GraphSelection,
    pub center: Option<OrganizationCenter>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationRequest {
    pub scope: GraphSelection,
    pub center: Option<OrganizationCenter>,
    pub cursor: Option<OrganizationCursor>,
    #[serde(default = "card_limit")]
    pub limit: usize,
}
fn card_limit() -> usize {
    50
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationTopicKind {
    Facet,
    Community,
    Split,
    ManualOverlay,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationCard {
    pub id: Uuid,
    pub kind: String,
    /// Additive stored topic provenance. Older card envelopes may omit it.
    #[serde(default)]
    pub topic_kind: Option<OrganizationTopicKind>,
    pub label: String,
    pub source_count: usize,
    pub entity_count: usize,
    pub pinned: bool,
    pub change_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationMention {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub input_id: Uuid,
    pub source_key: String,
    pub byte_start: i32,
    pub byte_end: i32,
    pub quote: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationConnection {
    pub id: Uuid,
    pub from: Uuid,
    pub to: Uuid,
    pub relation: String,
    pub interpretation: String,
    pub source_key: String,
    pub byte_start: i32,
    pub byte_end: i32,
    pub quote: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationMembership {
    pub topic_id: Uuid,
    pub entity_id: Uuid,
    pub input_id: Uuid,
    pub source_key: String,
    pub reason: String,
    pub weight: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationSourceMembership {
    pub topic_id: Uuid,
    pub input_id: Uuid,
    pub source_key: String,
    pub reason: String,
    pub weight: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationRepresentativeSource {
    pub topic_id: Uuid,
    pub input_id: Uuid,
    pub source_key: String,
    pub provenance_kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationSourceCursor {
    pub brain_id: Uuid,
    pub after_source: Option<Uuid>,
    #[serde(default)]
    pub through_source: Option<Uuid>,
    pub after_topic: Option<Uuid>,
    pub after_input: Option<Uuid>,
    pub epoch: i64,
    pub scope: GraphSelection,
    pub topic_id: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationSourcesRequest {
    pub scope: GraphSelection,
    pub topic_id: Option<Uuid>,
    pub cursor: Option<OrganizationSourceCursor>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationSourcesView {
    pub brain_id: Uuid,
    pub scope: GraphSelection,
    pub epoch: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub state: String,
    pub topics: Vec<OrganizationCard>,
    pub evidence: Vec<GraphNode>,
    pub source_memberships: Vec<OrganizationSourceMembership>,
    pub representative_sources: Vec<OrganizationRepresentativeSource>,
    pub coverage: OrganizationCoverage,
    pub community_status: OrganizationCommunityStatus,
    pub next_cursor: Option<OrganizationSourceCursor>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct OrganizationCoverage {
    pub examined_sources: usize,
    pub qualified_sources: usize,
    pub mapped_sources: usize,
    pub unassigned_sources: usize,
    pub counts_are_lower_bounds: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommunityReadiness {
    Unconfigured,
    Blocked,
    Pending,
    Queued,
    Ready,
    Failed,
    Stale,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationCommunityStatus {
    pub state: CommunityReadiness,
    pub report_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_features: Option<VectorFeatureReadiness>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VectorFeatureReadiness {
    Ready,
    Partial,
    Unavailable,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProducerReadiness {
    Unconfigured,
    Blocked,
    Ready,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationView {
    pub brain_id: Uuid,
    pub scope: GraphSelection,
    pub epoch: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub state: String,
    pub topics: Vec<OrganizationCard>,
    pub entities: Vec<OrganizationCard>,
    /// Exact semantic endpoints for this page's detail rows, independent of
    /// the ranked overview-card page. These do not advance the card cursor.
    pub context_cards: Vec<OrganizationCard>,
    pub evidence: Vec<GraphNode>,
    pub mentions: Vec<OrganizationMention>,
    /// At most one exact visible mention for each overview card. These anchors
    /// do not consume or advance the independently paginated detail stream.
    pub representative_mentions: Vec<OrganizationMention>,
    pub memberships: Vec<OrganizationMembership>,
    #[serde(default)]
    pub source_memberships: Vec<OrganizationSourceMembership>,
    #[serde(default)]
    pub representative_sources: Vec<OrganizationRepresentativeSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_next_cursor: Option<OrganizationSourceCursor>,
    pub connections: Vec<OrganizationConnection>,
    pub coverage: OrganizationCoverage,
    /// Content-free readiness; entity facets do not imply qualified communities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_status: Option<OrganizationCommunityStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_status: Option<ProducerReadiness>,
    pub next_cursor: Option<OrganizationCursor>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum TopicOverride {
    RenameSource {
        label: String,
        basis_input: Uuid,
    },
    ExcludeSources {
        input_ids: Vec<Uuid>,
        excluded: bool,
    },
    SplitSources {
        input_ids: Vec<Uuid>,
    },
    Rename {
        label: String,
        basis_input: Uuid,
    },
    Pin {
        pinned: bool,
    },
    Exclude {
        input_ids: Vec<Uuid>,
        excluded: bool,
    },
    Split {
        input_ids: Vec<Uuid>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TopicCommand {
    pub base_change_id: Uuid,
    pub change: TopicOverride,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TopicCommandResult {
    pub id: Uuid,
    pub change_id: Uuid,
    pub split_topic_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AliasCursor {
    pub after_id: Uuid,
    pub epoch: i64,
    pub scope: GraphSelection,
    pub expires_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AliasRequest {
    pub scope: GraphSelection,
    pub cursor: Option<AliasCursor>,
    #[serde(default = "card_limit")]
    pub limit: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AliasCandidateView {
    pub id: Uuid,
    pub input_id: Uuid,
    pub from: OrganizationCard,
    pub to: OrganizationCard,
    pub source_key: String,
    pub byte_start: i32,
    pub byte_end: i32,
    pub quote: String,
    pub disposition: String,
    pub change_id: Option<Uuid>,
    pub acceptance_state: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AliasView {
    pub brain_id: Uuid,
    pub scope: GraphSelection,
    pub epoch: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub candidates: Vec<AliasCandidateView>,
    pub evidence: Vec<GraphNode>,
    pub next_cursor: Option<AliasCursor>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AliasAction {
    Accept,
    Reject,
    Revoke,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AliasCommand {
    pub base_change_id: Option<Uuid>,
    pub action: AliasAction,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AliasCommandResult {
    pub id: Uuid,
    pub change_id: Uuid,
    pub disposition: String,
}
