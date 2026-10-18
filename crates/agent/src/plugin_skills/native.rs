//! Owned native exports. Placement is deliberately separate from host discovery
//! and activation. The workspace ledger serializes every host's overlapping
//! discovery roots; immutable receipts retain exact bytes without product hashes.
use super::validate_bundle;
use crate::plugin_storage::{self, Config};
use anyhow::{Result, anyhow, bail, ensure};
use recollect_protocol::{ManagedSkillBundle, ManagedSkillFile, ManagedSkillKind};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
use uuid::Uuid;

const RECEIPT_LIMIT: usize = 2 * 1024 * 1024;
const REGISTRY_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeHost {
    Codex,
    #[serde(rename = "claude_code")]
    Claude,
    #[serde(rename = "opencode")]
    OpenCode,
}

impl NativeHost {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude_code" | "claude" => Ok(Self::Claude),
            "opencode" => Ok(Self::OpenCode),
            _ => bail!("invalid_native_host"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude_code",
            Self::OpenCode => "opencode",
        }
    }

    fn directory(self) -> &'static str {
        match self {
            Self::Codex => ".agents/skills",
            Self::Claude => ".claude/skills",
            Self::OpenCode => ".opencode/skills",
        }
    }

    fn alias(self, id: Uuid) -> String {
        let host = if self == Self::Claude {
            "claude"
        } else {
            self.as_str()
        };
        format!("recollect-{host}-{}", id.simple())
    }
}

#[derive(Clone, Copy)]
pub struct NativeScope<'a> {
    pub root: &'a Path,
    pub config: &'a Config,
    pub workspace: &'a Path,
    pub host: NativeHost,
    pub environment: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NativeIssue {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct NativeEntry {
    pub id: Uuid,
    pub name: String,
    pub native_name: String,
    pub path: PathBuf,
    pub version: i32,
    pub host: NativeHost,
    pub environment_id: Option<Uuid>,
    pub brain_id: Uuid,
    pub persistent: bool,
    pub tasks: Vec<Uuid>,
}

#[derive(Default, Debug, Serialize)]
pub struct NativeReport {
    pub state: String,
    pub files_changed: Vec<PathBuf>,
    pub files_removed: Vec<PathBuf>,
    pub kept: Vec<NativeIssue>,
    pub errors: Vec<NativeIssue>,
    pub entries: Vec<NativeEntry>,
    pub native_loaded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeKey {
    endpoint: String,
    brain: Uuid,
    environment: Option<Uuid>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskRef {
    device: Uuid,
    host: NativeHost,
    task: Uuid,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    workspace: PathBuf,
    root: PathBuf,
    scope: ScopeKey,
    device: Uuid,
    host: NativeHost,
    id: Uuid,
    name: String,
    native_name: String,
    version: i32,
    applicability: Option<Uuid>,
    files: Vec<ManagedSkillFile>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    next: Uuid,
    stage: Uuid,
    backup: Uuid,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owned {
    id: Uuid,
    host: NativeHost,
    receipt: Option<Uuid>,
    pending: Option<Pending>,
    tasks: Vec<TaskRef>,
    pins: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    workspace: PathBuf,
    scope: Option<ScopeKey>,
    leases: Vec<TaskRef>,
    entries: Vec<Owned>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fence {
    workspace: PathBuf,
    scope: ScopeKey,
    device: Uuid,
    task: Uuid,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskIntent {
    workspace: PathBuf,
    scope: ScopeKey,
    device: Uuid,
    task: Uuid,
    scope_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    // Written before releasing prior refs. A retry must finish this cleanup;
    // checked exports cannot bypass an interrupted transition.
    release_pending: bool,
    #[serde(default)]
    release_targets: Vec<(NativeHost, Uuid)>,
}

impl NativeScope<'_> {
    fn key(&self) -> ScopeKey {
        ScopeKey {
            endpoint: self.config.endpoint.clone(),
            brain: self.config.brain,
            environment: self.environment,
        }
    }

    fn guard(&self) -> PathBuf {
        self.workspace.join(".recollect/managed-skills-native")
    }

    fn task_ref(&self, task: Uuid) -> TaskRef {
        TaskRef {
            device: self.config.device,
            host: self.host,
            task,
        }
    }

    fn target(&self, host: NativeHost, id: Uuid) -> PathBuf {
        self.workspace.join(host.directory()).join(host.alias(id))
    }

    fn fence(&self, task: Uuid) -> PathBuf {
        self.guard()
            .join("ended")
            .join(self.config.device.to_string())
            .join(format!("{task}.json"))
    }

    fn intent(&self, task: Uuid) -> PathBuf {
        self.guard()
            .join("tasks")
            .join(format!("{}-{task}.json", self.config.device))
    }
}

/// Validate every ancestor without changing permissions on existing workspace
/// directories. Only newly created directories are private.
fn safe_directory(path: &Path, create: bool) -> Result<()> {
    ensure!(path.is_absolute(), "invalid_native_directory");
    let mut at = PathBuf::new();
    for part in path.components() {
        ensure!(
            matches!(
                part,
                Component::RootDir | Component::Prefix(_) | Component::Normal(_)
            ),
            "invalid_native_directory"
        );
        at.push(part);
        match fs::symlink_metadata(&at) {
            Ok(meta) => ensure!(
                meta.is_dir() && !meta.is_symlink(),
                "unsafe_native_directory"
            ),
            Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(0o700);
                }
                match builder.create(&at) {
                    Ok(()) => (),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        let meta = fs::symlink_metadata(&at)?;
                        ensure!(
                            meta.is_dir() && !meta.is_symlink(),
                            "unsafe_native_directory"
                        );
                    }
                    Err(_) => bail!("native_directory_unavailable"),
                }
            }
            Err(_) => bail!("native_directory_unavailable"),
        }
    }
    Ok(())
}

fn present(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => bail!("native_path_unavailable"),
    }
}

fn read_json<T: DeserializeOwned>(path: &Path, limit: usize) -> Result<T> {
    safe_directory(
        path.parent()
            .ok_or_else(|| anyhow!("invalid_native_state"))?,
        false,
    )?;
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        meta.is_file() && !meta.is_symlink() && meta.len() <= limit as u64,
        "invalid_native_state"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= limit, "invalid_native_state");
    serde_json::from_slice(&bytes).map_err(|_| anyhow!("invalid_native_state"))
}

fn store_registry(scope: &NativeScope<'_>, registry: &Registry) -> Result<()> {
    ensure!(
        registry.entries.len() <= 192 && registry.leases.len() <= 4096,
        "native_registry_limit"
    );
    ensure!(
        serde_json::to_vec(registry)?.len() <= REGISTRY_LIMIT,
        "native_registry_limit"
    );
    plugin_storage::write(&scope.guard().join("registry.json"), registry)
}

fn lock_registry(scope: &NativeScope<'_>) -> Result<(fs::File, Registry)> {
    safe_directory(scope.workspace, false)?;
    ensure!(
        scope.workspace.parent().is_some(),
        "invalid_native_workspace"
    );
    ensure!(
        scope.workspace.canonicalize()? == scope.workspace,
        "noncanonical_native_workspace"
    );
    safe_directory(&scope.guard(), true)?;
    plugin_storage::directory(&scope.guard())?;
    let lock = plugin_storage::private_file(&scope.guard().join("registry.lock"), false)?;
    lock.lock()?;
    let path = scope.guard().join("registry.json");
    let registry = if present(&path)? {
        let registry: Registry = read_json(&path, REGISTRY_LIMIT)?;
        ensure!(
            registry.workspace == scope.workspace
                && registry.entries.len() <= 192
                && registry.leases.len() <= 4096,
            "native_registry_mismatch"
        );
        let mut ids = BTreeSet::new();
        for item in &registry.entries {
            ensure!(
                ids.insert((item.host.as_str(), item.id)),
                "native_registry_mismatch"
            );
        }
        registry
    } else {
        Registry {
            workspace: scope.workspace.to_owned(),
            scope: None,
            leases: vec![],
            entries: vec![],
        }
    };
    Ok((lock, registry))
}

fn receipt_path(scope: &NativeScope<'_>, id: Uuid) -> PathBuf {
    scope.guard().join("receipts").join(format!("{id}.json"))
}

fn read_receipt(
    scope: &NativeScope<'_>,
    id: Uuid,
    owned: &Owned,
    key: &ScopeKey,
) -> Result<Receipt> {
    let value: Receipt = read_json(&receipt_path(scope, id), RECEIPT_LIMIT)?;
    ensure!(
        value.workspace == scope.workspace
            && value.scope == *key
            && value.host == owned.host
            && value.id == owned.id
            && value.native_name == owned.host.alias(owned.id)
            && value.version > 0,
        "native_receipt_mismatch"
    );
    ensure!(
        value.root.is_absolute() && !value.device.is_nil() && value.files.len() <= 65,
        "native_receipt_mismatch"
    );
    let paths: BTreeSet<_> = value.files.iter().map(|f| f.path.as_str()).collect();
    ensure!(
        paths.len() == value.files.len()
            && paths.contains("SKILL.md")
            && value
                .files
                .iter()
                .all(|f| recollect_protocol::validate_managed_skill_path(&f.path)
                    && f.content.len() <= 132 * 1024),
        "native_receipt_mismatch"
    );
    Ok(value)
}

fn expected_names(files: &[ManagedSkillFile]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for file in files {
        names.insert(file.path.clone());
        for parent in Path::new(&file.path)
            .ancestors()
            .skip(1)
            .filter(|p| !p.as_os_str().is_empty())
        {
            names.insert(format!("{}/", parent.to_string_lossy()));
        }
    }
    names
}

