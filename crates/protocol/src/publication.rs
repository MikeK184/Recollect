use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use utoipa::ToSchema;
use uuid::Uuid;

pub const PUBLICATION_MAX_BYTES: usize = 24 * 1024 * 1024;
pub const FACTS_MAX_BYTES: usize = 10 * 1024 * 1024;
pub const REPOSITORY_TEXT_MAX_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RepositoryPolicy {
    pub allow_file_content: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ExtractedFile {
    pub path: String,
    pub object_id: String,
    pub mode: String,
    pub size: Option<u64>,
    pub status: String,
    pub extraction: String,
    pub content: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicationInput {
    pub publication_id: Uuid,
    pub operation_id: Uuid,
    pub origin: String,
    pub revision: String,
    pub branch: Option<String>,
    pub dirty: bool,
    pub captured_at: DateTime<Utc>,
    pub adapter: String,
    pub adapter_build: String,
    pub extractor_version: String,
    pub settings: Value,
    pub files: Vec<ExtractedFile>,
    pub facts: Vec<Value>,
    pub insights: Value,
    pub receipt: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RepositorySnapshot {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub repository_id: Uuid,
    pub revision: String,
    pub adapter: String,
    pub adapter_build: String,
    pub extractor_version: String,
    pub settings: Value,
    pub coverage: Value,
    pub file_count: i64,
    pub fact_count: i64,
    pub retained_file_count: i64,
    pub processing: String,
    pub job: Option<crate::Job>,
    pub created_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RepositoryContribution {
    pub id: Uuid,
    pub actor_id: Uuid,
    pub device_id: Uuid,
    pub operation_id: Uuid,
    pub scope: crate::ScopeSnapshot,
    pub origin: String,
    pub branch: Option<String>,
    pub dirty: bool,
    pub captured_at: DateTime<Utc>,
    pub accepted_at: DateTime<Utc>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PublicationResult {
    pub snapshot: RepositorySnapshot,
    pub contribution_id: Uuid,
    pub reused: bool,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SnapshotPage {
    pub items: Vec<RepositorySnapshot>,
    pub total: i64,
    pub offset: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SnapshotDetail {
    pub snapshot: RepositorySnapshot,
    pub contributors: Vec<RepositoryContribution>,
    pub contributor_total: i64,
    pub contributor_offset: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepositoryFile {
    pub id: Uuid,
    pub path: String,
    pub object_id: String,
    pub mode: String,
    pub size: Option<i64>,
    pub status: String,
    pub extraction: String,
    pub availability: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepositoryFilePage {
    pub items: Vec<RepositoryFile>,
    pub total: i64,
    pub offset: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepositoryFileContent {
    pub file: RepositoryFile,
    pub content: Option<String>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepositoryFact {
    pub id: Uuid,
    pub ordinal: i32,
    pub record: Value,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepositoryFactPage {
    pub items: Vec<RepositoryFact>,
    pub total: i64,
    pub offset: i64,
    pub processing: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepositoryArtifact {
    pub kind: String,
    pub availability: String,
    pub content: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ManifestEntry {
    pub repository_id: Uuid,
    pub revision: String,
    pub snapshot_id: Option<Uuid>,
    #[serde(default)]
    pub config_paths: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ManifestInput {
    pub environment_id: Uuid,
    pub name: String,
    pub kind: String,
    pub entries: Vec<ManifestEntry>,
    pub base_revision: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub observed_at: Option<DateTime<Utc>>,
    pub observation_reference: Option<String>,
    #[serde(default)]
    pub notes: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ManifestRevision {
    pub id: Uuid,
    pub manifest_id: Uuid,
    pub brain_id: Uuid,
    pub environment_id: Uuid,
    pub name: String,
    pub kind: String,
    pub entries: Vec<ManifestEntry>,
    pub scope: Option<crate::ScopeSnapshot>,
    pub actor_id: Uuid,
    pub observed_at: Option<DateTime<Utc>>,
    pub observation_reference: Option<String>,
    pub notes: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ManifestPage {
    pub items: Vec<ManifestRevision>,
    pub total: i64,
    pub offset: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ManifestDetail {
    pub current: ManifestRevision,
    pub history: Vec<ManifestRevision>,
    pub total: i64,
    pub offset: i64,
}

pub fn repository_path(path: &str) -> bool {
    !path.is_empty()
        && path.chars().count() <= 2000
        && !path.chars().any(char::is_control)
        && !path.contains(['\\', ':'])
        && !path.split('/').any(|p| matches!(p, "" | "." | ".."))
}
pub fn git_object_id(id: &str) -> bool {
    matches!(id.len(), 40 | 64)
        && id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn excluded_repository_path(path: &str) -> bool {
    path.split('/').any(|p| {
        let p = p.to_ascii_lowercase();
        matches!(
            p.as_str(),
            ".git"
                | ".recollect"
                | ".codex"
                | ".claude"
                | ".ssh"
                | ".aws"
                | ".kube"
                | ".terraform"
                | ".enola"
                | ".env"
                | "credentials"
                | "credentials.json"
                | "credentials.yaml"
                | "credentials.yml"
                | "id_rsa"
                | "id_ed25519"
                | "id_ecdsa"
                | ".netrc"
                | ".npmrc"
                | ".pypirc"
                | "kubeconfig"
        ) || p.starts_with(".env.")
            || p.starts_with(".secrets")
            || p.contains(".tfstate")
            || [".pem", ".key", ".p12", ".pfx", ".jks", ".keystore"]
                .iter()
                .any(|suffix| p.ends_with(suffix))
    })
}
/// Bounded whole-file exclusion. Returns only a boolean, never matched material.
pub fn repository_secret(text: &str, configured: &[String]) -> bool {
    if configured.iter().any(|v| !v.is_empty() && text.contains(v)) {
        return true;
    }
    if text.contains("-----BEGIN ") && text.contains("PRIVATE KEY-----") {
        return true;
    }
    for prefix in [
        "sk-proj-",
        "sk-svcacct-",
        "ghp_",
        "github_pat_",
        "glpat-",
        "xoxb-",
        "xoxp-",
        "AKIA",
        "ASIA",
    ] {
        if text.match_indices(prefix).any(|(i, _)| {
            text[i + prefix.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
                .count()
                >= 12
        }) {
            return true;
        }
    }
    for token in text.split(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | '<' | '>')) {
        let token = token.strip_prefix("git::").unwrap_or(token);
        if (token.starts_with("https://")
            || token.starts_with("http://")
            || token.starts_with("ssh://"))
            && url::Url::parse(token).is_ok_and(|u| {
                u.password().is_some() || (u.scheme() != "ssh" && !u.username().is_empty())
            })
        {
            return true;
        }
    }
    text.lines().any(|line| {
        let Some((key, value)) = line.split_once('=').or_else(|| line.split_once(':')) else {
            return false;
        };
        let key = key
            .trim()
            .trim_matches(['"', '\''])
            .to_ascii_lowercase()
            .replace('-', "_");
        let sensitive = matches!(
            key.as_str(),
            "password"
                | "passwd"
                | "token"
                | "secret"
                | "api_key"
                | "apikey"
                | "access_key"
                | "secret_key"
                | "client_secret"
        ) || key.ends_with("_password")
            || key.ends_with("_secret")
            || key.ends_with("_token")
            || key.ends_with("_api_key");
        let value = value
            .trim()
            .trim_matches([',', ';'])
            .trim()
            .trim_matches(['"', '\'']);
        sensitive
            && value.len() >= 4
            && !value.starts_with(['$', '<'])
            && !value.contains("${")
            && ![
                "var.",
                "env.",
                "process.env",
                "std::env",
                "os.environ",
                "null",
                "None",
            ]
            .iter()
            .any(|p| value.starts_with(p))
    })
}
fn json_bounded(value: &Value, depth: usize) -> bool {
    if depth > 16 {
        return false;
    }
    match value {
        Value::String(s) => s.len() <= 65536,
        Value::Array(v) => v.len() <= 100_000 && v.iter().all(|v| json_bounded(v, depth + 1)),
        Value::Object(v) => {
            v.len() <= 1000
                && v.iter()
                    .all(|(k, v)| k.len() <= 256 && json_bounded(v, depth + 1))
        }
        _ => true,
    }
}
fn artifact_locations(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().all(artifact_locations),
        Value::Object(items) => items.iter().all(|(key, value)| {
            (!matches!(key.as_str(), "file" | "file_path" | "repo_path")
                || value.is_null()
                || value
                    .as_str()
                    .is_some_and(|path| path == "." || repository_path(path)))
                && artifact_locations(value)
        }),
        _ => true,
    }
}
pub fn validate_repository_facts(
    facts: &[Value],
    files: &[ExtractedFile],
) -> Result<(), &'static str> {
    if facts.len() > 100_000
        || serde_json::to_vec(facts)
            .map_err(|_| "Invalid facts")?
            .len()
            > FACTS_MAX_BYTES
    {
        return Err("Repository facts exceed the publication limit.");
    }
    let paths: BTreeSet<_> = files
        .iter()
        .filter(|f| f.status == "materialized")
        .map(|f| f.path.as_str())
        .collect();
    for fact in facts {
        if !fact.is_object()
            || !json_bounded(fact, 0)
            || !artifact_locations(fact)
            || fact.get("id").is_some_and(|id| !id.is_string())
            || !["kind", "name"].iter().all(|k| {
                fact.get(k)
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.is_empty() && s.len() <= 4096)
            })
        {
            return Err("Invalid repository fact shape.");
        }
        if let Some(location) = fact.get("file") {
            let path = location.as_str().ok_or("Invalid fact location.")?;
            if path != "."
                && (!repository_path(path)
                    || !paths.contains(path)
                        && !paths.iter().any(|p| p.starts_with(&format!("{path}/"))))
            {
                return Err("Fact location is outside the permitted committed inventory.");
            }
        }
        let mut start = None;
        for key in ["line", "end_line"] {
            if let Some(line) = fact.get(key) {
                let n = line
                    .as_u64()
                    .filter(|n| *n > 0 && *n <= 10_000_000)
                    .ok_or("Invalid fact span.")?;
                if key == "line" {
                    start = Some(n);
                } else if start.is_some_and(|s| n < s) {
                    return Err("Invalid fact span.");
                }
            }
        }
        if let Some(relations) = fact.get("relations") {
            let relations = relations.as_array().ok_or("Invalid fact relations.")?;
            for relation in relations {
                if !relation.is_object()
                    || !["kind", "target"].iter().all(|k| {
                        relation
                            .get(k)
                            .and_then(Value::as_str)
                            .is_some_and(|v| !v.is_empty())
                    })
                    || relation.get("target_id").is_some_and(|v| !v.is_string())
                {
                    return Err("Invalid fact relation.");
                }
            }
        }
    }
    Ok(())
}
pub fn validate_publication(input: &PublicationInput) -> Result<(), &'static str> {
    if serde_json::to_vec(input)
        .map_err(|_| "Invalid publication")?
        .len()
        > PUBLICATION_MAX_BYTES
    {
        return Err("Publication exceeds 24 MiB.");
    }
    if !git_object_id(&input.revision)
        || crate::canonical_origin(&input.origin).ok().as_deref() != Some(input.origin.as_str())
    {
        return Err("Use an exact commit and canonical repository origin.");
    }
    for value in [
        &input.adapter,
        &input.adapter_build,
        &input.extractor_version,
    ] {
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err("Invalid extractor identity.");
        }
    }
    if input
        .branch
        .as_ref()
        .is_some_and(|b| b.len() > 1024 || b.chars().any(char::is_control))
    {
        return Err("Invalid branch observation.");
    }
    if input.files.len() > 5000 {
        return Err("Repository inventory exceeds 5000 entries.");
    }
    let mut paths = BTreeSet::new();
    let mut retained = BTreeSet::new();
    let mut text_bytes = 0;
    let mut materialized_bytes = 0;
    for file in &input.files {
        if !repository_path(&file.path)
            || !paths.insert(&file.path)
            || !git_object_id(&file.object_id)
        {
            return Err("Invalid or duplicate committed file identity.");
        }
        if !matches!(
            file.mode.as_str(),
            "100644" | "100755" | "120000" | "160000"
        ) || !matches!(
            file.status.as_str(),
            "materialized"
                | "excluded_path"
                | "excluded_secret"
                | "binary"
                | "oversized"
                | "budget_exceeded"
                | "symlink"
                | "gitlink"
        ) || !matches!(
            file.extraction.as_str(),
            "facts_emitted" | "no_facts" | "excluded"
        ) {
            return Err("Invalid file coverage status.");
        }
        if file.size.is_some_and(|n| n > i64::MAX as u64) {
            return Err("Invalid file size.");
        }
        if (file.mode == "160000") != (file.status == "gitlink")
            || (file.mode == "120000") != (file.status == "symlink")
            || (file.mode == "160000") != file.size.is_none()
        {
            return Err("File mode, availability and size disagree.");
        }
        if file.status == "materialized" {
            if !matches!(file.mode.as_str(), "100644" | "100755")
                || excluded_repository_path(&file.path)
                || file
                    .size
                    .is_none_or(|n| n > REPOSITORY_TEXT_MAX_BYTES as u64)
                || file.extraction == "excluded"
            {
                return Err("Materialized file violates capture limits.");
            }
            materialized_bytes += file.size.unwrap_or(0);
        } else if file.content.is_some() || file.extraction != "excluded" {
            return Err("Excluded file cannot contain retained content or facts.");
        }
        if let Some(content) = &file.content {
            if file.size != Some(content.len() as u64)
                || content.contains('\0')
                || repository_secret(content, &[])
            {
                return Err("Retained file violates text capture rules.");
            }
            text_bytes += content.len();
            retained.insert(file.path.clone());
        }
    }
    if retained.len() > 20 || text_bytes > 8 * 1024 * 1024 || materialized_bytes > 64 * 1024 * 1024
    {
        return Err("Repository text exceeds capture limits.");
    }
    if !input.settings.is_object()
        || !json_bounded(&input.settings, 0)
        || serde_json::to_vec(&input.settings)
            .map_err(|_| "Invalid settings")?
            .len()
            > 65536
    {
        return Err("Invalid publication settings.");
    }
    let selected: Vec<String> = serde_json::from_value(
        input
            .settings
            .get("retained_files")
            .cloned()
            .unwrap_or(Value::Null),
    )
    .map_err(|_| "Settings must identify retained files.")?;
    if selected != retained.into_iter().collect::<Vec<_>>() {
        return Err("Retained files differ from publication settings.");
    }
    for (v, limit, object) in [
        (&input.insights, 1024 * 1024, false),
        (&input.receipt, 65536, true),
    ] {
        if (object && !v.is_object())
            || (!object && !v.is_array())
            || !json_bounded(v, 0)
            || !artifact_locations(v)
            || serde_json::to_vec(v)
                .map_err(|_| "Invalid extraction artifact")?
                .len()
                > limit
        {
            return Err("Invalid or oversized extraction artifact.");
        }
    }
    validate_repository_facts(&input.facts, &input.files)
}

/// Upstream runtime paths, timing and digests are deliberately not identity fields.
pub fn stable_receipt(receipt: &Value) -> Value {
    let mut value = serde_json::Map::new();
    for key in [
        "format_version",
        "enola_version",
        "extractor_version",
        "extractors",
        "explainers",
        "fact_count",
        "insight_count",
        "quality",
    ] {
        if let Some(v) = receipt.get(key) {
            value.insert(key.into(), v.clone());
        }
    }
    Value::Object(value)
}
