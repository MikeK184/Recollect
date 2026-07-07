use super::*;
#[path = "agent_native.rs"]
mod agent_native;
#[path = "observations.rs"]
mod observations;
use recollect_mcp_runtime::{
    CancellationToken,
    credentials::CredentialResolver,
    executor::{Coordinator, Executor},
    outbox::Outbox,
};
use recollect_server::mcp::runtime::central::CentralCoordinator;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime},
};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/examples/mcp-fixture")
}
fn execution_manifest() -> McpDefinitionManifest {
    assert!(fixture().is_file(), "Build the mcp-fixture example first");
    let tools = ["inspect","slow","effect","receipt","error","reject"].into_iter().map(|name| json!({
        "name":name,"description":"Owned synthetic execution fixture",
        "inputSchema":{"type":"object","properties":{"text":{"type":"string"},"millis":{"type":"integer"},"operation_id":{"type":"string"}},"additionalProperties":false},
        "annotations":{"readOnlyHint":name!="effect"}
    })).collect::<Vec<_>>();
    serde_json::from_value(json!({
        "key":"execution-fixture","name":"Execution fixture","description":"Isolated SDK proof",
        "transport":"stdio","command":fixture(),"arguments":["serve"],"placements":["central","local","private"],"credential_aliases":[],
        "configuration_schema":{"type":"object","properties":{"marker":{"type":"string"}},"additionalProperties":false},
        "tools":tools,"receipt_policies":[{"tool_name":"effect","operation_id_argument":"operation_id","receipt_tool":"receipt","receipt_id_argument":"operation_id"}]
    })).unwrap()
}
async fn execution_setup() -> (Harness, Login, Uuid, Value, Value, PathBuf) {
    execution_setup_transport(None).await
}
async fn execution_setup_transport(
    target: Option<&str>,
) -> (Harness, Login, Uuid, Value, Value, PathBuf) {
    let h = Harness::new().await;
    let owner = h.login().await;
    let mut definition = execution_manifest();
    if target.is_some() {
        definition.transport = "streamable_http".into();
        definition.command = None;
        definition.arguments.clear();
    }
    definitions::import_manifest(&h.admin, definition)
        .await
        .unwrap();
    let brain = ok(
        &h,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":"Actual MCP execution"}),
    )
    .await;
    let brain = brain["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    let marker = std::env::current_dir()
        .unwrap()
        .join(&h.state.config.artifact_dir)
        .join("effect-proof.txt");
    tokio::fs::create_dir_all(marker.parent().unwrap())
        .await
        .unwrap();
    let mut input = connection_body(None);
    input["definition_key"] = json!("execution-fixture");
    input["credential_alias"] = Value::Null;
    input["configuration"] = json!({"marker":marker});
    if let Some(target) = target {
        input["target"] = json!(target);
    }
    let base = format!("/api/brains/{brain}/mcp");
    let connection = ok(&h, &owner, "POST", &format!("{base}/connections"), input).await;
    let profile = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/profiles"),
        profile_body(&connection, None),
    )
    .await;
    allow_owner(&h, &owner, brain, &profile, true).await;
    (h, owner, brain, connection, profile, marker)
}
async fn http_fixture() -> (tokio::process::Child, String) {
    use tokio::io::AsyncBufReadExt;
    let mut process = tokio::process::Command::new(fixture())
        .arg("serve-http")
        .env_clear()
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut output = tokio::io::BufReader::new(process.stdout.take().unwrap());
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(5), output.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    let target = serde_json::from_str::<Value>(&line).unwrap()["target"]
        .as_str()
        .unwrap()
        .to_string();
    (process, target)
}
async fn finished(h: &Harness, owner: &Login, path: &str) -> Value {
    // These fixtures allow 30 seconds for provider startup and another 30
    // for the call. Include bounded queue/poll overhead so a cold native
    // process is not abandoned before its actual product deadline expires.
    tokio::time::timeout(Duration::from_secs(75), async {
        loop {
            let value = ok(h, owner, "GET", path, Value::Null).await;
            if !matches!(
                value["state"].as_str(),
                Some("queued" | "starting" | "running")
            ) {
                return value;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("bounded actual execution")
}
fn executor(h: &Harness, runner: Runner, outbox: Outbox) -> Executor<CentralCoordinator> {
    Executor::new(
        CentralCoordinator {
            state: h.state.clone(),
            runner,
        },
        CredentialResolver::new(None),
        PathBuf::from(env!("CARGO_BIN_EXE_recollect-server")),
        outbox,
    )
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and built mcp-fixture"]
async fn mcp_executor_sdk_effect_once_receipt_reconciliation_and_session_release() {
    let (h, owner, brain, connection, profile, marker) = execution_setup().await;
    let runner = Runner::central(Uuid::new_v4());
    let lease = checked(runner.register(&h.state).await);
    let outbox = Outbox::open(
        PathBuf::from(&h.state.config.artifact_dir).join("receipts"),
        brain.to_string(),
    )
    .await
    .unwrap();
    let worker = executor(&h, runner, outbox);
    let stop = CancellationToken::new();
    let running_stop = stop.clone();
    let running = tokio::spawn(async move { worker.run(lease, running_stop).await });
    let base = format!("/api/brains/{brain}/mcp/calls");
    let mut input = body(&connection, &profile);
    input["arguments"] = json!({"text":"synthetic"});
    let first = ok(&h, &owner, "POST", &base, input.clone()).await;
    let first = finished(
        &h,
        &owner,
        &format!("{base}/{}", first["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(first["state"], "succeeded", "{first}");
    assert_eq!(
        first["result"]["structuredContent"]["arguments"]["text"],
        "synthetic"
    );
    assert_eq!(
        first["result"]["structuredContent"]["ambient_leaked"],
        false
    );
    input["request_id"] = json!(Uuid::new_v4());
    input["tool_name"] = json!("error");
    let second = ok(&h, &owner, "POST", &base, input.clone()).await;
    let second = finished(
        &h,
        &owner,
        &format!("{base}/{}", second["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(second["state"], "tool_error", "{second}");
    assert_eq!(
        first["instance_id"], second["instance_id"],
        "compatible session reuses the provider"
    );
    input["request_id"] = json!(Uuid::new_v4());
    input["tool_name"] = json!("effect");
    input["arguments"] = json!({"operation_id":Uuid::new_v4()});
    assert_eq!(
        h.call("POST", &base, Some(&owner), input.clone()).await.0,
        StatusCode::BAD_REQUEST
    );
    input["arguments"] = json!({});
    let effect = ok(&h, &owner, "POST", &base, input.clone()).await;
    let effect_path = format!("{base}/{}", effect["id"].as_str().unwrap());
    let effect = finished(&h, &owner, &effect_path).await;
    assert_eq!(effect["state"], "unknown", "{effect}");
    assert_eq!(
        ok(&h, &owner, "POST", &base, input.clone()).await["id"],
        effect["id"]
    );
    let lookup = json!({"request_id":Uuid::new_v4(),"client_session_id":Uuid::new_v4()});
    let receipt = ok(
        &h,
        &owner,
        "POST",
        &format!("{effect_path}/reconcile"),
        lookup.clone(),
    )
    .await;
    assert_ne!(receipt["id"], effect["id"]);
    let receipt = finished(
        &h,
        &owner,
        &format!("{base}/{}", receipt["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(receipt["state"], "succeeded", "{receipt}");
    assert_eq!(
        receipt["result"]["structuredContent"]["operation_id"],
        effect["id"]
    );
    assert_eq!(
        ok(
            &h,
            &owner,
            "POST",
            &format!("{effect_path}/reconcile"),
            lookup
        )
        .await["id"],
        receipt["id"]
    );
    let original = ok(&h, &owner, "GET", &effect_path, Value::Null).await;
    assert_eq!(original["state"], "unknown");
    assert_eq!(original["resolutions"][0]["kind"], "connector_receipt");
    assert_eq!(original["resolutions"][0]["outcome"], "succeeded");
    let marker = tokio::fs::read_to_string(marker).await.unwrap();
    assert_eq!(
        marker.lines().filter(|line| *line == "call effect").count(),
        1
    );
    assert_eq!(
        marker
            .lines()
            .filter(|line| *line == "call receipt")
            .count(),
        1
    );
    assert!(
        marker
            .lines()
            .any(|line| line == format!("effect {}", effect["id"].as_str().unwrap()))
    );
    let release = format!("/api/brains/{brain}/mcp/session/release");
    ok(
        &h,
        &owner,
        "POST",
        &release,
        json!({"client_session_id":input["client_session_id"]}),
    )
    .await;
    input["request_id"] = json!(Uuid::new_v4());
    assert_eq!(
        h.call("POST", &base, Some(&owner), input).await.0,
        StatusCode::CONFLICT
    );
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let active: i64 =
        sqlx::query_scalar("SELECT count(*) FROM mcp_instances WHERE state<>'stopped'")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(active, 0, "graceful shutdown closes owned providers");
    h.finish().await;
}

#[derive(Clone)]
struct StartupExpiry {
    inner: CentralCoordinator,
    admin: sqlx::PgPool,
    mode: Arc<AtomicUsize>,
}
impl Coordinator for StartupExpiry {
    async fn receipt_removals(
        &self,
        input: McpReceiptCheck,
    ) -> recollect_mcp_runtime::Result<McpReceiptRemovals> {
        self.inner.receipt_removals(input).await
    }
    async fn credentials(&self, input: McpAttempt) -> recollect_mcp_runtime::Result<()> {
        self.inner.credentials(input).await
    }
    async fn heartbeat(&self) -> recollect_mcp_runtime::Result<McpRunnerLease> {
        self.inner.heartbeat().await
    }
    async fn claim(&self) -> recollect_mcp_runtime::Result<McpClaim> {
        self.inner.claim().await
    }
    async fn instance(&self, v: McpInstanceUpdate) -> recollect_mcp_runtime::Result<()> {
        let mode = if v.state == "starting" {
            self.mode.swap(0, Ordering::AcqRel)
        } else {
            0
        };
        if mode != 0 {
            let query = if mode == 1 {
                "UPDATE mcp_calls SET lease_until=now()-interval '1 second' WHERE id=$1"
            } else {
                "UPDATE mcp_calls SET deadline=now()-interval '1 second' WHERE id=$1"
            };
            sqlx::query(query)
                .bind(v.attempt.call_id)
                .execute(&self.admin)
                .await
                .unwrap();
            assert!(sqlx::query_scalar::<_,bool>("SELECT lease_until>clock_timestamp() FROM mcp_runners WHERE reference='central'")
                .fetch_one(&self.admin).await.unwrap(), "runner remains live while only its attempt expires");
            let accepted: bool = sqlx::query_scalar(
                "SELECT recollect_mcp_instance('central',$1,$2,$3,$4,$5,'starting',1,0)",
            )
            .bind(v.attempt.epoch)
            .bind(v.attempt.call_id)
            .bind(v.attempt.attempt_token)
            .bind(v.instance_id)
            .bind(v.credential_generation)
            .fetch_one(&self.inner.state.pool)
            .await
            .unwrap();
            assert!(
                !accepted,
                "the narrow SQL admission also rejects the expired attempt"
            );
        }
        self.inner.instance(v).await
    }
    async fn start(&self, v: McpStart) -> recollect_mcp_runtime::Result<McpStartPermit> {
        self.inner.start(v).await
    }
    async fn defer(&self, v: McpAttempt) -> recollect_mcp_runtime::Result<()> {
        self.inner.defer(v).await
    }
    async fn complete(&self, v: McpCompletion) -> recollect_mcp_runtime::Result<()> {
        self.inner.complete(v).await
    }
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and built mcp-fixture"]
async fn mcp_expired_startup_cannot_spawn_and_fresh_attempt_still_executes() {
    let (h, owner, brain, connection, profile, marker) = execution_setup().await;
    let runner = Runner::central(Uuid::new_v4());
    let lease = checked(runner.register(&h.state).await);
    let mode = Arc::new(AtomicUsize::new(0));
    let worker = Executor::new(
        StartupExpiry {
            inner: CentralCoordinator {
                state: h.state.clone(),
                runner,
            },
            admin: h.admin.clone(),
            mode: mode.clone(),
        },
        CredentialResolver::new(None),
        PathBuf::from(env!("CARGO_BIN_EXE_recollect-server")),
        Outbox::open(
            PathBuf::from(&h.state.config.artifact_dir).join("receipts"),
            brain.to_string(),
        )
        .await
        .unwrap(),
    );
    let stop = CancellationToken::new();
    let running_stop = stop.clone();
    let running = tokio::spawn(async move { worker.run(lease, running_stop).await });
    let base = format!("/api/brains/{brain}/mcp/calls");
    for expiry in [1, 2, 0] {
        mode.store(expiry, Ordering::Release);
        let mut input = body(&connection, &profile);
        input["arguments"] = json!({});
        let call = ok(&h, &owner, "POST", &base, input).await;
        let result = finished(
            &h,
            &owner,
            &format!("{base}/{}", call["id"].as_str().unwrap()),
        )
        .await;
        if expiry != 0 {
            assert_eq!(result["state"], "failed", "{result}");
            assert_eq!(result["code"], "mcp_runner_fenced");
            assert!(
                !marker.exists(),
                "expired startup must not spawn its provider"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM mcp_instances")
                    .fetch_one(&h.admin)
                    .await
                    .unwrap(),
                0
            );
        } else {
            assert_eq!(result["state"], "succeeded", "{result}");
            let events = tokio::fs::read_to_string(&marker).await.unwrap();
            assert_eq!(
                events
                    .lines()
                    .filter(|line| line.starts_with("start "))
                    .count(),
                1
            );
            assert_eq!(
                events
                    .lines()
                    .filter(|line| *line == "call inspect")
                    .count(),
                1
            );
        }
    }
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    h.finish().await;
}

#[derive(Clone)]
struct ReceiptFault<C> {
    inner: C,
    unavailable: Arc<AtomicBool>,
}
impl<C: Coordinator> Coordinator for ReceiptFault<C> {
    async fn receipt_removals(
        &self,
        input: McpReceiptCheck,
    ) -> recollect_mcp_runtime::Result<McpReceiptRemovals> {
        self.inner.receipt_removals(input).await
    }
    async fn credentials(&self, input: McpAttempt) -> recollect_mcp_runtime::Result<()> {
        self.inner.credentials(input).await
    }
    async fn heartbeat(&self) -> recollect_mcp_runtime::Result<McpRunnerLease> {
        self.inner.heartbeat().await
    }
    async fn claim(&self) -> recollect_mcp_runtime::Result<McpClaim> {
        self.inner.claim().await
    }
    async fn instance(&self, v: McpInstanceUpdate) -> recollect_mcp_runtime::Result<()> {
        self.inner.instance(v).await
    }
    async fn start(&self, v: McpStart) -> recollect_mcp_runtime::Result<McpStartPermit> {
        self.inner.start(v).await
    }
    async fn defer(&self, v: McpAttempt) -> recollect_mcp_runtime::Result<()> {
        self.inner.defer(v).await
    }
    async fn complete(&self, v: McpCompletion) -> recollect_mcp_runtime::Result<()> {
        if self.unavailable.load(Ordering::Acquire) {
            return Err(recollect_mcp_runtime::RuntimeError(
                "synthetic_receipt_unavailable",
            ));
        }
        self.inner.complete(v).await
    }
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and built mcp-fixture"]
async fn mcp_executor_restart_uploads_receipt_without_tool_replay() {
    restart_receipt("inspect", "succeeded").await;
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and built mcp-fixture"]
async fn mcp_executor_restart_preserves_late_protocol_failure_without_replay() {
    restart_receipt("reject", "failed").await;
}
async fn restart_receipt(tool: &str, outcome: &str) {
    let (h, owner, brain, connection, profile, marker) = execution_setup().await;
    observations::enable(&h, &owner, brain).await;
    let directory = PathBuf::from(&h.state.config.artifact_dir).join("receipts");
    let outbox = Outbox::open(directory.clone(), brain.to_string())
        .await
        .unwrap();
    let runner = Runner::central(Uuid::new_v4());
    let lease = checked(runner.register(&h.state).await);
    let unavailable = Arc::new(AtomicBool::new(false));
    let fault = ReceiptFault {
        inner: CentralCoordinator {
            state: h.state.clone(),
            runner: runner.clone(),
        },
        unavailable: unavailable.clone(),
    };
    let worker = Executor::new(
        fault,
        CredentialResolver::new(None),
        PathBuf::from(env!("CARGO_BIN_EXE_recollect-server")),
        outbox,
    );
    let stop = CancellationToken::new();
    let running_stop = stop.clone();
    let running = tokio::spawn(async move { worker.run(lease, running_stop).await });
    unavailable.store(true, Ordering::Release);
    let base = format!("/api/brains/{brain}/mcp/calls");
    let mut input = body(&connection, &profile);
    input["tool_name"] = json!(tool);
    input["arguments"] = json!({});
    let call = ok(&h, &owner, "POST", &base, input.clone()).await;
    let saved = Outbox::open(directory.clone(), brain.to_string())
        .await
        .unwrap();
    let receipt = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(receipt) = saved.pending(SystemTime::now()).await.unwrap().pop() {
                break receipt;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(receipt.state, outcome);
    if outcome == "failed" {
        assert_eq!(receipt.code, "provider_protocol_rejected");
    }
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let id: Uuid = call["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE mcp_calls SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(id)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE mcp_runners SET lease_until=now()-interval '1 second'")
        .execute(&h.admin)
        .await
        .unwrap();
    checked(runner::maintain(&h.state).await);
    let path = format!("{base}/{id}");
    assert_eq!(
        ok(&h, &owner, "GET", &path, Value::Null).await["state"],
        "unknown"
    );
    let mut unsupported = receipt.clone();
    unsupported.state = "failed".into();
    unsupported.code = "unproven_failure".into();
    unsupported.result = None;
    assert!(runner.complete(&h.state, &unsupported).await.is_err());
    let runner = Runner::central(Uuid::new_v4());
    let lease = checked(runner.register(&h.state).await);
    let worker = executor(
        &h,
        runner.clone(),
        Outbox::open(directory, brain.to_string()).await.unwrap(),
    );
    let stop = CancellationToken::new();
    let running_stop = stop.clone();
    let running = tokio::spawn(async move { worker.run(lease, running_stop).await });
    let resolved = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let result = ok(&h, &owner, "GET", &path, Value::Null).await;
            if !result["resolutions"].as_array().unwrap().is_empty() {
                break result;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(resolved["state"], "unknown");
    assert_eq!(resolved["resolutions"][0]["kind"], "late_receipt");
    assert_eq!(resolved["resolutions"][0]["outcome"], outcome);
    let coordinator = CentralCoordinator {
        state: h.state.clone(),
        runner,
    };
    coordinator.complete(receipt.clone()).await.unwrap();
    coordinator.complete(receipt).await.unwrap();
    checked(recollect_server::mcp::runtime::observations::run_once(&h.state).await);
    let observed = ok(
        &h,
        &owner,
        "GET",
        &format!("{path}/observations"),
        Value::Null,
    )
    .await;
    assert_eq!(observed["total"], 2);
    assert_eq!(observed["items"][0]["outcome"], "unknown");
    assert_eq!(observed["items"][1]["stage"], "late_receipt");
    assert_eq!(observed["items"][1]["outcome"], outcome);
    assert_eq!(
        observed["items"][1]["state"],
        if outcome == "succeeded" {
            "published"
        } else {
            "filtered"
        }
    );
    assert_eq!(
        ok(&h, &owner, "GET", &path, Value::Null).await["resolutions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(ok(&h, &owner, "POST", &base, input).await["id"], call["id"]);
    assert_eq!(
        tokio::fs::read_to_string(marker)
            .await
            .unwrap()
            .lines()
            .filter(|line| *line == format!("call {tool}"))
            .count(),
        1
    );
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(saved.count(SystemTime::now()).await.unwrap(), 0);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL, OS store and built native binaries"]
async fn mcp_local_native_runner_and_cli_execute_on_the_paired_device() {
    local_native(false, false).await;
    local_native(true, false).await;
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL, OS store and built native binaries"]
async fn mcp_private_native_runner_executes_stdio_and_http_with_exact_registration() {
    local_native(false, true).await;
    local_native(true, true).await;
}
async fn local_native(http: bool, private: bool) {
    use recollect_agent::{Client, CredentialSlot, StoredDevice};
    let hosted = if http {
        Some(http_fixture().await)
    } else {
        None
    };
    let (h, owner, brain, connection, profile, marker) =
        execution_setup_transport(hosted.as_ref().map(|(_, target)| target.as_str())).await;
    let (device, token) = h.pair_device(&owner, "Owned native MCP proof").await;
    let mut edit = connection_body(None);
    edit["definition_key"] = json!("execution-fixture");
    edit["credential_alias"] = Value::Null;
    edit["configuration"] = json!({"marker":marker});
    edit["base_revision"] = connection["summary"]["revision"].clone();
    let private_id = if private {
        let registration = ok(&h, &owner, "POST", &format!("/api/brains/{brain}/mcp/private-runners"),
            json!({"name":"Native private host","device_id":device,"enabled":true,"base_revision":null})).await;
        Some(registration["id"].as_str().unwrap().to_string())
    } else {
        None
    };
    edit["placement"] = json!(if private { "private" } else { "local" });
    edit["runner_reference"] = json!(
        private_id
            .as_ref()
            .map(|id| format!("private:{id}"))
            .unwrap_or_else(|| format!("device:{device}"))
    );
    if let Some((_, target)) = &hosted {
        edit["target"] = json!(target);
    }
    let connection = ok(
        &h,
        &owner,
        "PUT",
        &format!(
            "/api/brains/{brain}/mcp/connections/{}",
            connection["summary"]["id"].as_str().unwrap()
        ),
        edit,
    )
    .await;
    let (status,task)=h.bearer("POST",&format!("/api/brains/{brain}/workspace/tasks"),&token,
        json!({"label":"Native MCP scope","selection":{"repository_ids":[],"area_ids":[],"environment_id":null}})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, operation) = h
        .bearer(
            "POST",
            &format!(
                "/api/brains/{brain}/workspace/tasks/{}/operations",
                task["task"]["id"].as_str().unwrap()
            ),
            &token,
            json!({"kind":"tool"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let stored = StoredDevice {
        endpoint: endpoint.clone(),
        device_id: device,
        token: token.parse().unwrap(),
    };
    let profile_name = format!("mcp-proof-{}", Uuid::new_v4());
    let slot = CredentialSlot::new(&endpoint, &profile_name).unwrap();
    assert!(slot.load().unwrap().is_none());
    slot.save(&stored).unwrap();
    struct Forget(CredentialSlot);
    impl Drop for Forget {
        fn drop(&mut self) {
            let _ = self.0.forget();
        }
    }
    let _saved = Forget(slot);
    let binary =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/recollect-agent");
    assert!(binary.is_file(), "Build recollect-agent first");
    let mut input = body(&connection, &profile);
    input["arguments"] = json!({"text":"paired native execution"});
    input["operation_id"] = operation["id"].clone();
    let input_file = PathBuf::from(&h.state.config.artifact_dir).join("native-call.json");
    tokio::fs::write(&input_file, serde_json::to_vec(&input).unwrap())
        .await
        .unwrap();
    let output = tokio::time::timeout(
        Duration::from_secs(120),
        tokio::process::Command::new(&binary)
            .env_clear()
            .envs(
                std::env::var_os("DBUS_SESSION_BUS_ADDRESS")
                    .map(|value| ("DBUS_SESSION_BUS_ADDRESS", value)),
            )
            .env("RECOLLECT_URL", &endpoint)
            .env("RECOLLECT_DEVICE_PROFILE", &profile_name)
            .args([
                "mcp",
                "call",
                &brain.to_string(),
                input_file.to_str().unwrap(),
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("bounded native credential-store submission")
    .unwrap();
    assert!(
        output.status.success(),
        "Native call submission failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let call: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(call["state"], "queued");
    let outbox = PathBuf::from(&h.state.config.artifact_dir).join("native-receipts");
    let native_args = private_id
        .as_ref()
        .map(|id| vec!["private-runner", id.as_str(), outbox.to_str().unwrap()])
        .unwrap_or_else(|| vec!["mcp-runner", outbox.to_str().unwrap()]);
    let mut native = tokio::process::Command::new(&binary)
        .env_clear()
        .envs(
            std::env::var_os("DBUS_SESSION_BUS_ADDRESS")
                .map(|value| ("DBUS_SESSION_BUS_ADDRESS", value)),
        )
        .env("RECOLLECT_URL", &endpoint)
        .env("RECOLLECT_DEVICE_PROFILE", &profile_name)
        .args(native_args)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let path = format!(
        "/api/brains/{brain}/mcp/calls/{}",
        call["id"].as_str().unwrap()
    );
    let result = finished(&h, &owner, &path).await;
    assert_eq!(result["state"], "succeeded", "{result}");
    assert_eq!(result["scope"], operation["scope"]);
    assert_eq!(
        result["result"]["structuredContent"]["device_leaked"],
        false
    );
    let client = Client::new(&endpoint).unwrap();
    let readback = recollect_agent::mcp_cli::run(
        &client,
        &stored,
        &[
            "status".into(),
            brain.to_string(),
            call["id"].as_str().unwrap().into(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(readback["result"], result["result"]);
    let repeated = recollect_agent::mcp_cli::run(
        &client,
        &stored,
        &[
            "call".into(),
            brain.to_string(),
            input_file.to_str().unwrap().into(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(repeated["id"], call["id"]);
    assert!(
        tokio::process::Command::new("/bin/kill")
            .args(["-INT", &native.id().unwrap().to_string()])
            .status()
            .await
            .unwrap()
            .success()
    );
    assert!(
        tokio::time::timeout(Duration::from_secs(12), native.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    if !http {
        assert_eq!(
            tokio::fs::read_to_string(marker)
                .await
                .unwrap()
                .lines()
                .filter(|line| *line == "call inspect")
                .count(),
            1
        );
    } else {
        assert_eq!(result["result"]["structuredContent"]["call"], 1);
        assert!(
            reqwest::Client::new()
                .post(hosted.as_ref().unwrap().1.clone())
                .json(&json!({"jsonrpc":"2.0","id":1,"method":"server/discover"}))
                .send()
                .await
                .is_ok(),
            "runner shutdown must leave hosted HTTP process alive"
        );
    }
    serving.abort();
    let _ = serving.await;
    h.finish().await;
    if let Some((mut process, _)) = hosted {
        process.kill().await.unwrap();
        process.wait().await.unwrap();
    }
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and built mcp-fixture"]
async fn mcp_central_executor_uses_the_actual_streamable_http_transport() {
    let (mut hosted, target) = http_fixture().await;
    let (h, owner, brain, connection, profile, _) = execution_setup_transport(Some(&target)).await;
    let runner = Runner::central(Uuid::new_v4());
    let lease = checked(runner.register(&h.state).await);
    let outbox = Outbox::open(
        PathBuf::from(&h.state.config.artifact_dir).join("http-receipts"),
        brain.to_string(),
    )
    .await
    .unwrap();
    let worker = executor(&h, runner, outbox);
    let stop = CancellationToken::new();
    let running_stop = stop.clone();
    let running = tokio::spawn(async move { worker.run(lease, running_stop).await });
    let base = format!("/api/brains/{brain}/mcp/calls");
    let mut input = body(&connection, &profile);
    input["arguments"] = json!({});
    let first = ok(&h, &owner, "POST", &base, input.clone()).await;
    let first = finished(
        &h,
        &owner,
        &format!("{base}/{}", first["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(first["state"], "succeeded", "{first}");
    assert_eq!(first["result"]["structuredContent"]["call"], 1);
    assert_eq!(
        ok(&h, &owner, "POST", &base, input.clone()).await["id"],
        first["id"]
    );
    input["request_id"] = json!(Uuid::new_v4());
    let second = ok(&h, &owner, "POST", &base, input).await;
    let second = finished(
        &h,
        &owner,
        &format!("{base}/{}", second["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(second["state"], "succeeded", "{second}");
    assert_eq!(second["result"]["structuredContent"]["call"], 2);
    assert_eq!(first["instance_id"], second["instance_id"]);
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(hosted.try_wait().unwrap().is_none());
    hosted.kill().await.unwrap();
    hosted.wait().await.unwrap();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and built mcp-fixture"]
async fn mcp_private_other_caller_receipt_restart_and_provider_interruption_never_replay() {
    use recollect_agent::{Client, StoredDevice, mcp::LocalCoordinator};
    let (h, owner, brain, connection, profile, marker) = execution_setup().await;
    let (host, token) = h.pair_device(&owner, "Private shared host").await;
    let base = format!("/api/brains/{brain}/mcp");
    let private = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/private-runners"),
        json!({"name":"Shared private host","device_id":host,"enabled":true,"base_revision":null}),
    )
    .await;
    let private_id = private["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    let mut edit = connection_body(None);
    edit["definition_key"] = json!("execution-fixture");
    edit["credential_alias"] = Value::Null;
    edit["configuration"] = json!({"marker":marker});
    edit["placement"] = json!("private");
    edit["runner_reference"] = json!(format!("private:{private_id}"));
    edit["base_revision"] = connection["summary"]["revision"].clone();
    let connection = ok(
        &h,
        &owner,
        "PUT",
        &format!(
            "{base}/connections/{}",
            connection["summary"]["id"].as_str().unwrap()
        ),
        edit,
    )
    .await;
    let (actor, caller) = h.fixture_member().await;
    reader(&h, &owner, brain, actor).await;
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &username(&h, actor).await,
        rights(true, false, false),
    )
    .await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = h.router.clone();
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let device = StoredDevice {
        endpoint: endpoint.clone(),
        device_id: host,
        token: token.parse().unwrap(),
    };
    let client = Arc::new(Client::new(&endpoint).unwrap());
    let (coordinator, lease) =
        LocalCoordinator::register_selected(client.clone(), &device, Some(private_id))
            .await
            .unwrap();
    let directory = PathBuf::from(&h.state.config.artifact_dir).join("private-receipts");
    let receipt_owner = format!("{endpoint}#device:{host}#private:{private_id}");
    let saved = Outbox::open(directory.clone(), receipt_owner.clone())
        .await
        .unwrap();
    let fault = ReceiptFault {
        inner: coordinator,
        unavailable: Arc::new(AtomicBool::new(true)),
    };
    let worker = Executor::new(
        fault,
        CredentialResolver::new(None),
        PathBuf::from(env!("CARGO_BIN_EXE_recollect-server")),
        Outbox::open(directory.clone(), receipt_owner.clone())
            .await
            .unwrap(),
    );
    let stop = CancellationToken::new();
    let running_stop = stop.clone();
    let running = tokio::spawn(async move { worker.run(lease, running_stop).await });
    let calls = format!("{base}/calls");
    let mut input = body(&connection, &profile);
    input["arguments"] = json!({"text":"separate caller"});
    let call = ok(&h, &caller, "POST", &calls, input.clone()).await;
    let receipt = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(receipt) = saved
                .pending(SystemTime::now())
                .await
                .unwrap()
                .into_iter()
                .next()
            {
                break receipt;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(receipt.state, "succeeded");
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE mcp_calls SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(receipt.attempt.call_id)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE mcp_runners SET lease_until=now()-interval '1 second' WHERE reference=$1")
        .bind(format!("private:{private_id}"))
        .execute(&h.admin)
        .await
        .unwrap();
    checked(runner::maintain(&h.state).await);
    let (coordinator, lease) =
        LocalCoordinator::register_selected(client, &device, Some(private_id))
            .await
            .unwrap();
    let worker = Executor::new(
        coordinator.clone(),
        CredentialResolver::new(None),
        PathBuf::from(env!("CARGO_BIN_EXE_recollect-server")),
        Outbox::open(directory, receipt_owner).await.unwrap(),
    );
    let stop = CancellationToken::new();
    let running_stop = stop.clone();
    let running = tokio::spawn(async move { worker.run(lease, running_stop).await });
    let path = format!("{calls}/{}", call["id"].as_str().unwrap());
    let restored = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let call = ok(&h, &caller, "GET", &path, Value::Null).await;
            if !call["resolutions"].as_array().unwrap().is_empty() {
                break call;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(restored["state"], "unknown");
    assert_eq!(restored["actor_id"], actor.to_string());
    assert_eq!(
        restored["runner_reference"],
        format!("private:{private_id}")
    );
    assert_eq!(restored["resolutions"][0]["outcome"], "succeeded");
    coordinator.complete(receipt).await.unwrap();
    assert_eq!(
        ok(&h, &caller, "POST", &calls, input.clone()).await["id"],
        call["id"]
    );
    input["request_id"] = json!(Uuid::new_v4());
    input["tool_name"] = json!("effect");
    input["arguments"] = json!({});
    let effect = ok(&h, &caller, "POST", &calls, input.clone()).await;
    let effect_path = format!("{calls}/{}", effect["id"].as_str().unwrap());
    assert_eq!(
        finished(&h, &caller, &effect_path).await["state"],
        "unknown"
    );
    let lookup = ok(
        &h,
        &caller,
        "POST",
        &format!("{effect_path}/reconcile"),
        json!({"request_id":Uuid::new_v4(),"client_session_id":Uuid::new_v4()}),
    )
    .await;
    assert_eq!(
        finished(
            &h,
            &caller,
            &format!("{calls}/{}", lookup["id"].as_str().unwrap())
        )
        .await["state"],
        "succeeded"
    );
    let original = ok(&h, &caller, "GET", &effect_path, Value::Null).await;
    assert_eq!(original["state"], "unknown");
    assert_eq!(original["resolutions"][0]["kind"], "connector_receipt");
    assert_eq!(original["resolutions"][0]["outcome"], "succeeded");
    let lines = tokio::fs::read_to_string(marker).await.unwrap();
    for tool in ["inspect", "effect", "receipt"] {
        assert_eq!(
            lines
                .lines()
                .filter(|line| *line == format!("call {tool}"))
                .count(),
            1
        );
    }
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(12), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(saved.count(SystemTime::now()).await.unwrap(), 0);
    serving.abort();
    let _ = serving.await;
    h.finish().await;
}
