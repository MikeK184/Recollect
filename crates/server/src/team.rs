use crate::{
    AppState,
    auth::{self, Auth},
    credentials,
    error::{Error, Result},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use sqlx::{Postgres, Transaction, types::Json as DbJson};
use uuid::Uuid;

pub fn owner(auth: &Auth) -> Result<()> {
    auth.require_browser()?;
    if !auth.user.installation_owner {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "owner_required",
            "Only the installation owner can manage accounts.",
        ));
    }
    Ok(())
}
fn username(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 || value.chars().any(char::is_control) {
        return Err(Error::invalid(
            "Use a username between 1 and 120 characters.",
        ));
    }
    Ok(value.into())
}
fn conflict() -> Error {
    Error(
        StatusCode::CONFLICT,
        "account_exists",
        "That username or identity is already enrolled.",
    )
}
pub fn gone() -> Error {
    Error(
        StatusCode::GONE,
        "invitation_unavailable",
        "This invitation was used, revoked or expired. Ask the installation owner for a new one.",
    )
}
async fn capacity(tx: &mut Transaction<'_, Postgres>) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(73241003)")
        .execute(&mut **tx)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&mut **tx)
        .await?;
    if count >= 1000 {
        return Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "account_capacity",
            "This installation has reached its local account capacity.",
        ));
    }
    Ok(())
}
pub async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    target: Uuid,
    action: &str,
    disposition: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO mutation_audit(id,actor_id,target_id,action,disposition) VALUES($1,$2,$3,$4,$5)")
        .bind(Uuid::new_v4()).bind(actor).bind(target).bind(action).bind(disposition).execute(&mut **tx).await?;
    Ok(())
}
async fn account(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Account> {
    let row:DbJson<Account>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'username',username,'enabled',enabled,'installation_owner',installation_owner,'auth_kind',auth_kind,'oidc_subject',oidc_subject) FROM accounts WHERE id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or_else(Error::missing)?;
    Ok(row.0)
}
async fn new_invite(
    tx: &mut Transaction<'_, Postgres>,
    account_id: Uuid,
    username: String,
) -> Result<EnrollmentLink> {
    sqlx::query("UPDATE invitations SET revoked_at=now() WHERE account_id=$1 AND consumed_at IS NULL AND revoked_at IS NULL").bind(account_id).execute(&mut **tx).await?;
    let (id, token) = (Uuid::new_v4(), Uuid::new_v4());
    let expires_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO invitations(id,account_id,token) VALUES($1,$2,$3) RETURNING expires_at",
    )
    .bind(id)
    .bind(account_id)
    .bind(token)
    .fetch_one(&mut **tx)
    .await?;
    Ok(EnrollmentLink {
        id,
        token,
        username,
        expires_at,
    })
}