fn unchanged(path: &Path, files: &[ManagedSkillFile]) -> Result<()> {
    safe_directory(path, false)?;
    fn walk(root: &Path, path: &Path, names: &mut BTreeSet<String>) -> Result<()> {
        ensure!(
            path.strip_prefix(root)?.components().count() <= 9 && names.len() <= 1024,
            "native_local_changes"
        );
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let meta = fs::symlink_metadata(entry.path())?;
            ensure!(!meta.is_symlink(), "native_local_changes");
            let name = entry
                .path()
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            if meta.is_dir() {
                names.insert(format!("{name}/"));
                walk(root, &entry.path(), names)?;
            } else {
                ensure!(
                    meta.is_file() && meta.len() <= RECEIPT_LIMIT as u64,
                    "native_local_changes"
                );
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    ensure!(
                        meta.permissions().mode() & 0o111 == 0,
                        "native_local_changes"
                    );
                }
                names.insert(name);
            }
        }
        Ok(())
    }
    let mut names = BTreeSet::new();
    walk(path, path, &mut names)?;
    ensure!(names == expected_names(files), "native_local_changes");
    for file in files {
        ensure!(
            fs::read(path.join(&file.path))? == file.content.as_bytes(),
            "native_local_changes"
        );
    }
    Ok(())
}

fn render(
    scope: &NativeScope<'_>,
    host: NativeHost,
    bundle: &ManagedSkillBundle,
) -> Result<Receipt> {
    ensure!(
        scope.root.is_absolute() && !scope.config.device.is_nil(),
        "invalid_native_scope"
    );
    let _ = crate::Client::new(&scope.config.endpoint)?;
    validate_bundle(bundle, scope.config)?;
    ensure!(
        bundle.skill.environment_id.is_none() || bundle.skill.environment_id == scope.environment,
        "managed_guidance_scope_mismatch"
    );
    let entry = bundle
        .files
        .iter()
        .find(|f| f.path == bundle.skill.kind.entrypoint())
        .ok_or_else(|| anyhow!("invalid_managed_guidance_bundle"))?;
    let body = if bundle.skill.kind == ManagedSkillKind::Skill {
        let normalized = entry.content.replace("\r\n", "\n");
        let rest = normalized
            .strip_prefix("---\n")
            .ok_or_else(|| anyhow!("invalid_managed_guidance_bundle"))?;
        rest.split_once("\n---\n")
            .map(|(_, body)| body.to_owned())
            .or_else(|| rest.strip_suffix("\n---").map(|_| String::new()))
            .ok_or_else(|| anyhow!("invalid_managed_guidance_bundle"))?
    } else {
        "Read the source instruction resource [AGENTS.md](AGENTS.md) after the current scoped canonical check below.".into()
    };
    let name = host.alias(bundle.skill.id);
    // JSON strings are valid YAML double-quoted scalars; authored descriptions
    // cannot create permission or hook frontmatter fields.
    let header = format!(
        "---\nname: {}\ndescription: {}\n---\n",
        serde_json::to_string(&name)?,
        serde_json::to_string(&bundle.skill.description)?
    );
    let provenance = serde_json::json!({"endpoint":scope.config.endpoint,"brain_id":scope.config.brain,"environment_id":scope.environment,"bundle_environment_id":bundle.skill.environment_id,"bundle_id":bundle.skill.id,"version":bundle.skill.version,"source_name":bundle.skill.name});
    let content = format!(
        "{header}\nRecollect manager-published guidance, last fetched export. Provenance: `{provenance}`.\n\nBefore following this body or its references, call current scoped `skills.fetch` with this bundle ID and the current read operation. Verify the operation's endpoint, Brain and environment match this export. Use current guidance if its version changed; stop this guidance if unavailable, disabled or scope-mismatched. Human, host and applicable local repository instructions retain priority. This guidance grants no tool permissions.\n\n{body}"
    );
    // Instructions-kind metadata has no source SKILL frontmatter to validate.
    // Recheck the complete generated entrypoint as well as every source file:
    // Claude's dynamic-context syntax must never be introduced by adaptation.
    ensure!(!content.contains("!`"), "invalid_native_entrypoint");
    let mut files = bundle.files.clone();
    if bundle.skill.kind == ManagedSkillKind::Skill {
        files.retain(|f| f.path != "SKILL.md");
    } else {
        ensure!(
            !files.iter().any(|f| f.path.eq_ignore_ascii_case("SKILL.md")
                || f.path.to_lowercase().starts_with("skill.md/")),
            "native_wrapper_path_conflict"
        );
    }
    files.push(ManagedSkillFile {
        path: "SKILL.md".into(),
        content,
    });
    let value = Receipt {
        workspace: scope.workspace.to_owned(),
        root: scope.root.to_owned(),
        scope: scope.key(),
        device: scope.config.device,
        host,
        id: bundle.skill.id,
        name: bundle.skill.name.clone(),
        native_name: name,
        version: bundle.skill.version,
        applicability: bundle.skill.environment_id,
        files,
    };
    ensure!(
        serde_json::to_vec(&value)?.len() <= RECEIPT_LIMIT,
        "native_receipt_limit"
    );
    Ok(value)
}

fn write_stage(scope: &NativeScope<'_>, id: Uuid, files: &[ManagedSkillFile]) -> Result<PathBuf> {
    let path = scope.guard().join("staging").join(id.to_string());
    ensure!(!present(&path)?, "native_stage_conflict");
    plugin_storage::directory(&path)?;
    for file in files {
        let target = path.join(&file.path);
        plugin_storage::directory(
            target
                .parent()
                .ok_or_else(|| anyhow!("invalid_native_path"))?,
        )?;
        let mut output = plugin_storage::private_file(&target, true)?;
        output.write_all(file.content.as_bytes())?;
        output.sync_all()?;
    }
    unchanged(&path, files)?;
    fs::File::open(&path)?.sync_all()?;
    Ok(path)
}

/// A committed pending receipt makes the two directory renames recoverable.
/// Unknown/modified stages and backups are left for explicit inspection.
fn recover(scope: &NativeScope<'_>, registry: &mut Registry) -> Result<()> {
    let Some(key) = registry.scope.clone() else {
        return Ok(());
    };
    for index in 0..registry.entries.len() {
        let owned = registry.entries[index].clone();
        let Some(pending) = owned.pending.clone() else {
            continue;
        };
        let next = read_receipt(scope, pending.next, &owned, &key)?;
        let old = owned
            .receipt
            .map(|id| read_receipt(scope, id, &owned, &key))
            .transpose()?;
        let target = scope.target(owned.host, owned.id);
        let stage = scope
            .guard()
            .join("staging")
            .join(pending.stage.to_string());
        let backup = scope
            .guard()
            .join("staging")
            .join(pending.backup.to_string());
        if present(&target)? && unchanged(&target, &next.files).is_ok() {
            registry.entries[index].receipt = Some(pending.next);
            registry.entries[index].pending = None;
            store_registry(scope, registry)?;
            if let Some(old) = &old {
                if present(&backup)? && unchanged(&backup, &old.files).is_ok() {
                    fs::remove_dir_all(&backup)?;
                }
            }
            continue;
        }
        if present(&target)? {
            let old = old
                .as_ref()
                .ok_or_else(|| anyhow!("native_recovery_conflict"))?;
            unchanged(&target, &old.files)?;
            ensure!(!present(&backup)?, "native_recovery_conflict");
            unchanged(&stage, &next.files)?;
            fs::rename(&target, &backup)?;
        } else if let Some(old) = &old {
            // Old export may have been moved just before interruption.
            ensure!(present(&backup)?, "native_recovery_conflict");
            unchanged(&backup, &old.files)?;
        }
        unchanged(&stage, &next.files)?;
        safe_directory(
            target
                .parent()
                .ok_or_else(|| anyhow!("invalid_native_path"))?,
            true,
        )?;
        fs::rename(&stage, &target)?;
        fs::File::open(target.parent().unwrap())?.sync_all()?;
        registry.entries[index].receipt = Some(pending.next);
        registry.entries[index].pending = None;
        store_registry(scope, registry)?;
        if let Some(old) = &old {
            if present(&backup)? && unchanged(&backup, &old.files).is_ok() {
                fs::remove_dir_all(&backup)?;
            }
        }
    }
    Ok(())
}

fn remove_entry(
    scope: &NativeScope<'_>,
    registry: &mut Registry,
    index: usize,
    report: &mut NativeReport,
) -> Result<()> {
    let owned = &registry.entries[index];
    let key = registry
        .scope
        .as_ref()
        .ok_or_else(|| anyhow!("native_registry_mismatch"))?;
    let receipt = read_receipt(
        scope,
        owned
            .receipt
            .ok_or_else(|| anyhow!("native_recovery_required"))?,
        owned,
        key,
    )?;
    let path = scope.target(owned.host, owned.id);
    if present(&path)? {
        unchanged(&path, &receipt.files)?;
        fs::remove_dir_all(&path)?;
        report.files_removed.push(path.join("SKILL.md"));
    }
    registry.entries.remove(index);
    store_registry(scope, registry)
}

