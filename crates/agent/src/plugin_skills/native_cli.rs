//! Native CLI and bounded lifecycle synchronization. No harness is reported
//! loaded merely because files were exported or a refresh was requested.
use super::{
    GuidanceHttpStatus, catalogue,
    native::{self, NativeHost, NativeScope},
    query, read_json,
};
use crate::{
    Client, StoredDevice,
    plugin_session::Session,
    plugin_storage::{self, Config},
};
use anyhow::{Result, anyhow, bail, ensure};
use recollect_protocol::{ManagedSkillBundle, OperationBinding, TaskDetail};
use reqwest::Method;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

pub const USAGE: &str = "skills install ID --operation UUID --host HOST --directory PATH (--session TASK_UUID|--persist); skills sync --operation UUID --host HOST --directory PATH [--session TASK_UUID|--persist]; skills reload|status --host HOST --directory PATH; skills uninstall ID --host HOST --directory PATH (--session TASK_UUID|--persist); skills leases --directory PATH";

fn host(value: &str) -> Result<NativeHost> {
    NativeHost::parse(value)
}

/// Use the nearest configured boundary, then the Git root; an explicitly chosen
/// standalone directory is also useful for clean/isolated workspaces.
pub async fn workspace(
    config: &Config,
    client: &Client,
    device: &StoredDevice,
    input: &Path,
    offline: bool,
) -> Result<PathBuf> {
    ensure!(input.is_absolute(), "native_workspace_must_be_absolute");
    let mut checked = PathBuf::new();
    for component in input.components() {
        ensure!(
            !matches!(component, std::path::Component::ParentDir),
            "native_workspace_parent_component"
        );
        checked.push(component);
        ensure!(
            !tokio::fs::symlink_metadata(&checked).await?.is_symlink(),
            "native_workspace_symlink"
        );
    }
    let directory = tokio::fs::canonicalize(input).await?;
    ensure!(directory.is_dir(), "native_workspace_not_directory");
    let mut selector = false;
    for parent in directory.ancestors() {
        match tokio::fs::symlink_metadata(parent.join(".recollect/workspace.toml")).await {
            Ok(_) => {
                selector = true;
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => bail!("native_workspace_selector_unavailable"),
        }
    }
    let root = if selector {
        let (root, selected) = crate::workspace::selector(&directory).await?;
        if let Ok(brain) = selected.parse::<Uuid>() {
            ensure!(
                offline || brain == config.brain,
                "native_workspace_brain_mismatch"
            );
        } else if !offline {
            let matches: Vec<_> = client
                .brains(device)
                .await?
                .into_iter()
                .filter(|b| b.name == selected)
                .collect();
            ensure!(
                matches.len() == 1 && matches[0].id == config.brain,
                "native_workspace_brain_mismatch"
            );
        }
        root
    } else {
        let git = tokio::time::timeout(
            Duration::from_secs(2),
            tokio::process::Command::new("git")
                .args(["-C"])
                .arg(&directory)
                .args(["rev-parse", "--show-toplevel"])
                .output(),
        )
        .await;
        match git {
            Ok(Ok(output)) if output.status.success() => {
                tokio::fs::canonicalize(String::from_utf8(output.stdout)?.trim()).await?
            }
            _ => directory,
        }
    };
    ensure!(root.parent().is_some(), "native_workspace_is_root");
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        ensure!(
            root != home && !home.starts_with(&root),
            "native_workspace_is_home"
        );
        for global in [
            home.join(".agents"),
            home.join(".claude"),
            home.join(".codex"),
            home.join(".config/opencode"),
        ] {
            ensure!(
                !root.starts_with(global),
                "native_global_installation_forbidden"
            );
        }
    }
    for (variable, suffix) in [
        ("CODEX_HOME", ""),
        ("CLAUDE_CONFIG_DIR", ""),
        ("XDG_CONFIG_HOME", "opencode"),
        ("OPENCODE_CONFIG_DIR", ""),
    ] {
        if let Some(value) = std::env::var_os(variable) {
            check_global_root(&root, &PathBuf::from(value).join(suffix)).await?;
        }
    }
    Ok(root)
}

