use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, DiscoverResult,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
    },
    service::RequestContext,
};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Default)]
pub struct Fixture {
    pub calls: Arc<AtomicUsize>,
    pub legacy: bool,
    pub marker: Option<std::path::PathBuf>,
    pub change: Option<Arc<tokio::sync::Notify>>,
    pub listening: Arc<tokio::sync::Notify>,
}
pub fn input() -> Value {
    json!({"type":"object","properties":{
        "text":{"type":"string"},"millis":{"type":"integer","minimum":0,"maximum":30000},
        "operation_id":{"type":"string"}},"additionalProperties":false})
}
pub fn definition(command: Option<String>) -> recollect_protocol::McpDefinitionManifest {
    let tools = [
        "inspect", "slow", "effect", "receipt", "error", "large", "reject",
    ]
    .into_iter()
    .map(|name| {
        json!({
        "name":name,"description":"Synthetic fixture operation","inputSchema":input(),
        "outputSchema":null,"annotations":{}})
    })
    .collect::<Vec<_>>();
    let arguments = if command.is_some() {
        vec!["serve"]
    } else {
        vec![]
    };
    serde_json::from_value(json!({
        "key":"runtime-fixture","name":"Runtime fixture","description":"Owned runtime integration fixture",
        "transport":if command.is_some(){"stdio"}else{"streamable_http"},"command":command,
        "arguments":arguments,"placements":["central","local"],"credential_aliases":["fixture"],
        "configuration_schema":{"type":"object","additionalProperties":false},
        "tools":tools
    })).expect("synthetic manifest")
}
impl ServerHandler for Fixture {
    fn get_info(&self) -> ServerConfig {
        let mut config = ServerConfig::default();
        config.capabilities = ServerCapabilities::builder().enable_tools().build();
        if self.change.is_some() {
            config.capabilities = ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build();
        }
        config
    }
    fn accepted_subscription_filter(
        &self,
        requested: &rmcp::model::SubscriptionFilter,
    ) -> Option<rmcp::model::SubscriptionFilter> {
        self.change
            .as_ref()
            .map(|_| requested.supported_by(&self.get_info().capabilities))
    }
    async fn listen(&self, context: rmcp::service::SubscriptionContext) -> Result<(), ErrorData> {
        self.listening.notify_one();
        if let Some(change) = &self.change {
            tokio::select! {
                _ = change.notified() => { context.sink().notify_tool_list_changed().await.map_err(|_|ErrorData::internal_error("Fixture subscription closed",None))?; },
                _ = context.cancelled() => return Ok(()),
            }
        }
        context.cancelled().await;
        Ok(())
    }
    async fn discover(&self, _: RequestContext<RoleServer>) -> Result<DiscoverResult, ErrorData> {
        if self.legacy {
            return Err(ErrorData::method_not_found::<
                rmcp::model::DiscoverRequestMethod,
            >());
        }
        Ok(DiscoverResult::from_server_info(
            vec![rmcp::model::ProtocolVersion::V_2026_07_28],
            self.get_info(),
        ))
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(
            [
                "inspect", "slow", "effect", "receipt", "error", "large", "reject",
            ]
            .into_iter()
            .map(|name| {
                Tool::new(
                    name.to_string(),
                    "Synthetic fixture operation",
                    Arc::new(input().as_object().unwrap().clone()),
                )
            })
            .collect(),
        ))
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let arguments = request.arguments.unwrap_or_default();
        if let Some(marker) = &self.marker {
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(marker)
                .await
                .unwrap();
            use tokio::io::AsyncWriteExt;
            file.write_all(format!("call {}\n", request.name).as_bytes())
                .await
                .unwrap();
            file.sync_all().await.unwrap();
        }
        match request.name.as_ref() {
            "reject" => {
                return Err(ErrorData::invalid_params(
                    "Synthetic invalid arguments",
                    None,
                ));
            }
            "slow" => {
                tokio::time::sleep(std::time::Duration::from_millis(
                    arguments
                        .get("millis")
                        .and_then(Value::as_u64)
                        .unwrap_or(150),
                ))
                .await
            }
            "effect" if self.marker.is_some() => {
                if let Some(id) = arguments.get("operation_id").and_then(Value::as_str) {
                    use tokio::io::AsyncWriteExt;
                    let mut file = tokio::fs::OpenOptions::new()
                        .append(true)
                        .open(self.marker.as_ref().unwrap())
                        .await
                        .unwrap();
                    file.write_all(format!("effect {id}\n").as_bytes())
                        .await
                        .unwrap();
                    file.sync_all().await.unwrap();
                }
                std::process::exit(0);
            }
            "receipt" => {
                let id = arguments
                    .get("operation_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let recorded = match &self.marker {
                    Some(marker) => tokio::fs::read_to_string(marker).await.unwrap_or_default(),
                    None => String::new(),
                };
                let observed = recorded.lines().any(|line| line == format!("effect {id}"));
                return Ok(CallToolResult::structured(
                    json!({"outcome":if observed {"succeeded"} else {"unknown"},"operation_id":id}),
                )
                .into());
            }
            "error" => {
                return Ok(
                    CallToolResult::error(vec![ContentBlock::text("Synthetic tool error")]).into(),
                );
            }
            "large" => {
                return Ok(CallToolResult::success(vec![ContentBlock::text(
                    "x".repeat(crate::WIRE_BOUND_FOR_FIXTURE),
                )])
                .into());
            }
            _ => {}
        }
        let value = json!({"call":call,"arguments":arguments,"pid":std::process::id(),
            "root_token_leaked":std::env::var_os("VAULT_TOKEN").is_some(),
            "kv_pair_coherent":std::env::var("FIXTURE_USER").ok().zip(std::env::var("FIXTURE_CREDENTIAL").ok()).is_some_and(|(u,p)|u.strip_prefix("fixture-user-").is_some_and(|v| p.strip_prefix("fixture-password-")==Some(v))),
            "ambient_leaked":std::env::var_os("RECOLLECT_MCP_AMBIENT_TEST").is_some(),
            "device_leaked":std::env::var_os("RECOLLECT_DEVICE_PROFILE").is_some(),
            "echo":std::env::var("FIXTURE_CREDENTIAL").ok(),
            "numeric_echo":std::env::var("FIXTURE_CREDENTIAL").ok().and_then(|v|v.parse::<u64>().ok()),
            "target":std::env::var("RECOLLECT_MCP_TARGET").ok()});
        Ok(CallToolResult::structured(value).into())
    }
}
