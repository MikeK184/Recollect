use super::*;

const PROFILE_SELECT: &str = "SELECT p.*, ARRAY(SELECT connection_id FROM mcp_profile_connections m WHERE m.profile_id=p.id ORDER BY connection_id) AS connection_ids, recollect_mcp_can(p.id,'use') AS can_use, recollect_mcp_can(p.id,'manage') AS can_manage, recollect_mcp_can(p.id,'share') AS can_share FROM mcp_profiles p";
#[derive(sqlx::FromRow)]
struct ProfileRow {
    id: Uuid,
    brain_id: Uuid,
    name: String,
    description: String,
    environment_id: Option<Uuid>,
    enabled: bool,
    revision: Uuid,
    updated_at: DateTime<Utc>,
    connection_ids: Vec<Uuid>,
    can_use: bool,
    can_manage: bool,
    can_share: bool,
}
impl From<ProfileRow> for McpProfile {
    fn from(row: ProfileRow) -> Self {
        Self {
            id: row.id,
            brain_id: row.brain_id,
            name: row.name,
            description: row.description,
            environment_id: row.environment_id,
            enabled: row.enabled,
            revision: row.revision,
            updated_at: row.updated_at,
            connection_ids: row.connection_ids,
            rights: McpRights {
                use_profile: row.can_use,
                manage: row.can_manage,
                share: row.can_share,
            },
        }
    }
}
pub(super) async fn list(tx: &mut Tx<'_>, brain: Uuid) -> Result<Vec<McpProfile>> {
    Ok(sqlx::query_as::<_, ProfileRow>(&format!(
        "{PROFILE_SELECT} WHERE p.brain_id=$1 ORDER BY lower(p.name),p.id"
    ))
    .bind(brain)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(Into::into)
    .collect())
}
pub(super) async fn load(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<McpProfile> {
    sqlx::query_as::<_, ProfileRow>(&format!("{PROFILE_SELECT} WHERE p.brain_id=$1 AND p.id=$2"))
        .bind(brain)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .map(Into::into)
        .ok_or_else(Error::missing)
}
pub(super) async fn effective(tx: &mut Tx<'_>, id: Uuid, principal: Uuid) -> Result<McpRights> {
    let (use_profile,manage,share):(bool,bool,bool)=sqlx::query_as("SELECT recollect_mcp_account_can($1,$2,'use'),recollect_mcp_account_can($1,$2,'manage'),recollect_mcp_account_can($1,$2,'share')")
        .bind(id).bind(principal).fetch_one(&mut **tx).await?;
    Ok(McpRights {
        use_profile,
        manage,
        share,
    })
}
pub(super) async fn detail(
    tx: &mut Tx<'_>,
    brain: Uuid,
    id: Uuid,
    affected: Option<Uuid>,
) -> Result<McpProfileDetail> {
    let profile = load(tx, brain, id).await?;
    let (grants, effective_members) = if profile.rights.share {
        (
            Some(grants::list(tx, id).await?),
            grants::members(tx, id, affected).await?,
        )
    } else {
        (None, vec![])
    };
    Ok(McpProfileDetail {
        profile,
        grants,
        effective_members,
    })
}
#[utoipa::path(get,path="/api/brains/{brain}/mcp/profiles/{id}",operation_id="mcpProfile",params(("brain"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=McpProfileDetail)))]
pub async fn get_profile(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<McpProfileDetail>> {
    let mut tx = transaction(&state, &auth, brain, false).await?;
    let result = detail(&mut tx, brain, id, None).await?;
    tx.commit().await?;
    Ok(Json(result))
}
async fn write_profile(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    id: Option<Uuid>,
    headers: HeaderMap,
    mut input: McpProfileInput,
) -> Result<Json<McpProfileDetail>> {
    input.name = input.name.trim().into();
    text(&input.name, 120, false)?;
    text(&input.description, 2000, true)?;
    input.connection_ids.sort();
    if input.connection_ids.len() > 20 || input.connection_ids.windows(2).any(|p| p[0] == p[1]) {
        return Err(Error::invalid("Choose at most 20 distinct connections."));
    }
    let key = commands::key(&headers)?;
    let mut tx = transaction(state, auth, brain, true).await?;
    if let Some(id) = id {
        let old = load(&mut tx, brain, id).await?;
        if !old.rights.manage {
            return Err(denied());
        }
        if input.base_revision != Some(old.revision) {
            return Err(conflict("This profile changed. Refresh it before saving."));
        }
    } else {
        require_admin(&mut tx, brain).await?;
        if input.base_revision.is_some() {
            return Err(Error::invalid("A new profile has no base revision."));
        }
        if let Some(saved) = commands::reserve::<McpProfileDetail>(
            &mut tx,
            key.as_deref(),
            &format!("mcp.profile.create:{brain}"),
            serde_json::to_value(&input).expect("profile serialization"),
        )
        .await?
        {
            tx.commit().await?;
            return Ok(Json(saved));
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM mcp_profiles WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
        if count >= 100 {
            return Err(capacity());
        }
    }
    environment(&mut tx, brain, input.environment_id).await?;
    for connection_id in &input.connection_ids {
        let connection = connection(&mut tx, brain, *connection_id).await?;
        if connection.environment_id.is_some() && connection.environment_id != input.environment_id
        {
            return Err(conflict(
                "Every connection must be Brain-wide or match this profile's environment.",
            ));
        }
    }
    let existing = id.is_some();
    let id = id.unwrap_or_else(Uuid::new_v4);
    let revision = Uuid::new_v4();
    if existing {
        sqlx::query("UPDATE mcp_profiles SET name=$3,description=$4,environment_id=$5,enabled=$6,revision=$7,updated_at=clock_timestamp() WHERE id=$1 AND brain_id=$2")
            .bind(id).bind(brain).bind(&input.name).bind(&input.description).bind(input.environment_id).bind(input.enabled).bind(revision).execute(&mut *tx).await.map_err(mutation_error)?;
    } else {
        sqlx::query("INSERT INTO mcp_profiles(id,brain_id,name,description,environment_id,enabled,revision,created_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(id).bind(brain).bind(&input.name).bind(&input.description).bind(input.environment_id).bind(input.enabled).bind(revision).bind(auth.user.id).execute(&mut *tx).await.map_err(mutation_error)?;
    }
    sqlx::query("DELETE FROM mcp_profile_connections WHERE profile_id=$1 AND brain_id=$2")
        .bind(id)
        .bind(brain)
        .execute(&mut *tx)
        .await?;
    for connection in &input.connection_ids {
        sqlx::query("INSERT INTO mcp_profile_connections(brain_id,profile_id,connection_id) VALUES($1,$2,$3)")
            .bind(brain).bind(id).bind(connection).execute(&mut *tx).await?;
    }
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "mcp.profile",
        id,
        if existing { "updated" } else { "created" },
    )
    .await?;
    let result = detail(&mut tx, brain, id, None).await?;
    if !existing {
        commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    }
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/brains/{brain}/mcp/profiles",operation_id="createMcpProfile",params(("brain"=Uuid,Path)),request_body=McpProfileInput,responses((status=200,body=McpProfileDetail)))]
pub async fn create_profile(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<McpProfileInput>,
) -> Result<Json<McpProfileDetail>> {
    write_profile(&state, &auth, brain, None, headers, input).await
}
#[utoipa::path(put,path="/api/brains/{brain}/mcp/profiles/{id}",operation_id="updateMcpProfile",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=McpProfileInput,responses((status=200,body=McpProfileDetail)))]
pub async fn update_profile(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<McpProfileInput>,
) -> Result<Json<McpProfileDetail>> {
    write_profile(&state, &auth, brain, Some(id), headers, input).await
}

#[utoipa::path(post,path="/api/brains/{brain}/mcp/discover",operation_id="discoverMcpTools",params(("brain"=Uuid,Path)),request_body=McpDiscoverRequest,responses((status=200,body=McpDiscovery)))]
pub async fn discover(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<McpDiscoverRequest>,
) -> Result<Json<McpDiscovery>> {
    let mut tx = transaction(&state, &auth, brain, false).await?;
    require_open(&mut tx, brain).await?;
    let profile = load(&mut tx, brain, input.profile_id).await?;
    if !profile.rights.use_profile {
        return Err(denied());
    }
    if !profile.enabled {
        return Err(conflict("This execution profile is disabled."));
    }
    environment(&mut tx, brain, input.environment_id).await?;
    if profile.environment_id.is_some() && profile.environment_id != input.environment_id {
        return Err(Error::invalid(
            "Select this execution profile's exact environment.",
        ));
    }
    if let Some(operation) = input.operation_id {
        let operation = workspace::bound_operation(&mut tx, brain, operation).await?;
        if !operation.scope_valid
            || operation.actor_id != auth.user.id
            || (auth.device_id.is_some() && operation.device_id != auth.device_id)
        {
            return Err(Error::missing());
        }
        if !matches!(operation.kind.as_str(), "context" | "tool")
            || operation.scope.selection.environment_id != input.environment_id
        {
            return Err(Error::invalid(
                "Use a context or tool operation bound to this exact environment.",
            ));
        }
    } else if auth.device_id.is_some() {
        return Err(Error::invalid(
            "A companion must supply its owned immutable context or tool operation.",
        ));
    }
    if input.offset > 1000 {
        return Err(Error::invalid("Discovery offset exceeds catalogue bounds."));
    }
    let mut tools = Vec::new();
    let mut unavailable = Vec::new();
    for id in profile.connection_ids {
        let row = connection(&mut tx, brain, id).await?;
        let def = definitions::load(&mut tx, &row.definition_key).await?;
        let summary = row.summary(&def);
        if summary.availability != "configured" {
            unavailable.push(McpUnavailableConnection {
                connection_id: id,
                name: row.name,
                reason: summary.availability,
            });
            continue;
        }
        // IDs and cached metadata only: no process, HTTP client or credential provider.
        for tool in def.manifest.0.tools {
            tools.push(McpDiscoveredTool {
                connection_id: id,
                connection_name: row.name.clone(),
                tool,
            });
        }
    }
    tools.sort_by(|a, b| {
        a.connection_id
            .cmp(&b.connection_id)
            .then_with(|| a.tool.name.cmp(&b.tool.name))
    });
    let total = tools.len();
    let next_offset = (input.offset.saturating_add(20) < total).then_some(input.offset + 20);
    let tools = tools.into_iter().skip(input.offset).take(20).collect();
    tx.commit().await?;
    Ok(Json(McpDiscovery {
        profile_id: input.profile_id,
        environment_id: input.environment_id,
        operation_id: input.operation_id,
        source: "approved_catalogue".into(),
        execution_state: "not_connected".into(),
        tools,
        unavailable_connections: unavailable,
        total,
        offset: input.offset,
        next_offset,
    }))
}
