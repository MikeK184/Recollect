//! Irreversible Brain-wide deletion (ADR 0016). The SQL layer owns closure,
//! tombstones and visibility; these handlers enforce the caller class, map the
//! function's distinct rejection codes to actionable responses, and expose the
//! companion fence. No handler performs a Vault call.
use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::types::Json as SqlJson;
use utoipa::ToSchema;
use uuid::Uuid;

/// Per-class dependent identity counts. The keys mirror the migration-owned
/// preview document exactly; a drift here is a schema break, not a default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BrainDeletionCounts {
    pub sources: i64,
    pub source_versions: i64,
    pub excerpts: i64,
    pub chunks: i64,
    pub claims: i64,
    pub claim_revisions: i64,
    pub review_decisions: i64,
    pub rejected_rules: i64,
    pub snapshots: i64,
    pub facts: i64,
    pub manifests: i64,
    pub manifest_revisions: i64,
    pub environments: i64,
    pub collections: i64,
    pub areas: i64,
    pub memberships: i64,
    pub repositories: i64,
    pub workspaces: i64,
    pub checkouts: i64,
    pub tasks: i64,
    pub capture_bindings: i64,
    pub capture_events: i64,
    pub capture_reports: i64,
    pub connections: i64,
    pub profiles: i64,
    pub profile_grants: i64,
    pub calls: i64,
    pub private_runners: i64,
    pub semantic_entries: i64,
    pub semantic_profiles: i64,
    pub learning_runs: i64,
    pub handover_runs: i64,
    pub graph_generations: i64,
    pub analytics_reports: i64,
    pub members: i64,
    pub group_members: i64,
    pub policies: i64,
    pub artifacts: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BrainDeletionPreview {
    pub brain_id: Uuid,
    pub name: String,
    pub archived: bool,
    #[serde(with = "decimal_counter")]
    #[schema(value_type = String)]
    pub closure: i64,
    pub counts: BrainDeletionCounts,
    pub repository_content_allowed: bool,
    pub pending_work: i64,
    pub backup_days: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BrainDeletionInput {
    #[serde(with = "decimal_counter")]
    #[schema(value_type = String)]
    pub closure: i64,
    pub confirmation: String,
}

/// Bounded, content-free deletion status derived from the retained journal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BrainDeletionStatus {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub actor_id: Option<Uuid>,
    #[serde(with = "decimal_counter")]
    #[schema(value_type = String)]
    pub closure: i64,
    pub disposition: String,
    pub created_at: DateTime<Utc>,
    pub deleted_at: DateTime<Utc>,
    pub sequence: i64,
    pub state: String,
    pub journaled: bool,
    pub graph_pending: bool,
    pub error_code: Option<String>,
    pub pending_artifacts: i64,
    pub acknowledged_devices: i64,
}

/// DELETE response: the request status plus the backup window that applies to
/// retained copies, captured before the deletion removes the policy row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BrainDeletionResult {
    pub request: BrainDeletionStatus,
    pub backup_days: i32,
}

/// Companion fence for a paired device's next check-in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BrainDeletionFence {
    pub brain_id: Uuid,
    pub deletion_id: Uuid,
    pub sequence: i64,
}

