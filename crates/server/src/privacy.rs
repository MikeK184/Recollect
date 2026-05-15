use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    memory_evidence::Tx,
    publication,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub source_versions: Vec<Uuid>,
    pub source_ids: Vec<Uuid>,
    pub claim_revisions: Vec<Uuid>,
    pub claim_ids: Vec<Uuid>,
    pub snapshots: Vec<Uuid>,
    pub facts: Vec<Uuid>,
    pub manifest_revisions: Vec<Uuid>,
    pub rules: Vec<Uuid>,
    pub decisions: Vec<Uuid>,
    pub artifacts: Vec<Uuid>,
    pub jobs: Vec<Uuid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_input_sources: Vec<Uuid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_input_claim_revisions: Vec<Uuid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capture_event_fences: Vec<CaptureFence>,
    pub publication_fences: Vec<PublicationFence>,
    pub independent_claim_revisions: i64,
    pub shared_sources: bool,
}
fn validate(target: &ErasureTarget) -> Result<()> {
    if !matches!(
        target.kind.as_str(),
        "source" | "claim" | "snapshot" | "manifest" | "collection"
    ) {
        return Err(Error::invalid(
            "Choose a source, claim, snapshot, manifest or collection.",
        ));
    }
    Ok(())
}
async fn epoch(tx: &mut Tx<'_>, brain: Uuid) -> Result<i64> {
    Ok(
        sqlx::query_scalar(
            "SELECT coalesce((SELECT epoch FROM memory_epochs WHERE brain_id=$1),0)",
        )
        .bind(brain)
        .fetch_one(&mut **tx)
        .await?,
    )
}
#[utoipa::path(post,path="/api/brains/{brain}/erasures/preview",operation_id="previewErasure",params(("brain"=Uuid,Path)),request_body=ErasureTarget,responses((status=200,body=ErasurePreview)))]
pub async fn preview(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(target): Json<ErasureTarget>,
) -> Result<Json<ErasurePreview>> {
    auth.require_browser()?;
    validate(&target)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, true).await?;
    let closure: Option<SqlJson<Manifest>> =
        sqlx::query_scalar("SELECT recollect_privacy_preview($1,$2,$3)")
            .bind(brain)
            .bind(&target.kind)
            .bind(target.id)
            .fetch_one(&mut *tx)
            .await?;
    let m = closure.ok_or_else(Error::missing)?.0;
    let result = ErasurePreview {
        brain_id: brain,
        target,
        eligibility_epoch: epoch(&mut tx, brain).await?,
        source_versions: m.source_versions.len() as i64,
        claim_revisions: m.claim_revisions.len() as i64,
        snapshots: m.snapshots.len() as i64,
        manifest_revisions: m.manifest_revisions.len() as i64,
        artifacts: m.artifacts.len() as i64,
        jobs: m.jobs.len() as i64,
        model_input_fences: m.model_input_sources.len() as i64,
        model_claim_fences: m.model_input_claim_revisions.len() as i64,
        capture_event_fences: m.capture_event_fences.len() as i64,
        independent_claim_revisions: m.independent_claim_revisions,
        shared_sources: m.shared_sources,
    };
    tx.commit().await?;
    Ok(Json(result))
}
#[derive(sqlx::FromRow)]
struct StatusRow {
    id: Uuid,
    brain_id: Uuid,
    sequence: i64,
    target: SqlJson<ErasureTarget>,
    cause: String,
    created_at: DateTime<Utc>,
    state: String,
    journaled: bool,
    pending_artifacts: i64,
    graph_pending: bool,
    error_code: Option<String>,
    acknowledged_devices: i64,
}
impl From<StatusRow> for ErasureStatus {
    fn from(r: StatusRow) -> Self {
        Self {
            id: r.id,
            brain_id: r.brain_id,
            sequence: r.sequence,
            target: r.target.0,
            cause: r.cause,
            created_at: r.created_at,
            state: r.state,
            journaled: r.journaled,
            pending_artifacts: r.pending_artifacts,
            graph_pending: r.graph_pending,
            error_code: r.error_code,
            local_copies: "device_check_in_required".into(),
            acknowledged_devices: r.acknowledged_devices,
        }
    }
}
const STATUS: &str = "r.*,(SELECT count(*) FROM privacy_artifacts a WHERE a.request_id=r.id AND a.removed_at IS NULL) AS pending_artifacts,(SELECT count(*) FROM privacy_device_positions d WHERE d.brain_id=r.brain_id AND d.sequence>=r.sequence) AS acknowledged_devices FROM privacy_requests r";
async fn status(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<ErasureStatus> {
    sqlx::query_as::<_, StatusRow>(&format!("SELECT {STATUS} WHERE r.brain_id=$1 AND r.id=$2"))
        .bind(brain)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .map(Into::into)
        .ok_or_else(Error::missing)
}
#[utoipa::path(post,path="/api/brains/{brain}/erasures",operation_id="eraseMemory",params(("brain"=Uuid,Path)),request_body=ErasureInput,responses((status=200,body=ErasureStatus)))]
pub async fn erase(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<ErasureInput>,
) -> Result<Json<ErasureStatus>> {
    auth.require_browser()?;
    validate(&input.target)?;
    let key = commands::key(&headers)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, true).await?;
    if let Some(saved) = commands::reserve::<ErasureStatus>(
        &mut tx,
        key.as_deref(),
        "memory.erase",
        json!({"brain":brain,"input":input}),
    )
    .await?
    {
        let result = status(&mut tx, brain, saved.id).await?;
        tx.commit().await?;
        return Ok(Json(result));
    }
    if epoch(&mut tx, brain).await? != input.eligibility_epoch {
        return Err(Error(
            StatusCode::CONFLICT,
            "erasure_preview_changed",
            "Memory or scope changed. Preview the erasure again.",
        ));
    }
    let id = Uuid::new_v4();
    let accepted: Option<Uuid> = sqlx::query_scalar("SELECT recollect_erase($1,$2,$3,$4,$5)")
        .bind(brain)
        .bind(&input.target.kind)
        .bind(input.target.id)
        .bind(input.eligibility_epoch)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if accepted.is_none() {
        return Err(Error::missing());
    }
    let result = status(&mut tx, brain, id).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/brains/{brain}/erasures",operation_id="erasures",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=ErasurePage)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(page): Query<publication::Page>,
) -> Result<Json<ErasurePage>> {
    let offset = publication::offset(&page)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let (total, latest_sequence): (i64, i64) = sqlx::query_as(
        "SELECT count(*),coalesce(max(sequence),0) FROM privacy_requests WHERE brain_id=$1",
    )
    .bind(brain)
    .fetch_one(&mut *tx)
    .await?;
    let rows = sqlx::query_as::<_, StatusRow>(&format!(
        "SELECT {STATUS} WHERE r.brain_id=$1 ORDER BY r.sequence DESC LIMIT 20 OFFSET $2"
    ))
    .bind(brain)
    .bind(offset)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(ErasurePage {
        items: rows.into_iter().map(Into::into).collect(),
        total,
        offset,
        latest_sequence,
    }))
}

