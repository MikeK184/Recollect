//! One auditable adoption of the managed preset; domain workers still own work.
use crate::{
    AppState, auth::Auth, capture, commands, db, error::Result, memory_evidence::Tx, model_policy,
    retention,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use recollect_protocol::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AutomationSettings {
    pub models: ModelSettings,
    pub capture: CaptureSettings,
    pub retention: RetentionSettings,
}
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AutomationUpdate {
    pub model_change: Uuid,
    pub capture_change: Uuid,
}
async fn settings(state: &AppState, tx: &mut Tx<'_>, brain: Uuid) -> Result<AutomationSettings> {
    Ok(AutomationSettings {
        models: ModelSettings {
            current: model_policy::current(state, tx, brain).await?,
            installed: model_policy::installed(state),
        },
        capture: capture::current(tx, brain).await?,
        retention: retention::settings(tx, brain).await?,
    })
}
pub(crate) async fn apply(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
) -> Result<AutomationSettings> {
    let mut policy = model_policy::defaults(state);
    policy.enabled = true;
    policy.purposes = ["extraction", "synthesis", "embedding", "answering"]
        .map(String::from)
        .to_vec();
    policy.content_classes = model_policy::CLASSES.iter().map(|c| (*c).into()).collect();
    policy.max_input_bytes = 32 * 1024;
    policy.max_output_tokens = 4096;
    policy.daily_token_limit = 1_000_000;
    policy.max_concurrent = 2;
    policy.automatic_learning = true;
    policy.automatic_embedding = true;
    policy.autonomous_memory = true;
    model_policy::persist(state, tx, brain, actor, policy).await?;
    let mut policy = capture::current(tx, brain).await?.policy;
    policy.enabled = true;
    policy.managed_tools = true;
    policy.kinds = CAPTURE_KINDS.iter().map(|k| (*k).into()).collect();
    policy.max_event_bytes = 32 * 1024;
    capture::persist(tx, brain, actor, policy).await?;
    settings(state, tx, brain).await
}

#[utoipa::path(get,path="/api/brains/{brain}/automation",operation_id="automationSettings",params(("brain"=Uuid,Path)),responses((status=200,body=AutomationSettings)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<AutomationSettings>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let result = settings(&state, &mut tx, brain).await?;
    tx.commit().await?;
    Ok(Json(result))
}

#[utoipa::path(put,path="/api/brains/{brain}/automation",operation_id="adoptManagedMemory",params(("brain"=Uuid,Path)),request_body=AutomationUpdate,responses((status=200,body=AutomationSettings)))]
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<AutomationUpdate>,
) -> Result<Json<AutomationSettings>> {
    auth.require_browser()?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, true).await?;
    db::require_writer(&mut tx, brain).await?;
    let key = commands::key(&headers)?;
    if let Some(saved) = commands::reserve(
        &mut tx,
        key.as_deref(),
        "memory.managed",
        json!({"brain":brain,"input":input}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(saved));
    }
    let current = settings(&state, &mut tx, brain).await?;
    if current.models.current.change_id != input.model_change
        || current.capture.change_id != input.capture_change
    {
        return Err(model_policy::failure(
            "automation_changed",
            "Brain settings changed. Reload before adopting managed memory.",
        ));
    }
    let result = apply(&state, &mut tx, brain, auth.user.id).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    tx.commit().await?;
    Ok(Json(result))
}
