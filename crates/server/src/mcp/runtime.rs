mod api;
pub mod central;
pub mod observations;
mod reconciliation;
pub use reconciliation::*;
pub mod runner;
pub use api::*;

use super::*;
use crate::publication;
use axum::extract::Query;
use serde::Deserialize;
use serde_json::json;

#[derive(sqlx::FromRow)]
pub(super) struct CallRow {
    id: Uuid,
    brain_id: Uuid,
    actor_id: Uuid,
    device_id: Option<Uuid>,
    request_id: Uuid,
    profile_id: Uuid,
    connection_id: Uuid,
    profile_revision: Uuid,
    connection_revision: Uuid,
    definition_revision: DateTime<Utc>,
    tool_name: String,
    environment_id: Option<Uuid>,
    operation_id: Option<Uuid>,
    scope: Option<DbJson<ScopeSnapshot>>,
    client_session_id: Uuid,
    runner_reference: String,
    state: String,
    code: Option<String>,
    timeout_seconds: i32,
    cancel_requested: bool,
    runner_epoch: Option<Uuid>,
    attempt_token: Option<Uuid>,
    instance_id: Option<Uuid>,
    lease_until: Option<DateTime<Utc>>,
    deadline: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    queue_expires_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    dispatched_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    payload_expires_at: Option<DateTime<Utc>>,
    reconciles_call_id: Option<Uuid>,
    capture_disposition: String,
}

fn runtime_conflict(code: &'static str, message: &'static str) -> Error {
    Error(StatusCode::CONFLICT, code, message)
}
async fn execution_tx<'a>(
    pool: &'a sqlx::PgPool,
    actor: Uuid,
    device: Option<Uuid>,
    brain: Uuid,
) -> Result<Tx<'a>> {
    let mut tx = db::device_tx(pool, actor, device).await?;
    db::require_role(&mut tx, brain, false).await?;
    definitions::lock(&mut tx).await?;
    Ok(tx)
}
async fn load_call(tx: &mut Tx<'_>, brain: Uuid, id: Uuid, lock: bool) -> Result<CallRow> {
    sqlx::query_as(if lock {
        "SELECT * FROM mcp_calls WHERE brain_id=$1 AND id=$2 FOR UPDATE"
    } else {
        "SELECT * FROM mcp_calls WHERE brain_id=$1 AND id=$2"
    })
    .bind(brain)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(Error::missing)
}
async fn can_use(tx: &mut Tx<'_>, profile: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar("SELECT recollect_mcp_can($1,'use')")
        .bind(profile)
        .fetch_one(&mut **tx)
        .await?)
}
async fn dto(tx: &mut Tx<'_>, row: CallRow, include_result: bool) -> Result<McpCall> {
    let permitted = can_use(tx, row.profile_id).await?;
    let expired = row.payload_expires_at.is_some_and(|at| at <= Utc::now());
    let output_access = if !permitted {
        "permission_required"
    } else if expired {
        "expired"
    } else if row.completed_at.is_none() {
        "pending"
    } else {
        "available"
    };
    let result = if include_result && permitted && !expired {
        sqlx::query_scalar::<_, Option<Value>>(
            "SELECT result FROM mcp_call_payloads WHERE call_id=$1",
        )
        .bind(row.id)
        .fetch_optional(&mut **tx)
        .await?
        .flatten()
    } else {
        None
    };
    let resolutions = if include_result && permitted {
        sqlx::query_scalar::<_,DbJson<McpResolution>>("SELECT (to_jsonb(r)-'call_id'-'brain_id'-'request_id') || jsonb_build_object('explanation',CASE WHEN r.explanation_expires_at>clock_timestamp() AND EXISTS(SELECT 1 FROM source_versions v WHERE v.id=r.source_version_id AND v.brain_id=r.brain_id AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active') THEN r.explanation END) FROM mcp_call_resolutions r WHERE call_id=$1 ORDER BY created_at DESC,id DESC LIMIT 50")
            .bind(row.id).fetch_all(&mut **tx).await?.into_iter().map(|v|v.0).collect()
    } else {
        vec![]
    };
    Ok(McpCall {
        id: row.id,
        request_id: row.request_id,
        brain_id: row.brain_id,
        actor_id: row.actor_id,
        device_id: row.device_id,
        profile_id: row.profile_id,
        connection_id: row.connection_id,
        tool_name: row.tool_name,
        environment_id: row.environment_id,
        operation_id: row.operation_id,
        scope: row.scope.map(|v| v.0),
        client_session_id: row.client_session_id,
        runner_reference: row.runner_reference,
        instance_id: row.instance_id,
        state: row.state,
        code: row.code,
        cancel_requested: row.cancel_requested,
        timeout_seconds: row.timeout_seconds as u32,
        created_at: row.created_at,
        started_at: row.started_at,
        dispatched_at: row.dispatched_at,
        completed_at: row.completed_at,
        payload_expires_at: row.payload_expires_at,
        output_access: output_access.into(),
        result,
        reconciles_call_id: row.reconciles_call_id,
        resolutions,
        capture_disposition: row.capture_disposition,
        observations: observations::page(tx, row.id, 0).await?.items,
    })
}

