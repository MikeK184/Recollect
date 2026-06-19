use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    jobs,
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
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

pub(crate) const PURPOSES: &[&str] = &["extraction", "synthesis", "embedding", "reranking"];
pub(crate) const CLASSES: &[&str] = &[
    "document",
    "raw_session",
    "tool_output",
    "support_excerpt",
    "repository",
    "claim",
    "query",
];

pub(crate) fn denied() -> Error {
    Error(
        StatusCode::FORBIDDEN,
        "model_policy_denied",
        "This Brain has not allowed this provider, model, purpose and content class.",
    )
}
pub(crate) fn failure(code: &'static str, message: &'static str) -> Error {
    Error(StatusCode::CONFLICT, code, message)
}
pub(crate) fn installed(state: &AppState) -> InstalledModels {
    let model = &state.config.models;
    InstalledModels {
        provider: "openai".into(),
        endpoint: model.endpoint.clone(),
        text_model: model.text_model.clone(),
        embedding_model: model.embedding_model.clone(),
        embedding_dimensions: model.embedding_dimensions,
        credentials_present: model.key.is_some(),
    }
}
pub(crate) fn defaults(state: &AppState) -> ModelPolicy {
    let model = &state.config.models;
    ModelPolicy {
        enabled: false,
        provider: "openai".into(),
        text_model: model.text_model.clone(),
        embedding_model: model.embedding_model.clone(),
        embedding_dimensions: model.embedding_dimensions,
        purposes: vec!["extraction".into(), "synthesis".into(), "embedding".into()],
        content_classes: vec!["document".into(), "claim".into(), "query".into()],
        max_input_bytes: 16384,
        max_output_tokens: 1024,
        daily_token_limit: 100000,
        max_concurrent: 1,
        automatic_learning: false,
        automatic_embedding: false,
        autonomous_memory: true,
        acceptance: None,
    }
}
pub(crate) async fn current(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
) -> Result<ModelPolicyVersion> {
    type Row = (Uuid, SqlJson<ModelPolicy>, Uuid, DateTime<Utc>);
    let row:Option<Row>=sqlx::query_as("SELECT p.id,p.policy,p.created_by,p.created_at FROM model_policy_heads h JOIN model_policies p ON p.id=h.policy_id AND p.brain_id=h.brain_id WHERE h.brain_id=$1")
        .bind(brain).fetch_optional(&mut **tx).await?;
    Ok(match row {
        Some((id, policy, actor, at)) => ModelPolicyVersion {
            change_id: id,
            brain_id: brain,
            policy: policy.0,
            created_by: Some(actor),
            created_at: Some(at),
        },
        None => ModelPolicyVersion {
            change_id: Uuid::nil(),
            brain_id: brain,
            policy: defaults(state),
            created_by: None,
            created_at: None,
        },
    })
}
pub(crate) fn identifier(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-/:".contains(&c))
}
pub(crate) fn matches_installation(state: &AppState, policy: &ModelPolicy) -> bool {
    let model = &state.config.models;
    policy.provider == "openai"
        && policy.text_model == model.text_model
        && policy.embedding_model == model.embedding_model
        && policy.embedding_dimensions == model.embedding_dimensions
}
fn values(values: &mut Vec<String>, allowed: &[&str]) -> Result<()> {
    if values.len() > allowed.len() || values.iter().any(|v| !allowed.contains(&v.as_str())) {
        return Err(Error::invalid(
            "Select supported model purposes and content classes.",
        ));
    }
    values.sort();
    values.dedup();
    Ok(())
}
async fn validate(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    p: &mut ModelPolicy,
) -> Result<()> {
    values(&mut p.purposes, PURPOSES)?;
    values(&mut p.content_classes, CLASSES)?;
    if !matches_installation(state, p) {
        return Err(failure(
            "model_configuration_changed",
            "Choose the models currently installed before saving this policy.",
        ));
    }
    if !(256..=32768).contains(&p.max_input_bytes)
        || !(128..=4096).contains(&p.max_output_tokens)
        || !(1000..=10000000).contains(&p.daily_token_limit)
        || !(1..=4).contains(&p.max_concurrent)
    {
        return Err(Error::invalid(
            "Model limits are outside the supported ranges.",
        ));
    }
    if p.automatic_learning && (!p.enabled || !p.purposes.iter().any(|v| v == "extraction")) {
        return Err(Error::invalid(
            "Automatic learning requires enabled extraction permission.",
        ));
    }
    if p.automatic_embedding && (!p.enabled || !p.purposes.iter().any(|v| v == "embedding")) {
        return Err(Error::invalid(
            "Automatic embedding requires enabled embedding permission.",
        ));
    }
    if p.enabled
        && p.autonomous_memory
        && (!["extraction", "synthesis"]
            .iter()
            .all(|v| p.purposes.iter().any(|p| p == v))
            || !p.content_classes.iter().any(|c| c == "claim"))
    {
        return Err(Error::invalid(
            "Autonomous memory requires extraction, synthesis and claim permission.",
        ));
    }
    if let Some(rule) = &mut p.acceptance {
        rule.name = rule.name.trim().into();
        if !identifier(&rule.name, 80)
            || rule.properties.is_empty()
            || rule.properties.len() > 20
            || rule.properties.iter().any(|v| !identifier(v, 80))
            || rule.collection_ids.len() > 20
        {
            return Err(Error::invalid(
                "Name the literal acceptance rule and select 1–20 bounded property identifiers.",
            ));
        }
        values(
            &mut rule.source_classes,
            &["document", "raw_session", "tool_output", "support_excerpt"],
        )?;
        if rule.source_classes.is_empty() {
            return Err(Error::invalid(
                "Select source classes for literal acceptance.",
            ));
        }
        rule.properties.sort();
        rule.properties.dedup();
        rule.collection_ids.sort();
        rule.collection_ids.dedup();
        for id in &rule.collection_ids {
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_groups WHERE brain_id=$1 AND id=$2 AND kind='collection')")
                .bind(brain).bind(id).fetch_one(&mut **tx).await?;
            if !exists {
                return Err(Error::missing());
            }
        }
    }
    publication::safe_payload(state, &json!(p))?;
    Ok(())
}
pub(crate) fn permits(
    state: &AppState,
    policy: &ModelPolicy,
    purpose: &str,
    classes: &[String],
) -> Result<()> {
    if !policy.enabled
        || !matches_installation(state, policy)
        || !policy.purposes.iter().any(|p| p == purpose)
        || classes.iter().any(|c| !policy.content_classes.contains(c))
    {
        return Err(denied());
    }
    if state.config.models.key.is_none() {
        return Err(Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "model_credentials_missing",
            "The installed provider credential is missing.",
        ));
    }
    Ok(())
}

