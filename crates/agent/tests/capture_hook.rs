use chrono::Utc;
use recollect_agent::{
    capture::{CachedCaptureBinding, Inbox, profile_root},
    capture_cli::HookSetup,
    capture_setup, publication,
};
use recollect_protocol::*;
use serde_json::json;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use uuid::Uuid;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn run(command: &str, raw: &[u8], setup: &Path) -> (std::process::Output, Duration) {
    let start = Instant::now();
    let mut child = Command::new("/bin/sh")
        .args(["-c", command])
        .env("RECOLLECT_URL", "invalid-url-must-not-be-used-by-hook")
        .env("RECOLLECT_DEVICE_PROFILE", "")
        .env("CAPTURE_TEST_SECRET", "synthetic-configured-credential")
        .env("RECOLLECT_CAPTURE_SETUP", setup)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.take().unwrap().write_all(raw); // Oversized input can close the pipe early.
    let output = child.wait_with_output().unwrap();
    (output, start.elapsed())
}

#[test]
fn hook_process_is_local_nonblocking_and_keeps_only_sanitized_events() {
    let root = Fixture(
        publication::project_root()
            .join(".cache")
            .join(format!("capture-process-{}", Uuid::new_v4())),
    );
    fs::create_dir_all(&root.0).unwrap();
    let host_directory = root.0.join("host 'quoted' directory");
    fs::create_dir(&host_directory).unwrap();
    let now = Utc::now();
    let device = Uuid::new_v4();
    let brain = Uuid::new_v4();
    let task = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let saved = CachedCaptureBinding {
        binding: CaptureBinding {
            id: binding,
            brain_id: brain,
            device_id: device,
            host: "codex".into(),
            host_version: "0.154.0".into(),
            created_at: now,
            operation: OperationBinding {
                id: Uuid::new_v4(),
                brain_id: brain,
                task_id: task,
                scope: ScopeSnapshot {
                    id: Uuid::new_v4(),
                    task_id: task,
                    brain_id: brain,
                    selection: ScopeSelection::default(),
                    repositories: vec![],
                    areas: vec![],
                    environment: None,
                    created_at: now,
                },
                scope_valid: true,
                kind: "capture".into(),
                actor_id: Uuid::new_v4(),
                device_id: Some(device),
                created_at: now,
            },
        },
        policy: CapturePolicy {
            enabled: true,
            ..Default::default()
        },
        retention: RetentionPolicy::default(),
        synchronized_at: now,
        agent_id: None,
    };
    let endpoint = "http://127.0.0.1:1";
    let inbox_root = profile_root(&root.0, device);
    let mut inbox = Inbox::open(&inbox_root, endpoint, device).unwrap();
    inbox.remember(&saved).unwrap();
    let setup = HookSetup {
        endpoint: endpoint.into(),
        device_id: device,
        brain_id: brain,
        binding_id: binding,
        evidence_root: root.0.clone(),
        device_profile: "unused-fixture".into(),
        host: "codex".into(),
        working_directory: root.0.clone(),
        task_id: task,
        created_task: true,
        plugin_root: None,
    };
    let setup_path = host_directory.join("capture.json");
    fs::write(&setup_path, serde_json::to_vec(&setup).unwrap()).unwrap();
    let transcript = root.0.join("transcript.json");
    fs::write(&transcript, "DO_NOT_READ_SYNTHETIC_TRANSCRIPT").unwrap();
    let executable = Path::new(env!("CARGO_BIN_EXE_recollect-agent"));
    let config = capture_setup::hooks("codex", executable, &setup_path).unwrap();
    let command = config["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    assert!(capture_own_transport(command));
    let raw=serde_json::to_vec(&json!({"hook_event_name":"UserPromptSubmit","session_id":"process-session","turn_id":"process-turn","transcript_path":transcript,
        "prompt":"Keep this synthetic fact: Amber.port = 8080\nsynthetic-configured-credential\nAPI_KEY=synthetic-inline-credential"})).unwrap();
    let (output, elapsed) = run(command, &raw, &setup_path);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "Local hook took {elapsed:?}"
    );
    eprintln!(
        "Native hook process completed in {} ms",
        elapsed.as_millis()
    );
    let pending = inbox.pending(Utc::now()).unwrap();
    assert_eq!(pending.len(), 1);
    let text = pending[0].event.content.as_ref().unwrap();
    assert!(text.contains("8080"));
    assert!(!text.contains("synthetic-configured-credential"));
    assert!(!text.contains("synthetic-inline-credential"));
    assert!(!text.contains("DO_NOT_READ_SYNTHETIC_TRANSCRIPT"));
    assert!(run(command, &raw, &setup_path).0.status.success());
    assert_eq!(inbox.pending(Utc::now()).unwrap()[0].id, pending[0].id);
    for bytes in [
        b"RAW_INVALID_EVENT_MUST_NOT_PERSIST".to_vec(),
        vec![b'x'; CAPTURE_STDIN_BYTES + 200],
    ] {
        let (output, elapsed) = run(command, &bytes, &setup_path);
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "Recollect capture gap. Check the companion's capture status.\n"
        );
        assert!(elapsed < Duration::from_secs(1));
    }
    let gaps = inbox.status().unwrap().gaps;
    assert!(
        gaps.iter()
            .any(|(code, n)| code == "malformed_capture_event" && *n == 1)
    );
    assert!(
        gaps.iter()
            .any(|(code, n)| code == "capture_input_too_large" && *n == 1)
    );
    for path in fs::read_dir(&inbox_root).unwrap() {
        let bytes = fs::read(path.unwrap().path()).unwrap();
        for marker in [
            "synthetic-configured-credential",
            "synthetic-inline-credential",
            "RAW_INVALID_EVENT_MUST_NOT_PERSIST",
            "DO_NOT_READ_SYNTHETIC_TRANSCRIPT",
        ] {
            assert!(!bytes.windows(marker.len()).any(|w| w == marker.as_bytes()));
        }
    }
    let output = Command::new(executable)
        .args(["capture", "status"])
        .arg(&setup_path)
        .env("RECOLLECT_URL", "invalid-local-status-endpoint")
        .env("RECOLLECT_DEVICE_PROFILE", "")
        .output()
        .unwrap();
    assert!(output.status.success());
    let status: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["connection"], "not_checked");
    assert_eq!(status["inbox"]["pending"], 1);
    drop(inbox);
}
