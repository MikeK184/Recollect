//! Plugin-owned portable storage. Never depend on the build checkout at runtime.
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub endpoint: String,
    pub brain: Uuid,
    pub device: Uuid,
    pub profile: String,
    #[serde(default)]
    pub with_runner: bool,
    #[serde(default)]
    pub runner_id: Option<Uuid>,
}

pub fn root() -> Result<PathBuf> {
    let path = if let Some(path) = std::env::var_os("RECOLLECT_PLUGIN_DATA") {
        PathBuf::from(path)
    } else {
        let home = PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| anyhow!("plugin_data_unavailable"))?,
        );
        if cfg!(target_os = "macos") {
            home.join("Library/Application Support/Recollect/plugin")
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/share"))
                .join("recollect/plugin")
        }
    };
    ensure!(
        path.is_absolute()
            && path.components().all(|c| matches!(
                c,
                Component::RootDir | Component::Prefix(_) | Component::Normal(_)
            )),
        "invalid_plugin_data_directory"
    );
    Ok(path)
}

/// Check every component without following links; create only missing directories.
pub fn directory(path: &Path) -> Result<()> {
    check_directory(path, true)
}

fn check_directory(path: &Path, create: bool) -> Result<()> {
    ensure!(path.is_absolute(), "invalid_plugin_directory");
    let mut current = PathBuf::new();
    for part in path.components() {
        ensure!(
            !matches!(part, Component::ParentDir | Component::CurDir),
            "invalid_plugin_directory"
        );
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) => ensure!(
                meta.is_dir() && !meta.is_symlink(),
                "unsafe_plugin_directory"
            ),
            Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(0o700);
                }
                if let Err(error) = builder.create(&current) {
                    ensure!(
                        error.kind() == std::io::ErrorKind::AlreadyExists,
                        "plugin_directory_unavailable"
                    );
                    let meta = fs::symlink_metadata(&current)?;
                    ensure!(
                        meta.is_dir() && !meta.is_symlink(),
                        "unsafe_plugin_directory"
                    );
                }
            }
            Err(_) => return Err(anyhow!("plugin_directory_unavailable")),
        }
    }
    private_permissions(&fs::metadata(path)?)?;
    Ok(())
}

fn private_permissions(meta: &fs::Metadata) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            meta.permissions().mode() & 0o077 == 0,
            "plugin_storage_not_private"
        );
    }
    Ok(())
}

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    check_directory(
        path.parent()
            .ok_or_else(|| anyhow!("invalid_plugin_state"))?,
        false,
    )?;
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        meta.is_file() && !meta.is_symlink() && meta.len() <= 65536,
        "invalid_plugin_state"
    );
    private_permissions(&meta)?;
    serde_json::from_slice(&fs::read(path)?).map_err(|_| anyhow!("invalid_plugin_state"))
}

