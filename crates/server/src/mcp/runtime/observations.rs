//! The coordinator produces observations; canonical capture owns the evidence.
//! This path never calls a connector or a model, including on retry.
use super::*;

#[derive(Deserialize)]
struct Descriptor {
    id: Uuid,
    call_id: Uuid,
    stage: String,
    outcome: String,
    code: Option<String>,
    captured_at: DateTime<Utc>,
    admission_policy: CapturePolicy,
    current_policy: Value,
    tool_name: String,
    client_session_id: Uuid,
    operation_id: Option<Uuid>,
    target: Value,
    selection: Value,
    arguments: Option<Value>,
    result: Option<Value>,
    source_version_id: Option<Uuid>,
    receipt_call_id: Option<Uuid>,
}
fn prepare(value: Value, secrets: &[String]) -> Result<CapturedHook> {
    let d: Descriptor = serde_json::from_value(value)
        .map_err(|_| Error::invalid("The canonical observation descriptor is unavailable."))?;
    let current = serde_json::from_value(d.current_policy).unwrap_or_default();
    let has_content =
        d.result.is_some() || (d.stage == "evidence" && d.source_version_id.is_some());
    let mut coverage = vec![];
    if d.outcome == "unknown" {
        coverage.push("outcome_unknown".into());
    }
    if matches!(d.outcome.as_str(), "succeeded" | "tool_error") && !has_content {
        coverage.push("receipt_payload_unavailable".into());
    }
    let event = CapturedHook {
        host_event: match d.stage.as_str() {
            "terminal" => "ManagedTool",
            "evidence" => "ManagedResolution",
            _ => "ManagedReceipt",
        }
        .into(),
        host_session_id: d.client_session_id.to_string(),
        turn_id: d.operation_id.map(|id| id.to_string()),
        agent_id: None,
        tool_use_id: Some(d.call_id.to_string()),
        tool_name: Some(d.tool_name),
        kind: "tool_result".into(),
        outcome: d.outcome.clone(),
        content: None,
        coverage,
        captured_at: d.captured_at,
    };
    let envelope = json!({"provenance":"reported_tool_observation","observation_id":d.id,
        "call_id":d.call_id,"stage":d.stage,"outcome":d.outcome,"code":d.code,
        "captured_at":d.captured_at,"operation_id":d.operation_id,"selection":d.selection,
        "target":d.target,"arguments":d.arguments,"result":d.result,
        "source_version_id":d.source_version_id,"receipt_call_id":d.receipt_call_id});
    let mut event =
        prepare_managed_capture(event, &envelope, &d.admission_policy, &current, secrets)
            .map_err(|_| Error::invalid("The canonical observation cannot be normalized."))?;
    if !has_content {
        event.content = None;
    }
    Ok(event)
}

/// Runs in the receipt transaction, before either the completion or its intent
/// becomes visible. The SQL descriptor is canonical, including parent receipts.
pub(super) async fn completed(
    state: &AppState,
    tx: &mut Tx<'_>,
    reference: &str,
    input: &McpCompletion,
) -> Result<()> {
    let descriptors: Vec<Value> =
        sqlx::query_scalar("SELECT * FROM recollect_mcp_observation_prepare($1,$2,$3,$4)")
            .bind(reference)
            .bind(input.attempt.epoch)
            .bind(input.attempt.call_id)
            .bind(input.attempt.attempt_token)
            .fetch_all(&mut **tx)
            .await?;
    for value in descriptors {
        let id = serde_json::from_value::<Uuid>(value["id"].clone())
            .map_err(|_| Error::invalid("The observation identity is unavailable."))?;
        let event = prepare(value, &publication::configured_secrets(state))?;
        let stored: bool =
            sqlx::query_scalar("SELECT recollect_mcp_observation_store($1,$2,$3,$4,$5,$6)")
                .bind(reference)
                .bind(input.attempt.epoch)
                .bind(input.attempt.call_id)
                .bind(input.attempt.attempt_token)
                .bind(id)
                .bind(DbJson(event))
                .fetch_one(&mut **tx)
                .await?;
        if !stored {
            return Err(runtime_conflict(
                "mcp_observation_unavailable",
                "The observation intent could not be retained.",
            ));
        }
    }
    Ok(())
}

