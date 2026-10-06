//! Shared capture wire types and deterministic sanitization. Raw host payloads
//! stay in memory; only the normalized result may enter the native inbox.
use crate::OperationBinding;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;
use utoipa::ToSchema;
use uuid::Uuid;

pub const CAPTURE_STDIN_BYTES: usize = 256 * 1024;
pub const CAPTURE_MAX_BYTES: usize = 64 * 1024;
pub const CAPTURE_KINDS: &[&str] = &["prompt", "reply", "tool_result", "lifecycle"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CapturePolicy {
    pub enabled: bool,
    #[serde(default)]
    pub managed_tools: bool,
    pub kinds: Vec<String>,
    pub excluded_tools: Vec<String>,
    pub excluded_content: Vec<String>,
    pub max_event_bytes: i32,
}
impl Default for CapturePolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            managed_tools: false,
            kinds: CAPTURE_KINDS.iter().map(|s| (*s).into()).collect(),
            excluded_tools: vec![],
            excluded_content: vec![],
            max_event_bytes: 16 * 1024,
        }
    }
}
pub fn validate_capture_policy(p: &CapturePolicy) -> Result<(), &'static str> {
    if !(1024..=CAPTURE_MAX_BYTES as i32).contains(&p.max_event_bytes)
        || p.kinds.is_empty()
        || p.kinds.len() > CAPTURE_KINDS.len()
        || p.kinds.iter().any(|v| !CAPTURE_KINDS.contains(&v.as_str()))
        || p.kinds
            .iter()
            .enumerate()
            .any(|(i, k)| p.kinds[..i].contains(k))
        || [&p.excluded_tools, &p.excluded_content].iter().any(|list| {
            list.len() > 20
                || list.iter().any(|s| {
                    s.trim().is_empty() || s.len() > 200 || s.chars().any(char::is_control)
                })
        })
    {
        return Err("invalid_capture_policy");
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CaptureSettings {
    pub brain_id: Uuid,
    pub change_id: Uuid,
    pub policy: CapturePolicy,
}
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CapturePolicyUpdate {
    pub base_change: Uuid,
    pub policy: CapturePolicy,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureBindingInput {
    pub id: Uuid,
    pub operation_id: Uuid,
    pub host: String,
    pub host_version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CaptureBinding {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub device_id: Uuid,
    pub operation: OperationBinding,
    pub host: String,
    pub host_version: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CapturedHook {
    pub host_event: String,
    pub host_session_id: String,
    pub turn_id: Option<String>,
    pub agent_id: Option<String>,
    pub tool_use_id: Option<String>,
    pub tool_name: Option<String>,
    pub kind: String,
    pub outcome: String,
    pub content: Option<String>,
    pub coverage: Vec<String>,
    pub captured_at: DateTime<Utc>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureEventInput {
    pub id: Uuid,
    pub binding_id: Uuid,
    pub event: CapturedHook,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CaptureReceipt {
    pub event_id: Uuid,
    pub binding_id: Uuid,
    pub state: String,
    pub source_id: Option<Uuid>,
    pub source_version_id: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
    pub received_at: DateTime<Utc>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureFence {
    pub event_id: Uuid,
    pub binding_id: Uuid,
    pub native_key: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CaptureEventView {
    pub device_id: Option<Uuid>,
    pub agent_name: Option<String>,
    pub user_name: String,
    pub receipt: CaptureReceipt,
    pub host: String,
    pub host_version: String,
    pub operation_id: Option<Uuid>,
    pub managed_call_id: Option<Uuid>,
    pub selection: crate::ScopeSelection,
    pub event: Option<CapturedHook>,
    pub source_available: bool,
    pub processing: Option<String>,
    pub learning: Option<crate::PipelineLearning>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CaptureEventPage {
    pub items: Vec<CaptureEventView>,
    pub total: i64,
    pub offset: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureDeviceReport {
    pub pending: i64,
    pub denied: i64,
    pub device_gap_count: i64,
    pub issue: Option<String>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CaptureDeviceView {
    pub device_id: Uuid,
    pub bindings: i64,
    pub reported_at: Option<DateTime<Utc>>,
    pub report: Option<CaptureDeviceReport>,
    pub last_publication: Option<DateTime<Utc>>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CaptureDevicePage {
    pub items: Vec<CaptureDeviceView>,
    pub total: i64,
    pub offset: i64,
}
pub fn capture_issue(code: &str) -> bool {
    matches!(
        code,
        "server_unavailable"
            | "transport_unavailable"
            | "capture_denied"
            | "capture_sync_unavailable"
            | "capture_identity_conflict"
            | "capture_binding_missing"
            | "capture_delivery_failed"
            | "capture_response_unreadable"
    )
}

/// The key contains only bounded host identities, never tool arguments or text.
pub fn capture_native_key(host: &str, event: &CapturedHook) -> Option<String> {
    if event.turn_id.is_none()
        || (matches!(
            event.host_event.as_str(),
            "PreToolUse" | "PostToolUse" | "PostToolUseFailure"
        ) && event.tool_use_id.is_none())
    {
        return None;
    }
    Some(
        serde_json::to_string(&(
            host,
            &event.host_session_id,
            &event.turn_id,
            event.agent_id.as_deref().unwrap_or(""),
            &event.tool_use_id,
            &event.host_event,
        ))
        .expect("capture identity serialization"),
    )
}

/// Revalidate the normalized wire event at admission. No host payload is trusted
/// to have passed the local sanitizer, and errors never include its contents.
pub fn sanitize_captured_event(
    event: &mut CapturedHook,
    policy: &CapturePolicy,
    secrets: &[String],
) -> Result<(), &'static str> {
    validate_capture_policy(policy)?;
    let managed = matches!(
        event.host_event.as_str(),
        "ManagedTool" | "ManagedReceipt" | "ManagedResolution"
    );
    let kind = match event.host_event.as_str() {
        "UserPromptSubmit" => "prompt",
        "Stop" | "SubagentStop" => "reply",
        "PostToolUse" | "PostToolUseFailure" => "tool_result",
        "ManagedTool" | "ManagedReceipt" | "ManagedResolution" => "tool_result",
        "SessionStart" | "SessionEnd" | "PreToolUse" | "SubagentStart" | "PreCompact"
        | "PostCompact" | "Interrupt" | "StopFailure" | "StepStart" => "lifecycle",
        _ => return Err("unsupported_capture_event"),
    };
    const COVERAGE: &[&str] = &[
        "partial_host_coverage",
        "excluded_event_kind",
        "excluded_tool_content",
        "excluded_content",
        "redacted",
        "truncated",
        "missing_content",
        "missing_turn_identity",
        "ambiguous_attribution",
        "delayed_tool_completion",
        "host_deduplication_unavailable",
        "managed_execution_receipt",
        "outcome_unknown",
        "receipt_payload_unavailable",
        "publication_capacity",
    ];
    if event.kind != kind
        || !(if managed {
            matches!(
                event.outcome.as_str(),
                "succeeded" | "tool_error" | "failed" | "cancelled" | "unknown" | "not_executed"
            )
        } else {
            matches!(event.outcome.as_str(), "reported" | "failed")
        })
        || event.coverage.len() > COVERAGE.len()
        || event
            .coverage
            .iter()
            .any(|flag| !COVERAGE.contains(&flag.as_str()))
        || event
            .content
            .as_ref()
            .is_some_and(|s| s.len() > CAPTURE_MAX_BYTES || s.contains('\0'))
        || std::iter::once(Some(&event.host_session_id))
            .chain([
                event.turn_id.as_ref(),
                event.agent_id.as_ref(),
                event.tool_use_id.as_ref(),
                event.tool_name.as_ref(),
            ])
            .flatten()
            .any(|s| !capture_identity(s) || sanitize_capture_text(s, secrets) != *s)
    {
        return Err("invalid_capture_event");
    }
    event.coverage.push(
        if managed {
            "managed_execution_receipt"
        } else {
            "partial_host_coverage"
        }
        .into(),
    );
    if !policy.kinds.iter().any(|k| k == kind) {
        event.coverage.push("excluded_event_kind".into());
        event.content = None;
    } else if kind == "lifecycle" {
        event.content = None;
    } else if kind == "tool_result"
        && (event
            .tool_name
            .as_ref()
            .is_some_and(|t| policy.excluded_tools.contains(t) || own_mcp_tool(t))
            || event
                .content
                .as_ref()
                .is_some_and(|s| excluded_tool_envelope(&Value::String(s.clone()), 0)))
    {
        event.coverage.push("excluded_tool_content".into());
        event.content = None;
    } else if policy
        .excluded_content
        .iter()
        .any(|term| event.content.as_ref().is_some_and(|s| s.contains(term)))
    {
        event.coverage.push("excluded_content".into());
        event.content = None;
    }
    if event.turn_id.is_none() && kind != "lifecycle" && !managed {
        event.coverage.push("missing_turn_identity".into());
        event.coverage.push("ambiguous_attribution".into());
    }
    if event.host_event == "SubagentStop" && event.agent_id.is_none() {
        event.coverage.push("ambiguous_attribution".into());
    }
    if event.coverage.iter().any(|flag| {
        matches!(
            flag.as_str(),
            "ambiguous_attribution"
                | "excluded_event_kind"
                | "excluded_tool_content"
                | "excluded_content"
        )
    }) {
        event.content = None;
    }
    if let Some(text) = event.content.take().filter(|s| !s.is_empty()) {
        let mut safe = if kind == "tool_result" {
            match serde_json::from_str::<Value>(&text) {
                Ok(value) => serde_json::to_string_pretty(&sanitize_value(&value, secrets, 0))
                    .map_err(|_| "invalid_capture_event")?,
                Err(_) => sanitize_capture_text(&text, secrets), // Host text may already be truncated.
            }
        } else {
            sanitize_capture_text(&text, secrets)
        };
        if safe != text {
            event.coverage.push("redacted".into());
        }
        if safe.len() > policy.max_event_bytes as usize {
            let mut end = policy.max_event_bytes as usize;
            while !safe.is_char_boundary(end) {
                end -= 1;
            }
            safe.truncate(end);
            event.coverage.push("truncated".into());
        }
        event.content = Some(safe);
    }
    event.coverage.sort();
    event.coverage.dedup();
    Ok(())
}

/// Managed receipts are already bounded and credential-redacted by the runtime.
/// Inspect the complete envelope before truncation, then reuse normal capture
/// normalization under both the admitted and current policies.
pub fn prepare_managed_capture(
    mut event: CapturedHook,
    envelope: &Value,
    admitted: &CapturePolicy,
    current: &CapturePolicy,
    secrets: &[String],
) -> Result<CapturedHook, &'static str> {
    if !matches!(
        event.host_event.as_str(),
        "ManagedTool" | "ManagedReceipt" | "ManagedResolution"
    ) {
        return Err("invalid_managed_observation");
    }
    validate_capture_policy(admitted)?;
    validate_capture_policy(current)?;
    let original = serde_json::to_string(envelope).map_err(|_| "invalid_managed_observation")?;
    if original.len() > 1024 * 1024 {
        return Err("managed_observation_too_large");
    }
    event.content = None;
    if [admitted, current]
        .iter()
        .any(|p| !p.enabled || !p.managed_tools || !p.kinds.iter().any(|k| k == "tool_result"))
    {
        event.coverage.push("excluded_event_kind".into());
    } else if excluded_tool_envelope(envelope, 0) {
        event.coverage.push("excluded_tool_content".into());
    } else if [admitted, current]
        .iter()
        .any(|p| p.excluded_content.iter().any(|v| original.contains(v)))
    {
        event.coverage.push("excluded_content".into());
    } else {
        let safe = sanitize_capture_value(envelope, secrets);
        if safe != *envelope {
            event.coverage.push("redacted".into());
        }
        let mut text =
            serde_json::to_string_pretty(&safe).map_err(|_| "invalid_managed_observation")?;
        if text.len() > CAPTURE_MAX_BYTES {
            let mut end = CAPTURE_MAX_BYTES;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
            event.coverage.push("truncated".into());
        }
        event.content = Some(text);
    }
    sanitize_captured_event(&mut event, admitted, secrets)?;
    sanitize_captured_event(&mut event, current, secrets)?;
    Ok(event)
}

fn patterns() -> &'static Vec<Regex> {
    static RULES: OnceLock<Vec<Regex>> = OnceLock::new();
    RULES.get_or_init(|| {
        [
            r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?(?:-----END [A-Z ]*PRIVATE KEY-----|$)",
            r"(?i)\b(?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?|redis|amqps?)://[^\s\x22'<>]+",
            r"(?i)\b(?:bearer|basic)\s+[a-z0-9._~+/=-]+",
            r#"(?i)["']?\b(?:[\w.-]*[_-])?(?:password|passwd|secret|token|api[_-]?key|x-api-key)["']?\s*[:=]\s*(?:"[^"]*"|'[^']*'|[^\s,;]+)"#,
            r"\b(?:sk-(?:proj-|svcacct-|ant-)?|gh[pousr]_|github_pat_|glpat-|xox[baprs]-|whsec_)[A-Za-z0-9_-]{12,}",
            r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b",
            r"https?://[^\s/@:]+:[^\s/@]+@[^\s\x22'<>]+",
        ]
        .iter()
        .map(|p| Regex::new(p).expect("constant capture redaction pattern"))
        .collect()
    })
}
pub fn sanitize_capture_text(text: &str, secrets: &[String]) -> String {
    let mut safe = text.to_owned();
    for secret in secrets.iter().filter(|s| s.len() >= 4) {
        safe = safe.replace(secret, "[redacted]");
    }
    for pattern in patterns() {
        safe = pattern.replace_all(&safe, "[redacted]").into_owned();
    }
    safe
}
fn secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace('-', "_");
    [
        "authorization",
        "cookie",
        "set_cookie",
        "x_api_key",
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
    ]
    .iter()
    .any(|suffix| key == *suffix || key.ends_with(&format!("_{suffix}")))
}
/// Shared structured redaction for capture and managed connector responses.
pub fn sanitize_capture_value(value: &Value, secrets: &[String]) -> Value {
    sanitize_value(value, secrets, 0)
}

fn sanitize_value(v: &Value, secrets: &[String], depth: usize) -> Value {
    if depth > 24 {
        return Value::String("[omitted:depth]".into());
    }
    match v {
        Value::String(s) => Value::String(sanitize_capture_text(s, secrets)),
        Value::Array(a) => Value::Array(
            a.iter()
                .map(|v| sanitize_value(v, secrets, depth + 1))
                .collect(),
        ),
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, v)| {
                    (
                        sanitize_capture_text(k, secrets),
                        if secret_key(k) {
                            Value::String("[redacted]".into())
                        } else {
                            sanitize_value(v, secrets, depth + 1)
                        },
                    )
                })
                .collect(),
        ),
        _ => {
            let raw = v.to_string();
            if secrets.iter().any(|s| s.len() >= 4 && raw.contains(s)) {
                Value::String("[redacted]".into())
            } else {
                v.clone()
            }
        }
    }
}
pub fn capture_sensitive_path(value: &str) -> bool {
    let normalized = value.replace('\\', "/").to_ascii_lowercase();
    normalized
        .split(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | ';' | '(' | ')' | '='))
        .any(|word| {
            let name = word.rsplit('/').next().unwrap_or(word);
            name == ".env"
                || name.starts_with(".env.")
                || [".pem", ".key", ".p12", ".pfx"]
                    .iter()
                    .any(|s| name.ends_with(s))
                || ["id_rsa", "id_ed25519", "secrets.", "credentials."]
                    .iter()
                    .any(|s| name.starts_with(s))
                || matches!(name, ".npmrc" | ".netrc")
                || word.contains("/.ssh/")
                || word.ends_with("/.aws/credentials")
        })
}
pub fn capture_own_transport(text: &str) -> bool {
    // First-party recall is derived context, not fresh independent evidence.
    // Generated commands quote executable paths; preserve whitespace boundaries.
    static OWN: OnceLock<Regex> = OnceLock::new();
    OWN.get_or_init(|| {
        Regex::new(r"(?:^|[^a-zA-Z0-9_.-])(?:recollect-plugin\b|tools\s*(?:\.\s*recollect\b|\[\s*recollect\s*\])|recollect-agent\s+(?:capture\b|scope\s+recall\b|mcp\b|mcp-serve\b|mcp-config\b))")
            .expect("constant first-party capture exclusion")
    })
    .is_match(&text.replace(['\'', '"', '\\'], ""))
}
fn own_mcp_tool(name: &str) -> bool {
    // Generated host settings reserve this first-party server name. This only
    // excludes derived output; it never grants trust based on a caller's name.
    name.starts_with("mcp__recollect__")
        || name.starts_with("recollect_")
        || name.starts_with("mcp__plugin_recollect-memory_recollect__")
        || name.starts_with("mcp__plugin_recollect_memory_recollect__")
}
fn excluded_tool_input(v: &Value) -> bool {
    excluded_tool_value(v, 0)
}
fn excluded_tool_envelope(v: &Value, depth: usize) -> bool {
    if depth > 8 {
        return true;
    }
    match v {
        // A documentation read can mention recall in its output without having
        // invoked recall. Whole-payload secret redaction still follows below.
        Value::Object(m) if m.contains_key("input") => excluded_tool_input(&m["input"]),
        Value::String(s) => match serde_json::from_str::<Value>(s) {
            Ok(decoded) => excluded_tool_envelope(&decoded, depth + 1),
            Err(_) => excluded_tool_input(v),
        },
        _ => excluded_tool_input(v),
    }
}
fn excluded_tool_value(v: &Value, depth: usize) -> bool {
    if depth > 32 {
        return true;
    }
    match v {
        Value::String(s) => {
            capture_sensitive_path(s)
                || capture_own_transport(s)
                || serde_json::from_str::<Value>(s)
                    .is_ok_and(|decoded| excluded_tool_value(&decoded, depth + 1))
        }
        Value::Array(a) => a.iter().any(|v| excluded_tool_value(v, depth + 1)),
        Value::Object(m) => m.values().any(|v| excluded_tool_value(v, depth + 1)),
        _ => false,
    }
}
pub fn capture_identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 160
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
}
fn identity(v: &Value, key: &str, secrets: &[String]) -> Result<Option<String>, &'static str> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s))
            if capture_identity(s) && sanitize_capture_text(s, secrets) == *s =>
        {
            Ok(Some(s.clone()))
        }
        _ => Err("invalid_capture_identity"),
    }
}

/// Does not infer a scope or agent. The inbox must associate known turns/tools
/// with immutable bindings before persisting any returned content.
pub fn normalize_capture_hook(
    host: &str,
    raw: &[u8],
    policy: &CapturePolicy,
    secrets: &[String],
    at: DateTime<Utc>,
) -> Result<CapturedHook, &'static str> {
    validate_capture_policy(policy)?;
    if !policy.enabled {
        return Err("capture_disabled");
    }
    if !matches!(host, "codex" | "claude_code" | "opencode") {
        return Err("unsupported_capture_host");
    }
    if raw.len() > CAPTURE_STDIN_BYTES {
        return Err("capture_input_too_large");
    }
    let v: Value = serde_json::from_slice(raw).map_err(|_| "malformed_capture_event")?;
    let name = v
        .get("hook_event_name")
        .and_then(Value::as_str)
        .ok_or("missing_capture_event")?;
    let mut tool_redacted = false;
    let mut raw_content_excluded = false;
    let (kind, mut content) = match name {
        "UserPromptSubmit" => (
            "prompt",
            v.get("prompt").and_then(Value::as_str).map(str::to_owned),
        ),
        "Stop" | "SubagentStop" => (
            "reply",
            v.get("last_assistant_message")
                .and_then(Value::as_str)
                .map(str::to_owned),
        ),
        "PostToolUse" | "PostToolUseFailure" => {
            let body = serde_json::json!({"input":v.get("tool_input"),"output":v.get("tool_response").or_else(||v.get("error"))});
            let raw_text =
                serde_json::to_string_pretty(&body).map_err(|_| "malformed_capture_event")?;
            raw_content_excluded = policy
                .excluded_content
                .iter()
                .any(|term| raw_text.contains(term));
            let safe = sanitize_value(&body, secrets, 0);
            tool_redacted = safe != body;
            (
                "tool_result",
                Some(serde_json::to_string_pretty(&safe).map_err(|_| "malformed_capture_event")?),
            )
        }
        "SessionStart" | "SessionEnd" | "PreToolUse" | "SubagentStart" | "PreCompact"
        | "PostCompact" | "Interrupt" | "StopFailure" | "StepStart" => ("lifecycle", None),
        _ => return Err("unsupported_capture_event"),
    };
    let mut event = CapturedHook {
        host_event: name.into(),
        host_session_id: identity(&v, "session_id", secrets)?.ok_or("missing_session_identity")?,
        turn_id: identity(
            &v,
            if host == "codex" {
                "turn_id"
            } else {
                "prompt_id"
            },
            secrets,
        )?,
        agent_id: identity(&v, "agent_id", secrets)?,
        tool_use_id: identity(&v, "tool_use_id", secrets)?,
        tool_name: identity(&v, "tool_name", secrets)?,
        kind: kind.into(),
        outcome: if matches!(name, "PostToolUseFailure" | "StopFailure")
            || v.get("error").is_some_and(|e| !e.is_null() && e != "")
            || v.get("tool_response").is_some_and(|r| {
                r.get("isError").and_then(Value::as_bool) == Some(true)
                    || r.get("is_error").and_then(Value::as_bool) == Some(true)
                    || r.get("success").and_then(Value::as_bool) == Some(false)
                    || r.get("exit_code")
                        .and_then(Value::as_i64)
                        .is_some_and(|c| c != 0)
            }) {
            "failed"
        } else {
            "reported"
        }
        .into(),
        content: None,
        coverage: vec!["partial_host_coverage".into()],
        captured_at: at,
    };
    if !policy.kinds.iter().any(|k| k == kind) {
        event.coverage.push("excluded_event_kind".into());
        content = None;
    } else if kind == "tool_result"
        && (event
            .tool_name
            .as_ref()
            .is_some_and(|t| policy.excluded_tools.contains(t) || own_mcp_tool(t))
            || v.get("tool_input").is_some_and(excluded_tool_input))
    {
        event.coverage.push("excluded_tool_content".into());
        content = None;
    } else if raw_content_excluded
        || policy
            .excluded_content
            .iter()
            .any(|term| content.as_ref().is_some_and(|s| s.contains(term)))
    {
        event.coverage.push("excluded_content".into());
        content = None;
    }
    if let Some(text) = content.filter(|s| !s.is_empty()) {
        let mut safe = sanitize_capture_text(&text, secrets);
        if safe != text || tool_redacted {
            event.coverage.push("redacted".into());
        }
        let limit = policy.max_event_bytes as usize;
        if safe.len() > limit {
            let mut end = limit;
            while !safe.is_char_boundary(end) {
                end -= 1;
            }
            safe.truncate(end);
            event.coverage.push("truncated".into());
        }
        event.content = Some(safe);
    } else if kind != "lifecycle" && event.coverage.len() == 1 {
        event.coverage.push("missing_content".into());
    }
    if kind != "lifecycle" && event.turn_id.is_none() {
        event.coverage.push("missing_turn_identity".into());
    }
    if name == "SubagentStop" && event.agent_id.is_none() {
        event.coverage.push("ambiguous_attribution".into());
        event.content = None;
    }
    Ok(event)
}

