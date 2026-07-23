use crate::{
    Client, StoredDevice,
    capture::{CachedCaptureBinding, Inbox, profile_root},
    capture_cli::{HookSetup, USAGE},
    decode, privacy, publication, workspace,
};
use anyhow::{Result, anyhow, ensure};
use chrono::Utc;
use recollect_protocol::{
    CaptureBinding, CaptureSettings, CreateTask, OperationBinding, TaskChange, capture_identity,
};
use reqwest::Method;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};
use uuid::Uuid;

pub struct SetupOptions {
    pub host: String,
    pub host_version: String,
    pub directory: PathBuf,
    pub brain: Option<Uuid>,
    pub task: Option<Uuid>,
    pub agent_id: Option<String>,
    pub output: Option<PathBuf>,
    pub executable: PathBuf,
    pub device_profile: String,
    pub evidence_root: PathBuf,
}
#[derive(Serialize)]
pub struct PreparedCapture {
    pub setup_file: PathBuf,
    pub hooks_file: PathBuf,
    pub brain_url: String,
    pub binding: CaptureBinding,
    pub capture_enabled: bool,
    pub state: &'static str,
    pub host_arguments: Vec<String>,
    pub host_notice: &'static str,
    pub run_command: String,
}
fn shell_word(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}
pub fn hooks(host: &str, executable: &Path, setup: &Path) -> Result<Value> {
    let executable = executable
        .to_str()
        .ok_or_else(|| anyhow!("Capture executable needs a UTF-8 path."))?;
    let setup = setup
        .to_str()
        .ok_or_else(|| anyhow!("Capture setup needs a UTF-8 path."))?;
    ensure!(
        !executable.chars().any(char::is_control) && !setup.chars().any(char::is_control),
        "Capture hook paths must not contain control characters."
    );
    let command = if host == "codex" {
        format!("{} capture hook", shell_word(executable))
    } else {
        format!(
            "{} capture hook {}",
            shell_word(executable),
            shell_word(setup)
        )
    };
    let mut events = vec![
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "Stop",
        "SubagentStart",
        "SubagentStop",
        "PreCompact",
        "PostCompact",
        "SessionEnd",
    ];
    match host {
        "codex" => events.push("Interrupt"),
        "claude_code" => events.extend(["PostToolUseFailure", "StopFailure"]),
        _ => return Err(anyhow!("Choose codex or claude_code.")),
    }
    let mut hooks = serde_json::Map::new();
    for event in events {
        hooks.insert(
            event.into(),
            json!([{"hooks":[{"type":"command","command":command,"timeout":3}]}]),
        );
    }
    Ok(json!({"hooks":hooks}))
}
pub fn host_arguments(host: &str, hooks_file: &Path, config: &Value) -> Result<Vec<String>> {
    if host == "claude_code" {
        return Ok(vec![
            "--settings".into(),
            hooks_file
                .to_str()
                .ok_or_else(|| anyhow!("Use UTF-8 hook paths."))?
                .into(),
        ]);
    }
    ensure!(host == "codex", "Choose codex or claude_code.");
    ensure!(config["hooks"].is_object(), "Invalid generated hooks.");
    Ok(vec![
        "--enable".into(),
        "hooks".into(),
        "--config".into(),
        "plugins.\"recollect-capture@recollect-capture\".enabled=true".into(),
    ])
}
/// The same local marketplace/plugin shape used by Cognee's Codex integration.
/// Explicit managed launch registers this source through the host's installer.
pub async fn codex_plugin(root: &Path, hooks: &Value) -> Result<()> {
    async fn generated(path: &Path, content: &[u8]) -> Result<()> {
        // These exact paths are application-owned generated plugin files.
        let tmp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
        publication::write_private(&tmp, content).await?;
        tokio::fs::rename(&tmp, path).await?;
        Ok(())
    }
    let manifests = publication::private_root(&root.join(".agents/plugins")).await?;
    let plugin =
        publication::private_root(&root.join("plugins/recollect-capture/.codex-plugin")).await?;
    let commands = publication::private_root(&root.join("plugins/recollect-capture/hooks")).await?;
    generated(&plugin.join("plugin.json"), &serde_json::to_vec_pretty(&json!({
        "name":"recollect-capture", "version":"0.1.0", "description":"Capture permitted session evidence into Recollect.",
        "author":{"name":"Recollect"},
        "interface":{"displayName":"Recollect capture","shortDescription":"Automatic session evidence capture","category":"Productivity",
          "longDescription":"Capture supported prompts, replies and tool observations into the native Recollect companion under the active launch binding.",
          "developerName":"Recollect","capabilities":["Read","Write"],"defaultPrompt":"Check Recollect session capture activity."}
    }))?).await?;
    generated(
        &commands.join("hooks.json"),
        &serde_json::to_vec_pretty(hooks)?,
    )
    .await?;
    generated(&manifests.join("marketplace.json"),&serde_json::to_vec_pretty(&json!({
        "name":"recollect-capture", "interface":{"displayName":"Recollect capture"},
        "plugins":[{"name":"recollect-capture","source":{"source":"local","path":"./plugins/recollect-capture"},
        "policy":{"installation":"AVAILABLE","authentication":"ON_INSTALL"},"category":"Productivity"}]
    }))?).await?;
    Ok(())
}
pub async fn register_codex_plugin(root: &Path) -> Result<()> {
    let root = publication::private_root(root).await?;
    let root = root
        .to_str()
        .ok_or_else(|| anyhow!("Use UTF-8 plugin paths."))?;
    for args in [
        vec!["plugin", "marketplace", "add", root],
        vec!["plugin", "add", "recollect-capture@recollect-capture"],
    ] {
        let mut child = Command::new("codex")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| anyhow!("Codex could not register the Recollect capture plugin."))?;
        let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
            .await
            .map_err(|_| anyhow!("Codex plugin registration timed out. Retry the launch."))??;
        ensure!(
            status.success(),
            "Codex could not register the generated plugin. Inspect its plugin configuration and retry."
        );
    }
    Ok(())
}
pub async fn refresh_version(
    client: &Client,
    device: &StoredDevice,
    path: &Path,
    setup: &HookSetup,
    installed_version: &str,
    executable: &Path,
) -> Result<(PathBuf, HookSetup)> {
    let path = tokio::fs::canonicalize(path).await?;
    let mut inbox = Inbox::open(
        &profile_root(&setup.evidence_root, device.device_id),
        &client.endpoint,
        device.device_id,
    )?;
    let saved = inbox.cached_binding(setup.binding_id)?;
    ensure!(
        saved.binding.host == setup.host && saved.binding.brain_id == setup.brain_id,
        "Capture setup does not match its original binding."
    );
    if saved.binding.host_version == installed_version {
        return Ok((path, setup.clone()));
    }
    // Reusing the original launch command after an upgrade reuses its refreshed
    // immutable setup. Active hooks keep their own setup file unchanged.
    for existing in inbox.cached_bindings()?.into_iter().filter(|b| {
        b.binding.operation.id == saved.binding.operation.id
            && b.binding.host == setup.host
            && b.binding.host_version == installed_version
    }) {
        let candidate = profile_root(&setup.evidence_root, device.device_id)
            .join("hosts")
            .join(existing.binding.id.to_string())
            .join("capture.json");
        if let Ok(fresh) = crate::capture_cli::read_setup(&candidate).await
            && fresh.binding_id == existing.binding.id
            && fresh.device_id == device.device_id
            && fresh.brain_id == setup.brain_id
        {
            return Ok((candidate, fresh));
        }
    }
    let binding: CaptureBinding = decode(
        client
            .send(
                Method::POST,
                &format!("/api/brains/{}/capture/bindings", setup.brain_id),
                Some(device.token),
                Some(
                    json!({"id":Uuid::new_v4(),"operation_id":saved.binding.operation.id,
            "host":setup.host,"host_version":installed_version}),
                ),
            )
            .await?,
    )
    .await?;
    ensure!(
        binding.operation.id == saved.binding.operation.id
            && binding.brain_id == setup.brain_id
            && binding.device_id == device.device_id,
        "Refreshed capture binding does not match its original operation."
    );
    let fresh = HookSetup {
        binding_id: binding.id,
        ..setup.clone()
    };
    inbox.remember(&CachedCaptureBinding { binding, ..saved })?;
    let directory = publication::private_root(
        &profile_root(&setup.evidence_root, device.device_id)
            .join("hosts")
            .join(fresh.binding_id.to_string()),
    )
    .await?;
    let path = directory.join("capture.json");
    let hooks = hooks(&setup.host, executable, &path)?;
    publication::write_private(&path, &serde_json::to_vec_pretty(&fresh)?).await?;
    publication::write_private(
        &directory.join("hooks.json"),
        &serde_json::to_vec_pretty(&hooks)?,
    )
    .await?;
    if let Some(plugin) = &fresh.plugin_root {
        codex_plugin(plugin, &hooks).await?;
    }
    Ok((path, fresh))
}
pub async fn prepare(
    client: &Client,
    device: &StoredDevice,
    options: SetupOptions,
) -> Result<PreparedCapture> {
    ensure!(
        device.endpoint == client.endpoint,
        "Capture setup belongs to another service."
    );
    ensure!(
        capture_identity(&options.host_version) && options.host_version.len() <= 120,
        "Invalid host version."
    );
    ensure!(
        options.agent_id.as_deref().is_none_or(capture_identity),
        "Use a bounded opaque agent ID."
    );
    ensure!(
        !options.device_profile.is_empty()
            && options.device_profile.len() <= 80
            && !options.device_profile.chars().any(char::is_control),
        "Invalid device profile."
    );
    let directory = tokio::fs::canonicalize(&options.directory)
        .await
        .map_err(|_| anyhow!("Choose an existing host workspace."))?;
    ensure!(directory.is_dir(), "Choose a host workspace directory.");
    let output = if let Some(output) = &options.output {
        let output = publication::private_root(output).await?;
        for name in ["capture.json", "hooks.json"] {
            ensure!(
                tokio::fs::symlink_metadata(output.join(name))
                    .await
                    .is_err(),
                "Capture setup files already exist. Choose a new output directory."
            );
        }
        Some(output)
    } else {
        None
    };
    let selected = if options.brain.is_none() {
        Some(workspace::selector(&directory).await?)
    } else {
        None
    };
    let brains = client.brains(device).await?;
    let matching: Vec<_> = brains
        .iter()
        .filter(|b| {
            options.brain.map(|id| b.id == id).unwrap_or_else(|| {
                selected
                    .as_ref()
                    .is_some_and(|(_, s)| b.id.to_string() == *s || b.name == *s)
            })
        })
        .collect();
    ensure!(
        matching.len() == 1,
        "Choose exactly one accessible Brain; use --brain UUID if names are ambiguous."
    );
    let brain = matching[0];
    ensure!(
        matches!(brain.role.as_str(), "writer" | "admin") && !brain.archived,
        "Capture setup needs writer access to an open Brain."
    );
    if let Some(selected) = selected {
        ensure!(
            workspace::selector(&directory).await? == selected,
            "Workspace Brain changed during setup. Retry explicitly."
        );
    }
    let root = publication::private_root(&options.evidence_root).await?;
    privacy::synchronize(client, device, brain.id, &root).await?;
    let base = format!("/api/brains/{}", brain.id);
    let settings: CaptureSettings = decode(
        client
            .send(
                Method::GET,
                &format!("{base}/capture/policy"),
                Some(device.token),
                None,
            )
            .await?,
    )
    .await?;
    let task = if let Some(task) = options.task {
        task
    } else {
        let change: TaskChange = decode(
            client
                .send(
                    Method::POST,
                    &format!("{base}/workspace/tasks"),
                    Some(device.token),
                    Some(serde_json::to_value(CreateTask {
                        label: format!("{} session capture", options.host),
                        parent_task_id: None,
                        workspace_id: None,
                        selection: None,
                    })?),
                )
                .await?,
        )
        .await?;
        change.task.id
    };
    let operation: OperationBinding = decode(
        client
            .send(
                Method::POST,
                &format!("{base}/workspace/tasks/{task}/operations"),
                Some(device.token),
                Some(json!({"kind":"capture"})),
            )
            .await?,
    )
    .await?;
    let binding: CaptureBinding = decode(client.send(Method::POST,&format!("{base}/capture/bindings"),Some(device.token),Some(json!({"id":Uuid::new_v4(),"operation_id":operation.id,"host":options.host,"host_version":options.host_version}))).await?).await?;
    ensure!(
        binding.brain_id == brain.id
            && binding.device_id == device.device_id
            && binding.operation.id == operation.id,
        "Capture binding does not match this task/device."
    );
    let privacy = privacy::cached(&root, &client.endpoint, device.device_id, brain.id)
        .await?
        .ok_or_else(|| anyhow!("Capture privacy state is unavailable."))?;
    let mut inbox = Inbox::open(
        &profile_root(&root, device.device_id),
        &client.endpoint,
        device.device_id,
    )?;
    inbox.remember(&CachedCaptureBinding {
        binding: binding.clone(),
        policy: settings.policy.clone(),
        retention: privacy.sync.policy,
        synchronized_at: Utc::now(),
        agent_id: options.agent_id,
    })?;
    let directory_out = publication::private_root(&output.unwrap_or_else(|| {
        profile_root(&root, device.device_id)
            .join("hosts")
            .join(binding.id.to_string())
    }))
    .await?;
    let setup_file = directory_out.join("capture.json");
    let hooks_file = directory_out.join("hooks.json");
    let setup = HookSetup {
        endpoint: client.endpoint.clone(),
        device_id: device.device_id,
        brain_id: brain.id,
        binding_id: binding.id,
        plugin_root: (options.host == "codex")
            .then(|| profile_root(&root, device.device_id).join("codex-plugin")),
        evidence_root: root,
        device_profile: options.device_profile,
        host: options.host.clone(),
        working_directory: directory,
        task_id: task,
        created_task: options.task.is_none(),
    };
    let hooks = hooks(&options.host, &options.executable, &setup_file)?;
    publication::write_private(&setup_file, &serde_json::to_vec_pretty(&setup)?).await?;
    publication::write_private(&hooks_file, &serde_json::to_vec_pretty(&hooks)?).await?;
    if options.host == "codex" {
        codex_plugin(setup.plugin_root.as_ref().unwrap(), &hooks).await?;
    }
    tokio::fs::File::open(&directory_out)
        .await?
        .sync_all()
        .await?;
    let host_arguments = host_arguments(&options.host, &hooks_file, &hooks)?;
    Ok(PreparedCapture {
        run_command: format!(
            "{} capture run {}",
            shell_word(
                options
                    .executable
                    .to_str()
                    .ok_or_else(|| anyhow!("Use UTF-8 hook paths."))?
            ),
            shell_word(setup_file.to_str().unwrap())
        ),
        setup_file,
        hooks_file,
        brain_url: format!("{}/brains/{}", client.endpoint, brain.id),
        binding,
        capture_enabled: settings.policy.enabled,
        state: "configured_only",
        host_arguments,
        host_notice: if options.host == "codex" {
            "Codex requires initial trust for generated hooks through /hooks. Publication is verified separately by capture status and the Brain activity."
        } else {
            "Generated command hooks capture supported events. Publication is verified separately by capture status and the Brain activity."
        },
    })
}
pub(crate) async fn version(host: &str) -> Result<String> {
    let mut child = Command::new(if host == "codex" { "codex" } else { "claude" })
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| anyhow!("Install the selected host before capture setup."))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Cannot inspect host version."))?;
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut bytes = vec![];
        stdout.take(4097).read_to_end(&mut bytes).await?;
        ensure!(
            bytes.len() <= 4096 && child.wait().await?.success(),
            "Cannot inspect host version."
        );
        let text = std::str::from_utf8(&bytes).map_err(|_| anyhow!("Unreadable host version."))?;
        let version = text
            .split_whitespace()
            .find(|s| s.starts_with(|c: char| c.is_ascii_digit()) && capture_identity(s))
            .ok_or_else(|| anyhow!("Unrecognized host version."))?;
        Ok(version.to_owned())
    })
    .await
    .map_err(|_| anyhow!("Host version check timed out."))?
}
pub async fn run(client: &Client, device: &StoredDevice, args: &[String]) -> Result<Value> {
    ensure!(args.len() >= 2, "{USAGE}");
    let host = match args[0].as_str() {
        "codex" => "codex",
        "claude" | "claude_code" => "claude_code",
        _ => return Err(anyhow!("Choose codex or claude_code.")),
    };
    let mut options = SetupOptions {
        host: host.into(),
        host_version: version(host).await?,
        directory: PathBuf::from(&args[1]),
        brain: None,
        task: None,
        agent_id: None,
        output: None,
        executable: std::env::current_exe()?,
        device_profile: std::env::var("RECOLLECT_DEVICE_PROFILE").unwrap_or("default".into()),
        evidence_root: std::env::var_os("RECOLLECT_PUBLICATION_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| publication::project_root().join(".data/publications")),
    };
    let mut i = 2;
    while i < args.len() {
        let value = args.get(i + 1).ok_or_else(|| anyhow!(USAGE))?;
        match args[i].as_str() {
            "--brain" => {
                options.brain = Some(value.parse().map_err(|_| anyhow!("Use a Brain UUID."))?)
            }
            "--task" => {
                options.task = Some(value.parse().map_err(|_| anyhow!("Use a task UUID."))?)
            }
            "--agent" => options.agent_id = Some(value.clone()),
            "--output" => options.output = Some(PathBuf::from(value)),
            _ => return Err(anyhow!(USAGE)),
        }
        i += 2;
    }
    Ok(serde_json::to_value(
        prepare(client, device, options).await?,
    )?)
}