pub(super) async fn page(tx: &mut Tx<'_>, call: Uuid, offset: i64) -> Result<McpObservationPage> {
    let total = sqlx::query_scalar("SELECT count(*) FROM mcp_observation_outbox WHERE call_id=$1")
        .bind(call)
        .fetch_one(&mut **tx)
        .await?;
    let items: Vec<DbJson<McpObservation>> = sqlx::query_scalar("SELECT jsonb_build_object(
        'id',o.id,'call_id',o.call_id,'stage',o.stage,'outcome',o.outcome,'code',o.code,
        'state',CASE WHEN e.state IN ('expired','removed') THEN e.state
          WHEN o.state IN ('pending','error','published','filtered') AND deadline.expires_at<=clock_timestamp() THEN 'expired' ELSE o.state END,
        'captured_at',o.captured_at,'published_at',o.published_at,
        'expires_at',deadline.expires_at,
        'source_id',e.source_id,'source_version_id',e.source_version_id,
        'coverage',CASE WHEN deadline.expires_at>clock_timestamp() AND o.state NOT IN ('expired','removed') AND coalesce(e.state,'accepted')='accepted' THEN o.coverage ELSE '[]'::jsonb END,
        'attempts',o.attempts,'next_attempt_at',CASE WHEN o.state IN ('pending','error') THEN o.next_attempt_at END,'error_code',o.error_code)
        FROM mcp_observation_outbox o LEFT JOIN capture_events e ON e.id=o.id
        CROSS JOIN LATERAL (SELECT CASE WHEN e.state='accepted' THEN recollect_retention_deadline(o.brain_id,'tool_output',o.captured_at)
          WHEN e.id IS NOT NULL THEN e.expires_at ELSE least(o.expires_at,recollect_retention_deadline(o.brain_id,'tool_output',o.captured_at)) END expires_at) deadline
        WHERE o.call_id=$1 ORDER BY o.captured_at,o.id LIMIT 20 OFFSET $2")
        .bind(call).bind(offset).fetch_all(&mut **tx).await?;
    Ok(McpObservationPage {
        total,
        offset,
        items: items.into_iter().map(|v| v.0).collect(),
    })
}
#[utoipa::path(get,path="/api/brains/{brain}/mcp/calls/{id}/observations",operation_id="mcpObservations",params(("brain"=Uuid,Path),("id"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=McpObservationPage)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(query): Query<publication::Page>,
) -> Result<Json<McpObservationPage>> {
    let offset = publication::offset(&query)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    load_call(&mut tx, brain, id, false).await?;
    let result = page(&mut tx, id, offset).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/brains/{brain}/capture/managed",operation_id="managedCaptureStatus",params(("brain"=Uuid,Path)),responses((status=200,body=McpObservationStatus)))]
pub async fn status(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<McpObservationStatus>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let result: DbJson<McpObservationStatus> = sqlx::query_scalar("SELECT jsonb_build_object(
        'pending',count(*) FILTER(WHERE o.state='pending'), 'errors',count(*) FILTER(WHERE o.state='error'),
        'published',count(*) FILTER(WHERE o.state='published'), 'filtered',count(*) FILTER(WHERE o.state='filtered'),
        'skipped',count(*) FILTER(WHERE o.state='skipped'), 'unknown',count(*) FILTER(WHERE o.outcome='unknown'),
        'last_publication',max(o.published_at),'oldest_pending',min(o.captured_at) FILTER(WHERE o.state IN ('pending','error')))
        FROM mcp_observation_outbox o LEFT JOIN capture_events e ON e.id=o.id WHERE o.brain_id=$1
          AND o.state NOT IN ('expired','removed') AND coalesce(e.state,'accepted')='accepted'
          AND CASE WHEN e.state='accepted' THEN recollect_retention_deadline(o.brain_id,'tool_output',o.captured_at)
            ELSE least(o.expires_at,recollect_retention_deadline(o.brain_id,'tool_output',o.captured_at)) END>clock_timestamp()")
        .bind(brain).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(result.0))
}