#[utoipa::path(post,path="/api/brains/{brain}/erasures/{request}/retry",operation_id="retryErasure",params(("brain"=Uuid,Path),("request"=Uuid,Path)),responses((status=200,body=ErasureStatus)))]
pub async fn retry(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ErasureStatus>> {
    auth.require_browser()?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, true).await?;
    status(&mut tx, brain, id).await?;
    tx.commit().await?;
    crate::privacy_journal::maintain(&state)
        .await
        .map_err(|_| {
            Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "privacy_maintenance_unavailable",
                "Privacy maintenance is unavailable. Its committed request remains pending.",
            )
        })?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let result = status(&mut tx, brain, id).await?;
    tx.commit().await?;
    Ok(Json(result))
}

#[utoipa::path(get,path="/api/brains/{brain}/privacy-sync",operation_id="privacyDeviceSync",params(("brain"=Uuid,Path)),responses((status=200,body=PrivacyDeviceSync)))]
pub async fn device_sync(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<PrivacyDeviceSync>> {
    auth.device_id.ok_or_else(Error::forbidden)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let policy = crate::retention::settings(&mut tx, brain).await?.policy;
    let sequence = sqlx::query_scalar(
        "SELECT coalesce(max(sequence),0) FROM privacy_requests WHERE brain_id=$1",
    )
    .bind(brain)
    .fetch_one(&mut *tx)
    .await?;
    let rows:Vec<(Uuid,String)>=sqlx::query_as("SELECT repository_id,revision FROM privacy_publication_fences WHERE brain_id=$1 UNION SELECT repository_id,revision FROM repository_snapshots WHERE brain_id=$1 AND recollect_content_state(brain_id,'repository',privacy_state,created_at)<>'active'").bind(brain).fetch_all(&mut *tx).await?;
    let fences: Vec<(Uuid,Uuid,Option<String>)> = sqlx::query_as("SELECT event_id,binding_id,native_key FROM privacy_capture_fences WHERE brain_id=$1 UNION SELECT id,binding_id,native_key FROM capture_events WHERE brain_id=$1 AND (state<>'accepted' OR recollect_retention_deadline(brain_id,retention_class,captured_at)<=clock_timestamp())")
        .bind(brain).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(PrivacyDeviceSync {
        brain_id: brain,
        policy,
        sequence,
        capture_event_fences: fences
            .into_iter()
            .map(|(event_id, binding_id, native_key)| CaptureFence {
                event_id,
                binding_id,
                native_key,
            })
            .collect(),
        publication_fences: rows
            .into_iter()
            .map(|(repository_id, revision)| PublicationFence {
                repository_id,
                revision,
            })
            .collect(),
    }))
}
#[utoipa::path(post,path="/api/brains/{brain}/privacy-sync",operation_id="acknowledgePrivacy",params(("brain"=Uuid,Path)),request_body=PrivacyDeviceAck,responses((status=200,body=PrivacyDeviceReceipt)))]
pub async fn device_ack(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<PrivacyDeviceAck>,
) -> Result<Json<PrivacyDeviceReceipt>> {
    let device_id = auth.device_id.ok_or_else(Error::forbidden)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let latest: i64 = sqlx::query_scalar(
        "SELECT coalesce(max(sequence),0) FROM privacy_requests WHERE brain_id=$1",
    )
    .bind(brain)
    .fetch_one(&mut *tx)
    .await?;
    if input.sequence < 0 || input.sequence > latest {
        return Err(Error::invalid("Acknowledge an observed deletion position."));
    }
    let (sequence, checked_at): (i64, DateTime<Utc>) =
        sqlx::query_as("SELECT sequence,checked_at FROM recollect_privacy_device_ack($1,$2)")
            .bind(brain)
            .bind(input.sequence)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(Json(PrivacyDeviceReceipt {
        brain_id: brain,
        device_id,
        sequence,
        checked_at,
    }))
}
