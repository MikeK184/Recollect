use crate::{
    AppState, auth,
    error::{Error, Result},
    team,
    throttle::{self, Peer},
};
use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::{DateTime, Utc};
use openidconnect::{core::*, *};
use recollect_protocol::{AuthOptions, User};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap, future::Future, pin::Pin, sync::Arc, time::Duration, time::Instant,
};
use uuid::Uuid;

/// Provider metadata (endpoints and signing keys) stays valid this long unless
/// a verification failure shows the provider rotated keys.
const METADATA_TTL: Duration = Duration::from_secs(30 * 60);
pub type MetadataCache = Arc<tokio::sync::Mutex<Option<(Instant, CoreProviderMetadata)>>>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExtraClaims {
    #[serde(flatten)]
    pub values: BTreeMap<String, Value>,
}
impl AdditionalClaims for ExtraClaims {}
type TeamToken = StandardTokenResponse<
    IdTokenFields<
        ExtraClaims,
        EmptyExtraTokenFields,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
    >,
    CoreTokenType,
>;
type TeamClient = Client<
    ExtraClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    TeamToken,
    CoreTokenIntrospectionResponse,
    CoreRevocableToken,
    CoreRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

fn unavailable() -> Error {
    Error(
        StatusCode::SERVICE_UNAVAILABLE,
        "oidc_unavailable",
        "Organization sign-in is unavailable. Retry or use a local account.",
    )
}
/// Upper bound on one provider response (discovery document, key set or token
/// response): generous for real providers, bounded against a broken or hostile one.
const RESPONSE_LIMIT: usize = 1024 * 1024;

