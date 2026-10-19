//! Local normalization and server admission share one policy application. These
//! cases pin the exact output of both entry points so the pipelines cannot drift.
use super::*;
use serde_json::json;

const SECRET: &str = "synthetic-configured-credential";

fn policy(edit: impl FnOnce(&mut CapturePolicy)) -> CapturePolicy {
    let mut p = CapturePolicy {
        enabled: true,
        ..Default::default()
    };
    edit(&mut p);
    p
}
fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-10-19T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}
/// One line per event; long content is abbreviated so the table stays readable.
fn shown(event: &CapturedHook) -> String {
    let content = event.content.as_deref().map(|text| {
        if text.len() > 120 {
            format!("{}…[{} bytes]", &text[..40], text.len())
        } else {
            text.to_owned()
        }
    });
    format!(
        "{}/{}/{} turn={:?} agent={:?} content={:?} coverage={:?}",
        event.host_event,
        event.kind,
        event.outcome,
        event.turn_id,
        event.agent_id,
        content,
        event.coverage
    )
}
fn local(raw: Value, p: &CapturePolicy) -> String {
    let secrets = [SECRET.to_owned()];
    match normalize_capture_hook(
        "codex",
        &serde_json::to_vec(&raw).unwrap(),
        p,
        &secrets,
        at(),
    ) {
        Ok(event) => shown(&event),
        Err(code) => format!("error:{code}"),
    }
}
fn hook(kind: &str, event: &str, content: Option<&str>) -> CapturedHook {
    CapturedHook {
        host_event: event.into(),
        host_session_id: "session-1".into(),
        turn_id: Some("turn-1".into()),
        agent_id: None,
        tool_use_id: (kind == "tool_result").then(|| "tool-1".into()),
        tool_name: (kind == "tool_result").then(|| "Bash".into()),
        kind: kind.into(),
        outcome: "reported".into(),
        content: content.map(str::to_owned),
        coverage: vec!["partial_host_coverage".into()],
        captured_at: at(),
    }
}
fn admit(mut event: CapturedHook, p: &CapturePolicy) -> String {
    let secrets = [SECRET.to_owned()];
    match sanitize_captured_event(&mut event, p, &secrets) {
        Ok(()) => shown(&event),
        Err(code) => format!("error:{code}"),
    }
}

