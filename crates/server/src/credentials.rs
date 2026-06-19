//! Local-development credentials under ADR 0003. Never log this file or its values.
use crate::error::{Error, Result};
use axum::http::StatusCode;
use std::{collections::BTreeMap, io::Write, path::Path, sync::LazyLock};
use uuid::Uuid;

static LOCK: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));
fn unavailable() -> Error {
    Error(
        StatusCode::SERVICE_UNAVAILABLE,
        "credentials_unavailable",
        "Local credentials are unavailable. Ask the installation owner to check the credential file.",
    )
}
fn read(path: &Path, allow_missing: bool) -> Result<BTreeMap<Uuid, String>> {
    match std::fs::read(path) {
        Ok(bytes) if bytes.len() <= 8 * 1024 * 1024 => {
            serde_json::from_slice(&bytes).map_err(|_| unavailable())
        }
        Err(e) if allow_missing && e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        _ => Err(unavailable()),
    }
}
pub async fn matches(path: &str, id: Uuid, password: &str) -> Result<bool> {
    let (path, password) = (path.to_owned(), password.to_owned());
    let _guard = LOCK.lock().await;
    tokio::task::spawn_blocking(move || {
        Ok(read(Path::new(&path), false)?.get(&id) == Some(&password))
    })
    .await
    .map_err(|_| unavailable())?
}
pub async fn store(path: &str, id: Uuid, password: &str) -> Result<()> {
    let (path, password) = (path.to_owned(), password.to_owned());
    let _guard = LOCK.lock().await;
    tokio::task::spawn_blocking(move || {
        use std::os::unix::fs::OpenOptionsExt;
        let path = Path::new(&path);
        let mut values = read(path, true)?;
        values.insert(id, password);
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|_| unavailable())?;
        let temp = parent.join(format!(".credentials-{}.tmp", Uuid::new_v4()));
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&temp)
                .map_err(|_| unavailable())?;
            file.write_all(&serde_json::to_vec(&values).map_err(|_| unavailable())?)
                .map_err(|_| unavailable())?;
            file.sync_all().map_err(|_| unavailable())?;
            std::fs::rename(&temp, path).map_err(|_| unavailable())?;
            std::fs::File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|_| unavailable())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temp);
        }
        result
    })
    .await
    .map_err(|_| unavailable())?
}
