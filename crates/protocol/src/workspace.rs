use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Canonical identity only. Never return the raw input in errors.
pub fn canonical_origin(input: &str) -> Result<String, &'static str> {
    let input = input.trim();
    let invalid =
        "Use a full HTTPS/SSH repository origin without query, fragment or ambiguous path.";
    if input.is_empty()
        || input.len() > 2000
        || input.chars().any(char::is_whitespace)
        || input.chars().any(char::is_control)
        || input.contains(['?', '#', '\\'])
    {
        return Err(invalid);
    }
    let transport = input.contains("://")
        || (!input.starts_with('[')
            && ((!input.contains('/') && input.contains(':'))
                || input.split_once(':').is_some_and(|(host, path)| {
                    !host.contains('/')
                        && (host.contains('@')
                            || !path
                                .split('/')
                                .next()
                                .unwrap_or("")
                                .chars()
                                .all(|c| c.is_ascii_digit()))
                })));
    let text = if input.contains("://") {
        input.to_owned()
    } else if transport {
        let (host, path) = input.split_once(':').ok_or(invalid)?;
        if host.contains(['[', ']']) {
            return Err(invalid);
        }
        format!("ssh://{host}/{path}")
    } else {
        format!("recollect://{input}")
    };
    let url = url::Url::parse(&text).map_err(|_| invalid)?;
    if !matches!(url.scheme(), "http" | "https" | "ssh" | "recollect") {
        return Err(invalid);
    }
    let host = url.host_str().ok_or(invalid)?.to_lowercase();
    if host.is_empty() {
        return Err(invalid);
    }
    let raw_path = text
        .split_once("://")
        .and_then(|(_, s)| s.split_once('/'))
        .map(|(_, p)| p)
        .ok_or(invalid)?;
    if raw_path.starts_with('/')
        || raw_path.contains("//")
        || raw_path.contains('%')
        || raw_path.split('/').any(|p| p == "." || p == "..")
    {
        return Err(invalid);
    }
    let mut path = url.path().trim_start_matches('/').trim_end_matches('/');
    if transport {
        path = path.strip_suffix(".git").unwrap_or(path);
    }
    if path.is_empty() || path.split('/').any(|p| p.is_empty() || p.contains('@')) {
        return Err(invalid);
    }
    let port = url
        .port()
        .filter(|port| !(url.scheme() == "ssh" && *port == 22));
    Ok(match port {
        Some(port) => format!("{host}:{port}/{path}"),
        None => format!("{host}/{path}"),
    })
}

