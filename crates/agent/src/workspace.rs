use anyhow::{Result, anyhow, bail, ensure};
use recollect_protocol::{CheckoutObservation, CheckoutRefresh, canonical_git_origin};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{fs, io::AsyncReadExt, process::Command};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selector {
    brain: String,
}

#[derive(Serialize)]
pub struct Discovery {
    pub brain_selector: String,
    #[serde(flatten)]
    pub refresh: CheckoutRefresh,
    pub excluded_workspaces: Vec<String>,
}

pub async fn selector(start: &Path) -> Result<(PathBuf, String)> {
    let mut root = fs::canonicalize(start)
        .await
        .map_err(|_| anyhow!("Choose an existing workspace directory."))?;
    ensure!(
        root.is_dir(),
        "Workspace discovery starts from a directory."
    );
    loop {
        let file = root.join(".recollect/workspace.toml");
        match fs::symlink_metadata(&file).await {
            Ok(meta) => {
                ensure!(
                    meta.is_file() && !meta.is_symlink() && meta.len() <= 8192,
                    "The nearest workspace selector must be a regular UTF-8 file of at most 8 KiB."
                );
                let folder = fs::symlink_metadata(root.join(".recollect")).await?;
                ensure!(
                    !folder.is_symlink(),
                    "The workspace configuration directory must not be a symlink."
                );
                let mut bytes = Vec::new();
                fs::File::open(&file)
                    .await
                    .map_err(|_| anyhow!("Cannot read the nearest workspace selector."))?
                    .take(8193)
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(|_| anyhow!("Cannot read the nearest workspace selector."))?;
                ensure!(bytes.len() <= 8192, "Workspace selector exceeds 8 KiB.");
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| anyhow!("Workspace selector must be UTF-8."))?;
                let config: Selector = toml::from_str(text).map_err(|_| anyhow!("Invalid nearest workspace selector. Use only brain = \"Brain name or UUID\"."))?;
                ensure!(
                    !config.brain.trim().is_empty()
                        && config.brain.chars().count() <= 120
                        && !config.brain.chars().any(char::is_control),
                    "Workspace brain must be a nonempty name or UUID of at most 120 characters."
                );
                return Ok((root, config.brain.trim().into()));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                bail!("Cannot inspect the nearest workspace selector. Check directory access.")
            }
        }
        if !root.pop() {
            bail!("No .recollect/workspace.toml found in this directory or its parents.");
        }
    }
}

async fn git(path: &Path, args: &[&str]) -> Result<Option<String>> {
    let mut command = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-C",
        ])
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|_| anyhow!("Git is unavailable."))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Git output is unavailable."))?;
    let mut bytes = Vec::new();
    let result = tokio::time::timeout(Duration::from_secs(2), async {
        stdout.take(1048577).read_to_end(&mut bytes).await?;
        if bytes.len() > 1048576 {
            return Err(anyhow!("Git metadata exceeded its output limit."));
        }
        Ok::<_, anyhow::Error>(child.wait().await?)
    })
    .await;
    match result {
        Ok(Ok(status)) if status.success() => Ok(Some(
            String::from_utf8(bytes)
                .map_err(|_| anyhow!("Git metadata is not UTF-8."))?
                .trim_end()
                .to_owned(),
        )),
        Ok(Ok(_)) => Ok(None),
        _ => {
            let _ = child.kill().await;
            bail!("Git metadata is unavailable or exceeded its time/output limit.");
        }
    }
}

/// Read bounded Git metadata at an explicitly selected canonical checkout path.
/// This does not choose a Brain, persist a registration or read source contents.
pub async fn observe_checkout(path: &Path) -> CheckoutObservation {
    let mut observation = CheckoutObservation {
        local_path: path.to_string_lossy().into(),
        origin: None,
        branch: None,
        head: None,
        dirty: None,
        status: "git_unavailable".into(),
    };
    let Ok(Some(top)) = git(path, &["rev-parse", "--show-toplevel"]).await else {
        return observation;
    };
    if fs::canonicalize(top).await.ok().as_deref() != Some(path) {
        return observation;
    }
    let (origin, head, branch, dirty) = tokio::join!(
        git(path, &["config", "--local", "--get", "remote.origin.url"]),
        git(path, &["rev-parse", "--verify", "HEAD"]),
        git(path, &["symbolic-ref", "--quiet", "--short", "HEAD"]),
        git(
            path,
            &["status", "--porcelain=v1", "--untracked-files=normal"]
        )
    );
    let metadata_failed =
        origin.is_err() || head.is_err() || branch.is_err() || !matches!(&dirty, Ok(Some(_)));
    observation.status = match origin {
        Ok(Some(origin)) => match canonical_git_origin(&origin) {
            Ok(origin) => {
                observation.origin = Some(origin);
                "available"
            }
            Err(_) => "unsupported_origin",
        },
        Ok(None) => "no_origin",
        Err(_) => "git_unavailable",
    }
    .into();
    observation.head = head
        .ok()
        .flatten()
        .filter(|h| matches!(h.len(), 40 | 64) && h.chars().all(|c| c.is_ascii_hexdigit()));
    observation.branch = branch
        .ok()
        .flatten()
        .filter(|b| b.chars().count() <= 256 && !b.chars().any(char::is_control));
    observation.dirty = dirty.ok().flatten().map(|value| !value.is_empty());
    if metadata_failed {
        observation.status = "git_unavailable".into();
    }
    observation
}

