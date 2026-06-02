use super::*;

pub(super) async fn list(tx: &mut Tx<'_>, id: Uuid) -> Result<Vec<McpGrant>> {
    let values:Vec<DbJson<McpGrant>>=sqlx::query_scalar("SELECT jsonb_build_object('id',g.id,'account_id',g.account_id,'username',a.username,'issuer',g.issuer,'group_name',g.group_name,'rights',jsonb_build_object('use_profile',g.can_use,'manage',g.can_manage,'share',g.can_share)) FROM mcp_profile_grants g LEFT JOIN accounts a ON a.id=g.account_id WHERE g.profile_id=$1 ORDER BY coalesce(a.username,g.group_name),g.id")
        .bind(id).fetch_all(&mut **tx).await?;
    Ok(values.into_iter().map(|v| v.0).collect())
}
pub(super) async fn members(
    tx: &mut Tx<'_>,
    id: Uuid,
    affected: Option<Uuid>,
) -> Result<Vec<McpEffectiveMember>> {
    let values: Vec<DbJson<McpEffectiveMember>> = sqlx::query_scalar(include_str!("members.sql"))
        .bind(id)
        .bind(affected)
        .fetch_all(&mut **tx)
        .await?;
    if values.len() > 1000 {
        return Err(capacity());
    }
    Ok(values.into_iter().map(|v| v.0).collect())
}
async fn result(
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    id: Uuid,
    affected: Option<Uuid>,
) -> Result<McpGrantChange> {
    let rights = profiles::effective(tx, id, auth.user.id).await?;
    let profile = if rights.use_profile || rights.manage || rights.share {
        Some(profiles::detail(tx, brain, id, affected).await?)
    } else {
        None
    };
    Ok(McpGrantChange { rights, profile })
}
#[utoipa::path(put,path="/api/brains/{brain}/mcp/profiles/{id}/grants",operation_id="setMcpProfileGrant",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=McpGrantInput,responses((status=200,body=McpGrantChange)))]
pub async fn grant(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<McpGrantInput>,
) -> Result<Json<McpGrantChange>> {
    let mut tx = transaction(&state, &auth, brain, true).await?;
    let profile = profiles::load(&mut tx, brain, id).await?;
    if !profile.rights.share {
        return Err(denied());
    }
    let enabled = input.rights.use_profile || input.rights.manage || input.rights.share;
    let (account, issuer, group_name) = match (&input.username, &input.group_name) {
        (Some(username), None) => {
            text(username, 120, false)?;
            let row: Option<(Uuid, bool)> =
                sqlx::query_as("SELECT id,enabled FROM accounts WHERE username=$1")
                    .bind(username.trim())
                    .fetch_optional(&mut *tx)
                    .await?;
            let (account, active) =
                row.ok_or_else(|| Error::invalid("Choose an enrolled account by exact username."))?;
            if enabled && !active {
                return Err(Error::invalid(
                    "Enable this account before assigning profile rights.",
                ));
            }
            (Some(account), None, None)
        }
        (None, Some(group)) => {
            text(group, 200, false)?;
            let issuer = state
                .config
                .oidc
                .as_ref()
                .ok_or_else(|| {
                    Error::invalid(
                        "Configure the organization identity provider before group mappings.",
                    )
                })?
                .issuer
                .clone();
            (None, Some(issuer), Some(group.clone()))
        }
        _ => {
            return Err(Error::invalid(
                "Choose exactly one enrolled username or organization group.",
            ));
        }
    };
    let existing:Option<Uuid>=sqlx::query_scalar("SELECT id FROM mcp_profile_grants WHERE profile_id=$1 AND account_id IS NOT DISTINCT FROM $2 AND issuer IS NOT DISTINCT FROM $3 AND group_name IS NOT DISTINCT FROM $4")
        .bind(id).bind(account).bind(&issuer).bind(&group_name).fetch_optional(&mut *tx).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM mcp_profile_grants WHERE profile_id=$1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if enabled && existing.is_none() && count >= 100 {
        return Err(capacity());
    }
    // Record the command while the grant authorizing it is still present.
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "mcp.profile.grant",
        id,
        if enabled { "assigned" } else { "removed" },
    )
    .await?;
    if enabled {
        sqlx::query("INSERT INTO mcp_profile_grants(id,brain_id,profile_id,account_id,issuer,group_name,can_use,can_manage,can_share) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(id) DO UPDATE SET can_use=excluded.can_use,can_manage=excluded.can_manage,can_share=excluded.can_share")
            .bind(existing.unwrap_or_else(Uuid::new_v4)).bind(brain).bind(id).bind(account).bind(&issuer).bind(&group_name).bind(input.rights.use_profile).bind(input.rights.manage).bind(input.rights.share).execute(&mut *tx).await.map_err(mutation_error)?;
    } else if let Some(grant) = existing {
        sqlx::query("DELETE FROM mcp_profile_grants WHERE id=$1 AND profile_id=$2")
            .bind(grant)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    let response = result(&mut tx, &auth, brain, id, account).await?;
    tx.commit().await?;
    Ok(Json(response))
}
#[utoipa::path(delete,path="/api/brains/{brain}/mcp/profiles/{id}/grants/{grant}",operation_id="removeMcpProfileGrant",params(("brain"=Uuid,Path),("id"=Uuid,Path),("grant"=Uuid,Path)),responses((status=200,body=McpGrantChange)))]
pub async fn remove_grant(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id, grant)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<McpGrantChange>> {
    let mut tx = transaction(&state, &auth, brain, true).await?;
    let profile = profiles::load(&mut tx, brain, id).await?;
    if !profile.rights.share {
        return Err(denied());
    }
    let account: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT account_id FROM mcp_profile_grants WHERE id=$1 AND profile_id=$2 AND brain_id=$3",
    )
    .bind(grant)
    .bind(id)
    .bind(brain)
    .fetch_optional(&mut *tx)
    .await?;
    let account = account.ok_or_else(Error::missing)?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "mcp.profile.grant",
        id,
        "removed",
    )
    .await?;
    sqlx::query("DELETE FROM mcp_profile_grants WHERE id=$1 AND profile_id=$2")
        .bind(grant)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let response = result(&mut tx, &auth, brain, id, account).await?;
    tx.commit().await?;
    Ok(Json(response))
}
