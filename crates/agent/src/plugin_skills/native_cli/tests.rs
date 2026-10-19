use super::*;
use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    response::IntoResponse,
};
use recollect_protocol::{
    ManagedSkillFile, ManagedSkillKind, ManagedSkillSummary, ScopeSnapshot, WorkspaceTask,
};
use reqwest::Method;
use std::sync::{Arc, Mutex};

struct Fixture {
    base: PathBuf,
    root: PathBuf,
    workspace: PathBuf,
    config: Config,
    session: Session,
    bundles: Vec<ManagedSkillBundle>,
}
impl Fixture {
    fn new() -> Self {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache")
            .canonicalize()
            .unwrap()
            .join(format!("native-cli-{}", Uuid::new_v4()));
        let root = base.join("plugin");
        let workspace = base.join("workspace");
        plugin_storage::directory(&root).unwrap();
        plugin_storage::directory(&workspace.join(".recollect")).unwrap();
        let config = Config {
            endpoint: "http://127.0.0.1:1".into(),
            brain: Uuid::new_v4(),
            device: Uuid::new_v4(),
            profile: "test".into(),
            with_runner: false,
            runner_id: None,
        };
        std::fs::write(
            workspace.join(".recollect/workspace.toml"),
            format!("brain = \"{}\"\n", config.brain),
        )
        .unwrap();
        let task = Uuid::new_v4();
        let now = chrono::Utc::now();
        let scope = ScopeSnapshot {
            id: Uuid::new_v4(),
            task_id: task,
            brain_id: config.brain,
            selection: Default::default(),
            repositories: vec![],
            areas: vec![],
            environment: None,
            created_at: now,
        };
        let session = Session {
            id: Uuid::new_v4(),
            setup: crate::capture_cli::HookSetup {
                endpoint: config.endpoint.clone(),
                device_id: config.device,
                brain_id: config.brain,
                binding_id: Uuid::nil(),
                evidence_root: root.clone(),
                device_profile: config.profile.clone(),
                host: "opencode".into(),
                working_directory: workspace.clone(),
                task_id: task,
                created_task: true,
                plugin_root: None,
            },
            scope,
            host_version: "test".into(),
            agent_id: None,
            ended: false,
            closed: false,
            last_event_at: now,
        };
        let bundles=(0..2).map(|n|{let name=format!("test-guide-{n}");let description="Follow the test guidance.";let files=vec![ManagedSkillFile{path:"SKILL.md".into(),content:format!("---\nname: {name}\ndescription: {description}\n---\n\nTest body {n}. Read [reference](references/guide.md).")},ManagedSkillFile{path:"references/guide.md".into(),content:"Test reference.".into()}];ManagedSkillBundle{skill:ManagedSkillSummary{id:Uuid::new_v4(),brain_id:config.brain,name,description:description.into(),kind:ManagedSkillKind::Skill,version:1,environment_id:None,profile_ids:vec![],tags:vec![],enabled:true,file_paths:files.iter().map(|f|f.path.clone()).collect(),updated_at:now},files}}).collect();
        Self {
            base,
            root,
            workspace,
            config,
            session,
            bundles,
        }
    }
    fn scope(&self) -> NativeScope<'_> {
        NativeScope {
            root: &self.root,
            config: &self.config,
            workspace: &self.workspace,
            host: NativeHost::OpenCode,
            environment: self.session.scope.selection.environment_id,
        }
    }
    fn device(&self) -> StoredDevice {
        StoredDevice {
            endpoint: self.config.endpoint.clone(),
            device_id: self.config.device,
            token: Uuid::new_v4(),
        }
    }
    fn install(&self) {
        remember_task(
            &self.root,
            &self.config,
            &self.workspace,
            NativeHost::OpenCode,
            &self.session.scope,
        )
        .unwrap();
        let r = native::sync(
            &self.scope(),
            self.session.setup.task_id,
            &self.bundles,
            false,
        )
        .unwrap();
        assert!(r.errors.is_empty());
        snapshot(
            &self.root,
            &self.config,
            &self.workspace,
            self.session.setup.task_id,
            self.session.scope.id,
            &r,
        )
        .unwrap();
    }
    fn args(&self, action: &str, with_id: bool) -> Vec<String> {
        let mut args = vec![action.into()];
        if with_id {
            args.push(self.bundles[0].skill.id.to_string());
        }
        args.extend([
            "--host".into(),
            "opencode".into(),
            "--directory".into(),
            self.workspace.to_str().unwrap().into(),
        ]);
        args
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}
#[derive(Clone)]
struct Server {
    operation: OperationBinding,
    bundles: Vec<ManagedSkillBundle>,
    op_status: u16,
    fail_bundle: Option<Uuid>,
    slow: bool,
    closed: bool,
    hold: Option<(Uuid, Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>,
    fail_status: u16,
}
async fn serve(
    State(state): State<Arc<Mutex<Server>>>,
    request: Request,
) -> axum::response::Response {
    let f = state.lock().unwrap().clone();
    let path = request.uri().path();
    if path.ends_with("/operations") && request.method() == Method::POST {
        if f.slow {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        if f.op_status != 200 {
            return StatusCode::from_u16(f.op_status).unwrap().into_response();
        }
        return Json(f.operation).into_response();
    }
    if path.contains("/operations/") {
        return Json(f.operation).into_response();
    }
    if path.contains("/tasks/") {
        let now = chrono::Utc::now();
        let task = WorkspaceTask {
            id: f.operation.task_id,
            brain_id: f.operation.brain_id,
            parent_task_id: None,
            continuation_of_task_id: None,
            workspace_id: None,
            created_by: Uuid::new_v4(),
            device_id: f.operation.device_id,
            label: "test".into(),
            closed: f.closed,
            scope: f.operation.scope,
            scope_valid: true,
            created_at: now,
            updated_at: now,
        };
        return Json(TaskDetail {
            task,
            scopes: vec![],
            scope_total: 0,
            scope_offset: 0,
            operations: vec![],
            operation_total: 0,
            operation_offset: 0,
        })
        .into_response();
    }
    if path.ends_with("/skills") {
        return Json(
            f.bundles
                .iter()
                .map(|b| b.skill.clone())
                .collect::<Vec<_>>(),
        )
        .into_response();
    }
    if let Some(id) = path.rsplit('/').next().and_then(|s| s.parse::<Uuid>().ok()) {
        if let Some((held, entered, released)) = &f.hold {
            if *held == id {
                entered.notify_one();
                released.notified().await;
            }
        }
        if Some(id) == f.fail_bundle {
            return StatusCode::from_u16(f.fail_status).unwrap().into_response();
        }
        if let Some(b) = f.bundles.into_iter().find(|b| b.skill.id == id) {
            return Json(b).into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}
async fn server(f: &mut Fixture) -> (Arc<Mutex<Server>>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    f.config.endpoint = format!("http://{}", listener.local_addr().unwrap());
    f.session.setup.endpoint = f.config.endpoint.clone();
    let state = Arc::new(Mutex::new(Server {
        operation: OperationBinding {
            id: Uuid::new_v4(),
            brain_id: f.config.brain,
            task_id: f.session.setup.task_id,
            scope: f.session.scope.clone(),
            scope_valid: true,
            kind: "retrieval".into(),
            actor_id: Uuid::new_v4(),
            device_id: Some(f.config.device),
            created_at: chrono::Utc::now(),
        },
        bundles: f.bundles.clone(),
        op_status: 200,
        fail_bundle: None,
        slow: false,
        closed: false,
        hold: None,
        fail_status: 503,
    }));
    let app = Router::new().fallback(serve).with_state(state.clone());
    (
        state,
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() }),
    )
}
#[tokio::test]
async fn cli_temporary_install_is_recovered_at_session_end_and_offline_cleanup_uses_ledger_brain() {
    let mut f = Fixture::new();
    let (state, task) = server(&mut f).await;
    let op = state.lock().unwrap().operation.id;
    let mut args = f.args("install", true);
    args.extend([
        "--operation".into(),
        op.to_string(),
        "--session".into(),
        f.session.setup.task_id.to_string(),
    ]);
    run_at(
        &f.root,
        &f.config,
        &Client::new(&f.config.endpoint).unwrap(),
        &f.device(),
        &args,
    )
    .await
    .unwrap();
    assert_eq!(native::status(&f.scope()).unwrap().entries.len(), 1);
    task.abort();
    let mut default = f.config.clone();
    default.brain = Uuid::new_v4();
    let client = Client::new(&default.endpoint).unwrap();
    let status = run_at(
        &f.root,
        &default,
        &client,
        &f.device(),
        &f.args("status", false),
    )
    .await
    .unwrap();
    assert_eq!(status["entries"].as_array().unwrap().len(), 1);
    release_session_at(&f.root, &f.config, &f.session).unwrap();
    assert!(native::status(&f.scope()).unwrap().entries.is_empty());
    assert!(native::sync(&f.scope(), f.session.setup.task_id, &f.bundles, false).is_err());
}
#[tokio::test]
async fn offline_named_selector_allows_local_status_and_explicit_uninstall() {
    let f = Fixture::new();
    f.install();
    std::fs::write(
        f.workspace.join(".recollect/workspace.toml"),
        "brain = \"Friendly Brain\"\n",
    )
    .unwrap();
    let client = Client::new(&f.config.endpoint).unwrap();
    let mut args = f.args("uninstall", true);
    args.extend(["--session".into(), f.session.setup.task_id.to_string()]);
    run_at(&f.root, &f.config, &client, &f.device(), &args)
        .await
        .unwrap();
    assert_eq!(native::status(&f.scope()).unwrap().entries.len(), 1);
}
#[tokio::test]
async fn failed_member_fetch_never_prunes_complete_owned_snapshot() {
    let mut f = Fixture::new();
    let (state, task) = server(&mut f).await;
    f.install();
    state.lock().unwrap().fail_bundle = Some(f.bundles[1].skill.id);
    assert!(
        sync_session_standalone(
            &f.root,
            &f.config,
            &Client::new(&f.config.endpoint).unwrap(),
            &f.device(),
            &f.session
        )
        .await
        .is_err()
    );
    assert_eq!(native::status(&f.scope()).unwrap().entries.len(), 2);
    task.abort();
}
#[tokio::test]
async fn op_creation_revocation_retracts_only_this_device_preserving_peer() {
    let mut f = Fixture::new();
    let (state, task) = server(&mut f).await;
    f.install();
    let mut peer = f.config.clone();
    peer.device = Uuid::new_v4();
    let peerscope = NativeScope {
        config: &peer,
        ..f.scope()
    };
    native::install(&peerscope, Uuid::new_v4(), &f.bundles[..1], false).unwrap();
    state.lock().unwrap().op_status = 401;
    let output = sync_session_standalone(
        &f.root,
        &f.config,
        &Client::new(&f.config.endpoint).unwrap(),
        &f.device(),
        &f.session,
    )
    .await
    .unwrap();
    assert_eq!(output["state"], "access_denied");
    let catalog: Value =
        serde_json::from_slice(&std::fs::read(output["snapshot"].as_str().unwrap()).unwrap())
            .unwrap();
    assert_eq!(catalog["entries"].as_array().unwrap().len(), 1);
    assert_eq!(catalog["entries"][0]["id"], json!(f.bundles[0].skill.id));
    let left = native::status(&f.scope()).unwrap();
    assert_eq!(left.entries.len(), 1);
    assert!(
        left.entries[0]
            .tasks
            .iter()
            .all(|id| *id != f.session.setup.task_id)
    );
    task.abort();
}
#[tokio::test]
async fn scope_change_offline_releases_old_refs_and_clears_host_snapshot() {
    let mut f = Fixture::new();
    f.install();
    f.session.scope.id = Uuid::new_v4();
    f.session.scope.created_at += chrono::Duration::milliseconds(1);
    f.session.scope.selection.environment_id = Some(Uuid::new_v4());
    let output = sync_session_standalone(
        &f.root,
        &f.config,
        &Client::new(&f.config.endpoint).unwrap(),
        &f.device(),
        &f.session,
    )
    .await
    .unwrap();
    assert_eq!(output["state"], "unavailable");
    assert!(native::status(&f.scope()).unwrap().entries.is_empty());
}
#[tokio::test]
async fn closed_operation_and_timeout_do_not_allow_delayed_export() {
    let mut f = Fixture::new();
    let (state, task) = server(&mut f).await;
    f.install();
    state.lock().unwrap().op_status = 409;
    let output = sync_session_standalone(
        &f.root,
        &f.config,
        &Client::new(&f.config.endpoint).unwrap(),
        &f.device(),
        &f.session,
    )
    .await
    .unwrap();
    assert_eq!(output["state"], "access_denied");
    assert!(native::status(&f.scope()).unwrap().entries.is_empty());
    {
        let mut s = state.lock().unwrap();
        s.op_status = 200;
        s.slow = true;
    }
    let start = std::time::Instant::now();
    let timed_out = sync_session_standalone(
        &f.root,
        &f.config,
        &Client::new(&f.config.endpoint).unwrap(),
        &f.device(),
        &f.session,
    )
    .await
    .unwrap();
    assert_eq!(timed_out["state"], "unavailable");
    assert!(start.elapsed() < Duration::from_millis(2850));
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(native::status(&f.scope()).unwrap().entries.is_empty());
    task.abort();
}
#[tokio::test]
async fn sync_installs_complete_snapshot_and_disable_prunes_eligible_exports() {
    let mut f = Fixture::new();
    let (state, task) = server(&mut f).await;
    let client = Client::new(&f.config.endpoint).unwrap();
    let output = sync_session_standalone(&f.root, &f.config, &client, &f.device(), &f.session)
        .await
        .unwrap();
    assert_eq!(output["native_loaded"], false);
    assert_eq!(native::status(&f.scope()).unwrap().entries.len(), 2);
    state.lock().unwrap().bundles.clear();
    sync_session_standalone(&f.root, &f.config, &client, &f.device(), &f.session)
        .await
        .unwrap();
    assert!(native::status(&f.scope()).unwrap().entries.is_empty());
    task.abort();
}
#[tokio::test]
async fn parser_rejects_ambiguous_lifetimes_duplicate_options_and_symlink_workspace() {
    let f = Fixture::new();
    let client = Client::new(&f.config.endpoint).unwrap();
    for extra in [
        vec![
            "--persist".to_owned(),
            "--session".into(),
            Uuid::new_v4().to_string(),
        ],
        vec!["--persist".into(), "--persist".into()],
    ] {
        let mut args = f.args("install", true);
        args.extend(extra);
        assert!(
            run_at(&f.root, &f.config, &client, &f.device(), &args)
                .await
                .is_err()
        );
    }
    #[cfg(unix)]
    {
        let link = f.base.join("alias");
        std::os::unix::fs::symlink(&f.workspace, &link).unwrap();
        assert!(
            workspace(&f.config, &client, &f.device(), &link, true)
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn offline_scope_change_preserves_another_tasks_native_catalog() {
    let mut f = Fixture::new();
    f.install();
    let peer = Uuid::new_v4();
    native::install(&f.scope(), peer, &f.bundles[..1], false).unwrap();
    f.session.scope.id = Uuid::new_v4();
    f.session.scope.created_at += chrono::Duration::milliseconds(1);
    f.session.scope.selection.environment_id = Some(Uuid::new_v4());
    let output = sync_session_standalone(
        &f.root,
        &f.config,
        &Client::new(&f.config.endpoint).unwrap(),
        &f.device(),
        &f.session,
    )
    .await
    .unwrap();
    assert_eq!(output["state"], "scope_busy");
    let status = native::status(&NativeScope {
        environment: None,
        ..f.scope()
    })
    .unwrap();
    assert_eq!(status.entries.len(), 1);
    assert!(status.entries[0].tasks.contains(&peer));
}

#[tokio::test]
async fn delayed_old_scope_fetch_or_denial_cannot_recreate_exports_or_overwrite_new_snapshot() {
    for deny in [false, true] {
        let mut f = Fixture::new();
        let (state, server) = server(&mut f).await;
        f.install();
        let entered = Arc::new(tokio::sync::Notify::new());
        let released = Arc::new(tokio::sync::Notify::new());
        {
            let mut s = state.lock().unwrap();
            s.hold = Some((f.bundles[0].skill.id, entered.clone(), released.clone()));
            if deny {
                s.fail_bundle = Some(f.bundles[0].skill.id);
                s.fail_status = 403;
            }
        }
        let old_root = f.root.clone();
        let old_config = f.config.clone();
        let old_device = f.device();
        let old_session = f.session.clone();
        let old = tokio::spawn(async move {
            sync_session_standalone(
                &old_root,
                &old_config,
                &Client::new(&old_config.endpoint).unwrap(),
                &old_device,
                &old_session,
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(1), entered.notified())
            .await
            .unwrap();
        let mut next = f.session.scope.clone();
        next.id = Uuid::new_v4();
        next.created_at += chrono::Duration::milliseconds(1);
        next.selection.environment_id = Some(Uuid::new_v4());
        remember_task(
            &f.root,
            &f.config,
            &f.workspace,
            NativeHost::OpenCode,
            &next,
        )
        .unwrap();
        let next_scope = NativeScope {
            environment: next.selection.environment_id,
            ..f.scope()
        };
        let report = if deny {
            native::sync_scoped(&next_scope, next.task_id, next.id, &f.bundles, false).unwrap()
        } else {
            native::NativeReport {
                state: "unavailable".into(),
                ..Default::default()
            }
        };
        let pointer = snapshot(
            &f.root,
            &f.config,
            &f.workspace,
            next.task_id,
            next.id,
            &report,
        )
        .unwrap();
        released.notify_one();
        assert!(old.await.unwrap().is_err());
        let saved: Value =
            serde_json::from_slice(&std::fs::read(pointer["snapshot"].as_str().unwrap()).unwrap())
                .unwrap();
        assert_eq!(saved["scope_id"], json!(next.id));
        let status = native::status(&NativeScope {
            environment: None,
            ..f.scope()
        })
        .unwrap();
        assert_eq!(status.entries.len(), if deny { 2 } else { 0 });
        server.abort();
    }
}

#[tokio::test]
async fn configured_global_symlink_target_is_not_a_workspace() {
    let f = Fixture::new();
    #[cfg(unix)]
    {
        let configured = f.base.join("configured-codex-home");
        std::os::unix::fs::symlink(&f.workspace, &configured).unwrap();
        assert!(check_global_root(&f.workspace, &configured).await.is_err());
    }
}

#[tokio::test]
async fn retry_after_scope_withdrawal_interrupted_before_publication_clears_cached_host_view() {
    for applied in [false, true] {
        let mut f = Fixture::new();
        let (state, server) = server(&mut f).await;
        f.install();
        let old = native::status(&f.scope()).unwrap();
        let pointer = snapshot(
            &f.root,
            &f.config,
            &f.workspace,
            f.session.setup.task_id,
            f.session.scope.id,
            &old,
        )
        .unwrap();
        f.session.scope.id = Uuid::new_v4();
        f.session.scope.created_at += chrono::Duration::milliseconds(1);
        f.session.scope.selection.environment_id = Some(Uuid::new_v4());
        remember_task(
            &f.root,
            &f.config,
            &f.workspace,
            NativeHost::OpenCode,
            &f.session.scope,
        )
        .unwrap();
        if applied {
            native::sync_scoped(
                &f.scope(),
                f.session.setup.task_id,
                f.session.scope.id,
                &f.bundles,
                false,
            )
            .unwrap();
        }
        {
            let mut s = state.lock().unwrap();
            s.operation.scope = f.session.scope.clone();
            s.op_status = 503;
        }
        let output = sync_session_standalone(
            &f.root,
            &f.config,
            &Client::new(&f.config.endpoint).unwrap(),
            &f.device(),
            &f.session,
        )
        .await
        .unwrap();
        assert_eq!(output["state"], "unavailable");
        let saved: Value =
            serde_json::from_slice(&std::fs::read(pointer["snapshot"].as_str().unwrap()).unwrap())
                .unwrap();
        assert_eq!(saved["scope_id"], json!(f.session.scope.id));
        assert_eq!(
            saved["entries"].as_array().unwrap().len(),
            if applied { 2 } else { 0 }
        );
        server.abort();
    }
}
