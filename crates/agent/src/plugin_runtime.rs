//! Bundled plugin entry points. The coding host owns hooks, not a launch wrapper.
use crate::{
    Client, CredentialSlot, StoredDevice, capture_delivery,
    plugin_session::{self, Session},
    plugin_storage::{self, Config},
};
use anyhow::{Result, anyhow, bail, ensure};
use chrono::Utc;
use recollect_protocol::{CAPTURE_STDIN_BYTES, sanitize_capture_text};
use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use uuid::Uuid;

pub const USAGE: &str = "Recollect plugin: connect --url URL --brain UUID [--token-stdin] [--with-runner] [--runner-id UUID] | status | disconnect | hook HOST | mcp | drain | runner | workspace discover|refresh|list [DIRECTORY] | scope ... | repository ...";

fn option(args: &[String], name: &str) -> Result<Option<String>> {
    let found: Vec<_> = args
        .iter()
        .enumerate()
        .filter(|(_, arg)| arg.as_str() == name)
        .collect();
    ensure!(found.len() <= 1, "duplicate_plugin_option");
    found
        .first()
        .map(|(i, _)| {
            args.get(i + 1)
                .filter(|v| !v.starts_with("--"))
                .cloned()
                .ok_or_else(|| anyhow!("missing_plugin_option"))
        })
        .transpose()
}

pub fn credential(config: &Config) -> Result<StoredDevice> {
    let device = CredentialSlot::new(&config.endpoint, &config.profile)?
        .load()?
        .ok_or_else(|| anyhow!("plugin_not_connected"))?;
    ensure!(
        device.endpoint == config.endpoint && device.device_id == config.device,
        "plugin_credential_mismatch"
    );
    Ok(device)
}

pub(crate) async fn selected(
    mut config: Config,
    directory: &Path,
) -> Result<(Config, Option<PathBuf>)> {
    let cwd = tokio::fs::canonicalize(directory).await?;
    let mut found = false;
    for parent in cwd.ancestors() {
        match tokio::fs::symlink_metadata(parent.join(".recollect/workspace.toml")).await {
            Ok(_) => {
                found = true;
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => bail!("workspace_selector_unavailable"),
        }
    }
    if !found {
        return Ok((config, None));
    }
    let (root, selector) = crate::workspace::selector(&cwd).await?;
    // UUID selectors need no network on the local capture path. Authority is
    // checked at binding creation, recall and publication, including offline replay.
    config.brain = if let Ok(id) = selector.parse() {
        id
    } else {
        let client = Client::new(&config.endpoint)?;
        let brains = client.brains(&credential(&config)?).await?;
        let matches: Vec<_> = brains.iter().filter(|b| b.name == selector).collect();
        ensure!(matches.len() == 1, "workspace_brain_ambiguous");
        matches[0].id
    };
    Ok((config, Some(root)))
}

async fn connect(args: &[String]) -> Result<Value> {
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--url" | "--brain" | "--runner-id" => index += 2,
            "--token-stdin" | "--with-runner" => index += 1,
            _ => bail!("unknown_plugin_option"),
        }
    }
    let client = Client::new(
        &option(args, "--url")?
            .ok_or_else(|| anyhow!("Choose your Recollect server with --url."))?,
    )?;
    let brain: Uuid = option(args, "--brain")?
        .ok_or_else(|| anyhow!("Choose a Brain with --brain UUID."))?
        .parse()?;
    let with_runner = args.iter().any(|v| v == "--with-runner");
    let runner_id = option(args, "--runner-id")?
        .map(|id| id.parse())
        .transpose()?;
    ensure!(
        runner_id.is_none() || with_runner,
        "runner_id_requires_with_runner"
    );
    let root = plugin_storage::root()?;
    plugin_storage::directory(&root)?;
    let previous = plugin_storage::config().ok();
    let mut profile = previous
        .as_ref()
        .filter(|c| c.endpoint == client.endpoint)
        .map(|c| c.profile.clone())
        .unwrap_or_else(|| format!("plugin-{}", Uuid::new_v4()));
    let supplied = args.iter().any(|v| v == "--token-stdin");
    let device = if supplied {
        let mut token = String::new();
        std::io::stdin().take(129).read_to_string(&mut token)?;
        let token: Uuid = token
            .trim()
            .parse()
            .map_err(|_| anyhow!("invalid_access_token"))?;
        let provisional = StoredDevice {
            endpoint: client.endpoint.clone(),
            device_id: Uuid::nil(),
            token,
        };
        let response = client
            .send(reqwest::Method::GET, "/api/auth/me", Some(token), None)
            .await?;
        let identity: recollect_protocol::SessionInfo = crate::decode(response).await?;
        let device = identity
            .device_id
            .ok_or_else(|| anyhow!("plugin_device_required"))?;
        if !previous
            .as_ref()
            .is_some_and(|c| c.endpoint == client.endpoint && c.device == device)
        {
            profile = format!("plugin-{}", Uuid::new_v4());
        }
        StoredDevice {
            device_id: device,
            ..provisional
        }
    } else {
        let slot = CredentialSlot::new(&client.endpoint, &profile)?;
        if slot.load()?.is_none() {
            client.pair(&slot, "Recollect plugin".into()).await?;
        }
        slot.load()?
            .ok_or_else(|| anyhow!("plugin_not_connected"))?
    };
    client.whoami(&device).await?;
    let brains = client.brains(&device).await?;
    ensure!(
        brains.iter().any(|b| b.id == brain && !b.archived),
        "plugin_brain_unavailable"
    );
    // Validate the destination before saving a supplied credential, and keep a
    // different device in its own slot so older queued captures retain authority.
    if supplied {
        CredentialSlot::new(&client.endpoint, &profile)?.save(&device)?;
    }
    let config = Config {
        endpoint: client.endpoint,
        brain,
        device: device.device_id,
        profile,
        with_runner,
        runner_id,
    };
    if let Some(previous) = previous {
        plugin_storage::remember_destination(&previous)?;
    }
    plugin_storage::remember_destination(&config)?;
    plugin_storage::write(&root.join("config.json"), &config)?;
    plugin_storage::runtime_pointer()?;
    wake_delivery()?;
    if with_runner {
        spawn("runner")?;
    }
    Ok(
        json!({"state":"connected","brain_id":brain,"runner_enabled":with_runner,
        "migration":crate::plugin_migration::all(&std::env::current_dir()?),
        "next":"Start your coding host normally. Trust the installed plugin hooks when the host asks. Capture follows the Brain's standing policy."}),
    )
}

