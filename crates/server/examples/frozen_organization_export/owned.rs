//! Isolated nonce-owned canonical application, never the normal DB or worker.
use super::io;
pub use super::io::json as private_json;
use anyhow::{Context, Result, ensure};
use axum::{
    body::{Body, to_bytes},
    extract::FromRequestParts,
    http::Request,
};
use recollect_server::{AppState, app, auth::Auth, config::Config, db};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{path::Path, time::Duration};
use tower::ServiceExt;
use uuid::Uuid;
pub struct Login {
    pub cookie: String,
    pub csrf: String,
}
pub struct Owned {
    pub state: AppState,
    pub admin: PgPool,
    pub cluster: PgPool,
    pub database: String,
    pub marker: String,
    pub login: Login,
    pub actor: Uuid,
    pub nonce_hash: String,
}
pub async fn http(
    state: &AppState,
    login: Option<&Login>,
    method: &str,
    path: &str,
    body: Value,
) -> Result<(Value, String)> {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("idempotency-key", Uuid::new_v4().to_string());
    if let Some(l) = login {
        req = req
            .header("cookie", &l.cookie)
            .header("x-csrf-token", &l.csrf);
    }
    let response = app(state.clone())
        .oneshot(req.body(Body::from(body.to_string()))?)
        .await?;
    let status = response.status();
    let cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .into();
    let b = to_bytes(response.into_body(), 1024 * 1024).await?;
    let body = if b.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&b)?
    };
    ensure!(status.is_success(), "Canonical API failed: {status}");
    Ok((body, cookie))
}
pub fn api<T>(r: recollect_server::error::Result<T>) -> Result<T> {
    r.map_err(|e| anyhow::anyhow!("Native operation rejected ({})", e.1))
}
pub fn database_url(value: &str, db: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(value)?;
    ensure!(
        matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")),
        "Database must be loopback"
    );
    ensure!(
        url.query().is_none() && url.fragment().is_none(),
        "Database URL options forbidden"
    );
    url.set_path(db);
    Ok(url.into())
}
impl Owned {
    pub async fn setup(out: &Path, mut config: Config) -> Result<Self> {
        let admin_url = std::env::var("DATABASE_ADMIN_URL").context("Local admin unavailable")?;
        let a = reqwest::Url::parse(&admin_url)?;
        let b = reqwest::Url::parse(&config.database_url)?;
        ensure!(
            a.host_str() == b.host_str() && a.port_or_known_default() == b.port_or_known_default(),
            "Database endpoints differ"
        );
        ensure!(
            config.models.endpoint.starts_with("http://127.0.0.1:")
                && config
                    .models
                    .openrouter_endpoint
                    .starts_with("http://127.0.0.1:"),
            "Controlled transports required before application state"
        );
        let nonce = Uuid::new_v4();
        let nonce_hash = io::sha(nonce.as_bytes())?;
        let database = format!("recollect_native_export_{}", nonce.simple());
        let marker = format!("Recollect native export conformance {nonce_hash}");
        let tagged = |url: String, role: &str| -> Result<String> {
            let mut url = reqwest::Url::parse(&url)?;
            url.query_pairs_mut().append_pair(
                "application_name",
                &format!("native_export_{}_{}", nonce.simple(), role),
            );
            Ok(url.into())
        };
        let cluster = db::pool(&tagged(database_url(&admin_url, "postgres")?, "cluster")?).await?;
        io::json(
            &out.join("ownership.json"),
            &json!({"database":database,"owner_comment":marker,"nonce_sha256":nonce_hash,"normal_data_copied":false,"background_worker_started":false}),
        )?;
        sqlx::query(&format!("CREATE DATABASE {database}"))
            .execute(&cluster)
            .await?;
        sqlx::query(&format!("COMMENT ON DATABASE {database} IS '{marker}'"))
            .execute(&cluster)
            .await?;
        let admin = db::pool(&tagged(database_url(&admin_url, &database)?, "admin")?).await?;
        db::migrate(&admin).await?;
        config.database_url = tagged(database_url(&config.database_url, &database)?, "app")?;
        config.owner_username = format!("native-{}", Uuid::new_v4());
        config.owner_password = Uuid::new_v4().to_string();
        config.oidc = None;
        config.erasure_mirror = None;
        io::new_directory(&out.join("state"))?;
        config.credential_file = out.join("state/credentials.json").to_string_lossy().into();
        config.artifact_dir = out.join("state/artifacts").to_string_lossy().into();
        config.erasure_journal = out.join("state/erasure-journal").to_string_lossy().into();
        let actor = db::bootstrap(&admin, &config.owner_username).await?;
        recollect_server::privacy_journal::initialize(&admin, &config).await?;
        let mut state = AppState::new(db::pool(&config.database_url).await?, config)?;
        state.http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(45))
            .build()?;
        let (session, cookie) = http(
            &state,
            None,
            "POST",
            "/api/auth/login",
            json!({"username":state.config.owner_username,"password":state.config.owner_password}),
        )
        .await?;
        let login = Login {
            cookie,
            csrf: session["csrf_token"]
                .as_str()
                .context("CSRF missing")?
                .into(),
        };
        Ok(Self {
            state,
            admin,
            cluster,
            database,
            marker,
            login,
            actor,
            nonce_hash,
        })
    }
    pub async fn auth(&self) -> Result<Auth> {
        let (mut p, _) = Request::builder()
            .uri("/api/auth/me")
            .header("cookie", &self.login.cookie)
            .body(())?
            .into_parts();
        let auth = api(Auth::from_request_parts(&mut p, &self.state).await)?;
        api(auth.require_browser())?;
        ensure!(auth.user.id == self.actor, "Evaluator changed");
        Ok(auth)
    }
    pub async fn verify(&self) -> Result<()> {
        let (name,comment):(String,Option<String>)=sqlx::query_as("SELECT current_database()::text,shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database()").fetch_one(&self.admin).await?;
        ensure!(
            name == self.database && comment.as_deref() == Some(self.marker.as_str()),
            "Owned DB mismatch"
        );
        Ok(())
    }
    pub async fn drop_database(self, out: &Path) -> Result<()> {
        self.verify().await.context("drop:verify")?;
        self.state.pool.close().await;
        self.admin.close().await;
        drop(self.state);
        drop(self.admin);
        let mut closed = false;
        let mut connection = self.cluster.acquire().await?;
        let cluster_database: String = sqlx::query_scalar("SELECT current_database()::text")
            .fetch_one(&mut *connection)
            .await?;
        ensure!(
            cluster_database == "postgres",
            "drop:cluster_database_mismatch"
        );
        for _ in 0..200 {
            sqlx::query("SELECT pg_stat_clear_snapshot()")
                .execute(&mut *connection)
                .await?;
            let n: i64 =
                sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=$1")
                    .bind(&self.database)
                    .fetch_one(&mut *connection)
                    .await?;
            if n == 0 {
                closed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        if !closed {
            let observations: Value = sqlx::query_scalar("SELECT coalesce(jsonb_agg(jsonb_build_object('pid',pid,'state',state,'backend_type',backend_type,'client_port',client_port,'usename',usename,'application_name',application_name,'backend_start',backend_start)), '[]'::jsonb) FROM pg_stat_activity WHERE datname=$1").bind(&self.database).fetch_one(&mut *connection).await?;
            io::json(
                &out.join("owned-sessions-retained.json"),
                &json!({"database":self.database,"sessions":observations,"no_termination":true}),
            )?;
        }
        drop(connection);
        ensure!(closed, "drop:sessions_remain");
        let comment: Option<String> = sqlx::query_scalar(
            "SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=$1",
        )
        .bind(&self.database)
        .fetch_one(&self.cluster)
        .await?;
        ensure!(
            comment.as_deref() == Some(self.marker.as_str()),
            "drop:identity_mismatch"
        );
        sqlx::query(&format!("DROP DATABASE {}", self.database))
            .execute(&self.cluster)
            .await
            .context("drop:execute")?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)")
                .bind(&self.database)
                .fetch_one(&self.cluster)
                .await?;
        ensure!(!exists, "drop:absence");
        let state_path = out.join("state");
        io::remove_owned_tree(&state_path)?;
        ensure!(
            std::fs::symlink_metadata(&state_path).is_err(),
            "drop:state_still_exists"
        );
        io::json(
            &out.join("database-cleanup.json"),
            &json!({"database":self.database,"ownership_verified":true,"remaining_sessions":0,"drop_without_force":true,"absent":true,"neighbor_state_changed":false,"private_state_removed":true}),
        )?;
        self.cluster.close().await;
        Ok(())
    }
}
