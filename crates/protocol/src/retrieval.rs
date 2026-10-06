use crate::{FactValidity, ScopeSelection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecallReference {
    pub kind: String,
    pub id: Uuid,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(default, deny_unknown_fields)]
pub struct RecallRequest {
    pub query: String,
    pub strategy: String,
    pub exact: Option<RecallReference>,
    pub operation_id: Option<Uuid>,
    pub selection: ScopeSelection,
    pub collection_id: Option<Uuid>,
    pub manifest_revision_id: Option<Uuid>,
    pub knowledge_at: Option<DateTime<Utc>>,
    pub fact_at: Option<DateTime<Utc>>,
    pub mode: String,
    pub channels: Vec<String>,
    pub semantic_request_id: Option<Uuid>,
    pub semantic_min_similarity: Option<f32>,
    pub graph: Option<RecallGraphOptions>,
    pub source_diversity: bool,
    pub limit: usize,
    pub context_bytes: usize,
}
impl Default for RecallRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            strategy: "manual".into(),
            exact: None,
            operation_id: None,
            selection: ScopeSelection::default(),
            collection_id: None,
            manifest_revision_id: None,
            knowledge_at: None,
            fact_at: None,
            mode: "investigation".into(),
            channels: vec!["exact".into(), "lexical".into()],
            semantic_request_id: None,
            semantic_min_similarity: None,
            graph: None,
            source_diversity: true,
            limit: 10,
            context_bytes: 8192,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecallProvenance {
    pub kind: String,
    pub id: Uuid,
    pub source_id: Option<Uuid>,
    pub repository_id: Option<Uuid>,
    pub snapshot_id: Option<Uuid>,
    pub revision: Option<String>,
    pub path: Option<String>,
    pub line_from: Option<i32>,
    pub line_to: Option<i32>,
    pub byte_from: Option<i32>,
    pub byte_to: Option<i32>,
    pub availability: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecallClaimState {
    pub kind: String,
    pub review: String,
    pub lifecycle: String,
    pub freshness: String,
    pub operational: String,
    pub origin: String,
    pub reviewer_id: Option<Uuid>,
    pub acceptance_policy: Option<String>,
    pub validity: FactValidity,
    pub knowledge_until: Option<DateTime<Utc>>,
    pub conflicting_claim_ids: Vec<Uuid>,
    pub rule_ids: Vec<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecallItem {
    pub kind: String,
    pub id: Uuid,
    pub revision_id: Uuid,
    pub label: String,
    pub text: String,
    pub recorded_at: DateTime<Utc>,
    pub selection: ScopeSelection,
    pub channels: Vec<String>,
    pub score: f32,
    pub semantic_similarity: Option<f32>,
    pub graph_match: Option<RecallGraphMatch>,
    pub qualifications: Vec<String>,
    pub provenance: Vec<RecallProvenance>,
    pub claim: Option<RecallClaimState>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecallContext {
    pub instruction: String,
    pub mode: String,
    pub items: Vec<RecallItem>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct RecallCoverage {
    pub examined: usize,
    pub withheld: usize,
    pub unavailable: usize,
    pub partial: bool,
    pub reasons: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecallResponse {
    pub brain_id: Uuid,
    pub operation_id: Option<Uuid>,
    pub scope_id: Option<Uuid>,
    pub selection: ScopeSelection,
    pub manifest_revision_id: Option<Uuid>,
    pub knowledge_at: DateTime<Utc>,
    pub fact_at: Option<DateTime<Utc>>,
    pub status: String,
    pub algorithm: String,
    pub semantic: Option<RecallSemantic>,
    pub graph: Option<RecallGraph>,
    pub context_selection: RecallContextSelection,
    pub memory_epoch: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub context: RecallContext,
    pub context_bytes: usize,
    pub coverage: RecallCoverage,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecallSemantic {
    pub profile: Option<crate::SemanticProfile>,
    pub model_request_id: Option<Uuid>,
    pub scoped_entries: usize,
    pub state: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(default, deny_unknown_fields)]
pub struct RecallGraphOptions {
    pub kind: String,
    pub direction: String,
    pub max_hops: usize,
    pub relations: Vec<String>,
}
impl Default for RecallGraphOptions {
    fn default() -> Self {
        Self {
            kind: "knowledge".into(),
            direction: "both".into(),
            max_hops: 2,
            relations: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecallGraphEntity {
    pub key: String,
    pub kind: String,
    pub id: Uuid,
    pub revision_id: Uuid,
    pub label: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecallGraphMatch {
    pub generation_id: Uuid,
    pub anchor_key: String,
    pub direction: String,
    pub nodes: Vec<RecallGraphEntity>,
    pub edges: Vec<crate::GraphEdge>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RecallGraph {
    pub view: crate::GraphView,
    pub anchors: Vec<String>,
    pub candidates: usize,
    pub state: String,
    pub expires_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct RecallContextSelection {
    pub source_diversity: bool,
    pub distinct_source_groups: usize,
    pub unknown_lineage_items: usize,
}
