//! Explicit typed canonical fields are not literal source-span mentions.
use crate::{GraphNode, GraphSelection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldReferenceKind {
    Repository,
    Environment,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FieldOrganizationEntity {
    pub id: Uuid,
    pub kind: FieldReferenceKind,
    pub reference_id: Uuid,
    pub label: String,
    /// Field on the original canonical reference which supplies its current name.
    pub label_pointer: String,
    pub contributor_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldOriginKind {
    SourceContext,
    ClaimRevision,
    RepositorySnapshot,
    ManifestRevision,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationFieldOrigin {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub input_id: Uuid,
    pub origin_kind: FieldOriginKind,
    pub logical_id: Uuid,
    pub revision_id: Uuid,
    /// RFC 6901 pointer into the immutable canonical input/context snapshot.
    pub field_pointer: String,
    /// Exact original UUID value at the declared pointer; never a text quote.
    pub value: Uuid,
    pub evidence_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldOrganizationCursor {
    pub after_input: Option<Uuid>,
    pub after_origin: Option<Uuid>,
    pub epoch: i64,
    pub scope: GraphSelection,
    pub entity_id: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldOrganizationRequest {
    pub scope: GraphSelection,
    pub entity_id: Option<Uuid>,
    pub cursor: Option<FieldOrganizationCursor>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct FieldOrganizationCoverage {
    pub examined_inputs: usize,
    pub qualified_inputs: usize,
    pub assigned_inputs: usize,
    pub unassigned_inputs: usize,
    pub counts_are_lower_bounds: bool,
    /// Content-free refresh state, independent of model/native profiles.
    pub refresh_state: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FieldOrganizationView {
    pub brain_id: Uuid,
    pub scope: GraphSelection,
    pub epoch: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub entities: Vec<FieldOrganizationEntity>,
    pub origins: Vec<OrganizationFieldOrigin>,
    pub evidence: Vec<GraphNode>,
    pub coverage: FieldOrganizationCoverage,
    pub next_cursor: Option<FieldOrganizationCursor>,
}