#[utoipa::path(get,path="/api/brains/{brain}/models/policy",operation_id="modelPolicy",params(("brain"=Uuid,Path)),responses((status=200,body=ModelSettings)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<ModelSettings>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let current = current(&state, &mut tx, brain).await?;
    tx.commit().await?;
    Ok(Json(ModelSettings {
        current,
        installed: installed(&state),
    }))
}
#[utoipa::path(put,path="/api/brains/{brain}/models/policy",operation_id="updateModelPolicy",params(("brain"=Uuid,Path)),request_body=ModelPolicyUpdate,responses((status=200,body=ModelPolicyVersion)))]
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(mut input): Json<ModelPolicyUpdate>,
) -> Result<Json<ModelPolicyVersion>> {
    auth.require_browser()?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, true).await?;
    db::lock_brain(&mut tx, brain, true).await?;
    validate(&state, &mut tx, brain, &mut input.policy).await?;
    let key = commands::key(&headers)?;
    if let Some(saved) = commands::reserve::<ModelPolicyVersion>(
        &mut tx,
        key.as_deref(),
        "model.policy",
        json!({"brain":brain,"input":input}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(saved));
    }
    if current(&state, &mut tx, brain).await?.change_id != input.base_change {
        return Err(failure(
            "model_policy_changed",
            "This model policy changed. Reload it before saving.",
        ));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM model_policies WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    if count >= 1000 {
        return Err(failure(
            "model_policy_capacity",
            "This Brain has reached its model policy history capacity.",
        ));
    }
    let id = Uuid::new_v4();
    let at=sqlx::query_scalar("INSERT INTO model_policies(id,brain_id,policy,created_by) VALUES($1,$2,$3,$4) RETURNING created_at")
        .bind(id).bind(brain).bind(SqlJson(&input.policy)).bind(auth.user.id).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO model_policy_heads(brain_id,policy_id) VALUES($1,$2) ON CONFLICT(brain_id) DO UPDATE SET policy_id=excluded.policy_id")
        .bind(brain).bind(id).execute(&mut *tx).await?;
    let response = ModelPolicyVersion {
        change_id: id,
        brain_id: brain,
        policy: input.policy,
        created_by: Some(auth.user.id),
        created_at: Some(at),
    };
    let audit = db::audit(&mut tx, auth.user.id, brain, "model.policy", id, "updated").await?;
    jobs::enqueue(&mut tx, auth.user.id, brain, audit).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &response).await?;
    tx.commit().await?;
    Ok(Json(response))
}
#[derive(Default, Deserialize)]
pub struct PageQuery {
    pub offset: Option<i64>,
}
pub(crate) fn offset(query: &PageQuery) -> Result<i64> {
    let value = query.offset.unwrap_or(0);
    if !(0..=100000).contains(&value) {
        return Err(Error::invalid("Invalid page offset."));
    }
    Ok(value)
}
#[utoipa::path(get,path="/api/brains/{brain}/models/policy/history",operation_id="modelPolicyHistory",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=Vec<ModelPolicyVersion>)))]
pub async fn history(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<PageQuery>,
) -> Result<Json<Vec<ModelPolicyVersion>>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let rows:Vec<SqlJson<ModelPolicyVersion>>=sqlx::query_scalar("SELECT jsonb_build_object('change_id',id,'brain_id',brain_id,'policy',policy,'created_by',created_by,'created_at',created_at) FROM model_policies WHERE brain_id=$1 ORDER BY created_at DESC,id DESC LIMIT 20 OFFSET $2")
        .bind(brain).bind(offset(&query)?).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(rows.into_iter().map(|r| r.0).collect()))
}
pub(crate) async fn request(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<ModelRequest> {
    let row: SqlJson<ModelRequest> =
        sqlx::query_scalar("SELECT to_jsonb(r) FROM model_requests r WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(Error::missing)?;
    Ok(row.0)
}
#[utoipa::path(get,path="/api/brains/{brain}/models/usage",operation_id="modelUsage",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=ModelUsage)))]
pub async fn usage(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ModelUsage>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    sqlx::query("SELECT recollect_expire_model_requests($1)")
        .bind(brain)
        .execute(&mut *tx)
        .await?;
    let policy = current(&state, &mut tx, brain).await?;
    let (day,charged,in_flight):(String,i64,i64)=sqlx::query_as("SELECT to_char(clock_timestamp() AT TIME ZONE 'UTC','YYYY-MM-DD'),coalesce(sum(charged_tokens) FILTER(WHERE created_at >= (date_trunc('day',clock_timestamp() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC')),0)::bigint,count(*) FILTER(WHERE state='running') FROM model_requests WHERE brain_id=$1").bind(brain).fetch_one(&mut *tx).await?;
    let offset = offset(&query)?;
    let total = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    let rows:Vec<SqlJson<ModelRequest>>=sqlx::query_scalar("SELECT to_jsonb(r) FROM model_requests r WHERE brain_id=$1 ORDER BY created_at DESC,id DESC LIMIT 20 OFFSET $2").bind(brain).bind(offset).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(ModelUsage {
        day,
        daily_limit: policy.policy.daily_token_limit,
        charged_tokens: charged,
        remaining_tokens: (policy.policy.daily_token_limit - charged).max(0),
        in_flight,
        requests: rows.into_iter().map(|r| r.0).collect(),
        total,
        offset,
    }))
}
