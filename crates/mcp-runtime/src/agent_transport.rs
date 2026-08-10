//! The native stdio bridge reuses the runtime's bounded SDK HTTP adapter.
use crate::{Result, RuntimeError, WIRE_LIMIT, http::BoundedHttp};
use rmcp::{
    RoleClient, ServiceExt,
    service::RunningService,
    transport::{
        common::client_side_sse::NeverRetry,
        streamable_http_client::{
            StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
        },
    },
};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

pub async fn connect(
    endpoint: &str,
    brain: Uuid,
    token: Uuid,
) -> Result<RunningService<RoleClient, ()>> {
    let mut uri =
        reqwest::Url::parse(endpoint).map_err(|_| RuntimeError("agent_endpoint_invalid"))?;
    if !matches!(uri.scheme(), "http" | "https")
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.query().is_some()
        || uri.fragment().is_some()
        || !matches!(uri.path(), "" | "/")
        || brain.is_nil()
        || token.is_nil()
    {
        return Err(RuntimeError("agent_endpoint_invalid"));
    }
    if uri.scheme() == "http"
        && !matches!(
            uri.host_str(),
            Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
        )
    {
        return Err(RuntimeError("agent_endpoint_requires_https"));
    }
    uri.set_path(&format!("/api/brains/{brain}/mcp/agent"));
    let mut http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(65));
    if let Some(pem) = crate::agent_tls::configured_pem()? {
        let roots = reqwest::Certificate::from_pem_bundle(&pem)
            .map_err(|_| RuntimeError("agent_ca_file_invalid"))?;
        if roots.is_empty() || roots.len() > 16 {
            return Err(RuntimeError("agent_ca_file_invalid"));
        }
        http = http.tls_certs_merge(roots);
    }
    let http = http
        .build()
        .map_err(|_| RuntimeError("agent_http_unavailable"))?;
    let mut config = StreamableHttpClientTransportConfig::with_uri(uri.to_string())
        .reinit_on_expired_session(false)
        .max_sse_event_size(WIRE_LIMIT)
        .max_concurrent_requests(16)
        .control_request_timeout(Duration::from_secs(5));
    config.retry_config = Arc::new(NeverRetry::default());
    config.custom_headers.insert(
        http::header::AUTHORIZATION,
        http::HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| RuntimeError("agent_credential_invalid"))?,
    );
    tokio::time::timeout(
        Duration::from_secs(10),
        ().serve(StreamableHttpClientTransport::with_client(
            BoundedHttp(http),
            config,
        )),
    )
    .await
    .map_err(|_| RuntimeError("agent_connection_timeout"))?
    .map_err(|_| RuntimeError("agent_connection_unavailable"))
}
