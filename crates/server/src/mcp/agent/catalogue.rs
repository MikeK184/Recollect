use super::*;
use rmcp::model::Tool;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
use utoipa::OpenApi;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    None,
    Read,
    Write,
    Graph,
    Managed,
    ExistingCall,
    Discover,
    Handover,
}
pub(super) struct Spec {
    pub tool: Tool,
    pub validator: jsonschema::Validator,
    pub writer: bool,
    pub scope: Scope,
    pub method: &'static str,
    pub path: &'static str,
}

fn uuid() -> Value {
    json!({"type":"string","format":"uuid"})
}
fn dereference(value: &Value, definitions: &Value) -> Result<Value, String> {
    match value.get("$ref").and_then(Value::as_str) {
        Some(reference) => definitions
            .get(reference.rsplit('/').next().unwrap_or_default())
            .cloned()
            .ok_or_else(|| "missing_schema".into()),
        None => Ok(value.clone()),
    }
}
fn remove(value: &mut Value, path: &[&str], definitions: &Value) -> Result<(), String> {
    *value = dereference(value, definitions)?;
    // These are typed API objects. Removed authority fields must be rejected,
    // including nested content/scope, rather than accepted as unknown extras.
    if value.get("properties").is_some_and(Value::is_object) {
        value["additionalProperties"] = json!(false);
    }
    let Some((field, tail)) = path.split_first() else {
        return Ok(());
    };
    if tail.is_empty() {
        if let Some(properties) = value.get_mut("properties").and_then(Value::as_object_mut) {
            properties.remove(*field);
        }
        if let Some(required) = value.get_mut("required").and_then(Value::as_array_mut) {
            required.retain(|v| v != field);
        }
    } else if let Some(child) = value
        .get_mut("properties")
        .and_then(|properties| properties.get_mut(*field))
    {
        remove(child, tail, definitions)?;
    }
    Ok(())
}
fn references(value: &mut Value, pending: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(reference)) = map.get_mut("$ref")
                && let Some(name) = reference.strip_prefix("#/components/schemas/")
            {
                let name = name.to_string();
                pending.insert(name.clone());
                *reference = format!("#/$defs/{name}");
            }
            for child in map.values_mut() {
                references(child, pending);
            }
        }
        Value::Array(values) => {
            for child in values {
                references(child, pending);
            }
        }
        _ => {}
    }
}