fn spawn(command: &str) -> Result<()> {
    let mut child = std::process::Command::new(std::env::current_exe()?);
    child
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        child.process_group(0);
    }
    child.spawn()?;
    Ok(())
}

fn wake_delivery() -> Result<()> {
    // A worker holding the lock may be just about to go idle. Persist a wake
    // generation before spawning so that worker can hand off after unlocking.
    plugin_storage::write(
        &plugin_storage::root()?.join("delivery-wakeup.json"),
        &Uuid::new_v4(),
    )?;
    spawn("drain")
}

fn secrets() -> Vec<String> {
    crate::publication::configured_secrets()
}

pub async fn hook(host: &str, raw: &[u8]) -> Result<Value> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(7500);
    ensure!(
        matches!(host, "codex" | "claude_code" | "opencode"),
        "unsupported_capture_host"
    );
    ensure!(raw.len() <= CAPTURE_STDIN_BYTES, "capture_input_too_large");
    let event: Value =
        serde_json::from_slice(raw).map_err(|_| anyhow!("malformed_capture_event"))?;
    let name = event["hook_event_name"]
        .as_str()
        .ok_or_else(|| anyhow!("missing_capture_event"))?;
    let session_id = event["session_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing_session_identity"))?;
    let cwd = PathBuf::from(
        event["cwd"]
            .as_str()
            .ok_or_else(|| anyhow!("missing_workspace"))?,
    );
    let cwd = tokio::fs::canonicalize(&cwd).await?;
    if !crate::plugin_migration::inspect(host, &cwd).is_empty() {
        eprintln!(
            "Recollect plugin found a legacy MCP/capture setup. Drain its pending captures, remove its first-party host entry and start the host normally. Plugin capture is withheld to avoid duplicate evidence; inspect plugin status for the file paths."
        );
        bail!("plugin_migration_required");
    }
    let (config, _) = selected(plugin_storage::config()?, &cwd).await?;
    let root = plugin_storage::root()?;
    if host == "opencode" && name == "CoverageGap" {
        crate::capture::Inbox::open(
            &crate::capture::profile_root(&root, config.device),
            &config.endpoint,
            config.device,
        )?
        .record_gap("native_event_coverage_gap")?;
        wake_delivery()?;
        return Ok(json!({}));
    }
    let agent = match event.get("agent_id") {
        Some(Value::String(id)) if recollect_protocol::capture_identity(id) => Some(id.as_str()),
        None | Some(Value::Null) => None,
        _ => bail!("invalid_agent_identity"),
    };
    if matches!(name, "SubagentStart" | "SubagentStop") && agent.is_none() {
        crate::capture::Inbox::open(
            &crate::capture::profile_root(&root, config.device),
            &config.endpoint,
            config.device,
        )?
        .record_gap("ambiguous_attribution")?;
        bail!("missing_agent_identity");
    }
    let state = plugin_session::agent_state_path(&root, &config, host, session_id, &cwd, agent)?;
    let lock = plugin_storage::private_file(&state.with_extension("lock"), false)?;
    lock.try_lock()
        .map_err(|_| anyhow!("plugin_session_busy"))?;
    let mut session = if state.try_exists()? {
        Some(plugin_storage::read::<Session>(&state)?)
    } else {
        None
    };
    let client = Client::new(&config.endpoint)?;
    // Ordinary tool hooks never open the OS store or make a network request.
    // New sessions are established only at a documented start/prompt boundary.
    let boundary = matches!(
        name,
        "SessionStart" | "UserPromptSubmit" | "ContextRequest" | "StepStart" | "SubagentStart"
    );
    let mut resume_selection = None;
    if boundary && session.as_ref().is_some_and(|s| s.ended) {
        let previous = session.as_mut().expect("existing session");
        let device = credential(&config)?;
        tokio::time::timeout_at(deadline, plugin_session::close(&client, &device, previous))
            .await
            .map_err(|_| anyhow!("plugin_session_timeout"))??;
        plugin_storage::write(&state, previous)?;
        resume_selection = Some(previous.scope.selection.clone());
        session = None;
    }
    if session.is_none() {
        ensure!(
            matches!(
                name,
                "SessionStart"
                    | "UserPromptSubmit"
                    | "ContextRequest"
                    | "StepStart"
                    | "SubagentStart"
            ),
            "plugin_session_missing"
        );
        // Do not infer an unobserved child's original scope from a later root
        // prompt. Only its native start event establishes the independent task.
        ensure!(
            agent.is_none() || name == "SubagentStart",
            "plugin_child_start_missing"
        );
        let device = credential(&config)?;
        let id = Uuid::new_v4();
        let version = if let Some(version) = event["host_version"].as_str() {
            version.to_owned()
        } else {
            crate::capture_setup::version(host).await?
        };
        let native_parent = event["parent_session_id"].as_str();
        let parent_task = if let Some(parent) = native_parent.or(agent.map(|_| session_id)) {
            let parent_cwd = if native_parent.is_some() {
                Path::new(
                    event["parent_cwd"]
                        .as_str()
                        .ok_or_else(|| anyhow!("parent_workspace_missing"))?,
                )
            } else {
                &cwd
            };
            let parent_state =
                plugin_session::state_path(&root, &config, host, parent, parent_cwd)?;
            let parent: Session = plugin_storage::read(&parent_state)?;
            ensure!(
                parent.setup.brain_id == config.brain
                    && parent.setup.device_id == config.device
                    && parent.setup.endpoint == config.endpoint
                    && !parent.ended
                    && !parent.closed,
                "parent_session_destination_mismatch"
            );
            Some(parent.setup.task_id)
        } else {
            None
        };
        let created = tokio::time::timeout_at(
            deadline,
            plugin_session::create(
                &client,
                &device,
                &config,
                plugin_session::SessionStart {
                    host,
                    version: &version,
                    cwd: &cwd,
                    root: &root,
                    id,
                    parent_task,
                    selection: resume_selection,
                    agent_id: agent,
                },
            ),
        )
        .await
        .map_err(|_| anyhow!("plugin_session_timeout"))??;
        plugin_storage::write(&state, &created)?;
        session = Some(created);
    }
    let mut session = session.ok_or_else(|| anyhow!("plugin_session_missing"))?;
    ensure!(
        session.setup.brain_id == config.brain
            && session.setup.device_id == config.device
            && session.setup.endpoint == config.endpoint
            && session.setup.working_directory == cwd
            && session.setup.host == host
            && session.agent_id.as_deref() == agent,
        "plugin_session_destination_changed"
    );
    let needs_context = matches!(
        name,
        "UserPromptSubmit" | "PostCompact" | "ContextRequest" | "SubagentStart"
    ) || (name == "SessionStart"
        && matches!(event["source"].as_str(), Some("compact" | "resume")));
    let needs_context = needs_context && event["recollect_capture_only"] != true;
    let device = if needs_context || boundary {
        Some(credential(&config)?)
    } else {
        None
    };
    if let Some(device) = &device {
        let _ = tokio::time::timeout_at(
            deadline,
            plugin_session::refresh(&client, device, &mut session),
        )
        .await;
    }
    session.last_event_at = Utc::now();
    if matches!(name, "SessionEnd" | "SubagentStop") {
        session.ended = true;
    }
    plugin_storage::write(&state, &session)?;
    let launch = session.launch();
    let configured_secrets = secrets();
    let capture_scope_ready = if boundary && !session.setup.binding_id.is_nil() {
        launch
            .current()
            .and_then(|id| launch.inbox()?.cached_binding(id))
            .is_ok_and(|saved| saved.binding.operation.scope.id == session.scope.id)
    } else {
        true
    };
    if !capture_scope_ready {
        launch.gap();
    }
    // Capture's disabled/denied/stale-policy gaps must never suppress authorized
    // recall. Existing turns keep their immutable cached bindings while offline.
    let mut captured = None;
    if name != "ContextRequest" && !session.setup.binding_id.is_nil() && capture_scope_ready {
        captured = launch
            .inbox()?
            .capture_in_launch(
                session.setup.binding_id,
                Some(session.id),
                raw,
                &configured_secrets,
                Utc::now(),
            )
            .ok();
    }
    wake_delivery()?;
    if config.with_runner {
        spawn("runner")?;
    }
    let mut output = json!({});
    if needs_context && let Some(device) = device {
        let query = event["prompt"]
            .as_str()
            .unwrap_or("Important decisions, facts and procedures for continuing this task");
        let mut query = sanitize_capture_text(query, &configured_secrets);
        let mut end = query.len().min(2000);
        while !query.is_char_boundary(end) {
            end -= 1;
        }
        query.truncate(end);
        if !query.trim().is_empty()
            && let Ok(Ok(context)) = tokio::time::timeout_at(
                deadline,
                plugin_session::recall(&client, &device, &session, &query),
            )
            .await
        {
            output = plugin_session::output(name, &context);
        }
    }
    if host == "opencode" && capture_scope_ready && !session.setup.binding_id.is_nil() {
        output["recollectCaptureBinding"] = json!(launch.current()?);
        output["recollectCapturedEvent"] = json!(captured);
    }
    Ok(output)
}

