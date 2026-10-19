use crate::{Result, RuntimeError};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{io::AsyncReadExt, sync::Mutex};
use uuid::Uuid;
#[cfg(test)]
mod tests_live;
mod vault;

/// Private development values only; bounded, no symlinks and current owner permissions.
pub fn read_local_values(
    path: &std::path::Path,
    allow_missing: bool,
) -> Result<BTreeMap<Uuid, String>> {
    #[cfg(unix)]
    {
        use std::{
            io::Read,
            os::unix::fs::{MetadataExt, OpenOptionsExt},
        };
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_NOFOLLOW)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeMap::new());
            }
            Err(_) => return Err(RuntimeError("credential_unavailable")),
        };
        let metadata = file
            .metadata()
            .map_err(|_| RuntimeError("credential_unavailable"))?;
        if !metadata.is_file()
            || metadata.mode() & 0o077 != 0
            || metadata.uid() != nix::unistd::Uid::effective().as_raw()
        {
            return Err(RuntimeError("credential_file_invalid"));
        }
        let mut bytes = Vec::new();
        file.take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| RuntimeError("credential_unavailable"))?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(RuntimeError("credential_file_invalid"));
        }
        serde_json::from_slice(&bytes).map_err(|_| RuntimeError("credential_file_invalid"))
    }
    #[cfg(not(unix))]
    Err(RuntimeError("credential_file_unsupported"))
}

