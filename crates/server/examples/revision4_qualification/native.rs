//! Verify the repository-owned loopback engine without touching neighbors or
//! changing the source-only benchmark's intentionally disabled native default.
use super::owned as legacy;
use anyhow::{Context, Result, ensure};
use recollect_server::{AppState, config::Config};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use uuid::Uuid;

#[derive(Clone)]
pub struct Native {
    url: String,
    username: String,
    password: String,
}
async fn local_daemon() -> Result<String> {
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::process::Command::new("docker")
            .args([
                "context",
                "inspect",
                "--format",
                "{{.Endpoints.docker.Host}}",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("Docker context inspection timed out")??;
    ensure!(
        output.status.success(),
        "Local Docker context cannot be verified"
    );
    let host = String::from_utf8(output.stdout)?.trim().to_owned();
    let socket = host
        .strip_prefix("unix://")
        .filter(|path| Path::new(path).is_absolute())
        .context("Remote Docker contexts cannot own this loopback graph endpoint")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        ensure!(
            std::fs::metadata(socket)?.file_type().is_socket(),
            "Docker context must use a local Unix socket"
        );
    }
    Ok(host)
}
async fn docker(root: &Path, args: &[&str]) -> Result<String> {
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new(root.join("scripts/docker.sh"))
            .args(args)
            .current_dir(root)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("Owned graph container inspection timed out")??;
    ensure!(
        result.status.success(),
        "Cannot verify repository-owned graph container"
    );
    Ok(String::from_utf8(result.stdout)?.trim().into())
}
fn labels(value: &Value, root: &Path) -> bool {
    value["com.docker.compose.project"] == "recollect"
        && value["com.docker.compose.service"] == "neo4j"
        && value["com.docker.compose.project.working_dir"]
            .as_str()
            .is_some_and(|p| Path::new(p).canonicalize().ok().as_deref() == Some(root))
        && value["com.docker.compose.project.config_files"]
            .as_str()
            .is_some_and(|p| {
                p.split(',').count() == 1
                    && Path::new(p).canonicalize().ok().as_deref()
                        == Some(root.join("compose.yaml").as_path())
            })
}
async fn query(state: &AppState, statement: &str, parameters: Value) -> Result<Value> {
    let response = state
        .http
        .post(format!("{}/db/neo4j/query/v2", state.config.neo4j_url))
        .basic_auth(
            &state.config.neo4j_username,
            Some(&state.config.neo4j_password),
        )
        .json(&json!({"statement":statement,"parameters":parameters,"maxExecutionTime":3}))
        .send()
        .await?;
    ensure!(
        response.status().is_success(),
        "Owned native engine query failed"
    );
    let body: Value = response.json().await?;
    ensure!(
        body["errors"].as_array().is_none_or(|e| e.is_empty()),
        "Owned native engine rejected bounded query"
    );
    Ok(body["data"]["values"].clone())
}
impl Native {
    pub async fn verify(config: &Config, root: &Path, out: &Path, expected: &str) -> Result<Self> {
        ensure!(
            config.neo4j_url == "http://127.0.0.1:57474",
            "Combined native evaluation permits only the fixed owned loopback engine"
        );
        let daemon = local_daemon().await?;
        let ids = docker(
            root,
            &[
                "ps",
                "--filter",
                "label=com.docker.compose.project=recollect",
                "--filter",
                "label=com.docker.compose.service=neo4j",
                "--format",
                "{{.ID}} ",
            ],
        )
        .await?;
        let ids: Vec<_> = ids.split_whitespace().collect();
        ensure!(
            ids.len() == 1 && ids[0].len() >= 12 && ids[0].chars().all(|c| c.is_ascii_hexdigit()),
            "Exactly one owned graph service required"
        );
        let metadata = docker(
            root,
            &[
                "inspect",
                "--format",
                "{{json .Config.Labels}}\n{{json .NetworkSettings.Ports}}\n{{json .State.Running}}",
                ids[0],
            ],
        )
        .await?;
        let parts: Vec<_> = metadata.lines().collect();
        ensure!(parts.len() == 3, "Bounded graph ownership metadata invalid");
        let owner: Value = serde_json::from_str(parts[0])?;
        let ports: Value = serde_json::from_str(parts[1])?;
        ensure!(
            labels(&owner, root) && parts[2] == "true",
            "Graph service ownership/running state changed"
        );
        let binding = ports["7474/tcp"]
            .as_array()
            .context("Graph service has no owned HTTP binding")?;
        ensure!(
            binding.len() == 1
                && binding[0]["HostIp"] == "127.0.0.1"
                && binding[0]["HostPort"] == "57474",
            "Graph service fixed loopback binding changed"
        );
        let native = Self {
            url: config.neo4j_url.clone(),
            username: config.neo4j_username.clone(),
            password: config.neo4j_password.clone(),
        };
        ensure!(
            local_daemon().await? == daemon,
            "Docker context changed during native ownership verification"
        );
        let http = reqwest::Client::builder()
            .no_proxy()
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let response=http.post(format!("{}/db/neo4j/query/v2",native.url)).basic_auth(&native.username,Some(&native.password)).json(&json!({"statement":"RETURN gds.version() AS version","parameters":{},"maxExecutionTime":3})).send().await?;
        ensure!(
            response.status().is_success(),
            "Verified owned native engine unavailable"
        );
        let body: Value = response.json().await?;
        ensure!(
            body["errors"].as_array().is_none_or(|e| e.is_empty())
                && body["data"]["values"] == json!([[expected]]),
            "Actual owned GDS version differs from frozen expected version"
        );
        legacy::private_json(
            &out.join("native-runtime-preflight.json"),
            &json!({"container_id":ids[0],"compose_project":"recollect","compose_service":"neo4j","working_directory":root,"config_file":root.join("compose.yaml"),"http_binding":"127.0.0.1:57474","local_daemon":true,"actual_gds_version":expected,"read_only_preflight":true,"provider_calls":0}),
        )?;
        Ok(native)
    }
    pub fn attach(&self, state: &AppState) -> AppState {
        let mut result = state.clone();
        let mut config = (*state.config).clone();
        config.neo4j_url = self.url.clone();
        config.neo4j_username = self.username.clone();
        config.neo4j_password = self.password.clone();
        result.config = std::sync::Arc::new(config);
        result
    }
}

/// Remove only this controlled fixture's exact owned native rows and scratch.
/// Immutable whitelisted observations must be persisted before this call.
pub async fn cleanup_owned(
    verified: &Native,
    state: &AppState,
    admin: &sqlx::PgPool,
    out: &Path,
    owner_marker: &str,
) -> Result<Value> {
    let owner:Option<String>=sqlx::query_scalar("SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database()").fetch_one(admin).await?;
    ensure!(
        owner.as_deref() == Some(owner_marker)
            && state.config.neo4j_url == verified.url
            && state.config.neo4j_username == verified.username
            && state.config.neo4j_password == verified.password,
        "Native cleanup requires exact owned synthetic DB and verified local endpoint"
    );
    recollect_server::graph::analytics::reconcile(&state.config, &state.pool).await?;
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM analytics_attempts WHERE cleaned_at IS NULL")
            .fetch_one(admin)
            .await?;
    ensure!(pending == 0, "Own native attempt remains unreconciled");
    let installation: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(admin)
        .await?;
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM analytics_attempts ORDER BY id")
        .fetch_all(admin)
        .await?;
    let graphs: Vec<_> = ids
        .iter()
        .map(|id| {
            format!(
                "recollect_analytics_{}_{}",
                installation.simple(),
                id.simple()
            )
        })
        .collect();
    let live = query(
        state,
        "CALL gds.graph.list() YIELD graphName WHERE graphName IN $graphs RETURN count(*) AS count",
        json!({"graphs":graphs}),
    )
    .await?;
    ensure!(
        live == json!([[0]]),
        "Own native GDS projection remains; no database drop allowed"
    );
    let brains: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM brains ORDER BY id")
        .fetch_all(admin)
        .await?;
    let removed=query(state,"MATCH(n) WHERE n.brain IN $brains AND any(label IN labels(n) WHERE label IN ['RecollectGraphEntity','RecollectGraphGeneration','RecollectGraphBrain','RecollectGraphFence']) WITH collect(n) AS owned FOREACH(n IN owned | DETACH DELETE n) RETURN size(owned) AS count",json!({"brains":brains})).await?;
    let remaining=query(state,"MATCH(n) WHERE n.brain IN $brains AND any(label IN labels(n) WHERE label IN ['RecollectGraphEntity','RecollectGraphGeneration','RecollectGraphBrain','RecollectGraphFence']) RETURN count(n) AS count",json!({"brains":brains})).await?;
    ensure!(remaining == json!([[0]]), "Own native rows remain");
    let receipt = json!({"pending_own_journals":pending,"live_own_gds_graphs":0,"remaining_own_native_rows":0,"removed_own_native_rows":removed,"exact_owned_brain_ids":brains,"neighbors_changed":false});
    legacy::private_json(&out.join("native-cleanup-receipt.json"), &receipt)?;
    Ok(receipt)
}

/// Harness-only transport observation. Forwards to the verified actual engine;
/// keeps no request body, credentials, parameter values, or transaction IDs.
#[derive(Clone)]
struct Observer {
    target: Native,
    http: reqwest::Client,
    installation: Uuid,
    allowed: std::sync::Arc<tokio::sync::Mutex<std::collections::BTreeSet<String>>>,
    terminated: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    termination_requests: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    termination_responses: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    statements: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
pub struct NativeObservation {
    server: tokio::task::JoinHandle<std::io::Result<()>>,
    observer: Observer,
}
impl NativeObservation {
    pub fn receipt(&self) -> Value {
        use std::sync::atomic::Ordering;
        json!({"controlled_loopback_relay":true,"destination":"verified_owned_neo4j","installation_id":self.observer.installation,"forwarded_statements":self.observer.statements.load(Ordering::SeqCst),"terminated_exact_owned_transactions":self.observer.terminated.load(Ordering::SeqCst),"termination_requests_forwarded":self.observer.termination_requests.load(Ordering::SeqCst),"uncertain_termination_responses":self.observer.termination_requests.load(Ordering::SeqCst)-self.observer.termination_responses.load(Ordering::SeqCst),"unrelated_transaction_termination_permitted":false,"raw_queries_parameters_tokens_persisted":false})
    }
    pub async fn stop(self) {
        self.server.abort();
        let _ = self.server.await;
    }
}
impl Native {
    pub async fn observe(&self, state: &AppState) -> Result<(Native, AppState, NativeObservation)> {
        use axum::{Router, extract::DefaultBodyLimit, routing::post};
        let installation = sqlx::query_scalar("SELECT id FROM privacy_installation")
            .fetch_one(&state.pool)
            .await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let observer = Observer {
            target: self.clone(),
            http: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .timeout(Duration::from_secs(120))
                .build()?,
            installation,
            allowed: Default::default(),
            terminated: Default::default(),
            termination_requests: Default::default(),
            termination_responses: Default::default(),
            statements: Default::default(),
        };
        let router = Router::new()
            .route("/db/neo4j/query/v2", post(observe_query))
            .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
            .with_state(observer.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await });
        let mut controlled = self.clone();
        controlled.url = url;
        let observed_state = controlled.attach(state);
        Ok((
            controlled,
            observed_state,
            NativeObservation { server, observer },
        ))
    }
}
async fn observe_query(
    axum::extract::State(o): axum::extract::State<Observer>,
    headers: axum::http::HeaderMap,
    axum::Json(body): axum::Json<Value>,
) -> Result<(axum::http::StatusCode, axum::Json<Value>), axum::http::StatusCode> {
    use axum::http::StatusCode;
    use base64::Engine;
    use std::sync::atomic::Ordering;
    let expected = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", o.target.username, o.target.password))
    );
    if headers.get("authorization").and_then(|v| v.to_str().ok()) != Some(expected.as_str()) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let statement = body["statement"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let is_show = statement.starts_with("SHOW TRANSACTIONS");
    let is_terminate = statement.starts_with("TERMINATE TRANSACTIONS");
    if is_show && body["parameters"]["installation"] != json!(o.installation) {
        return Err(StatusCode::FORBIDDEN);
    }
    if is_terminate {
        let ids = body["parameters"]["ids"]
            .as_array()
            .ok_or(StatusCode::FORBIDDEN)?;
        let allowed = o.allowed.lock().await;
        if ids.len() > 10
            || ids
                .iter()
                .any(|id| id.as_str().is_none_or(|s| !allowed.contains(s)))
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    if is_terminate {
        o.termination_requests.fetch_add(1, Ordering::SeqCst);
    }
    let response = o
        .http
        .post(format!("{}/db/neo4j/query/v2", o.target.url))
        .basic_auth(&o.target.username, Some(&o.target.password))
        .json(&body)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_GATEWAY)?;
    if status.is_success() && value["errors"].as_array().is_none_or(|e| e.is_empty()) {
        let rows = value["data"]["values"].as_array();
        if is_show {
            let mut ids = o.allowed.lock().await;
            for row in rows.into_iter().flatten() {
                let id = row[0].as_str().ok_or(StatusCode::BAD_GATEWAY)?;
                if ids.len() >= 100 {
                    return Err(StatusCode::BAD_GATEWAY);
                }
                ids.insert(id.into());
            }
        }
        if is_terminate {
            o.termination_responses.fetch_add(1, Ordering::SeqCst);
            o.terminated
                .fetch_add(rows.map_or(0, Vec::len), Ordering::SeqCst);
        }
    }
    o.statements.fetch_add(1, Ordering::SeqCst);
    Ok((
        StatusCode::from_u16(status.as_u16()).map_err(|_| StatusCode::BAD_GATEWAY)?,
        axum::Json(value),
    ))
}
#[cfg(test)]
mod observer_tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, routing::post};
    use tower::ServiceExt;
    #[tokio::test]
    async fn relay_denies_unauthenticated_query_without_forwarding() {
        let observer = Observer {
            target: Native {
                url: "http://127.0.0.1:9".into(),
                username: "controlled".into(),
                password: "toy-only".into(),
            },
            http: reqwest::Client::builder().no_proxy().build().unwrap(),
            installation: Uuid::new_v4(),
            allowed: Default::default(),
            terminated: Default::default(),
            termination_requests: Default::default(),
            termination_responses: Default::default(),
            statements: Default::default(),
        };
        let router = Router::new()
            .route("/db/neo4j/query/v2", post(observe_query))
            .with_state(observer.clone());
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/db/neo4j/query/v2")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"statement":"TERMINATE TRANSACTIONS $ids","parameters":{"ids":[]}})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(
            observer
                .statements
                .load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert_eq!(
            observer
                .termination_requests
                .load(std::sync::atomic::Ordering::SeqCst),
            0
        );
    }
}