/// OS keychain calls and local filesystem calls can block synchronously, outside
/// Tokio's timeout. Bound the entire standalone hook process as well. Atomic
/// output ownership prevents a late normal completion from emitting a second JSON.
async fn hook_command(host: &str) -> Result<()> {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    let output = Arc::new(AtomicBool::new(true));
    let timer_output = output.clone();
    let (finished, wait) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        if matches!(
            wait.recv_timeout(Duration::from_secs(8)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ) && timer_output.swap(false, Ordering::AcqRel)
        {
            eprintln!("Recollect memory timed out for this event. Coding can continue.");
            println!("{{}}");
            std::process::exit(0);
        }
    });
    let result = async {
        if plugin_storage::config().is_ok() {
            let _ = plugin_storage::runtime_pointer();
        }
        let mut raw = Vec::new();
        std::io::stdin()
            .take((CAPTURE_STDIN_BYTES + 1) as u64)
            .read_to_end(&mut raw)?;
        hook(host, &raw).await
    }
    .await;
    if output.swap(false, Ordering::AcqRel) {
        let _ = finished.send(());
        let result = match result {
            Ok(value) => value,
            Err(_) => {
                eprintln!("Recollect memory unavailable for this event. Check plugin status.");
                json!({})
            }
        };
        println!("{}", serde_json::to_string(&result)?);
    }
    Ok(())
}

