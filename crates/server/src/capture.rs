use crate::{
    AppState, artifacts,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    evidence, jobs,
    memory_evidence::Tx,
    publication, workspace,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Duration, Utc};
use recollect_protocol::*;
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

fn conflict() -> Error {
    Error(
        StatusCode::CONFLICT,
        "capture_identity_conflict",
        "This capture identity already belongs to different input.",
    )
}
pub(crate) async fn current(tx: &mut Tx<'_>, brain: Uuid) -> Result<CaptureSettings> {
    let row: Option<(Uuid, SqlJson<CapturePolicy>)> =
        sqlx::query_as("SELECT change_id,policy FROM capture_policies WHERE brain_id=$1")
            .bind(brain)
            .fetch_optional(&mut **tx)
            .await?;
    let (change_id, policy) = row.map(|(id, p)| (id, p.0)).unwrap_or_default();
    Ok(CaptureSettings {
        brain_id: brain,
        change_id,
        policy,
    })
}
#[utoipa::path(get,path="/api/brains/{brain}/capture/policy",operation_id="capturePolicy",params(("brain"=Uuid,Path)),responses((status=200,body=CaptureSettings)))]
pub async fn get_policy(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<CaptureSettings>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let response = current(&mut tx, brain).await?;
    tx.commit().await?;
    Ok(Json(response))
}
#[utoipa::path(put,path="/api/brains/{brain}/capture/policy",operation_id="updateCapturePolicy",params(("brain"=Uuid,Path)),request_body=CapturePolicyUpdate,responses((status=200,body=CaptureSettings)))]
pub async fn update_policy(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<CapturePolicyUpdate>,
) -> Result<Json<CaptureSettings>> {
    auth.require_browser()?;
    validate_capture_policy(&input.policy).map_err(|_| {
        Error::invalid(
            "Choose supported capture kinds, bounded exclusions and a 1–64 KiB event limit.",
        )
    })?;
    publication::safe_payload(&state, &json!(input.policy))?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, true).await?;
    let key = commands::key(&headers)?;
    if let Some(saved) = commands::reserve(
        &mut tx,
        key.as_deref(),
        "capture.policy",
        json!({"brain":brain,"input":input}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(saved));
    }
    if current(&mut tx, brain).await?.change_id != input.base_change {
        return Err(Error(
            StatusCode::CONFLICT,
            "capture_policy_changed",
            "Capture policy changed. Reload before saving.",
        ));
    }
    let response = persist(&mut tx, brain, auth.user.id, input.policy).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &response).await?;
    tx.commit().await?;
    Ok(Json(response))
}

pub(crate) async fn persist(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    policy: CapturePolicy,
) -> Result<CaptureSettings> {
    let response = CaptureSettings {
        brain_id: brain,
        change_id: Uuid::new_v4(),
        policy,
    };
    sqlx::query("INSERT INTO capture_policies(brain_id,change_id,policy,updated_by) VALUES($1,$2,$3,$4) ON CONFLICT(brain_id) DO UPDATE SET change_id=excluded.change_id,policy=excluded.policy,updated_by=excluded.updated_by,updated_at=now()")
        .bind(brain).bind(response.change_id).bind(SqlJson(&response.policy)).bind(actor).execute(&mut **tx).await?;
    let audit = db::audit(
        tx,
        actor,
        brain,
        "capture.policy",
        response.change_id,
        "updated",
    )
    .await?;
    jobs::enqueue(tx, actor, brain, audit).await?;
    Ok(response)
}