fn local_cases() -> Vec<(&'static str, Value, CapturePolicy)> {
    let base =
        |name: &str| json!({"hook_event_name":name,"session_id":"session-1","turn_id":"turn-1"});
    let with = |name: &str, extra: Value| {
        let mut v = base(name);
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        v
    };
    vec![
        (
            "prompt_redacted",
            with(
                "UserPromptSubmit",
                json!({"prompt":format!("port 8080 {SECRET} API_KEY=abc123456789")}),
            ),
            policy(|_| {}),
        ),
        (
            "prompt_plain",
            with("UserPromptSubmit", json!({"prompt":"Amber.port = 8080"})),
            policy(|_| {}),
        ),
        (
            "prompt_empty",
            with("UserPromptSubmit", json!({"prompt":""})),
            policy(|_| {}),
        ),
        (
            "prompt_excluded_kind",
            with("UserPromptSubmit", json!({"prompt":"hello"})),
            policy(|p| p.kinds = vec!["reply".into()]),
        ),
        (
            "prompt_excluded_term",
            with(
                "UserPromptSubmit",
                json!({"prompt":"contains forbidden-term here"}),
            ),
            policy(|p| p.excluded_content = vec!["forbidden-term".into()]),
        ),
        (
            "prompt_truncated",
            with(
                "UserPromptSubmit",
                json!({"prompt":format!("{} {SECRET}", "a".repeat(3000))}),
            ),
            policy(|p| p.max_event_bytes = 1024),
        ),
        (
            "prompt_no_turn",
            json!({"hook_event_name":"UserPromptSubmit","session_id":"session-1","prompt":"orphan"}),
            policy(|_| {}),
        ),
        (
            "tool_structured",
            with(
                "PostToolUse",
                json!({"tool_use_id":"tool-1","tool_name":"Bash","tool_input":{"command":"ls","api_key":"abc"},"tool_response":{"stdout":SECRET,"exit_code":0}}),
            ),
            policy(|_| {}),
        ),
        (
            "tool_failed",
            with(
                "PostToolUse",
                json!({"tool_use_id":"tool-1","tool_name":"Bash","tool_input":{"command":"false"},"tool_response":{"stdout":"","exit_code":2}}),
            ),
            policy(|_| {}),
        ),
        (
            "tool_excluded_name",
            with(
                "PostToolUse",
                json!({"tool_use_id":"tool-1","tool_name":"Bash","tool_input":{"command":"ls"},"tool_response":{"stdout":"x"}}),
            ),
            policy(|p| p.excluded_tools = vec!["Bash".into()]),
        ),
        (
            "tool_own_mcp",
            with(
                "PostToolUse",
                json!({"tool_use_id":"tool-1","tool_name":"mcp__recollect__memory.recall","tool_input":{},"tool_response":{"stdout":"x"}}),
            ),
            policy(|_| {}),
        ),
        (
            "tool_sensitive_input",
            with(
                "PostToolUse",
                json!({"tool_use_id":"tool-1","tool_name":"Read","tool_input":{"path":"/home/me/.env"},"tool_response":{"stdout":"x"}}),
            ),
            policy(|_| {}),
        ),
        (
            "tool_excluded_term_in_raw",
            with(
                "PostToolUse",
                json!({"tool_use_id":"tool-1","tool_name":"Bash","tool_input":{"command":"ls"},"tool_response":{"stdout":format!("pre {SECRET} forbidden-term")}}),
            ),
            policy(|p| p.excluded_content = vec!["forbidden-term".into()]),
        ),
        (
            "tool_truncated",
            with(
                "PostToolUse",
                json!({"tool_use_id":"tool-1","tool_name":"Bash","tool_input":{"command":"ls"},"tool_response":{"stdout":"b".repeat(4000)}}),
            ),
            policy(|p| p.max_event_bytes = 1024),
        ),
        (
            "reply",
            with("Stop", json!({"last_assistant_message":"done"})),
            policy(|_| {}),
        ),
        (
            "subagent_stop_orphan",
            with("SubagentStop", json!({"last_assistant_message":"done"})),
            policy(|_| {}),
        ),
        (
            "lifecycle",
            with("SessionStart", json!({"source":"startup"})),
            policy(|_| {}),
        ),
        (
            "disabled",
            with("UserPromptSubmit", json!({"prompt":"x"})),
            CapturePolicy::default(),
        ),
    ]
}

fn admission_cases() -> Vec<(&'static str, CapturedHook, CapturePolicy)> {
    let tool = |content: &str| hook("tool_result", "PostToolUse", Some(content));
    vec![
        (
            "prompt_redacted",
            hook(
                "prompt",
                "UserPromptSubmit",
                Some(&format!("port 8080 {SECRET} API_KEY=abc123456789")),
            ),
            policy(|_| {}),
        ),
        (
            "prompt_plain",
            hook("prompt", "UserPromptSubmit", Some("Amber.port = 8080")),
            policy(|_| {}),
        ),
        (
            "prompt_empty",
            hook("prompt", "UserPromptSubmit", Some("")),
            policy(|_| {}),
        ),
        (
            "prompt_excluded_kind",
            hook("prompt", "UserPromptSubmit", Some("hello")),
            policy(|p| p.kinds = vec!["reply".into()]),
        ),
        (
            "prompt_excluded_term",
            hook(
                "prompt",
                "UserPromptSubmit",
                Some("contains forbidden-term here"),
            ),
            policy(|p| p.excluded_content = vec!["forbidden-term".into()]),
        ),
        (
            "prompt_truncated",
            hook(
                "prompt",
                "UserPromptSubmit",
                Some(&format!("{} {SECRET}", "a".repeat(3000))),
            ),
            policy(|p| p.max_event_bytes = 1024),
        ),
        (
            "prompt_no_turn",
            CapturedHook {
                turn_id: None,
                ..hook("prompt", "UserPromptSubmit", Some("orphan"))
            },
            policy(|_| {}),
        ),
        (
            "tool_json",
            tool(
                &json!({"input":{"command":"ls","api_key":"abc"},"output":{"stdout":SECRET}})
                    .to_string(),
            ),
            policy(|_| {}),
        ),
        (
            "tool_text",
            tool(&format!("plain output {SECRET}")),
            policy(|_| {}),
        ),
        (
            "tool_excluded_name",
            tool("x"),
            policy(|p| p.excluded_tools = vec!["Bash".into()]),
        ),
        (
            "tool_envelope_sensitive",
            tool(&json!({"input":{"path":"/home/me/.env"},"output":"x"}).to_string()),
            policy(|_| {}),
        ),
        (
            "tool_excluded_term",
            tool("pre forbidden-term"),
            policy(|p| p.excluded_content = vec!["forbidden-term".into()]),
        ),
        (
            "tool_truncated",
            tool(&"b".repeat(4000)),
            policy(|p| p.max_event_bytes = 1024),
        ),
        ("reply", hook("reply", "Stop", Some("done")), policy(|_| {})),
        (
            "subagent_stop_orphan",
            hook("reply", "SubagentStop", Some("done")),
            policy(|_| {}),
        ),
        (
            "lifecycle_with_content",
            hook("lifecycle", "SessionStart", Some("leaked")),
            policy(|_| {}),
        ),
        (
            "invalid_kind",
            hook("reply", "UserPromptSubmit", Some("x")),
            policy(|_| {}),
        ),
    ]
}

