//! Optional operation scope for the existing evidence/history read handlers.
use crate::{
    auth::Auth,
    error::{Error, Result},
    memory_evidence::Tx,
    workspace,
};
use recollect_protocol::ScopeSelection;
use serde_json::json;
use sqlx::types::Json;
use std::collections::BTreeSet;
use uuid::Uuid;

pub(crate) async fn scope(
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    operation: Option<Uuid>,
) -> Result<Option<ScopeSelection>> {
    let Some(operation) = operation else {
        return Ok(None);
    };
    let binding = workspace::bound_operation(tx, brain, operation).await?;
    if binding.actor_id != auth.user.id
        || binding.device_id != auth.device_id
        || !matches!(binding.kind.as_str(), "context" | "retrieval")
        || !binding.scope_valid
    {
        return Err(Error::forbidden());
    }
    Ok(Some(binding.scope.selection))
}

/// Batch the canonical recall predicate; do not copy its applicability rules.
pub(crate) async fn mask<'a>(
    tx: &mut Tx<'_>,
    requested: &ScopeSelection,
    selections: impl IntoIterator<Item = &'a ScopeSelection>,
) -> Result<BTreeSet<usize>> {
    let candidates: Vec<_> = selections
        .into_iter()
        .enumerate()
        .map(|(index, selection)| json!({"ordinal":index,"selection":selection}))
        .collect();
    let allowed: Vec<i64> = sqlx::query_scalar(
        "SELECT ordinal FROM jsonb_to_recordset($1) a(ordinal bigint,selection jsonb)
         WHERE recollect_recall_scope(selection,$2)",
    )
    .bind(Json(candidates))
    .bind(Json(requested))
    .fetch_all(&mut **tx)
    .await?;
    Ok(allowed.into_iter().map(|n| n as usize).collect())
}