fn finish(
    scope: &NativeScope<'_>,
    registry: &Registry,
    mut report: NativeReport,
) -> Result<NativeReport> {
    if registry.scope.as_ref() == Some(&scope.key()) {
        for owned in registry.entries.iter().filter(|e| e.host == scope.host) {
            let Some(id) = owned.receipt else {
                continue;
            };
            let receipt = read_receipt(scope, id, owned, &scope.key())?;
            let path = scope.target(owned.host, owned.id).join("SKILL.md");
            if !present(&path)? {
                continue;
            }
            if let Err(error) = unchanged(path.parent().unwrap(), &receipt.files) {
                if !report
                    .errors
                    .iter()
                    .any(|issue| issue.path == path.parent().unwrap())
                {
                    report.errors.push(NativeIssue {
                        path: path.parent().unwrap().to_owned(),
                        reason: error.to_string(),
                    });
                }
                continue;
            }
            report.entries.push(NativeEntry {
                id: owned.id,
                name: receipt.name,
                native_name: receipt.native_name,
                path,
                version: receipt.version,
                host: owned.host,
                environment_id: receipt.applicability,
                brain_id: receipt.scope.brain,
                persistent: !owned.pins.is_empty(),
                tasks: owned.tasks.iter().map(|t| t.task).collect(),
            });
        }
    }
    report.state = if !report.errors.is_empty() {
        "partial"
    } else if !report.files_changed.is_empty() {
        "placed"
    } else if !report.files_removed.is_empty() {
        "removed"
    } else {
        "unchanged"
    }
    .into();
    Ok(report)
}

fn begin_scope(
    scope: &NativeScope<'_>,
    registry: &mut Registry,
    report: &mut NativeReport,
) -> Result<()> {
    if registry
        .scope
        .as_ref()
        .is_some_and(|key| key != &scope.key())
    {
        ensure!(registry.leases.is_empty(), "native_scope_busy");
        // Preflight the complete previous scope before removing anything.
        for owned in &registry.entries {
            ensure!(owned.pending.is_none(), "native_recovery_required");
            let prior = read_receipt(
                scope,
                owned
                    .receipt
                    .ok_or_else(|| anyhow!("native_recovery_required"))?,
                owned,
                registry.scope.as_ref().unwrap(),
            )?;
            let target = scope.target(owned.host, owned.id);
            if present(&target)? {
                unchanged(&target, &prior.files)?;
            }
        }
        while !registry.entries.is_empty() {
            remove_entry(scope, registry, registry.entries.len() - 1, report)?;
        }
    }
    registry.scope = Some(scope.key());
    Ok(())
}

fn apply(
    scope: &NativeScope<'_>,
    task: Uuid,
    bundles: &[ManagedSkillBundle],
    persistent: bool,
    reconcile: bool,
    expected_scope: Option<Uuid>,
) -> Result<NativeReport> {
    ensure!(
        !task.is_nil() && bundles.len() <= 64,
        "invalid_native_install"
    );
    let mut ids = BTreeSet::new();
    for bundle in bundles {
        ensure!(ids.insert(bundle.skill.id), "duplicate_native_bundle");
        let _ = render(scope, scope.host, bundle)?;
    }
    let (_lock, mut registry) = lock_registry(scope)?;
    if let Some(scope_id) = expected_scope {
        let intent = checked_intent(scope, task, scope_id)?;
        ensure!(
            !intent.release_pending,
            "native_scope_reconciliation_required"
        );
    }
    // Check incompatible scope before recovery: even a pending prior transaction
    // must not permit a conflicting session to write the native roots.
    if registry
        .scope
        .as_ref()
        .is_some_and(|key| key != &scope.key())
        && !registry.leases.is_empty()
    {
        bail!("native_scope_busy");
    }
    ensure!(!present(&scope.fence(task))?, "native_session_ended");
    recover(scope, &mut registry)?;
    let mut report = NativeReport::default();
    begin_scope(scope, &mut registry, &mut report)?;
    let task_ref = scope.task_ref(task);
    if !persistent && !registry.leases.contains(&task_ref) {
        registry.leases.push(task_ref.clone());
    }
    store_registry(scope, &registry)?;

    if reconcile {
        for index in (0..registry.entries.len()).rev() {
            let entry = &registry.entries[index];
            if ids.contains(&entry.id) {
                continue;
            }
            let path = scope.target(entry.host, entry.id);
            // The authoritative complete applicable catalog has withdrawn this
            // export. Local edits still take precedence over automatic deletion.
            if let Err(error) = remove_entry(scope, &mut registry, index, &mut report) {
                report.errors.push(NativeIssue {
                    path,
                    reason: error.to_string(),
                });
            }
        }
    }

    let mut work = Vec::new();
    for bundle in bundles {
        work.push((scope.host, bundle));
        if reconcile {
            for owned in registry
                .entries
                .iter()
                .filter(|e| e.id == bundle.skill.id && e.host != scope.host)
            {
                work.push((owned.host, bundle));
            }
        }
    }
    for (host, bundle) in work {
        let result = (|| -> Result<()> {
            let receipt = render(scope, host, bundle)?;
            let target = scope.target(host, bundle.skill.id);
            let existing = registry
                .entries
                .iter()
                .position(|entry| entry.id == bundle.skill.id && entry.host == host);
            let mut owned = existing
                .map(|i| registry.entries[i].clone())
                .unwrap_or(Owned {
                    id: bundle.skill.id,
                    host,
                    receipt: None,
                    pending: None,
                    tasks: vec![],
                    pins: vec![],
                });
            if let Some(prior_id) = owned.receipt {
                let prior = read_receipt(scope, prior_id, &owned, &scope.key())?;
                unchanged(&target, &prior.files)?;
                ensure!(prior.version <= receipt.version, "native_stale_version");
                if prior.version == receipt.version {
                    ensure!(
                        prior.files == receipt.files
                            && prior.name == receipt.name
                            && prior.applicability == receipt.applicability,
                        "native_version_conflict"
                    );
                    if host == scope.host && !persistent && !owned.tasks.contains(&task_ref) {
                        owned.tasks.push(task_ref.clone());
                    }
                    if host == scope.host
                        && persistent
                        && !owned.pins.contains(&scope.config.device)
                    {
                        owned.pins.push(scope.config.device);
                    }
                    registry.entries[existing.unwrap()] = owned;
                    store_registry(scope, &registry)?;
                    return Ok(());
                }
            } else {
                ensure!(!present(&target)?, "native_unowned_target");
            }
            safe_directory(
                target
                    .parent()
                    .ok_or_else(|| anyhow!("invalid_native_path"))?,
                true,
            )?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                ensure!(
                    fs::metadata(scope.guard())?.dev()
                        == fs::metadata(target.parent().unwrap())?.dev(),
                    "native_filesystem_mismatch"
                );
            }
            let receipt_id = Uuid::new_v4();
            let pending = Pending {
                next: receipt_id,
                stage: Uuid::new_v4(),
                backup: Uuid::new_v4(),
            };
            write_stage(scope, pending.stage, &receipt.files)?;
            plugin_storage::write(&receipt_path(scope, receipt_id), &receipt)?;
            owned.pending = Some(pending);
            if host == scope.host && !persistent && !owned.tasks.contains(&task_ref) {
                owned.tasks.push(task_ref.clone());
            }
            if host == scope.host && persistent && !owned.pins.contains(&scope.config.device) {
                owned.pins.push(scope.config.device);
            }
            if let Some(index) = existing {
                registry.entries[index] = owned;
            } else {
                registry.entries.push(owned);
            }
            store_registry(scope, &registry)?;
            recover(scope, &mut registry)?;
            report.files_changed.push(target.join("SKILL.md"));
            Ok(())
        })();
        if let Err(error) = result {
            report.errors.push(NativeIssue {
                path: scope.target(host, bundle.skill.id),
                reason: error.to_string(),
            });
        }
    }
    finish(scope, &registry, report)
}

pub fn install(
    scope: &NativeScope<'_>,
    task: Uuid,
    bundles: &[ManagedSkillBundle],
    persistent: bool,
) -> Result<NativeReport> {
    apply(scope, task, bundles, persistent, false, None)
}

pub fn sync(
    scope: &NativeScope<'_>,
    task: Uuid,
    bundles: &[ManagedSkillBundle],
    persistent: bool,
) -> Result<NativeReport> {
    apply(scope, task, bundles, persistent, true, None)
}

pub fn install_scoped(
    scope: &NativeScope<'_>,
    task: Uuid,
    scope_id: Uuid,
    bundles: &[ManagedSkillBundle],
    persistent: bool,
) -> Result<NativeReport> {
    apply(scope, task, bundles, persistent, false, Some(scope_id))
}

pub fn sync_scoped(
    scope: &NativeScope<'_>,
    task: Uuid,
    scope_id: Uuid,
    bundles: &[ManagedSkillBundle],
    persistent: bool,
) -> Result<NativeReport> {
    apply(scope, task, bundles, persistent, true, Some(scope_id))
}

fn read_intent(scope: &NativeScope<'_>, task: Uuid) -> Result<TaskIntent> {
    let intent: TaskIntent = read_json(&scope.intent(task), 65536)?;
    ensure!(
        intent.workspace == scope.workspace
            && intent.device == scope.config.device
            && intent.task == task
            && !intent.scope_id.is_nil()
            && !intent.task.is_nil()
            && !intent.scope.brain.is_nil(),
        "native_scope_intent_mismatch"
    );
    let mut targets = BTreeSet::new();
    ensure!(
        intent.release_targets.len() <= 192
            && intent
                .release_targets
                .iter()
                .all(|(host, id)| !id.is_nil() && targets.insert((host.as_str(), *id))),
        "native_scope_intent_mismatch"
    );
    Ok(intent)
}

