//! Bounded POST bodies around the SDK's Streamable HTTP state machine. The SDK
//! still owns protocol headers, lifecycle, request matching and cancellation.
use crate::WIRE_LIMIT;
use futures::{StreamExt, stream::BoxStream};
use http::{HeaderName, HeaderValue};
use rmcp::{
    model::{ClientJsonRpcMessage, ClientRequest, JsonRpcMessage, ServerJsonRpcMessage},
    transport::streamable_http_client::{
        SseError, StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
    },
};
use sse_stream::{Sse, SseStream};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub(crate) struct BoundedHttp(pub reqwest::Client);
type Error = StreamableHttpError<reqwest::Error>;
fn invalid(code: &'static str) -> Error {
    StreamableHttpError::UnexpectedServerResponse(code.into())
}

impl StreamableHttpClient for BoundedHttp {
    type Error = reqwest::Error;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> std::result::Result<StreamableHttpPostResponse, Error> {
        self.post_message_with_max_sse_event_size(
            uri,
            message,
            session_id,
            auth_header,
            custom_headers,
            WIRE_LIMIT,
        )
        .await
    }
    async fn post_message_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
        max_sse_event_size: usize,
    ) -> std::result::Result<StreamableHttpPostResponse, Error> {
        let mut request = self
            .0
            .post(uri.as_ref())
            .header("accept", "application/json, text/event-stream");
        for (name, value) in custom_headers {
            request = request.header(name, value);
        }
        if let Some(value) = auth_header {
            request = request.bearer_auth(value);
        }
        if let Some(value) = &session_id {
            request = request.header("mcp-session-id", value.as_ref());
        }
        let mut response = request
            .json(&message)
            .send()
            .await
            .map_err(StreamableHttpError::Client)?;
        let status = response.status();
        if status == http::StatusCode::NOT_FOUND && session_id.is_some() {
            return Err(StreamableHttpError::SessionExpired);
        }
        if matches!(
            status,
            http::StatusCode::ACCEPTED | http::StatusCode::NO_CONTENT
        ) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        if matches!(
            status,
            http::StatusCode::UNAUTHORIZED | http::StatusCode::FORBIDDEN
        ) {
            return Err(invalid("provider_auth_rejected"));
        }
        let session = response
            .headers()
            .get("mcp-session-id")
            .and_then(|s| s.to_str().ok())
            .map(str::to_owned);
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|s| s.to_str().ok())
            .unwrap_or("");
        let is_json = content_type.starts_with("application/json");
        if status.is_success() && content_type.starts_with("text/event-stream") {
            let mut event_bytes = 0usize;
            let mut line_bytes = 0usize;
            let mut cr = false;
            let max = max_sse_event_size.min(WIRE_LIMIT);
            let bytes = response.bytes_stream().map(move |chunk| {
                let chunk =
                    chunk.map_err(|_| std::io::Error::other("provider_stream_interrupted"))?;
                for &byte in &chunk {
                    if cr && byte == b'\n' {
                        cr = false;
                        continue;
                    }
                    cr = byte == b'\r';
                    event_bytes = event_bytes.saturating_add(1);
                    if event_bytes > max {
                        return Err(std::io::Error::other("provider_event_too_large"));
                    }
                    if matches!(byte, b'\n' | b'\r') {
                        if line_bytes == 0 {
                            event_bytes = 0;
                        }
                        line_bytes = 0;
                    } else {
                        line_bytes += 1;
                    }
                }
                Ok(chunk)
            });
            return Ok(StreamableHttpPostResponse::Sse(
                SseStream::from_bytes_stream(bytes).boxed(),
                session,
            ));
        }
        if response
            .content_length()
            .is_some_and(|n| n > WIRE_LIMIT as u64)
        {
            return Err(invalid("provider_body_too_large"));
        }
        let mut bytes = vec![];
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(StreamableHttpError::Client)?
        {
            if bytes.len().saturating_add(chunk.len()) > WIRE_LIMIT {
                return Err(invalid("provider_body_too_large"));
            }
            bytes.extend_from_slice(&chunk);
        }
        if is_json
            && let Ok(reply) = serde_json::from_slice::<ServerJsonRpcMessage>(&bytes)
            && (status.is_success() || matches!(reply, JsonRpcMessage::Error(_)))
        {
            return Ok(StreamableHttpPostResponse::Json(reply, session));
        }
        // Match the SDK's documented legacy detection for sessionless discovery.
        // Never turn an authentication/server failure into a fallback signal.
        if session_id.is_none()
            && status.is_client_error()
            && let ClientJsonRpcMessage::Request(request) = &message
            && matches!(request.request, ClientRequest::DiscoverRequest(_))
        {
            let error = serde_json::from_value(serde_json::json!({
                "jsonrpc":"2.0", "id":request.id,
                "error":{"code":-32601,"message":"legacy_discovery_unavailable"}
            }))
            .map_err(|_| invalid("provider_protocol_invalid"))?;
            return Ok(StreamableHttpPostResponse::Json(error, None));
        }
        if status.is_success() && !matches!(message, ClientJsonRpcMessage::Request(_)) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        Err(invalid("provider_response_invalid"))
    }
    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> std::result::Result<(), Error> {
        self.0
            .delete_session(uri, session_id, auth_header, custom_headers)
            .await
    }
    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> std::result::Result<BoxStream<'static, std::result::Result<Sse, SseError>>, Error> {
        self.0
            .get_stream_with_max_sse_event_size(
                uri,
                session_id,
                last_event_id,
                auth_header,
                custom_headers,
                WIRE_LIMIT,
            )
            .await
    }
    async fn get_stream_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
        max: usize,
    ) -> std::result::Result<BoxStream<'static, std::result::Result<Sse, SseError>>, Error> {
        self.0
            .get_stream_with_max_sse_event_size(
                uri,
                session_id,
                last_event_id,
                auth_header,
                custom_headers,
                max.min(WIRE_LIMIT),
            )
            .await
    }
}
