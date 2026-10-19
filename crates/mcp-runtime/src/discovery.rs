//! Explicit anonymous metadata inspection. Never executes a remote tool.
use crate::{Result, RuntimeError, WIRE_LIMIT, http::BoundedHttp};
use recollect_protocol::McpToolDescriptor;
use rmcp::{
    ClientLifecycleMode, ClientServiceExt,
    model::{PaginatedRequestParams, ProtocolVersion},
    transport::{
        common::client_side_sse::NeverRetry,
        streamable_http_client::{
            StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
        },
    },
};
use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

/// Whether a connection target is a public unicast address. Private, loopback,
/// link-local, shared, documentation and transition ranges are all refused.
pub fn is_public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(matches!(a, 0 | 10 | 127 | 224..=255)
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && (b == 0 || b == 168 || (b == 88 && c == 99)))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            // Accept native global unicast only. Reject mapped/transition,
            // documentation, special assignments and all local address ranges.
            (0x2000..=0x3fff).contains(&s[0])
                && s[0] != 0x2002
                && s[0] < 0x3ffe
                && !(s[0] == 0x2001 && (s[1] < 0x200 || s[1] == 0x0db8))
        }
    }
}

/// Resolve a host to addresses that are all public, or none. Callers must
/// connect to the returned addresses (not re-resolve) to stay rebinding-proof.
pub async fn resolve_public(host: &str, port: u16) -> Option<Vec<SocketAddr>> {
    let addresses: Vec<SocketAddr> = match host.trim_matches(['[', ']']).parse::<IpAddr>() {
        Ok(ip) => vec![SocketAddr::new(ip, port)],
        Err(_) => tokio::net::lookup_host((host, port))
            .await
            .ok()?
            .take(17)
            .collect(),
    };
    (!addresses.is_empty()
        && addresses.len() <= 16
        && addresses.iter().all(|a| is_public_address(a.ip())))
    .then_some(addresses)
}

pub async fn inspect_http(target: &str) -> Result<Vec<McpToolDescriptor>> {
    inspect_http_with_headers(target, &std::collections::BTreeMap::new()).await
}

pub async fn inspect_http_with_headers(
    target: &str,
    headers: &std::collections::BTreeMap<String, String>,
) -> Result<Vec<McpToolDescriptor>> {
    Ok(inspect_http_metadata_with_headers(target, headers)
        .await?
        .tools)
}

pub struct HttpMetadata {
    pub tools: Vec<McpToolDescriptor>,
    pub icon_sources: Vec<String>,
}

fn description(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .take(2000)
        .collect()
}