fn checked_intent(scope: &NativeScope<'_>, task: Uuid, scope_id: Uuid) -> Result<TaskIntent> {
    ensure!(
        !scope_id.is_nil() && present(&scope.intent(task))?,
        "native_scope_changed"
    );
    let intent = read_intent(scope, task)?;
    ensure!(
        intent.scope_id == scope_id && intent.scope == scope.key(),
        "native_scope_changed"
    );
    Ok(intent)
}

// The caller holds the workspace lock. This intentionally does not recover a
// pending export transaction from the withdrawn generation. Such a conflict
// keeps the new intent fenced until explicit conservative recovery.
fn release_task_refs_locked(
    scope: &NativeScope<'_>,
    registry: &mut Registry,
    task: Uuid,
    targets: &[(NativeHost, Uuid)],
) -> Result<NativeReport> {
    let mut report = NativeReport::default();
    for index in (0..registry.entries.len()).rev() {
        let owned = &mut registry.entries[index];
        if !targets.contains(&(owned.host, owned.id)) {
            continue;
        }
        owned
            .tasks
            .retain(|r| !(r.device == scope.config.device && r.task == task));
        if !owned.tasks.is_empty() || !owned.pins.is_empty() {
            continue;
        }
        let path = scope.target(owned.host, owned.id);
        if let Err(error) = remove_entry(scope, registry, index, &mut report) {
            report.errors.push(NativeIssue {
                path,
                reason: error.to_string(),
            });
        }
    }
    registry
        .leases
        .retain(|r| !(r.device == scope.config.device && r.task == task));
    store_registry(scope, registry)?;
    Ok(report)
}

