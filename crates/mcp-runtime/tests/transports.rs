pub mod support;
const WIRE_BOUND_FOR_FIXTURE: usize = recollect_mcp_runtime::WIRE_LIMIT + 20;
use recollect_mcp_runtime::{
    CancellationToken, Connected, RuntimeError,
    credentials::{CredentialResolver, ResolvedCredentials},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use uuid::Uuid;

fn fixture_executable() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/mcp-fixture")
}
fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.cache/mcp-runtime-proof")
        .join(Uuid::new_v4().to_string())
}

#[tokio::test]
#[ignore = "Build the owned fixture with scripts/test-mcp-runtime.sh"]
async fn blocked_stdio_writes_cannot_defeat_the_absolute_call_deadline() {
    use nix::{
        sys::signal::{Signal, kill},
        unistd::Pid,
    };
    let executable = fixture_executable();
    let definition = support::definition(Some(executable.to_str().unwrap().into()));
    let connected = Arc::new(
        Connected::open(
            &definition,
            "fixture",
            &json!({}),
            ResolvedCredentials::anonymous(),
            &executable,
        )
        .await
        .unwrap(),
    );
    let reply = connected
        .call(
            "inspect",
            json!({}),
            Duration::from_secs(3),
            CancellationToken::new(),
        )
        .await;
    let child = reply.result.unwrap()["structuredContent"]["pid"]
        .as_u64()
        .unwrap() as u32;
    kill(Pid::from_raw(child as i32), Signal::SIGSTOP).unwrap();
    let mut waiting = Vec::new();
    // Enough bounded frames to exceed both pipes and the forwarding buffer. The
    // test targets transport backpressure; manager admission remains four calls.
    for _ in 0..64 {
        let connected = connected.clone();
        waiting.push(tokio::spawn(async move {
            connected
                .call(
                    "inspect",
                    json!({"text":"x".repeat(32000)}),
                    Duration::from_millis(150),
                    CancellationToken::new(),
                )
                .await
        }));
    }
    let completed = tokio::time::timeout(Duration::from_secs(4), async {
        for call in waiting {
            assert_eq!(call.await.unwrap().state, "unknown");
        }
    })
    .await;
    // Resume the owned fixture even if the assertion failed, so this regression
    // cannot leave a stopped test process behind.
    let _ = kill(Pid::from_raw(child as i32), Signal::SIGCONT);
    let _ = connected.close().await;
    wait_gone(connected.owned_pid.unwrap()).await;
    wait_gone(child).await;
    completed.expect("outer deadline must bound blocked SDK cancellation");
}