/// Output of both entry points, recorded before they were collapsed into
/// `apply_policy`. Admission is byte-for-byte unchanged. Local output differs
/// only in that coverage flags are now sorted and de-duplicated like admission's.
const LOCAL_EXPECTED: &[(&str, &str)] = &[
    (
        "prompt_redacted",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=Some("port 8080 [redacted] [redacted]") coverage=["partial_host_coverage", "redacted"]"##,
    ),
    (
        "prompt_plain",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=Some("Amber.port = 8080") coverage=["partial_host_coverage"]"##,
    ),
    (
        "prompt_empty",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=None coverage=["missing_content", "partial_host_coverage"]"##,
    ),
    (
        "prompt_excluded_kind",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_event_kind", "partial_host_coverage"]"##,
    ),
    (
        "prompt_excluded_term",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_content", "partial_host_coverage"]"##,
    ),
    (
        "prompt_truncated",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa…[1024 bytes]") coverage=["partial_host_coverage", "redacted", "truncated"]"##,
    ),
    (
        "prompt_no_turn",
        r##"UserPromptSubmit/prompt/reported turn=None agent=None content=Some("orphan") coverage=["missing_turn_identity", "partial_host_coverage"]"##,
    ),
    (
        "tool_structured",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=Some("{\n  \"input\": {\n    [redacted],\n    \"comm…[122 bytes]") coverage=["partial_host_coverage", "redacted"]"##,
    ),
    (
        "tool_failed",
        r##"PostToolUse/tool_result/failed turn=Some("turn-1") agent=None content=Some("{\n  \"input\": {\n    \"command\": \"false\"\n  },\n  \"output\": {\n    \"exit_code\": 2,\n    \"stdout\": \"\"\n  }\n}") coverage=["partial_host_coverage"]"##,
    ),
    (
        "tool_excluded_name",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_tool_content", "partial_host_coverage"]"##,
    ),
    (
        "tool_own_mcp",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_tool_content", "partial_host_coverage"]"##,
    ),
    (
        "tool_sensitive_input",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_tool_content", "partial_host_coverage"]"##,
    ),
    (
        "tool_excluded_term_in_raw",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_content", "partial_host_coverage"]"##,
    ),
    (
        "tool_truncated",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=Some("{\n  \"input\": {\n    \"command\": \"ls\"\n  },\n…[1024 bytes]") coverage=["partial_host_coverage", "truncated"]"##,
    ),
    (
        "reply",
        r##"Stop/reply/reported turn=Some("turn-1") agent=None content=Some("done") coverage=["partial_host_coverage"]"##,
    ),
    (
        "subagent_stop_orphan",
        r##"SubagentStop/reply/reported turn=Some("turn-1") agent=None content=None coverage=["ambiguous_attribution", "partial_host_coverage"]"##,
    ),
    (
        "lifecycle",
        r##"SessionStart/lifecycle/reported turn=Some("turn-1") agent=None content=None coverage=["partial_host_coverage"]"##,
    ),
    ("disabled", r##"error:capture_disabled"##),
];
const ADMISSION_EXPECTED: &[(&str, &str)] = &[
    (
        "prompt_redacted",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=Some("port 8080 [redacted] [redacted]") coverage=["partial_host_coverage", "redacted"]"##,
    ),
    (
        "prompt_plain",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=Some("Amber.port = 8080") coverage=["partial_host_coverage"]"##,
    ),
    (
        "prompt_empty",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=None coverage=["partial_host_coverage"]"##,
    ),
    (
        "prompt_excluded_kind",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_event_kind", "partial_host_coverage"]"##,
    ),
    (
        "prompt_excluded_term",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_content", "partial_host_coverage"]"##,
    ),
    (
        "prompt_truncated",
        r##"UserPromptSubmit/prompt/reported turn=Some("turn-1") agent=None content=Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa…[1024 bytes]") coverage=["partial_host_coverage", "redacted", "truncated"]"##,
    ),
    (
        "prompt_no_turn",
        r##"UserPromptSubmit/prompt/reported turn=None agent=None content=None coverage=["ambiguous_attribution", "missing_turn_identity", "partial_host_coverage"]"##,
    ),
    (
        "tool_json",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=Some("{\n  \"input\": {\n    \"api_key\": \"[redacted]\",\n    \"command\": \"ls\"\n  },\n  \"output\": {\n    \"stdout\": \"[redacted]\"\n  }\n}") coverage=["partial_host_coverage", "redacted"]"##,
    ),
    (
        "tool_text",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=Some("plain output [redacted]") coverage=["partial_host_coverage", "redacted"]"##,
    ),
    (
        "tool_excluded_name",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_tool_content", "partial_host_coverage"]"##,
    ),
    (
        "tool_envelope_sensitive",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_tool_content", "partial_host_coverage"]"##,
    ),
    (
        "tool_excluded_term",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=None coverage=["excluded_content", "partial_host_coverage"]"##,
    ),
    (
        "tool_truncated",
        r##"PostToolUse/tool_result/reported turn=Some("turn-1") agent=None content=Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb…[1024 bytes]") coverage=["partial_host_coverage", "truncated"]"##,
    ),
    (
        "reply",
        r##"Stop/reply/reported turn=Some("turn-1") agent=None content=Some("done") coverage=["partial_host_coverage"]"##,
    ),
    (
        "subagent_stop_orphan",
        r##"SubagentStop/reply/reported turn=Some("turn-1") agent=None content=None coverage=["ambiguous_attribution", "partial_host_coverage"]"##,
    ),
    (
        "lifecycle_with_content",
        r##"SessionStart/lifecycle/reported turn=Some("turn-1") agent=None content=None coverage=["partial_host_coverage"]"##,
    ),
    ("invalid_kind", r##"error:invalid_capture_event"##),
];

#[test]
fn local_normalization_output_is_pinned() {
    let cases = local_cases();
    assert_eq!(cases.len(), LOCAL_EXPECTED.len());
    for ((name, raw, p), (expected_name, expected)) in cases.into_iter().zip(LOCAL_EXPECTED) {
        assert_eq!(name, *expected_name);
        assert_eq!(local(raw, &p), *expected, "local case {name}");
    }
}

#[test]
fn admission_output_is_pinned() {
    let cases = admission_cases();
    assert_eq!(cases.len(), ADMISSION_EXPECTED.len());
    for ((name, event, p), (expected_name, expected)) in cases.into_iter().zip(ADMISSION_EXPECTED) {
        assert_eq!(name, *expected_name);
        assert_eq!(admit(event, &p), *expected, "admission case {name}");
    }
}

/// The two entry points apply one policy: re-admitting what local normalization
/// produced (under the same policy) changes nothing.
#[test]
fn admission_accepts_local_output_unchanged() {
    let secrets = [SECRET.to_owned()];
    for (name, raw, p) in local_cases() {
        let Ok(event) = normalize_capture_hook(
            "codex",
            &serde_json::to_vec(&raw).unwrap(),
            &p,
            &secrets,
            at(),
        ) else {
            continue;
        };
        // The one deliberate divergence: local capture keeps content that has no
        // turn identity, while admission refuses it as unattributable.
        if event.turn_id.is_none() {
            continue;
        }
        let mut admitted = event.clone();
        sanitize_captured_event(&mut admitted, &p, &secrets).unwrap();
        assert_eq!(admitted, event, "re-admission changed local case {name}");
    }
}