/// Git origins without a transport use SCP syntax or are local paths, never canonical keys.
pub fn canonical_git_origin(input: &str) -> Result<String, &'static str> {
    if input.contains("://") {
        return canonical_origin(input);
    }
    let (host, path) = input
        .split_once(':')
        .ok_or("Local repository origins are not published automatically.")?;
    if host.is_empty()
        || host.contains(['/', '\\', '[', ']'])
        || (host.len() == 1 && host.as_bytes()[0].is_ascii_alphabetic())
    {
        return Err("Local or ambiguous repository origins are not published automatically.");
    }
    canonical_origin(&format!("ssh://{host}/{path}"))
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Repository {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub canonical_origin: String,
    pub origins: Vec<String>,
    pub created_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckoutObservation {
    pub local_path: String,
    pub origin: Option<String>,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub dirty: Option<bool>,
    pub status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckoutRefresh {
    pub workspace_root: String,
    pub complete: bool,
    pub notes: Vec<String>,
    pub checkouts: Vec<CheckoutObservation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceRegistration {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub device_id: Uuid,
    pub root: String,
    pub complete: bool,
    pub notes: Vec<String>,
    pub refreshed_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckoutRegistration {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub repository_id: Option<Uuid>,
    pub observation: CheckoutObservation,
    pub present: bool,
    pub observed_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckoutRefreshResult {
    pub workspace: WorkspaceRegistration,
    pub observed_checkouts: i64,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ScopeSelection {
    #[serde(default)]
    pub repository_ids: Vec<Uuid>,
    #[serde(default)]
    pub area_ids: Vec<Uuid>,
    pub environment_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ScopeResource {
    pub id: Uuid,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ScopeSnapshot {
    pub id: Uuid,
    pub task_id: Uuid,
    pub brain_id: Uuid,
    pub selection: ScopeSelection,
    pub repositories: Vec<ScopeResource>,
    pub areas: Vec<ScopeResource>,
    pub environment: Option<ScopeResource>,
    pub created_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceTask {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub parent_task_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub created_by: Uuid,
    pub device_id: Option<Uuid>,
    pub label: String,
    pub closed: bool,
    pub scope: ScopeSnapshot,
    pub scope_valid: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OperationBinding {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub task_id: Uuid,
    pub scope: ScopeSnapshot,
    pub scope_valid: bool,
    pub kind: String,
    pub actor_id: Uuid,
    pub device_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ScopeHandoff {
    pub previous_scope_id: Option<Uuid>,
    pub current_scope_id: Uuid,
    pub fresh_context_required: bool,
    pub retrieval_available: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TaskChange {
    pub task: WorkspaceTask,
    pub handoff: ScopeHandoff,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateTask {
    pub label: String,
    pub parent_task_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub selection: Option<ScopeSelection>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ChangeScope {
    pub base_scope: Uuid,
    pub selection: ScopeSelection,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct StartOperation {
    pub kind: String,
    pub expected_scope: Option<Uuid>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepositoryOrigin {
    pub origin: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct TaskDetail {
    pub task: WorkspaceTask,
    pub scopes: Vec<ScopeSnapshot>,
    pub scope_total: i64,
    pub scope_offset: i64,
    pub operations: Vec<OperationBinding>,
    pub operation_total: i64,
    pub operation_offset: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct WorkspaceCatalogue {
    pub repositories: Vec<Repository>,
    pub areas: Vec<ScopeResource>,
    pub environments: Vec<ScopeResource>,
    pub workspaces: Vec<WorkspaceRegistration>,
    pub selected_workspace: Option<Uuid>,
    pub checkouts: Vec<CheckoutRegistration>,
    pub checkout_total: i64,
    pub checkout_offset: i64,
    pub tasks: Vec<WorkspaceTask>,
    pub task_total: i64,
    pub task_offset: i64,
}

#[cfg(test)]
mod tests {
    use super::{canonical_git_origin, canonical_origin as origin};
    #[test]
    fn origins_preserve_repository_identity_without_credentials() {
        for value in [
            "https://member:discard@example.test/Team/repo.git",
            "git@example.test:Team/repo.git",
            "ssh://git@EXAMPLE.test:22/Team/repo.git",
            "example.test/Team/repo",
        ] {
            assert_eq!(origin(value).unwrap(), "example.test/Team/repo");
        }
        let custom = origin("ssh://git@example.test:443/Team/repo.git").unwrap();
        assert_eq!(custom, "example.test:443/Team/repo");
        assert_eq!(origin(&custom).unwrap(), custom);
        let ipv6 = origin("ssh://git@[::1]:2222/Team/repo.git").unwrap();
        assert_eq!(origin(&ipv6).unwrap(), ipv6);
        assert_eq!(
            canonical_git_origin("example.test:123/repo.git").unwrap(),
            "example.test/123/repo"
        );
        for local in [
            "../repo",
            "repo/subdir",
            "/tmp/repo",
            "C:/repo",
            "file:///tmp/repo",
        ] {
            assert!(canonical_git_origin(local).is_err());
        }
        assert_ne!(
            origin("https://example.test/Team/fork.git"),
            origin("https://example.test/Team/repo.git")
        );
        for value in [
            "/tmp/local",
            "file:///tmp/local",
            "https://example.test/repo?token=discard",
            "https://example.test/a/../repo",
            "https://example.test/a%2frepo",
            "https://example.test//repo",
        ] {
            assert!(origin(value).is_err(), "{value}");
        }
    }
}