/// The OpenID Connect library's HTTP transport, over the server's shared client.
///
/// `openidconnect` 4 and `oauth2` 5 ship only a reqwest 0.12 transport, which
/// bundles its own roots and ignores the system trust store and `SSL_CERT_FILE`.
/// This newtype (the orphan rule forbids implementing the library trait for
/// `reqwest::Client` here) sends the same requests through `AppState::http`, so
/// provider calls share its trust store, five-second timeout, no-redirect policy
/// and connection pool, and one reqwest version serves the whole workspace.
struct SharedHttp(reqwest::Client);
impl<'c> AsyncHttpClient<'c> for SharedHttp {
    type Error = HttpClientError<reqwest::Error>;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<HttpResponse, Self::Error>> + Send + 'c>>;
    fn call(&'c self, request: HttpRequest) -> Self::Future {
        Box::pin(async move {
            let mut response = self
                .0
                .execute(request.try_into().map_err(Box::new)?)
                .await
                .map_err(Box::new)?;
            let mut builder = openidconnect::http::Response::builder()
                .status(response.status())
                .version(response.version());
            for (name, value) in response.headers() {
                builder = builder.header(name, value);
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(Box::new)? {
                if body.len() + chunk.len() > RESPONSE_LIMIT {
                    return Err(HttpClientError::Other(
                        "provider response exceeded its bounded size".into(),
                    ));
                }
                body.extend_from_slice(&chunk);
            }
            builder.body(body).map_err(HttpClientError::Http)
        })
    }
}
fn oidc_http(state: &AppState) -> SharedHttp {
    SharedHttp(state.http.clone())
}
async fn metadata(state: &AppState, refresh: bool) -> Result<CoreProviderMetadata> {
    let settings = state.config.oidc.as_ref().ok_or_else(unavailable)?;
    // Held across discovery so concurrent sign-ins share one fetch.
    let mut cached = state.oidc_metadata.lock().await;
    if !refresh
        && let Some((at, metadata)) = cached.as_ref()
        && at.elapsed() < METADATA_TTL
    {
        return Ok(metadata.clone());
    }
    let discovered = CoreProviderMetadata::discover_async(
        IssuerUrl::new(settings.issuer.clone()).map_err(|_| unavailable())?,
        &oidc_http(state),
    )
    .await
    .map_err(|_| unavailable())?;
    *cached = Some((Instant::now(), discovered.clone()));
    Ok(discovered)
}
async fn client(state: &AppState, refresh: bool) -> Result<TeamClient> {
    let settings = state.config.oidc.as_ref().ok_or_else(unavailable)?;
    let metadata = metadata(state, refresh).await?;
    Ok(TeamClient::from_provider_metadata(
        metadata,
        ClientId::new(settings.client_id.clone()),
        Some(ClientSecret::new(settings.client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!(
            "{}/api/auth/oidc/callback",
            state.config.public_origin
        ))
        .map_err(|_| unavailable())?,
    ))
}
pub fn complete_groups(claims: &ExtraClaims) -> Vec<String> {
    if claims.values.contains_key("hasgroups")
        || claims
            .values
            .get("_claim_names")
            .and_then(|v| v.get("groups"))
            .is_some()
    {
        return vec![];
    }
    let Some(Value::Array(groups)) = claims.values.get("groups") else {
        return vec![];
    };
    if groups.len() > 200 {
        return vec![];
    }
    let mut values = vec![];
    for group in groups {
        let Some(value) = group.as_str() else {
            return vec![];
        };
        if value.is_empty() || value.chars().count() > 200 || value.chars().any(char::is_control) {
            return vec![];
        }
        values.push(value.to_owned());
    }
    values.sort();
    values.dedup();
    values
}
#[utoipa::path(get,path="/api/auth/options",responses((status=200,body=AuthOptions)))]
pub async fn options(State(state): State<AppState>) -> Json<AuthOptions> {
    Json(AuthOptions {
        oidc_configured: state.config.oidc.is_some(),
        membership_minutes: 5,
    })
}
#[utoipa::path(get,path="/api/auth/oidc/start",responses((status=303)))]
pub async fn start(State(state): State<AppState>, Peer(peer): Peer) -> Response {
    // Unauthenticated and writes a row: bound each address so the shared
    // pending-flow cap cannot be exhausted by one caller.
    if !throttle::allow(&state.start_attempts, &format!("oidc|{peer}"), 30) {
        return Redirect::to("/?auth_error=organization_unavailable").into_response();
    }
    match begin(&state).await {
        Ok(response) => response,
        Err(_) => Redirect::to("/?auth_error=organization_unavailable").into_response(),
    }
}
async fn begin(state: &AppState) -> Result<Response> {
    let client = client(state, false).await?;
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let mut authorization = client.authorize_url(
        CoreAuthenticationFlow::AuthorizationCode,
        CsrfToken::new_random,
        Nonce::new_random,
    );
    for scope in &state.config.oidc.as_ref().ok_or_else(unavailable)?.scopes {
        authorization = authorization.add_scope(Scope::new(scope.clone()));
    }
    let (url, csrf, nonce) = authorization
        .add_prompt(CoreAuthPrompt::Login)
        .add_extra_param("max_age", "0")
        .set_pkce_challenge(challenge)
        .url();
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(73241004)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM oidc_flows WHERE expires_at<=now()")
        .execute(&mut *tx)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM oidc_flows")
        .fetch_one(&mut *tx)
        .await?;
    if count >= 1000 {
        return Err(unavailable());
    }
    sqlx::query("INSERT INTO oidc_flows(state,nonce,verifier) VALUES($1,$2,$3)")
        .bind(csrf.secret())
        .bind(nonce.secret())
        .bind(verifier.secret())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let secure = if state.config.public_origin.starts_with("https://") {
        "; Secure"
    } else {
        ""
    };
    Ok(([(header::SET_COOKIE,format!("recollect_oidc={}; HttpOnly; SameSite=Lax; Path=/api/auth/oidc; Max-Age=300{secure}",csrf.secret()))],Redirect::to(url.as_str())).into_response())
}
#[derive(Deserialize)]
pub struct Callback {
    pub state: Option<String>,
    pub code: Option<String>,
}
#[utoipa::path(get,path="/api/auth/oidc/callback",responses((status=303)))]
pub async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(input): Query<Callback>,
) -> Response {
    let result = finish(&state, &headers, input).await;
    let mut response = match result {
        Ok(response) => response,
        Err(_) => Redirect::to("/?auth_error=organization_denied").into_response(),
    };
    response.headers_mut().append(
        header::SET_COOKIE,
        header::HeaderValue::from_static(
            "recollect_oidc=; HttpOnly; SameSite=Lax; Path=/api/auth/oidc; Max-Age=0",
        ),
    );
    response
}
async fn finish(state: &AppState, headers: &HeaderMap, input: Callback) -> Result<Response> {
    let supplied = input
        .state
        .filter(|v| v.len() <= 256)
        .ok_or_else(Error::unauthorized)?;
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| {
            v.split(';')
                .find_map(|part| part.trim().strip_prefix("recollect_oidc="))
        });
    if cookie != Some(supplied.as_str()) {
        return Err(Error::unauthorized());
    }
    let flow: Option<(String, String)> = sqlx::query_as(
        "DELETE FROM oidc_flows WHERE state=$1 AND expires_at>now() RETURNING nonce,verifier",
    )
    .bind(supplied)
    .fetch_optional(&state.pool)
    .await?;
    let (nonce, verifier) = flow.ok_or_else(Error::unauthorized)?;
    let code = input
        .code
        .filter(|v| !v.is_empty() && v.len() <= 8192)
        .ok_or_else(Error::unauthorized)?;
    let client = client(state, false).await?;
    let tokens = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|_| unavailable())?
        .set_pkce_verifier(PkceCodeVerifier::new(verifier))
        .request_async(&oidc_http(state))
        .await
        .map_err(|_| Error::unauthorized())?;
    let id_token = tokens.id_token().ok_or_else(Error::unauthorized)?;
    let nonce = Nonce::new(nonce);
    let claims = match id_token.claims(&client.id_token_verifier(), &nonce) {
        Ok(claims) => claims,
        Err(_) => {
            // The signing keys may have rotated since the metadata was cached:
            // verify once more against freshly discovered keys.
            let fresh = self::client(state, true).await?;
            id_token
                .claims(&fresh.id_token_verifier(), &nonce)
                .map_err(|_| Error::unauthorized())?
        }
    };
    let seconds = (claims.expiration() - Utc::now()).num_seconds().min(300);
    if seconds <= 0 {
        return Err(Error::unauthorized());
    }
    let issuer = &state.config.oidc.as_ref().ok_or_else(unavailable)?.issuer;
    let groups = complete_groups(claims.additional_claims());
    let mut tx = state.pool.begin().await?;
    let row:Option<(Uuid,String)>=sqlx::query_as("SELECT id,username FROM accounts WHERE auth_kind='oidc' AND oidc_issuer=$1 AND oidc_subject=$2 AND enabled FOR UPDATE")
        .bind(issuer).bind(claims.subject().as_str()).fetch_optional(&mut *tx).await?;
    let (id, username) = row.ok_or_else(Error::unauthorized)?;
    let deadline: DateTime<Utc> = Utc::now() + chrono::Duration::seconds(seconds);
    sqlx::query("UPDATE accounts SET oidc_groups=$2,membership_until=$3 WHERE id=$1")
        .bind(id)
        .bind(groups)
        .bind(deadline)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT set_config('recollect.actor',$1,true)")
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
    team::audit(&mut tx, id, id, "auth.oidc", "authenticated").await?;
    let mut response = auth::session_response(
        state,
        &mut tx,
        User {
            id,
            username,
            installation_owner: false,
        },
        seconds,
    )
    .await?;
    tx.commit().await?;
    *response.status_mut() = StatusCode::SEE_OTHER;
    response
        .headers_mut()
        .insert(header::LOCATION, header::HeaderValue::from_static("/"));
    *response.body_mut() = axum::body::Body::empty();
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Answers every request on a loopback port with `head` and `body`.
    async fn serve(head: &'static str, body: Vec<u8>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/discovery", listener.local_addr().unwrap());
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let body = body.clone();
                tokio::spawn(async move {
                    let mut request = [0u8; 1024];
                    let _ = stream.read(&mut request).await;
                    let response = format!(
                        "HTTP/1.1 {head}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                    let _ = stream.shutdown().await;
                });
            }
        });
        url
    }
    fn get(url: &str) -> HttpRequest {
        openidconnect::http::Request::builder()
            .uri(url)
            .body(Vec::new())
            .unwrap()
    }
    fn transport() -> SharedHttp {
        SharedHttp(crate::http_client().build().unwrap())
    }

    #[tokio::test]
    async fn the_shared_transport_returns_status_headers_and_body() {
        let url = serve(
            "200 OK\r\nContent-Type: application/json",
            b"{\"ok\":1}".to_vec(),
        )
        .await;
        let response = transport().call(get(&url)).await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], "application/json");
        assert_eq!(response.body(), b"{\"ok\":1}");
    }

    #[tokio::test]
    async fn the_shared_transport_does_not_follow_redirects() {
        let url = serve(
            "302 Found\r\nLocation: http://127.0.0.1:9/elsewhere",
            Vec::new(),
        )
        .await;
        let response = transport().call(get(&url)).await.unwrap();
        assert_eq!(response.status(), 302);
    }

    #[tokio::test]
    async fn the_shared_transport_bounds_a_provider_response() {
        let url = serve("200 OK", vec![b'x'; RESPONSE_LIMIT + 1]).await;
        assert!(transport().call(get(&url)).await.is_err());
        let at_limit = serve("200 OK", vec![b'x'; RESPONSE_LIMIT]).await;
        let response = transport().call(get(&at_limit)).await.unwrap();
        assert_eq!(response.body().len(), RESPONSE_LIMIT);
    }
}