#[utoipa::path(get,path="/api/team",operation_id="teamList",responses((status=200,body=Team)))]
pub async fn list(State(state): State<AppState>, auth: Auth) -> Result<Json<Team>> {
    owner(&auth)?;
    let mut tx = auth.tx(&state.pool).await?;
    let rows:Vec<DbJson<Account>>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'username',username,'enabled',enabled,'installation_owner',installation_owner,'auth_kind',auth_kind,'oidc_subject',oidc_subject) FROM accounts ORDER BY installation_owner DESC,username LIMIT 1000").fetch_all(&mut *tx).await?;
    let invitations:Vec<DbJson<Invitation>>=sqlx::query_scalar("SELECT jsonb_build_object('id',i.id,'account_id',i.account_id,'username',a.username,'expires_at',i.expires_at,'state',CASE WHEN i.consumed_at IS NOT NULL THEN 'accepted' WHEN i.revoked_at IS NOT NULL THEN 'revoked' WHEN i.expires_at<=now() THEN 'expired' ELSE 'pending' END) FROM invitations i JOIN accounts a ON a.id=i.account_id ORDER BY i.created_at DESC LIMIT 1000").fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(Team {
        accounts: rows.into_iter().map(|r| r.0).collect(),
        invitations: invitations.into_iter().map(|r| r.0).collect(),
    }))
}
#[utoipa::path(post,path="/api/team/invitations",request_body=InviteRequest,responses((status=200,body=EnrollmentLink)))]
pub async fn invite(
    State(state): State<AppState>,
    auth: Auth,
    Json(input): Json<InviteRequest>,
) -> Result<Json<EnrollmentLink>> {
    owner(&auth)?;
    let name = username(&input.username)?;
    let mut tx = auth.tx(&state.pool).await?;
    capacity(&mut tx).await?;
    let id = Uuid::new_v4();
    let row=sqlx::query("INSERT INTO accounts(id,username,enabled) VALUES($1,$2,false) ON CONFLICT(username) DO NOTHING").bind(id).bind(&name).execute(&mut *tx).await?;
    if row.rows_affected() != 1 {
        return Err(conflict());
    }
    let response = new_invite(&mut tx, id, name).await?;
    audit(&mut tx, auth.user.id, id, "account.invite", "pending").await?;
    tx.commit().await?;
    Ok(Json(response))
}
#[utoipa::path(delete,path="/api/team/invitations/{id}",params(("id"=Uuid,Path)),responses((status=204)))]
pub async fn revoke_invite(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    owner(&auth)?;
    let mut tx = auth.tx(&state.pool).await?;
    let target:Option<Uuid>=sqlx::query_scalar("UPDATE invitations SET revoked_at=now() WHERE id=$1 AND consumed_at IS NULL AND revoked_at IS NULL AND expires_at>now() RETURNING account_id").bind(id).fetch_optional(&mut *tx).await?;
    audit(
        &mut tx,
        auth.user.id,
        target.ok_or_else(gone)?,
        "invitation.revoke",
        "revoked",
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(post,path="/api/auth/enroll",request_body=EnrollRequest,responses((status=200,body=SessionInfo)))]
pub async fn enroll(
    State(state): State<AppState>,
    Json(input): Json<EnrollRequest>,
) -> Result<Response> {
    if !(8..=4096).contains(&input.password.len()) {
        return Err(Error::invalid("Use a password between 8 and 4,096 bytes."));
    }
    let mut tx = state.pool.begin().await?;
    let id: Option<Uuid> = sqlx::query_scalar("SELECT account_id FROM invitations WHERE token=$1")
        .bind(input.token)
        .fetch_optional(&mut *tx)
        .await?;
    let id = id.ok_or_else(gone)?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let usable:Option<Uuid>=sqlx::query_scalar("SELECT id FROM invitations WHERE token=$1 AND consumed_at IS NULL AND revoked_at IS NULL AND expires_at>now() FOR UPDATE").bind(input.token).fetch_optional(&mut *tx).await?;
    usable.ok_or_else(gone)?;
    let data = account(&mut tx, id).await?;
    if data.installation_owner || data.auth_kind != "local" {
        return Err(gone());
    }
    let credential = Uuid::new_v4();
    credentials::store(&state.config.credential_file, credential, &input.password).await?;
    sqlx::query("UPDATE accounts SET enabled=true,credential_id=$2 WHERE id=$1")
        .bind(id)
        .bind(credential)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE invitations SET consumed_at=now() WHERE token=$1")
        .bind(input.token)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT set_config('recollect.actor',$1,true)")
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
    audit(&mut tx, id, id, "account.enroll", "activated").await?;
    let response = auth::session_response(
        &state,
        &mut tx,
        User {
            id,
            username: data.username,
            installation_owner: false,
        },
        43200,
    )
    .await?;
    tx.commit().await?;
    Ok(response)
}
#[utoipa::path(patch,path="/api/team/accounts/{id}",operation_id="accountStatus",params(("id"=Uuid,Path)),request_body=AccountStatus,responses((status=200,body=Account)))]
pub async fn status(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    Json(input): Json<AccountStatus>,
) -> Result<Json<Account>> {
    owner(&auth)?;
    let mut tx = auth.tx(&state.pool).await?;
    if id == auth.user.id {
        return Err(Error::invalid("The installation owner cannot be disabled."));
    }
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let data = account(&mut tx, id).await?;
    if data.installation_owner {
        return Err(Error::invalid("The installation owner cannot be disabled."));
    }
    if input.enabled {
        let ready: bool = sqlx::query_scalar(
            "SELECT auth_kind='oidc' OR credential_id IS NOT NULL FROM accounts WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if !ready {
            return Err(Error(
                StatusCode::CONFLICT,
                "enrollment_required",
                "This account must accept its invitation first.",
            ));
        }
    }
    sqlx::query("UPDATE accounts SET enabled=$2 WHERE id=$1")
        .bind(id)
        .bind(input.enabled)
        .execute(&mut *tx)
        .await?;
    if !input.enabled {
        sqlx::query("DELETE FROM sessions WHERE account_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE invitations SET revoked_at=now() WHERE account_id=$1 AND consumed_at IS NULL AND revoked_at IS NULL").bind(id).execute(&mut *tx).await?;
    }
    audit(
        &mut tx,
        auth.user.id,
        id,
        "account.status",
        if input.enabled { "enabled" } else { "disabled" },
    )
    .await?;
    let response = account(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(response))
}
#[utoipa::path(post,path="/api/team/accounts/{id}/reset",params(("id"=Uuid,Path)),responses((status=200,body=EnrollmentLink)))]
pub async fn reset(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<EnrollmentLink>> {
    owner(&auth)?;
    let mut tx = auth.tx(&state.pool).await?;
    // The account row serializes recovery, disable, login and enrollment.
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let data = account(&mut tx, id).await?;
    if data.installation_owner || data.auth_kind != "local" {
        return Err(Error::invalid(
            "Use operator recovery for the owner, or provider recovery for an OIDC identity.",
        ));
    }
    sqlx::query("UPDATE accounts SET enabled=false,credential_id=NULL WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE account_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let response = new_invite(&mut tx, id, data.username).await?;
    audit(
        &mut tx,
        auth.user.id,
        id,
        "account.reset",
        "enrollment_required",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(response))
}
#[utoipa::path(post,path="/api/team/oidc-accounts",request_body=OidcAccountRequest,responses((status=200,body=Account)))]
pub async fn oidc_account(
    State(state): State<AppState>,
    auth: Auth,
    Json(input): Json<OidcAccountRequest>,
) -> Result<Json<Account>> {
    owner(&auth)?;
    let oidc = state
        .config
        .oidc
        .as_ref()
        .ok_or_else(|| Error::invalid("Configure the organization identity provider first."))?;
    let name = username(&input.username)?;
    if input.subject.is_empty()
        || input.subject.len() > 255
        || input.subject.chars().any(char::is_control)
    {
        return Err(Error::invalid(
            "Supply the provider's exact stable subject, within 255 bytes.",
        ));
    }
    let mut tx = auth.tx(&state.pool).await?;
    capacity(&mut tx).await?;
    let id = Uuid::new_v4();
    let row=sqlx::query("INSERT INTO accounts(id,username,auth_kind,oidc_issuer,oidc_subject) VALUES($1,$2,'oidc',$3,$4) ON CONFLICT DO NOTHING")
        .bind(id).bind(name).bind(&oidc.issuer).bind(input.subject).execute(&mut *tx).await?;
    if row.rows_affected() != 1 {
        return Err(conflict());
    }
    audit(
        &mut tx,
        auth.user.id,
        id,
        "account.oidc_provision",
        "enabled",
    )
    .await?;
    let data = account(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(data))
}
#[utoipa::path(get,path="/api/team/audit",responses((status=200,body=Vec<AuditEvent>)))]
pub async fn history(State(state): State<AppState>, auth: Auth) -> Result<Json<Vec<AuditEvent>>> {
    owner(&auth)?;
    let mut tx = auth.tx(&state.pool).await?;
    let rows:Vec<DbJson<AuditEvent>>=sqlx::query_scalar("SELECT to_jsonb(a) FROM (SELECT id,actor_id,action,target_id,disposition,created_at FROM mutation_audit WHERE brain_id IS NULL ORDER BY created_at DESC,id LIMIT 100) a").fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(rows.into_iter().map(|r| r.0).collect()))
}
