use crate::{Job, RecallCoverage, RecallItem, ScopeSelection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphRebuild {
    pub kind: String,
    pub snapshot_id: Option<Uuid>,
    pub manifest_revision_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphGeneration {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub kind: String,
    pub snapshot_id: Option<Uuid>,
    #[serde(default)]
    pub input_snapshot_ids: Vec<Uuid>,
    pub input_epoch: i64,
    pub adapter: String,
    pub actor_id: Uuid,
    pub job_id: Uuid,
    pub state: String,
    pub node_count: i64,
    pub edge_count: i64,
    pub unresolved: i64,
    pub ambiguous: i64,
    pub unsupported: i64,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphStatus {
    pub generations: Vec<GraphGeneration>,
    pub jobs: Vec<Job>,
    pub total: i64,
    pub offset: i64,
    pub memory_epoch: i64,
    pub link_epoch: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(default, deny_unknown_fields)]
pub struct GraphSelection {
    pub kind: String,
    pub snapshot_id: Option<Uuid>,
    pub selection: ScopeSelection,
    pub operation_id: Option<Uuid>,
    pub collection_id: Option<Uuid>,
    pub manifest_revision_id: Option<Uuid>,
    pub fact_at: Option<DateTime<Utc>>,
    pub mode: String,
    pub relations: Vec<String>,
}
impl Default for GraphSelection {
    fn default() -> Self {
        Self {
            kind: "repository".into(),
            snapshot_id: None,
            selection: ScopeSelection::default(),
            operation_id: None,
            collection_id: None,
            manifest_revision_id: None,
            fact_at: None,
            mode: "investigation".into(),
            relations: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphViewRequest {
    pub scope: GraphSelection,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphNode {
    pub key: String,
    pub evidence: RecallItem,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct GraphEdge {
    pub id: Uuid,
    pub from: String,
    pub to: String,
    pub family: String,
    pub relation: String,
    pub evidence_kind: String,
    pub evidence_id: Uuid,
    pub evidence_ordinal: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphView {
    pub brain_id: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    pub scope: GraphSelection,
    pub scope_id: Option<Uuid>,
    pub generation: GraphGeneration,
    pub inputs: Vec<GraphGeneration>,
    pub link_issues: Vec<GraphLinkIssue>,
    pub link_issues_total: usize,
    pub state: String,
    pub memory_epoch: i64,
    pub nodes: Vec<GraphNode>,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub offset: usize,
    pub relations: Vec<String>,
    pub coverage: RecallCoverage,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphLinkIssue {
    pub source: String,
    pub source_snapshot_id: Uuid,
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphPathRequest {
    pub scope: GraphSelection,
    pub start: String,
    pub end: String,
    pub direction: String,
    pub max_hops: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphPath {
    pub view: GraphView,
    pub status: String,
    pub direction: String,
    pub max_hops: usize,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphExploreRequest {
    pub scope: GraphSelection,
    pub center: Option<String>,
    pub direction: String,
    pub max_hops: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphReach {
    pub key: String,
    pub nodes: Vec<String>,
    pub edges: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphExploration {
    pub view: GraphView,
    pub center: Option<String>,
    pub direction: String,
    pub max_hops: usize,
    pub expires_at: Option<DateTime<Utc>>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub reach: Vec<GraphReach>,
}