#[derive(Clone, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case", deny_unknown_fields)]
pub enum SecretReference {
    Environment { name: String },
    OsStore { account: String },
    Vault { source: String, field: String },
    LocalFile { path: PathBuf, key: Uuid },
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeaderBinding {
    pub source: SecretReference,
    #[serde(default)]
    pub prefix: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialBinding {
    pub connection_id: Uuid,
    pub alias: String,
    pub runner_reference: String,
    #[serde(default)]
    pub environment: BTreeMap<String, SecretReference>,
    #[serde(default)]
    pub headers: BTreeMap<String, HeaderBinding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    bindings: Vec<CredentialBinding>,
    #[serde(default)]
    vault_sources: BTreeMap<String, vault::Source>,
}

/// Deliberately has no Debug/Serialize implementation. Values stay on the runner.
pub struct ResolvedCredentials {
    pub generation: Uuid,
    leases: Vec<Arc<vault::Lease>>,
    ended: AtomicBool,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) secrets: Vec<String>,
    /// The resolver cache entry this resolution may be dropped from.
    origin: Option<Origin>,
}
struct Origin {
    cache: Weak<Cache>,
    key: Key,
}

impl ResolvedCredentials {
    pub fn anonymous() -> Arc<Self> {
        Arc::new(Self {
            generation: Uuid::nil(),
            leases: vec![],
            ended: AtomicBool::new(false),
            environment: BTreeMap::new(),
            headers: BTreeMap::new(),
            secrets: vec![],
            origin: None,
        })
    }
    /// An upstream refused these credentials (HTTP 401 or 403). Drop this
    /// connection's cached resolution so the next operation resolves again and
    /// gets a fresh generation, even when the stored value has not changed, and
    /// no running instance keeps serving a refused credential. The call that met
    /// the rejection is not retried.
    pub(crate) async fn rejected(&self) {
        let Some(origin) = &self.origin else { return };
        let Some(cache) = origin.cache.upgrade() else {
            return;
        };
        let mut cache = cache.lock().await;
        if cache
            .get(&origin.key)
            .is_some_and(|current| current.generation == self.generation)
        {
            cache.remove(&origin.key);
        }
    }
    pub fn sanitize(&self, value: &serde_json::Value) -> serde_json::Value {
        crate::sanitize(value, &self.secrets)
    }
    pub fn valid(&self) -> bool {
        !self.ended.load(Ordering::Acquire) && self.leases.iter().all(|lease| lease.valid())
    }
    pub(crate) fn reusable(&self) -> bool {
        self.valid() && self.leases.iter().all(|lease| lease.reusable())
    }
    pub async fn expired(&self) {
        if !self.valid() {
            return;
        }
        if self.leases.is_empty() {
            return std::future::pending().await;
        }
        futures::future::select_all(self.leases.iter().map(|lease| Box::pin(lease.expired())))
            .await;
    }
    /// The operation owns renewal, including during startup. Dropping it stops
    /// renewal and prevents reuse, but never revokes a Vault token or secret lease.
    pub fn operation<F, Fut>(self: &Arc<Self>, authorize: F) -> CredentialOperation
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        let renewal = if self.leases.is_empty() {
            None
        } else {
            let credentials = self.clone();
            Some(tokio::spawn(async move {
                futures::future::join_all(
                    credentials
                        .leases
                        .iter()
                        .map(|lease| lease.renew(&authorize)),
                )
                .await;
            }))
        };
        CredentialOperation {
            credentials: self.clone(),
            renewal,
        }
    }
}
pub struct CredentialOperation {
    credentials: Arc<ResolvedCredentials>,
    renewal: Option<tokio::task::JoinHandle<()>>,
}
impl Drop for CredentialOperation {
    fn drop(&mut self) {
        if let Some(renewal) = &self.renewal {
            self.credentials.ended.store(true, Ordering::Release);
            renewal.abort();
        }
    }
}
type Key = (Uuid, String, String);
type Cache = Mutex<BTreeMap<Key, Arc<ResolvedCredentials>>>;
/// Every operation resolves each secret again: environment values cannot change
/// without a restart, OS-store lookups take milliseconds, local reference files
/// microseconds, and Vault is never cached. The cache holds only the latest
/// resolution of each connection so that an unchanged value returns the same
/// object (and generation) after the read; a running instance is then not
/// restarted for nothing, while a rotated value yields a fresh generation.
pub struct CredentialResolver {
    path: Option<PathBuf>,
    cache: Arc<Cache>,
}
impl CredentialResolver {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            cache: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
    /// Forget the cached resolutions so the next resolve returns a fresh
    /// generation even for unchanged values. Rotation does not need this: every
    /// resolve already reads each secret source.
    pub async fn invalidate(&self) {
        self.cache.lock().await.clear();
    }
    pub async fn resolve(
        &self,
        connection: Uuid,
        alias: Option<&str>,
        runner: &str,
    ) -> Result<Arc<ResolvedCredentials>> {
        self.resolve_with(connection, alias, runner, || async { Ok(()) })
            .await
    }
    pub async fn resolve_with<F, Fut>(
        &self,
        connection: Uuid,
        alias: Option<&str>,
        runner: &str,
        authorize: F,
    ) -> Result<Arc<ResolvedCredentials>>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        authorize().await?;
        let Some(alias) = alias else {
            return Ok(ResolvedCredentials::anonymous());
        };
        let path = self
            .path
            .as_ref()
            .ok_or(RuntimeError("credential_binding_missing"))?;
        let key = (connection, alias.to_owned(), runner.to_owned());
        let file = tokio::fs::File::open(path)
            .await
            .map_err(|_| RuntimeError("credential_binding_unavailable"))?;
        if !file
            .metadata()
            .await
            .map_err(|_| RuntimeError("credential_binding_unavailable"))?
            .is_file()
        {
            return Err(RuntimeError("credential_binding_invalid"));
        }
        let mut bytes = vec![];
        file.take(131073)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| RuntimeError("credential_binding_unavailable"))?;
        if bytes.len() > 131072 {
            return Err(RuntimeError("credential_binding_invalid"));
        }
        let file: File = serde_json::from_slice(&bytes)
            .map_err(|_| RuntimeError("credential_binding_invalid"))?;
        if file.bindings.len() > 1000 || file.vault_sources.len() > 128 {
            return Err(RuntimeError("credential_binding_invalid"));
        }
        let mut matches = file.bindings.into_iter().filter(|b| {
            b.connection_id == connection && b.alias == alias && b.runner_reference == runner
        });
        let binding = matches
            .next()
            .ok_or(RuntimeError("credential_binding_missing"))?;
        if matches.next().is_some() || binding.environment.len() + binding.headers.len() > 32 {
            return Err(RuntimeError("credential_binding_invalid"));
        }
        let mut sources = BTreeSet::new();
        for reference in binding
            .environment
            .values()
            .chain(binding.headers.values().map(|h| &h.source))
        {
            if let SecretReference::Vault { source, field } = reference {
                if source.is_empty()
                    || source.len() > 120
                    || field.is_empty()
                    || field.len() > 120
                    || source.chars().chain(field.chars()).any(char::is_control)
                {
                    return Err(RuntimeError("credential_reference_invalid"));
                }
                sources.insert(source.clone());
            }
        }
        if sources.len() > 8 {
            return Err(RuntimeError("credential_binding_invalid"));
        }
        let mut acquired = BTreeMap::new();
        for name in sources {
            let source = file
                .vault_sources
                .get(&name)
                .ok_or(RuntimeError("credential_binding_missing"))?;
            authorize().await?;
            acquired.insert(name, source.acquire().await?);
        }
        let mut resolved = ResolvedCredentials {
            generation: Uuid::new_v4(),
            leases: acquired.values().filter_map(|a| a.lease.clone()).collect(),
            ended: AtomicBool::new(false),
            environment: BTreeMap::new(),
            headers: BTreeMap::new(),
            secrets: vec![],
            origin: Some(Origin {
                cache: Arc::downgrade(&self.cache),
                key: key.clone(),
            }),
        };
        for (name, reference) in binding.environment {
            if !environment_destination(&name) {
                return Err(RuntimeError("credential_destination_invalid"));
            }
            let value = resolve(reference, &acquired).await?;
            resolved.secrets.push(value.clone());
            resolved.environment.insert(name, value);
        }
        for (name, reference) in binding.headers {
            let name = name.to_ascii_lowercase();
            if !header_destination(&name) || !matches!(reference.prefix.as_str(), "" | "Bearer ") {
                return Err(RuntimeError("credential_destination_invalid"));
            }
            let value = resolve(reference.source, &acquired).await?;
            let delivered = format!("{}{}", reference.prefix, value);
            if http::HeaderValue::from_str(&delivered).is_err()
                || resolved.headers.contains_key(&name)
            {
                return Err(RuntimeError("credential_destination_invalid"));
            }
            resolved.secrets.push(value);
            resolved.headers.insert(name, delivered);
        }
        if !resolved.valid() {
            return Err(RuntimeError("credential_expired"));
        }
        // A dynamic lease belongs to this operation, even if an issuer happens
        // to return the same bytes. Never share it with another caller or call.
        if !resolved.leases.is_empty() {
            return Ok(Arc::new(resolved));
        }
        // The same values keep the same object, so a running compatible
        // instance is not restarted; a rotated value gets a fresh generation.
        // An upstream-rejected resolution is dropped by `rejected`, so it is
        // never found here and the unchanged value still gets a fresh one.
        let mut cache = self.cache.lock().await;
        if let Some(old) = cache.get(&key) {
            if old.environment == resolved.environment && old.headers == resolved.headers {
                return Ok(old.clone());
            }
        } else if cache.len() >= 1000 {
            return Err(RuntimeError("credential_cache_capacity"));
        }
        let resolved = Arc::new(resolved);
        cache.insert(key, resolved.clone());
        Ok(resolved)
    }
}

