use recollect_agent::publication;
use serde_json::Value;
use std::{path::Path, time::Duration};
use tokio::process::Command;
use uuid::Uuid;

#[tokio::test]
async fn native_host_settings_need_no_network_or_secret_and_freeze_the_directory() {
    let directory = publication::project_root()
        .join(".cache")
        .join(format!("mcp-config 'quoted' {}", Uuid::new_v4()));
    tokio::fs::create_dir_all(&directory).await.unwrap();
    let brain = Uuid::new_v4();
    let executable = Path::new(env!("CARGO_BIN_EXE_recollect-agent"));
    for host in ["codex", "claude"] {
        let output = tokio::time::timeout(
            Duration::from_secs(5),
            Command::new(executable)
                .env_clear()
                .env("RECOLLECT_URL", "http://127.0.0.1:1")
                .env("RECOLLECT_DEVICE_PROFILE", "not-paired-config-fixture")
                .current_dir(&directory)
                .args([
                    "mcp-config",
                    host,
                    "--brain",
                    &brain.to_string(),
                    "--directory",
                    ".",
                ])
                .kill_on_drop(true)
                .output(),
        )
        .await
        .expect("settings should not wait on a service or OS credential")
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let generated: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(generated["state"], "configured_only");
        let server: Value = if host == "codex" {
            let parsed: toml::Value =
                toml::from_str(generated["configuration"].as_str().unwrap()).unwrap();
            let inline = generated["host_arguments"][1].as_str().unwrap();
            let args: toml::Value = toml::from_str(inline).unwrap();
            assert_eq!(
                parsed["mcp_servers"]["recollect"],
                args["mcp_servers"]["recollect"]
            );
            serde_json::to_value(&parsed["mcp_servers"]["recollect"]).unwrap()
        } else {
            generated["configuration"]["mcpServers"]["recollect"].clone()
        };
        assert_eq!(server["args"][4], serde_json::json!(directory));
        assert_eq!(server["env"].as_object().unwrap().len(), 2);
        assert_eq!(server["env"]["RECOLLECT_URL"], "http://127.0.0.1:1");
        if cfg!(target_os = "linux") && host == "codex" {
            assert_eq!(
                server["env_vars"],
                serde_json::json!(["DBUS_SESSION_BUS_ADDRESS", "XDG_RUNTIME_DIR"])
            );
        } else {
            assert!(server.get("env_vars").is_none());
        }
        assert!(server.get("token").is_none());
    }
    tokio::fs::remove_dir(&directory).await.unwrap();
}
