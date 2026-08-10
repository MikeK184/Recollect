use crate::{Result, RuntimeError};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{io::AsyncReadExt, sync::Mutex};
use uuid::Uuid;
#[cfg(test)]
mod tests_live;
mod vault;

#[derive(Clone, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case", deny_unknown_fields)]
pub enum SecretReference {
    Environment { name: String },
    OsStore { account: String },
    Vault { source: String, field: String },
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
        })
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
pub struct CredentialResolver {
    path: Option<PathBuf>,
    cache: Mutex<BTreeMap<Key, Arc<ResolvedCredentials>>>,
}
impl CredentialResolver {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            cache: Mutex::new(BTreeMap::new()),
        }
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
        let key = (connection, alias.to_owned(), runner.to_owned());
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
fn environment_destination(name: &str) -> bool {
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
fn header_destination(name: &str) -> bool {
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
    };
    // Exact redaction uses the shared minimum of four bytes.
    if value.len() < 4 || value.len() > 16384 || value.contains('\0') {
        return Err(RuntimeError("credential_value_invalid"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
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
