//! Concurrent hooks of one session must be serialized, not dropped. Claude Code
//! can run PostToolUse hooks in parallel for the same native session.
use chrono::Utc;
use recollect_agent::{
    capture::{CachedCaptureBinding, Inbox, profile_root},
    capture_cli::HookSetup,
    plugin_runtime, plugin_session, plugin_storage, publication,
};
use recollect_protocol::*;
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use uuid::Uuid;

struct Fixture {
    root: PathBuf,
    cwd: PathBuf,
    config: plugin_storage::Config,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.root.parent().unwrap());
    }
}

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

impl Fixture {
    fn new() -> Self {
        let base = publication::project_root()
            .join(".cache")
            .join(format!("plugin-hook-{}", Uuid::new_v4()));
        let root = base.join("plugin");
        let cwd = base.join("workspace");
        fs::create_dir_all(&cwd).unwrap();
        // The runtime resolves its storage from the environment. This is the only
        // test in this binary, so the process-wide variable is not contended.
        unsafe { std::env::set_var("RECOLLECT_PLUGIN_DATA", &root) };
        plugin_storage::directory(&root).unwrap();
        let config = plugin_storage::Config {
            endpoint: "http://127.0.0.1:1".into(),
            brain: Uuid::new_v4(),
            device: Uuid::new_v4(),
            profile: "hook-contention-fixture".into(),
            with_runner: false,
            runner_id: None,
        };
        plugin_storage::write(&root.join("config.json"), &config).unwrap();
        Self {
            root,
            cwd: cwd.canonicalize().unwrap(),
            config,
        }
    }

    /// An existing session with a cached capture binding, as after a prompt boundary.
    fn session(&self, native: &str) -> PathBuf {
        let now = Utc::now();
        let task = Uuid::new_v4();
        let scope = ScopeSnapshot {
            id: Uuid::new_v4(),
            task_id: task,
            brain_id: self.config.brain,
            selection: ScopeSelection::default(),
            repositories: vec![],
            areas: vec![],
            environment: None,
            created_at: now,
        };
        let binding = CaptureBinding {
            id: Uuid::new_v4(),
            brain_id: self.config.brain,
            device_id: self.config.device,
            host: "codex".into(),
            host_version: "0.154.0".into(),
            created_at: now,
            operation: OperationBinding {
                id: Uuid::new_v4(),
                brain_id: self.config.brain,
                task_id: task,
                scope: scope.clone(),
                scope_valid: true,
                kind: "capture".into(),
                actor_id: Uuid::new_v4(),
                device_id: Some(self.config.device),
                created_at: now,
            },
        };
        let session = plugin_session::Session {
            id: Uuid::new_v4(),
            setup: HookSetup {
                endpoint: self.config.endpoint.clone(),
                device_id: self.config.device,
                brain_id: self.config.brain,
                binding_id: binding.id,
                evidence_root: self.root.clone(),
                device_profile: self.config.profile.clone(),
                host: "codex".into(),
                working_directory: self.cwd.clone(),
                task_id: task,
                created_task: true,
                plugin_root: None,
            },
            scope,
            host_version: "0.154.0".into(),
            agent_id: None,
            ended: false,
            closed: false,
            last_event_at: now,
        };
        let mut inbox = self.inbox();
        inbox
            .remember(&CachedCaptureBinding {
                binding: binding.clone(),
                policy: CapturePolicy {
                    enabled: true,
                    ..Default::default()
                },
                retention: RetentionPolicy::default(),
                synchronized_at: now,
                agent_id: None,
            })
            .unwrap();
        inbox.start_launch(session.id, binding.id).unwrap();
        let state =
            plugin_session::state_path(&self.root, &self.config, "codex", native, &self.cwd)
                .unwrap();
        plugin_storage::write(&state, &session).unwrap();
        state
    }

    fn inbox(&self) -> Inbox {
        Inbox::open(
            &profile_root(&self.root, self.config.device),
            &self.config.endpoint,
            self.config.device,
        )
        .unwrap()
    }

    fn event(&self, native: &str, tool: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "hook_event_name":"PostToolUse","session_id":native,"cwd":self.cwd,
            "turn_id":"turn-1","tool_use_id":tool,"tool_name":"Bash",
            "tool_input":{"command":format!("echo {tool}")},
            "tool_response":{"stdout":tool,"exit_code":0}
        }))
        .unwrap()
    }
}

fn hold(state: &Path, how_long: Duration) -> std::thread::JoinHandle<()> {
    let lock = plugin_storage::private_file(&state.with_extension("lock"), false).unwrap();
    lock.lock().unwrap();
    std::thread::spawn(move || {
        std::thread::sleep(how_long);
        drop(lock);
    })
}

#[test]
fn concurrent_hooks_of_one_session_retain_every_event() {
    let f = Fixture::new();

    // The runtime pointer is rewritten (two fsyncs) only when its content changes.
    plugin_storage::runtime_pointer().unwrap();
    let pointer = f.root.join("runtime-path");
    let first = fs::metadata(&pointer).unwrap().modified().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    plugin_storage::runtime_pointer().unwrap();
    assert_eq!(fs::metadata(&pointer).unwrap().modified().unwrap(), first);
    fs::write(&pointer, "/stale/path\n").unwrap();
    plugin_storage::runtime_pointer().unwrap();
    assert_ne!(fs::read_to_string(&pointer).unwrap(), "/stale/path\n");

    // 1. A holder that lets go inside the wait: the hook waits and captures.
    let state = f.session("held");
    let holder = hold(&state, Duration::from_millis(400));
    let started = Instant::now();
    block_on(plugin_runtime::hook(
        "codex",
        &f.event("held", "tool-after-wait"),
    ))
    .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(300));
    holder.join().unwrap();
    assert_eq!(f.inbox().status().unwrap().pending, 1);

    // 2. Two hooks racing for one session, each on its own runtime like two
    // separate hook processes: both events are retained.
    let _ = f.session("race");
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let raw = f.event("race", &format!("tool-race-{n}"));
            std::thread::spawn(move || block_on(plugin_runtime::hook("codex", &raw)))
        })
        .collect();
    for handle in handles {
        handle.join().unwrap().unwrap();
    }
    let status = f.inbox().status().unwrap();
    assert_eq!(status.pending, 3, "both racing events must be queued");
    assert!(
        !status
            .gaps
            .iter()
            .any(|(code, _)| code == "capture_hook_contended")
    );

    // 3. A holder that outlasts the wait: the event cannot be captured, and the
    // loss is recorded as a gap instead of vanishing.
    let state = f.session("stuck");
    let holder = hold(&state, Duration::from_millis(1500));
    let deadline = tokio::time::Instant::now() + Duration::from_millis(3300);
    let result = block_on(plugin_runtime::hook_until(
        "codex",
        &f.event("stuck", "tool-lost"),
        deadline,
    ));
    assert!(result.is_err());
    holder.join().unwrap();
    let status = f.inbox().status().unwrap();
    assert_eq!(status.pending, 3);
    assert!(
        status
            .gaps
            .iter()
            .any(|(code, n)| code == "capture_hook_contended" && *n == 1),
        "{:?}",
        status.gaps
    );
}
