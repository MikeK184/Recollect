use super::*;

async fn get(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<McpPrivateRunner> {
    let row: DbJson<McpPrivateRunner> = sqlx::query_scalar("SELECT to_jsonb(p) || jsonb_build_object('eligible',recollect_mcp_private_eligible(p.brain_id,'private:'||p.id::text),'available',recollect_mcp_private_eligible(p.brain_id,'private:'||p.id::text) AND coalesce(r.lease_until>clock_timestamp(),false),'lease_until',r.lease_until) FROM mcp_private_runners p CROSS JOIN LATERAL (SELECT recollect_mcp_private_lease(p.id) AS lease_until) r WHERE p.brain_id=$1 AND p.id=$2")
        .bind(brain).bind(id).fetch_optional(&mut **tx).await?.ok_or_else(Error::missing)?;
    Ok(row.0)
}

pub(super) async fn require(tx: &mut Tx<'_>, brain: Uuid, reference: &str) -> Result<()> {
    let id = reference
        .strip_prefix("private:")
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| Error::invalid("Select a registered private runner UUID."))?;
    if reference != format!("private:{id}") {
        return Err(Error::invalid(
            "Use the canonical private runner UUID reference.",
        ));
    }
    let row = get(tx, brain, id).await?;
    if !row.eligible {
        return Err(Error(
            StatusCode::CONFLICT,
            "mcp_runner_unavailable",
            "This private runner is disabled or its paired administrator is no longer eligible.",
        ));
    }
    Ok(())
}

#[utoipa::path(get,path="/api/brains/{brain}/mcp/private-runners",operation_id="listPrivateMcpRunners",params(("brain"=Uuid,Path)),responses((status=200,body=Vec<McpPrivateRunner>)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<Vec<McpPrivateRunner>>> {
    let mut tx = transaction(&state, &auth, brain, false).await?;
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM mcp_private_runners WHERE brain_id=$1 ORDER BY lower(name),id LIMIT 32",
    )
    .bind(brain)
    .fetch_all(&mut *tx)
    .await?;
    let mut rows = Vec::new();
    for id in ids {
        rows.push(get(&mut tx, brain, id).await?);
    }
    tx.commit().await?;
    Ok(Json(rows))
}

#[utoipa::path(post,path="/api/brains/{brain}/mcp/private-runners",operation_id="createPrivateMcpRunner",params(("brain"=Uuid,Path)),request_body=McpPrivateRunnerInput,responses((status=200,body=McpPrivateRunner)))]
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<McpPrivateRunnerInput>,
) -> Result<Json<McpPrivateRunner>> {
    save(&state, &auth, brain, None, headers, input).await
}
#[utoipa::path(put,path="/api/brains/{brain}/mcp/private-runners/{id}",operation_id="updatePrivateMcpRunner",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=McpPrivateRunnerInput,responses((status=200,body=McpPrivateRunner)))]
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<McpPrivateRunnerInput>,
) -> Result<Json<McpPrivateRunner>> {
    save(&state, &auth, brain, Some(id), headers, input).await
}
async fn save(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    id: Option<Uuid>,
    headers: HeaderMap,
    mut input: McpPrivateRunnerInput,
) -> Result<Json<McpPrivateRunner>> {
    input.name = input.name.trim().into();
    text(&input.name, 120, false)?;
    let key = commands::key(&headers)?;
    let mut tx = transaction(state, auth, brain, true).await?;
    require_admin(&mut tx, brain).await?;
    if let Some(saved) = commands::reserve::<McpPrivateRunner>(
        &mut tx,
        key.as_deref(),
        &format!(
            "mcp.private.save:{brain}:{}",
            id.map(|id| id.to_string()).unwrap_or_default()
        ),
        serde_json::to_value(&input).expect("runner input"),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(saved));
    }
    if let Some(id) = id {
        let old = get(&mut tx, brain, id).await?;
        if input.base_revision != Some(old.revision) {
            return Err(conflict("The runner changed. Refresh before saving."));
        }
        if input.device_id != old.device_id {
            return Err(Error::invalid(
                "Register a new runner to change the paired device.",
            ));
        }
        sqlx::query("UPDATE mcp_private_runners SET name=$3,enabled=$4,revision=$5,updated_at=clock_timestamp() WHERE brain_id=$1 AND id=$2")
            .bind(brain).bind(id).bind(&input.name).bind(input.enabled).bind(Uuid::new_v4()).execute(&mut *tx).await.map_err(mutation_error)?;
    } else {
        if input.base_revision.is_some() {
            return Err(Error::invalid("A new runner has no base revision."));
        }
        let valid: Option<Uuid> = sqlx::query_scalar("SELECT id FROM devices WHERE id=$1 AND account_id=$2 AND claimed AND revoked_at IS NULL AND expires_at>clock_timestamp() FOR SHARE")
            .bind(input.device_id).bind(auth.user.id).fetch_optional(&mut *tx).await?;
        if valid.is_none() {
            return Err(Error::invalid("Select your own active paired device."));
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM mcp_private_runners WHERE brain_id=$1")
                .bind(brain)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 32 {
            return Err(capacity());
        }
    }
    let existing = id.is_some();
    let id = id.unwrap_or_else(Uuid::new_v4);
    if !existing {
        sqlx::query("INSERT INTO mcp_private_runners(id,brain_id,device_id,name,enabled,revision,created_by) VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(id).bind(brain).bind(input.device_id).bind(&input.name).bind(input.enabled).bind(Uuid::new_v4()).bind(auth.user.id)
            .execute(&mut *tx).await.map_err(mutation_error)?;
    }
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "mcp.private_runner",
        id,
        if existing { "updated" } else { "created" },
    )
    .await?;
    let saved = get(&mut tx, brain, id).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &saved).await?;
    tx.commit().await?;
    Ok(Json(saved))
}
