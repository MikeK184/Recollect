//! Local credential files. Never log these files or any value in them.
//!
//! Invited local account passwords are stored only as Argon2id PHC strings
//! (ADR 0024); the installation owner's password stays in the environment and
//! is not stored here. `store_private_many` keeps the separate private MCP
//! provider secrets, which are delivered upstream and so cannot be hashed.
//! Every read uses the MCP runtime's strict secret-file gate: a regular file,
//! never a symlink, no group or other permission bits, owned by this user.
use crate::error::{Error, Result};
use argon2::{
    Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version, password_hash::phc,
};
use axum::http::StatusCode;
use recollect_mcp_runtime::credentials::read_local_values;
use std::{
    collections::BTreeMap,
    io::Write,
    path::Path,
    sync::{Arc, LazyLock},
};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use uuid::Uuid;

static LOCK: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));
/// Each Argon2id operation holds about 19 MiB. Bound the concurrent ones so a
/// burst of sign-ins queues instead of exhausting memory.
static HASHING: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(4)));
/// Verified against when no real hash exists, so a sign-in for an unknown,
/// disabled or unprovisioned account takes as long as one for a real account.
static DUMMY: LazyLock<String> = LazyLock::new(|| {
    hash(&Uuid::new_v4().to_string())
        .unwrap_or_else(|_| panic!("fixed Argon2 parameters hash a generated value"))
});
const MAX_FILE: usize = 8 * 1024 * 1024;

fn unavailable() -> Error {
    Error(
        StatusCode::SERVICE_UNAVAILABLE,
        "credentials_unavailable",
        "Local credentials are unavailable. Ask the installation owner to check the credential file.",
    )
}
fn read(path: &Path, allow_missing: bool) -> Result<BTreeMap<Uuid, String>> {
    read_local_values(path, allow_missing).map_err(|_| unavailable())
}

/// Argon2id version 19 at the OWASP baseline (19 MiB, two passes, one lane).
/// Stated here rather than taken from the crate default so that a dependency
/// update cannot silently weaken newly stored hashes. Verification uses the
/// parameters embedded in each stored string.
fn argon2() -> Argon2<'static> {
    let params = Params::new(19 * 1024, 2, 1, Some(32)).expect("fixed Argon2 parameters are valid");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}
fn hash(password: &str) -> Result<String> {
    argon2()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|_| unavailable())
}
/// An entry is a hash exactly when it parses as an Argon2id PHC string;
/// anything else is a legacy plaintext password awaiting conversion.
fn is_hash(stored: &str) -> bool {
    stored.starts_with("$argon2id$") && phc::PasswordHash::new(stored).is_ok()
}
fn verify(stored: &str, password: &str) -> bool {
    argon2()
        .verify_password(password.as_bytes(), stored)
        .is_ok()
}
/// Run CPU- and memory-heavy work off the async threads, bounded by `HASHING`.
/// The permit moves into the task: dropping a cancelled caller does not stop
/// work already running, so it must keep counting against the bound.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T> {
    let permit = HASHING
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| unavailable())?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await
    .map_err(|_| unavailable())
}

/// Whether `password` matches the stored credential. `Ok(false)` is a wrong
/// password or a missing entry; an unreadable or insecure file is a 503.
pub async fn matches(path: &str, id: Uuid, password: &str) -> Result<bool> {
    let stored = {
        let _guard = LOCK.lock().await;
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || read(Path::new(&path), false))
            .await
            .map_err(|_| unavailable())??
    }
    .remove(&id);
    let Some(stored) = stored else {
        burn(password).await;
        return Ok(false);
    };
    if is_hash(&stored) {
        let password = password.to_owned();
        return blocking(move || verify(&stored, &password)).await;
    }
    // A legacy plaintext entry that startup conversion did not reach.
    let matched = bool::from(stored.as_bytes().ct_eq(password.as_bytes()));
    if matched && let Err(Error(_, code, _)) = convert(path, Some(id)).await {
        tracing::warn!(
            code,
            "Could not upgrade a legacy local credential at sign-in"
        );
    }
    Ok(matched)
}
/// Spend one verification's time without a real account. The result is
/// meaningless and the dummy hash matches nothing a caller can send.
pub async fn burn(password: &str) {
    let password = password.to_owned();
    let _ = blocking(move || verify(&DUMMY, &password)).await;
}

/// Hash and store one invited account's password under its credential ID.
pub async fn store(path: &str, id: Uuid, password: &str) -> Result<()> {
    let password = password.to_owned();
    let hashed = blocking(move || hash(&password)).await??;
    store_values(path, vec![(id, hashed)]).await
}
/// Store private MCP provider secrets verbatim: they are sent to upstream
/// servers, so unlike account passwords they cannot be one-way hashed.
pub async fn store_private_many(path: &str, additions: Vec<(Uuid, String)>) -> Result<()> {
    store_values(path, additions).await
}
/// Convert every legacy plaintext entry to an Argon2id hash in one atomic
/// replacement and return how many changed. A missing file has none.
pub async fn upgrade_legacy(path: &str) -> Result<usize> {
    convert(path, None).await
}