async fn close_ended_sessions(client: &Client, device: &StoredDevice, root: &Path) -> Result<bool> {
    let directory = root.join("sessions");
    if !directory.try_exists()? {
        return Ok(false);
    }
    let mut pending = false;
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let lock = plugin_storage::private_file(&path.with_extension("lock"), false)?;
        if lock.try_lock().is_err() {
            pending = true;
            continue;
        }
        let mut session: Session = plugin_storage::read(&path)?;
        if session.setup.endpoint != client.endpoint
            || session.setup.device_id != device.device_id
            || !session.ended
            || session.closed
        {
            continue;
        }
        if let Ok(Ok(())) = tokio::time::timeout(
            Duration::from_secs(3),
            plugin_session::close(client, device, &mut session),
        )
        .await
        {
            plugin_storage::write(&path, &session)?;
        } else {
            pending = true;
        }
    }
    Ok(pending)
}

async fn deliver_destination(root: &Path, config: &Config, current: &Config) -> Result<bool> {
    let mut inbox = crate::capture::Inbox::open(
        &crate::capture::profile_root(root, config.device),
        &config.endpoint,
        config.device,
    )?;
    // Expiry does not depend on a working network or a still-present credential.
    inbox.expire(Utc::now())?;
    let status_path = root
        .join("delivery-reports")
        .join(format!("{}.json", config.device));
    let device = match credential(config) {
        Ok(device) => device,
        Err(_) => {
            plugin_storage::write(
                &status_path,
                &json!({"state":"credential_unavailable","checked_at":Utc::now()}),
            )?;
            return Ok(inbox.status()?.pending == 0);
        }
    };
    drop(inbox);
    let client = Client::new(&config.endpoint)?;
    let result = tokio::time::timeout(
        Duration::from_secs(15),
        capture_delivery::run_once(&client, &device, root, None),
    )
    .await;
    let Ok(Ok(report)) = result else {
        return Ok(false);
    };
    let value = json!({"checked_at":Utc::now(),"report":report});
    plugin_storage::write(&status_path, &value)?;
    if current.device == config.device && current.endpoint == config.endpoint {
        plugin_storage::write(&root.join("delivery-status.json"), &value)?;
    }
    if report.inbox.pending != 0 {
        return Ok(false);
    }
    let close_pending = close_ended_sessions(&client, &device, root).await?;
    // A hook can append while a close request is in flight.
    let pending = crate::capture::Inbox::open(
        &crate::capture::profile_root(root, config.device),
        &config.endpoint,
        config.device,
    )?
    .status()?
    .pending;
    Ok(!close_pending && pending == 0)
}

