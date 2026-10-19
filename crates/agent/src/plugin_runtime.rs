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

pub const USAGE: &str = "Recollect plugin: connect --url URL --brain UUID [--token-stdin] [--with-runner] [--runner-id UUID] | status | disconnect | hook HOST | mcp | drain | runner | coordination announce|heartbeat|peers|inbox|ack|send|close|watch|status ... | skills list|fetch|save|discard|install|sync|reload|uninstall|status|leases ... | workspace discover|refresh|list [DIRECTORY] | scope ... | repository ...";

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
    config: Config,
    directory: &Path,
) -> Result<(Config, Option<PathBuf>)> {
    selected_with_network(config, directory, true).await
}

async fn selected_with_network(
    mut config: Config,
    directory: &Path,
    network: bool,
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
        let data = plugin_storage::root()?;
        plugin_storage::directory(&data)?;
        let path = data.join("workspace-selectors.sqlite");
        plugin_storage::private_file(&path, false)?;
        let db = rusqlite::Connection::open(path)?;
        db.busy_timeout(Duration::from_millis(100))?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS selectors(endpoint TEXT,device TEXT,root TEXT,selector TEXT,brain TEXT,PRIMARY KEY(endpoint,device,root,selector));")?;
        if !network {
            let brain:String=db.query_row("SELECT brain FROM selectors WHERE endpoint=?1 AND device=?2 AND root=?3 AND selector=?4",rusqlite::params![config.endpoint,config.device.to_string(),root.to_string_lossy(),selector],|r|r.get(0)).map_err(|_|anyhow!("workspace_selector_needs_start"))?;
            config.brain = brain.parse()?;
            return Ok((config, Some(root)));
        }
        let client = Client::new(&config.endpoint)?;
        let brains = client.brains(&credential(&config)?).await?;
        let matches: Vec<_> = brains.iter().filter(|b| b.name == selector).collect();
        ensure!(matches.len() == 1, "workspace_brain_ambiguous");
        let id = matches[0].id;
        db.execute("INSERT INTO selectors VALUES(?1,?2,?3,?4,?5) ON CONFLICT(endpoint,device,root,selector) DO UPDATE SET brain=excluded.brain",rusqlite::params![config.endpoint,config.device.to_string(),root.to_string_lossy(),selector,id.to_string()])?;
        id
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

/// Time kept in reserve for a hook's own work when waiting on a busy session.
const HOOK_WORK_RESERVE: Duration = Duration::from_millis(3000);
/// Longest hook budget before the standalone watchdog gives up on the event.
const HOOK_BUDGET: Duration = Duration::from_millis(7500);
/// Persist a bare activity timestamp at most this often (peers treat 90 s as live).
const SESSION_TOUCH: chrono::Duration = chrono::Duration::seconds(60);

pub async fn hook(host: &str, raw: &[u8]) -> Result<Value> {
    hook_until(host, raw, tokio::time::Instant::now() + HOOK_BUDGET).await
}

/// Take the per-session state lock, waiting for a concurrent hook of the same
/// session. Claude Code can run PostToolUse hooks concurrently, so contention
/// is normal and must not discard the event. Returns false if `until` passes.
async fn lock_session(lock: &std::fs::File, until: tokio::time::Instant) -> Result<bool> {
    use std::fs::TryLockError;
    let mut delay = Duration::from_millis(5);
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(true),
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
        if tokio::time::Instant::now() + delay > until {
            return Ok(false);
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_millis(100));
    }
}

