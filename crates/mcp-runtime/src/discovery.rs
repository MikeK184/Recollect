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
use std::{collections::HashSet, sync::Arc, time::Duration};

pub async fn inspect_http(target: &str) -> Result<Vec<McpToolDescriptor>> {
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
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| RuntimeError("provider_http_unavailable"))?;
        let mut config = StreamableHttpClientTransportConfig::with_uri(target.to_owned())
            .reinit_on_expired_session(false)
            .max_sse_event_size(WIRE_LIMIT)
            .max_concurrent_requests(1)
            .control_request_timeout(Duration::from_secs(2));
        config.retry_config = Arc::new(NeverRetry::default());
        let transport = StreamableHttpClientTransport::with_client(BoundedHttp(http), config);
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
                        description: tool
                            .description
                            .as_deref()
                            .unwrap_or("")
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .chars()
                            .filter(|c| !c.is_control())
                            .take(2000)
                            .collect(),
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
                    None => return Ok(tools),
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