/// Publish the server-observed live task generation before any network fetch.
/// Timestamp ordering rejects delayed hook observations; the same generation
/// resumes interrupted prior-ref cleanup without ending the live task.
pub fn observe_task_scope(
    scope: &NativeScope<'_>,
    task: Uuid,
    scope_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Result<NativeReport> {
    ensure!(
        !task.is_nil() && !scope_id.is_nil(),
        "invalid_native_scope_intent"
    );
    let (_lock, mut registry) = lock_registry(scope)?;
    ensure!(!present(&scope.fence(task))?, "native_session_ended");
    let prior = if present(&scope.intent(task))? {
        Some(read_intent(scope, task)?)
    } else {
        None
    };
    let changed = prior
        .as_ref()
        .is_none_or(|prior| prior.scope_id != scope_id);
    if let Some(prior) = &prior {
        if prior.scope_id == scope_id {
            ensure!(
                prior.scope == scope.key() && prior.created_at == created_at,
                "native_scope_intent_mismatch"
            );
        } else {
            ensure!(created_at > prior.created_at, "native_scope_stale");
        }
    }
    let cleanup = changed
        || prior.as_ref().is_some_and(|prior| prior.release_pending)
        || registry
            .scope
            .as_ref()
            .is_some_and(|key| key != &scope.key());
    let mut intent = TaskIntent {
        workspace: scope.workspace.to_owned(),
        scope: scope.key(),
        device: scope.config.device,
        task,
        scope_id,
        created_at,
        release_pending: cleanup,
        release_targets: registry
            .entries
            .iter()
            .filter(|owned| {
                owned
                    .tasks
                    .iter()
                    .any(|r| r.device == scope.config.device && r.task == task)
            })
            .map(|owned| (owned.host, owned.id))
            .collect(),
    };
    if let Some(prior) = &prior {
        if prior.release_pending {
            for target in &prior.release_targets {
                if !intent.release_targets.contains(target) {
                    intent.release_targets.push(*target);
                }
            }
        }
    }
    plugin_storage::write(&scope.intent(task), &intent)?;
    let report = if cleanup {
        release_task_refs_locked(scope, &mut registry, task, &intent.release_targets)?
    } else {
        NativeReport::default()
    };
    intent.release_pending = !report.errors.is_empty();
    plugin_storage::write(&scope.intent(task), &intent)?;
    finish(scope, &registry, report)
}

/// A delayed denial may release only the task generation it actually checked.
pub fn release_scope_task_scoped(
    scope: &NativeScope<'_>,
    task: Uuid,
    scope_id: Uuid,
) -> Result<NativeReport> {
    let (_lock, mut registry) = lock_registry(scope)?;
    let mut intent = checked_intent(scope, task, scope_id)?;
    for owned in &registry.entries {
        let target = (owned.host, owned.id);
        if owned
            .tasks
            .iter()
            .any(|r| r.device == scope.config.device && r.task == task)
            && !intent.release_targets.contains(&target)
        {
            intent.release_targets.push(target);
        }
    }
    intent.release_pending = true;
    plugin_storage::write(&scope.intent(task), &intent)?;
    let report = release_task_refs_locked(scope, &mut registry, task, &intent.release_targets)?;
    intent.release_pending = !report.errors.is_empty();
    plugin_storage::write(&scope.intent(task), &intent)?;
    finish(scope, &registry, report)
}

/// Snapshot publication shares the generation guard with filesystem apply.
/// Ended tasks may publish their final withdrawal; this conveys no new lease.
pub fn publish_scoped(
    scope: &NativeScope<'_>,
    task: Uuid,
    scope_id: Uuid,
    path: &Path,
    value: &serde_json::Value,
) -> Result<()> {
    let (_lock, _) = lock_registry(scope)?;
    let intent = read_intent(scope, task)?;
    ensure!(
        intent.scope_id == scope_id
            && intent.scope.endpoint == scope.config.endpoint
            && intent.scope.brain == scope.config.brain,
        "native_scope_changed"
    );
    let filename = format!("{}-{task}.json", scope.config.device);
    let recovery = scope.root.join("native-skills/tasks").join(&filename);
    ensure!(
        path == scope.root.join("native-skills/catalogues").join(&filename) || path == recovery,
        "native_snapshot_path_mismatch"
    );
    if path == recovery {
        ensure!(
            value["endpoint"] == serde_json::json!(scope.config.endpoint)
                && value["device_id"] == serde_json::json!(scope.config.device),
            "native_snapshot_scope_mismatch"
        );
    }
    ensure!(
        value["scope_id"] == serde_json::json!(scope_id)
            && value["task_id"] == serde_json::json!(task)
            && value["brain_id"] == serde_json::json!(scope.config.brain)
            && value["workspace"] == serde_json::json!(scope.workspace)
            && serde_json::to_vec(value)?.len() <= REGISTRY_LIMIT,
        "native_snapshot_scope_mismatch"
    );
    plugin_storage::write(path, value)
}

pub fn status(scope: &NativeScope<'_>) -> Result<NativeReport> {
    let (_lock, registry) = lock_registry(scope)?;
    let environment = matching_environment(scope, &registry)?;
    let scoped = NativeScope {
        environment,
        ..*scope
    };
    finish(&scoped, &registry, NativeReport::default())
}

pub fn leases(scope: &NativeScope<'_>) -> Result<serde_json::Value> {
    let (_lock, registry) = lock_registry(scope)?;
    Ok(
        serde_json::json!({"workspace":registry.workspace,"scope":registry.scope,"leases":registry.leases}),
    )
}

fn release(
    scope: &NativeScope<'_>,
    id: Option<Uuid>,
    task: Option<Uuid>,
    end: bool,
    across_hosts: bool,
) -> Result<NativeReport> {
    let (_lock, mut registry) = lock_registry(scope)?;
    // A late end from a previously selected Brain must still fence its task,
    // while leaving the new Brain's registry/exports entirely untouched.
    let environment = if end {
        matching_environment(scope, &registry).unwrap_or(scope.environment)
    } else {
        matching_environment(scope, &registry)?
    };
    let scoped = NativeScope {
        environment,
        ..*scope
    };
    let scope = &scoped;
    let mut report = NativeReport::default();
    if end {
        let task = task.ok_or_else(|| anyhow!("invalid_native_task"))?;
        let path = scope.fence(task);
        if !present(&path)? {
            plugin_storage::write(
                &path,
                &Fence {
                    workspace: scope.workspace.to_owned(),
                    scope: scope.key(),
                    device: scope.config.device,
                    task,
                },
            )?;
        } else {
            let prior: Fence = read_json(&path, REGISTRY_LIMIT)?;
            ensure!(
                prior.workspace == scope.workspace
                    && prior.device == scope.config.device
                    && prior.task == task,
                "native_fence_mismatch"
            );
        }
    }
    if registry.scope.as_ref() != Some(&scope.key()) {
        return finish(scope, &registry, report);
    }
    recover(scope, &mut registry)?;
    for index in (0..registry.entries.len()).rev() {
        let owned = &mut registry.entries[index];
        if id.is_some_and(|id| id != owned.id) || (!across_hosts && owned.host != scope.host) {
            continue;
        }
        if let Some(task) = task {
            owned.tasks.retain(|r| {
                !(r.device == scope.config.device
                    && r.task == task
                    && (across_hosts || r.host == scope.host))
            });
        } else {
            owned.pins.retain(|device| device != &scope.config.device);
        }
        if !owned.tasks.is_empty() || !owned.pins.is_empty() {
            report.kept.push(NativeIssue {
                path: scope.target(owned.host, owned.id),
                reason: "active_native_references".into(),
            });
            continue;
        }
        let host = owned.host;
        let id = owned.id;
        if let Err(error) = remove_entry(scope, &mut registry, index, &mut report) {
            report.errors.push(NativeIssue {
                path: scope.target(host, id),
                reason: error.to_string(),
            });
        }
    }
    if let Some(task) = task {
        if across_hosts {
            registry
                .leases
                .retain(|r| !(r.device == scope.config.device && r.task == task));
        } else {
            let still_referenced = registry.entries.iter().any(|e| {
                e.tasks.iter().any(|r| {
                    r.device == scope.config.device && r.task == task && r.host == scope.host
                })
            });
            if !still_referenced {
                registry.leases.retain(|r| {
                    !(r.device == scope.config.device && r.task == task && r.host == scope.host)
                });
            }
        }
    }
    store_registry(scope, &registry)?;
    finish(scope, &registry, report)
}

/// Closing any host session fences that device/task across every host, and only
/// releases its own references. Retained pins and other live tasks survive.
pub fn release_task(scope: &NativeScope<'_>, task: Uuid) -> Result<NativeReport> {
    release(scope, None, Some(task), true, true)
}

/// Some(task) unregisters that task's selected host references. None removes
/// the current device's persistent intent, preserving all active references.
pub fn uninstall(
    scope: &NativeScope<'_>,
    id: Option<Uuid>,
    task: Option<Uuid>,
) -> Result<NativeReport> {
    release(scope, id, task, false, false)
}

/// A live task changing scope releases its old references without writing an
/// ended-task tombstone. This never releases another task's scope lease.
pub fn release_scope_task(scope: &NativeScope<'_>, task: Uuid) -> Result<NativeReport> {
    release(scope, None, Some(task), false, true)
}

/// Explicit host refresh can retrigger native watchers by replacing only the
/// unchanged owned entrypoint with identical bytes. This requests a refresh;
/// discovery and activation still require evidence from the actual host.
pub fn request_refresh(scope: &NativeScope<'_>) -> Result<NativeReport> {
    let (_lock, mut registry) = lock_registry(scope)?;
    let environment = matching_environment(scope, &registry)?;
    let scoped = NativeScope {
        environment,
        ..*scope
    };
    let scope = &scoped;
    recover(scope, &mut registry)?;
    let mut report = NativeReport::default();
    for owned in registry
        .entries
        .iter()
        .filter(|entry| entry.host == scope.host)
    {
        let target = scope.target(owned.host, owned.id);
        let result = (|| -> Result<()> {
            let receipt = read_receipt(
                scope,
                owned
                    .receipt
                    .ok_or_else(|| anyhow!("native_recovery_required"))?,
                owned,
                &scope.key(),
            )?;
            unchanged(&target, &receipt.files)?;
            let file = receipt
                .files
                .iter()
                .find(|file| file.path == "SKILL.md")
                .ok_or_else(|| anyhow!("native_receipt_mismatch"))?;
            let stage = scope
                .guard()
                .join("staging")
                .join(Uuid::new_v4().to_string());
            plugin_storage::directory(&stage)?;
            let pending = stage.join("SKILL.md");
            let mut output = plugin_storage::private_file(&pending, true)?;
            output.write_all(file.content.as_bytes())?;
            output.sync_all()?;
            ensure!(
                fs::read(&pending)? == file.content.as_bytes(),
                "native_stage_conflict"
            );
            unchanged(&target, &receipt.files)?;
            fs::rename(&pending, target.join("SKILL.md"))?;
            fs::File::open(&target)?.sync_all()?;
            fs::remove_dir(&stage)?;
            report.files_changed.push(target.join("SKILL.md"));
            Ok(())
        })();
        if let Err(error) = result {
            report.errors.push(NativeIssue {
                path: target,
                reason: error.to_string(),
            });
        }
    }
    let mut report = finish(scope, &registry, report)?;
    if report.errors.is_empty() && !report.files_changed.is_empty() {
        report.state = "refresh_requested".into();
    }
    Ok(report)
}

fn matching_environment(scope: &NativeScope<'_>, registry: &Registry) -> Result<Option<Uuid>> {
    if let Some(key) = &registry.scope {
        ensure!(
            key.endpoint == scope.config.endpoint && key.brain == scope.config.brain,
            "native_scope_mismatch"
        );
        Ok(key.environment)
    } else {
        Ok(scope.environment)
    }
}

/// Confirmed current-device revocation retracts only that device's local intent.
/// Other authorized devices/tasks keep their references and scope leases.
pub fn revoke(scope: &NativeScope<'_>) -> Result<NativeReport> {
    let (_lock, mut registry) = lock_registry(scope)?;
    let environment = matching_environment(scope, &registry)?;
    let scoped = NativeScope {
        environment,
        ..*scope
    };
    let scope = &scoped;
    recover(scope, &mut registry)?;
    let mut report = NativeReport::default();
    for index in (0..registry.entries.len()).rev() {
        let owned = &mut registry.entries[index];
        owned.tasks.retain(|r| r.device != scope.config.device);
        owned.pins.retain(|device| device != &scope.config.device);
        if !owned.tasks.is_empty() || !owned.pins.is_empty() {
            continue;
        }
        let path = scope.target(owned.host, owned.id);
        if let Err(error) = remove_entry(scope, &mut registry, index, &mut report) {
            report.errors.push(NativeIssue {
                path,
                reason: error.to_string(),
            });
        }
    }
    registry.leases.retain(|r| r.device != scope.config.device);
    store_registry(scope, &registry)?;
    finish(scope, &registry, report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use recollect_protocol::ManagedSkillSummary;

    struct Fixture {
        base: PathBuf,
        root: PathBuf,
        workspace: PathBuf,
        config: Config,
        bundle: ManagedSkillBundle,
    }

    impl Fixture {
        fn new() -> Self {
            let base = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(".cache")
                .join(format!("native-skills-{}", Uuid::new_v4()));
            let root = base.join("plugin");
            let workspace = base.join("workspace");
            plugin_storage::directory(&root).unwrap();
            plugin_storage::directory(&workspace).unwrap();
            let config = Config {
                endpoint: "https://skills.example.test".into(),
                brain: Uuid::new_v4(),
                device: Uuid::new_v4(),
                profile: "fixture".into(),
                with_runner: false,
                runner_id: None,
            };
            let files = vec![ManagedSkillFile { path: "SKILL.md".into(), content: "---\nname: team-guide\ndescription: Apply the team technology guidance.\n---\n\nNative marker one. Read [reference](references/guide.md).".into() }, ManagedSkillFile { path: "references/guide.md".into(), content: "Reference marker one.".into() }];
            let bundle = ManagedSkillBundle {
                skill: ManagedSkillSummary {
                    id: Uuid::new_v4(),
                    brain_id: config.brain,
                    name: "team-guide".into(),
                    description: "Apply the team technology guidance.".into(),
                    kind: ManagedSkillKind::Skill,
                    version: 1,
                    environment_id: None,
                    profile_ids: vec![],
                    tags: vec!["rust".into()],
                    enabled: true,
                    file_paths: files.iter().map(|f| f.path.clone()).collect(),
                    updated_at: chrono::Utc::now(),
                },
                files,
            };
            Self {
                base,
                root,
                workspace,
                config,
                bundle,
            }
        }

        fn scope(&self, host: NativeHost, environment: Option<Uuid>) -> NativeScope<'_> {
            NativeScope {
                root: &self.root,
                config: &self.config,
                workspace: &self.workspace,
                host,
                environment,
            }
        }

        fn update(&self) -> ManagedSkillBundle {
            let mut bundle = self.bundle.clone();
            bundle.skill.version = 2;
            for file in &mut bundle.files {
                file.content = file.content.replace("marker one", "marker two");
            }
            bundle
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    #[test]
    fn all_native_roots_preserve_local_instructions_and_safe_relative_resources() {
        let f = Fixture::new();
        fs::write(f.workspace.join("AGENTS.md"), "Local AGENTS").unwrap();
        fs::write(f.workspace.join("CLAUDE.md"), "Local CLAUDE").unwrap();
        let task = Uuid::new_v4();
        for host in [NativeHost::Codex, NativeHost::Claude, NativeHost::OpenCode] {
            let scope = f.scope(host, None);
            let report = install(&scope, task, &[f.bundle.clone()], false).unwrap();
            assert!(report.errors.is_empty(), "{:?}", report.errors);
            assert!(!report.native_loaded);
            let entry = &report.entries[0];
            assert!(entry.native_name.len() <= 64);
            assert_eq!(
                entry.path,
                scope.target(host, f.bundle.skill.id).join("SKILL.md")
            );
            let body = fs::read_to_string(&entry.path).unwrap();
            assert!(body.contains("Native marker one."));
            assert!(body.contains("current scoped `skills.fetch`"));
            assert!(body.contains(&f.config.brain.to_string()));
            assert!(body.contains(&format!("name: \"{}\"", entry.native_name)));
            assert_eq!(
                fs::read_to_string(entry.path.parent().unwrap().join("references/guide.md"))
                    .unwrap(),
                "Reference marker one."
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    fs::metadata(&entry.path).unwrap().permissions().mode() & 0o777,
                    0o600
                );
                assert_eq!(
                    fs::metadata(entry.path.parent().unwrap())
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777,
                    0o700
                );
            }
        }
        assert_eq!(
            fs::read_to_string(f.workspace.join("AGENTS.md")).unwrap(),
            "Local AGENTS"
        );
        assert_eq!(
            fs::read_to_string(f.workspace.join("CLAUDE.md")).unwrap(),
            "Local CLAUDE"
        );
        assert_eq!(
            release_task(&f.scope(NativeHost::Codex, None), task)
                .unwrap()
                .files_removed
                .len(),
            3
        );
    }

    #[test]
    fn instructions_are_wrapped_and_original_agents_resource_is_preserved() {
        let f = Fixture::new();
        let mut instructions = f.bundle.clone();
        instructions.skill.kind = ManagedSkillKind::Instructions;
        instructions.files[0] = ManagedSkillFile {
            path: "AGENTS.md".into(),
            content: "Original team instructions. Read references/guide.md".into(),
        };
        instructions.skill.file_paths[0] = "AGENTS.md".into();
        let scope = f.scope(NativeHost::Claude, None);
        let report = install(&scope, Uuid::new_v4(), &[instructions.clone()], false).unwrap();
        assert!(report.errors.is_empty());
        let entry = &report.entries[0].path;
        assert!(
            fs::read_to_string(entry)
                .unwrap()
                .contains("[AGENTS.md](AGENTS.md)")
        );
        assert_eq!(
            fs::read_to_string(entry.parent().unwrap().join("AGENTS.md")).unwrap(),
            instructions.files[0].content
        );
        let mut dynamic_description = instructions.clone();
        dynamic_description.skill.description = "Read !`dangerous` instructions.".into();
        assert_eq!(
            install(&scope, Uuid::new_v4(), &[dynamic_description], false)
                .unwrap_err()
                .to_string(),
            "invalid_native_entrypoint"
        );
        instructions.files.push(ManagedSkillFile {
            path: "SKILL.md".into(),
            content: "Foreign wrapper".into(),
        });
        instructions.skill.file_paths.push("SKILL.md".into());
        assert!(
            install(&scope, Uuid::new_v4(), &[instructions], false)
                .unwrap_err()
                .to_string()
                .contains("native_wrapper_path_conflict")
        );
    }

    #[test]
    fn two_tasks_close_in_both_orders_and_retained_pin_survives() {
        for reverse in [false, true] {
            let f = Fixture::new();
            let scope = f.scope(NativeHost::Codex, None);
            let a = Uuid::new_v4();
            let b = Uuid::new_v4();
            let first = install(&scope, a, &[f.bundle.clone()], false).unwrap();
            let path = first.entries[0].path.clone();
            assert!(
                install(&scope, b, &[f.bundle.clone()], false)
                    .unwrap()
                    .files_changed
                    .is_empty()
            );
            let order = if reverse { [b, a] } else { [a, b] };
            assert!(
                release_task(&scope, order[0])
                    .unwrap()
                    .files_removed
                    .is_empty()
            );
            assert!(path.exists());
            assert_eq!(
                release_task(&scope, order[1]).unwrap().files_removed.len(),
                1
            );
            assert!(!path.exists());
        }
        let f = Fixture::new();
        let scope = f.scope(NativeHost::OpenCode, None);
        let task = Uuid::new_v4();
        let path = install(&scope, task, &[f.bundle.clone()], true)
            .unwrap()
            .entries[0]
            .path
            .clone();
        install(&scope, task, &[f.bundle.clone()], false).unwrap();
        assert!(release_task(&scope, task).unwrap().files_removed.is_empty());
        assert!(path.exists());
        assert_eq!(
            uninstall(&scope, Some(f.bundle.skill.id), None)
                .unwrap()
                .files_removed
                .len(),
            1
        );
    }

    #[test]
    fn retained_native_uninstall_keeps_separate_canonical_copy() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::Claude, None);
        let cached = super::super::save(&f.root, &f.config, f.bundle.clone(), None).unwrap();
        let report = install(&scope, Uuid::new_v4(), &[f.bundle.clone()], true).unwrap();
        assert!(report.entries[0].persistent);
        assert!(report.entries[0].tasks.is_empty());
        assert_eq!(
            uninstall(&scope, None, None).unwrap().files_removed.len(),
            1
        );
        assert!(Path::new(cached["path"].as_str().unwrap()).exists());
    }

    #[test]
    fn workspace_lease_rejects_conflicting_brain_endpoint_or_environment_across_hosts() {
        let f = Fixture::new();
        let environment = Uuid::new_v4();
        let codex = f.scope(NativeHost::Codex, Some(environment));
        install(&codex, Uuid::new_v4(), &[f.bundle.clone()], false).unwrap();
        let claude = f.scope(NativeHost::Claude, Some(environment));
        assert!(
            install(&claude, Uuid::new_v4(), &[f.bundle.clone()], false)
                .unwrap()
                .errors
                .is_empty()
        );
        let conflicting = f.scope(NativeHost::OpenCode, None);
        let separate_root = f.base.join("other-plugin-root");
        plugin_storage::directory(&separate_root).unwrap();
        let mut separate_config = f.config.clone();
        separate_config.device = Uuid::new_v4();
        let separate_scope = NativeScope {
            root: &separate_root,
            config: &separate_config,
            ..conflicting
        };
        assert_eq!(
            install(&separate_scope, Uuid::new_v4(), &[f.bundle.clone()], false)
                .unwrap_err()
                .to_string(),
            "native_scope_busy"
        );
        assert_eq!(
            install(&conflicting, Uuid::new_v4(), &[f.bundle.clone()], false)
                .unwrap_err()
                .to_string(),
            "native_scope_busy"
        );
        let mut config = f.config.clone();
        config.endpoint = "https://other.example.test".into();
        let remote = NativeScope {
            config: &config,
            ..claude
        };
        assert_eq!(
            install(&remote, Uuid::new_v4(), &[f.bundle.clone()], false)
                .unwrap_err()
                .to_string(),
            "native_scope_busy"
        );
        config.brain = Uuid::new_v4();
        let mut other = f.bundle.clone();
        other.skill.brain_id = config.brain;
        let foreign = NativeScope {
            config: &config,
            ..claude
        };
        assert_eq!(
            install(&foreign, Uuid::new_v4(), &[other], false)
                .unwrap_err()
                .to_string(),
            "native_scope_busy"
        );
        assert!(!f.workspace.join(".opencode").exists());
    }

    #[test]
    fn scope_switch_retracts_old_native_pins_but_retains_canonical_cache() {
        let f = Fixture::new();
        let environment = Uuid::new_v4();
        let old = f.scope(NativeHost::Claude, Some(environment));
        let task = Uuid::new_v4();
        let cached = super::super::save(&f.root, &f.config, f.bundle.clone(), None).unwrap();
        let old_path = install(&old, task, &[f.bundle.clone()], true)
            .unwrap()
            .entries[0]
            .path
            .clone();
        release_task(&old, task).unwrap();
        assert!(old_path.exists());
        let new = f.scope(NativeHost::OpenCode, None);
        let report = install(&new, Uuid::new_v4(), &[f.bundle.clone()], false).unwrap();
        assert!(report.errors.is_empty());
        assert!(!old_path.exists());
        assert!(report.entries[0].path.exists());
        assert!(Path::new(cached["path"].as_str().unwrap()).exists());
    }

    #[test]
    fn ended_task_cannot_resurrect_on_another_host_or_a_concurrent_install() {
        let f = Fixture::new();
        let task = Uuid::new_v4();
        let scope = f.scope(NativeHost::Codex, None);
        install(&scope, task, &[f.bundle.clone()], false).unwrap();
        std::thread::scope(|threads| {
            let release = threads.spawn(|| release_task(&scope, task));
            let save = threads.spawn(|| install(&scope, task, &[f.bundle.clone()], false));
            assert!(release.join().unwrap().is_ok());
            let _ = save.join().unwrap();
        });
        assert!(!scope.target(scope.host, f.bundle.skill.id).exists());
        let delayed = install(
            &f.scope(NativeHost::Claude, None),
            task,
            &[f.bundle.clone()],
            false,
        );
        assert_eq!(delayed.unwrap_err().to_string(), "native_session_ended");
        assert!(
            install(&scope, Uuid::new_v4(), &[f.bundle.clone()], false)
                .unwrap()
                .errors
                .is_empty()
        );
    }

    #[test]
    fn successful_full_sync_updates_and_disables_every_compatibility_root() {
        let f = Fixture::new();
        let task = Uuid::new_v4();
        for host in [NativeHost::Codex, NativeHost::Claude, NativeHost::OpenCode] {
            install(
                &f.scope(host, None),
                task,
                &[f.bundle.clone()],
                host == NativeHost::Claude,
            )
            .unwrap();
        }
        let scope = f.scope(NativeHost::OpenCode, None);
        let update = sync(&scope, task, &[f.update()], false).unwrap();
        assert!(update.errors.is_empty());
        assert_eq!(update.files_changed.len(), 3);
        for host in [NativeHost::Codex, NativeHost::Claude, NativeHost::OpenCode] {
            let state = status(&f.scope(host, None)).unwrap();
            assert_eq!(state.entries[0].version, 2);
            assert!(
                fs::read_to_string(&state.entries[0].path)
                    .unwrap()
                    .contains("Native marker two")
            );
        }
        let disabled = sync(&scope, task, &[], false).unwrap();
        assert_eq!(disabled.files_removed.len(), 3);
        assert!(disabled.entries.is_empty());
    }

    #[test]
    fn local_edits_unknown_files_and_empty_directories_block_update_and_cleanup() {
        for mode in ["edit", "file", "directory"] {
            let f = Fixture::new();
            let scope = f.scope(NativeHost::Codex, None);
            let task = Uuid::new_v4();
            let entry = install(&scope, task, &[f.bundle.clone()], false)
                .unwrap()
                .entries[0]
                .path
                .clone();
            match mode {
                "edit" => fs::write(&entry, "Local changes").unwrap(),
                "file" => {
                    fs::write(entry.parent().unwrap().join("unknown.md"), "Local content").unwrap()
                }
                _ => fs::create_dir(entry.parent().unwrap().join("unknown")).unwrap(),
            }
            assert!(
                !install(&scope, task, &[f.update()], false)
                    .unwrap()
                    .errors
                    .is_empty()
            );
            assert!(!release_task(&scope, task).unwrap().errors.is_empty());
            assert!(entry.exists());
            assert!(status(&scope).unwrap().entries.is_empty());
            assert!(
                install(
                    &f.scope(NativeHost::Claude, Some(Uuid::new_v4())),
                    Uuid::new_v4(),
                    &[f.bundle.clone()],
                    false
                )
                .is_err()
            );
        }
    }

    #[test]
    fn unknown_native_target_is_never_adopted() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::Claude, None);
        let target = scope.target(scope.host, f.bundle.skill.id);
        safe_directory(&target, true).unwrap();
        fs::write(target.join("SKILL.md"), "Foreign content").unwrap();
        let report = install(&scope, Uuid::new_v4(), &[f.bundle.clone()], false).unwrap();
        assert_eq!(report.errors[0].reason, "native_unowned_target");
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "Foreign content"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_native_parent_entry_resource_and_guard_are_rejected() {
        use std::os::unix::fs::symlink;
        for mode in ["parent", "target", "resource", "guard"] {
            let f = Fixture::new();
            let scope = f.scope(NativeHost::Codex, None);
            let target = scope.target(scope.host, f.bundle.skill.id);
            let outside = f.base.join("foreign");
            plugin_storage::directory(&outside).unwrap();
            fs::write(outside.join("canary.md"), "Foreign canary").unwrap();
            let task = Uuid::new_v4();
            match mode {
                "parent" => symlink(&outside, f.workspace.join(".agents")).unwrap(),
                "target" => {
                    safe_directory(target.parent().unwrap(), true).unwrap();
                    symlink(&outside, &target).unwrap();
                }
                "guard" => {
                    safe_directory(&f.workspace.join(".recollect"), true).unwrap();
                    symlink(&outside, scope.guard()).unwrap();
                }
                _ => {
                    install(&scope, task, &[f.bundle.clone()], false).unwrap();
                    let resource = target.join("references/guide.md");
                    fs::remove_file(&resource).unwrap();
                    symlink(outside.join("canary.md"), resource).unwrap();
                }
            }
            let result = install(&scope, task, &[f.update()], false);
            assert!(
                result.is_err() || !result.unwrap().errors.is_empty(),
                "{mode}"
            );
            let _ = release_task(&scope, task);
            assert_eq!(
                fs::read_to_string(outside.join("canary.md")).unwrap(),
                "Foreign canary"
            );
        }
    }

    #[test]
    fn existing_public_host_parent_permissions_are_preserved() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::Claude, None);
        let parent = f.workspace.join(".claude/skills");
        safe_directory(&parent, true).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert!(
            install(&scope, Uuid::new_v4(), &[f.bundle.clone()], false)
                .unwrap()
                .errors
                .is_empty()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(parent).unwrap().permissions().mode() & 0o777,
                0o755
            );
        }
    }

    #[test]
    fn offline_lifecycle_reads_current_environment_and_releases_only_addressed_refs() {
        let f = Fixture::new();
        let scoped = f.scope(NativeHost::Codex, Some(Uuid::new_v4()));
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        install(&scoped, a, &[f.bundle.clone()], false).unwrap();
        install(&scoped, b, &[f.bundle.clone()], false).unwrap();
        let offline = f.scope(NativeHost::Codex, None);
        assert_eq!(status(&offline).unwrap().entries.len(), 1);
        assert!(
            uninstall(&offline, None, Some(a))
                .unwrap()
                .files_removed
                .is_empty()
        );
        assert_eq!(release_task(&offline, b).unwrap().files_removed.len(), 1);
    }

    #[test]
    fn device_revocation_preserves_other_devices_same_scope_refs_and_pins() {
        let f = Fixture::new();
        let first = f.scope(NativeHost::Codex, None);
        let mut other = f.config.clone();
        other.device = Uuid::new_v4();
        let separate_root = f.base.join("other-plugin-root");
        plugin_storage::directory(&separate_root).unwrap();
        let second = NativeScope {
            root: &separate_root,
            config: &other,
            ..first
        };
        install(&first, Uuid::new_v4(), &[f.bundle.clone()], true).unwrap();
        install(&second, Uuid::new_v4(), &[f.bundle.clone()], false).unwrap();
        assert!(revoke(&first).unwrap().files_removed.is_empty());
        assert_eq!(status(&second).unwrap().entries.len(), 1);
        assert_eq!(revoke(&second).unwrap().files_removed.len(), 1);
        assert!(
            leases(&first).unwrap()["leases"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn stale_versions_and_same_version_content_changes_cannot_replace_exports() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::Codex, None);
        let task = Uuid::new_v4();
        install(&scope, task, &[f.update()], false).unwrap();
        assert_eq!(
            install(&scope, task, &[f.bundle.clone()], false)
                .unwrap()
                .errors[0]
                .reason,
            "native_stale_version"
        );
        let mut changed = f.update();
        changed.files[1].content = "Changed bytes without version".into();
        assert_eq!(
            install(&scope, task, &[changed], false).unwrap().errors[0].reason,
            "native_version_conflict"
        );
        assert!(
            fs::read_to_string(scope.target(scope.host, f.bundle.skill.id).join("SKILL.md"))
                .unwrap()
                .contains("marker two")
        );
    }

    #[test]
    fn malformed_and_wrong_environment_bundles_leave_native_roots_uncreated() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::Codex, None);
        for mode in ["environment", "partial", "traversal", "dynamic"] {
            let mut invalid = f.bundle.clone();
            match mode {
                "environment" => invalid.skill.environment_id = Some(Uuid::new_v4()),
                "partial" => {
                    invalid.files.pop();
                }
                "traversal" => {
                    invalid.files[1].path = "../escape.md".into();
                    invalid.skill.file_paths[1] = "../escape.md".into();
                }
                _ => invalid.files[1].content = "!`run arbitrary command`".into(),
            }
            assert!(
                install(&scope, Uuid::new_v4(), &[invalid], false).is_err(),
                "{mode}"
            );
        }
        assert!(!scope.guard().exists());
        assert!(!f.workspace.join(".agents").exists());
    }

    #[test]
    fn journal_recovers_each_interruption_boundary_and_preserves_orphan_stages() {
        for phase in ["prepared", "old_moved", "new_moved"] {
            let f = Fixture::new();
            let scope = f.scope(NativeHost::Codex, None);
            let task = Uuid::new_v4();
            install(&scope, task, &[f.bundle.clone()], false).unwrap();
            let (_lock, mut registry) = lock_registry(&scope).unwrap();
            let next = render(&scope, scope.host, &f.update()).unwrap();
            let pending = Pending {
                next: Uuid::new_v4(),
                stage: Uuid::new_v4(),
                backup: Uuid::new_v4(),
            };
            let stage = write_stage(&scope, pending.stage, &next.files).unwrap();
            plugin_storage::write(&receipt_path(&scope, pending.next), &next).unwrap();
            registry.entries[0].pending = Some(pending.clone());
            store_registry(&scope, &registry).unwrap();
            let target = scope.target(scope.host, f.bundle.skill.id);
            let backup = scope
                .guard()
                .join("staging")
                .join(pending.backup.to_string());
            if phase != "prepared" {
                fs::rename(&target, &backup).unwrap();
            }
            if phase == "new_moved" {
                fs::rename(&stage, &target).unwrap();
            }
            let orphan = scope
                .guard()
                .join("staging")
                .join(Uuid::new_v4().to_string());
            plugin_storage::directory(&orphan).unwrap();
            fs::write(orphan.join("unknown.md"), "Orphan user content").unwrap();
            drop(_lock);
            let report = install(&scope, task, &[f.update()], false).unwrap();
            assert!(report.errors.is_empty(), "{phase}: {:?}", report.errors);
            assert_eq!(report.entries[0].version, 2);
            assert!(!backup.exists());
            assert_eq!(
                fs::read_to_string(orphan.join("unknown.md")).unwrap(),
                "Orphan user content"
            );
        }
    }

    #[test]
    fn update_keeps_shared_task_refs_and_retained_intent() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::Codex, None);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        install(&scope, a, &[f.bundle.clone()], true).unwrap();
        install(&scope, a, &[f.bundle.clone()], false).unwrap();
        install(&scope, b, &[f.update()], false).unwrap();
        let state = status(&scope).unwrap();
        assert_eq!(state.entries[0].tasks.len(), 2);
        assert!(state.entries[0].persistent);
        release_task(&scope, a).unwrap();
        release_task(&scope, b).unwrap();
        assert!(status(&scope).unwrap().entries[0].persistent);
        uninstall(&scope, None, None).unwrap();
        assert!(status(&scope).unwrap().entries.is_empty());
    }

    #[test]
    fn persistent_uninstall_leaves_no_phantom_temporary_lease() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::Claude, Some(Uuid::new_v4()));
        install(&scope, Uuid::new_v4(), &[f.bundle.clone()], true).unwrap();
        assert!(
            leases(&scope).unwrap()["leases"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        uninstall(&scope, None, None).unwrap();
        let other = f.scope(NativeHost::OpenCode, None);
        assert!(
            install(&other, Uuid::new_v4(), &[f.bundle.clone()], false)
                .unwrap()
                .errors
                .is_empty()
        );
    }

    #[test]
    fn live_scope_transition_releases_only_its_refs_and_does_not_end_task() {
        let f = Fixture::new();
        let old = f.scope(NativeHost::Codex, Some(Uuid::new_v4()));
        let task = Uuid::new_v4();
        let other_task = Uuid::new_v4();
        install(&old, task, &[f.bundle.clone()], false).unwrap();
        install(&old, other_task, &[f.bundle.clone()], false).unwrap();
        release_scope_task(&old, task).unwrap();
        let new = f.scope(NativeHost::OpenCode, None);
        assert_eq!(
            install(&new, task, &[f.bundle.clone()], false)
                .unwrap_err()
                .to_string(),
            "native_scope_busy"
        );
        release_task(&old, other_task).unwrap();
        assert!(
            install(&new, task, &[f.bundle.clone()], false)
                .unwrap()
                .errors
                .is_empty()
        );
    }

    #[test]
    fn explicit_refresh_rewrites_only_unchanged_owned_entrypoint_and_never_claims_loading() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::Codex, None);
        let report = install(&scope, Uuid::new_v4(), &[f.bundle.clone()], false).unwrap();
        let path = &report.entries[0].path;
        let original = fs::read(path).unwrap();
        let refreshed = request_refresh(&scope).unwrap();
        assert_eq!(refreshed.state, "refresh_requested");
        assert!(!refreshed.native_loaded);
        assert_eq!(fs::read(path).unwrap(), original);
        fs::write(path, "User changed file").unwrap();
        assert!(!request_refresh(&scope).unwrap().errors.is_empty());
        assert_eq!(fs::read_to_string(path).unwrap(), "User changed file");
    }

    #[test]
    fn late_old_brain_session_end_is_fenced_without_touching_the_new_brain() {
        let f = Fixture::new();
        let old = f.scope(NativeHost::Codex, None);
        let old_task = Uuid::new_v4();
        install(&old, old_task, &[f.bundle.clone()], false).unwrap();
        release_scope_task(&old, old_task).unwrap();
        let mut config = f.config.clone();
        config.brain = Uuid::new_v4();
        let mut bundle = f.bundle.clone();
        bundle.skill.brain_id = config.brain;
        let new = NativeScope {
            config: &config,
            host: NativeHost::Claude,
            ..old
        };
        let new_task = Uuid::new_v4();
        let path = install(&new, new_task, &[bundle], false).unwrap().entries[0]
            .path
            .clone();
        assert!(
            release_task(&old, old_task)
                .unwrap()
                .files_removed
                .is_empty()
        );
        assert!(path.exists());
        release_task(&new, new_task).unwrap();
        assert_eq!(
            install(&old, old_task, &[f.bundle.clone()], false)
                .unwrap_err()
                .to_string(),
            "native_session_ended"
        );
    }

    #[test]
    fn scoped_export_and_delayed_denial_are_fenced_by_new_live_intent() {
        let f = Fixture::new();
        let task = Uuid::new_v4();
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let first = f.scope(NativeHost::Codex, Some(Uuid::new_v4()));
        observe_task_scope(&first, task, first_id, now).unwrap();
        install_scoped(&first, task, first_id, &[f.bundle.clone()], false).unwrap();
        let second = f.scope(NativeHost::Codex, Some(Uuid::new_v4()));
        let withdrawn =
            observe_task_scope(&second, task, second_id, now + chrono::Duration::seconds(1))
                .unwrap();
        assert_eq!(withdrawn.files_removed.len(), 1);
        for persistent in [false, true] {
            assert_eq!(
                sync_scoped(&first, task, first_id, &[f.bundle.clone()], persistent)
                    .unwrap_err()
                    .to_string(),
                "native_scope_changed"
            );
        }
        assert!(
            sync_scoped(&second, task, second_id, &[f.update()], false)
                .unwrap()
                .errors
                .is_empty()
        );
        assert_eq!(
            release_scope_task_scoped(&first, task, first_id)
                .unwrap_err()
                .to_string(),
            "native_scope_changed"
        );
        assert_eq!(status(&second).unwrap().entries[0].version, 2);
        assert_eq!(
            observe_task_scope(&first, task, first_id, now)
                .unwrap_err()
                .to_string(),
            "native_scope_stale"
        );
        assert_eq!(
            observe_task_scope(&first, task, first_id, now + chrono::Duration::seconds(1))
                .unwrap_err()
                .to_string(),
            "native_scope_stale"
        );
    }

    #[test]
    fn identical_scope_observation_is_idempotent_and_never_relabels_its_environment() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::Claude, Some(Uuid::new_v4()));
        let task = Uuid::new_v4();
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        observe_task_scope(&scope, task, id, now).unwrap();
        install_scoped(&scope, task, id, &[f.bundle.clone()], false).unwrap();
        assert!(
            observe_task_scope(&scope, task, id, now)
                .unwrap()
                .files_removed
                .is_empty()
        );
        assert_eq!(status(&scope).unwrap().entries[0].tasks, vec![task]);
        let other_environment = NativeScope {
            environment: None,
            ..scope
        };
        assert_eq!(
            observe_task_scope(&other_environment, task, id, now)
                .unwrap_err()
                .to_string(),
            "native_scope_intent_mismatch"
        );
    }

    #[test]
    fn observed_scope_transition_preserves_other_tasks_and_persistent_pins() {
        let f = Fixture::new();
        let scope = f.scope(NativeHost::OpenCode, Some(Uuid::new_v4()));
        let task = Uuid::new_v4();
        let peer = Uuid::new_v4();
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        observe_task_scope(&scope, task, id, now).unwrap();
        install_scoped(&scope, task, id, &[f.bundle.clone()], true).unwrap();
        install_scoped(&scope, task, id, &[f.bundle.clone()], false).unwrap();
        install(&scope, peer, &[f.bundle.clone()], false).unwrap();
        let next = NativeScope {
            environment: None,
            ..scope
        };
        let next_id = Uuid::new_v4();
        observe_task_scope(&next, task, next_id, now + chrono::Duration::seconds(1)).unwrap();
        let state = status(&scope).unwrap();
        assert_eq!(state.entries[0].tasks, vec![peer]);
        assert!(state.entries[0].persistent);
        assert_eq!(
            sync_scoped(&next, task, next_id, &[f.bundle.clone()], false)
                .unwrap_err()
                .to_string(),
            "native_scope_busy"
        );
    }

    #[test]
    fn interrupted_intent_cleanup_is_retried_before_any_new_export() {
        let f = Fixture::new();
        let old = f.scope(NativeHost::Codex, None);
        let task = Uuid::new_v4();
        let old_id = Uuid::new_v4();
        let new_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        observe_task_scope(&old, task, old_id, now).unwrap();
        let path = install_scoped(&old, task, old_id, &[f.bundle.clone()], false)
            .unwrap()
            .entries[0]
            .path
            .clone();
        let original = fs::read(&path).unwrap();
        fs::write(&path, "User edit").unwrap();
        let new = NativeScope {
            environment: Some(Uuid::new_v4()),
            ..old
        };
        let created_at = now + chrono::Duration::seconds(1);
        assert!(
            !observe_task_scope(&new, task, new_id, created_at)
                .unwrap()
                .errors
                .is_empty()
        );
        assert_eq!(
            install_scoped(&new, task, new_id, &[f.bundle.clone()], false)
                .unwrap_err()
                .to_string(),
            "native_scope_reconciliation_required"
        );
        assert_eq!(
            install_scoped(&old, task, old_id, &[f.bundle.clone()], false)
                .unwrap_err()
                .to_string(),
            "native_scope_changed"
        );
        fs::write(&path, original).unwrap();
        assert_eq!(
            observe_task_scope(&new, task, new_id, created_at)
                .unwrap()
                .files_removed
                .len(),
            1
        );
        assert!(
            install_scoped(&new, task, new_id, &[f.bundle.clone()], false)
                .unwrap()
                .errors
                .is_empty()
        );
    }

    #[test]
    fn metadata_publication_cannot_overwrite_a_newer_generation_and_can_publish_after_end() {
        let f = Fixture::new();
        let task = Uuid::new_v4();
        let now = chrono::Utc::now();
        let old = f.scope(NativeHost::OpenCode, None);
        let first_id = Uuid::new_v4();
        observe_task_scope(&old, task, first_id, now).unwrap();
        let new = NativeScope {
            environment: Some(Uuid::new_v4()),
            ..old
        };
        let second_id = Uuid::new_v4();
        observe_task_scope(&new, task, second_id, now + chrono::Duration::seconds(1)).unwrap();
        let path = f
            .root
            .join("native-skills/catalogues")
            .join(format!("{}-{task}.json", f.config.device));
        let body = serde_json::json!({"scope_id":second_id,"task_id":task,"brain_id":f.config.brain,"workspace":f.workspace,"entries":[]});
        publish_scoped(&new, task, second_id, &path, &body).unwrap();
        assert_eq!(
            publish_scoped(&old, task, first_id, &path, &body)
                .unwrap_err()
                .to_string(),
            "native_scope_changed"
        );
        release_task(&new, task).unwrap();
        // Offline cleanup can omit the environment selector, but never identity.
        publish_scoped(&old, task, second_id, &path, &body).unwrap();
        assert_eq!(
            read_json::<serde_json::Value>(&path, REGISTRY_LIMIT).unwrap(),
            body
        );
        assert_eq!(
            install_scoped(&new, task, second_id, &[f.bundle.clone()], false)
                .unwrap_err()
                .to_string(),
            "native_session_ended"
        );
    }
}
