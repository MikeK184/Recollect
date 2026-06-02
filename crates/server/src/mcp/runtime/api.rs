use super::*;

#[utoipa::path(get,path="/api/brains/{brain}/mcp/runtime",operation_id="mcpRuntime",params(("brain"=Uuid,Path)),responses((status=200,body=McpRuntimeStatus)))]
pub async fn status(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<McpRuntimeStatus>> {
    let mut tx = execution_tx(&state.pool, auth.user.id, auth.device_id, brain).await?;
    let instances=sqlx::query_scalar::<_,DbJson<McpInstance>>("SELECT jsonb_build_object('id',id,'profile_id',profile_id,'connection_id',connection_id,'actor_id',actor_id,'device_id',device_id,'client_session_id',client_session_id,'runner_reference',runner_reference,'state',CASE WHEN state IN ('starting','ready','draining') AND NOT recollect_mcp_runner_alive(runner_reference,runner_epoch) THEN 'lost' ELSE state END,'active_calls',active_calls,'idle_seconds',idle_seconds,'updated_at',updated_at) FROM mcp_instances WHERE brain_id=$1 ORDER BY (state IN ('starting','ready','draining')) DESC,updated_at DESC,id DESC LIMIT 50")
        .bind(brain).fetch_all(&mut *tx).await?.into_iter().map(|v|v.0).collect();
    let references:Vec<String>=sqlx::query_scalar("SELECT DISTINCT CASE WHEN c.placement='central' THEN 'central' ELSE c.runner_reference END FROM mcp_connections c WHERE c.brain_id=$1 AND (recollect_role(c.brain_id)='admin' OR EXISTS(SELECT 1 FROM mcp_profile_connections m WHERE m.connection_id=c.id AND (recollect_mcp_can(m.profile_id,'use') OR recollect_mcp_can(m.profile_id,'manage') OR recollect_mcp_can(m.profile_id,'share')))) ORDER BY 1 LIMIT 100")
        .bind(brain).fetch_all(&mut *tx).await?;
    let mut runners = Vec::with_capacity(references.len());
    for reference in references {
        if let Some(value) = sqlx::query_scalar::<_, Option<DbJson<McpRunnerStatus>>>(
            "SELECT recollect_mcp_runner_status($1,$2)",
        )
        .bind(brain)
        .bind(reference)
        .fetch_one(&mut *tx)
        .await?
        {
            runners.push(value.0);
        }
    }
    tx.commit().await?;
    Ok(Json(McpRuntimeStatus { instances, runners }))
}

#[utoipa::path(post,path="/api/brains/{brain}/mcp/calls",operation_id="submitMcpCall",params(("brain"=Uuid,Path)),request_body=McpCallInput,responses((status=200,body=McpCall)))]
pub async fn submit(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<McpCallInput>,
) -> Result<Json<McpCall>> {
    let mut tx = execution_tx(&state.pool, auth.user.id, auth.device_id, brain).await?;
    let id = admit(&mut tx, brain, auth.user.id, auth.device_id, &input, None).await?;
    let row = load_call(&mut tx, brain, id, false).await?;
    let result = dto(&mut tx, row, true).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/brains/{brain}/mcp/calls/{id}",operation_id="getMcpCall",params(("brain"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=McpCall)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<McpCall>> {
    let mut tx = execution_tx(&state.pool, auth.user.id, auth.device_id, brain).await?;
    let row = load_call(&mut tx, brain, id, false).await?;
    let result = dto(&mut tx, row, true).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub cursor: Option<Uuid>,
}
#[utoipa::path(get,path="/api/brains/{brain}/mcp/calls",operation_id="listMcpCalls",params(("brain"=Uuid,Path),("cursor"=Option<Uuid>,Query)),responses((status=200,body=McpCallPage)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(input): Query<History>,
) -> Result<Json<McpCallPage>> {
    let mut tx = execution_tx(&state.pool, auth.user.id, auth.device_id, brain).await?;
    if let Some(cursor) = input.cursor {
        load_call(&mut tx, brain, cursor, false).await?;
    }
    let mut rows:Vec<CallRow>=sqlx::query_as("SELECT * FROM mcp_calls WHERE brain_id=$1 AND ($2::uuid IS NULL OR (created_at,id)<(SELECT created_at,id FROM mcp_calls WHERE id=$2 AND brain_id=$1)) ORDER BY created_at DESC,id DESC LIMIT 51")
        .bind(brain).bind(input.cursor).fetch_all(&mut *tx).await?;
    let more = rows.len() > 50;
    rows.truncate(50);
    let next_cursor = if more {
        rows.last().map(|r| r.id)
    } else {
        None
    };
    let mut calls = Vec::with_capacity(rows.len());
    for row in rows {
        calls.push(dto(&mut tx, row, false).await?);
    }
    tx.commit().await?;
    Ok(Json(McpCallPage { calls, next_cursor }))
}
#[utoipa::path(post,path="/api/brains/{brain}/mcp/calls/{id}/cancel",operation_id="cancelMcpCall",params(("brain"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=McpCall)))]
pub async fn cancel(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<McpCall>> {
    let mut tx = execution_tx(&state.pool, auth.user.id, auth.device_id, brain).await?;
    let row = load_call(&mut tx, brain, id, true).await?;
    if row.actor_id != auth.user.id {
        require_admin(&mut tx, brain).await?;
    }
    if matches!(row.state.as_str(), "queued" | "starting" | "running") && !row.cancel_requested {
        sqlx::query("UPDATE mcp_calls SET cancel_requested=true,state=CASE WHEN state='queued' THEN 'cancelled' ELSE state END,code=CASE WHEN state='queued' THEN 'cancelled_before_dispatch' ELSE code END,completed_at=CASE WHEN state='queued' THEN clock_timestamp() ELSE completed_at END,payload_expires_at=CASE WHEN state='queued' THEN clock_timestamp()+interval '1 hour' ELSE payload_expires_at END WHERE id=$1")
            .bind(id).execute(&mut *tx).await?;
        db::audit(
            &mut tx,
            auth.user.id,
            brain,
            "mcp.cancel",
            id,
            if row.state == "queued" {
                "cancelled"
            } else {
                "requested"
            },
        )
        .await?;
    }
    let row = load_call(&mut tx, brain, id, false).await?;
    let result = dto(&mut tx, row, true).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/brains/{brain}/mcp/session/release",operation_id="releaseMcpSession",params(("brain"=Uuid,Path)),request_body=McpSessionRelease,responses((status=200,body=serde_json::Value)))]
pub async fn release(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<McpSessionRelease>,
) -> Result<Json<Value>> {
    let mut tx = execution_tx(&state.pool, auth.user.id, auth.device_id, brain).await?;
    sqlx::query("INSERT INTO mcp_session_releases(brain_id,actor_id,client_session_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
        .bind(brain).bind(auth.user.id).bind(input.client_session_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"released":true})))
}
