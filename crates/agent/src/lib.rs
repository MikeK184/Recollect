use anyhow::{Result, anyhow, bail, ensure};
use recollect_protocol::{
    ApiError, Brain, PairingCode, PairingPoll, PairingRequest, PairingStart, SessionInfo,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::Duration;
use uuid::Uuid;

pub mod capture;
pub mod capture_delivery;
pub mod capture_scope;
pub mod mcp;
pub mod mcp_bridge;
pub mod mcp_cli;
pub mod mcp_host;
#[cfg(target_os = "macos")]
pub mod os_store;
pub mod plugin;
pub mod plugin_coordination;
pub mod plugin_migration;
pub mod plugin_runtime;
pub mod plugin_session;
pub mod plugin_skills;
pub mod plugin_storage;
pub mod presentation;
pub mod privacy;
pub mod publication;
pub mod publication_cli;
mod publication_hcl;
pub mod workspace;
pub mod workspace_cli;

#[derive(Serialize, Deserialize)]
pub struct StoredDevice {
    pub endpoint: String,
    pub device_id: Uuid,
    pub token: Uuid,
}

pub struct CredentialSlot {
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    account: String,
    /// Explicit file-backed store for test fixtures and CI. Set only through
    /// `RECOLLECT_CREDENTIAL_STORE_DIR`; normal installs never use it.
    file: Option<std::path::PathBuf>,
    /// `None` only for the file-backed store, which never touches the OS store.
    #[cfg(not(target_os = "macos"))]
    entry: Option<keyring::Entry>,
}
const COMPANION_SERVICE: &str = "recollect-companion";
/// Directory that replaces the OS credential store when set. Intended for
/// test fixtures that would otherwise prompt for Keychain access on every
/// rebuilt binary. Files are private (0600) JSON, one per profile.
pub const CREDENTIAL_STORE_DIR_VAR: &str = "RECOLLECT_CREDENTIAL_STORE_DIR";
fn storage_error() -> anyhow::Error {
    anyhow!(
        "The OS credential store is unavailable or locked. Unlock it and retry; no file fallback is used."
    )
}
fn file_store_path(account: &str) -> Option<std::path::PathBuf> {
    let dir = std::env::var_os(CREDENTIAL_STORE_DIR_VAR)?;
    if dir.is_empty() {
        return None;
    }
    let name: String = account
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    Some(std::path::PathBuf::from(dir).join(format!("{name}.json")))
}
impl CredentialSlot {
    pub fn new(endpoint: &str, profile: &str) -> Result<Self> {
        ensure!(
            !profile.is_empty() && profile.len() <= 80 && !profile.chars().any(char::is_control),
            "Use a local device profile between 1 and 80 printable bytes"
        );
        let account = format!("{endpoint}#{profile}");
        let file = file_store_path(&account);
        #[cfg(target_os = "macos")]
        return Ok(Self { account, file });
        #[cfg(not(target_os = "macos"))]
        let entry =
            keyring::Entry::new(COMPANION_SERVICE, &account).map_err(|_| storage_error())?;
        #[cfg(not(target_os = "macos"))]
        return Ok(Self {
            account,
            file,
            entry: Some(entry),
        });
    }
    fn file_load(path: &std::path::Path) -> Result<Option<StoredDevice>> {
        match std::fs::read(path) {
            Ok(data) => Ok(Some(serde_json::from_slice(&data).map_err(|_| {
                anyhow!("This profile's stored credential is unreadable. Use forget, revoke it in Devices and pair again.")
            })?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(storage_error()),
        }
    }
    fn file_save(path: &std::path::Path, device: &StoredDevice) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| storage_error())?;
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path).map_err(|_| storage_error())?;
        use std::io::Write;
        file.write_all(serde_json::to_string(device)?.as_bytes())
            .map_err(|_| storage_error())
    }
    fn file_forget(path: &std::path::Path) -> Result<()> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(storage_error()),
        }
    }
    /// Slot backed by one private file; used when the store directory is set.
    #[doc(hidden)]
    pub fn file_backed(path: std::path::PathBuf) -> Self {
        #[cfg(target_os = "macos")]
        return Self {
            account: String::new(),
            file: Some(path),
        };
        #[cfg(not(target_os = "macos"))]
        return Self {
            account: String::new(),
            file: Some(path),
            entry: None,
        };
    }
    pub fn load(&self) -> Result<Option<StoredDevice>> {
        if let Some(path) = &self.file {
            return Self::file_load(path);
        }
        #[cfg(target_os = "macos")]
        let data = os_store::load(COMPANION_SERVICE, &self.account)?;
        #[cfg(target_os = "macos")]
        return match data {
            Some(data)=>Ok(Some(serde_json::from_slice(&data).map_err(|_|anyhow!("This profile's stored credential is unreadable. Use forget, revoke it in Devices and pair again."))?)),
            None=>Ok(None),
        };
        #[cfg(not(target_os = "macos"))]
        match self.entry.as_ref().ok_or_else(storage_error)?.get_password() {
            Ok(data)=>Ok(Some(serde_json::from_str(&data).map_err(|_|anyhow!("This profile's stored credential is unreadable. Use forget, revoke it in Devices and pair again."))?)),
            Err(keyring::Error::NoEntry)=>Ok(None),
            Err(_)=>Err(storage_error()),
        }
    }
    pub fn save(&self, device: &StoredDevice) -> Result<()> {
        if let Some(path) = &self.file {
            return Self::file_save(path, device);
        }
        #[cfg(target_os = "macos")]
        return os_store::save(
            COMPANION_SERVICE,
            &self.account,
            serde_json::to_string(device)?.as_bytes(),
        )
        .map_err(|_| storage_error());
        #[cfg(not(target_os = "macos"))]
        self.entry
            .as_ref()
            .ok_or_else(storage_error)?
            .set_password(&serde_json::to_string(device)?)
            .map_err(|_| storage_error())
    }
    pub fn forget(&self) -> Result<()> {
        if let Some(path) = &self.file {
            return Self::file_forget(path);
        }
        #[cfg(target_os = "macos")]
        return match os_store::delete(COMPANION_SERVICE, &self.account) {
            Ok(_) => Ok(()),
            Err(_) => Err(storage_error()),
        };
        #[cfg(not(target_os = "macos"))]
        match self
            .entry
            .as_ref()
            .ok_or_else(storage_error)?
            .delete_credential()
        {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(storage_error()),
        }
    }
}

