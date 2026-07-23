use anyhow::{Result, anyhow, bail, ensure};
use recollect_protocol::{
    ApiError, Brain, PairingCode, PairingPoll, PairingRequest, PairingStart, SessionInfo,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::Duration;
use uuid::Uuid;

pub mod capture;
pub mod capture_cli;
pub mod capture_delivery;
pub mod capture_launch;
pub mod capture_run;
pub mod capture_setup;
pub mod mcp;
pub mod mcp_bridge;
pub mod mcp_cli;
pub mod mcp_host;
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
    entry: keyring::Entry,
}
fn storage_error() -> anyhow::Error {
    anyhow!(
        "The OS credential store is unavailable or locked. Unlock it and retry; no file fallback is used."
    )
}
impl CredentialSlot {
    pub fn new(endpoint: &str, profile: &str) -> Result<Self> {
        ensure!(
            !profile.is_empty() && profile.len() <= 80 && !profile.chars().any(char::is_control),
            "Use a local device profile between 1 and 80 printable bytes"
        );
        let entry = keyring::Entry::new("recollect-companion", &format!("{endpoint}#{profile}"))
            .map_err(|_| storage_error())?;
        Ok(Self { entry })
    }
    pub fn load(&self) -> Result<Option<StoredDevice>> {
        match self.entry.get_password() {
            Ok(data)=>Ok(Some(serde_json::from_str(&data).map_err(|_|anyhow!("This profile's stored credential is unreadable. Use forget, revoke it in Devices and pair again."))?)),
            Err(keyring::Error::NoEntry)=>Ok(None),
            Err(_)=>Err(storage_error()),
        }
    }
    pub fn save(&self, device: &StoredDevice) -> Result<()> {
        self.entry
            .set_password(&serde_json::to_string(device)?)
            .map_err(|_| storage_error())
    }
    pub fn forget(&self) -> Result<()> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(storage_error()),
        }
    }
}

pub struct Client {
    pub endpoint: String,
    http: reqwest::Client,
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
            .timeout(Duration::from_secs(5))
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
        Ok(Self { endpoint, http })
    }
    pub async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        token: Option<Uuid>,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::Response> {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.endpoint));
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
                Some(serde_json::to_value(PairingRequest { name })?),
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