#[derive(sqlx::FromRow)]
struct BindingRow {
    id: Uuid,
    brain_id: Uuid,
    operation_id: Uuid,
    actor_id: Uuid,
    device_id: Uuid,
    host: String,
    host_version: String,
    created_at: DateTime<Utc>,
}
impl BindingRow {
    async fn dto(self, tx: &mut Tx<'_>) -> Result<CaptureBinding> {
        Ok(CaptureBinding {
            id: self.id,
            brain_id: self.brain_id,
            device_id: self.device_id,
            operation: workspace::bound_operation(tx, self.brain_id, self.operation_id).await?,
            host: self.host,
            host_version: self.host_version,
            created_at: self.created_at,
        })
    }
}
async fn own_binding(tx: &mut Tx<'_>, brain: Uuid, id: Uuid, auth: &Auth) -> Result<BindingRow> {
    let row: BindingRow = sqlx::query_as(
        "SELECT * FROM capture_bindings WHERE host IN ('codex','claude_code','opencode') AND brain_id=$1 AND id=$2 AND actor_id=$3",
    )
    .bind(brain)
    .bind(id)
    .bind(auth.user.id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(Error::missing)?;
    if auth.device_id.is_some_and(|id| id != row.device_id) {
        return Err(Error::missing());
    }
    Ok(row)
}
#[utoipa::path(post,path="/api/brains/{brain}/capture/bindings",operation_id="bindCapture",params(("brain"=Uuid,Path)),request_body=CaptureBindingInput,responses((status=200,body=CaptureBinding)))]
pub async fn bind(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<CaptureBindingInput>,
) -> Result<Json<CaptureBinding>> {
    let device = auth.device_id.ok_or_else(Error::forbidden)?;
    if input.id.is_nil()
        || !matches!(input.host.as_str(), "codex" | "claude_code" | "opencode")
        || !capture_identity(&input.host_version)
        || input.host_version.len() > 120
    {
        return Err(Error::invalid(
            "Provide a capture UUID, supported host and bounded host version.",
        ));
    }
    publication::safe_payload(&state, &json!(input))?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    let existing: Option<BindingRow> =
        sqlx::query_as("SELECT * FROM capture_bindings WHERE host IN ('codex','claude_code','opencode') AND brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(input.id)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some(row) = existing {
        if row.actor_id != auth.user.id
            || row.device_id != device
            || row.operation_id != input.operation_id
            || row.host != input.host
            || row.host_version != input.host_version
        {
            return Err(conflict());
        }
        let response = row.dto(&mut tx).await?;
        tx.commit().await?;
        return Ok(Json(response));
    }
    let operation = workspace::bound_operation(&mut tx, brain, input.operation_id).await?;
    if operation.actor_id != auth.user.id
        || operation.device_id != Some(device)
        || operation.kind != "capture"
    {
        return Err(Error::missing());
    }
    let closed: bool =
        sqlx::query_scalar("SELECT closed FROM workspace_tasks WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(operation.task_id)
            .fetch_one(&mut *tx)
            .await?;
    if closed || !operation.scope_valid {
        return Err(Error(
            StatusCode::CONFLICT,
            "capture_scope_unavailable",
            "Start capture from an open task with an available scope.",
        ));
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM capture_bindings WHERE host IN ('codex','claude_code','opencode') AND brain_id=$1 AND device_id=$2",
    )
    .bind(brain)
    .bind(device)
    .fetch_one(&mut *tx)
    .await?;
    if count >= 5000 {
        return Err(Error(
            StatusCode::CONFLICT,
            "capture_binding_capacity",
            "This companion has reached its capture binding capacity.",
        ));
    }
    let inserted = sqlx::query("INSERT INTO capture_bindings(id,brain_id,operation_id,actor_id,device_id,host,host_version,selection) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
        .bind(input.id).bind(brain).bind(input.operation_id).bind(auth.user.id).bind(device)
        .bind(input.host).bind(input.host_version).bind(SqlJson(&operation.scope.selection)).execute(&mut *tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(conflict());
    }
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "capture.bind",
        input.id,
        "configured",
    )
    .await?;
    let response = own_binding(&mut tx, brain, input.id, &auth)
        .await?
        .dto(&mut tx)
        .await?;
    tx.commit().await?;
    Ok(Json(response))
}
#[derive(Default, Deserialize)]
pub struct CaptureQuery {
    pub offset: Option<i64>,
    pub binding_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub kind: Option<String>,
}
#[utoipa::path(post,path="/api/brains/{brain}/capture/devices",operation_id="reportCaptureDevice",params(("brain"=Uuid,Path)),request_body=CaptureDeviceReport,responses((status=204)))]
pub async fn report_device(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<CaptureDeviceReport>,
) -> Result<StatusCode> {
    let device = auth.device_id.ok_or_else(Error::forbidden)?;
    if !(0..=5000).contains(&input.pending)
        || !(0..=5000).contains(&input.denied)
        || input.pending + input.denied > 5000
        || !(0..=1_000_000_000_000).contains(&input.device_gap_count)
        || input.issue.as_deref().is_some_and(|s| !capture_issue(s))
    {
        return Err(Error::invalid(
            "Use bounded queue counts and a supported capture issue code.",
        ));
    }
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let owns: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM capture_bindings WHERE host IN ('codex','claude_code','opencode') AND brain_id=$1 AND device_id=$2 AND actor_id=$3)")
        .bind(brain).bind(device).bind(auth.user.id).fetch_one(&mut *tx).await?;
    if !owns {
        return Err(Error::missing());
    }
    sqlx::query("INSERT INTO capture_device_reports(brain_id,device_id,report) VALUES($1,$2,$3) ON CONFLICT(brain_id,device_id) DO UPDATE SET report=excluded.report,reported_at=now()")
        .bind(brain).bind(device).bind(SqlJson(input)).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get,path="/api/brains/{brain}/capture/devices",operation_id="captureDevices",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=CaptureDevicePage)))]