/// Default whole-request budget. Large uploads pass `payload_timeout` instead.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Whole-request budget for a body of `bytes`: the default plus time at a floor
/// of 128 KiB/s, so a 24 MiB publication is not capped by the ordinary budget.
pub fn payload_timeout(bytes: usize) -> Duration {
    REQUEST_TIMEOUT + Duration::from_secs((bytes / (128 * 1024)) as u64)
}

#[derive(Clone)]
pub struct Client {
    pub endpoint: String,
    http: reqwest::Client,
    host: Option<String>,
}
impl Client {
    pub fn new(endpoint: &str) -> Result<Self> {
        let url = reqwest::Url::parse(endpoint)
            .map_err(|_| anyhow!("RECOLLECT_URL is not a valid service URL"))?;
        ensure!(
            url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "RECOLLECT_URL must not contain credentials, a query or a fragment"
        );
        ensure!(
            url.scheme() == "https"
                || (url.scheme() == "http"
                    && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))),
            "Use HTTPS for a shared Recollect service, or loopback HTTP locally"
        );
        let endpoint = url.as_str().trim_end_matches('/').to_owned();
        let mut http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none());
        if let Some(pem) = recollect_mcp_runtime::agent_tls::configured_pem()? {
            let roots = reqwest::Certificate::from_pem_bundle(&pem)
                .map_err(|_| anyhow!("agent_ca_file_invalid"))?;
            ensure!(
                !roots.is_empty() && roots.len() <= 16,
                "agent_ca_file_invalid"
            );
            for root in roots {
                http = http.add_root_certificate(root);
            }
        }
        let http = http.build()?;
        Ok(Self {
            endpoint,
            http,
            host: None,
        })
    }
    pub fn with_host(mut self, host: &str) -> Self {
        if matches!(host, "codex" | "claude_code" | "opencode") {
            self.host = Some(host.to_owned());
        }
        self
    }
    pub async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        token: Option<Uuid>,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::Response> {
        self.send_inner(method, path, token, body, None).await
    }
    /// Like `send` with an explicit whole-request timeout (e.g. `payload_timeout`).
    pub async fn send_with_timeout(
        &self,
        method: reqwest::Method,
        path: &str,
        token: Option<Uuid>,
        body: Option<serde_json::Value>,
        timeout: Duration,
    ) -> Result<reqwest::Response> {
        self.send_inner(method, path, token, body, Some(timeout))
            .await
    }
    async fn send_inner(
        &self,
        method: reqwest::Method,
        path: &str,
        token: Option<Uuid>,
        body: Option<serde_json::Value>,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Response> {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.endpoint));
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        if let Some(host) = &self.host {
            request = request.header("x-recollect-host", host);
        }
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        request.send().await.map_err(|_| {
            anyhow!("Cannot reach Recollect. Check the service URL and connection, then retry.")
        })
    }
    pub async fn health(&self) -> Result<()> {
        let response = self
            .send(reqwest::Method::GET, "/health/ready", None, None)
            .await?;
        ensure!(
            response.status().is_success(),
            "Recollect is unavailable or a dependency is degraded"
        );
        Ok(())
    }
    pub async fn whoami(&self, device: &StoredDevice) -> Result<SessionInfo> {
        let identity: SessionInfo = decode(
            self.send(
                reqwest::Method::GET,
                "/api/auth/me",
                Some(device.token),
                None,
            )
            .await?,
        )
        .await?;
        ensure!(
            identity.device_id == Some(device.device_id),
            "The stored device identity does not match this service"
        );
        Ok(identity)
    }
    pub async fn brains(&self, device: &StoredDevice) -> Result<Vec<Brain>> {
        decode(
            self.send(
                reqwest::Method::GET,
                "/api/brains",
                Some(device.token),
                None,
            )
            .await?,
        )
        .await
    }
    pub async fn revoke(&self, device: &StoredDevice) -> Result<()> {
        check(
            self.send(
                reqwest::Method::POST,
                "/api/devices/revoke-self",
                Some(device.token),
                None,
            )
            .await?,
        )
        .await?;
        Ok(())
    }
    pub async fn pair(&self, slot: &CredentialSlot, name: String) -> Result<Uuid> {
        ensure!(
            slot.load()?.is_none(),
            "This endpoint/profile is already paired. Use unpair before replacing it."
        );
        let start: PairingStart = decode(
            self.send(
                reqwest::Method::POST,
                "/api/devices/pairings",
                None,
                Some(serde_json::to_value(PairingRequest {
                    name,
                    host_kind: None,
                    integration: Some("plugin".into()),
                })?),
            )
            .await?,
        )
        .await?;
        println!("Open {}", start.verification_url);
        println!(
            "Compare code {} in your browser. Waiting for approval…",
            start.user_code
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
        let code = serde_json::to_value(PairingCode {
            device_code: start.device_code,
        })?;
        let mut connection_failed = false;
        loop {
            tokio::select! {
                _=tokio::signal::ctrl_c()=>{
                    let _=self.send(reqwest::Method::POST,"/api/devices/pairings/cancel",None,Some(code.clone())).await;
                    bail!("Pairing cancelled");
                }
                _=tokio::time::sleep(Duration::from_secs(start.interval_seconds.clamp(2,10).into()))=>{}
            }
            if tokio::time::Instant::now() >= deadline {
                if connection_failed {
                    bail!(
                        "Pairing could not finish while Recollect was unreachable. Check the connection and start again."
                    );
                }
                bail!("Pairing expired. Start again when your browser is ready.");
            }
            let response = match self
                .send(
                    reqwest::Method::POST,
                    "/api/devices/pairings/poll",
                    None,
                    Some(code.clone()),
                )
                .await
            {
                Ok(response) => response,
                Err(_) => {
                    if !connection_failed {
                        eprintln!("Connection interrupted. Retrying until the pairing expires…");
                    }
                    connection_failed = true;
                    continue;
                }
            };
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                || response.status().is_server_error()
            {
                if response.status().is_server_error() {
                    if !connection_failed {
                        eprintln!("Recollect is temporarily unavailable. Retrying…");
                    }
                    connection_failed = true;
                }
                continue;
            }
            connection_failed = false;
            let reply: PairingPoll = decode(response).await?;
            if reply.state == "pending" {
                continue;
            }
            ensure!(
                reply.state == "approved",
                "This pairing cannot be completed. Start pairing again."
            );
            let device = reply
                .device
                .ok_or_else(|| anyhow!("Pairing returned no device identity"))?;
            let stored = StoredDevice {
                endpoint: self.endpoint.clone(),
                device_id: device.id,
                token: reply
                    .token
                    .ok_or_else(|| anyhow!("Pairing returned no device credential"))?,
            };
            if let Err(error) = slot.save(&stored) {
                let _ = self
                    .send(
                        reqwest::Method::POST,
                        "/api/devices/pairings/cancel",
                        None,
                        Some(code.clone()),
                    )
                    .await;
                return Err(error);
            }
            let response = self
                .send(
                    reqwest::Method::POST,
                    "/api/devices/pairings/finish",
                    None,
                    Some(code.clone()),
                )
                .await;
            let acknowledged = match response {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            };
            if !acknowledged && self.whoami(&stored).await.is_err() {
                bail!(
                    "Credential saved, but pairing acknowledgement was not confirmed. Retry whoami or unpair before pairing again."
                );
            }
            self.whoami(&stored).await?;
            return Ok(device.id);
        }
    }
}
async fn check(response: reqwest::Response) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        bail!(
            "Device access is unavailable: revoked, expired, disabled, or organization sign-in needs refreshing. Open Recollect to check."
        );
    }
    let error = response.json::<ApiError>().await.ok();
    bail!(
        "{}",
        error
            .map(|e| e.message)
            .unwrap_or("Recollect rejected the request. Retry shortly.".into())
    )
}
async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    check(response)
        .await?
        .json()
        .await
        .map_err(|_| anyhow!("Recollect returned an unreadable response"))
}

