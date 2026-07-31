use crate::{Client, StoredDevice, mcp::request};
use anyhow::{Result, anyhow, bail};
use recollect_protocol::{McpCallInput, McpReconcileInput, McpResolveInput};
use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub const USAGE: &str = "mcp call BRAIN INPUT.json; mcp status BRAIN [CALL]; mcp cancel BRAIN CALL; mcp reconcile|resolve BRAIN CALL INPUT.json; mcp release BRAIN SESSION; mcp-runner OUTBOX_DIRECTORY; private-runner RUNNER_UUID OUTBOX_DIRECTORY. Use UUIDs; call inputs require an owned tool operation and stable request/session IDs. Connector credential references use RECOLLECT_MCP_CREDENTIALS_FILE.";
fn id(input: &str) -> Result<Uuid> {
    input
        .parse()
        .map_err(|_| anyhow!("Use a resource UUID. {USAGE}"))
}
async fn file<T: DeserializeOwned + Serialize>(path: &str) -> Result<Value> {
    let mut bytes = Vec::new();
    if path == "-" {
        tokio::io::stdin()
            .take(65537)
            .read_to_end(&mut bytes)
            .await?;
    } else {
        let file = tokio::fs::File::open(path)
            .await
            .map_err(|_| anyhow!("Cannot read the MCP input file."))?;
        if !file.metadata().await?.is_file() {
            bail!("Use a regular JSON input file.");
        }
        file.take(65537).read_to_end(&mut bytes).await?;
    }
    if bytes.len() > 65536 {
        bail!("MCP input file exceeds 64 KiB.");
    }
    let input: T = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow!("MCP input does not match the documented request fields."))?;
    Ok(serde_json::to_value(input)?)
}
pub async fn run(client: &Client, device: &StoredDevice, args: &[String]) -> Result<Value> {
    if client.endpoint != device.endpoint {
        bail!("The saved device belongs to another endpoint.");
    }
    let (method, path, body) = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["call", brain, input] => (
            Method::POST,
            format!("/api/brains/{}/mcp/calls", id(brain)?),
            Some(file::<McpCallInput>(input).await?),
        ),
        ["status", brain, call] => (
            Method::GET,
            format!("/api/brains/{}/mcp/calls/{}", id(brain)?, id(call)?),
            None,
        ),
        ["status", brain] => {
            let brain = id(brain)?;
            let calls: Value = request(
                client,
                device.token,
                Method::GET,
                &format!("/api/brains/{brain}/mcp/calls"),
                None,
            )
            .await?;
            let runtime: Value = request(
                client,
                device.token,
                Method::GET,
                &format!("/api/brains/{brain}/mcp/runtime"),
                None,
            )
            .await?;
            return Ok(json!({"calls":calls,"runtime":runtime}));
        }
        ["cancel", brain, call] => (
            Method::POST,
            format!("/api/brains/{}/mcp/calls/{}/cancel", id(brain)?, id(call)?),
            None,
        ),
        ["reconcile", brain, call, input] => (
            Method::POST,
            format!(
                "/api/brains/{}/mcp/calls/{}/reconcile",
                id(brain)?,
                id(call)?
            ),
            Some(file::<McpReconcileInput>(input).await?),
        ),
        ["resolve", brain, call, input] => (
            Method::POST,
            format!("/api/brains/{}/mcp/calls/{}/resolve", id(brain)?, id(call)?),
            Some(file::<McpResolveInput>(input).await?),
        ),
        ["release", brain, session] => (
            Method::POST,
            format!("/api/brains/{}/mcp/session/release", id(brain)?),
            Some(json!({"client_session_id":id(session)?})),
        ),
        _ => bail!("{USAGE}"),
    };
    Ok(request(client, device.token, method, &path, body).await?)
}
