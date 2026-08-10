//! Authentication belongs to an operator-owned Vault Proxy. This module only
//! reads approved secret paths and renews the exact secret leases it acquired.
//! There is deliberately no token API, revoke API, cache or fallback transport.
use crate::{Result, RuntimeError};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::{future::Future, path::PathBuf, sync::Arc, time::Duration};
use tokio::{sync::watch, time::Instant};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Kind {
    KvV2,
    Leased,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    socket: PathBuf,
    path: String,
    kind: Kind,
}
pub(super) struct Acquired {
    pub fields: Map<String, Value>,
    pub lease: Option<Arc<Lease>>,
}
// No Debug or serialization: the ID is an in-memory renewal capability.
pub(super) struct Lease {
    client: reqwest::Client,
    id: String,
    deadline: watch::Sender<Instant>,
    renewable: bool,
    failed: std::sync::atomic::AtomicBool,
}

fn plain_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 512
        && path.split('/').all(|part| {
            !part.is_empty()
                && !matches!(part, "." | "..")
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        })
        && !matches!(path.split('/').next(), Some("sys" | "auth"))
}
#[cfg(unix)]
async fn client(socket: &std::path::Path) -> Result<reqwest::Client> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let invalid = || RuntimeError("credential_proxy_invalid");
    if !socket.is_absolute() {
        return Err(invalid());
    }
    let parent = socket.parent().ok_or_else(invalid)?;
    let directory = tokio::fs::symlink_metadata(parent)
        .await
        .map_err(|_| invalid())?;
    let metadata = tokio::fs::symlink_metadata(socket)
        .await
        .map_err(|_| invalid())?;
    let uid = nix::unistd::Uid::effective().as_raw();
    if !directory.is_dir()
        || !metadata.file_type().is_socket()
        || directory.uid() != uid
        || metadata.uid() != uid
        || directory.mode() & 0o077 != 0
        || metadata.mode() & 0o077 != 0
    {
        return Err(invalid());
    }
    reqwest::Client::builder()
        .unix_socket(socket.to_owned())
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|_| invalid())
}
#[cfg(not(unix))]
async fn client(_: &std::path::Path) -> Result<reqwest::Client> {
    Err(RuntimeError("credential_proxy_unsupported"))
}
async fn response(request: reqwest::RequestBuilder) -> Result<Value> {
    // The outer deadline includes bounded body reads and JSON decoding too.
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut response = request
            .send()
            .await
            .map_err(|_| RuntimeError("credential_vault_unavailable"))?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(RuntimeError("credential_vault_denied"));
        }
        if !response.status().is_success() {
            return Err(RuntimeError("credential_vault_unavailable"));
        }
        if response.content_length().is_some_and(|n| n > 262144) {
            return Err(RuntimeError("credential_vault_response_invalid"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| RuntimeError("credential_vault_unavailable"))?
        {
            if bytes.len() + chunk.len() > 262144 {
                return Err(RuntimeError("credential_vault_response_invalid"));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| RuntimeError("credential_vault_response_invalid"))
    })
    .await
    .map_err(|_| RuntimeError("credential_vault_unavailable"))?
}
fn validity(value: &Value, requested: Instant) -> Result<(String, Instant, bool)> {
    let invalid = || RuntimeError("credential_lease_invalid");
    let id = value
        .get("lease_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 1024 && !id.chars().any(char::is_control))
        .ok_or_else(invalid)?;
    let seconds = value
        .get("lease_duration")
        .and_then(Value::as_u64)
        .filter(|n| (1..=86400).contains(n))
        .ok_or_else(invalid)?;
    let renewable = value
        .get("renewable")
        .and_then(Value::as_bool)
        .ok_or_else(invalid)?;
    // Count the full round trip against returned TTL, never extending validity
    // merely because the response took a long time to reach this host.
    let deadline = requested + Duration::from_secs(seconds);
    if deadline <= Instant::now() {
        return Err(RuntimeError("credential_expired"));
    }
    Ok((id.to_owned(), deadline, renewable))
}
impl Source {
    pub async fn acquire(&self) -> Result<Acquired> {
        if !plain_path(&self.path) {
            return Err(RuntimeError("credential_reference_invalid"));
        }
        let client = client(&self.socket).await?;
        let requested = Instant::now();
        let value = response(client.get(format!("http://localhost/v1/{}", self.path))).await?;
        let fields = match self.kind {
            Kind::KvV2 => value.pointer("/data/data"),
            Kind::Leased => value.get("data"),
        }
        .and_then(Value::as_object)
        .cloned()
        .ok_or(RuntimeError("credential_vault_response_invalid"))?;
        let lease = match self.kind {
            Kind::KvV2 => None,
            Kind::Leased => {
                let (id, deadline, renewable) = validity(&value, requested)?;
                Some(Arc::new(Lease {
                    client,
                    id,
                    deadline: watch::channel(deadline).0,
                    renewable,
                    failed: std::sync::atomic::AtomicBool::new(false),
                }))
            }
        };
        Ok(Acquired { fields, lease })
    }
}
impl Lease {
    #[cfg(test)]
    pub(super) fn deadline(&self) -> Instant {
        *self.deadline.borrow()
    }
    pub fn valid(&self) -> bool {
        *self.deadline.borrow() > Instant::now()
    }
    pub fn reusable(&self) -> bool {
        self.valid() && !self.failed.load(std::sync::atomic::Ordering::Acquire)
    }
    pub async fn expired(&self) {
        let mut deadline = self.deadline.subscribe();
        loop {
            let until = *deadline.borrow_and_update();
            tokio::select! {
                biased;
                _ = tokio::time::sleep_until(until) => if !self.valid() { return; },
                changed = deadline.changed() => if changed.is_err() { return; },
            }
        }
    }
    pub async fn renew<F, Fut>(&self, authorize: F)
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        if !self.renewable {
            return;
        }
        loop {
            let until = *self.deadline.borrow();
            let remaining = until.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return;
            }
            tokio::time::sleep(remaining / 2).await;
            let renewal = async {
                // Authorization is deliberately outside Proxy auto-auth, and is
                // never cached. Bound it by the old lease's remaining validity.
                authorize()
                    .await
                    .map_err(|_| RuntimeError("credential_renewal_denied"))?;
                let requested = Instant::now();
                let value = response(
                    self.client
                        .post("http://localhost/v1/sys/leases/renew")
                        .json(&json!({"lease_id":self.id})),
                )
                .await?;
                let (id, deadline, renewable) = validity(&value, requested)?;
                if id != self.id {
                    return Err(RuntimeError("credential_lease_invalid"));
                }
                Ok((deadline, renewable))
            };
            match tokio::time::timeout_at(until, renewal).await {
                Ok(Ok((deadline, renewable))) => {
                    self.deadline.send_replace(deadline);
                    if !renewable {
                        return;
                    }
                }
                result => {
                    self.failed
                        .store(true, std::sync::atomic::Ordering::Release);
                    let code = match result {
                        Ok(Err(error)) => error.0,
                        _ => "credential_expired",
                    };
                    tracing::warn!(
                        code,
                        "MCP credential renewal stopped; natural expiry applies"
                    );
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn vault_references_cannot_select_control_or_escape_paths() {
        for path in [
            "auth/token/create",
            "sys/mounts",
            "/kv/data/test",
            "kv/../sys",
            "kv/%2e/data",
            "kv//data",
            "kv/data?x=y",
            "kv/data#fragment",
            "kv/data\\test",
        ] {
            assert!(!super::plain_path(path));
        }
        assert!(super::plain_path("mcp-test-kv/data/fixture"));
        assert!(super::plain_path("mcp-test-kv/data/service.v2"));
    }
}
