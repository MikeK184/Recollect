//! Canonical descriptors own graph meaning; Neo4j is a rebuildable projection.
use crate::{
    AppState,
    error::{Error, Result},
    memory_evidence::Tx,
};
use axum::http::StatusCode;
use recollect_protocol::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::types::Json;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

mod adapter;
pub mod analytics;
mod combined;
mod descriptor;
mod exploration;
mod queue;
mod read;
pub(crate) mod recall;
mod work;
pub use adapter::erase;
pub use exploration::{__path_explore, explore};
pub use queue::{__path_get, __path_rebuild, get, maintain_brain, rebuild, run_once};
pub use read::{__path_path, __path_view, path, view};
pub use work::execute;

pub(crate) const STRUCTURAL: &[&str] = &[
    "declares",
    "imports",
    "calls",
    "implements",
    "depends_on",
    "instantiates",
    "injects",
    "has_method",
    "handled_by",
    "implemented_by",
    "names",
];
pub(crate) const KNOWLEDGE: &[&str] = &["supported_by", "contributed_to"];
pub(crate) const NODE_LIMIT: usize = 5000;
pub(crate) const EDGE_LIMIT: usize = 20000;

pub(crate) fn failure(code: &'static str, message: &'static str) -> Error {
    Error(StatusCode::SERVICE_UNAVAILABLE, code, message)
}
pub(crate) fn scope_limit() -> Error {
    Error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "graph_scope_too_large",
        "This graph selection exceeds 5,000 candidates or 20,000 edges. Select a smaller scope.",
    )
}
pub(crate) fn selection_limit(nodes: usize) -> Error {
    if nodes == NODE_LIMIT {
        scope_limit()
    } else {
        Error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "analytics_scope_too_large",
            "This analysis exceeds 10,000 candidates or 50,000 relationships. Select a smaller scope.",
        )
    }
}
pub(crate) fn key(kind: &str, id: Uuid) -> String {
    format!("{kind}:{id}")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Entity {
    pub kind: String,
    pub id: Uuid,
    pub revision_id: Uuid,
}
impl Entity {
    pub fn key(&self) -> String {
        key(&self.kind, self.revision_id)
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Descriptor {
    pub nodes: Vec<Entity>,
    pub edges: Vec<GraphEdge>,
    pub unresolved: i64,
    pub ambiguous: i64,
    pub unsupported: i64,
    #[serde(default)]
    pub issues: Vec<GraphLinkIssue>,
    #[serde(default)]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub(crate) async fn epoch(tx: &mut Tx<'_>, brain: Uuid) -> Result<i64> {
    Ok(
        sqlx::query_scalar(
            "SELECT coalesce((SELECT epoch FROM memory_epochs WHERE brain_id=$1),0)",
        )
        .bind(brain)
        .fetch_one(&mut **tx)
        .await?,
    )
}
pub(crate) async fn generation(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<GraphGeneration> {
    let row: Option<Json<GraphGeneration>> = sqlx::query_scalar(
        "SELECT to_jsonb(g)-'descriptor' FROM graph_generations g WHERE brain_id=$1 AND id=$2",
    )
    .bind(brain)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|r| r.0).ok_or_else(Error::missing)
}
pub(crate) fn note(coverage: &mut RecallCoverage, reason: &str) {
    coverage.partial = true;
    if !coverage.reasons.iter().any(|r| r == reason) {
        coverage.reasons.push(reason.into());
    }
}
