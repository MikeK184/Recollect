//! Real installed API/proxy and paired OS store; no wire or credential substitute.
use rmcp::{RoleClient, ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
use tokio::process::Command;

async fn tool(peer: &rmcp::Peer<RoleClient>, name: &str, input: Value) -> Value {
    let reply = peer
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(input.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(reply.is_error, Some(true), "Installed tool failed: {name}");
    reply.structured_content.unwrap()
}

fn command(binary: &str, trusted: bool) -> Command {
    let mut command = Command::new(PathBuf::from("/workspace/target/debug").join(binary));
    command.env_clear().kill_on_drop(true);
    for key in [
        "PATH",
        "HOME",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_RUNTIME_DIR",
        "RECOLLECT_URL",
        "RECOLLECT_DEVICE_PROFILE",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    if trusted {
        command.env(
            "RECOLLECT_CA_FILE",
            std::env::var_os("RECOLLECT_CA_FILE").unwrap(),
        );
    }
    command
}

#[tokio::test]
#[ignore = "Owned installed HTTPS, browser pairing and Linux OS-store fixture"]
async fn installed_https_native_pairing_and_scoped_memory() {
    assert_eq!(
        std::env::var("RECOLLECT_URL").unwrap(),
        "https://localhost:8443"
    );
    assert!(
        std::env::var("RECOLLECT_DEVICE_PROFILE")
            .unwrap()
            .starts_with("installation-proof-")
    );
    let brain = std::env::var("RECOLLECT_TEST_BRAIN")
        .unwrap()
        .parse::<uuid::Uuid>()
        .unwrap();
    let marker = std::env::var("RECOLLECT_TEST_MARKER").unwrap();
    // Every invocation is a fresh native process reading the real OS credential.
    let identity = command("recollect-agent", true)
        .arg("whoami")
        .output()
        .await
        .unwrap();
    assert!(identity.status.success());
    let identity: Value = serde_json::from_slice(&identity.stdout).unwrap();
    assert_eq!(identity["username"], "owner");
    let brains = command("recollect-agent", true)
        .arg("brains")
        .output()
        .await
        .unwrap();
    assert!(brains.status.success());
    let brains: Value = serde_json::from_slice(&brains.stdout).unwrap();
    assert!(
        brains
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["id"] == json!(brain))
    );
    for (trusted, endpoint) in [
        (false, "https://localhost:8443"),
        (true, "https://127.0.0.1:8443"),
    ] {
        let health = command("recollect-agent", trusted)
            .env("RECOLLECT_URL", endpoint)
            .arg("health")
            .output()
            .await
            .unwrap();
        assert!(
            !health.status.success(),
            "Native client ignored certificate trust or hostname"
        );
    }
    let connect = |trusted| {
        let mut native = command("recollect-mcp-bridge", trusted);
        native.args([
            "--brain",
            &brain.to_string(),
            "--directory",
            "/tmp/installation-workspace",
        ]);
        ().serve(TokioChildProcess::new(native).unwrap())
    };
    let rejected = tokio::time::timeout(Duration::from_secs(25), connect(false))
        .await
        .unwrap();
    assert!(rejected.is_err(), "MCP bridge admitted an untrusted CA");
    let service = tokio::time::timeout(Duration::from_secs(25), connect(true))
        .await
        .unwrap()
        .unwrap();
    let peer = service.peer();
    assert!(
        peer.list_all_tools()
            .await
            .unwrap()
            .iter()
            .any(|tool| tool.name == "memory.recall")
    );
    let task = tool(
        peer,
        "workspace.start_task",
        json!({
            "input":{"label":"Installed native HTTPS proof", "selection":null},
            "context_query":marker
        }),
    )
    .await;
    assert_eq!(task["context"]["state"], "ready");
    let recalled = tool(
        peer,
        "memory.recall",
        json!({
            "operation_id":task["context"]["operation_id"], "input":{"query":marker}
        }),
    )
    .await;
    assert_eq!(recalled["brain_id"], json!(brain));
    assert!(
        recalled["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["text"]
                .as_str()
                .is_some_and(|text| text.contains(&marker))),
        "Installed native memory read did not return its permitted retained source"
    );
    service.cancel().await.unwrap();
}