// PostgreSQL's 60-bit closure fingerprint exceeds JavaScript's exact integer
// range. Keep the database value intact and make the HTTP token opaque. Numeric
// input remains readable for pre-existing native clients and SQL JSON parsing.
mod decimal_counter {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(value: &i64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Counter {
            Decimal(String),
            Integer(i64),
        }
        let value = match Counter::deserialize(deserializer)? {
            Counter::Decimal(value) => value.parse::<i64>().map_err(D::Error::custom)?,
            Counter::Integer(value) => value,
        };
        if !(0..(1_i64 << 60)).contains(&value) {
            return Err(D::Error::custom("Invalid deletion preview counter"));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::BrainDeletionInput;

    #[test]
    fn closure_survives_a_browser_json_round_trip_without_numeric_rounding() {
        let input = BrainDeletionInput {
            closure: (1_i64 << 60) - 1,
            confirmation: "Disposable".into(),
        };
        let wire = serde_json::to_value(&input).unwrap();
        assert_eq!(wire["closure"], "1152921504606846975");
        assert_eq!(
            serde_json::from_value::<BrainDeletionInput>(wire).unwrap(),
            input
        );
        let native =
            serde_json::json!({"closure": input.closure, "confirmation": input.confirmation});
        assert_eq!(
            serde_json::from_value::<BrainDeletionInput>(native)
                .unwrap()
                .closure,
            input.closure
        );
        for invalid in [
            serde_json::json!(-1),
            serde_json::json!("1152921504606846976"),
            serde_json::json!("1.1"),
        ] {
            assert!(
                serde_json::from_value::<BrainDeletionInput>(
                    serde_json::json!({"closure":invalid,"confirmation":"Disposable"})
                )
                .is_err()
            );
        }
    }
}

fn unreadable(code: &'static str, message: &'static str) -> Error {
    Error(StatusCode::INTERNAL_SERVER_ERROR, code, message)
}

async fn parse_status(
    brain: Uuid,
    id: Uuid,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<BrainDeletionStatus> {
    let row: Option<SqlJson<serde_json::Value>> =
        sqlx::query_scalar("SELECT recollect_brain_deletion($1,$2)")
            .bind(brain)
            .bind(id)
            .fetch_one(&mut **tx)
            .await?;
    serde_json::from_value(row.ok_or_else(Error::missing)?.0).map_err(|_| {
        unreadable(
            "deletion_status_unreadable",
            "The deletion status could not be read.",
        )
    })
}

#[utoipa::path(post,path="/api/brains/{brain}/deletions/preview",operation_id="previewBrainDeletion",params(("brain"=Uuid,Path)),responses((status=200,body=BrainDeletionPreview)))]
pub async fn preview(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<BrainDeletionPreview>> {
    auth.require_browser()?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, true).await?;
    let preview: Option<SqlJson<serde_json::Value>> =
        sqlx::query_scalar("SELECT recollect_brain_preview($1)")
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
    let result = serde_json::from_value(preview.ok_or_else(Error::missing)?.0).map_err(|_| {
        unreadable(
            "deletion_preview_unreadable",
            "The deletion preview could not be read.",
        )
    })?;
    tx.commit().await?;
    Ok(Json(result))
}

fn delete_error(error: sqlx::Error) -> Error {
    let code = error.as_database_error().and_then(|d| d.code());
    match code.as_deref() {
        Some("22023") => Error(
            StatusCode::BAD_REQUEST,
            "brain_deletion_confirmation_mismatch",
            "The confirmation does not match this Brain. Preview the deletion again.",
        ),
        Some("40001") => Error(
            StatusCode::CONFLICT,
            "brain_deletion_preview_changed",
            "This Brain changed since the preview. Preview the deletion again.",
        ),
        Some("42501") => Error::forbidden(),
        _ => Error::from(error),
    }
}

#[utoipa::path(delete,path="/api/brains/{brain}",operation_id="deleteBrain",params(("brain"=Uuid,Path)),request_body=BrainDeletionInput,responses((status=200,body=BrainDeletionResult),(status=400,body=recollect_protocol::ApiError),(status=403,body=recollect_protocol::ApiError),(status=404,body=recollect_protocol::ApiError),(status=409,body=recollect_protocol::ApiError)))]
pub async fn delete(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<BrainDeletionInput>,
) -> Result<Json<BrainDeletionResult>> {
    auth.require_browser()?;
    if input.confirmation.is_empty() || input.confirmation.len() > 120 {
        return Err(Error::invalid(
            "Type the exact Brain name to confirm its deletion.",
        ));
    }
    let key = commands::key(&headers)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, true).await?;
    // A committed deletion invalidates its receipt, so a keyed replay reaches
    // GONE inside reserve. This operation never saves a response, so reserve
    // only returns GONE, a key conflict, or None here.
    commands::reserve::<BrainDeletionResult>(
        &mut tx,
        key.as_deref(),
        "brain.delete",
        json!({"brain":brain,"closure":input.closure,"confirmation":input.confirmation}),
    )
    .await?;
    // Bind the receipt to the Brain before the deletion so the existing apply
    // chain invalidates it (key kept, input and response emptied); writing a
    // post-deletion receipt would violate row-level security.
    commands::bind_brain(&mut tx, key.as_deref(), brain).await?;
    let backup_days: i32 = sqlx::query_scalar(
        "SELECT coalesce((SELECT (policy->>'backup_days')::integer FROM retention_policies WHERE brain_id=$1),7)",
    )
    .bind(brain)
    .fetch_one(&mut *tx)
    .await?;
    let rid = Uuid::new_v4();
    let accepted: Option<Uuid> = sqlx::query_scalar("SELECT recollect_brain_delete($1,$2,$3,$4)")
        .bind(brain)
        .bind(input.closure)
        .bind(&input.confirmation)
        .bind(rid)
        .fetch_one(&mut *tx)
        .await
        .map_err(delete_error)?;
    if accepted.is_none() {
        return Err(Error::missing());
    }
    let result = BrainDeletionResult {
        request: parse_status(brain, rid, &mut tx).await?,
        backup_days,
    };
    tx.commit().await?;
    Ok(Json(result))
}

#[utoipa::path(get,path="/api/brains/{brain}/deletions/{request}",operation_id="brainDeletionStatus",params(("brain"=Uuid,Path),("request"=Uuid,Path)),responses((status=200,body=BrainDeletionStatus)))]
pub async fn status(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<BrainDeletionStatus>> {
    // No role check: the tombstone RLS limits visibility to the initiating
    // actor or the installation owner, and everything else is unknown.
    let mut tx = auth.tx(&state.pool).await?;
    let result = parse_status(brain, id, &mut tx).await?;
    tx.commit().await?;
    Ok(Json(result))
}

#[utoipa::path(get,path="/api/brains/{brain}/deletions/fence",operation_id="brainDeletionFence",params(("brain"=Uuid,Path)),responses((status=200,body=BrainDeletionFence)))]
pub async fn fence(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<BrainDeletionFence>> {
    // Devices only; a browser caller cannot apply local cleanup.
    auth.device_id.ok_or_else(Error::forbidden)?;
    let mut tx = auth.tx(&state.pool).await?;
    let deletion: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM brain_deletions WHERE brain_id=$1")
            .bind(brain)
            .fetch_optional(&mut *tx)
            .await?;
    let status = parse_status(brain, deletion.ok_or_else(Error::missing)?, &mut tx).await?;
    tx.commit().await?;
    Ok(Json(BrainDeletionFence {
        brain_id: brain,
        deletion_id: status.id,
        sequence: status.sequence,
    }))
}