async fn store_values(path: &str, additions: Vec<(Uuid, String)>) -> Result<()> {
    let path = path.to_owned();
    let _guard = LOCK.lock().await;
    tokio::task::spawn_blocking(move || {
        let path = Path::new(&path);
        let mut values = read(path, true)?;
        values.extend(additions);
        write(path, &values)
    })
    .await
    .map_err(|_| unavailable())?
}
async fn convert(path: &str, only: Option<Uuid>) -> Result<usize> {
    let path = path.to_owned();
    let _guard = LOCK.lock().await;
    blocking(move || {
        let path = Path::new(&path);
        let mut values = read(path, true)?;
        let mut converted = 0;
        for (id, stored) in &mut values {
            if is_hash(stored) || only.is_some_and(|only| only != *id) {
                continue;
            }
            *stored = hash(stored)?;
            converted += 1;
        }
        if converted > 0 {
            write(path, &values)?;
        }
        Ok(converted)
    })
    .await?
}
/// Replace the file atomically: a private temporary file, flushed to disk, is
/// renamed over the target and then the directory entry is flushed.
fn write(path: &Path, values: &BTreeMap<Uuid, String>) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let bytes = serde_json::to_vec(values).map_err(|_| unavailable())?;
    if bytes.len() > MAX_FILE {
        return Err(unavailable());
    }
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
        file.write_all(&bytes).map_err(|_| unavailable())?;
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
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct Directory(std::path::PathBuf);
    impl Directory {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("recollect-credentials-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn file(&self) -> String {
            self.0
                .join("credentials.json")
                .to_string_lossy()
                .into_owned()
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn mode(path: &str) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }
    fn legacy(path: &str, entries: &[(Uuid, &str)]) {
        let values: BTreeMap<Uuid, &str> = entries.iter().copied().collect();
        std::fs::write(path, serde_json::to_vec(&values).unwrap()).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn stored(path: &str) -> BTreeMap<Uuid, String> {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn stored_passwords_are_salted_argon2id_hashes_that_verify() {
        let directory = Directory::new();
        let path = directory.file();
        let (first, second) = (Uuid::new_v4(), Uuid::new_v4());
        store(&path, first, "correct horse battery").await.unwrap();
        store(&path, second, "correct horse battery").await.unwrap();
        let values = stored(&path);
        for hash in values.values() {
            assert!(
                hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
                "{hash}"
            );
            assert!(!hash.contains("correct horse"));
        }
        assert_ne!(
            values[&first], values[&second],
            "each entry has its own salt"
        );
        assert_eq!(mode(&path), 0o600);
        assert!(
            matches(&path, first, "correct horse battery")
                .await
                .unwrap()
        );
        assert!(!matches(&path, first, "correct horse batter").await.unwrap());
        assert!(
            !matches(&path, Uuid::new_v4(), "correct horse battery")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn startup_conversion_hashes_every_legacy_entry_once() {
        let directory = Directory::new();
        let path = directory.file();
        let (one, two, three) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        store(&path, three, "already hashed").await.unwrap();
        let mut values = stored(&path);
        values.insert(one, "plain one pass".into());
        values.insert(two, "plain two pass".into());
        std::fs::write(&path, serde_json::to_vec(&values).unwrap()).unwrap();
        assert_eq!(upgrade_legacy(&path).await.unwrap(), 2);
        let values = stored(&path);
        assert!(values.values().all(|value| is_hash(value)));
        assert!(
            !std::fs::read_to_string(&path)
                .unwrap()
                .contains("plain one")
        );
        assert_eq!(mode(&path), 0o600);
        assert!(matches(&path, one, "plain one pass").await.unwrap());
        assert!(matches(&path, two, "plain two pass").await.unwrap());
        assert!(!matches(&path, two, "plain one pass").await.unwrap());
        assert!(matches(&path, three, "already hashed").await.unwrap());
        let before = std::fs::read(&path).unwrap();
        assert_eq!(upgrade_legacy(&path).await.unwrap(), 0);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "a second pass rewrites nothing"
        );
        assert_eq!(
            upgrade_legacy(&directory.file().replace("credentials", "absent"))
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn a_legacy_entry_is_accepted_and_upgraded_at_sign_in() {
        let directory = Directory::new();
        let path = directory.file();
        let (one, two) = (Uuid::new_v4(), Uuid::new_v4());
        legacy(&path, &[(one, "plain one pass"), (two, "plain two pass")]);
        assert!(!matches(&path, one, "wrong one pass").await.unwrap());
        assert!(
            stored(&path).values().all(|value| !is_hash(value)),
            "a wrong password converts nothing"
        );
        assert!(matches(&path, one, "plain one pass").await.unwrap());
        let values = stored(&path);
        assert!(is_hash(&values[&one]));
        assert_eq!(
            values[&two], "plain two pass",
            "only the signing-in entry changes"
        );
        assert_eq!(mode(&path), 0o600);
        assert!(matches(&path, one, "plain one pass").await.unwrap());
        assert!(!matches(&path, one, "wrong one pass").await.unwrap());
    }

    #[tokio::test]
    async fn loose_permissions_a_symlink_or_corruption_fail_closed() {
        let directory = Directory::new();
        let path = directory.file();
        let id = Uuid::new_v4();
        store(&path, id, "a good password").await.unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        for result in [
            matches(&path, id, "a good password").await.map(|_| ()),
            store(&path, Uuid::new_v4(), "another one").await,
            upgrade_legacy(&path).await.map(|_| ()),
        ] {
            assert_eq!(result.unwrap_err().1, "credentials_unavailable");
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(matches(&path, id, "a good password").await.unwrap());
        let link = directory.0.join("link.json").to_string_lossy().into_owned();
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert_eq!(
            matches(&link, id, "a good password").await.unwrap_err().0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        std::fs::write(&path, b"not json").unwrap();
        assert_eq!(
            matches(&path, id, "a good password").await.unwrap_err().0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            matches(&directory.file().replace("credentials", "absent"), id, "x")
                .await
                .unwrap_err()
                .0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn private_mcp_secrets_stay_verbatim_in_a_private_file() {
        let directory = Directory::new();
        let path = directory.file();
        let id = Uuid::new_v4();
        store_private_many(&path, vec![(id, "upstream-token-value".into())])
            .await
            .unwrap();
        assert_eq!(stored(&path)[&id], "upstream-token-value");
        assert_eq!(mode(&path), 0o600);
    }
}