#[cfg(test)]
mod tests {
    #[test]
    fn structured_exact_redaction_covers_scalar_values_and_keeps_unrelated_numbers() {
        let input =
            serde_json::json!({"echo":1234,"nested":[912345,8080,true,null],"1234":"visible"});
        let safe = super::sanitize_capture_value(&input, &["1234".into()]);
        assert_eq!(
            safe,
            serde_json::json!({"echo":"[redacted]","nested":["[redacted]",8080,true,null],"[redacted]":"visible"})
        );
        assert_eq!(
            super::sanitize_capture_value(
                &serde_json::json!([true, false, null]),
                &["true".into(), "null".into()]
            ),
            serde_json::json!(["[redacted]", false, "[redacted]"])
        );
    }
    use super::*;
    use serde_json::json;
    fn policy() -> CapturePolicy {
        CapturePolicy {
            enabled: true,
            ..Default::default()
        }
    }
    #[test]
    fn managed_envelopes_check_whole_input_then_redact_and_apply_both_limits() {
        let mut admitted = policy();
        admitted.managed_tools = true;
        admitted.max_event_bytes = 1024;
        let mut current = admitted.clone();
        current.excluded_content = vec!["outside-the-retained-prefix".into()];
        let event = CapturedHook {
            host_event: "ManagedTool".into(),
            host_session_id: Uuid::new_v4().to_string(),
            turn_id: None,
            agent_id: None,
            tool_use_id: Some(Uuid::new_v4().to_string()),
            tool_name: Some("inspect".into()),
            kind: "tool_result".into(),
            outcome: "succeeded".into(),
            content: None,
            coverage: vec![],
            captured_at: Utc::now(),
        };
        let excluded = prepare_managed_capture(
            event.clone(),
            &json!({"result":format!("{}outside-the-retained-prefix","é".repeat(10000))}),
            &admitted,
            &current,
            &[],
        )
        .unwrap();
        assert!(excluded.content.is_none());
        assert!(excluded.coverage.contains(&"excluded_content".into()));
        let secrets = vec!["configured-opaque-fixture-value".into()];
        let permitted=prepare_managed_capture(event.clone(),&json!({"result":{"value":secrets[0],"password":"synthetic-secret"},"text":"é".repeat(10000)}),&admitted,&current,&secrets).unwrap();
        let content = permitted.content.unwrap();
        assert!(
            content.len() <= 1024
                && !content.contains(&secrets[0])
                && !content.contains("synthetic-secret")
        );
        assert!(permitted.coverage.contains(&"redacted".into()));
        assert!(permitted.coverage.contains(&"truncated".into()));
        current.managed_tools = false;
        assert!(
            prepare_managed_capture(
                event,
                &json!({"result":"previously permitted"}),
                &admitted,
                &current,
                &[]
            )
            .unwrap()
            .content
            .is_none()
        );
    }
    #[test]
    fn own_recall_is_excluded_at_both_capture_boundaries_without_losing_tool_evidence() {
        for command in [
            "recollect-agent capture drain",
            "recollect-agent scope recall brain operation query",
            "'/workspace/with spaces/recollect-agent' scope recall brain operation query",
            "\"/workspace/recollect-agent\"\t scope  recall brain operation query",
            "recollect-agent mcp status brain call",
            "recollect-agent mcp-serve --brain uuid",
            "return await tools.recollect.workspace_list({});",
            "return await tools['recollect']['memory_recall']({});",
        ] {
            let raw = json!({"hook_event_name":"PostToolUse","session_id":"s","turn_id":"t",
                "tool_use_id":"tool1","tool_name":"Bash","tool_input":{"command":command},
                "tool_response":{"stdout":"RECALLED_ORIGINAL_SENTINEL"}});
            let event = normalize_capture_hook(
                "codex",
                &serde_json::to_vec(&raw).unwrap(),
                &policy(),
                &[],
                Utc::now(),
            )
            .unwrap();
            assert!(event.content.is_none(), "{command}");
            assert!(event.coverage.contains(&"excluded_tool_content".into()));
            // A direct caller can omit the local exclusion flag. Revalidate its
            // independently encoded wire content at the server boundary.
            let mut direct = event;
            direct.coverage.clear();
            direct.content = Some(
                json!({"input":{"command":command},"output":"RECALLED_ORIGINAL_SENTINEL"})
                    .to_string()
                    .replace("recollect-agent", "\\u0072ecollect-agent"),
            );
            sanitize_captured_event(&mut direct, &policy(), &[]).unwrap();
            assert!(direct.content.is_none(), "encoded {command}");
            assert!(direct.coverage.contains(&"excluded_tool_content".into()));
        }
        for host in ["codex", "claude_code", "opencode"] {
            for tool in [
                "mcp__recollect__memory.recall",
                "mcp__recollect__workspace_set_scope",
                "mcp__recollect__mcp_status",
                "recollect_memory_recall",
                "mcp__plugin_recollect-memory_recollect__memory_recall",
            ] {
                let raw = json!({"hook_event_name":"PostToolUse","session_id":"s","turn_id":"t","prompt_id":"t",
                    "tool_use_id":"tool1","tool_name":tool,"tool_input":{},"tool_response":{"text":"DERIVED_MCP_CONTEXT"}});
                let mut event = normalize_capture_hook(
                    host,
                    &serde_json::to_vec(&raw).unwrap(),
                    &policy(),
                    &[],
                    Utc::now(),
                )
                .unwrap();
                assert!(event.content.is_none());
                event.coverage.clear();
                event.content = Some("DERIVED_MCP_CONTEXT".into());
                sanitize_captured_event(&mut event, &policy(), &[]).unwrap();
                assert!(event.content.is_none());
                assert!(event.coverage.contains(&"excluded_tool_content".into()));
            }
        }
        let raw = json!({"hook_event_name":"PostToolUse","session_id":"s","turn_id":"t",
            "tool_name":"Bash","tool_input":{"command":"inspect synthetic service"},
            "tool_response":{"stdout":"INDEPENDENT_TOOL_OBSERVATION\nDocumentation: recollect-agent scope recall BRAIN OPERATION QUERY\nAPI_KEY=fixture-doc-secret"}});
        let mut event = normalize_capture_hook(
            "codex",
            &serde_json::to_vec(&raw).unwrap(),
            &policy(),
            &[],
            Utc::now(),
        )
        .unwrap();
        sanitize_captured_event(&mut event, &policy(), &[]).unwrap();
        let content = event.content.unwrap();
        assert!(content.contains("INDEPENDENT_TOOL_OBSERVATION"));
        assert!(content.contains("recollect-agent scope recall"));
        assert!(!content.contains("fixture-doc-secret"));
    }
    #[test]
    fn host_capture_sanitizes_before_truncating_and_excludes_sensitive_tools() {
        let secrets = vec!["fixture-configured-credential".into()];
        let input = json!({"hook_event_name":"PostToolUse","session_id":"s1","turn_id":"t1","tool_use_id":"tool1","tool_name":"Bash",
            "tool_input":{"command":"inspect service","Authorization":"unrecognized-opaque-credential"},
            "tool_response":{"stdout":"port=8080\nAPI_KEY=fixture-assigned-credential\nfixture-configured-credential\n-----BEGIN PRIVATE KEY-----\nprivate-material\n-----END PRIVATE KEY-----\nokay"}});
        let event = normalize_capture_hook(
            "codex",
            &serde_json::to_vec(&input).unwrap(),
            &policy(),
            &secrets,
            Utc::now(),
        )
        .unwrap();
        let stored = serde_json::to_string(&event).unwrap();
        for excluded in [
            "fixture-configured-credential",
            "fixture-assigned-credential",
            "unrecognized-opaque-credential",
            "private-material",
        ] {
            assert!(!stored.contains(excluded));
        }
        assert!(stored.contains("port=8080"));
        assert!(stored.contains("okay"));
        let mut sensitive = input;
        sensitive["tool_input"] = json!({"file_path":"/workspace/.env.local"});
        let event = normalize_capture_hook(
            "codex",
            &serde_json::to_vec(&sensitive).unwrap(),
            &policy(),
            &secrets,
            Utc::now(),
        )
        .unwrap();
        assert!(event.content.is_none());
        assert!(event.coverage.contains(&"excluded_tool_content".into()));
        let mut limited = policy();
        limited.max_event_bytes = 1024;
        let payload = json!({"hook_event_name":"UserPromptSubmit","session_id":"s1","prompt_id":"p1","prompt":format!("{}\n-----BEGIN PRIVATE KEY-----\n{}", "é".repeat(510),"not-retained".repeat(100))});
        let event = normalize_capture_hook(
            "claude_code",
            &serde_json::to_vec(&payload).unwrap(),
            &limited,
            &[],
            Utc::now(),
        )
        .unwrap();
        assert!(event.content.as_ref().unwrap().len() <= 1024);
        assert!(!event.content.unwrap().contains("not-retained"));
        assert_eq!(event.turn_id.as_deref(), Some("p1"));
    }
    #[test]
    fn host_capture_reports_missing_identity_and_keeps_raw_payload_out_of_errors() {
        let raw = json!({"hook_event_name":"SubagentStop","session_id":"parent","turn_id":"unknown-child","last_assistant_message":"never attribute this to the parent"});
        let event = normalize_capture_hook(
            "codex",
            &serde_json::to_vec(&raw).unwrap(),
            &policy(),
            &[],
            Utc::now(),
        )
        .unwrap();
        assert!(event.content.is_none());
        assert!(event.coverage.contains(&"ambiguous_attribution".into()));
        assert_eq!(
            normalize_capture_hook(
                "codex",
                b"raw-sensitive-invalid-json",
                &policy(),
                &[],
                Utc::now()
            ),
            Err("malformed_capture_event")
        );
        assert_eq!(
            normalize_capture_hook(
                "codex",
                &vec![b'x'; CAPTURE_STDIN_BYTES + 1],
                &policy(),
                &[],
                Utc::now()
            ),
            Err("capture_input_too_large")
        );
        assert_eq!(
            normalize_capture_hook("codex", b"{}", &CapturePolicy::default(), &[], Utc::now()),
            Err("capture_disabled")
        );
        let mut p = policy();
        p.excluded_content = vec!["internal-fixture".into()];
        let raw = json!({"hook_event_name":"UserPromptSubmit","session_id":"s1","turn_id":"t1","prompt":"internal-fixture message"});
        let event = normalize_capture_hook(
            "codex",
            &serde_json::to_vec(&raw).unwrap(),
            &p,
            &[],
            Utc::now(),
        )
        .unwrap();
        assert!(event.content.is_none());
        assert!(event.coverage.contains(&"excluded_content".into()));
    }
}