pub fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("invalid_plugin_state"))?;
    directory(parent)?;
    if let Ok(meta) = fs::symlink_metadata(path) {
        ensure!(meta.is_file() && !meta.is_symlink(), "unsafe_plugin_state");
    }
    let tmp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let bytes = serde_json::to_vec(value)?;
    let mut file = private_file(&tmp, true)?;
    use std::io::Write;
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn private_file(path: &Path, new: bool) -> Result<fs::File> {
    check_directory(
        path.parent()
            .ok_or_else(|| anyhow!("invalid_plugin_state"))?,
        false,
    )?;
    if let Ok(meta) = fs::symlink_metadata(path) {
        ensure!(meta.is_file() && !meta.is_symlink(), "unsafe_plugin_state");
        private_permissions(&meta)?;
    }
    let mut opts = fs::OpenOptions::new();
    opts.read(true).write(true);
    if new {
        opts.create_new(true);
    } else {
        opts.create(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    Ok(opts.open(path)?)
}

pub fn config() -> Result<Config> {
    let config: Config = read(&root()?.join("config.json"))?;
    crate::Client::new(&config.endpoint)?;
    ensure!(
        !config.brain.is_nil() && !config.device.is_nil() && !config.profile.is_empty(),
        "invalid_plugin_configuration"
    );
    Ok(config)
}

/// Retain only credential references for original queue destinations. Reconnecting
/// never migrates their events or overwrites another device's credential slot.
pub fn remember_destination(config: &Config) -> Result<()> {
    let path = root()?
        .join("destinations")
        .join(format!("{}.json", config.device));
    if path.try_exists()? {
        let previous: Config = read(&path)?;
        ensure!(
            previous.endpoint == config.endpoint
                && previous.device == config.device
                && previous.profile == config.profile,
            "plugin_destination_conflict"
        );
    }
    let mut saved = config.clone();
    saved.with_runner = false;
    saved.runner_id = None;
    write(&path, &saved)
}

pub fn destinations() -> Result<Vec<Config>> {
    let current = config()?;
    let mut result = vec![current.clone()];
    let directory = root()?.join("destinations");
    if directory.try_exists()? {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.extension().and_then(|v| v.to_str()) != Some("json") {
                continue;
            }
            let saved: Config = read(&path)?;
            crate::Client::new(&saved.endpoint)?;
            ensure!(
                path.file_stem().and_then(|v| v.to_str()) == Some(&saved.device.to_string()),
                "plugin_destination_conflict"
            );
            if saved.device != current.device || saved.endpoint != current.endpoint {
                result.push(saved);
            }
        }
    }
    Ok(result)
}

/// The legacy Codex MCP loader does not expand plugin-root variables. Its small
/// POSIX bootstrap reads this private pointer written by the installed runtime.
/// No PATH installation or source checkout is required, and shell metacharacters
/// in paths remain data (the bootstrap uses a quoted exec, never eval).
pub fn runtime_pointer() -> Result<()> {
    let root = root()?;
    directory(&root)?;
    let executable = std::env::current_exe()?.canonicalize()?;
    let path = executable
        .to_str()
        .ok_or_else(|| anyhow!("invalid_plugin_runtime"))?;
    ensure!(
        executable.is_absolute() && !path.chars().any(char::is_control),
        "invalid_plugin_runtime"
    );
    let temporary = root.join(format!("runtime-{}.tmp", Uuid::new_v4()));
    let mut file = private_file(&temporary, true)?;
    use std::io::Write;
    writeln!(file, "{path}")?;
    file.sync_all()?;
    let destination = root.join("runtime-path");
    if let Ok(meta) = fs::symlink_metadata(&destination) {
        ensure!(
            meta.is_file() && !meta.is_symlink(),
            "unsafe_plugin_runtime"
        );
    }
    fs::rename(temporary, destination)?;
    fs::File::open(&root)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_state_rejects_symlinks_and_preserves_previous_value() {
        let root = crate::publication::project_root()
            .join(".cache")
            .join(format!("plugin-storage-{}", Uuid::new_v4()));
        directory(&root).unwrap();
        let path = root.join("state.json");
        write(&path, &serde_json::json!({"a":1})).unwrap();
        write(&path, &serde_json::json!({"a":2})).unwrap();
        assert_eq!(read::<serde_json::Value>(&path).unwrap()["a"], 2);
        #[cfg(unix)]
        {
            let link = root.join("link.json");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(write(&link, &serde_json::json!({"a":3})).is_err());
            assert!(read::<serde_json::Value>(&link).is_err());
            assert_eq!(read::<serde_json::Value>(&path).unwrap()["a"], 2);
            let folder = root.join("linked-parent");
            std::os::unix::fs::symlink(&root, &folder).unwrap();
            assert!(read::<serde_json::Value>(&folder.join("state.json")).is_err());
            assert!(private_file(&folder.join("other.json"), true).is_err());
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(read::<serde_json::Value>(&path).is_err());
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
            assert!(read::<serde_json::Value>(&path).is_err());
            assert!(directory(&root).is_err());
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}