struct Authorized {
    profile: McpProfile,
    connection: ConnectionRow,
    definition: definitions::DefinitionRow,
    scope: Option<ScopeSnapshot>,
    runner_reference: String,
}
async fn authorize(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
    input: &McpCallInput,
) -> Result<Authorized> {
    require_open(tx, brain).await?;
    let profile = profiles::load(tx, brain, input.profile_id).await?;
    if !profile.rights.use_profile {
        return Err(denied());
    }
    if !profile.enabled || !profile.connection_ids.contains(&input.connection_id) {
        return Err(runtime_conflict(
            "mcp_profile_unavailable",
            "This enabled profile must contain the selected connection.",
        ));
    }
    environment(tx, brain, input.environment_id).await?;
    if profile.environment_id.is_some() && profile.environment_id != input.environment_id {
        return Err(Error::invalid(
            "Select the execution profile's exact environment.",
        ));
    }
    let scope = if let Some(operation_id) = input.operation_id {
        let operation = workspace::bound_operation(tx, brain, operation_id).await?;
        if !operation.scope_valid
            || operation.actor_id != actor
            || (device.is_some() && operation.device_id != device)
        {
            return Err(Error::missing());
        }
        if operation.kind != "tool"
            || operation.scope.selection.environment_id != input.environment_id
        {
            return Err(Error::invalid(
                "Use an owned immutable tool operation for the selected environment.",
            ));
        }
        Some(operation.scope)
    } else if device.is_some() {
        return Err(Error::invalid(
            "A companion call requires its owned immutable tool operation.",
        ));
    } else {
        None
    };
    let connection = connection(tx, brain, input.connection_id).await?;
    let definition = definitions::load(tx, &connection.definition_key).await?;
    if connection.summary(&definition).availability != "configured" {
        return Err(runtime_conflict(
            "mcp_connection_unavailable",
            "The approved connection must be enabled and valid.",
        ));
    }
    if connection.environment_id.is_some() && connection.environment_id != input.environment_id {
        return Err(Error::invalid("Select the connection's exact environment."));
    }
    let runner_reference = match connection.placement.as_str() {
        "central" => "central".into(),
        "local" => {
            let reference = connection.runner_reference.as_deref().unwrap_or_default();
            let runner = reference
                .strip_prefix("device:")
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or_else(|| {
                    Error::invalid("Local placement requires an exact device UUID reference.")
                })?;
            if reference != format!("device:{runner}") {
                return Err(Error::invalid("Use the canonical device UUID reference."));
            }
            let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM devices WHERE id=$1 AND account_id=$2 AND claimed AND revoked_at IS NULL AND expires_at>now())")
                .bind(runner).bind(actor).fetch_one(&mut **tx).await?;
            if !valid {
                return Err(runtime_conflict(
                    "mcp_runner_unavailable",
                    "The local runner must be an active device owned by the caller.",
                ));
            }
            reference.into()
        }
        "private" => {
            let reference = connection.runner_reference.as_deref().unwrap_or_default();
            private::require(tx, brain, reference).await?;
            reference.into()
        }
        _ => {
            return Err(runtime_conflict(
                "mcp_runner_unavailable",
                "The configured runner placement is unavailable.",
            ));
        }
    };
    let released:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM mcp_session_releases WHERE brain_id=$1 AND actor_id=$2 AND client_session_id=$3)")
        .bind(brain).bind(actor).bind(input.client_session_id).fetch_one(&mut **tx).await?;
    if released {
        return Err(runtime_conflict(
            "mcp_session_released",
            "Start a new caller session before dispatching more work.",
        ));
    }
    Ok(Authorized {
        profile,
        connection,
        definition,
        scope,
        runner_reference,
    })
}
fn arguments(
    input: &McpCallInput,
    definition: &McpDefinitionManifest,
    id: Uuid,
    receipt: bool,
) -> Result<Value> {
    if !(1..=3600).contains(&input.timeout_seconds)
        || !input.arguments.is_object()
        || serde_json::to_vec(&input.arguments).map_or(true, |v| v.len() > 32768)
        || !argument_tree(&input.arguments, 0)
        || sanitize_capture_value(&input.arguments, &[]) != input.arguments
    {
        return Err(Error::invalid(
            "Use bounded non-secret object arguments and a timeout from 1 to 3600 seconds.",
        ));
    }
    let tool = definition
        .tools
        .iter()
        .find(|t| t.name == input.tool_name)
        .ok_or_else(|| Error::invalid("Select an approved tool from this connection."))?;
    let mut arguments = input.arguments.clone();
    if !receipt
        && let Some(policy) = definition
            .receipt_policies
            .iter()
            .find(|p| p.tool_name == input.tool_name)
    {
        let map = arguments.as_object_mut().expect("validated object");
        if map.contains_key(&policy.operation_id_argument) {
            return Err(Error::invalid(
                "The approved receipt operation ID is reserved for the coordinator.",
            ));
        }
        map.insert(policy.operation_id_argument.clone(), json!(id));
    }
    if serde_json::to_vec(&arguments).map_or(true, |v| v.len() > 32768)
        || !definitions::validator(&tool.input_schema, true, 32768)?.is_valid(&arguments)
    {
        return Err(Error::invalid(
            "Arguments do not match the approved tool schema.",
        ));
    }
    Ok(arguments)
}
fn argument_tree(value: &Value, depth: usize) -> bool {
    depth <= 16
        && match value {
            Value::Object(m) => m.values().all(|v| argument_tree(v, depth + 1)),
            Value::Array(v) => v.iter().all(|v| argument_tree(v, depth + 1)),
            _ => true,
        }
}
fn unchanged(row: &CallRow, authorized: &Authorized) -> bool {
    row.profile_revision == authorized.profile.revision
        && row.connection_revision == authorized.connection.revision
        && row.definition_revision == authorized.definition.updated_at
        && row.runner_reference == authorized.runner_reference
        && serde_json::to_value(&row.scope).ok() == serde_json::to_value(&authorized.scope).ok()
}

