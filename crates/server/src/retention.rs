use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    jobs,
    memory_evidence::Tx,
    memory_rules,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use recollect_protocol::{RetentionPolicy, RetentionSettings, RetentionUpdate};
use serde_json::json;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

pub fn effective_state(recorded: &str, deadline: Option<chrono::DateTime<chrono::Utc>>) -> &str {
    if recorded == "active" && deadline.is_some_and(|at| at <= chrono::Utc::now()) {
        "expired"
    } else {
        recorded
    }
}
pub fn unavailable() -> Error {
    Error(
        StatusCode::GONE,
        "content_removed",
        "This content expired or was erased and cannot be used for new work.",
    )
}
pub async fn job_available(tx: &mut Tx<'_>, brain: Uuid, target: Uuid, kind: &str) -> Result<bool> {
    if !matches!(kind, "source.process" | "repository.process") {
        return Ok(true);
    }
    Ok(sqlx::query_scalar("SELECT CASE $3 WHEN 'source.process' THEN EXISTS(SELECT 1 FROM source_versions WHERE brain_id=$1 AND id=$2 AND recollect_content_state(brain_id,retention_class,privacy_state,created_at)='active') ELSE EXISTS(SELECT 1 FROM repository_snapshots WHERE brain_id=$1 AND id=$2 AND recollect_content_state(brain_id,'repository',privacy_state,created_at)='active') END")
        .bind(brain).bind(target).bind(kind).fetch_one(&mut **tx).await?)
}

pub fn validate(policy: &RetentionPolicy) -> Result<()> {
    let days = [
        Some(policy.raw_session_days),
        Some(policy.tool_output_days),
        policy.document_days,
        policy.support_excerpt_days,
        policy.repository_days,
        policy.claim_days,
        Some(policy.audit_days),
    ];
    if days.into_iter().flatten().any(|d| !(1..=3650).contains(&d))
        || !(1..=365).contains(&policy.backup_days)
    {
        return Err(Error::invalid(
            "Retention is 1–3,650 days; backup retention is 1–365 days. Durable classes may use no automatic expiry.",
        ));
    }
    Ok(())
}
pub async fn settings(tx: &mut Tx<'_>, brain: Uuid) -> Result<RetentionSettings> {
    let saved: Option<(Uuid, SqlJson<RetentionPolicy>)> =
        sqlx::query_as("SELECT change_id,policy FROM retention_policies WHERE brain_id=$1")
            .bind(brain)
            .fetch_optional(&mut **tx)
            .await?;
    let (change_id, policy) = saved
        .map(|(id, policy)| (id, policy.0))
        .unwrap_or((Uuid::nil(), RetentionPolicy::default()));
    Ok(RetentionSettings {
        brain_id: brain,
        change_id,
        policy,
    })
}
#[utoipa::path(get,path="/api/brains/{brain}/retention",operation_id="retentionSettings",params(("brain"=Uuid,Path)),responses((status=200,body=RetentionSettings)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<RetentionSettings>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let result = settings(&mut tx, brain).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(put,path="/api/brains/{brain}/retention",operation_id="updateRetention",params(("brain"=Uuid,Path)),request_body=RetentionUpdate,responses((status=200,body=RetentionSettings)))]
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<RetentionUpdate>,
) -> Result<Json<RetentionSettings>> {
    auth.require_browser()?;
    validate(&input.policy)?;
    let key = commands::key(&headers)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, true).await?;
    if let Some(result) = commands::reserve(
        &mut tx,
        key.as_deref(),
        "retention.update",
        json!({"brain":brain,"input":input}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(result));
    }
    let previous = settings(&mut tx, brain).await?;
    if previous.change_id != input.base_change {
        return Err(Error(
            StatusCode::CONFLICT,
            "retention_changed",
            "Retention policy changed. Reload before updating it.",
        ));
    }
    jobs::capacity(&mut tx, brain).await?;
    let result = RetentionSettings {
        brain_id: brain,
        change_id: Uuid::new_v4(),
        policy: input.policy,
    };
    sqlx::query("INSERT INTO retention_policies(brain_id,change_id,policy,updated_by) VALUES($1,$2,$3,$4) ON CONFLICT(brain_id) DO UPDATE SET change_id=excluded.change_id,policy=excluded.policy,updated_by=excluded.updated_by,updated_at=clock_timestamp()")
        .bind(brain).bind(result.change_id).bind(SqlJson(&result.policy)).bind(auth.user.id).execute(&mut *tx).await?;
    memory_rules::advance_epoch(&mut tx, brain).await?;
    sqlx::query("SELECT recollect_invalidate_receipts($1)")
        .bind(brain)
        .execute(&mut *tx)
        .await?;
    let audit = db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "retention.update",
        brain,
        "policy_changed",
    )
    .await?;
    jobs::enqueue(&mut tx, auth.user.id, brain, audit).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    tx.commit().await?;
    Ok(Json(result))
}