fn variable(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 120
        && name
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        && !name.as_bytes()[0].is_ascii_digit()
}
pub fn environment_destination(name: &str) -> bool {
    variable(name)
        && !name.starts_with("RECOLLECT_")
        && !name.starts_with("DYLD_")
        && !name.starts_with("LD_")
        && !name.starts_with("DATABASE_")
        && !matches!(
            name,
            "PATH"
                | "HOME"
                | "BASH_ENV"
                | "ENV"
                | "NODE_OPTIONS"
                | "NODE_PATH"
                | "RUBYOPT"
                | "RUBYLIB"
                | "PERL5OPT"
                | "PERL5LIB"
                | "PERLLIB"
                | "JAVA_TOOL_OPTIONS"
                | "JDK_JAVA_OPTIONS"
                | "_JAVA_OPTIONS"
                | "GEM_HOME"
                | "GEM_PATH"
                | "SHELLOPTS"
                | "BASHOPTS"
                | "IFS"
                | "CDPATH"
                | "PYTHONPATH"
                | "PYTHONHOME"
                | "RUST_LOG"
                | "POSTGRES_PASSWORD"
        )
}
pub fn header_destination(name: &str) -> bool {
    http::HeaderName::from_bytes(name.as_bytes()).is_ok()
        && !name.starts_with("mcp-")
        && !name.starts_with("proxy-")
        && !name.starts_with("x-forwarded-")
        && !matches!(
            name,
            "host"
                | "forwarded"
                | "x-original-url"
                | "x-rewrite-url"
                | "cookie"
                | "set-cookie"
                | "content-type"
                | "content-length"
                | "accept"
                | "connection"
                | "transfer-encoding"
                | "origin"
                | "referer"
                | "last-event-id"
                | "upgrade"
                | "te"
                | "trailer"
        )
}