async fn check_global_root(root: &Path, configured: &Path) -> Result<()> {
    let global = tokio::fs::canonicalize(configured)
        .await
        .unwrap_or_else(|_| configured.to_owned());
    ensure!(
        !root.starts_with(global),
        "native_global_installation_forbidden"
    );
    Ok(())
}

async fn operation(
    config: &Config,
    client: &Client,
    device: &StoredDevice,
    id: Uuid,
) -> Result<OperationBinding> {
    let operation: OperationBinding = read_json(
        client,
        device,
        &format!("/api/brains/{}/workspace/operations/{id}", config.brain),
    )
    .await?;
    ensure!(
        operation.id == id
            && operation.brain_id == config.brain
            && operation.device_id == Some(device.device_id)
            && operation.scope.task_id == operation.task_id
            && operation.scope.brain_id == config.brain
            && operation.scope_valid
            && matches!(operation.kind.as_str(), "context" | "retrieval"),
        "native_operation_mismatch"
    );
    let task: TaskDetail = read_json(
        client,
        device,
        &format!(
            "/api/brains/{}/workspace/tasks/{}",
            config.brain, operation.task_id
        ),
    )
    .await?;
    ensure!(
        !task.task.closed
            && task.task.scope_valid
            && task.task.id == operation.task_id
            && task.task.brain_id == config.brain
            && task.task.scope.id == operation.scope.id
            && task.task.device_id == Some(device.device_id),
        "native_task_closed_or_foreign"
    );
    Ok(operation)
}

async fn bundle(
    config: &Config,
    client: &Client,
    device: &StoredDevice,
    operation: Uuid,
    id: Uuid,
) -> Result<ManagedSkillBundle> {
    let result: ManagedSkillBundle = read_json(
        client,
        device,
        &query(
            &format!("/api/brains/{}/skills/{id}", config.brain),
            &[("operation_id", operation.to_string())],
        )?,
    )
    .await?;
    ensure!(result.skill.id == id, "native_bundle_mismatch");
    super::validate_bundle(&result, config)?;
    Ok(result)
}

/// Fetch the full authoritative snapshot before mutating native roots. A failed
/// member fetch must never look like a canonical deletion to the installer.
async fn bundles(
    config: &Config,
    client: &Client,
    device: &StoredDevice,
    operation: Uuid,
) -> Result<Vec<ManagedSkillBundle>> {
    let entries = catalogue(client, device, config.brain, operation, None).await?;
    let mut result = Vec::with_capacity(entries.len());
    for batch in entries.chunks(4) {
        let mut tasks = tokio::task::JoinSet::new();
        for entry in batch {
            let endpoint = config.endpoint.clone();
            let config = config.clone();
            let device = StoredDevice {
                endpoint: device.endpoint.clone(),
                device_id: device.device_id,
                token: device.token,
            };
            let id = entry.id;
            tasks.spawn(async move {
                bundle(&config, &Client::new(&endpoint)?, &device, operation, id).await
            });
        }
        while let Some(value) = tasks.join_next().await {
            result.push(value??);
        }
    }
    Ok(result)
}

pub fn refresh_state(host: NativeHost, report: Value) -> Value {
    let name = serde_json::to_value(host).unwrap_or(Value::Null);
    let strategy = match name.as_str() {
        Some("codex") => "native_watcher",
        Some("claude_code") => "native_watcher_or_reload_skills",
        _ => "opencode_skill_reload_on_context",
    };
    json!({"placement":report,"refresh_state":"refresh_requested","strategy":strategy,"native_loaded":false,"next_action":match name.as_str(){Some("claude_code")=>"The plugin requests a SessionStart skill rescan on supported Claude Code. An already running host with a newly created root can use /reload-skills.",Some("codex")=>"Codex watches skill files; refresh the native catalog or restart if the host has not observed a change.",_=>"The installed Recollect OpenCode adapter refreshes its native transform on the next context boundary."}})
}

