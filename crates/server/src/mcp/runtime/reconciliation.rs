use super::*;

#[utoipa::path(post,path="/api/brains/{brain}/mcp/calls/{id}/reconcile",operation_id="reconcileMcpCall",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=McpReconcileInput,responses((status=200,body=McpCall)))]
pub async fn reconcile(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<McpReconcileInput>,
) -> Result<Json<McpCall>> {
    let mut tx = execution_tx(&state.pool, auth.user.id, auth.device_id, brain).await?;
    let original = load_call(&mut tx, brain, id, true).await?;
    if !can_use(&mut tx, original.profile_id).await? {
        return Err(denied());
    }
    if original.state != "unknown" || original.reconciles_call_id.is_some() {
        return Err(runtime_conflict(
            "mcp_reconciliation_unavailable",
            "Only an original call with unknown completion supports receipt lookup.",
        ));
    }
    let connection = connection(&mut tx, brain, original.connection_id).await?;
    let definition = definitions::load(&mut tx, &connection.definition_key).await?;
    let policy = definition
        .manifest
        .receipt_policies
        .iter()
        .find(|p| p.tool_name == original.tool_name)
        .ok_or_else(|| {
            runtime_conflict(
                "mcp_receipt_unsupported",
                "This connector has no approved receipt lookup. Record retained evidence instead.",
            )
        })?;
    // The original operation scope is immutable. A different paired device or
    // principal cannot adopt it merely by knowing its ID.
    let call = McpCallInput {
        request_id: input.request_id,
        profile_id: original.profile_id,
        connection_id: original.connection_id,
        tool_name: policy.receipt_tool.clone(),
        arguments: json!({policy.receipt_id_argument.clone():original.id}),
        environment_id: original.environment_id,
        operation_id: original.operation_id,
        client_session_id: input.client_session_id,
        timeout_seconds: 30,
    };
    let id = admit(
        &mut tx,
        brain,
        auth.user.id,
        auth.device_id,
        &call,
        Some(&original),
    )
    .await?;
    let row = load_call(&mut tx, brain, id, false).await?;
    let result = dto(&mut tx, row, true).await?;
    tx.commit().await?;
    Ok(Json(result))
}

#[utoipa::path(post,path="/api/brains/{brain}/mcp/calls/{id}/resolve",operation_id="resolveMcpCall",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=McpResolveInput,responses((status=200,body=McpCall)))]
pub async fn resolve(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<McpResolveInput>,
) -> Result<Json<McpCall>> {
    text(&input.explanation, 2000, false)?;
    if !matches!(
        input.outcome.as_str(),
        "succeeded" | "failed" | "not_executed" | "unknown"
    ) || sanitize_capture_text(&input.explanation, &[]) != input.explanation
    {
        return Err(Error::invalid(
            "Record a supported observed outcome and a bounded non-secret explanation.",
        ));
    }
    let mut tx = execution_tx(&state.pool, auth.user.id, auth.device_id, brain).await?;
    require_open(&mut tx, brain).await?;
    let original = load_call(&mut tx, brain, id, true).await?;
    if !can_use(&mut tx, original.profile_id).await? {
        return Err(denied());
    }
    if original.actor_id != auth.user.id {
        require_admin(&mut tx, brain).await?;
    }
    if original.state != "unknown" {
        return Err(runtime_conflict(
            "mcp_reconciliation_unavailable",
            "This call does not have an unknown completion to reconcile.",
        ));
    }
    // Serialize one principal's reconciliation identities, including requests
    // aimed at different calls. A concurrent duplicate must produce a meaningful
    // replay/conflict response rather than a uniqueness error.
    sqlx::query("SELECT pg_advisory_xact_lock(73241023)")
        .execute(&mut *tx)
        .await?;
    let saved:Option<(Uuid,String,Option<String>,Option<Uuid>)>=sqlx::query_as("SELECT call_id,outcome,explanation,source_version_id FROM mcp_call_resolutions WHERE brain_id=$1 AND actor_id=$2 AND request_id=$3")
        .bind(brain).bind(auth.user.id).bind(input.request_id).fetch_optional(&mut *tx).await?;
    if let Some((call, outcome, explanation, source)) = saved {
        let fresh:bool=sqlx::query_scalar("SELECT coalesce(explanation_expires_at>clock_timestamp(),false) AND explanation IS NOT NULL AND EXISTS(SELECT 1 FROM source_versions v WHERE v.id=source_version_id AND v.brain_id=$1 AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active') FROM mcp_call_resolutions WHERE brain_id=$1 AND actor_id=$2 AND request_id=$3")
            .bind(brain).bind(auth.user.id).bind(input.request_id).fetch_one(&mut *tx).await?;
        if !fresh {
            return Err(runtime_conflict(
                "mcp_replay_expired",
                "This reconciliation's explanation expired. Read its retained metadata.",
            ));
        }
        if call != id
            || outcome != input.outcome
            || explanation.as_deref() != Some(input.explanation.as_str())
            || source != Some(input.source_version_id)
        {
            return Err(runtime_conflict(
                "mcp_request_conflict",
                "This reconciliation request ID already has different inputs.",
            ));
        }
    } else {
        let source:Option<(Option<Uuid>,i32,Option<DateTime<Utc>>)>=sqlx::query_as("SELECT artifact_id,byte_length,recollect_retention_deadline(brain_id,retention_class,created_at) FROM source_versions WHERE brain_id=$1 AND id=$2 AND recollect_content_state(brain_id,retention_class,privacy_state,created_at)='active'")
            .bind(brain).bind(input.source_version_id).fetch_optional(&mut *tx).await?;
        let Some((Some(artifact), length, source_deadline)) = source else {
            return Err(Error::invalid(
                "Choose an accessible retained source version as evidence.",
            ));
        };
        crate::artifacts::read(&state.config.artifact_dir, brain, artifact, length)
            .await
            .map_err(|_| Error::invalid("The selected evidence is missing or unreadable."))?;
        if source_deadline.is_some_and(|at| at <= Utc::now()) {
            return Err(Error::invalid("The selected evidence has expired."));
        }
        let expires = (Utc::now() + chrono::Duration::hours(1))
            .min(source_deadline.unwrap_or(DateTime::<Utc>::MAX_UTC));
        sqlx::query("INSERT INTO mcp_call_resolutions(id,call_id,brain_id,actor_id,request_id,kind,outcome,source_version_id,explanation,explanation_expires_at) VALUES($1,$2,$3,$4,$5,'evidence',$6,$7,$8,$9)")
            .bind(Uuid::new_v4()).bind(id).bind(brain).bind(auth.user.id).bind(input.request_id).bind(&input.outcome)
            .bind(input.source_version_id).bind(&input.explanation).bind(expires).execute(&mut *tx).await?;
        db::audit(
            &mut tx,
            auth.user.id,
            brain,
            "mcp.reconcile",
            id,
            "evidence",
        )
        .await?;
    }
    let row = load_call(&mut tx, brain, id, false).await?;
    let result = dto(&mut tx, row, true).await?;
    tx.commit().await?;
    Ok(Json(result))
}