#[cfg(test)]
mod credential_file_store_tests {
    use super::*;

    #[test]
    fn file_backed_slot_round_trips_without_the_os_store() {
        let dir = std::env::temp_dir().join(format!("recollect-slot-{}", Uuid::new_v4()));
        let slot = CredentialSlot::file_backed(dir.join("profile.json"));
        assert!(slot.load().unwrap().is_none());
        let device = StoredDevice {
            endpoint: "http://127.0.0.1:1".into(),
            device_id: Uuid::new_v4(),
            token: Uuid::new_v4(),
        };
        slot.save(&device).unwrap();
        let loaded = slot.load().unwrap().unwrap();
        assert_eq!(loaded.device_id, device.device_id);
        assert_eq!(loaded.token, device.token);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("profile.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        slot.forget().unwrap();
        assert!(slot.load().unwrap().is_none());
        slot.forget().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn store_path_is_sanitized_per_account() {
        let dir = std::env::temp_dir().join("recollect-slot-path");
        unsafe { std::env::set_var(CREDENTIAL_STORE_DIR_VAR, &dir) };
        let path = file_store_path("http://h:1#profile/one").unwrap();
        unsafe { std::env::remove_var(CREDENTIAL_STORE_DIR_VAR) };
        assert_eq!(path.parent().unwrap(), dir);
        assert_eq!(path.file_name().unwrap(), "http___h_1_profile_one.json");
        assert!(file_store_path("x").is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uploads_get_a_payload_sized_budget_beyond_the_default() {
        assert_eq!(payload_timeout(0), REQUEST_TIMEOUT);
        let upload = payload_timeout(recollect_protocol::PUBLICATION_MAX_BYTES);
        assert!(upload > Duration::from_secs(120), "{upload:?}");
        assert!(payload_timeout(1024 * 1024) < upload);
    }
}