fn build() -> Result<Vec<Spec>, String> {
    let api =
        serde_json::to_value(crate::ApiDoc::openapi()).map_err(|_| "api_schema_unavailable")?;
    let definitions = &api["components"]["schemas"];
    // Routes are an internal closed vocabulary. Arguments never select a URL.
    let entries = [
        (
            "workspace.list",
            "List this Brain's repository, area, environment and own task inventory. Does not start tools or extract repositories.",
            "GET",
            "/workspace",
            "",
            Scope::None,
        ),
        (
            "workspace.start_task",
            "Create a task or independent child with explicit scope, and return fresh exact/lexical context for context_query.",
            "POST",
            "/workspace/tasks",
            "CreateTask",
            Scope::None,
        ),
        (
            "workspace.set_scope",
            "Change a task's future scope from base_scope and return new context. Existing operations and child scopes remain unchanged.",
            "PUT",
            "/workspace/tasks/{id}/scope",
            "ChangeScope",
            Scope::None,
        ),
        (
            "workspace.inspect_task",
            "Read this account's current task and paginated immutable scope/operation history.",
            "GET",
            "/workspace/tasks/{id}",
            "",
            Scope::None,
        ),
        (
            "workspace.begin",
            "Begin an immutable context, retrieval, write, capture or tool operation. expected_scope prevents a changed default from being substituted.",
            "POST",
            "/workspace/tasks/{id}/operations",
            "StartOperation",
            Scope::None,
        ),
        (
            "workspace.close",
            "Close a task to new operations. Its children and existing operations keep their own scope.",
            "POST",
            "/workspace/tasks/{id}/close",
            "",
            Scope::None,
        ),
        (
            "memory.recall",
            "Recall qualified attributed evidence under this operation's scope. Exact/lexical is the default; semantic/graph are explicit governed channels.",
            "POST",
            "/recall",
            "RecallRequest",
            Scope::Read,
        ),
        (
            "memory.inspect",
            "Inspect an exact claim and retained history applicable to this operation. History and rejected assertions are not current accepted knowledge.",
            "GET",
            "/claims/{id}",
            "",
            Scope::Read,
        ),
        (
            "memory.review_history",
            "Inspect applicable review decisions, rejection rules and conflicts. Cannot submit human review; hidden transition bodies are withheld.",
            "GET",
            "/claims/{id}/review",
            "",
            Scope::Read,
        ),
        (
            "memory.contribute",
            "Propose an evidence-backed claim, decision or procedure, or revise an unreviewed claim. Preserves device provenance; cannot label a contribution human-reviewed.",
            "POST",
            "/claims",
            "ClaimInput",
            Scope::Write,
        ),
        (
            "memory.handover",
            "Generate a handover from exact contributions under standing model policy and a write operation with the derived combined scope.",
            "POST",
            "/handovers",
            "HandoverInput",
            Scope::Handover,
        ),
        (
            "memory.handover_status",
            "Read a paginated handover status list, filtered by this operation's applicability.",
            "GET",
            "/handovers",
            "",
            Scope::Read,
        ),
        (
            "memory.graph_explore",
            "Explore bounded native graph relationships with the operation's scope and explicit snapshot/manifest. Graph proximity is not truth.",
            "POST",
            "/graph/explore",
            "GraphExploreRequest",
            Scope::Graph,
        ),
        (
            "memory.graph_path",
            "Find a bounded native graph path with the operation's scope. Preserve exact manifest, direction and relationship filters.",
            "POST",
            "/graph/path",
            "GraphPathRequest",
            Scope::Graph,
        ),
        (
            "mcp.profiles",
            "List configured execution profiles and separate Use/Manage/Share rights. Brain access never implies tool Use.",
            "GET",
            "/mcp",
            "",
            Scope::None,
        ),
        (
            "mcp.discover",
            "Read an authorized profile's approved cached tools under the operation's environment; does not start providers or fetch credentials.",
            "POST",
            "/mcp/discover",
            "McpDiscoverRequest",
            Scope::Discover,
        ),
        (
            "mcp.call",
            "Queue one authorized managed tool call with a stable request ID and immutable tool operation. Approved target, runner and credentials cannot be changed here.",
            "POST",
            "/mcp/calls",
            "McpCallInput",
            Scope::Managed,
        ),
        (
            "mcp.status",
            "Inspect the call belonging to the supplied original operation, including current outcome, retained output and later resolutions.",
            "GET",
            "/mcp/calls/{id}",
            "",
            Scope::ExistingCall,
        ),
        (
            "mcp.cancel",
            "Request cancellation of this operation's managed call. Cancellation does not prove an external effect did not occur.",
            "POST",
            "/mcp/calls/{id}/cancel",
            "",
            Scope::ExistingCall,
        ),
        (
            "mcp.reconcile",
            "Invoke the approved receipt tool for this operation's uncertain call; never replay its original effect.",
            "POST",
            "/mcp/calls/{id}/reconcile",
            "McpReconcileInput",
            Scope::ExistingCall,
        ),
        (
            "mcp.resolve",
            "Attach scoped retained evidence about this operation's uncertain call. The original uncertainty remains visible.",
            "POST",
            "/mcp/calls/{id}/resolve",
            "McpResolveInput",
            Scope::ExistingCall,
        ),
        (
            "mcp.release",
            "Release this account's idle connections for the named client session. Active work retains its leases.",
            "POST",
            "/mcp/session/release",
            "McpSessionRelease",
            Scope::None,
        ),
    ];
    let mut result = Vec::new();
    for (name, description, method, path, input_type, scope) in entries {
        let writer = matches!(scope, Scope::Write | Scope::Handover);
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        if !input_type.is_empty() {
            let mut input = definitions
                .get(input_type)
                .cloned()
                .ok_or_else(|| format!("missing_{input_type}"))?;
            match scope {
                Scope::Read => {
                    remove(&mut input, &["selection"], definitions)?;
                    remove(&mut input, &["operation_id"], definitions)?;
                }
                Scope::Write => {
                    remove(&mut input, &["content", "selection"], definitions)?;
                    remove(&mut input, &["operation_id"], definitions)?;
                }
                Scope::Graph => {
                    remove(&mut input, &["scope", "selection"], definitions)?;
                    remove(&mut input, &["scope", "operation_id"], definitions)?;
                }
                Scope::Managed | Scope::Discover => {
                    remove(&mut input, &["environment_id"], definitions)?;
                    remove(&mut input, &["operation_id"], definitions)?;
                }
                Scope::Handover => remove(&mut input, &["operation_id"], definitions)?,
                _ => {}
            }
            input["additionalProperties"] = json!(false);
            properties.insert("input".into(), input);
            required.push("input");
        }
        if scope != Scope::None {
            properties.insert("operation_id".into(), uuid());
            required.push("operation_id");
        }
        if path.contains("{id}") {
            properties.insert("id".into(), uuid());
            required.push("id");
        }
        if name == "memory.contribute" {
            properties.insert("claim_id".into(), uuid());
        }
        if writer {
            properties.insert("request_id".into(), uuid());
            required.push("request_id");
        }
        if matches!(name, "workspace.start_task" | "workspace.set_scope") {
            properties.insert(
                "context_query".into(),
                json!({"type":"string","minLength":1,"maxLength":2000}),
            );
            required.push("context_query");
        }
        if method == "GET" {
            let mut query = serde_json::Map::new();
            for field in match name {
                "workspace.list" => &["checkout_offset", "task_offset"][..],
                "workspace.inspect_task" => &["scope_offset", "operation_offset"][..],
                "memory.inspect" | "memory.review_history" | "memory.handover_status" => {
                    &["offset"][..]
                }
                _ => &[][..],
            } {
                query.insert((*field).into(), json!({"type":"integer","minimum":0}));
            }
            if name == "workspace.list" {
                query.insert("workspace_id".into(), uuid());
            }
            if name == "memory.inspect" {
                for field in ["fact_at", "knowledge_at"] {
                    query.insert(field.into(), json!({"type":"string","format":"date-time"}));
                }
            }
            if !query.is_empty() {
                properties.insert(
                    "query".into(),
                    json!({"type":"object","properties":query,"additionalProperties":false}),
                );
            }
        }
        let mut schema = json!({"type":"object","properties":properties,"required":required,"additionalProperties":false});
        let mut pending = BTreeSet::new();
        references(&mut schema, &mut pending);
        let mut used = BTreeMap::new();
        while let Some(key) = pending.pop_first() {
            if used.contains_key(&key) {
                continue;
            }
            let mut definition = definitions
                .get(&key)
                .cloned()
                .ok_or("missing_dependency_schema")?;
            references(&mut definition, &mut pending);
            used.insert(key, definition);
        }
        if !used.is_empty() {
            schema["$defs"] = json!(used);
        }
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .offline()
            .should_validate_formats(true)
            .build(&schema)
            .map_err(|error| {
                // Only the compile-time API schema is involved, never a request.
                tracing::warn!(schema = name, error = %error, "Generated tool schema is invalid");
                format!("invalid_{name}_schema")
            })?;
        let tool = serde_json::from_value(json!({"name":name,"description":description,"inputSchema":schema,
            "annotations":{"readOnlyHint": method=="GET" || matches!(name,"memory.recall"|"memory.graph_explore"|"memory.graph_path"|"mcp.discover"),
            "destructiveHint": matches!(name,"workspace.close"|"mcp.call"|"mcp.cancel"),"openWorldHint":name.starts_with("mcp.")}})).map_err(|_| "invalid_tool_schema")?;
        result.push(Spec {
            tool,
            validator,
            writer,
            scope,
            method,
            path,
        });
    }
    Ok(result)
}

pub(super) fn registry() -> Result<&'static [Spec], ErrorData> {
    static TOOLS: OnceLock<Result<Vec<Spec>, String>> = OnceLock::new();
    TOOLS.get_or_init(build).as_deref().map_err(|code| {
        tracing::warn!(code = %code, "Agent tool catalogue schema failed");
        ErrorData::internal_error("Agent tool catalogue is unavailable.", None)
    })
}
