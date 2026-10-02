use anyhow::{Result, anyhow, bail, ensure};
use chrono::Utc;
use recollect_protocol::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    fs,
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use uuid::Uuid;

const TOOL_PATH: &str = "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin";
pub fn project_root() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    path.canonicalize().unwrap_or(path)
}
pub fn default_enola() -> PathBuf {
    project_root().join(".cache/enola-tools/v0.4.19/enola")
}
#[derive(Serialize, Deserialize)]
pub struct PreparedPublication {
    pub endpoint: String,
    pub device_id: Uuid,
    pub brain_id: Uuid,
    pub repository_id: Uuid,
    pub input: PublicationInput,
}
pub struct Preparation<'a> {
    pub checkout: &'a Path,
    pub revision: &'a str,
    pub repository: &'a Repository,
    pub operation: &'a OperationBinding,
    pub retain_files: Vec<String>,
    pub allow_file_content: bool,
    pub enola: &'a Path,
    pub configured_secrets: Vec<String>,
    pub staging_root: &'a Path,
}
struct OwnedStage(PathBuf);
impl Drop for OwnedStage {
    fn drop(&mut self) {
        // Created exclusively by this invocation; never a caller-selected checkout.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub async fn private_root(path: &Path) -> Result<PathBuf> {
    // A distributed plugin has no source checkout. Its explicitly selected,
    // private application-data tree is the additional owned storage boundary.
    if let Ok(plugin) = crate::plugin_storage::root()
        && path.is_absolute()
        && path.starts_with(&plugin)
    {
        crate::plugin_storage::directory(path)?;
        return Ok(path.to_owned());
    }
    let project = project_root();
    let requested = if path.is_absolute() {
        path.to_owned()
    } else {
        project.join(path)
    };
    ensure!(
        requested.starts_with(&project),
        "Publication storage must stay inside Recollect."
    );
    let suffix = requested.strip_prefix(&project)?;
    ensure!(
        suffix
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_))),
        "Use a normalized publication storage directory."
    );
    let mut current = project;
    for component in suffix.components() {
        current.push(component);
        match fs::symlink_metadata(&current).await {
            Ok(meta) => ensure!(
                meta.is_dir() && !meta.is_symlink(),
                "Publication storage must use ordinary owned directories."
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                builder.mode(0o700);
                match builder.create(&current).await {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        let meta = fs::symlink_metadata(&current).await?;
                        ensure!(
                            meta.is_dir() && !meta.is_symlink(),
                            "Publication directory changed during creation."
                        );
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            Err(_) => bail!("Cannot access publication storage."),
        }
    }
    Ok(current)
}
pub(crate) async fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).await?;
    file.write_all(bytes).await?;
    file.sync_all().await?;
    Ok(())
}
async fn bounded_read(reader: impl AsyncRead + Unpin, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .await?;
    ensure!(
        bytes.len() <= limit,
        "Extraction exceeded its output limit."
    );
    Ok(bytes)
}
async fn run(mut command: Command, limit: usize, timeout: Duration) -> Result<Vec<u8>> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|_| {
        anyhow!("Required Git or Enola executable is unavailable. Run repository setup and retry.")
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Extraction output unavailable."))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("Extraction diagnostics unavailable."))?;
    tokio::time::timeout(timeout, async {
        let (out, _, status) = tokio::try_join!(bounded_read(stdout, limit), bounded_read(stderr, 256 * 1024), async { Ok::<_, anyhow::Error>(child.wait().await?) })?;
        ensure!(status.success(), "Git or Enola failed. Check the local revision, executable and supported input; source diagnostics are not persisted.");
        Ok::<_, anyhow::Error>(out)
    }).await.map_err(|_| anyhow!("Git or Enola exceeded its time limit."))?
}
fn clean_command(program: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .env("PATH", TOOL_PATH)
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_NO_REPLACE_OBJECTS", "1");
    command
}
async fn git(checkout: &Path, args: &[&str], limit: usize) -> Result<Vec<u8>> {
    let mut command = clean_command(Path::new("git"));
    command
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-C",
        ])
        .arg(checkout)
        .args(args);
    run(command, limit, Duration::from_secs(10)).await
}
pub(crate) async fn artifact(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)
        .await
        .map_err(|_| anyhow!("Enola did not produce a required artifact."))?;
    ensure!(
        metadata.is_file() && !metadata.is_symlink() && metadata.len() <= limit as u64,
        "Invalid or oversized extraction artifact."
    );
    bounded_read(fs::File::open(path).await?, limit).await
}
pub async fn prepare(options: Preparation<'_>) -> Result<PublicationInput> {
    tokio::time::timeout(Duration::from_secs(120), prepare_inner(options))
        .await
        .map_err(|_| anyhow!("Repository preparation exceeded 120 seconds; no upload was made."))?
}
async fn prepare_inner(mut options: Preparation<'_>) -> Result<PublicationInput> {
    ensure!(
        !options.revision.is_empty()
            && options.revision.len() <= 256
            && !options.revision.chars().any(char::is_control),
        "Use a printable revision of at most 256 bytes."
    );
    let op = options.operation;
    ensure!(
        op.scope_valid
            && op.kind == "capture"
            && op.device_id.is_some()
            && op.brain_id == options.repository.brain_id
            && (op.scope.selection.repository_ids.is_empty()
                || op
                    .scope
                    .selection
                    .repository_ids
                    .contains(&options.repository.id)),
        "Capture operation does not authorize this repository."
    );
    options.retain_files.sort();
    options.retain_files.dedup();
    ensure!(
        options.retain_files.len() <= 20 && options.retain_files.iter().all(|p| repository_path(p)),
        "Select at most 20 ordinary relative retained-file paths."
    );
    ensure!(
        options.retain_files.is_empty() || options.allow_file_content,
        "This Brain does not allow retaining repository file content."
    );
    let checkout = fs::canonicalize(options.checkout)
        .await
        .map_err(|_| anyhow!("Choose an existing local checkout."))?;
    let observation = crate::workspace::observe_checkout(&checkout).await;
    ensure!(
        observation.status == "available" && observation.dirty.is_some(),
        "Checkout Git metadata is unavailable."
    );
    let origin = observation
        .origin
        .ok_or_else(|| anyhow!("Checkout origin is unavailable."))?;
    ensure!(
        options.repository.origins.contains(&origin),
        "Checkout origin does not match this registered repository."
    );
    let commit = String::from_utf8(
        git(
            &checkout,
            &[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{}^{{commit}}", options.revision),
            ],
            128,
        )
        .await?,
    )
    .map_err(|_| anyhow!("Invalid Git revision response."))?
    .trim()
    .to_owned();
    ensure!(
        git_object_id(&commit),
        "Git did not resolve an exact commit."
    );
    let captured_at = Utc::now();
    let root = private_root(options.staging_root).await?;
    let stage_path = root.join(format!("stage-{}", Uuid::new_v4()));
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(&stage_path).await?;
    let stage = OwnedStage(stage_path);
    let tree = stage.0.join(options.repository.id.to_string());
    fs::create_dir(&tree).await?;
    fs::create_dir(stage.0.join("tmp")).await?;
    let listing = git(
        &checkout,
        &["ls-tree", "-rlz", "--full-tree", &commit],
        12 * 1024 * 1024,
    )
    .await?;
    let mut files = Vec::new();
    for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        ensure!(
            files.len() < 5000,
            "Committed tree exceeds 5000 entries; inventory is incomplete."
        );
        let (meta, path) = entry.split_at(
            entry
                .iter()
                .position(|b| *b == b'\t')
                .ok_or_else(|| anyhow!("Invalid Git tree listing."))?,
        );
        let path = std::str::from_utf8(&path[1..])
            .map_err(|_| anyhow!("Committed tree has a non-UTF-8 path."))?;
        ensure!(
            repository_path(path),
            "Committed tree has an unsupported or unsafe path."
        );
        let fields: Vec<_> = std::str::from_utf8(meta)?.split_whitespace().collect();
        ensure!(
            fields.len() == 4 && git_object_id(fields[2]),
            "Invalid Git tree entry."
        );
        let size = if fields[3] == "-" {
            None
        } else {
            Some(
                fields[3]
                    .parse::<u64>()
                    .map_err(|_| anyhow!("Invalid Git object size."))?,
            )
        };
        let status = match fields[0] {
            "120000" => "symlink",
            "160000" => "gitlink",
            "100644" | "100755" if excluded_repository_path(path) => "excluded_path",
            "100644" | "100755" if size.is_none_or(|n| n > REPOSITORY_TEXT_MAX_BYTES as u64) => {
                "oversized"
            }
            "100644" | "100755" => "materialized",
            _ => bail!("Unsupported Git tree mode."),
        };
        files.push(ExtractedFile {
            path: path.into(),
            object_id: fields[2].into(),
            mode: fields[0].into(),
            size,
            status: status.into(),
            extraction: "excluded".into(),
            content: None,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut total_bytes = 0;
    let mut hcl_evidence = std::collections::BTreeMap::new();
    let mut hcl_module_count = 0;
    for file in &mut files {
        if file.status != "materialized" {
            continue;
        }
        let size = file.size.unwrap_or(0);
        if total_bytes + size > 64 * 1024 * 1024 {
            file.status = "budget_exceeded".into();
            continue;
        }
        let bytes = git(
            &checkout,
            &["cat-file", "blob", &file.object_id],
            REPOSITORY_TEXT_MAX_BYTES,
        )
        .await?;
        ensure!(
            bytes.len() as u64 == size,
            "Git object size changed unexpectedly."
        );
        let text = match String::from_utf8(bytes) {
            Ok(s) if !s.contains('\0') => s,
            _ => {
                file.status = "binary".into();
                continue;
            }
        };
        if repository_secret(&text, &options.configured_secrets) {
            file.status = "excluded_secret".into();
            continue;
        }
        if crate::publication_hcl::is_hcl(&file.path) {
            let hcl_path = file.path.clone();
            let hcl_text = text.clone();
            let remaining = 100_000 - hcl_module_count;
            let scan = tokio::task::spawn_blocking(move || {
                crate::publication_hcl::inspect_limited(&hcl_path, &hcl_text, remaining)
            })
            .await
            .map_err(|_| anyhow!("Committed HCL inspection did not finish."))?;
            hcl_module_count += scan.module_count();
            ensure!(
                hcl_module_count <= 100_000,
                "Committed module declarations exceed the publication limit."
            );
            hcl_evidence.insert(file.path.clone(), scan);
        }
        let path = tree.join(&file.path);
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| anyhow!("Invalid materialization path."))?,
        )
        .await?;
        write_private(&path, text.as_bytes()).await?;
        total_bytes += size;
        file.extraction = "no_facts".into();
        if options.retain_files.binary_search(&file.path).is_ok() {
            file.content = Some(text);
        }
    }
    ensure!(
        options
            .retain_files
            .iter()
            .all(|p| files.iter().any(|f| &f.path == p && f.content.is_some())),
        "A selected retained file is absent, excluded or too large. Review the explicit selection."
    );
    let config = format!(
        "repo: {}\nproviders: []\nrenderers: []\nincremental: false\nhistory:\n  enabled: false\noutput:\n  dir: .enola\n",
        serde_json::to_string(&tree)?
    );
    let config_path = stage.0.join("extract.yaml");
    write_private(&config_path, config.as_bytes()).await?;
    let enola = fs::canonicalize(options.enola).await.map_err(|_| {
        anyhow!("Enola is unavailable. Run scripts/setup-enola.py or select RECOLLECT_ENOLA_BIN.")
    })?;
    let mut command = clean_command(&enola);
    command
        .current_dir(&stage.0)
        .env("TMPDIR", stage.0.join("tmp"))
        .env("GIT_CEILING_DIRECTORIES", &stage.0)
        .env("ENOLA_NO_UPDATE_CHECK", "1")
        .arg("--generate")
        .arg(config_path);
    run(command, 256 * 1024, Duration::from_secs(100)).await?;
    let output = tree.join(".enola");
    let mut facts = Vec::new();
    for line in artifact(&output.join("facts.jsonl"), FACTS_MAX_BYTES)
        .await?
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
    {
        let record: Value =
            serde_json::from_slice(line).map_err(|_| anyhow!("Enola emitted malformed facts."))?;
        facts.push(record);
    }
    crate::publication_hcl::attach(&mut facts, &files, &hcl_evidence);
    let insights: Value =
        serde_json::from_slice(&artifact(&output.join("insights.json"), 1024 * 1024).await?)
            .map_err(|_| anyhow!("Enola emitted malformed insights."))?;
    let receipt: Value =
        serde_json::from_slice(&artifact(&output.join("receipt.json"), 65536).await?)
            .map_err(|_| anyhow!("Enola emitted a malformed receipt."))?;
    let fact_paths: BTreeSet<_> = facts
        .iter()
        .filter_map(|f| f.get("file").and_then(Value::as_str))
        .collect();
    for file in &mut files {
        if file.status == "materialized" && fact_paths.contains(file.path.as_str()) {
            file.extraction = "facts_emitted".into();
        }
    }
    let input = PublicationInput {
        publication_id: Uuid::new_v4(),
        operation_id: op.id,
        origin,
        revision: commit,
        branch: observation.branch,
        dirty: observation.dirty.unwrap_or(false),
        captured_at,
        adapter: "enola-committed".into(),
        adapter_build: receipt
            .get("enola_version")
            .and_then(Value::as_str)
            .unwrap_or("unreported")
            .into(),
        extractor_version: receipt
            .get("extractor_version")
            .and_then(Value::as_str)
            .unwrap_or("unreported")
            .into(),
        settings: json!({"retained_files": options.retain_files, "input": "git-blobs", "providers": [], "renderers": [], "incremental": false, "history": false,"module_source_parser":crate::publication_hcl::PARSER}),
        files,
        facts,
        insights,
        receipt: stable_receipt(&receipt),
    };
    validate_publication(&input).map_err(|e| anyhow!(e))?;
    ensure!(
        !publication_has_secret(&input, &options.configured_secrets),
        "Extractor artifacts contain excluded credential material; no prepared bundle was saved."
    );
    Ok(input)
}
pub fn configured_secrets() -> Vec<String> {
    std::env::vars()
        .filter_map(|(key, value)| {
            let name = key.to_ascii_uppercase();
            (value.len() >= 4
                && ["KEY", "TOKEN", "PASSWORD", "SECRET"]
                    .iter()
                    .any(|s| name.contains(s)))
            .then_some(value)
        })
        .collect()
}
pub fn publication_has_secret(input: &PublicationInput, secrets: &[String]) -> bool {
    fn visit(value: &Value, secrets: &[String]) -> bool {
        match value {
            Value::String(s) => repository_secret(s, secrets),
            Value::Array(a) => a.iter().any(|v| visit(v, secrets)),
            Value::Object(o) => o
                .iter()
                .any(|(k, v)| repository_secret(k, secrets) || visit(v, secrets)),
            _ => false,
        }
    }
    serde_json::to_value(input).is_ok_and(|v| visit(&v, secrets))
}
pub async fn save_bundle(root: &Path, bundle: &PreparedPublication) -> Result<PathBuf> {
    validate_publication(&bundle.input).map_err(|e| anyhow!(e))?;
    ensure!(
        !publication_has_secret(&bundle.input, &configured_secrets()),
        "Prepared input contains excluded credential material."
    );
    let root = private_root(root).await?;
    let path = root.join(format!("{}.json", bundle.input.publication_id));
    write_private(&path, &serde_json::to_vec(bundle)?).await?;
    fs::File::open(&root).await?.sync_all().await?;
    Ok(path)
}
pub async fn load_bundle(root: &Path, id: Uuid) -> Result<PreparedPublication> {
    let root = private_root(root).await?;
    let bytes = artifact(
        &root.join(format!("{id}.json")),
        PUBLICATION_MAX_BYTES + 8192,
    )
    .await?;
    let bundle: PreparedPublication = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow!("Prepared publication is unreadable."))?;
    ensure!(
        bundle.input.publication_id == id,
        "Prepared publication identity does not match its filename."
    );
    validate_publication(&bundle.input).map_err(|e| anyhow!(e))?;
    ensure!(
        !publication_has_secret(&bundle.input, &configured_secrets()),
        "Prepared input contains excluded credential material."
    );
    Ok(bundle)
}
pub async fn remove_bundle(root: &Path, id: Uuid) -> Result<()> {
    // Only a validated UUID-named regular bundle under the configured owned root.
    load_bundle(root, id).await?;
    let root = private_root(root).await?;
    fs::remove_file(root.join(format!("{id}.json"))).await?;
    fs::File::open(root).await?.sync_all().await?;
    Ok(())
}