pub async fn inspect_http_metadata_with_headers(
    target: &str,
    headers: &std::collections::BTreeMap<String, String>,
) -> Result<HttpMetadata> {
    let url = reqwest::Url::parse(target).map_err(|_| RuntimeError("provider_target_invalid"))?;
    if target.len() > 2048
        || !(url.scheme() == "https"
            || (url.scheme() == "http"
                && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        return Err(RuntimeError("provider_target_invalid"));
    }
    tokio::time::timeout(Duration::from_secs(25), async {
        if headers.len() > 32 {
            return Err(RuntimeError("credential_destination_invalid"));
        }
        let mut selected = http::HeaderMap::new();
        for (name, value) in headers {
            let name = name.to_ascii_lowercase();
            if !crate::credentials::header_destination(&name)
                || value.len() < 4
                || value.len() > 16384
            {
                return Err(RuntimeError("credential_destination_invalid"));
            }
            let name = http::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| RuntimeError("credential_destination_invalid"))?;
            let value = http::HeaderValue::from_str(value)
                .map_err(|_| RuntimeError("credential_value_invalid"))?;
            if selected.insert(name, value).is_some() {
                return Err(RuntimeError("credential_destination_invalid"));
            }
        }
        let mut client = reqwest::Client::builder()
            .default_headers(selected)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(5));
        if url.scheme() == "https" {
            // Remote servers must be public. Connect to the addresses that were
            // checked so a later DNS answer cannot reach a private network.
            let host = url
                .host_str()
                .ok_or(RuntimeError("provider_target_invalid"))?;
            let port = url
                .port_or_known_default()
                .ok_or(RuntimeError("provider_target_invalid"))?;
            let addresses = resolve_public(host, port)
                .await
                .ok_or(RuntimeError("provider_target_invalid"))?;
            if host.trim_matches(['[', ']']).parse::<IpAddr>().is_err() {
                client = client.resolve_to_addrs(host, &addresses);
            }
        }
        let http = client
            .build()
            .map_err(|_| RuntimeError("provider_http_unavailable"))?;
        let mut config = StreamableHttpClientTransportConfig::with_uri(target.to_owned())
            .reinit_on_expired_session(false)
            .max_sse_event_size(WIRE_LIMIT)
            .max_concurrent_requests(1)
            .control_request_timeout(Duration::from_secs(2));
        config.retry_config = Arc::new(NeverRetry::default());
        let transport = StreamableHttpClientTransport::with_client(BoundedHttp(http, None), config);
        let mut service = ()
            .serve_with_lifecycle(
                transport,
                ClientLifecycleMode::Auto {
                    preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                    legacy_version: Some(ProtocolVersion::V_2025_11_25),
                },
            )
            .await
            .map_err(|_| RuntimeError("provider_initialization_failed"))?;
        let result = async {
            let icon_sources = service
                .peer_info()
                .and_then(|info| info.server_info.clone())
                .and_then(|info| info.icons)
                .unwrap_or_default()
                .into_iter()
                .take(3)
                .filter(|icon| icon.src.len() <= 360_000)
                .map(|icon| icon.src)
                .collect();
            let mut tools = Vec::new();
            let mut names = HashSet::new();
            let mut cursors = HashSet::new();
            let mut cursor = None;
            for _ in 0..10 {
                let page = service
                    .list_tools(
                        cursor.map(|s| PaginatedRequestParams::default().with_cursor(Some(s))),
                    )
                    .await
                    .map_err(|_| RuntimeError("provider_catalogue_unavailable"))?;
                for tool in page.tools {
                    if tools.len() >= 50 || !names.insert(tool.name.to_string()) {
                        return Err(RuntimeError("provider_catalogue_limit"));
                    }
                    tools.push(McpToolDescriptor {
                        name: tool.name.to_string(),
                        description: description(tool.description.as_deref().unwrap_or("")),
                        input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
                        output_schema: tool
                            .output_schema
                            .map(|s| serde_json::Value::Object((*s).clone())),
                        annotations: tool
                            .annotations
                            .map(|a| serde_json::to_value(a).expect("serializable annotations")),
                    });
                }
                cursor = page.next_cursor;
                match &cursor {
                    None => {
                        return Ok(HttpMetadata {
                            tools,
                            icon_sources,
                        });
                    }
                    Some(s) if s.len() <= 1024 && cursors.insert(s.clone()) => {}
                    _ => return Err(RuntimeError("provider_catalogue_invalid")),
                }
            }
            Err(RuntimeError("provider_catalogue_limit"))
        }
        .await;
        let _ = service.close_with_timeout(Duration::from_secs(2)).await;
        result
    })
    .await
    .map_err(|_| RuntimeError("provider_startup_timeout"))?
}

#[cfg(test)]
mod tests {
    use super::{description, inspect_http, is_public_address, resolve_public};

    #[tokio::test]
    async fn remote_https_targets_must_resolve_to_public_addresses() {
        for target in [
            "https://127.0.0.1/mcp",
            "https://10.1.2.3:8443/mcp",
            "https://[::1]/mcp",
            "https://169.254.169.254/latest",
            "https://192.168.0.10/mcp",
            "https://localhost/mcp",
        ] {
            assert_eq!(
                inspect_http(target).await.err().map(|e| e.0),
                Some("provider_target_invalid"),
                "{target}"
            );
        }
        assert!(resolve_public("8.8.8.8", 443).await.is_some());
        assert!(resolve_public("127.0.0.1", 443).await.is_none());
        assert!(is_public_address("2606:4700:4700::1111".parse().unwrap()));
        assert!(!is_public_address("fd00::1".parse().unwrap()));
    }
    #[test]
    fn descriptions_preserve_markdown_and_bound_hostile_controls() {
        let source = "# Title\n\n- one\n- two\n\n```json\n{\"a\":1}\n```\n\tend\0\u{1b}";
        assert_eq!(description(source), source.replace(['\0', '\u{1b}'], ""));
        assert_eq!(description(&"ü".repeat(2001)).chars().count(), 2000);
    }
}
