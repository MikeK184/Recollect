//! Narrow official A2A 1.0 adapter using the official Rust SDK's wire types.
use super::*;
use ::a2a::{Message, Part, PartContent, Role, SendMessageRequest, SendMessageResponse};
use axum::{
    body::Bytes,
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
fn failure(error: Error) -> Response {
    let (status, mut reason) = match error.0 {
        StatusCode::UNAUTHORIZED => ("UNAUTHENTICATED", "AUTHENTICATION_REQUIRED"),
        StatusCode::FORBIDDEN => ("PERMISSION_DENIED", "PERMISSION_DENIED"),
        StatusCode::NOT_FOUND => ("NOT_FOUND", "NOT_FOUND"),
        StatusCode::CONFLICT => ("ALREADY_EXISTS", "INVALID_ARGUMENT"),
        StatusCode::TOO_MANY_REQUESTS => ("RESOURCE_EXHAUSTED", "RESOURCE_EXHAUSTED"),
        StatusCode::NOT_IMPLEMENTED => ("UNIMPLEMENTED", "UNSUPPORTED_OPERATION"),
        _ => ("INVALID_ARGUMENT", "INVALID_ARGUMENT"),
    };
    if error.1 == "unsupported_operation" {
        reason = "UNSUPPORTED_OPERATION";
    }
    if error.1 == "version_not_supported" {
        reason = "VERSION_NOT_SUPPORTED";
    }
    if error.1 == "content_type_not_supported" {
        reason = "CONTENT_TYPE_NOT_SUPPORTED";
    }
    if error.1 == "task_not_found" {
        reason = "TASK_NOT_FOUND";
    }
    (error.0,[(header::CONTENT_TYPE,"application/a2a+json"),(axum::http::HeaderName::from_static("a2a-version"),"1.0")],Json(json!({"error":{"code":error.0.as_u16(),"status":status,"message":error.2,"details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":reason,"domain":"a2a-protocol.org"}]}}))).into_response()
}
#[utoipa::path(get,path="/api/brains/{brain}/coordination/a2a/.well-known/agent-card.json",operation_id="coordinationA2aAgentCard",params(("brain"=Uuid,Path)),responses((status=200,body=Value)))]
pub async fn card(
    State(state): State<AppState>,
    auth: std::result::Result<Auth, Error>,
    Path(brain): Path<Uuid>,
) -> Response {
    let auth = match auth {
        Ok(a) => a,
        Err(e) => return failure(e),
    };
    let tx = match tx(&state, &auth, brain, false).await {
        Ok(t) => t,
        Err(e) => return failure(e),
    };
    if let Err(e) = tx.commit().await {
        return failure(e.into());
    }
    Json(json!({"name":"Recollect Brain coordination","description":"Authenticated advisory peer messages; presence and durable inbox use the native coordination API.","version":"0.1.0","supportedInterfaces":[{"url":format!("{}/api/brains/{brain}/coordination/a2a",state.config.public_origin.trim_end_matches('/')),"protocolBinding":"HTTP+JSON","protocolVersion":"1.0"}],"capabilities":{"streaming":false,"pushNotifications":false},"defaultInputModes":["application/json"],"defaultOutputModes":["application/json"],"skills":[{"id":"recollect.coordination.send","name":"Send a peer coordination message","description":"Queue an attributed note or proposal between active Brain participants. Receipt means queued, not peer acceptance.","tags":["coordination"],"inputModes":["application/json"],"outputModes":["application/json"]}],"securitySchemes":{"recollect":{"httpAuthSecurityScheme":{"scheme":"Bearer"}}},"securityRequirements":[{"schemes":{"recollect":{"list":[]}}}]})).into_response()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    schema: String,
    action: String,
    message: CoordinationMessageInput,
}
#[utoipa::path(post,path="/api/brains/{brain}/coordination/a2a/message:send",operation_id="coordinationA2aSendMessage",params(("brain"=Uuid,Path)),request_body=Value,responses((status=200,body=Value)))]
pub async fn send(
    State(state): State<AppState>,
    auth: std::result::Result<Auth, Error>,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Query(parameters): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let auth = match auth {
        Ok(a) => a,
        Err(e) => return failure(e),
    };
    let run = async {
        let header_version = headers.get("a2a-version").and_then(|v| v.to_str().ok());
        let query_version = parameters.get("A2A-Version").map(String::as_str);
        if header_version.is_some() && query_version.is_some() && header_version != query_version {
            return Err(Error::invalid("Conflicting A2A protocol versions."));
        }
        if header_version.or(query_version).unwrap_or("0.3") != "1.0" {
            return Err(Error(
                StatusCode::BAD_REQUEST,
                "version_not_supported",
                "This endpoint supports A2A 1.0. Missing version means unsupported 0.3.",
            ));
        }
        if headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_none_or(|v| v.split(';').next() != Some("application/a2a+json"))
        {
            return Err(Error::invalid("Use Content-Type: application/a2a+json."));
        }
        let raw = crate::strict_json::from_slice(&body)
            .map_err(|_| Error::invalid("Invalid A2A JSON."))?;
        if raw
            .get("message")
            .is_some_and(|m| m.get("kind").is_some() || m.get("type").is_some())
        {
            return Err(Error::invalid("Use A2A 1.0 envelopes."));
        }
        let input: SendMessageRequest = serde_json::from_value(raw)
            .map_err(|_| Error::invalid("Invalid A2A 1.0 SendMessageRequest."))?;
        if input
            .configuration
            .as_ref()
            .and_then(|c| c.accepted_output_modes.as_ref())
            .is_some_and(|modes| !modes.iter().any(|m| m == "application/json"))
        {
            return Err(Error(
                StatusCode::BAD_REQUEST,
                "content_type_not_supported",
                "The coordination skill returns application/json receipts.",
            ));
        }
        if input.message.task_id.is_some() {
            return Err(Error(
                StatusCode::NOT_FOUND,
                "task_not_found",
                "This coordination endpoint has no A2A tasks.",
            ));
        }
        if input.message.role != Role::User
            || input.message.reference_task_ids.is_some()
            || input.message.parts.len() != 1
            || input.tenant.is_some()
            || input.configuration.as_ref().is_some_and(|c| {
                c.task_push_notification_config.is_some()
                    || c.history_length.is_some()
                    || c.return_immediately == Some(true)
            })
        {
            return Err(Error(
                StatusCode::BAD_REQUEST,
                "unsupported_operation",
                "Only synchronous coordination data messages are supported.",
            ));
        }
        if input
            .message
            .context_id
            .as_ref()
            .is_some_and(|s| !matching::text(s, 200, false))
        {
            return Err(Error::invalid("Use a bounded printable context ID."));
        }
        let part = &input.message.parts[0];
        if part.media_type.as_deref() != Some("application/json") {
            return Err(Error::invalid(
                "The coordination part requires application/json.",
            ));
        }
        let PartContent::Data(data) = &part.content else {
            return Err(Error::invalid(
                "The coordination skill requires a structured data part.",
            ));
        };
        let data: Data = serde_json::from_value(data.clone())
            .map_err(|_| Error::invalid("Invalid coordination data."))?;
        if data.schema != "recollect.coordination.v1"
            || data.action != "send"
            || input.message.message_id != data.message.id.to_string()
        {
            return Err(Error::invalid(
                "Use the coordination send schema and identical message IDs.",
            ));
        }
        let receipt = send_message(&state, &auth, brain, data.message).await?;
        let mut part = Part::data(json!({"schema":"recollect.coordination.v1","receipt":receipt}));
        part.media_type = Some("application/json".into());
        let mut output = Message::new(Role::Agent, vec![part]);
        output.message_id = format!("{}-receipt", receipt.id);
        output.context_id = input.message.context_id;
        Ok(SendMessageResponse::Message(output))
    };
    match run.await {
        Ok(value) => (
            [
                (header::CONTENT_TYPE, "application/a2a+json"),
                (axum::http::HeaderName::from_static("a2a-version"), "1.0"),
            ],
            Json(value),
        )
            .into_response(),
        Err(e) => failure(e),
    }
}
pub async fn unsupported() -> Response {
    failure(Error(
        StatusCode::BAD_REQUEST,
        "unsupported_operation",
        "This A2A operation is not implemented by the coordination skill.",
    ))
}