pub async fn run(
    config: &Config,
    client: &Client,
    device: &StoredDevice,
    args: &[String],
) -> Result<Value> {
    run_at(&plugin_storage::root()?, config, client, device, args).await
}
async fn run_at(
    root: &Path,
    config: &Config,
    client: &Client,
    device: &StoredDevice,
    args: &[String],
) -> Result<Value> {
    let action = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| anyhow!("{USAGE}"))?;
    let has_id = matches!(action, "install" | "uninstall");
    let id = if has_id {
        Some(
            args.get(1)
                .ok_or_else(|| anyhow!("{USAGE}"))?
                .parse::<Uuid>()?,
        )
    } else {
        None
    };
    let mut values = HashMap::new();
    let mut index = if has_id { 2 } else { 1 };
    while index < args.len() {
        let key = args[index].as_str();
        ensure!(
            matches!(
                key,
                "--operation" | "--host" | "--directory" | "--session" | "--persist"
            ),
            "{USAGE}"
        );
        let value = if key == "--persist" {
            "true".to_owned()
        } else {
            index += 1;
            args.get(index)
                .filter(|s| !s.starts_with("--"))
                .ok_or_else(|| anyhow!("{USAGE}"))?
                .clone()
        };
        ensure!(
            values.insert(key, value).is_none(),
            "duplicate_native_option"
        );
        index += 1;
    }
    let allowed: &[&str] = match action {
        "install" | "sync" => &[
            "--operation",
            "--host",
            "--directory",
            "--session",
            "--persist",
        ],
        "uninstall" => &["--host", "--directory", "--session", "--persist"],
        "reload" | "status" => &["--host", "--directory"],
        "leases" => &["--directory"],
        _ => bail!("{USAGE}"),
    };
    ensure!(values.keys().all(|k| allowed.contains(k)), "{USAGE}");
    let input = values
        .get("--directory")
        .ok_or_else(|| anyhow!("{USAGE}"))?;
    let workspace = workspace(
        config,
        client,
        device,
        Path::new(input),
        !matches!(action, "install" | "sync"),
    )
    .await?;
    if action == "leases" {
        return native::leases(&NativeScope {
            root: &root,
            config,
            workspace: &workspace,
            host: NativeHost::Codex,
            environment: None,
        });
    }
    let host = host(values.get("--host").ok_or_else(|| anyhow!("{USAGE}"))?)?;
    let mut local_config = config.clone();
    if !matches!(action, "install" | "sync") {
        let lease = native::leases(&NativeScope {
            root: &root,
            config,
            workspace: &workspace,
            host,
            environment: None,
        })?;
        if let Some(scope) = lease.get("scope").filter(|s| !s.is_null()) {
            ensure!(
                scope["endpoint"] == config.endpoint,
                "native_workspace_endpoint_mismatch"
            );
            local_config.brain = scope["brain"]
                .as_str()
                .ok_or_else(|| anyhow!("native_scope_invalid"))?
                .parse()?;
        }
    }
    let config = &local_config;
    let session = values
        .get("--session")
        .map(|id| id.parse::<Uuid>())
        .transpose()?;
    let persistent = values.contains_key("--persist");
    ensure!(
        !(persistent && session.is_some()),
        "choose_session_or_persist"
    );
    if matches!(action, "install" | "uninstall") {
        ensure!(persistent != session.is_some(), "choose_session_or_persist");
    }
    let op = if matches!(action, "install" | "sync") {
        Some(
            operation(
                config,
                client,
                device,
                values
                    .get("--operation")
                    .ok_or_else(|| anyhow!("{USAGE}"))?
                    .parse()?,
            )
            .await?,
        )
    } else {
        None
    };
    if let (Some(session), Some(op)) = (session, &op) {
        ensure!(session == op.task_id, "native_session_mismatch");
    }
    let scope = NativeScope {
        root: &root,
        config,
        workspace: &workspace,
        host,
        environment: op.as_ref().and_then(|op| op.scope.selection.environment_id),
    };
    if let Some(op) = &op {
        remember_task(root, config, &workspace, host, &op.scope)?;
    }
    let report = match action {
        "status" => return Ok(serde_json::to_value(native::status(&scope)?)?),
        "reload" => native::request_refresh(&scope)?,
        "uninstall" => native::uninstall(&scope, id, session)?,
        "install" => {
            let op = op.as_ref().expect("operation");
            let bundle = bundle(config, client, device, op.id, id.expect("id")).await?;
            super::save(
                &root,
                config,
                bundle.clone(),
                (!persistent).then_some(op.task_id),
            )?;
            native::install_scoped(&scope, op.task_id, op.scope.id, &[bundle], persistent)?
        }
        "sync" => {
            let op = op.as_ref().expect("operation");
            let bundles = bundles(config, client, device, op.id).await?;
            for bundle in &bundles {
                super::save(
                    &root,
                    config,
                    bundle.clone(),
                    (!persistent).then_some(op.task_id),
                )?;
            }
            native::sync_scoped(&scope, op.task_id, op.scope.id, &bundles, persistent)?
        }
        _ => bail!("{USAGE}"),
    };
    Ok(refresh_state(host, serde_json::to_value(report)?))
}

