//! Curated, administrator-published guidance. This channel carries no execution grants.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ManagedSkillKind {
    Skill,
    Instructions,
}
impl ManagedSkillKind {
    pub fn entrypoint(self) -> &'static str {
        match self {
            Self::Skill => "SKILL.md",
            Self::Instructions => "AGENTS.md",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ManagedSkillFile {
    pub path: String,
    pub content: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ManagedSkillInput {
    pub name: String,
    pub description: String,
    pub kind: ManagedSkillKind,
    pub files: Vec<ManagedSkillFile>,
    pub environment_id: Option<Uuid>,
    pub profile_ids: Vec<Uuid>,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub expected_version: Option<i32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ManagedSkillSummary {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub name: String,
    pub description: String,
    pub kind: ManagedSkillKind,
    pub version: i32,
    pub environment_id: Option<Uuid>,
    pub profile_ids: Vec<Uuid>,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub file_paths: Vec<String>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ManagedSkillBundle {
    pub skill: ManagedSkillSummary,
    pub files: Vec<ManagedSkillFile>,
}

pub fn validate_managed_skill_path(path: &str) -> bool {
    let count = path.split('/').count();
    crate::repository_path(path)
        && path.len() <= 240
        && count <= 8
        && !crate::excluded_repository_path(path)
        && path.rsplit('/').next().is_some_and(|file| {
            file == "LICENSE" || file.ends_with(".md") || file.ends_with(".txt")
        })
}

fn text(value: &str, max: usize, required: bool) -> bool {
    (!required || !value.trim().is_empty())
        && value.chars().count() <= max
        && !value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frontmatter {
    name: String,
    description: String,
    license: Option<String>,
    compatibility: Option<String>,
    #[serde(default, deserialize_with = "bounded_metadata")]
    metadata: BTreeMap<String, String>,
}

fn bounded_metadata<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    struct Metadata;
    impl<'de> serde::de::Visitor<'de> for Metadata {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("at most 32 distinct bounded string metadata entries")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some(key) = map.next_key::<String>()? {
                if values.len() >= 32 || !text(&key, 64, true) || values.contains_key(&key) {
                    return Err(serde::de::Error::custom("invalid bounded metadata map"));
                }
                let value = map.next_value::<String>()?;
                if !text(&value, 1024, false) {
                    return Err(serde::de::Error::custom("invalid bounded metadata value"));
                }
                values.insert(key, value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(Metadata)
}

/// Shared server/client boundary; errors deliberately contain no authored text.
pub fn validate_managed_skill_bundle(
    kind: ManagedSkillKind,
    name: &str,
    description: &str,
    files: &[ManagedSkillFile],
) -> Result<(), &'static str> {
    if name.is_empty()
        || name.len() > 64
        || name.starts_with('-')
        || name.ends_with('-')
        || name.contains("--")
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err(
            "Use a 1–64 character lowercase letter/digit/hyphen name without edge or consecutive hyphens.",
        );
    }
    if !text(description, 1024, true) || crate::repository_secret(description, &[]) {
        return Err(
            "Use a nonempty description of at most 1024 characters without secrets or control characters.",
        );
    }
    if files.is_empty()
        || files.len() > 64
        || files.iter().map(|f| f.content.len()).sum::<usize>() > 512 * 1024
        || files.iter().map(|f| f.path.len()).sum::<usize>() > 2048
    {
        return Err(
            "Use 1–64 text files, at most 512 KiB of content and 2048 path bytes in total.",
        );
    }
    let mut paths = BTreeSet::new();
    let mut spellings = BTreeMap::new();
    for file in files {
        let normalized = file.path.to_lowercase();
        if !validate_managed_skill_path(&file.path)
            || !paths.insert(normalized)
            || file.content.len() > 128 * 1024
            || !text(&file.content, 128 * 1024, false)
            || file.content.contains("!`")
            || crate::repository_secret(&file.content, &[])
        {
            return Err(
                "Use distinct safe Markdown/text paths and files of at most 128 KiB without secrets, dynamic shell interpolation or control characters.",
            );
        }
        let segments = file.path.split('/').collect::<Vec<_>>();
        for depth in 1..=segments.len() {
            let prefix = segments[..depth].join("/");
            if spellings
                .insert(prefix.to_lowercase(), prefix.clone())
                .is_some_and(|previous| previous != prefix)
            {
                return Err(
                    "Use consistent spelling for shared path components on case-insensitive filesystems.",
                );
            }
        }
    }
    for path in &paths {
        if path.split('/').enumerate().skip(1).any(|(depth, _)| {
            paths.contains(&path.split('/').take(depth).collect::<Vec<_>>().join("/"))
        }) {
            return Err("A bundle path cannot be both a file and a directory.");
        }
    }
    let entrypoint = files
        .iter()
        .find(|f| f.path == kind.entrypoint())
        .ok_or("Include the exact root SKILL.md or AGENTS.md entrypoint for this bundle kind.")?;
    if kind == ManagedSkillKind::Skill {
        // Normalize line endings only for parsing; immutable delivered bytes stay intact.
        let normalized = entrypoint.content.replace("\r\n", "\n");
        let rest = normalized
            .strip_prefix("---\n")
            .ok_or("SKILL.md requires YAML frontmatter beginning with a --- line.")?;
        let (header, _) = rest
            .split_once("\n---\n")
            .or_else(|| rest.strip_suffix("\n---").map(|h| (h, "")))
            .ok_or("Close SKILL.md YAML frontmatter with a --- line.")?;
        let frontmatter: Frontmatter = yaml_serde::from_str(header)
            .map_err(|_| "Use valid YAML with only name, description, license, compatibility and string metadata fields.")?;
        if frontmatter.name != name || frontmatter.description != description {
            return Err(
                "SKILL.md frontmatter name and description must match the published metadata.",
            );
        }
        if frontmatter
            .license
            .as_deref()
            .is_some_and(|v| !text(v, 1024, true))
            || frontmatter
                .compatibility
                .as_deref()
                .is_some_and(|v| !text(v, 500, true))
            || frontmatter.metadata.len() > 32
            || frontmatter
                .metadata
                .iter()
                .any(|(k, v)| !text(k, 64, true) || !text(v, 1024, false))
        {
            return Err(
                "Keep license, compatibility and string metadata within the supported text bounds.",
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn files() -> Vec<ManagedSkillFile> {
        vec![ManagedSkillFile { path: "SKILL.md".into(), content: "---\nname: vault-guide\ndescription: >-\n  Read Vault documentation\n  with evidence.\nmetadata:\n  team: platform\n---\nUse references/notes.md.\n".into() }, ManagedSkillFile { path: "references/notes.md".into(), content: "Reference notes.".into() }]
    }
    fn validate(files: &[ManagedSkillFile]) -> bool {
        validate_managed_skill_bundle(
            ManagedSkillKind::Skill,
            "vault-guide",
            "Read Vault documentation with evidence.",
            files,
        )
        .is_ok()
    }
    #[test]
    fn block_scalar_and_progressive_text_bundle() {
        assert!(validate(&files()));
    }
    #[test]
    fn rejects_unsafe_paths_and_executable_resources() {
        for path in [
            "../x.md",
            "/x.md",
            "C:/x.md",
            "a//x.md",
            "a/./x.md",
            "a/../x.md",
            "a\\x.md",
            "run.sh",
            ".claude/settings.md",
            "a\0.md",
        ] {
            assert!(!validate_managed_skill_path(path), "{path:?}");
        }
        let mut f = files();
        f.push(ManagedSkillFile {
            path: "skill.md".into(),
            content: "collision".into(),
        });
        assert!(!validate(&f));
        let mut f = files();
        f.push(ManagedSkillFile {
            path: "SKILL.md/nested.txt".into(),
            content: "collision".into(),
        });
        assert!(!validate(&f));
    }
    #[test]
    fn rejects_host_execution_metadata_malformed_yaml_and_mismatch() {
        for field in [
            "allowed-tools: Bash\n",
            "context: fork\n",
            "agent: general-purpose\n",
            "hooks: {}\n",
            "name: another-name\n",
            "metadata: [bad]\n",
        ] {
            let mut f = files();
            f[0].content = f[0]
                .content
                .replacen("---\nUse", &format!("{field}---\nUse"), 1);
            assert!(!validate(&f), "{field}");
        }
        let mut f = files();
        f[1].content = "!`id`".into();
        assert!(!validate(&f));
        let mut f = files();
        f[0].content = "---\nname: [\n---\nbody".into();
        assert!(!validate(&f));
    }
    #[test]
    fn rejects_size_and_secret_values() {
        let mut f = files();
        f[1].content = "a".repeat(128 * 1024 + 1);
        assert!(!validate(&f));
        let mut f = files();
        f[1].content = "password: synthetic-sensitive-value".into();
        assert!(!validate(&f));
        let mut f = files();
        for i in 0..5 {
            f.push(ManagedSkillFile {
                path: format!("ref{i}.txt"),
                content: "x".repeat(128 * 1024),
            });
        }
        assert!(!validate(&f));
    }
    #[test]
    fn rejects_conflicting_directory_case_before_local_materialization() {
        let mut f = files();
        f.push(ManagedSkillFile {
            path: "References/other.txt".into(),
            content: "Directory case collision.".into(),
        });
        assert!(!validate(&f));
        f.last_mut().unwrap().path = "references/other.txt".into();
        assert!(validate(&f));
    }
    #[test]
    fn bounded_yaml_metadata_rejects_alias_amplification_and_duplicate_keys() {
        let mut f = files();
        let huge = "x".repeat(2048);
        let metadata = format!("metadata:\n  seed: &seed {huge}\n  repeated: *seed");
        f[0].content = f[0]
            .content
            .replace("metadata:\n  team: platform", &metadata);
        assert!(!validate(&f));
        f = files();
        f[0].content = f[0]
            .content
            .replace("  team: platform", "  team: platform\n  team: other");
        assert!(!validate(&f));
    }
}