#[derive(sqlx::FromRow)]
struct Candidate {
    id: Uuid,
    brain_id: Uuid,
    actor_id: Uuid,
    device_id: Option<Uuid>,
}
#[derive(sqlx::FromRow)]
struct Pending {
    call_id: Uuid,
    expires_at: DateTime<Utc>,
    admission_policy: DbJson<CapturePolicy>,
    event: Option<DbJson<CapturedHook>>,
    descriptor: Value,
    selection: DbJson<ScopeSelection>,
    fenced: bool,
    evidence_available: bool,
}
async fn publish(state: &AppState, item: &Candidate, lease: Uuid) -> Result<()> {
    let mut tx = db::device_tx(&state.pool, item.actor_id, item.device_id).await?;
    db::require_writer(&mut tx, item.brain_id).await?;
    require_open(&mut tx, item.brain_id).await?;
    let pending: Pending = sqlx::query_as("SELECT o.call_id,
        least(o.expires_at,recollect_retention_deadline(o.brain_id,'tool_output',o.captured_at)) expires_at,
        o.admission_policy,o.event,b.selection,recollect_mcp_observation_fenced(o.brain_id,o.call_id) fenced,
        (o.stage<>'evidence' OR EXISTS(SELECT 1 FROM source_versions v WHERE v.id=r.source_version_id
          AND v.brain_id=o.brain_id AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active')) evidence_available,
        to_jsonb(o)||jsonb_build_object('tool_name',c.tool_name,'client_session_id',c.client_session_id,
          'operation_id',c.operation_id,'target',b.managed_target,'selection',b.selection,
          'current_policy',coalesce(p.policy,'{}'::jsonb),'arguments',NULL,'result',NULL,
          'source_version_id',r.source_version_id,'receipt_call_id',r.receipt_call_id) descriptor
        FROM mcp_observation_outbox o JOIN mcp_calls c ON c.id=o.call_id JOIN capture_bindings b ON b.id=c.id
        LEFT JOIN capture_policies p ON p.brain_id=o.brain_id LEFT JOIN mcp_call_resolutions r ON r.id=o.id
        WHERE o.id=$1 AND o.lease_token=$2 AND o.state IN ('pending','error') AND o.lease_until>clock_timestamp()
        FOR UPDATE OF o")
        .bind(item.id).bind(lease).fetch_optional(&mut *tx).await?.ok_or_else(Error::missing)?;
    if pending.fenced {
        return Err(runtime_conflict(
            "observation_removed",
            "This call's evidence was erased.",
        ));
    }
    if !pending.evidence_available {
        return Err(runtime_conflict(
            "observation_evidence_unavailable",
            "The resolution's retained evidence is unavailable.",
        ));
    }
    if pending.expires_at <= Utc::now() {
        return Err(runtime_conflict(
            "observation_expired",
            "This observation has expired.",
        ));
    }
    let call = load_call(&mut tx, item.brain_id, pending.call_id, false).await?;
    if !can_use(&mut tx, call.profile_id).await? {
        return Err(denied());
    }
    if !workspace::selection_valid(&mut tx, item.brain_id, &pending.selection.0).await? {
        return Err(runtime_conflict(
            "capture_scope_unavailable",
            "The original observation scope is unavailable.",
        ));
    }
    let current = crate::capture::current(&mut tx, item.brain_id)
        .await?
        .policy;
    let secrets = publication::configured_secrets(state);
    let mut event = match pending.event {
        Some(event) => event.0,
        None => prepare(pending.descriptor, &secrets)?,
    };
    if !current.enabled || !current.managed_tools {
        event.content = None;
        event.coverage.push("excluded_event_kind".into());
    }
    sanitize_captured_event(&mut event, &pending.admission_policy.0, &secrets)
        .and_then(|_| sanitize_captured_event(&mut event, &current, &secrets))
        .map_err(|_| Error::invalid("The retained observation cannot be published."))?;
    let has_content = event.content.is_some();
    let mut metadata = event.clone();
    metadata.content = None;
    sqlx::query("INSERT INTO capture_events(id,brain_id,binding_id,metadata,admission_policy,retention_class,captured_at,expires_at)
        VALUES($1,$2,$3,$4,$5,'tool_output',$6,$7)")
        .bind(item.id).bind(item.brain_id).bind(pending.call_id).bind(DbJson(metadata))
        .bind(&pending.admission_policy).bind(event.captured_at).bind(pending.expires_at).execute(&mut *tx).await?;
    if has_content {
        let (source, version) = crate::evidence::capture_source(
            state,
            &mut tx,
            item.brain_id,
            item.actor_id,
            item.id,
            "managed_mcp",
            &event,
        )
        .await?;
        sqlx::query("UPDATE capture_events SET source_id=$2,source_version_id=$3 WHERE id=$1")
            .bind(item.id)
            .bind(source)
            .bind(version)
            .execute(&mut *tx)
            .await?;
    }
    let disposition = if has_content { "published" } else { "filtered" };
    sqlx::query("UPDATE mcp_observation_outbox SET event=NULL,admission_policy=NULL,state=$3,coverage=$4,
        published_at=clock_timestamp(),lease_token=NULL,lease_until=NULL,error_code=NULL WHERE id=$1 AND lease_token=$2")
        .bind(item.id).bind(lease).bind(disposition).bind(DbJson(&event.coverage)).execute(&mut *tx).await?;
    db::audit(
        &mut tx,
        item.actor_id,
        item.brain_id,
        "capture.publish",
        item.id,
        disposition,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
/// Twenty bounded candidates per independent pass. Failure persists a safe code;
/// publication retries never extend the content deadline or dispatch tools.
pub async fn run_once(state: &AppState) -> Result<usize> {
    let mut processed = 0;
    for _ in 0..20 {
        let lease = Uuid::new_v4();
        let Some(item) =
            sqlx::query_as::<_, Candidate>("SELECT * FROM recollect_mcp_observation_next($1)")
                .bind(lease)
                .fetch_optional(&state.pool)
                .await?
        else {
            break;
        };
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            publish(state, &item, lease),
        )
        .await;
        let error = match result {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(e),
            Err(_) => Some(Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "publication_timeout",
                "Observation publication timed out.",
            )),
        };
        if let Some(error) = error {
            let disposition = match error.1 {
                "observation_removed" => "removed",
                "observation_expired" => "expired",
                _ if error.0.is_client_error() && error.0 != StatusCode::TOO_MANY_REQUESTS => {
                    "skipped"
                }
                _ => "error",
            };
            sqlx::query("SELECT recollect_mcp_observation_finish($1,$2,$3,$4)")
                .bind(item.id)
                .bind(lease)
                .bind(disposition)
                .bind(error.1)
                .execute(&state.pool)
                .await?;
        }
        processed += 1;
    }
    Ok(processed)
}