async fn admit(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
    input: &McpCallInput,
    parent: Option<&CallRow>,
) -> Result<Uuid> {
    if !can_use(tx, input.profile_id).await? {
        return Err(denied());
    }
    let capacity: bool = sqlx::query_scalar("SELECT recollect_mcp_capacity()")
        .fetch_one(&mut **tx)
        .await?;
    if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM mcp_calls WHERE brain_id=$1 AND actor_id=$2 AND request_id=$3",
    )
    .bind(brain)
    .bind(actor)
    .bind(input.request_id)
    .fetch_optional(&mut **tx)
    .await?
    {
        let original: Option<Value> =
            sqlx::query_scalar("SELECT request FROM mcp_call_payloads WHERE call_id=$1")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await?;
        let Some(original) = original else {
            return Err(runtime_conflict(
                "mcp_replay_expired",
                "Arguments have expired. Read the existing call by ID; do not repeat the operation.",
            ));
        };
        let old = load_call(tx, brain, id, false).await?;
        if original != serde_json::to_value(input).expect("call input")
            || old.reconciles_call_id != parent.map(|p| p.id)
        {
            return Err(runtime_conflict(
                "mcp_request_conflict",
                "This request ID is already bound to different inputs.",
            ));
        }
        return Ok(id);
    }
    if !capacity {
        return Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "mcp_call_capacity",
            "Pending call capacity has been reached.",
        ));
    }
    let prepared = authorize(tx, brain, actor, device, input).await?;
    if parent.is_some_and(|old| !unchanged(old, &prepared)) {
        return Err(runtime_conflict(
            "mcp_configuration_changed",
            "The original target or configuration changed. Receipt lookup cannot be routed to a replacement.",
        ));
    }
    let id = Uuid::new_v4();
    let arguments = arguments(input, &prepared.definition.manifest, id, parent.is_some())?;
    let policy = crate::capture::current(tx, brain).await?.policy;
    let writer: bool = sqlx::query_scalar("SELECT recollect_role($1) IN ('writer','admin')")
        .bind(brain)
        .fetch_one(&mut **tx)
        .await?;
    let capture = parent.is_none()
        && writer
        && policy.enabled
        && policy.managed_tools
        && policy.kinds.iter().any(|k| k == "tool_result");
    let target = capture.then(|| sanitize_capture_value(&json!({
        "profile_id":input.profile_id,"profile_revision":prepared.profile.revision,
        "connection_id":input.connection_id,"connection_revision":prepared.connection.revision,
        "definition_key":prepared.connection.definition_key,"definition_revision":prepared.definition.updated_at,
        "transport":prepared.definition.manifest.transport,"placement":prepared.connection.placement,
        "target":prepared.connection.target,"configuration":prepared.connection.configuration,
        "runner_reference":prepared.runner_reference,"environment_id":input.environment_id,
        "actor_id":actor,"device_id":device
    }), &[]));
    let disposition = if capture {
        "eligible"
    } else if parent.is_some() {
        "receipt_lookup"
    } else if !writer {
        "knowledge_write_required"
    } else {
        "disabled"
    };
    sqlx::query("INSERT INTO mcp_calls(id,brain_id,actor_id,device_id,request_id,profile_id,connection_id,profile_revision,connection_revision,definition_revision,tool_name,environment_id,operation_id,scope,client_session_id,runner_reference,state,timeout_seconds,reconciles_call_id,capture_policy,capture_target,capture_disposition) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,'queued',$17,$18,$19,$20,$21)")
        .bind(id).bind(brain).bind(actor).bind(device).bind(input.request_id).bind(input.profile_id).bind(input.connection_id)
        .bind(prepared.profile.revision).bind(prepared.connection.revision).bind(prepared.definition.updated_at)
        .bind(&input.tool_name).bind(input.environment_id).bind(input.operation_id).bind(prepared.scope.map(DbJson))
        .bind(input.client_session_id).bind(&prepared.runner_reference).bind(input.timeout_seconds as i32).bind(parent.map(|p|p.id))
        .bind(capture.then_some(DbJson(&policy))).bind(target).bind(disposition)
        .execute(&mut **tx).await?;
    sqlx::query(
        "INSERT INTO mcp_call_payloads(call_id,brain_id,request,arguments) VALUES($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(brain)
    .bind(DbJson(input))
    .bind(arguments)
    .execute(&mut **tx)
    .await?;
    db::audit(
        tx,
        actor,
        brain,
        "mcp.admission",
        id,
        if parent.is_some() {
            "receipt_queued"
        } else {
            "queued"
        },
    )
    .await?;
    Ok(id)
}
