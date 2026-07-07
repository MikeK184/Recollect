use axum::{Router, body::Body, http::Request, middleware, routing::get};
use recollect_server::{
    AppState, app,
    config::{Config, ModelConfig},
    operations,
};
use std::{
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tower::ServiceExt;

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<u8>>>);
impl Write for Log {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn real_http_metrics_exclude_request_and_configuration_payloads() {
    let log = Log(Arc::default());
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(move || writer.clone())
        .finish();
    let dispatch = tracing::Dispatch::new(subscriber);
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let state = AppState::new(
        pool,
        Config {
            database_url: "UNEXPOSED_DATABASE_CANARY".into(),
            owner_username: "fixture".into(),
            owner_password: "UNEXPOSED_CREDENTIAL_CANARY".into(),
            bind: "127.0.0.1:0".into(),
            public_origin: "http://127.0.0.1:8787".into(),
            static_dir: "/unused".into(),
            neo4j_url: "http://127.0.0.1:1".into(),
            neo4j_username: "unused".into(),
            neo4j_password: "UNEXPOSED_GRAPH_CANARY".into(),
            credential_file: "/unused".into(),
            artifact_dir: "/unused".into(),
            erasure_journal: "/unused".into(),
            erasure_mirror: None,
            models: ModelConfig {
                key: None,
                endpoint: "http://127.0.0.1:1".into(),
                text_model: "unused".into(),
                embedding_model: "unused".into(),
                embedding_dimensions: 1,
            },
            oidc: None,
        },
    )
    .unwrap();
    let metrics = state.metrics.clone();
    let app = app(state);
    for (path, method, status) in [
        ("/health/live?private=UNEXPOSED_QUERY_CANARY", "GET", 200),
        ("/api/UNEXPOSED_PATH_CANARY", "UNEXPOSED_METHOD_CANARY", 404),
        ("/api/operations", "GET", 401),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .method(method)
                    .header("x-private", "UNEXPOSED_HEADER_CANARY")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
    }
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.requests_started, 3);
    assert_eq!(snapshot.in_flight, 0);
    assert_eq!(snapshot.responses_by_status_class[2], 1);
    assert_eq!(snapshot.responses_by_status_class[4], 2);
    assert_eq!(snapshot.response_header_latency_ms.last().unwrap().count, 3);
    let output = serde_json::to_string(&snapshot).unwrap();
    assert!(!output.contains("CANARY"));
    let logs = log.0.lock().unwrap();
    let logs = std::str::from_utf8(&logs).unwrap();
    assert!(logs.contains("HTTP response headers sent"));
    assert!(!logs.contains("CANARY"));
}

#[tokio::test]
async fn interrupted_http_request_releases_its_in_flight_slot() {
    let metrics = Arc::new(operations::Metrics::default());
    let entered = Arc::new(tokio::sync::Notify::new());
    let signal = entered.clone();
    let app = Router::new()
        .route(
            "/pending",
            get(move || async move {
                signal.notify_one();
                std::future::pending::<()>().await;
            }),
        )
        .layer(middleware::from_fn_with_state(
            metrics.clone(),
            operations::record,
        ));
    let request = tokio::spawn(
        app.oneshot(
            Request::builder()
                .uri("/pending")
                .body(Body::empty())
                .unwrap(),
        ),
    );
    tokio::time::timeout(Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    assert_eq!(metrics.snapshot().in_flight, 1);
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.in_flight, 0);
    assert_eq!(snapshot.interrupted, 1);
    assert_eq!(snapshot.responses_by_status_class.iter().sum::<u64>(), 0);
}

#[cfg(unix)]
#[tokio::test]
async fn native_shutdown_handles_both_terminal_and_container_signals() {
    use tokio::io::{AsyncBufReadExt, BufReader};
    for signal in ["-INT", "-TERM"] {
        let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "registered_shutdown_probe",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .env("RECOLLECT_SHUTDOWN_PROBE", "1")
            .env_remove("VAULT_TOKEN")
            .kill_on_drop(true)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let line = lines
                    .next_line()
                    .await
                    .unwrap()
                    .expect("owned signal probe exited before readiness");
                if line == "SHUTDOWN_PROBE_READY" {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(child.try_wait().unwrap().is_none());
        assert!(
            tokio::process::Command::new("/bin/kill")
                .args([signal, &child.id().unwrap().to_string()])
                .status()
                .await
                .unwrap()
                .success()
        );
        let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(
            status.success(),
            "{signal} should use the registered shutdown path"
        );
    }
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Internal child of native_shutdown_handles_both_terminal_and_container_signals"]
async fn registered_shutdown_probe() {
    assert_eq!(
        std::env::var("RECOLLECT_SHUTDOWN_PROBE").as_deref(),
        Ok("1")
    );
    let signal = recollect_server::shutdown::requested();
    tokio::pin!(signal);
    // Poll the real signal future before notifying the parent so delivery cannot
    // race handler registration. No normal service or system process is signaled.
    tokio::select! {
        _ = &mut signal => panic!("unexpected early signal"),
        _ = tokio::time::sleep(Duration::from_millis(10)) => {},
    }
    println!("SHUTDOWN_PROBE_READY");
    tokio::time::timeout(Duration::from_secs(5), signal)
        .await
        .unwrap()
        .unwrap();
}