/// A small recovery journal is committed before native temporary references.
/// It deliberately contains no guidance body and fits portable storage's 64KiB read limit.
fn task_pointer(root: &Path, config: &Config, task: Uuid) -> PathBuf {
    root.join("native-skills/tasks")
        .join(format!("{}-{task}.json", config.device))
}
fn remember_task(
    root: &Path,
    config: &Config,
    workspace: &Path,
    host: NativeHost,
    scope: &recollect_protocol::ScopeSnapshot,
) -> Result<bool> {
    ensure!(scope.brain_id == config.brain, "native_snapshot_mismatch");
    let task = scope.task_id;
    let path = task_pointer(root, config, task);
    let mut changed = false;
    if path.try_exists()? {
        let previous: Value = plugin_storage::read(&path)?;
        ensure!(
            previous["endpoint"] == config.endpoint
                && previous["brain_id"] == json!(config.brain)
                && previous["device_id"] == json!(config.device)
                && previous["task_id"] == json!(task),
            "native_snapshot_mismatch"
        );
        ensure!(
            previous["workspace"].as_str() == workspace.to_str(),
            "native_task_workspace_mismatch"
        );
        changed = previous["scope_id"] != json!(scope.id);
    }
    // Recovery may have committed the new intent (or files) just before its
    // hook was interrupted. The last published catalog must catch up too.
    let published = root
        .join("native-skills/catalogues")
        .join(format!("{}-{task}.json", config.device));
    changed |= plugin_storage::read::<Value>(&published)
        .map_or(true, |catalog| catalog["scope_id"] != json!(scope.id));
    let native_scope = NativeScope {
        root,
        config,
        workspace,
        host,
        environment: scope.selection.environment_id,
    };
    native::observe_task_scope(&native_scope, task, scope.id, scope.created_at)?;
    // This journal is written while checking the common generation too, so a
    // slow old process cannot replace the new scope's recovery metadata.
    native::publish_scoped(
        &native_scope,
        task,
        scope.id,
        &path,
        &json!({"endpoint":config.endpoint,"brain_id":config.brain,"device_id":config.device,
        "task_id":task,"scope_id":scope.id,"workspace":workspace,"host":host}),
    )?;
    Ok(changed)
}
fn snapshot(
    root: &Path,
    config: &Config,
    workspace: &Path,
    task: Uuid,
    scope_id: Uuid,
    report: &native::NativeReport,
) -> Result<Value> {
    let path = root
        .join("native-skills/catalogues")
        .join(format!("{}-{task}.json", config.device));
    let mut entries = serde_json::to_value(&report.entries)?;
    for entry in entries
        .as_array_mut()
        .ok_or_else(|| anyhow!("native_snapshot_invalid"))?
    {
        entry
            .as_object_mut()
            .ok_or_else(|| anyhow!("native_snapshot_invalid"))?
            .remove("tasks");
    }
    let mut owned_exports = Vec::new();
    if !matches!(report.state.as_str(), "unavailable" | "scope_busy") {
        for host in [NativeHost::Codex, NativeHost::Claude, NativeHost::OpenCode] {
            if let Ok(status) = native::status(&NativeScope {
                root,
                config,
                workspace,
                host,
                environment: None,
            }) {
                owned_exports.extend(
                    status.entries.into_iter().map(
                        |e| json!({"id":e.id,"path":e.path,"host":e.host,"brain_id":e.brain_id}),
                    ),
                );
            }
        }
    }
    let body = json!({"owned_exports":owned_exports,"scope_id":scope_id,"task_id":task,"brain_id":config.brain,"workspace":workspace,"entries":entries,"native_loaded":false});
    ensure!(
        serde_json::to_vec(&body)?.len() <= 1024 * 1024,
        "native_snapshot_too_large"
    );
    native::publish_scoped(
        &NativeScope {
            root,
            config,
            workspace,
            host: NativeHost::OpenCode,
            environment: None,
        },
        task,
        scope_id,
        &path,
        &body,
    )?;
    Ok(
        json!({"snapshot":path,"workspace":workspace,"state":report.state,
        "changed":!report.files_changed.is_empty()||!report.files_removed.is_empty(),"native_loaded":false}),
    )
}