pub async fn devices(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<CaptureQuery>,
) -> Result<Json<CaptureDevicePage>> {
    let offset = query.offset.unwrap_or(0).max(0);
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    #[derive(sqlx::FromRow)]
    struct Row {
        device_id: Uuid,
        bindings: i64,
        reported_at: Option<DateTime<Utc>>,
        report: Option<SqlJson<CaptureDeviceReport>>,
        last_publication: Option<DateTime<Utc>>,
    }
    let total = sqlx::query_scalar(
        "SELECT count(DISTINCT device_id) FROM capture_bindings WHERE host IN ('codex','claude_code','opencode') AND brain_id=$1",
    )
    .bind(brain)
    .fetch_one(&mut *tx)
    .await?;
    let rows = sqlx::query_as::<_, Row>("WITH configured AS (SELECT device_id,count(*) bindings FROM capture_bindings WHERE host IN ('codex','claude_code','opencode') AND brain_id=$1 GROUP BY device_id), activity AS (SELECT b.device_id,max(e.received_at) last_publication FROM capture_events e JOIN capture_bindings b ON b.id=e.binding_id WHERE e.brain_id=$1 GROUP BY b.device_id) SELECT c.device_id,c.bindings,s.reported_at,s.report,a.last_publication FROM configured c LEFT JOIN capture_device_reports s ON s.device_id=c.device_id AND s.brain_id=$1 LEFT JOIN activity a ON a.device_id=c.device_id ORDER BY s.reported_at DESC NULLS LAST,c.device_id LIMIT 20 OFFSET $2")
        .bind(brain).bind(offset).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(CaptureDevicePage {
        total,
        offset,
        items: rows
            .into_iter()
            .map(|r| CaptureDeviceView {
                device_id: r.device_id,
                bindings: r.bindings,
                reported_at: r.reported_at,
                report: r.report.map(|v| v.0),
                last_publication: r.last_publication,
            })
            .collect(),
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/capture/bindings",operation_id="captureBindings",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=Vec<CaptureBinding>)))]
pub async fn bindings(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<CaptureQuery>,
) -> Result<Json<Vec<CaptureBinding>>> {
    let offset = publication::offset(&publication::Page {
        offset: query.offset,
    })?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let rows: Vec<BindingRow> = sqlx::query_as("SELECT * FROM capture_bindings WHERE host IN ('codex','claude_code','opencode') AND brain_id=$1 AND actor_id=$2 AND ($3::uuid IS NULL OR device_id=$3) ORDER BY created_at DESC,id DESC LIMIT 20 OFFSET $4")
        .bind(brain).bind(auth.user.id).bind(auth.device_id).bind(offset).fetch_all(&mut *tx).await?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.dto(&mut tx).await?);
    }
    tx.commit().await?;
    Ok(Json(items))
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    binding_id: Uuid,
    native_key: Option<String>,
    metadata: Option<SqlJson<CapturedHook>>,
    admission_policy: Option<SqlJson<CapturePolicy>>,
    source_id: Option<Uuid>,
    source_version_id: Option<Uuid>,
    state: String,
    expires_at: DateTime<Utc>,
    received_at: DateTime<Utc>,
    artifact_id: Option<Uuid>,
    byte_length: Option<i32>,
    host: String,
    host_version: String,
    operation_id: Option<Uuid>,
    managed_call_id: Option<Uuid>,
    selection: SqlJson<ScopeSelection>,
    device_id: Option<Uuid>,
    agent_name: Option<String>,
    user_name: String,
    processing: Option<String>,
    learning: Option<SqlJson<recollect_protocol::PipelineLearning>>,
}
const EVENT_SELECT: &str = "SELECT e.id,e.binding_id,e.native_key,e.metadata,e.admission_policy,e.source_id,e.source_version_id,
 CASE WHEN e.state='accepted' AND recollect_retention_deadline(e.brain_id,e.retention_class,e.captured_at)<=clock_timestamp() THEN 'expired' ELSE e.state END AS state,
 CASE WHEN e.state='accepted' THEN recollect_retention_deadline(e.brain_id,e.retention_class,e.captured_at) ELSE e.expires_at END AS expires_at,
 e.received_at,v.artifact_id,v.byte_length,b.host,b.host_version,b.operation_id,b.managed_call_id,b.selection,
 b.device_id,d.name AS agent_name,a.username AS user_name,
 CASE WHEN v.privacy_state='active' THEN v.processing END processing,
 CASE WHEN l.id IS NOT NULL AND v.privacy_state='active' THEN jsonb_build_object('id',l.id,'state',l.state,
 'created_at',l.created_at,'finished_at',l.finished_at,'accepted',l.accepted,'proposed',l.proposed,'blocked',l.blocked,
 'conflicting',l.conflicting,'reused',l.reused,'revised',l.revised,'retired',l.retired,'claim_ids',l.claim_ids,
 'job',jsonb_build_object('id',lj.id,'state',lj.state,'updated_at',lj.updated_at,'lease_until',lj.lease_until,'error_code',lj.error_code)) END learning
 FROM capture_events e JOIN capture_bindings b ON b.id=e.binding_id JOIN accounts a ON a.id=b.actor_id LEFT JOIN devices d ON d.id=b.device_id LEFT JOIN source_versions v ON v.id=e.source_version_id
 LEFT JOIN LATERAL (SELECT * FROM learning_runs WHERE brain_id=e.brain_id AND source_version_id=v.id AND state<>'removed' ORDER BY (state IN ('queued','running')) DESC,coalesce(finished_at,created_at) DESC,id DESC LIMIT 1) l ON true
 LEFT JOIN jobs lj ON lj.id=l.job_id AND lj.brain_id=e.brain_id";
impl EventRow {
    fn receipt(&self) -> CaptureReceipt {
        CaptureReceipt {
            event_id: self.id,
            binding_id: self.binding_id,
            state: self.state.clone(),
            source_id: self.source_id,
            source_version_id: self.source_version_id,
            expires_at: Some(self.expires_at),
            received_at: self.received_at,
        }
    }
}
async fn event_row(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<EventRow> {
    sqlx::query_as(&format!("{EVENT_SELECT} WHERE e.brain_id=$1 AND e.id=$2"))
        .bind(brain)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(Error::missing)
}
#[utoipa::path(post,path="/api/brains/{brain}/capture/events",operation_id="publishCapture",params(("brain"=Uuid,Path)),request_body=CaptureEventInput,responses((status=200,body=CaptureReceipt)))]
pub async fn publish(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(mut input): Json<CaptureEventInput>,
) -> Result<Json<CaptureReceipt>> {
    auth.device_id.ok_or_else(Error::forbidden)?;
    if matches!(
        input.event.host_event.as_str(),
        "ManagedTool" | "ManagedReceipt" | "ManagedResolution"
    ) {
        return Err(Error::invalid(
            "Managed observations are produced by the coordinator.",
        ));
    }
    if input.id.is_nil() {
        return Err(Error::invalid("Provide a stable event UUID."));
    }
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    let binding = own_binding(&mut tx, brain, input.binding_id, &auth)
        .await?
        .dto(&mut tx)
        .await?;
    if !binding.operation.scope_valid {
        return Err(Error(
            StatusCode::CONFLICT,
            "capture_scope_unavailable",
            "The original capture scope is no longer available.",
        ));
    }
    let policy = current(&mut tx, brain).await?.policy;
    let secrets = publication::configured_secrets(&state);
    let original_event = input.event.clone();
    let baseline = CapturePolicy {
        max_event_bytes: CAPTURE_MAX_BYTES as i32,
        ..Default::default()
    };
    sanitize_captured_event(&mut input.event, &baseline, &secrets)
        .map_err(|_| Error::invalid("Capture event fields or identities are invalid."))?;
    let native_key = capture_native_key(&binding.host, &input.event);
    type FenceRow = (Uuid, Uuid, String, DateTime<Utc>);
    let fence: Option<FenceRow> = sqlx::query_as("SELECT f.event_id,f.binding_id,CASE WHEN r.cause='erase' THEN 'removed' ELSE 'expired' END,r.created_at FROM privacy_capture_fences f JOIN privacy_requests r ON r.id=f.request_id WHERE f.brain_id=$1 AND (f.event_id=$2 OR (f.binding_id=$3 AND f.native_key=$4)) LIMIT 1")
        .bind(brain).bind(input.id).bind(input.binding_id).bind(&native_key).fetch_optional(&mut *tx).await?;
    if let Some((id, binding_id, removal, at)) = fence {
        if binding_id != input.binding_id {
            return Err(conflict());
        }
        let response = match sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM capture_events WHERE brain_id=$1 AND id=$2",
        )
        .bind(brain)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        {
            Some(id) => event_row(&mut tx, brain, id).await?.receipt(),
            None => CaptureReceipt {
                event_id: id,
                binding_id,
                state: removal,
                source_id: None,
                source_version_id: None,
                expires_at: None,
                received_at: at,
            },
        };
        tx.commit().await?;
        return Ok(Json(response));
    }
    let existing: Option<EventRow> = sqlx::query_as(&format!("{EVENT_SELECT} WHERE e.brain_id=$1 AND (e.id=$2 OR (e.binding_id=$3 AND e.native_key=$4)) ORDER BY (e.id=$2) DESC LIMIT 1"))
        .bind(brain).bind(input.id).bind(input.binding_id).bind(&native_key).fetch_optional(&mut *tx).await?;
    if let Some(row) = existing {
        if row.binding_id != input.binding_id || row.native_key != native_key {
            return Err(conflict());
        }
        if row.state == "accepted" {
            input.event = original_event;
            sanitize_captured_event(
                &mut input.event,
                &row.admission_policy.as_ref().ok_or_else(conflict)?.0,
                &secrets,
            )
            .map_err(|_| Error::invalid("Capture event fields or identities are invalid."))?;
            let mut expected = row.metadata.as_ref().ok_or_else(conflict)?.0.clone();
            expected.content = if let Some(artifact) = row.artifact_id {
                Some(
                    artifacts::read(
                        &state.config.artifact_dir,
                        brain,
                        artifact,
                        row.byte_length.unwrap_or_default(),
                    )
                    .await
                    .map_err(|_| {
                        Error(
                            StatusCode::SERVICE_UNAVAILABLE,
                            "artifact_unavailable",
                            "Retained capture is temporarily unavailable for replay verification.",
                        )
                    })?,
                )
            } else {
                None
            };
            // Native host retries have a fresh observation time but the same key.
            let mut compared = input.event.clone();
            if row.id != input.id {
                compared.captured_at = expected.captured_at;
            }
            if compared != expected {
                return Err(conflict());
            }
        } else {
            sqlx::query("SELECT recollect_expire_capture_event($1)")
                .bind(row.id)
                .execute(&mut *tx)
                .await?;
        }
        let response = row.receipt();
        tx.commit().await?;
        return Ok(Json(response));
    }
    if !policy.enabled {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "capture_denied",
            "Session capture is disabled for this Brain.",
        ));
    }
    input.event = original_event;
    sanitize_captured_event(&mut input.event, &policy, &secrets)
        .map_err(|_| Error::invalid("Capture event fields or identities are invalid."))?;
    let now = Utc::now();
    if input.event.captured_at > now + Duration::minutes(5)
        || input.event.captured_at < binding.created_at - Duration::minutes(5)
    {
        return Err(Error::invalid(
            "Capture time falls outside its binding or the permitted clock skew.",
        ));
    }
    let class = if input.event.kind == "tool_result" {
        "tool_output"
    } else {
        "raw_session"
    };
    let expires_at: DateTime<Utc> =
        sqlx::query_scalar("SELECT recollect_retention_deadline($1,$2,$3)")
            .bind(brain)
            .bind(class)
            .bind(input.event.captured_at)
            .fetch_one(&mut *tx)
            .await?;
    let mut metadata = input.event.clone();
    metadata.content = None;
    let inserted = sqlx::query("INSERT INTO capture_events(id,brain_id,binding_id,native_key,metadata,retention_class,captured_at,expires_at,admission_policy) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT DO NOTHING")
        .bind(input.id).bind(brain).bind(input.binding_id).bind(native_key)
        .bind((expires_at > now).then_some(SqlJson(&metadata))).bind(class).bind(input.event.captured_at).bind(expires_at)
        .bind((expires_at > now).then_some(SqlJson(&policy))).execute(&mut *tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(conflict());
    }
    if expires_at > now && input.event.content.is_some() {
        let (source, version) = evidence::capture_source(
            &state,
            &mut tx,
            brain,
            auth.user.id,
            input.id,
            &binding.host,
            &input.event,
        )
        .await?;
        sqlx::query("UPDATE capture_events SET source_id=$2,source_version_id=$3 WHERE id=$1")
            .bind(input.id)
            .bind(source)
            .bind(version)
            .execute(&mut *tx)
            .await?;
    }
    if expires_at <= now {
        sqlx::query("SELECT recollect_expire_capture_event($1)")
            .bind(input.id)
            .execute(&mut *tx)
            .await?;
    }
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "capture.publish",
        input.id,
        if expires_at <= now {
            "expired"
        } else {
            "accepted"
        },
    )
    .await?;
    let response = event_row(&mut tx, brain, input.id).await?.receipt();
    tx.commit().await?;
    Ok(Json(response))
}
#[utoipa::path(get,path="/api/brains/{brain}/capture/events",operation_id="captureEvents",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query),("binding_id"=Option<Uuid>,Query),("device_id"=Option<Uuid>,Query),("kind"=Option<String>,Query)),responses((status=200,body=CaptureEventPage)))]
pub async fn events(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<CaptureQuery>,
) -> Result<Json<CaptureEventPage>> {
    let offset = publication::offset(&publication::Page {
        offset: query.offset,
    })?;
    if query
        .kind
        .as_ref()
        .is_some_and(|k| !CAPTURE_KINDS.contains(&k.as_str()))
    {
        return Err(Error::invalid("Choose a supported capture kind."));
    }
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let filter = "e.brain_id=$1 AND ($2::uuid IS NULL OR e.binding_id=$2) AND ($3::text IS NULL OR e.metadata->>'kind'=$3) AND ($4::uuid IS NULL OR b.device_id=$4)";
    let total = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM capture_events e JOIN capture_bindings b ON b.id=e.binding_id WHERE {filter}"
    ))
    .bind(brain)
    .bind(query.binding_id)
    .bind(&query.kind)
    .bind(query.device_id)
    .fetch_one(&mut *tx)
    .await?;
    let rows: Vec<EventRow> = sqlx::query_as(&format!(
        "{EVENT_SELECT} WHERE {filter} ORDER BY e.received_at DESC,e.id DESC LIMIT 20 OFFSET $5"
    ))
    .bind(brain)
    .bind(query.binding_id)
    .bind(query.kind)
    .bind(query.device_id)
    .bind(offset)
    .fetch_all(&mut *tx)
    .await?;
    let mut items = Vec::new();
    for row in rows {
        let source_available = row.state == "accepted"
            && row.artifact_id.is_some()
            && artifacts::availability(
                &state.config.artifact_dir,
                brain,
                row.artifact_id,
                row.byte_length.unwrap_or_default(),
            )
            .await
                == "retained";
        items.push(CaptureEventView {
            processing: if row.state == "accepted" {
                row.processing.clone()
            } else {
                None
            },
            learning: if row.state == "accepted" {
                row.learning.as_ref().map(|v| v.0.clone())
            } else {
                None
            },
            device_id: row.device_id,
            agent_name: row.agent_name.clone(),
            user_name: row.user_name.clone(),
            receipt: row.receipt(),
            host: row.host,
            host_version: row.host_version,
            operation_id: row.operation_id,
            managed_call_id: row.managed_call_id,
            selection: row.selection.0,
            event: if row.state == "accepted" {
                row.metadata.map(|v| v.0)
            } else {
                None
            },
            source_available,
        });
    }
    tx.commit().await?;
    Ok(Json(CaptureEventPage {
        items,
        total,
        offset,
    }))
}

/// Automatic extraction runs under the standing policy actor, which can differ
/// from the device's actor. Published evidence carries its original selection.
pub(crate) async fn source_selection(
    tx: &mut Tx<'_>,
    brain: Uuid,
    source: Uuid,
) -> Result<ScopeSelection> {
    let selection: Option<SqlJson<ScopeSelection>> = sqlx::query_scalar("SELECT coalesce(a.selection,i.selection,b.selection) FROM source_versions v LEFT JOIN automatic_support_excerpts a ON a.version_id=v.id AND a.brain_id=v.brain_id AND a.privacy_state='active' LEFT JOIN source_import_scopes i ON i.version_id=v.id AND i.brain_id=v.brain_id LEFT JOIN capture_events e ON e.source_id=v.source_id AND e.brain_id=v.brain_id LEFT JOIN capture_bindings b ON b.id=e.binding_id WHERE v.brain_id=$1 AND v.id=$2 AND coalesce(a.selection,i.selection,b.selection) IS NOT NULL LIMIT 1")
        .bind(brain).bind(source).fetch_optional(&mut **tx).await?;
    Ok(selection.map(|s| s.0).unwrap_or_default())
}
