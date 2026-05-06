use crate::{
    AppState, credentials,
    error::{Error, Result},
};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{HeaderMap, Method, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use recollect_protocol::{LoginRequest, SessionInfo, User};
use uuid::Uuid;

pub struct Auth {
    pub user: User,
    pub session: Uuid,
    pub csrf: Uuid,
    pub device_id: Option<Uuid>,
}
impl Auth {
    pub fn require_browser(&self) -> Result<()> {
        if self.device_id.is_some() {
            return Err(Error(
                StatusCode::FORBIDDEN,
                "browser_required",
                "Use a signed-in browser for this action.",
            ));
        }
        Ok(())
    }
    pub async fn tx<'a>(
        &self,
        pool: &'a sqlx::PgPool,
    ) -> Result<sqlx::Transaction<'a, sqlx::Postgres>> {
        crate::db::device_tx(pool, self.user.id, self.device_id).await
    }
}

impl FromRequestParts<AppState> for Auth {
    type Rejection = Error;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        if let Some(header) = parts.headers.get(header::AUTHORIZATION) {
            let token = header
                .to_str()
                .ok()
                .and_then(|v| v.strip_prefix("Bearer "))
                .and_then(|v| Uuid::parse_str(v).ok())
                .ok_or_else(Error::unauthorized)?;
            let row:Option<(Uuid,String,bool,Uuid)>=sqlx::query_as("SELECT a.id,a.username,a.installation_owner,d.id FROM devices d JOIN accounts a ON a.id=d.account_id WHERE d.token=$1 AND d.claimed AND d.revoked_at IS NULL AND d.expires_at>now() AND a.enabled AND (a.auth_kind='local' OR a.membership_until>now())")
                .bind(token).fetch_optional(&state.pool).await?;
            let (id, username, installation_owner, device_id) =
                row.ok_or_else(Error::unauthorized)?;
            sqlx::query("UPDATE devices SET last_used_at=now() WHERE id=$1 AND (last_used_at IS NULL OR last_used_at<now()-interval '1 minute')").bind(device_id).execute(&state.pool).await?;
            return Ok(Self {
                user: User {
                    id,
                    username,
                    installation_owner,
                },
                session: token,
                csrf: Uuid::nil(),
                device_id: Some(device_id),
            });
        }
        let token = session_cookie(&parts.headers).ok_or_else(Error::unauthorized)?;
        let row: Option<(Uuid, String, bool, Uuid)> = sqlx::query_as(
            "SELECT a.id,a.username,a.installation_owner,s.csrf_token FROM sessions s JOIN accounts a ON a.id=s.account_id WHERE s.token=$1 AND s.expires_at>now() AND a.enabled")
            .bind(token).fetch_optional(&state.pool).await?;
        let (id, username, installation_owner, csrf) = row.ok_or_else(Error::unauthorized)?;
        if !matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS) {
            let supplied = parts
                .headers
                .get("x-csrf-token")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| Uuid::parse_str(v).ok());
            if supplied != Some(csrf) {
                return Err(Error(
                    StatusCode::FORBIDDEN,
                    "csrf_required",
                    "Refresh the page and retry this action.",
                ));
            }
        }
        Ok(Self {
            user: User {
                id,
                username,
                installation_owner,
            },
            session: token,
            csrf,
            device_id: None,
        })
    }
}

fn session_cookie(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == "recollect_session")
                .then(|| Uuid::parse_str(value).ok())
                .flatten()
        })
}

#[utoipa::path(post, path="/api/auth/login", request_body=LoginRequest, responses((status=200,body=SessionInfo),(status=401,body=recollect_protocol::ApiError),(status=429,body=recollect_protocol::ApiError)))]
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Response> {
    let username = input.username.trim();
    if username.is_empty() || username.len() > 120 || input.password.len() > 4096 {
        return Err(Error::unauthorized());
    }
    {
        let mut attempts = state
            .login_attempts
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let now = std::time::Instant::now();
        attempts.retain(|_, (at, _)| now.duration_since(*at).as_secs() < 60);
        if attempts.len() >= 1024 && !attempts.contains_key(username) {
            return Err(Error(
                StatusCode::TOO_MANY_REQUESTS,
                "throttled",
                "Too many sign-in attempts. Try again in a minute.",
            ));
        }
        let (_, count) = attempts.entry(username.to_string()).or_insert((now, 0));
        *count += 1;
        if *count > 10 {
            return Err(Error(
                StatusCode::TOO_MANY_REQUESTS,
                "throttled",
                "Too many sign-in attempts. Try again in a minute.",
            ));
        }
    }
    let mut tx = state.pool.begin().await?;
    let account: Option<(Uuid, String, bool, Option<Uuid>)> = sqlx::query_as("SELECT id,username,installation_owner,credential_id FROM accounts WHERE username=$1 AND enabled AND auth_kind='local' FOR SHARE")
        .bind(username).fetch_optional(&mut *tx).await?;
    let (id, username, installation_owner, credential) = account.ok_or_else(Error::unauthorized)?;
    let valid = if installation_owner {
        username == state.config.owner_username && input.password == state.config.owner_password
    } else if let Some(credential) = credential {
        credentials::matches(&state.config.credential_file, credential, &input.password).await?
    } else {
        false
    };
    if !valid {
        return Err(Error::unauthorized());
    }
    let response = session_response(
        &state,
        &mut tx,
        User {
            id,
            username,
            installation_owner,
        },
        43200,
    )
    .await?;
    tx.commit().await?;
    Ok(response)
}

pub async fn session_response(
    state: &AppState,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user: User,
    seconds: i64,
) -> Result<Response> {
    let token = Uuid::new_v4();
    let csrf = Uuid::new_v4();
    sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO sessions (token,account_id,csrf_token,expires_at) VALUES ($1,$2,$3,now()+$4::bigint*interval '1 second')")
        .bind(token).bind(user.id).bind(csrf).bind(seconds).execute(&mut **tx).await?;
    let secure = if state.config.public_origin.starts_with("https://") {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "recollect_session={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={seconds}{secure}"
    );
    Ok((
        [(header::SET_COOKIE, cookie)],
        Json(SessionInfo {
            user,
            csrf_token: csrf.to_string(),
            device_id: None,
        }),
    )
        .into_response())
}

#[utoipa::path(get, path="/api/auth/me", responses((status=200,body=SessionInfo),(status=401,body=recollect_protocol::ApiError)))]
pub async fn me(auth: Auth) -> Json<SessionInfo> {
    Json(SessionInfo {
        user: auth.user,
        csrf_token: auth.csrf.to_string(),
        device_id: auth.device_id,
    })
}

#[utoipa::path(post, path="/api/auth/logout", responses((status=204)))]
pub async fn logout(State(state): State<AppState>, auth: Auth) -> Result<Response> {
    auth.require_browser()?;
    sqlx::query("DELETE FROM sessions WHERE token=$1")
        .bind(auth.session)
        .execute(&state.pool)
        .await?;
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            "recollect_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
        )],
    )
        .into_response())
}