async fn resolve(
    reference: SecretReference,
    acquired: &BTreeMap<String, vault::Acquired>,
) -> Result<String> {
    let value = match reference {
        SecretReference::Environment { name } => {
            if !variable(&name)
                || (name.starts_with("RECOLLECT_") && !name.starts_with("RECOLLECT_MCP_SECRET_"))
                || name.starts_with("DATABASE_")
                || matches!(
                    name.as_str(),
                    "POSTGRES_PASSWORD" | "NEO4J_PASSWORD" | "VAULT_TOKEN"
                )
            {
                return Err(RuntimeError("credential_reference_invalid"));
            }
            std::env::var(name).map_err(|_| RuntimeError("credential_unavailable"))?
        }
        SecretReference::OsStore { account } => {
            if account.is_empty() || account.len() > 200 || account.chars().any(char::is_control) {
                return Err(RuntimeError("credential_reference_invalid"));
            }
            // Keep the permit inside the blocking operation: dropping a timed
            // out caller does not stop an OS-store lookup already in progress.
            static OS_STORE: std::sync::OnceLock<Arc<tokio::sync::Semaphore>> =
                std::sync::OnceLock::new();
            let permit = OS_STORE
                .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(4)))
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| RuntimeError("credential_unavailable"))?;
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                keyring::Entry::new("recollect-mcp", &account)
                    .and_then(|e| e.get_password())
                    .map_err(|_| RuntimeError("credential_unavailable"))
            })
            .await
            .map_err(|_| RuntimeError("credential_unavailable"))??
        }
        SecretReference::Vault { source, field } => acquired
            .get(&source)
            .and_then(|value| value.fields.get(&field))
            .and_then(serde_json::Value::as_str)
            .ok_or(RuntimeError("credential_value_invalid"))?
            .to_owned(),
        SecretReference::LocalFile { path, key } => {
            let values = tokio::task::spawn_blocking(move || read_local_values(&path, false))
                .await
                .map_err(|_| RuntimeError("credential_unavailable"))??;
            values
                .get(&key)
                .cloned()
                .ok_or(RuntimeError("credential_unavailable"))?
        }
    };
    // Exact redaction uses the shared minimum of four bytes.
    if value.len() < 4 || value.len() > 16384 || value.contains('\0') {
        return Err(RuntimeError("credential_value_invalid"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    /// A private local-file binding whose value the test can rotate.
    #[cfg(unix)]
    struct Fixture {
        directory: std::path::PathBuf,
        values: std::path::PathBuf,
        bindings: std::path::PathBuf,
        connection: Uuid,
        secret: Uuid,
        resolver: CredentialResolver,
    }
    #[cfg(unix)]
    fn write_private(path: &std::path::Path, body: serde_json::Value) {
        std::fs::write(path, serde_json::to_vec(&body).unwrap()).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    #[cfg(unix)]
    impl Fixture {
        fn new(first: &str) -> Self {
            let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../.cache/mcp-credential-cache-proof")
                .join(Uuid::new_v4().to_string());
            std::fs::create_dir_all(&directory).unwrap();
            let (connection, secret) = (Uuid::new_v4(), Uuid::new_v4());
            let values = directory.join("values.json");
            write_private(&values, serde_json::json!({ secret.to_string(): first }));
            let bindings = directory.join("bindings.json");
            write_private(
                &bindings,
                serde_json::json!({"bindings":[{"connection_id":connection,"alias":"fixture",
                    "runner_reference":"central","environment":{"FIXTURE_CREDENTIAL":
                    {"provider":"local_file","path":values,"key":secret}}}]}),
            );
            let resolver = CredentialResolver::new(Some(bindings.clone()));
            Self {
                directory,
                values,
                bindings,
                connection,
                secret,
                resolver,
            }
        }
        fn rotate(&self, value: &str) {
            write_private(
                &self.values,
                serde_json::json!({ self.secret.to_string(): value }),
            );
        }
        async fn resolve(&self) -> Result<Arc<ResolvedCredentials>> {
            self.resolver
                .resolve(self.connection, Some("fixture"), "central")
                .await
        }
    }
    #[cfg(unix)]
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rotated_values_are_resolved_on_the_next_call_and_unchanged_ones_keep_their_object() {
        let fixture = Fixture::new("first-secret");
        let first = fixture.resolve().await.unwrap();
        assert_eq!(first.environment["FIXTURE_CREDENTIAL"], "first-secret");
        // Unchanged values return the same object and generation, so a running
        // compatible instance is not restarted for nothing.
        let same = fixture.resolve().await.unwrap();
        assert!(Arc::ptr_eq(&first, &same));
        // A rotation behind an unchanged bindings file is seen by the very next
        // call, with a fresh generation and no invalidation. The old object
        // keeps its own values for operations still using it.
        fixture.rotate("second-secret");
        let second = fixture.resolve().await.unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(second.environment["FIXTURE_CREDENTIAL"], "second-secret");
        assert_ne!(first.generation, second.generation);
        assert_eq!(first.environment["FIXTURE_CREDENTIAL"], "first-secret");
        assert!(Arc::ptr_eq(&second, &fixture.resolve().await.unwrap()));
        // A loosened or replaced key file is refused on the next call.
        std::fs::set_permissions(&fixture.values, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(fixture.resolve().await.is_err());
        std::fs::set_permissions(&fixture.values, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(fixture.resolve().await.is_ok());
        // A changed bindings file is never served from the cache either.
        write_private(&fixture.bindings, serde_json::json!({"bindings":[]}));
        assert!(matches!(
            fixture.resolve().await,
            Err(RuntimeError("credential_binding_missing"))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_upstream_rejection_drops_only_that_resolution_from_the_cache() {
        let fixture = Fixture::new("first-secret");
        let first = fixture.resolve().await.unwrap();
        first.rejected().await;
        assert!(fixture.resolver.cache.lock().await.is_empty());
        // The value is unchanged but the next operation still gets a fresh
        // generation, so no running instance keeps a refused credential.
        let second = fixture.resolve().await.unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
        assert_ne!(first.generation, second.generation);
        // A late rejection of the superseded resolution leaves the new entry.
        first.rejected().await;
        assert!(Arc::ptr_eq(&second, &fixture.resolve().await.unwrap()));
        // Resolutions that never came from a resolver have nothing to drop.
        ResolvedCredentials::anonymous().rejected().await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_upstream_401_or_403_drops_the_entry_and_is_not_retried() {
        use rmcp::transport::streamable_http_client::StreamableHttpClient;
        use std::sync::atomic::AtomicUsize;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let fixture = Fixture::new("first-secret");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let served = requests.clone();
        let server = tokio::spawn(async move {
            for status in ["401 Unauthorized", "403 Forbidden"] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut seen = vec![0u8; 8192];
                let _ = stream.read(&mut seen).await.unwrap();
                served.fetch_add(1, Ordering::SeqCst);
                let reply =
                    format!("HTTP/1.1 {status}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
                stream.write_all(reply.as_bytes()).await.unwrap();
            }
        });
        let ping: rmcp::model::ClientJsonRpcMessage =
            serde_json::from_value(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"ping"}))
                .unwrap();
        for expected in 1..=2 {
            let credentials = fixture.resolve().await.unwrap();
            let http = crate::http::BoundedHttp(reqwest::Client::new(), Some(credentials.clone()));
            let Err(error) = http
                .post_message(
                    format!("http://{address}/mcp").into(),
                    ping.clone(),
                    None,
                    None,
                    Default::default(),
                )
                .await
            else {
                panic!("an upstream rejection fails the call");
            };
            assert!(
                error.to_string().contains("provider_auth_rejected"),
                "{error}"
            );
            assert_eq!(requests.load(Ordering::SeqCst), expected, "no retry");
            assert!(fixture.resolver.cache.lock().await.is_empty());
        }
        server.await.unwrap();
    }

    #[test]
    fn credential_destinations_cannot_change_loading_or_http_routing() {
        for name in [
            "RUBYOPT",
            "PERL5OPT",
            "JAVA_TOOL_OPTIONS",
            "LD_PRELOAD",
            "DYLD_INSERT_LIBRARIES",
            "NODE_PATH",
            "BASH_ENV",
            "RECOLLECT_DEVICE_PROFILE",
        ] {
            assert!(!super::environment_destination(name), "{name}");
        }
        for name in [
            "forwarded",
            "x-forwarded-host",
            "x-forwarded-proto",
            "x-original-url",
            "x-rewrite-url",
            "host",
            "mcp-session-id",
        ] {
            assert!(!super::header_destination(name), "{name}");
        }
        assert!(super::environment_destination("CONNECTOR_API_KEY"));
        assert!(super::header_destination("authorization"));
        assert!(super::header_destination("x-api-key"));
    }
}
