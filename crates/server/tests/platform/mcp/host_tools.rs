//! Actual host binaries, native bridge, paired HTTP handlers, PostgreSQL and OS
//! Secret Service. Synthetic model endpoints never need a paid provider key.
use super::*;
use axum::{
    body::to_bytes,
    extract::{Request, State},
    response::{IntoResponse, Response},
};
use std::sync::{Arc, Mutex};

const MARKER: &str = "HOST_FRESH_SCOPE_B_CONTEXT";
#[derive(Clone)]
pub(super) struct Model {
    pub(super) automatic: bool,
    pub(super) prompt: &'static str,
    pub(super) calls: usize,
    pub(super) args: Value,
    pub(super) saw_context: bool,
    pub(super) saw_recall_tool: bool,
    pub(super) names: Vec<String>,
    pub(super) bodies: Vec<Value>,
}
fn stream(events: Vec<(&str, Value)>) -> Response {
    let text: String = events
        .into_iter()
        .map(|(name, v)| format!("event: {name}\ndata: {v}\n\n"))
        .collect();
    ([("content-type", "text/event-stream")], text).into_response()
}
pub(super) async fn provider(State(state): State<Arc<Mutex<Model>>>, request: Request) -> Response {
    let path = request.uri().path().to_string();
    let bytes = to_bytes(request.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or_default();
    if path.ends_with("count_tokens") {
        return axum::Json(json!({"input_tokens":42})).into_response();
    }
    let mut callable = Vec::new();
    let mut code_mode = None;
    let specifications = body["tools"].as_array().into_iter().flatten().chain(
        body["input"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| item["type"] == "additional_tools")
            .flat_map(|item| item["tools"].as_array().into_iter().flatten()),
    );
    for spec in specifications {
        if spec["type"] == "namespace" {
            for tool in spec["tools"].as_array().into_iter().flatten() {
                if let (Some(namespace), Some(name)) =
                    (spec["name"].as_str(), tool["name"].as_str())
                {
                    if namespace == "functions" && name == "exec" && tool["type"] == "custom" {
                        code_mode = Some((namespace.to_owned(), name.to_owned()));
                    }
                    callable.push((Some(namespace.to_owned()), name.to_owned()));
                }
            }
        } else if let Some(name) = spec["name"].as_str() {
            callable.push((None, name.to_owned()));
        }
    }
    let names = callable
        .iter()
        .map(|(namespace, name)| {
            namespace
                .as_ref()
                .map_or_else(|| name.clone(), |ns| format!("{ns}__{name}"))
        })
        .collect::<Vec<_>>();
    let mut model = state.lock().unwrap();
    let main = body.to_string().contains(model.prompt)
        && (path.ends_with("responses") || path.ends_with("messages"))
        && (!model.automatic || !callable.is_empty());
    if !main {
        return axum::Json(json!({"id":"ancillary","type":"message","role":"assistant","model":"claude-sonnet-4-6","content":[{"type":"text","text":"Synthetic auxiliary reply"}],"stop_reason":"end_turn","usage":{"input_tokens":42,"output_tokens":8}})).into_response();
    }
    model.names.extend(names.clone());
    let step = model.calls;
    model.calls += 1;
    model.saw_context |= step >= 2 && body.to_string().contains(MARKER);
    // Code mode advertises deferred MCP tools through its live ALL_TOOLS list.
    // The generated script below never contains this name; seeing it on a later
    // model turn proves the host actually returned its tool catalogue.
    model.saw_recall_tool |=
        step >= 1 && body.to_string().contains("mcp__recollect__memory_recall");
    model.bodies.push(body.clone());
    let suffix = if step == 0 {
        "workspace_list"
    } else {
        "workspace_set_scope"
    };
    let steps = if model.automatic { 1 } else { 2 };
    let mut tool = if step < steps {
        callable
            .iter()
            .find(|(_, name)| name.replace('.', "_").ends_with(suffix))
            .cloned()
    } else {
        None
    };
    let mut args = if step == 0 {
        json!({})
    } else {
        model.args.clone()
    };
    // OpenCode V2 exposes MCP through its native execute tool by default.
    // Exercise that installed default instead of changing host configuration.
    if model.automatic
        && step == 0
        && tool.is_none()
        && callable.iter().any(|(_, name)| name == "execute")
    {
        tool = Some((None, "execute".into()));
        args = json!({"code":"return await tools.recollect.workspace_list({});"});
    }
    let reply = "Verified fresh Recollect context.";
    if path.ends_with("responses") {
        let item = if let Some((namespace, name)) = code_mode.filter(|_| step < steps) {
            let script = format!(
                "const available = ALL_TOOLS.filter(t => t.name.includes('recollect'));\n\
                 text({{available_tool_names: ALL_TOOLS.map(t => t.name)}});\n\
                 const chosen = available.find(t => t.name.replaceAll('.', '_').endsWith('{suffix}'));\n\
                 if (!chosen) throw new Error('owned_fixture_tool_missing');\n\
                 const result = await tools[chosen.name]({args});\n\
                 text({{tool_names: available.map(t => t.name), result}});"
            );
            json!({"type":"custom_tool_call","namespace":namespace,"name":name,"input":script,
                "id":format!("ct_{step}"),"call_id":format!("call_{step}"),"status":"completed"})
        } else {
            match tool {
                Some((namespace, name)) => {
                    json!({"type":"function_call","namespace":namespace,"id":format!("fc_{step}"),"call_id":format!("call_{step}"),"name":name,"arguments":args.to_string(),"status":"completed"})
                }
                None => {
                    json!({"type":"message","id":format!("msg_{step}"),"role":"assistant","status":"completed","content":[{"type":"output_text","text":reply,"annotations":[]}]})
                }
            }
        };
        return stream(vec![
            (
                "response.created",
                json!({"type":"response.created","response":{"id":format!("resp_{step}"),"status":"in_progress","output":[]}}),
            ),
            (
                "response.output_item.added",
                json!({"type":"response.output_item.added","output_index":0,"item":item}),
            ),
            (
                "response.output_item.done",
                json!({"type":"response.output_item.done","output_index":0,"item":item}),
            ),
            (
                "response.completed",
                json!({"type":"response.completed","response":{"id":format!("resp_{step}"),"status":"completed","output":[item],"usage":{"input_tokens":42,"output_tokens":20,"total_tokens":62}}}),
            ),
        ]);
    }
    let (block, start, delta, stop) = match tool {
        Some((_, name)) => (
            json!({"type":"tool_use","id":format!("tool_{step}"),"name":name,"input":args}),
            json!({"type":"tool_use","id":format!("tool_{step}"),"name":name,"input":{}}),
            json!({"type":"input_json_delta","partial_json":args.to_string()}),
            "tool_use",
        ),
        None => (
            json!({"type":"text","text":reply}),
            json!({"type":"text","text":""}),
            json!({"type":"text_delta","text":reply}),
            "end_turn",
        ),
    };
    let mut message = json!({"id":format!("msg_{step}"),"type":"message","role":"assistant","model":"claude-sonnet-4-6",
        "content":[block],"stop_reason":stop,"stop_sequence":null,"usage":{"input_tokens":42,"output_tokens":20}});
    if body["stream"] != true {
        return axum::Json(message).into_response();
    }
    message["content"] = json!([]);
    message["stop_reason"] = Value::Null;
    stream(vec![
        (
            "message_start",
            json!({"type":"message_start","message":message}),
        ),
        (
            "content_block_start",
            json!({"type":"content_block_start","index":0,"content_block":start}),
        ),
        (
            "content_block_delta",
            json!({"type":"content_block_delta","index":0,"delta":delta}),
        ),
        (
            "content_block_stop",
            json!({"type":"content_block_stop","index":0}),
        ),
        (
            "message_delta",
            json!({"type":"message_delta","delta":{"stop_reason":stop,"stop_sequence":null},"usage":{"output_tokens":20}}),
        ),
        ("message_stop", json!({"type":"message_stop"})),
    ])
}