/// All network work completes inside one short budget before filesystem mutation.
/// Dropping this future during network I/O cannot schedule a later export. Native
/// filesystem transactions remain synchronous and protected by the hook watchdog.
pub async fn sync_session(
    config: &Config,
    client: &Client,
    device: &StoredDevice,
    session: &Session,
) -> Result<Value> {
    sync_session_at(&plugin_storage::root()?, config, client, device, session).await
}
async fn sync_session_at(
    root: &Path,
    config: &Config,
    client: &Client,
    device: &StoredDevice,
    session: &Session,
) -> Result<Value> {
    let host = host(&session.setup.host)?;
    // The session has already resolved and bound its Brain. Offline boundary
    // discovery is essential so revoked credentials can still retract copies.
    let workspace = workspace(
        config,
        client,
        device,
        &session.setup.working_directory,
        true,
    )
    .await?;
    let scope = NativeScope {
        root,
        config,
        workspace: &workspace,
        host,
        environment: session.scope.selection.environment_id,
    };
    // Release the previous scope before fetching any new snapshot, even offline.
    let scope_changed = remember_task(root, config, &workspace, host, &session.scope)?;
    let empty = |state: &str| {
        // OpenCode's native registry belongs to the workspace, not one task.
        // A failed scope switch must not hide another live task's catalog.
        let active_peer = native::status(&NativeScope {
            environment: None,
            ..scope
        })
        .is_ok_and(|report| {
            report.entries.iter().any(|entry| {
                entry
                    .tasks
                    .iter()
                    .any(|task| *task != session.setup.task_id)
            })
        });
        let state = if state == "unavailable" && active_peer {
            "scope_busy"
        } else {
            state
        };
        snapshot(
            root,
            config,
            &workspace,
            session.setup.task_id,
            session.scope.id,
            &native::NativeReport {
                state: state.into(),
                ..Default::default()
            },
        )
    };
    let fetch = tokio::time::timeout(Duration::from_millis(2400), async {
        let op: OperationBinding = super::decode_guidance(
            client
                .send(
                    Method::POST,
                    &format!(
                        "/api/brains/{}/workspace/tasks/{}/operations",
                        config.brain, session.setup.task_id
                    ),
                    Some(device.token),
                    Some(json!({"kind":"retrieval","expected_scope":session.scope.id})),
                )
                .await?,
        )
        .await?;
        ensure!(
            op.brain_id == config.brain
                && op.task_id == session.setup.task_id
                && op.scope.id == session.scope.id
                && op.device_id == Some(config.device)
                && op.scope_valid
                && op.kind == "retrieval",
            "native_operation_scope_mismatch"
        );
        bundles(config, client, device, op.id).await
    })
    .await
    .unwrap_or_else(|_| Err(anyhow!("native_snapshot_timeout")));
    let bundles = match fetch {
        Ok(value) => value,
        Err(error) => {
            if let Some(status) = error.downcast_ref::<GuidanceHttpStatus>() {
                let report = if status.0 == reqwest::StatusCode::UNAUTHORIZED {
                    Some(native::revoke(&scope)?)
                } else if matches!(
                    status.0,
                    reqwest::StatusCode::FORBIDDEN
                        | reqwest::StatusCode::NOT_FOUND
                        | reqwest::StatusCode::CONFLICT
                ) {
                    Some(native::release_scope_task_scoped(
                        &scope,
                        session.setup.task_id,
                        session.scope.id,
                    )?)
                } else {
                    None
                };
                if let Some(mut report) = report {
                    // Native registration belongs to the workspace. Reconcile
                    // remaining peer-owned exports even when this task lost access;
                    // canonical reads still deny this task independently.
                    report.state = "access_denied".into();
                    // A scope transition can leave a different environment's
                    // live peer registration active in this shared location.
                    if let Ok(active) = native::status(&NativeScope {
                        environment: None,
                        ..scope
                    }) {
                        report.entries = active.entries;
                    }
                    if report.entries.is_empty()
                        && native::leases(&scope).is_ok_and(|value| {
                            value["leases"].as_array().is_some_and(|leases| {
                                leases.iter().any(|lease| {
                                    lease["task"] != json!(session.setup.task_id)
                                        || lease["device"] != json!(config.device)
                                })
                            })
                        })
                    {
                        report.state = "scope_busy".into();
                    }
                    return snapshot(
                        root,
                        config,
                        &workspace,
                        session.setup.task_id,
                        session.scope.id,
                        &report,
                    );
                }
            }
            let current_lease = native::leases(&scope).is_ok_and(|value| {
                value["scope"]["endpoint"] == config.endpoint
                    && value["scope"]["brain"] == json!(config.brain)
                    && value["scope"]["environment"]
                        == json!(session.scope.selection.environment_id)
                    && value["leases"].as_array().is_some_and(|leases| {
                        leases.iter().any(|lease| {
                            lease["task"] == json!(session.setup.task_id)
                                && lease["device"] == json!(config.device)
                        })
                    })
            });
            if scope_changed || !current_lease {
                if current_lease {
                    let mut report = native::status(&scope)?;
                    report.state = "unavailable".into();
                    return snapshot(
                        root,
                        config,
                        &workspace,
                        session.setup.task_id,
                        session.scope.id,
                        &report,
                    );
                }
                return empty("unavailable");
            }
            return Err(error);
        }
    };
    for bundle in &bundles {
        super::save(root, config, bundle.clone(), Some(session.setup.task_id))?;
    }
    let report = match native::sync_scoped(
        &scope,
        session.setup.task_id,
        session.scope.id,
        &bundles,
        false,
    ) {
        Ok(report) => report,
        Err(error) if error.to_string().contains("native_scope_busy") => {
            return empty("scope_busy");
        }
        Err(error) if error.to_string().contains("native_scope_changed") => return Err(error),
        Err(_) if scope_changed => return empty("unavailable"),
        Err(error) => return Err(error),
    };
    snapshot(
        root,
        config,
        &workspace,
        session.setup.task_id,
        session.scope.id,
        &report,
    )
}

