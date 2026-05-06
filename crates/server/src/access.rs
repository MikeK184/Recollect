use crate::{
    AppState,
    auth::Auth,
    db,
    error::{Error, Result},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use recollect_protocol::*;
use sqlx::{Postgres, Transaction, types::Json as DbJson};
use uuid::Uuid;

fn role(value: &str) -> Result<&str> {
    if !matches!(value, "reader" | "writer" | "admin") {
        return Err(Error::invalid("Choose reader, writer or admin."));
    }
    Ok(value)
}
async fn effective(
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    account: Uuid,
) -> Result<Json<AccessChange>> {
    let effective_role: Option<String> = sqlx::query_scalar("SELECT recollect_account_role($1,$2)")
        .bind(brain)
        .bind(account)
        .fetch_one(&mut **tx)
        .await?;
    Ok(Json(AccessChange { effective_role }))
}
#[utoipa::path(get,path="/api/brains/{id}/access",operation_id="brainAccess",params(("id"=Uuid,Path)),responses((status=200,body=BrainAccess)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<BrainAccess>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let rows:Vec<DbJson<EffectiveAccess>>=sqlx::query_scalar("SELECT jsonb_build_object('account',jsonb_build_object('id',a.id,'username',a.username,'enabled',a.enabled,'installation_owner',a.installation_owner,'auth_kind',a.auth_kind,'oidc_subject',a.oidc_subject),'owner',b.owner_id=a.id,'direct_role',d.role,'groups',coalesce((SELECT jsonb_agg(g.group_name ORDER BY g.group_name) FROM brain_group_grants g WHERE g.brain_id=b.id AND g.issuer=a.oidc_issuer AND g.group_name=ANY(a.oidc_groups) AND a.membership_until>now()),'[]'::jsonb),'effective_role',recollect_account_role(b.id,a.id),'membership_until',a.membership_until) FROM brains b CROSS JOIN accounts a LEFT JOIN brain_grants d ON d.brain_id=b.id AND d.account_id=a.id WHERE b.id=$1 AND (b.owner_id=a.id OR d.role IS NOT NULL OR EXISTS(SELECT 1 FROM brain_group_grants g WHERE g.brain_id=b.id AND g.issuer=a.oidc_issuer AND g.group_name=ANY(a.oidc_groups))) ORDER BY a.username LIMIT 1000")
        .bind(id).fetch_all(&mut *tx).await?;
    let groups:Vec<DbJson<GroupGrant>>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'group_name',group_name,'role',role) FROM brain_group_grants WHERE brain_id=$1 ORDER BY group_name LIMIT 1000").bind(id).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(BrainAccess {
        members: rows.into_iter().map(|r| r.0).collect(),
        group_grants: groups.into_iter().map(|r| r.0).collect(),
    }))
}
async fn grant(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    brain: Uuid,
    account: Uuid,
    value: Option<&str>,
) -> Result<Json<AccessChange>> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND enabled)")
            .bind(account)
            .fetch_one(&mut **tx)
            .await?;
    if value.is_some() && !exists {
        return Err(Error::invalid("Choose an enabled, enrolled account."));
    }
    // Audit while the authorizing grant is still present; the whole transaction commits together.
    db::audit(
        tx,
        actor,
        brain,
        "access.direct",
        account,
        if value.is_some() {
            "granted"
        } else {
            "removed"
        },
    )
    .await?;
    if let Some(value) = value {
        sqlx::query("INSERT INTO brain_grants(brain_id,account_id,role) VALUES($1,$2,$3) ON CONFLICT(brain_id,account_id) DO UPDATE SET role=excluded.role")
            .bind(brain).bind(account).bind(value).execute(&mut **tx).await?;
    } else {
        sqlx::query("DELETE FROM brain_grants WHERE brain_id=$1 AND account_id=$2")
            .bind(brain)
            .bind(account)
            .execute(&mut **tx)
            .await?;
    }
    effective(tx, brain, account).await
}
#[utoipa::path(put,path="/api/brains/{id}/grants/{account}",params(("id"=Uuid,Path),("account"=Uuid,Path)),request_body=GrantRequest,responses((status=200,body=AccessChange)))]
pub async fn set(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, account)): Path<(Uuid, Uuid)>,
    Json(input): Json<GrantRequest>,
) -> Result<Json<AccessChange>> {
    let role = role(&input.role)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let response = grant(&mut tx, auth.user.id, id, account, Some(role)).await?;
    tx.commit().await?;
    Ok(response)
}
#[utoipa::path(post,path="/api/brains/{id}/grants",params(("id"=Uuid,Path)),request_body=DirectGrantRequest,responses((status=200,body=AccessChange)))]
pub async fn by_name(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    Json(input): Json<DirectGrantRequest>,
) -> Result<Json<AccessChange>> {
    let role = role(&input.role)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let account: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM accounts WHERE username=$1 AND enabled")
            .bind(input.username.trim())
            .fetch_optional(&mut *tx)
            .await?;
    let response = grant(
        &mut tx,
        auth.user.id,
        id,
        account.ok_or_else(|| Error::invalid("No enabled account has that username."))?,
        Some(role),
    )
    .await?;
    tx.commit().await?;
    Ok(response)
}
#[utoipa::path(delete,path="/api/brains/{id}/grants/{account}",params(("id"=Uuid,Path),("account"=Uuid,Path)),responses((status=200,body=AccessChange)))]
pub async fn remove(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, account)): Path<(Uuid, Uuid)>,
) -> Result<Json<AccessChange>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let response = grant(&mut tx, auth.user.id, id, account, None).await?;
    tx.commit().await?;
    Ok(response)
}
#[utoipa::path(put,path="/api/brains/{id}/group-grants",params(("id"=Uuid,Path)),request_body=GroupGrantRequest,responses((status=200,body=GroupGrant)))]
pub async fn set_group(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    Json(input): Json<GroupGrantRequest>,
) -> Result<Json<GroupGrant>> {
    let issuer = &state
        .config
        .oidc
        .as_ref()
        .ok_or_else(|| Error::invalid("Configure the organization identity provider first."))?
        .issuer;
    let role = role(&input.role)?;
    if input.group_name.is_empty()
        || input.group_name.chars().count() > 200
        || input.group_name.chars().any(char::is_control)
    {
        return Err(Error::invalid(
            "Supply an exact group name or ID within 200 characters.",
        ));
    }
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM brain_group_grants WHERE brain_id=$1 AND group_name<>$2",
    )
    .bind(id)
    .bind(&input.group_name)
    .fetch_one(&mut *tx)
    .await?;
    if count >= 1000 {
        return Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "mapping_capacity",
            "This Brain has reached its group mapping capacity.",
        ));
    }
    let mapping:Uuid=sqlx::query_scalar("INSERT INTO brain_group_grants(id,brain_id,group_name,role,issuer) VALUES($1,$2,$3,$4,$5) ON CONFLICT(brain_id,group_name) DO UPDATE SET role=excluded.role,issuer=excluded.issuer RETURNING id")
        .bind(Uuid::new_v4()).bind(id).bind(&input.group_name).bind(role).bind(issuer).fetch_one(&mut *tx).await?;
    db::audit(
        &mut tx,
        auth.user.id,
        id,
        "access.group",
        mapping,
        "granted",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(GroupGrant {
        id: mapping,
        group_name: input.group_name,
        role: input.role,
    }))
}
#[utoipa::path(delete,path="/api/brains/{id}/group-grants/{mapping}",params(("id"=Uuid,Path),("mapping"=Uuid,Path)),responses((status=204)))]
pub async fn remove_group(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, mapping)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM brain_group_grants WHERE id=$1 AND brain_id=$2)",
    )
    .bind(mapping)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if !exists {
        return Err(Error::missing());
    }
    db::audit(
        &mut tx,
        auth.user.id,
        id,
        "access.group",
        mapping,
        "removed",
    )
    .await?;
    sqlx::query("DELETE FROM brain_group_grants WHERE id=$1 AND brain_id=$2")
        .bind(mapping)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(post,path="/api/brains/{id}/owner",params(("id"=Uuid,Path)),request_body=OwnerRequest,responses((status=200,body=AccessChange)))]
pub async fn transfer(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    Json(input): Json<OwnerRequest>,
) -> Result<Json<AccessChange>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let owner: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if owner != auth.user.id {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "brain_owner_required",
            "Only the current Brain owner can transfer ownership.",
        ));
    }
    let enabled: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND enabled)")
            .bind(input.account_id)
            .fetch_one(&mut *tx)
            .await?;
    if !enabled {
        return Err(Error::invalid("Choose an enabled account."));
    }
    db::audit(
        &mut tx,
        auth.user.id,
        id,
        "brain.owner",
        input.account_id,
        "transferred",
    )
    .await?;
    sqlx::query("UPDATE brains SET owner_id=$2,updated_at=now() WHERE id=$1")
        .bind(id)
        .bind(input.account_id)
        .execute(&mut *tx)
        .await?;
    let response = effective(&mut tx, id, auth.user.id).await?;
    tx.commit().await?;
    Ok(response)
}