/// `deadline` is the watchdog bound of the whole hook (`hook` passes its default).
pub async fn hook_until(host: &str, raw: &[u8], deadline: tokio::time::Instant) -> Result<Value> {
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
    let boundary = matches!(
        name,
        "SessionStart"
            | "UserPromptSubmit"
            | "ContextRequest"
            | "StepStart"
            | "SubagentStart"
            | "PostCompact"
    );
    // Reads dozens of host configuration files; a session is connected at a
    // boundary event, so ordinary tool hooks do not repeat the inspection.
    if boundary && !crate::plugin_migration::inspect(host, &cwd).is_empty() {
        eprintln!(
            "Recollect plugin found a legacy MCP/capture setup. Drain its pending captures, remove its first-party host entry and start the host normally. Plugin capture is withheld to avoid duplicate evidence; inspect plugin status for the file paths."
        );
        bail!("plugin_migration_required");
    }
    let (config, _) = selected_with_network(plugin_storage::config()?, &cwd, boundary).await?;
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
    let wait_until = deadline
        .checked_sub(HOOK_WORK_RESERVE)
        .map_or_else(tokio::time::Instant::now, |t| {
            t.max(tokio::time::Instant::now())
        });
    if !lock_session(&lock, wait_until).await? {
        // Record the loss on the inbox so the gap is reported, not silent.
        let _ = crate::capture::Inbox::open(
            &crate::capture::profile_root(&root, config.device),
            &config.endpoint,
            config.device,
        )
        .and_then(|mut inbox| inbox.record_gap("capture_hook_contended"));
        bail!("plugin_session_busy");
    }
    plugin_session::restore(&state)?;
    let mut session = if state.try_exists()? {
        Some(plugin_storage::read::<Session>(&state)?)
    } else {
        None
    };
    let client = Client::new(&config.endpoint)?.with_host(host);
    // Ordinary tool hooks never open the OS store or make a network request.
    // New sessions are established only at a documented start/prompt boundary.
    let mut resume_selection = None;
    let mut resume_task = None;
    if boundary && session.as_ref().is_some_and(|s| s.ended) {
        let previous = session.as_mut().expect("existing session");
        let device = credential(&config)?;
        tokio::time::timeout_at(deadline, plugin_session::close(&client, &device, previous))
            .await
            .map_err(|_| anyhow!("plugin_session_timeout"))??;
        plugin_storage::write(&state, previous)?;
        resume_selection = Some(previous.scope.selection.clone());
        resume_task = Some(previous.setup.task_id);
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
            crate::capture_scope::host_version(host).await?
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
                    continuation_of_task: resume_task,
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
    // What is on disk now; per-event timestamps alone do not justify two fsyncs.
    let persisted = serde_json::to_value(&session)?;
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
    let previous_event_at = session.last_event_at;
    session.last_event_at = Utc::now();
    let mut native_cleanup = None;
    if matches!(name, "SessionEnd" | "SubagentStop") {
        session.ended = true;
        match crate::plugin_skills::native_cli::release_session(&config, &session) {
            Ok(value) => native_cleanup = value,
            Err(_) => eprintln!(
                "Recollect kept native copies requiring inspection; native skill status reports their ownership."
            ),
        }
        if crate::plugin_skills::discard_session(&root, &config, session.setup.task_id).is_err() {
            eprintln!(
                "Recollect kept locally changed skill copies; discard them explicitly after inspection."
            );
        }
    }
    let changed = {
        let mut comparable = session.clone();
        comparable.last_event_at = previous_event_at;
        serde_json::to_value(&comparable)? != persisted
    };
    if changed || session.last_event_at - previous_event_at >= SESSION_TOUCH {
        plugin_storage::write(&state, &session)?;
    }
    let launch = session.launch();
    let configured_secrets = secrets();
    // Independent of capture policy: share only the bounded coordination
    // projection. This is local enqueue/cache access, never network/keychain I/O.
    if crate::plugin_coordination::observe(&root, &config, &session, &event, &configured_secrets)
        .is_ok()
    {
        let _ = crate::plugin_coordination::wake(&root, session.id);
    }
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
    let wants_capture =
        name != "ContextRequest" && !session.setup.binding_id.is_nil() && capture_scope_ready;
    let mut inbox = if wants_capture {
        Some(launch.inbox()?)
    } else {
        launch.inbox().ok()
    };
    if wants_capture && let Some(inbox) = inbox.as_mut() {
        captured = inbox
            .capture_in_launch(
                session.setup.binding_id,
                Some(session.id),
                raw,
                &configured_secrets,
                Utc::now(),
            )
            .ok();
    }
    // A drain is only useful when something waits: queued events, or an ended
    // session whose task still has to close. Starting one per event is waste.
    if session.ended
        || inbox
            .as_ref()
            .is_none_or(|i| i.has_pending().unwrap_or(true))
    {
        wake_delivery()?;
    }
    drop(inbox);
    if config.with_runner && boundary {
        spawn("runner")?;
    }
    let mut output = json!({});
    if let Some(snapshot) = native_cleanup {
        output["recollectNativeSkillsSnapshot"] = snapshot;
    }
    let native_boundary = matches!(
        name,
        "SessionStart" | "UserPromptSubmit" | "PostCompact" | "ContextRequest" | "SubagentStart"
    ) && event["recollect_capture_only"] != true
        && !session.ended;
    if let Some(device) = device.as_ref() {
        let query = event["prompt"]
            .as_str()
            .unwrap_or("Important decisions, facts and procedures for continuing this task");
        let mut query = sanitize_capture_text(query, &configured_secrets);
        let mut end = query.len().min(512);
        while !query.is_char_boundary(end) {
            end -= 1;
        }
        query.truncate(end);
        // One retrieval operation and one catalogue serve both consumers.
        let wanted = needs_context && !query.trim().is_empty() || native_boundary;
        let resources = if wanted {
            // Native synchronization always had a short budget for its network
            // work; prompt-only recall keeps the hook deadline for the operation
            // and the catalogue's own 800 ms bound.
            let (budget, until) = if native_boundary {
                let budget = crate::plugin_skills::native_cli::SNAPSHOT_BUDGET;
                (budget, deadline.min(tokio::time::Instant::now() + budget))
            } else {
                (Duration::from_millis(800), deadline)
            };
            Some(
                match tokio::time::timeout_at(
                    until,
                    crate::plugin_skills::boundary_resources(
                        &client, device, &config, &session, budget,
                    ),
                )
                .await
                {
                    Ok(resolved) => resolved,
                    Err(_) => Err(std::sync::Arc::new(anyhow!("native_snapshot_timeout"))),
                },
            )
        } else {
            None
        };
        let recall = async {
            if !needs_context || query.trim().is_empty() {
                return None;
            }
            let Some(Ok(resources)) = resources.as_ref() else {
                return None;
            };
            tokio::time::timeout_at(
                deadline,
                plugin_session::recall(&client, device, &session, &query, resources),
            )
            .await
            .ok()?
            .ok()
        };
        let native = async {
            let resources = resources.as_ref().filter(|_| native_boundary)?;
            Some(
                match tokio::time::timeout_at(
                    deadline,
                    crate::plugin_skills::native_cli::sync_session(
                        &config, &client, device, &session, resources,
                    ),
                )
                .await
                {
                    Ok(Ok(snapshot)) => snapshot,
                    _ => {
                        json!({"state":"unavailable","native_loaded":false,"next_action":"Retry skills sync with a current read operation; inspect native skill status for conflicts."})
                    }
                },
            )
        };
        let (context, native) = tokio::join!(recall, native);
        if let Some(context) = context {
            output = plugin_session::output(name, &context);
        }
        if let Some(native) = native {
            output["recollectNativeSkillsSnapshot"] = native;
        }
    }
    if host == "opencode" && capture_scope_ready && !session.setup.binding_id.is_nil() {
        output["recollectCaptureBinding"] = json!(launch.current()?);
        output["recollectCapturedEvent"] = json!(captured);
    }
    let coordination_delivery_boundary = if host == "opencode" {
        // Its execute.before callback consumes no hook return value. Leave
        // notices queued until the context adapter actually adds them to system.
        matches!(name, "UserPromptSubmit" | "ContextRequest")
    } else {
        matches!(
            name,
            "SessionStart"
                | "UserPromptSubmit"
                | "PreToolUse"
                | "PostCompact"
                | "ContextRequest"
                | "SubagentStart"
        )
    };
    if coordination_delivery_boundary
        && let Ok(context) = crate::plugin_coordination::context(&root, &config, session.id)
    {
        output = crate::plugin_coordination::merge_output(output, name, &context);
    }
    Ok(plugin_session::native_output(output, host, name))
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
        if session.setup.endpoint != client.endpoint || session.setup.device_id != device.device_id
        {
            continue;
        }
        if session.closed {
            // Closed state is only needed if the host resumes this session.
            plugin_session::archive(&path)?;
            continue;
        }
        // A host that died without its end hook leaves an open task behind.
        if !session.ended && Utc::now() - session.last_event_at > plugin_session::RETAIN {
            session.ended = true;
            plugin_storage::write(&path, &session)?;
        }
        if !session.ended {
            continue;
        }
        if let Ok(Ok(())) = tokio::time::timeout(
            Duration::from_secs(3),
            plugin_session::close(client, device, &mut session),
        )
        .await
        {
            plugin_storage::write(&path, &session)?;
            plugin_session::archive(&path)?;
        } else {
            pending = true;
        }
    }
    plugin_session::prune(root, Utc::now())?;
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
            )
            .await;
        }
        "drain" => drain().await?,
        "coordination-worker" => {
            ensure!(args.len() == 2, "invalid_coordination_worker");
            crate::plugin_coordination::worker(args[1].parse()?).await?
        }
        "coordination" => {
            let (config, _) =
                selected(plugin_storage::config()?, &std::env::current_dir()?).await?;
            let client = Client::new(&config.endpoint)?;
            crate::plugin_coordination::run(&config, &client, &credential(&config)?, &args[1..])
                .await?
        }
        "runner" => runner().await?,
        "status" => status().await?,
        "skills" => {
            let offline = args.get(1).is_some_and(|s| {
                matches!(
                    s.as_str(),
                    "discard" | "status" | "leases" | "reload" | "uninstall"
                )
            });
            let config = plugin_storage::config()?;
            let config = if offline {
                config
            } else {
                let directory = args
                    .iter()
                    .position(|s| s == "--directory")
                    .and_then(|i| args.get(i + 1))
                    .map(PathBuf::from)
                    .unwrap_or(std::env::current_dir()?);
                selected(config, &directory).await?.0
            };
            let client = Client::new(&config.endpoint)?;
            let device = if offline {
                StoredDevice {
                    endpoint: config.endpoint.clone(),
                    device_id: config.device,
                    token: Uuid::nil(),
                }
            } else {
                credential(&config)?
            };
            crate::plugin_skills::run(&config, &client, &device, &args[1..]).await?
        }
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
