use crate::{
    AppState,
    auth::Auth,
    error::{Error, Result},
    team,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use sqlx::{Postgres, Transaction, types::Json as DbJson};
use uuid::Uuid;

fn gone() -> Error {
    Error(
        StatusCode::GONE,
        "pairing_unavailable",
        "This pairing expired or is unavailable. Start pairing again from your companion.",
    )
}
fn conflict() -> Error {
    Error(
        StatusCode::CONFLICT,
        "pairing_state",
        "This pairing has already been decided or is not ready for this step.",
    )
}
fn code(value: &str) -> Result<String> {
    if value.len() != 8 || !value.bytes().all(|v| v.is_ascii_hexdigit()) {
        return Err(gone());
    }
    Ok(value.to_ascii_uppercase())
}
async fn device(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Device> {
    let row:DbJson<Device>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',name,'claimed',claimed,'revoked_at',revoked_at,'expires_at',expires_at,'last_used_at',last_used_at,'created_at',created_at) FROM devices WHERE id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or_else(Error::missing)?;
    Ok(row.0)
}
#[utoipa::path(post,path="/api/devices/pairings",operation_id="startPairing",request_body=PairingRequest,responses((status=200,body=PairingStart)))]
pub async fn start(
    State(state): State<AppState>,
    Json(input): Json<PairingRequest>,
) -> Result<Json<PairingStart>> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err(Error::invalid(
            "Use a device name between 1 and 120 characters.",
        ));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(73241005)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE devices d SET revoked_at=now() FROM device_pairings p WHERE p.device_id=d.id AND p.expires_at<=now() AND NOT d.claimed AND d.revoked_at IS NULL").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM device_pairings WHERE expires_at<=now()")
        .execute(&mut *tx)
        .await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM device_pairings WHERE state IN ('pending','approved')",
    )
    .fetch_one(&mut *tx)
    .await?;
    if count >= 1000 {
        return Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "pairing_capacity",
            "Too many pairings are pending. Retry in a few minutes.",
        ));
    }
    let device_code = Uuid::new_v4();
    let mut user_code = Uuid::new_v4().simple().to_string()[..8].to_ascii_uppercase();
    let expires_at = loop {
        let expires:Option<DateTime<Utc>>=sqlx::query_scalar("INSERT INTO device_pairings(id,device_code,user_code,name) VALUES($1,$2,$3,$4) ON CONFLICT(user_code) DO NOTHING RETURNING expires_at")
            .bind(Uuid::new_v4()).bind(device_code).bind(&user_code).bind(name).fetch_optional(&mut *tx).await?;
        if let Some(expires) = expires {
            break expires;
        }
        user_code = Uuid::new_v4().simple().to_string()[..8].to_ascii_uppercase();
    };
    tx.commit().await?;
    Ok(Json(PairingStart {
        device_code,
        verification_url: format!("{}/devices?code={user_code}", state.config.public_origin),
        user_code,
        expires_at,
        interval_seconds: 2,
    }))
}
#[utoipa::path(get,path="/api/devices/pairings/{code}",params(("code"=String,Path)),responses((status=200,body=PairingView)))]
pub async fn view(
    State(state): State<AppState>,
    auth: Auth,
    Path(value): Path<String>,
) -> Result<Json<PairingView>> {
    auth.require_browser()?;
    let code = code(&value)?;
    let row: Option<(String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT name,state,expires_at FROM device_pairings WHERE user_code=$1 AND expires_at>now()",
    )
    .bind(&code)
    .fetch_optional(&state.pool)
    .await?;
    let (name, status, expires_at) = row.ok_or_else(gone)?;
    Ok(Json(PairingView {
        name,
        user_code: code,
        state: status,
        expires_at,
    }))
}
#[utoipa::path(post,path="/api/devices/pairings/{code}/approve",params(("code"=String,Path)),request_body=PairingDecision,responses((status=200,body=PairingView)))]
pub async fn approve(
    State(state): State<AppState>,
    auth: Auth,
    Path(value): Path<String>,
    Json(input): Json<PairingDecision>,
) -> Result<Json<PairingView>> {
    auth.require_browser()?;
    let code = code(&value)?;
    let mut tx = auth.tx(&state.pool).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(73241006)")
        .execute(&mut *tx)
        .await?;
    let row:Option<(Uuid,String,String,DateTime<Utc>)>=sqlx::query_as("SELECT id,name,state,expires_at FROM device_pairings WHERE user_code=$1 AND expires_at>now() FOR UPDATE").bind(&code).fetch_optional(&mut *tx).await?;
    let (pairing, name, status, expires_at) = row.ok_or_else(gone)?;
    if status != "pending" {
        return Err(conflict());
    }
    let outcome = if input.approve {
        "approved"
    } else {
        "declined"
    };
    let device_id = if input.approve {
        let trimmed = name.trim().to_owned();
        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM devices WHERE account_id=$1 AND lower(name)=lower($2) AND revoked_at IS NULL AND expires_at>now() ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
        )
        .bind(auth.user.id)
        .bind(&trimmed)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(id) = existing {
            sqlx::query(
                "UPDATE devices SET token=$2, expires_at=now()+interval '30 days', claimed=false, name=$3 WHERE id=$1",
            )
            .bind(id)
            .bind(Uuid::new_v4())
            .bind(&trimmed)
            .execute(&mut *tx)
            .await?;
            team::audit(
                &mut tx,
                auth.user.id,
                id,
                "device.approve",
                "awaiting_companion (reused)",
            )
            .await?;
            Some(id)
        } else {
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM devices WHERE account_id=$1 AND revoked_at IS NULL AND expires_at>now()").bind(auth.user.id).fetch_one(&mut *tx).await?;
            if count >= 20 {
                return Err(Error(
                    StatusCode::TOO_MANY_REQUESTS,
                    "device_capacity",
                    "Revoke an unused device before pairing another. The account limit is 20.",
                ));
            }
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO devices(id,account_id,name,token) VALUES($1,$2,$3,$4)")
                .bind(id)
                .bind(auth.user.id)
                .bind(&trimmed)
                .bind(Uuid::new_v4())
                .execute(&mut *tx)
                .await?;
            team::audit(
                &mut tx,
                auth.user.id,
                id,
                "device.approve",
                "awaiting_companion",
            )
            .await?;
            Some(id)
        }
    } else {
        team::audit(&mut tx, auth.user.id, pairing, "device.decline", "declined").await?;
        None
    };
    sqlx::query("UPDATE device_pairings SET state=$2,device_id=$3 WHERE id=$1")
        .bind(pairing)
        .bind(outcome)
        .bind(device_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(PairingView {
        name,
        user_code: code,
        state: outcome.into(),
        expires_at,
    }))
}
#[utoipa::path(post,path="/api/devices/pairings/poll",request_body=PairingCode,responses((status=200,body=PairingPoll)))]
pub async fn poll(
    State(state): State<AppState>,
    Json(input): Json<PairingCode>,
) -> Result<Json<PairingPoll>> {
    let mut tx = state.pool.begin().await?;
    let row:Option<(Uuid,String,Option<Uuid>,bool)>=sqlx::query_as("SELECT id,state,device_id,last_polled_at IS NULL OR last_polled_at<clock_timestamp()-interval '1 second' FROM device_pairings WHERE device_code=$1 AND expires_at>now() FOR UPDATE").bind(input.device_code).fetch_optional(&mut *tx).await?;
    let (id, status, device_id, allowed) = row.ok_or_else(gone)?;
    if status == "declined" {
        return Err(Error(
            StatusCode::GONE,
            "pairing_declined",
            "Pairing was declined in the browser. Start a new request if you want to connect.",
        ));
    }
    if status == "cancelled" {
        return Err(Error(
            StatusCode::GONE,
            "pairing_cancelled",
            "Pairing was cancelled. Start a new request from your companion.",
        ));
    }
    if !allowed {
        return Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "slow_down",
            "Wait two seconds before polling again.",
        ));
    }
    sqlx::query("UPDATE device_pairings SET last_polled_at=clock_timestamp() WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let (metadata, token) = if let Some(device_id) = device_id {
        let item = device(&mut tx, device_id).await?;
        if item.revoked_at.is_some() || item.expires_at <= Utc::now() {
            return Err(gone());
        }
        let token = if status == "approved" {
            Some(
                sqlx::query_scalar("SELECT token FROM devices WHERE id=$1")
                    .bind(device_id)
                    .fetch_one(&mut *tx)
                    .await?,
            )
        } else {
            None
        };
        (Some(item), token)
    } else {
        (None, None)
    };
    tx.commit().await?;
    Ok(Json(PairingPoll {
        state: status,
        device: metadata,
        token,
    }))
}
#[utoipa::path(post,path="/api/devices/pairings/finish",request_body=PairingCode,responses((status=204)))]
pub async fn finish(
    State(state): State<AppState>,
    Json(input): Json<PairingCode>,
) -> Result<StatusCode> {
    let mut tx = state.pool.begin().await?;
    let row:Option<(Uuid,String,Option<Uuid>)>=sqlx::query_as("SELECT id,state,device_id FROM device_pairings WHERE device_code=$1 AND expires_at>now() FOR UPDATE").bind(input.device_code).fetch_optional(&mut *tx).await?;
    let (id, status, device_id) = row.ok_or_else(gone)?;
    if status == "claimed" {
        return Ok(StatusCode::NO_CONTENT);
    }
    if status != "approved" {
        return Err(conflict());
    }
    let device_id = device_id.ok_or_else(gone)?;
    let allowed:Option<bool>=sqlx::query_scalar("SELECT a.enabled AND (a.auth_kind='local' OR a.membership_until>now()) FROM accounts a JOIN devices d ON d.account_id=a.id WHERE d.id=$1 FOR SHARE OF a").bind(device_id).fetch_optional(&mut *tx).await?;
    if allowed != Some(true) {
        return Err(Error::unauthorized());
    }
    let updated = sqlx::query(
        "UPDATE devices SET claimed=true WHERE id=$1 AND revoked_at IS NULL AND expires_at>now()",
    )
    .bind(device_id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(gone());
    }
    sqlx::query("UPDATE device_pairings SET state='claimed' WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(post,path="/api/devices/pairings/cancel",operation_id="cancelPairing",request_body=PairingCode,responses((status=204)))]
pub async fn cancel(
    State(state): State<AppState>,
    Json(input): Json<PairingCode>,
) -> Result<StatusCode> {
    let mut tx = state.pool.begin().await?;
    let row:Option<(Uuid,Option<Uuid>)>=sqlx::query_as("UPDATE device_pairings SET state='cancelled' WHERE device_code=$1 AND state IN ('pending','approved') RETURNING id,device_id").bind(input.device_code).fetch_optional(&mut *tx).await?;
    if let Some((_, Some(device))) = row {
        sqlx::query(
            "UPDATE devices SET revoked_at=coalesce(revoked_at,now()) WHERE id=$1 AND NOT claimed",
        )
        .bind(device)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get,path="/api/devices",operation_id="listDevices",responses((status=200,body=Vec<Device>)))]
pub async fn list(State(state): State<AppState>, auth: Auth) -> Result<Json<Vec<Device>>> {
    let mut tx = auth.tx(&state.pool).await?;
    let rows:Vec<DbJson<Device>>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',name,'claimed',claimed,'revoked_at',revoked_at,'expires_at',expires_at,'last_used_at',last_used_at,'created_at',created_at) FROM devices WHERE account_id=$1 ORDER BY created_at DESC LIMIT 1000").bind(auth.user.id).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(rows.into_iter().map(|r| r.0).collect()))
}
#[utoipa::path(delete,path="/api/devices/{id}",params(("id"=Uuid,Path)),responses((status=204)))]
pub async fn revoke(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    if auth.device_id.is_some() && auth.device_id != Some(id) {
        return Err(Error::missing());
    }
    let mut tx = auth.tx(&state.pool).await?;
    let row: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(
        "SELECT revoked_at FROM devices WHERE id=$1 AND account_id=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(auth.user.id)
    .fetch_optional(&mut *tx)
    .await?;
    if row.ok_or_else(Error::missing)?.is_none() {
        sqlx::query("UPDATE devices SET revoked_at=now() WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        team::audit(&mut tx, auth.user.id, id, "device.revoke", "revoked").await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post,path="/api/devices/revoke-self",responses((status=204)))]
pub async fn revoke_self(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|v| Uuid::parse_str(v).ok())
        .ok_or_else(Error::unauthorized)?;
    let mut tx = state.pool.begin().await?;
    let row:Option<(Uuid,Uuid)>=sqlx::query_as("UPDATE devices SET revoked_at=now() WHERE token=$1 AND revoked_at IS NULL RETURNING id,account_id").bind(token).fetch_optional(&mut *tx).await?;
    if let Some((id, actor)) = row {
        sqlx::query(
            "SELECT set_config('recollect.actor',$1,true),set_config('recollect.device',$2,true)",
        )
        .bind(actor.to_string())
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
        team::audit(&mut tx, actor, id, "device.revoke", "revoked").await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