async fn fixture_http(
    fixture: support::Fixture,
    legacy: bool,
    delete_barrier: Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>,
) -> (String, CancellationToken, tokio::task::JoinHandle<()>) {
    use axum::{
        Router,
        extract::Request,
        middleware::{self, Next},
    };
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };
    let mut config = StreamableHttpServerConfig::default();
    config.legacy_session_mode = legacy;
    let shutdown = config.cancellation_token.clone();
    let service = StreamableHttpService::new(
        move || Ok(fixture.clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let blocked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let app = Router::new()
        .route_service("/mcp", service)
        .layer(middleware::from_fn(move |request: Request, next: Next| {
            let barrier = delete_barrier.clone();
            let blocked = blocked.clone();
            async move {
                if request.method() == axum::http::Method::DELETE
                    && !blocked.swap(true, Ordering::SeqCst)
                    && let Some((entered, release)) = barrier
                {
                    entered.notify_one();
                    release.notified().await;
                }
                next.run(request).await
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("http://{}/mcp", listener.local_addr().unwrap());
    let stop = shutdown.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(stop.cancelled_owned())
            .await
            .unwrap();
    });
    (target, shutdown, server)
}

#[tokio::test]
#[ignore = "Actual SDK HTTP subscription fixture; scripts/test-mcp-runtime.sh"]
async fn modern_tool_notifications_drain_reuse_and_preserve_dispatched_work() {
    let change = Arc::new(tokio::sync::Notify::new());
    let fixture = support::Fixture {
        change: Some(change.clone()),
        ..Default::default()
    };
    let listening = fixture.listening.clone();
    let calls = fixture.calls.clone();
    let (target, shutdown, server) = fixture_http(fixture, false, None).await;
    let connected = Arc::new(
        Connected::open(
            &support::definition(None),
            &target,
            &json!({}),
            ResolvedCredentials::anonymous(),
            &fixture_executable(),
        )
        .await
        .unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(2), listening.notified())
        .await
        .unwrap();
    let active = connected.clone();
    let long = tokio::spawn(async move {
        active
            .call(
                "slow",
                json!({"millis":350}),
                Duration::from_secs(3),
                CancellationToken::new(),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while calls.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    change.notify_one();
    tokio::time::timeout(Duration::from_secs(2), async {
        while connected.reusable() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let denied = connected
        .call(
            "inspect",
            json!({}),
            Duration::from_secs(1),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(denied.code, "provider_instance_stale");
    assert_eq!(long.await.unwrap().state, "succeeded");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    connected.close().await.unwrap();
    shutdown.cancel();
    server.await.unwrap();
}

#[tokio::test]
#[ignore = "Actual SDK HTTP shutdown barrier; scripts/test-mcp-runtime.sh"]
async fn closing_instances_retain_ownership_and_capacity_until_transport_close() {
    use recollect_mcp_runtime::manager::{InstanceKey, RuntimeManager, StartSpec};
    use std::time::Instant;
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let (target, shutdown, server) = fixture_http(
        support::Fixture {
            legacy: true,
            ..Default::default()
        },
        true,
        Some((entered.clone(), release.clone())),
    )
    .await;
    let definition = support::definition(None);
    let configuration = json!({});
    let executable = fixture_executable();
    let spec = StartSpec {
        definition: &definition,
        target: &target,
        configuration: &configuration,
        supervisor: &executable,
    };
    let secret = ResolvedCredentials::anonymous();
    let manager = Arc::new(RuntimeManager::default());
    let key = InstanceKey {
        brain_id: Uuid::new_v4(),
        profile_id: Uuid::new_v4(),
        connection_id: Uuid::new_v4(),
        actor_id: Uuid::new_v4(),
        device_id: None,
        client_session_id: Uuid::new_v4(),
        runner_epoch: Uuid::new_v4(),
        connection_revision: Uuid::new_v4(),
        definition_revision: "approved".into(),
        credential_generation: secret.generation,
    };
    for i in 0..16 {
        let mut next = key.clone();
        if i > 0 {
            next.client_session_id = Uuid::new_v4();
        }
        drop(
            manager
                .acquire(next, spec, secret.clone(), |_| async { Ok(()) })
                .await
                .unwrap(),
        );
    }
    manager.drain_all();
    let active = manager.clone();
    let closing = tokio::spawn(async move { active.maintain(Instant::now()).await });
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    assert_eq!(manager.statuses(Instant::now()).len(), 16);
    assert!(matches!(
        manager
            .acquire(key.clone(), spec, secret.clone(), |_| async { Ok(()) })
            .await,
        Err(RuntimeError("instance_draining"))
    ));
    let mut seventeenth = key.clone();
    seventeenth.client_session_id = Uuid::new_v4();
    assert!(matches!(
        manager
            .acquire(seventeenth, spec, secret.clone(), |_| async { Ok(()) })
            .await,
        Err(RuntimeError("runtime_capacity"))
    ));
    assert!(
        manager.maintain(Instant::now()).await.is_empty(),
        "another sweep cannot remove closing ownership"
    );
    release.notify_one();
    let closed = closing.await.unwrap();
    assert_eq!(closed.len(), 16);
    assert!(closed.iter().all(|(_, r)| r.is_ok()));
    assert!(manager.statuses(Instant::now()).is_empty());
    drop(
        manager
            .acquire(key, spec, secret, |_| async { Ok(()) })
            .await
            .unwrap(),
    );
    manager.drain_all();
    assert_eq!(manager.maintain(Instant::now()).await.len(), 1);
    shutdown.cancel();
    server.await.unwrap();
}

#[tokio::test]
#[ignore = "Exercises a uniquely owned native OS-store entry; scripts/test-mcp-runtime.sh"]
async fn credential_reference_native_store_rotation_and_invalid_delivery() {
    struct OwnedEntry(keyring::Entry);
    impl Drop for OwnedEntry {
        fn drop(&mut self) {
            let _ = self.0.delete_credential();
        }
    }
    let dir = directory();
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let connection = Uuid::new_v4();
    let account = format!("runtime-fixture-{}", Uuid::new_v4());
    let entry = OwnedEntry(keyring::Entry::new("recollect-mcp", &account).unwrap());
    assert!(matches!(
        entry.0.get_password(),
        Err(keyring::Error::NoEntry)
    ));
    let first = Uuid::new_v4().to_string();
    entry.0.set_password(&first).unwrap();
    let path = dir.join("credential-references.json");
    let mut binding = json!({"bindings":[{"connection_id":connection,"alias":"fixture","runner_reference":"central",
        "environment":{"FIXTURE_CREDENTIAL":{"provider":"os_store","account":account}}}]});
    tokio::fs::write(&path, serde_json::to_vec(&binding).unwrap())
        .await
        .unwrap();
    let resolver = CredentialResolver::new(Some(path.clone()));
    let one = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    assert_eq!(
        one.sanitize(&json!({"text":first})),
        json!({"text":"[redacted]"})
    );
    let numeric = (Uuid::new_v4().as_u128() % 9_000_000_000 + 1_000_000_000).to_string();
    entry.0.set_password(&numeric).unwrap();
    let numeric_secret = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    let executable = fixture_executable();
    let definition = support::definition(Some(executable.to_str().unwrap().into()));
    let connected = Connected::open(
        &definition,
        "fixture",
        &json!({}),
        numeric_secret,
        &executable,
    )
    .await
    .unwrap();
    let reply = connected
        .call(
            "inspect",
            json!({}),
            Duration::from_secs(3),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(reply.state, "succeeded");
    assert_eq!(
        reply.result.as_ref().unwrap()["structuredContent"]["numeric_echo"],
        "[redacted]"
    );
    assert!(!serde_json::to_string(&reply).unwrap().contains(&numeric));
    connected.close().await.unwrap();
    let second = Uuid::new_v4().to_string();
    entry.0.set_password(&second).unwrap();
    let two = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    assert_ne!(one.generation, two.generation);
    assert_eq!(
        two.sanitize(&json!({"text":second})),
        json!({"text":"[redacted]"})
    );
    // Old operations keep their own exact-value redactor while new ones rotate.
    assert_eq!(
        one.sanitize(&json!({"text":first})),
        json!({"text":"[redacted]"})
    );
    binding["bindings"][0]["environment"] =
        json!({"RECOLLECT_DEVICE_PROFILE":{"provider":"os_store","account":account}});
    tokio::fs::write(&path, serde_json::to_vec(&binding).unwrap())
        .await
        .unwrap();
    assert!(matches!(
        resolver
            .resolve(connection, Some("fixture"), "central")
            .await,
        Err(RuntimeError("credential_destination_invalid"))
    ));
    entry.0.delete_credential().unwrap();
    assert!(matches!(
        entry.0.get_password(),
        Err(keyring::Error::NoEntry)
    ));
    tokio::fs::remove_dir_all(dir).await.unwrap();
}

#[tokio::test]
#[ignore = "Build the owned fixture with scripts/test-mcp-runtime.sh"]
async fn executor_death_closes_parent_pipe_and_only_its_owned_process_group() {
    use std::process::Stdio;
    use tokio::io::AsyncBufReadExt;
    for mode in ["executor", "blocked-executor"] {
        let mut executor = tokio::process::Command::new(fixture_executable())
            .arg(mode)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut control = tokio::process::Command::new("/bin/sleep")
            .arg("20")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut reader = tokio::io::BufReader::new(executor.stdout.take().unwrap());
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        let ids: Value = serde_json::from_str(&line).unwrap();
        executor.kill().await.unwrap();
        wait_gone(ids["supervisor"].as_u64().unwrap() as u32).await;
        wait_gone(ids["backend"].as_u64().unwrap() as u32).await;
        assert!(
            control.try_wait().unwrap().is_none(),
            "unrelated owned test process must remain running"
        );
        control.kill().await.unwrap();
    }
}

#[tokio::test]
#[ignore = "Build the owned fixture and set synthetic credential environment with scripts/test-mcp-runtime.sh"]
async fn concurrent_startup_operation_leases_idle_threshold_sessions_and_rotation() {
    use recollect_mcp_runtime::{
        USEFUL_IDLE,
        manager::{InstanceKey, RuntimeManager, StartSpec},
    };
    use std::{sync::atomic::AtomicUsize, time::Instant};
    let dir = directory();
    let connection = Uuid::new_v4();
    let (resolver, secret) = credentials(&dir, connection, false).await;
    let marker = dir.join("lifecycle.txt");
    let executable = fixture_executable();
    let definition = support::definition(Some(executable.to_str().unwrap().into()));
    let configuration = json!({"marker":marker});
    let spec = StartSpec {
        definition: &definition,
        target: "fixed",
        configuration: &configuration,
        supervisor: &executable,
    };
    let manager = Arc::new(RuntimeManager::default());
    let recorded = Arc::new(AtomicUsize::new(0));
    let key = InstanceKey {
        brain_id: Uuid::new_v4(),
        profile_id: Uuid::new_v4(),
        connection_id: connection,
        actor_id: Uuid::new_v4(),
        device_id: Some(Uuid::new_v4()),
        client_session_id: Uuid::new_v4(),
        runner_epoch: Uuid::new_v4(),
        connection_revision: Uuid::new_v4(),
        definition_revision: "approved".into(),
        credential_generation: secret.generation,
    };
    let register = |_: Uuid| {
        recorded.fetch_add(1, Ordering::SeqCst);
        async { Ok(()) }
    };
    let (a, b) = tokio::join!(
        manager.acquire(key.clone(), spec, secret.clone(), register),
        manager.acquire(key.clone(), spec, secret.clone(), register)
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.instance_id(), b.instance_id());
    assert_eq!(recorded.load(Ordering::SeqCst), 1);
    let instance = a.instance_id();
    let long = tokio::spawn(async move {
        a.call(
            "slow",
            json!({"millis":500}),
            Duration::from_secs(3600),
            CancellationToken::new(),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if tokio::fs::read_to_string(&marker)
                .await
                .unwrap()
                .contains("call slow")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let mut other = key.clone();
    other.client_session_id = Uuid::new_v4();
    let c = manager
        .acquire(other.clone(), spec, secret.clone(), register)
        .await
        .unwrap();
    assert_ne!(instance, c.instance_id());
    let c_result = c
        .call(
            "inspect",
            json!({}),
            Duration::from_secs(3),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(c_result.state, "succeeded");
    manager.release_session(other.actor_id, other.client_session_id);
    let stopped = manager
        .maintain(Instant::now() + USEFUL_IDLE + Duration::from_secs(1))
        .await;
    assert_eq!(stopped.len(), 1);
    assert_ne!(stopped[0].0, instance);
    assert!(stopped[0].1.is_ok());
    assert_eq!(long.await.unwrap().state, "succeeded");
    assert_eq!(
        b.call(
            "inspect",
            json!({}),
            Duration::from_secs(3),
            CancellationToken::new()
        )
        .await
        .state,
        "succeeded"
    );
    let used = manager.statuses(Instant::now())[0].last_use;
    assert!(
        manager
            .maintain(used + USEFUL_IDLE - Duration::from_nanos(1))
            .await
            .is_empty()
    );
    let stopped = manager.maintain(used + USEFUL_IDLE).await;
    assert_eq!(stopped.len(), 1);
    assert_eq!(stopped[0].0, instance);
    assert!(stopped[0].1.is_ok());

    let old = manager
        .acquire(key.clone(), spec, secret.clone(), register)
        .await
        .unwrap();
    let path = dir.join("credential-references.json");
    let mut bindings: Value =
        serde_json::from_slice(&tokio::fs::read(&path).await.unwrap()).unwrap();
    bindings["bindings"][0]["environment"]["FIXTURE_CREDENTIAL"]["name"] =
        json!("RECOLLECT_MCP_SECRET_TEST_NEXT");
    tokio::fs::write(&path, serde_json::to_vec(&bindings).unwrap())
        .await
        .unwrap();
    let fresh = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    assert_ne!(fresh.generation, secret.generation);
    let mut next = key.clone();
    next.credential_generation = fresh.generation;
    let new = manager.acquire(next, spec, fresh, register).await.unwrap();
    assert_ne!(old.instance_id(), new.instance_id());
    assert_eq!(
        manager
            .statuses(Instant::now())
            .iter()
            .filter(|s| s.state == "draining")
            .count(),
        1
    );
    assert!(
        manager
            .maintain(Instant::now() + USEFUL_IDLE)
            .await
            .is_empty(),
        "active old and new leases survive rotation"
    );
    for lease in [old, new] {
        let reply = lease
            .call(
                "inspect",
                json!({}),
                Duration::from_secs(3),
                CancellationToken::new(),
            )
            .await;
        assert_eq!(reply.state, "succeeded");
        assert_eq!(
            reply.result.unwrap()["structuredContent"]["echo"],
            "[redacted]"
        );
    }
    manager.drain_all();
    assert_eq!(manager.maintain(Instant::now()).await.len(), 2);
    // A cancelled waiter or dropped async task must drain this instance while
    // preserving another already-held operation lease.
    let cancelled = manager
        .acquire(key.clone(), spec, secret.clone(), register)
        .await
        .unwrap();
    let survivor = manager
        .acquire(key.clone(), spec, secret.clone(), register)
        .await
        .unwrap();
    let token = CancellationToken::new();
    let cancel = token.clone();
    let previous = count_slow(&marker).await;
    let waiting = tokio::spawn(async move {
        cancelled
            .call(
                "slow",
                json!({"millis":5000}),
                Duration::from_secs(20),
                cancel,
            )
            .await
    });
    wait_slow(&marker, previous).await;
    token.cancel();
    assert_eq!(waiting.await.unwrap().state, "unknown");
    assert!(
        manager
            .maintain(Instant::now() + USEFUL_IDLE)
            .await
            .is_empty()
    );
    assert_eq!(
        survivor
            .call(
                "inspect",
                json!({}),
                Duration::from_secs(3),
                CancellationToken::new()
            )
            .await
            .state,
        "succeeded"
    );
    assert_eq!(manager.maintain(Instant::now()).await.len(), 1);
    let abandoned = manager
        .acquire(key.clone(), spec, secret.clone(), register)
        .await
        .unwrap();
    let previous = count_slow(&marker).await;
    let waiting = tokio::spawn(async move {
        abandoned
            .call(
                "slow",
                json!({"millis":5000}),
                Duration::from_secs(20),
                CancellationToken::new(),
            )
            .await
    });
    wait_slow(&marker, previous).await;
    waiting.abort();
    let _ = waiting.await;
    assert_eq!(manager.statuses(Instant::now())[0].state, "draining");
    assert_eq!(manager.maintain(Instant::now()).await.len(), 1);
    let recorded_before = recorded.load(Ordering::SeqCst);
    assert!(matches!(
        manager
            .acquire(key, spec, secret, |_| async {
                Err(RuntimeError("authority_lost"))
            })
            .await,
        Err(RuntimeError("authority_lost"))
    ));
    manager.maintain(Instant::now()).await;
    assert!(manager.statuses(Instant::now()).is_empty());
    assert_eq!(
        tokio::fs::read_to_string(&marker)
            .await
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("start "))
            .count(),
        recorded_before
    );
    tokio::fs::remove_dir_all(dir).await.unwrap();
}
async fn wait_gone(pid: u32) {
    #[cfg(unix)]
    {
        use nix::{sys::signal::kill, unistd::Pid};
        for _ in 0..150 {
            if kill(Pid::from_raw(pid as i32), None).is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("owned process {pid} was not reaped");
    }
}
async fn count_slow(marker: &Path) -> usize {
    tokio::fs::read_to_string(marker)
        .await
        .unwrap()
        .lines()
        .filter(|l| *l == "call slow")
        .count()
}
async fn wait_slow(marker: &Path, previous: usize) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while count_slow(marker).await <= previous {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
async fn credentials(
    dir: &Path,
    connection: Uuid,
    headers: bool,
) -> (CredentialResolver, Arc<ResolvedCredentials>) {
    tokio::fs::create_dir_all(dir).await.unwrap();
    let path = dir.join("credential-references.json");
    let mut binding =
        json!({"connection_id":connection,"alias":"fixture","runner_reference":"central"});
    if headers {
        binding["headers"] = json!({"Authorization":{"source":{"provider":"environment","name":"RECOLLECT_MCP_SECRET_TEST"},"prefix":"Bearer "}});
    } else {
        binding["environment"] = json!({"FIXTURE_CREDENTIAL":{"provider":"environment","name":"RECOLLECT_MCP_SECRET_TEST"}});
    }
    tokio::fs::write(
        &path,
        serde_json::to_vec(&json!({"bindings":[binding]})).unwrap(),
    )
    .await
    .unwrap();
    let resolver = CredentialResolver::new(Some(path));
    let resolved = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    (resolver, resolved)
}

#[tokio::test]
#[ignore = "Build the owned fixture and set synthetic credential environment with scripts/test-mcp-runtime.sh"]
async fn actual_stdio_calls_scrub_credentials_preserve_concurrency_and_reap_owned_processes() {
    let dir = directory();
    let id = Uuid::new_v4();
    let (resolver, secret) = credentials(&dir, id, false).await;
    let again = resolver
        .resolve(id, Some("fixture"), "central")
        .await
        .unwrap();
    assert_eq!(secret.generation, again.generation);
    assert!(matches!(
        resolver.resolve(id, Some("fixture"), "device:other").await,
        Err(RuntimeError("credential_binding_missing"))
    ));
    let executable = fixture_executable();
    let definition = support::definition(Some(executable.to_str().unwrap().into()));
    for legacy in [false, true] {
        let connected = Connected::open(
            &definition,
            "fixed-fixture-target",
            &json!({"legacy":legacy}),
            secret.clone(),
            &executable,
        )
        .await
        .unwrap_or_else(|error| panic!("stdio startup legacy={legacy}: {error}"));
        let pid = connected.owned_pid.unwrap();
        let reply = connected
            .call(
                "inspect",
                json!({"text":"permitted"}),
                Duration::from_secs(3),
                CancellationToken::new(),
            )
            .await;
        assert_eq!(reply.state, "succeeded", "{}", reply.code);
        let output = &reply.result.as_ref().unwrap()["structuredContent"];
        assert_eq!(output["ambient_leaked"], false);
        assert_eq!(output["device_leaked"], false);
        assert_eq!(output["echo"], "[redacted]");
        assert_eq!(output["target"], "fixed-fixture-target");
        assert!(
            !serde_json::to_string(&reply)
                .unwrap()
                .contains(&std::env::var("RECOLLECT_MCP_SECRET_TEST").unwrap())
        );
        let child_pid = output["pid"].as_u64().unwrap() as u32;
        let (slow, fast) = tokio::join!(
            connected.call(
                "slow",
                json!({"millis":200}),
                Duration::from_secs(3),
                CancellationToken::new()
            ),
            connected.call(
                "inspect",
                json!({}),
                Duration::from_secs(3),
                CancellationToken::new()
            )
        );
        assert_eq!(slow.state, "succeeded");
        assert_eq!(fast.state, "succeeded");
        let invalid = connected
            .call(
                "inspect",
                json!({"target":"replacement"}),
                Duration::from_secs(1),
                CancellationToken::new(),
            )
            .await;
        assert_eq!(invalid.code, "tool_arguments_invalid");
        let error = connected
            .call(
                "error",
                json!({}),
                Duration::from_secs(1),
                CancellationToken::new(),
            )
            .await;
        assert_eq!(error.state, "tool_error");
        let rejected = connected
            .call(
                "reject",
                json!({}),
                Duration::from_secs(1),
                CancellationToken::new(),
            )
            .await;
        assert_eq!(rejected.state, "failed");
        assert_eq!(rejected.code, "provider_protocol_rejected");
        connected.close().await.unwrap();
        wait_gone(pid).await;
        wait_gone(child_pid).await;
    }
    tokio::fs::remove_dir_all(&dir).await.unwrap();
}

#[tokio::test]
#[ignore = "Build the owned fixture with scripts/test-mcp-runtime.sh"]
async fn stdio_lost_response_and_oversize_output_remain_unknown_without_replay() {
    let dir = directory();
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let marker = dir.join("effect-metadata.txt");
    let executable = fixture_executable();
    let definition = support::definition(Some(executable.to_str().unwrap().into()));
    let connected = Connected::open(
        &definition,
        "fixture",
        &json!({"marker":marker}),
        ResolvedCredentials::anonymous(),
        &executable,
    )
    .await
    .unwrap();
    let pid = connected.owned_pid.unwrap();
    let lost = connected
        .call(
            "effect",
            json!({"operation_id":Uuid::new_v4()}),
            Duration::from_secs(3),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(lost.state, "unknown");
    assert!(lost.result.is_none());
    connected.close().await.unwrap();
    wait_gone(pid).await;
    let record = tokio::fs::read_to_string(&marker).await.unwrap();
    assert_eq!(
        record.lines().filter(|s| s.starts_with("start ")).count(),
        1
    );
    assert_eq!(record.lines().filter(|s| *s == "call effect").count(), 1);
    let connected = Connected::open(
        &definition,
        "fixture",
        &json!({}),
        ResolvedCredentials::anonymous(),
        &executable,
    )
    .await
    .unwrap();
    let pid = connected.owned_pid.unwrap();
    let huge = connected
        .call(
            "large",
            json!({}),
            Duration::from_secs(3),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(huge.state, "unknown");
    assert!(huge.result.is_none());
    connected.close().await.unwrap();
    wait_gone(pid).await;
    tokio::fs::remove_dir_all(dir).await.unwrap();
}

#[tokio::test]
#[ignore = "Set synthetic credential environment with scripts/test-mcp-runtime.sh"]
async fn actual_http_modern_legacy_json_sse_no_session_replay_and_host_survives_close() {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::Request,
        middleware::{self, Next},
        response::IntoResponse,
        routing::get,
    };
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };
    let dir = directory();
    let id = Uuid::new_v4();
    let (_resolver, secret) = credentials(&dir, id, true).await;
    let expected = format!(
        "Bearer {}",
        std::env::var("RECOLLECT_MCP_SECRET_TEST").unwrap()
    );
    for (legacy, json_response) in [(false, true), (false, false), (true, false)] {
        let fixture = support::Fixture {
            legacy,
            ..Default::default()
        };
        let calls = fixture.calls.clone();
        let mut config = StreamableHttpServerConfig::default();
        config.legacy_session_mode = legacy;
        config.json_response = json_response;
        let shutdown = config.cancellation_token.clone();
        let service = StreamableHttpService::new(
            move || Ok(fixture.clone()),
            Arc::new(LocalSessionManager::default()),
            config,
        );
        let expected = expected.clone();
        let app = Router::new()
            .route_service("/mcp", service)
            .layer(middleware::from_fn(
                move |request: Request<Body>, next: Next| {
                    let expected = expected.clone();
                    async move {
                        if request
                            .headers()
                            .get("authorization")
                            .and_then(|h| h.to_str().ok())
                            != Some(expected.as_str())
                        {
                            return axum::http::StatusCode::UNAUTHORIZED.into_response();
                        }
                        let (parts, body) = request.into_parts();
                        let bytes = to_bytes(body, recollect_mcp_runtime::WIRE_LIMIT)
                            .await
                            .unwrap();
                        let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                        let effect =
                            value["method"] == "tools/call" && value["params"]["name"] == "effect";
                        let reply = next
                            .run(Request::from_parts(parts, Body::from(bytes)))
                            .await;
                        if effect {
                            // Finish the actual SDK handler, then lose its response as 404.
                            let _ = to_bytes(reply.into_body(), recollect_mcp_runtime::WIRE_LIMIT)
                                .await
                                .unwrap();
                            axum::http::StatusCode::NOT_FOUND.into_response()
                        } else {
                            reply
                        }
                    }
                },
            ))
            .route("/alive", get(|| async { "alive" }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let target = format!("http://{address}/mcp");
        let definition = support::definition(None);
        let connected = Connected::open(
            &definition,
            &target,
            &json!({}),
            secret.clone(),
            Path::new("unused-http-supervisor"),
        )
        .await
        .unwrap();
        assert_eq!(connected.owned_pid, None);
        let reply = connected
            .call(
                "inspect",
                json!({"text":"http"}),
                Duration::from_secs(3),
                CancellationToken::new(),
            )
            .await;
        assert_eq!(
            reply.state, "succeeded",
            "{} (legacy {legacy}, json {json_response})",
            reply.code
        );
        let before = calls.load(Ordering::SeqCst);
        let effect = connected
            .call(
                "effect",
                json!({}),
                Duration::from_secs(3),
                CancellationToken::new(),
            )
            .await;
        assert_eq!(effect.state, "unknown");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            before + 1,
            "SDK must not replay expired-session calls"
        );
        connected.close().await.unwrap();
        let oversized = Connected::open(
            &definition,
            &target,
            &json!({}),
            secret.clone(),
            Path::new("unused-http-supervisor"),
        )
        .await
        .unwrap();
        let reply = oversized
            .call(
                "large",
                json!({}),
                Duration::from_secs(3),
                CancellationToken::new(),
            )
            .await;
        assert_eq!(reply.state, "unknown");
        assert!(reply.result.is_none());
        oversized.close().await.unwrap();
        assert_eq!(
            reqwest::get(format!("http://{address}/alive"))
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "alive"
        );
        shutdown.cancel();
        server.abort();
        let _ = server.await;
    }
    tokio::fs::remove_dir_all(dir).await.unwrap();
}
