use super::*;
use catalogue::{Scope, Spec};
use recollect_protocol::{ApiError, OperationBinding, RecallRequest};
use serde::Serialize;
use tower::ServiceExt;

#[derive(Serialize)]
pub(super) struct Failure {
    pub code: String,
    pub message: String,
}
impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Self {
            code: error.1.into(),
            message: error.2.into(),
        }
    }
}
type Result<T> = std::result::Result<T, Failure>;
fn invalid() -> Failure {
    Error::invalid("Use the documented tool arguments and resource IDs.").into()
}
fn unavailable() -> Failure {
    Error(
        StatusCode::SERVICE_UNAVAILABLE,
        "mcp_agent_service_unavailable",
        "The application response is unavailable. Inspect mutation history before retrying.",
    )
    .into()
}
fn id(value: &Value) -> Result<Uuid> {
    value
        .as_str()
        .and_then(|v| v.parse().ok())
        .ok_or_else(invalid)
}

impl AgentService {
    async fn request(
        &self,
        method: &str,
        suffix: &str,
        body: Option<Value>,
        key: Option<Uuid>,
    ) -> Result<Value> {
        let bytes = body
            .map(|b| serde_json::to_vec(&b))
            .transpose()
            .map_err(|_| invalid())?;
        let mut request = axum::http::Request::builder()
            .method(method)
            .uri(format!("/api/brains/{}{suffix}", self.brain))
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.auth.session),
            );
        if bytes.is_some() {
            request = request.header(header::CONTENT_TYPE, "application/json");
        }
        if let Some(key) = key {
            request = request.header("idempotency-key", key.to_string());
        }
        let request = request
            .body(axum::body::Body::from(bytes.unwrap_or_default()))
            .map_err(|_| invalid())?;
        // This router contains only the existing application routes, never the
        // MCP adapter itself. Each handler authenticates and opens its own tx.
        let response = self
            .routes
            .clone()
            .oneshot(request)
            .await
            .map_err(|_| unavailable())?;
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), RESPONSE_LIMIT)
            .await
            .map_err(|_| unavailable())?;
        if !status.is_success() {
            return Err(match serde_json::from_slice::<ApiError>(&bytes) {
                Ok(error) if error.code.len() <= 100 && error.message.len() <= 1000 => Failure {
                    code: error.code,
                    message: error.message,
                },
                _ => unavailable(),
            });
        }
        if bytes.is_empty() {
            return Ok(json!({"state":"completed"}));
        }
        serde_json::from_slice(&bytes).map_err(|_| unavailable())
    }

    async fn operation(&self, operation: Uuid, kind: Scope) -> Result<OperationBinding> {
        let mut tx = self.auth.tx(&self.state.pool).await?;
        db::require_role(&mut tx, self.brain, false).await?;
        let bound = crate::workspace::bound_operation(&mut tx, self.brain, operation).await?;
        let valid_kind = match kind {
            Scope::Write | Scope::Handover => bound.kind == "write",
            Scope::Managed | Scope::ExistingCall => bound.kind == "tool",
            Scope::Discover => matches!(bound.kind.as_str(), "context" | "retrieval" | "tool"),
            _ => matches!(bound.kind.as_str(), "context" | "retrieval"),
        };
        if bound.actor_id != self.auth.user.id
            || bound.device_id != self.auth.device_id
            || !bound.scope_valid
            || !valid_kind
        {
            return Err(Error::forbidden().into());
        }
        tx.commit().await.map_err(Error::from)?;
        Ok(bound)
    }

    pub(super) async fn dispatch(&self, spec: &Spec, arguments: Value) -> Result<Value> {
        if matches!(
            spec.tool.name.as_ref(),
            "workspace.start_task" | "workspace.set_scope"
        ) && arguments["context_query"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
        {
            return Err(invalid());
        }
        let operation = if spec.scope == Scope::None {
            None
        } else {
            Some(
                self.operation(id(&arguments["operation_id"])?, spec.scope)
                    .await?,
            )
        };
        let mut input = arguments.get("input").cloned();
        if let Some(bound) = &operation {
            let value = json!(bound.scope.selection);
            if let Some(input) = input.as_mut() {
                match spec.scope {
                    Scope::Read => {
                        input["selection"] = value;
                        input["operation_id"] = json!(bound.id);
                    }
                    Scope::Write => {
                        input["content"]["selection"] = value;
                        input["operation_id"] = json!(bound.id);
                    }
                    Scope::Graph => {
                        input["scope"]["selection"] = value;
                        input["scope"]["operation_id"] = json!(bound.id);
                    }
                    Scope::Managed | Scope::Discover => {
                        input["environment_id"] = json!(bound.scope.selection.environment_id);
                        input["operation_id"] = json!(bound.id);
                    }
                    Scope::Handover => input["operation_id"] = json!(bound.id),
                    _ => {}
                }
            }
        }
        let mut suffix = spec.path.to_owned();
        if suffix.contains("{id}") {
            suffix = suffix.replace("{id}", &id(&arguments["id"])?.to_string());
        }
        let mut method = spec.method;
        if spec.tool.name == "memory.contribute"
            && let Some(claim) = arguments.get("claim_id")
        {
            suffix = format!("/claims/{}", id(claim)?);
            method = "PUT";
        }
        if spec.scope == Scope::ExistingCall {
            let call = self
                .request(
                    "GET",
                    &format!("/mcp/calls/{}", id(&arguments["id"])?),
                    None,
                    None,
                )
                .await?;
            if call["operation_id"] != arguments["operation_id"] {
                return Err(Error::forbidden().into());
            }
            if spec.tool.name == "mcp.status" {
                return Ok(call);
            }
        }
        let mut query = arguments
            .get("query")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if matches!(
            spec.tool.name.as_ref(),
            "memory.inspect" | "memory.review_history" | "memory.handover_status"
        ) {
            query.insert("operation_id".into(), arguments["operation_id"].clone());
        }
        if !query.is_empty() {
            let mut encoded = reqwest::Url::parse("http://localhost/").map_err(|_| invalid())?;
            for (key, value) in query {
                let value = value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string());
                encoded.query_pairs_mut().append_pair(&key, &value);
            }
            suffix.push('?');
            suffix.push_str(encoded.query().unwrap_or_default());
        }
        let key = arguments.get("request_id").map(id).transpose()?;
        let mut reply = self.request(method, &suffix, input, key).await?;
        if matches!(
            spec.tool.name.as_ref(),
            "workspace.start_task" | "workspace.set_scope"
        ) {
            let refresh = self
                .context(
                    &reply,
                    arguments["context_query"].as_str().ok_or_else(invalid)?,
                )
                .await;
            reply["context"] = match refresh {
                Ok(context) => context,
                Err(error) => json!({"state":"unavailable","error":error,
                    "instruction":"The scope change committed. Inspect the current task and request fresh context; do not reuse previous scope context."}),
            };
        }
        Ok(reply)
    }

    async fn context(&self, change: &Value, query: &str) -> Result<Value> {
        let task = id(&change["task"]["id"])?;
        let snapshot = id(&change["handoff"]["current_scope_id"])?;
        let inventory = self.request("GET", "/workspace", None, None).await?;
        let operation = self
            .request(
                "POST",
                &format!("/workspace/tasks/{task}/operations"),
                Some(json!({"kind":"retrieval","expected_scope":snapshot})),
                None,
            )
            .await?;
        let operation: OperationBinding =
            serde_json::from_value(operation).map_err(|_| unavailable())?;
        let input = RecallRequest {
            query: query.into(),
            operation_id: Some(operation.id),
            selection: operation.scope.selection.clone(),
            limit: 6,
            context_bytes: 16 * 1024,
            ..Default::default()
        };
        let recall = self
            .request(
                "POST",
                "/recall",
                Some(serde_json::to_value(input).map_err(|_| invalid())?),
                None,
            )
            .await?;
        Ok(
            json!({"state":"ready","scope_id":snapshot,"operation_id":operation.id,"inventory":inventory,"recall":recall}),
        )
    }
}