pub fn release_session(config: &Config, session: &Session) -> Result<Option<Value>> {
    release_session_at(&plugin_storage::root()?, config, session)
}
fn release_session_at(root: &Path, config: &Config, session: &Session) -> Result<Option<Value>> {
    let pointer = task_pointer(root, config, session.setup.task_id);
    if !pointer.try_exists()? {
        return Ok(None);
    }
    let saved: Value = plugin_storage::read(&pointer)?;
    ensure!(
        saved["task_id"] == json!(session.setup.task_id)
            && saved["brain_id"] == json!(config.brain)
            && saved["device_id"] == json!(config.device)
            && saved["endpoint"] == config.endpoint,
        "native_snapshot_mismatch"
    );
    let directory = PathBuf::from(
        saved["workspace"]
            .as_str()
            .ok_or_else(|| anyhow!("native_snapshot_invalid"))?,
    );
    let report = native::release_task(
        &NativeScope {
            root,
            config,
            workspace: &directory,
            host: host(&session.setup.host)?,
            environment: None,
        },
        session.setup.task_id,
    )?;
    // Keep a final catalogue for the host adapter's cleanup reload. The saved
    // generation may be newer than an offline recovery session's scope copy.
    let scope_id = saved["scope_id"]
        .as_str()
        .ok_or_else(|| anyhow!("native_snapshot_invalid"))?
        .parse()?;
    let result = snapshot(
        root,
        config,
        &directory,
        session.setup.task_id,
        scope_id,
        &report,
    )?;
    std::fs::remove_file(pointer)?;
    Ok(Some(result))
}

#[cfg(test)]
mod tests;