async fn drain() -> Result<Value> {
    let root = plugin_storage::root()?;
    plugin_storage::directory(&root)?;
    let lock = plugin_storage::private_file(&root.join("delivery.lock"), false)?;
    if lock.try_lock().is_err() {
        return Ok(json!({"state":"already_running"}));
    }
    let config = plugin_storage::config()?;
    let until = tokio::time::Instant::now() + Duration::from_secs(120);
    loop {
        let wake = plugin_storage::read::<Uuid>(&root.join("delivery-wakeup.json")).ok();
        let result = tokio::time::timeout_at(until, async {
            let mut idle = true;
            for destination in plugin_storage::destinations()? {
                // A revoked/offline former destination cannot starve the current
                // one. Every delivery uses its original device and credential.
                idle &= deliver_destination(&root, &destination, &config)
                    .await
                    .unwrap_or(false);
            }
            Ok::<_, anyhow::Error>(idle)
        })
        .await;
        if matches!(result, Ok(Ok(true))) {
            drop(lock);
            // A hook either sees this unlocked worker and starts its own,
            // or wrote its wake while the lock was held; cover both orders.
            if plugin_storage::read::<Uuid>(&root.join("delivery-wakeup.json")).ok() != wake {
                spawn("drain")?;
            }
            return Ok(json!({"state":"idle"}));
        }
        if tokio::time::Instant::now() >= until {
            return Ok(json!({"state":"queued"}));
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn runner() -> Result<Value> {
    let root = plugin_storage::root()?;
    plugin_storage::directory(&root)?;
    let lock = plugin_storage::private_file(&root.join("runner.lock"), false)?;
    if lock.try_lock().is_err() {
        return Ok(json!({"state":"already_running"}));
    }
    let config = plugin_storage::config()?;
    ensure!(config.with_runner, "plugin_runner_disabled");
    let status_path = root.join("runner-status.json");
    plugin_storage::write(
        &status_path,
        &json!({"state":"starting","checked_at":Utc::now()}),
    )?;
    let stop = recollect_mcp_runtime::CancellationToken::new();
    let running = crate::mcp::run_selected(
        Client::new(&config.endpoint)?,
        credential(&config)?,
        root.join("runner-receipts").join(config.device.to_string()),
        stop.clone(),
        config.runner_id,
    );
    tokio::pin!(running);
    loop {
        tokio::select! {
            result = &mut running => {
                let (state, code) = match &result {
                    Ok(()) => ("stopped", None),
                    Err(error) => ("failed", Some(error.0)),
                };
                plugin_storage::write(&status_path, &json!({"state":state,"code":code,"checked_at":Utc::now()}))?;
                result?;
                break;
            }
            _ = tokio::signal::ctrl_c() => { stop.cancel(); running.await?; break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                let current = plugin_storage::config();
                if current.is_err() || current.is_ok_and(|c| !c.with_runner || c.endpoint != config.endpoint || c.device != config.device || c.runner_id != config.runner_id) {
                    stop.cancel(); running.await?; break;
                }
            }
        }
    }
    plugin_storage::write(
        &status_path,
        &json!({"state":"stopped","checked_at":Utc::now()}),
    )?;
    Ok(json!({"state":"stopped"}))
}

async fn status() -> Result<Value> {
    let root = plugin_storage::root()?;
    if !root.join("config.json").try_exists()? {
        return Ok(json!({"state":"not_configured"}));
    }
    let (config, _) = selected(plugin_storage::config()?, &std::env::current_dir()?).await?;
    let device = match credential(&config) {
        Ok(device) => device,
        Err(_) => return Ok(json!({"state":"credential_unavailable","brain_id":config.brain})),
    };
    let client = Client::new(&config.endpoint)?;
    let check = tokio::time::timeout(
        Duration::from_secs(8),
        client.send(
            reqwest::Method::GET,
            &format!("/api/brains/{}/capture/policy", config.brain),
            Some(device.token),
            None,
        ),
    )
    .await;
    let (state, policy) = match check {
        Ok(Ok(reply)) if reply.status().is_success() => {
            ("connected", reply.json::<Value>().await.ok())
        }
        Ok(Ok(reply)) if matches!(reply.status().as_u16(), 401 | 403 | 404) => ("denied", None),
        _ => ("offline", None),
    };
    let inbox = crate::capture::Inbox::open(
        &crate::capture::profile_root(&root, config.device),
        &config.endpoint,
        config.device,
    )?
    .status()?;
    Ok(
        json!({"state":state,"brain_id":config.brain,"runner_enabled":config.with_runner,
        "migration":crate::plugin_migration::all(&std::env::current_dir()?),
        "capture_enabled":policy.as_ref().and_then(|p| p["policy"]["enabled"].as_bool()),
        "inbox":inbox,"delivery":plugin_storage::read::<Value>(&root.join("delivery-reports").join(format!("{}.json", config.device))).ok()}),
    )
}

pub async fn run(args: &[String]) -> Result<()> {
    let Some(command) = args.first() else {
        bail!("{USAGE}");
    };
    if command == "mcp-supervise" {
        return Ok(recollect_mcp_runtime::supervisor::run().await?);
    }
    let result = match command.as_str() {
        "--help" | "help" => {
            println!("{USAGE}");
            return Ok(());
        }
        "connect" => connect(&args[1..]).await?,
        "hook" => {
            ensure!(args.len() == 2, "{USAGE}");
            return hook_command(&args[1]).await;
        }
        "mcp" => {
            let (config, root) =
                selected(plugin_storage::config()?, &std::env::current_dir()?).await?;
            return crate::mcp_bridge::serve_plugin(
                Client::new(&config.endpoint)?,
                credential(&config)?,
                config.brain,
                root,
                None,
            )
            .await;
        }
        "drain" => drain().await?,
        "runner" => runner().await?,
        "status" => status().await?,
        "disconnect" => {
            let mut config = plugin_storage::config()?;
            config.with_runner = false;
            plugin_storage::write(&plugin_storage::root()?.join("config.json"), &config)?;
            Client::new(&config.endpoint)?
                .revoke(&credential(&config)?)
                .await?;
            CredentialSlot::new(&config.endpoint, &config.profile)?.forget()?;
            json!({"state":"disconnected"})
        }
        "workspace" if args.get(1).is_some_and(|action| action == "discover") => {
            ensure!(args.len() <= 3, "{}", crate::workspace_cli::USAGE);
            let directory = Path::new(args.get(2).map(String::as_str).unwrap_or("."));
            serde_json::to_value(crate::workspace::discover(directory).await?)?
        }
        "workspace" | "scope" | "repository" => {
            let config = plugin_storage::config()?;
            let client = Client::new(&config.endpoint)?;
            let device = credential(&config)?;
            if command == "repository" {
                crate::publication_cli::run_with_defaults(
                    &client,
                    &device,
                    &args[1..],
                    plugin_storage::root()?,
                    std::env::current_exe()?.with_file_name("enola"),
                )
                .await?
            } else {
                crate::workspace_cli::run(&client, &device, command, &args[1..]).await?
            }
        }
        _ => bail!("{USAGE}"),
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
