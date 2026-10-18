use crate::error::{Error, Result};
use recollect_protocol::{CoordinationSession, CoordinationSessionInput};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(super) fn text(value: &str, max: usize, empty: bool) -> bool {
    value.chars().count() <= max
        && (empty || !value.trim().is_empty())
        && !value.chars().any(char::is_control)
}
pub(super) fn validate(input: &CoordinationSessionInput) -> Result<()> {
    if !text(&input.label, 120, false)
        || !text(&input.goal, 2000, true)
        || !text(&input.host_kind, 40, false)
        || !matches!(
            input.host_kind.as_str(),
            "codex" | "claude-code" | "claude_code" | "opencode" | "other"
        )
        || input.context_repository_ids.len() > 100
        || input.topics.len() > 32
        || input.intents.len() > 128
        || input.expected_revision.is_some_and(|v| v < 1)
        || input.topics.iter().any(|t| !text(t, 80, false))
    {
        return Err(Error::invalid(
            "Use bounded printable labels, goals, topics and a supported host kind.",
        ));
    }
    for intent in &input.intents {
        let path = intent.path.trim_end_matches('/');
        if !text(&intent.path, 1024, false)
            || path.is_empty()
            || intent.path.starts_with('/')
            || intent.path.contains(['\\', ':'])
            || path.split('/').any(|p| matches!(p, "" | "." | ".."))
            || intent.path.ends_with("//")
            || !matches!(intent.mode.as_str(), "read" | "write")
            || !matches!(intent.phase.as_str(), "planned" | "active")
        {
            return Err(Error::invalid(
                "Intents require repository-relative paths and read/write, planned/active values.",
            ));
        }
    }
    Ok(())
}
fn intersects(a: &str, b: &str) -> bool {
    a == b || a.ends_with('/') && b.starts_with(a) || b.ends_with('/') && a.starts_with(b)
}
fn normalized(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn tokens(value: &str) -> BTreeSet<String> {
    const STOP: &[&str] = &[
        "about",
        "after",
        "agent",
        "also",
        "before",
        "brain",
        "from",
        "goal",
        "have",
        "into",
        "need",
        "recollect",
        "same",
        "some",
        "task",
        "that",
        "their",
        "then",
        "there",
        "these",
        "this",
        "using",
        "want",
        "will",
        "with",
        "work",
        "working",
    ];
    value
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|s| s.chars().count() >= 4 && !STOP.contains(&s.as_str()))
        .collect()
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Reason {
    pub key: String,
    pub reason: String,
    pub paths: Vec<String>,
    pub tokens: Vec<String>,
    pub basis: Vec<String>,
}
pub(super) fn reasons(a: &CoordinationSession, b: &CoordinationSession) -> Vec<Reason> {
    let mut found = std::collections::BTreeMap::new();
    for x in &a.intents {
        for y in &b.intents {
            if x.repository_id != y.repository_id || !intersects(&x.path, &y.path) {
                continue;
            }
            let reason = if x.mode == "write" || y.mode == "write" {
                "file_write_overlap"
            } else {
                "file_read_overlap"
            };
            let mut paths = vec![x.path.clone(), y.path.clone()];
            paths.sort();
            paths.dedup();
            let key = format!("{reason}:{}:{}", x.repository_id, paths.join("|"));
            found.insert(
                key.clone(),
                Reason {
                    key,
                    reason: reason.into(),
                    paths,
                    tokens: vec![],
                    basis: vec![format!("repository:{}", x.repository_id)],
                },
            );
        }
    }
    if a.environment_id.is_some()
        && b.environment_id.is_some()
        && a.environment_id != b.environment_id
    {
        return found.into_values().collect();
    }
    let mut basis = Vec::new();
    for r in &a.repository_ids {
        if b.repository_ids.contains(r) {
            basis.push(format!("repository:{r}"))
        }
    }
    for r in &a.area_ids {
        if b.area_ids.contains(r) {
            basis.push(format!("area:{r}"))
        }
    }
    if let Some(env) = a.environment_id.filter(|e| Some(*e) == b.environment_id) {
        basis.push(format!("environment:{env}"))
    }
    if !basis.is_empty() {
        let at: BTreeSet<_> = a.topics.iter().map(|s| normalized(s)).collect();
        let bt: BTreeSet<_> = b.topics.iter().map(|s| normalized(s)).collect();
        let mut shared: Vec<_> = at.intersection(&bt).cloned().collect();
        let ag = tokens(&a.goal);
        let bg = tokens(&b.goal);
        let goal: Vec<_> = ag.intersection(&bg).cloned().collect();
        if goal.len() >= 2 {
            shared.extend(goal)
        }
        shared.sort();
        shared.dedup();
        if !shared.is_empty() {
            found.insert(
                "topic".into(),
                Reason {
                    key: "topic".into(),
                    reason: "topic_overlap".into(),
                    paths: vec![],
                    tokens: shared,
                    basis,
                },
            );
        }
    }
    found.into_values().collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_paths_reject_traversal_controls_and_url() {
        let mut input = CoordinationSessionInput {
            id: uuid::Uuid::new_v4(),
            task_id: uuid::Uuid::new_v4(),
            expected_scope: uuid::Uuid::new_v4(),
            expected_revision: None,
            context_repository_ids: vec![],
            host_kind: "codex".into(),
            label: "Agent".into(),
            goal: "Investigate vault issuance".into(),
            topics: vec![],
            intents: vec![],
        };
        for p in [
            "/tmp/secret",
            "a/../b",
            "a//b",
            "./a",
            "C:/a",
            "a\\b",
            "https://host/a",
            "a/\u{1b}",
        ] {
            input.intents = vec![recollect_protocol::CoordinationIntent {
                repository_id: uuid::Uuid::new_v4(),
                path: p.into(),
                mode: "write".into(),
                phase: "planned".into(),
            }];
            assert!(validate(&input).is_err(), "{p}")
        }
        input.intents[0].path = "charts/vault/".into();
        assert!(validate(&input).is_ok());
    }
    #[test]
    fn path_prefix_is_explicit_and_case_sensitive() {
        assert!(intersects("src/", "src/lib.rs"));
        assert!(!intersects("src", "src/lib.rs"));
        assert!(!intersects("src/", "src-other/a"));
        assert!(!intersects("Foo.rs", "foo.rs"));
    }
}

#[cfg(test)]
mod overlap_tests {
    use super::*;
    use chrono::Utc;
    use recollect_protocol::CoordinationIntent;
    use uuid::Uuid;
    fn peer(repo: Uuid) -> CoordinationSession {
        CoordinationSession {
            id: Uuid::new_v4(),
            revision: 1,
            account_id: Uuid::new_v4(),
            account_name: "Synthetic".into(),
            device_id: Uuid::new_v4(),
            device_name: "Fixture".into(),
            host_kind: "codex".into(),
            label: "Work".into(),
            goal: "Inspect vault certificate renewal".into(),
            topics: vec![" Vault ".into()],
            intents: vec![CoordinationIntent {
                repository_id: repo,
                path: "charts/vault.yaml".into(),
                mode: "read".into(),
                phase: "planned".into(),
            }],
            environment_id: None,
            environment_name: None,
            repository_ids: vec![repo],
            area_ids: vec![],
            status: "active".into(),
            last_seen_at: Utc::now(),
            expires_at: Utc::now(),
            task_id: None,
            scope_id: None,
        }
    }
    #[test]
    fn overlap_is_explainable_and_environment_aware() {
        let repo = Uuid::new_v4();
        let a = peer(repo);
        let mut b = peer(repo);
        assert!(
            reasons(&a, &b)
                .iter()
                .any(|r| r.reason == "file_read_overlap")
        );
        assert!(reasons(&a, &b).iter().any(|r| r.reason == "topic_overlap"));
        b.intents[0].mode = "write".into();
        assert!(
            reasons(&a, &b)
                .iter()
                .any(|r| r.reason == "file_write_overlap")
        );
        let mut a = a;
        a.environment_id = Some(Uuid::new_v4());
        b.environment_id = Some(Uuid::new_v4());
        assert_eq!(reasons(&a, &b).len(), 1);
        b.intents[0].repository_id = Uuid::new_v4();
        b.repository_ids = vec![b.intents[0].repository_id];
        assert!(reasons(&a, &b).is_empty());
    }
    #[test]
    fn topic_matching_needs_real_context_and_two_distinct_goal_tokens() {
        let mut a = peer(Uuid::new_v4());
        let mut b = peer(Uuid::new_v4());
        a.intents.clear();
        b.intents.clear();
        assert!(reasons(&a, &b).is_empty());
        b.repository_ids = a.repository_ids.clone();
        a.topics.clear();
        b.topics.clear();
        b.goal = "vault vault vault".into();
        assert!(reasons(&a, &b).is_empty());
        b.goal = "vault renewal".into();
        let result = reasons(&a, &b);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tokens, vec!["renewal", "vault"]);
    }
}