fn skipped(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "node_modules" | "target" | "vendor" | "venv" | "dist" | "build" | "__pycache__"
        )
}
fn note(report: &mut Discovery, kind: &str, path: &Path, root: &Path) {
    if report.refresh.notes.len() < 100 {
        let relative = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
        report.refresh.notes.push(format!(
            "{kind}: {}",
            relative.chars().take(140).collect::<String>()
        ));
    }
}

pub async fn discover(start: &Path) -> Result<Discovery> {
    let (root, brain_selector) = selector(start).await?;
    let root_text = root
        .to_str()
        .ok_or_else(|| anyhow!("Workspace path must be UTF-8."))?;
    ensure!(
        root_text.chars().count() <= 2000 && !root_text.chars().any(char::is_control),
        "Workspace path is too long or contains control characters."
    );
    let mut report = Discovery {
        brain_selector,
        refresh: CheckoutRefresh {
            workspace_root: root_text.into(),
            complete: true,
            notes: vec![],
            checkouts: vec![],
        },
        excluded_workspaces: vec![],
    };
    let mut queue = VecDeque::from([(root.clone(), 0usize)]);
    let mut directories = 0;
    let mut entries = 0;
    let started = Instant::now();
    while let Some((path, depth)) = queue.pop_front() {
        if started.elapsed() >= Duration::from_secs(30)
            || directories >= 10000
            || entries >= 100000
            || report.refresh.checkouts.len() >= 200
        {
            report.refresh.complete = false;
            note(&mut report, "Discovery limit reached", &path, &root);
            break;
        }
        directories += 1;
        if path != root {
            match fs::symlink_metadata(path.join(".recollect/workspace.toml")).await {
                Ok(_) => {
                    report
                        .excluded_workspaces
                        .push(path.to_string_lossy().into());
                    note(&mut report, "Excluded nested workspace", &path, &root);
                    continue;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {
                    report.refresh.complete = false;
                    note(
                        &mut report,
                        "Unclear workspace boundary excluded",
                        &path,
                        &root,
                    );
                    continue;
                }
            }
        }
        if fs::symlink_metadata(path.join(".git")).await.is_ok() {
            let remaining = Duration::from_secs(30).saturating_sub(started.elapsed());
            match tokio::time::timeout(remaining, observe_checkout(&path)).await {
                Ok(observation) => report.refresh.checkouts.push(observation),
                Err(_) => {
                    report.refresh.complete = false;
                    note(&mut report, "Discovery time limit reached", &path, &root);
                    break;
                }
            }
        }
        let Ok(mut reader) = fs::read_dir(&path).await else {
            report.refresh.complete = false;
            note(&mut report, "Cannot read directory", &path, &root);
            continue;
        };
        let mut children = vec![];
        loop {
            let entry = match reader.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(_) => {
                    report.refresh.complete = false;
                    note(&mut report, "Cannot read directory entry", &path, &root);
                    break;
                }
            };
            entries += 1;
            if entries >= 100000 || started.elapsed() >= Duration::from_secs(30) {
                report.refresh.complete = false;
                note(&mut report, "Discovery limit reached", &path, &root);
                break;
            }
            let Ok(kind) = entry.file_type().await else {
                report.refresh.complete = false;
                continue;
            };
            if !kind.is_dir() || kind.is_symlink() || skipped(&entry.file_name().to_string_lossy())
            {
                continue;
            }
            if depth >= 12 {
                report.refresh.complete = false;
                note(
                    &mut report,
                    "Discovery depth limit reached",
                    &entry.path(),
                    &root,
                );
                continue;
            }
            if entry.path().to_str().is_none() {
                report.refresh.complete = false;
                note(&mut report, "Non-UTF-8 directory excluded", &path, &root);
                continue;
            }
            children.push(entry.path());
        }
        children.sort();
        queue.extend(children.into_iter().map(|path| (path, depth + 1)));
    }
    report
        .refresh
        .checkouts
        .sort_by(|a, b| a.local_path.cmp(&b.local_path));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    async fn run(path: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(path)
            .args([
                "-c",
                "user.name=Recollect fixture",
                "-c",
                "user.email=fixture@example.test",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .unwrap();
        assert!(status.success());
    }
    #[tokio::test]
    async fn local_discovery_honors_boundaries_worktrees_and_dirty_state_without_writes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache")
            .join(format!("workspace-fixture-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".recollect")).await.unwrap();
        let root = fs::canonicalize(root).await.unwrap();
        let _owned = Fixture(root.clone());
        fs::write(root.join(".recollect/workspace.toml"), "brain = 'Parent'\n")
            .await
            .unwrap();
        let repo = root.join("infra");
        fs::create_dir_all(repo.join("deep")).await.unwrap();
        run(&repo, &["init", "-q"]).await;
        run(
            &repo,
            &[
                "remote",
                "add",
                "origin",
                "https://member:discard@example.test/Team/infra.git",
            ],
        )
        .await;
        fs::write(repo.join("evidence.txt"), "Committed fixture\n")
            .await
            .unwrap();
        run(&repo, &["add", "evidence.txt"]).await;
        run(&repo, &["commit", "-qm", "Fixture baseline"]).await;
        let worktree = root.join("worktree");
        run(
            &repo,
            &["worktree", "add", "--detach", worktree.to_str().unwrap()],
        )
        .await;
        fs::write(repo.join("evidence.txt"), "Dirty fixture\n")
            .await
            .unwrap();
        fs::create_dir_all(root.join("nested/.recollect"))
            .await
            .unwrap();
        fs::write(
            root.join("nested/.recollect/workspace.toml"),
            "brain = 'Nested'\n",
        )
        .await
        .unwrap();
        fs::create_dir_all(root.join("nested/excluded"))
            .await
            .unwrap();
        run(&root.join("nested/excluded"), &["init", "-q"]).await;
        fs::create_dir_all(root.join("unborn")).await.unwrap();
        run(&root.join("unborn"), &["init", "-q"]).await;
        #[cfg(unix)]
        std::os::unix::fs::symlink(&root, root.join("cycle")).unwrap();
        let before_index = fs::read(repo.join(".git/index")).await.unwrap();
        let before_config = fs::read(repo.join(".git/config")).await.unwrap();
        let report = discover(&repo.join("deep")).await.unwrap();
        assert_eq!(report.brain_selector, "Parent");
        assert!(report.refresh.complete);
        assert_eq!(report.refresh.checkouts.len(), 3);
        assert_eq!(report.excluded_workspaces.len(), 1);
        let dirty = report
            .refresh
            .checkouts
            .iter()
            .find(|c| c.local_path == repo.to_str().unwrap())
            .unwrap();
        assert_eq!(dirty.origin.as_deref(), Some("example.test/Team/infra"));
        assert_eq!(dirty.dirty, Some(true));
        assert!(dirty.head.is_some());
        assert_eq!(
            report
                .refresh
                .checkouts
                .iter()
                .find(|c| c.local_path.ends_with("worktree"))
                .unwrap()
                .dirty,
            Some(false)
        );
        assert_eq!(
            report
                .refresh
                .checkouts
                .iter()
                .find(|c| c.local_path.ends_with("unborn"))
                .unwrap()
                .status,
            "no_origin"
        );
        assert!(!serde_json::to_string(&report).unwrap().contains("discard"));
        assert_eq!(
            fs::read(repo.join(".git/index")).await.unwrap(),
            before_index
        );
        assert_eq!(
            fs::read(repo.join(".git/config")).await.unwrap(),
            before_config
        );
        assert_eq!(
            fs::read_to_string(repo.join("evidence.txt")).await.unwrap(),
            "Dirty fixture\n"
        );
        assert_eq!(
            selector(&root.join("nested/excluded")).await.unwrap().1,
            "Nested"
        );
        fs::write(
            root.join("nested/.recollect/workspace.toml"),
            "brain = 1\nsecret = 'discard'\n",
        )
        .await
        .unwrap();
        let error = selector(&root.join("nested/excluded"))
            .await
            .unwrap_err()
            .to_string();
        assert!(!error.contains("discard"));
        assert_eq!(discover(&repo).await.unwrap().excluded_workspaces.len(), 1);
    }
}
