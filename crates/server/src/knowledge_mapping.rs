//! Native candidate boundary for automatic organization. Parsing proves source
//! coordinates and shape, not extractor quality, semantic truth or publication.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub mod storage;

pub const SOURCE_SCHEMA: &str = "source-mentions-1";
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Person,
    Technology,
    Service,
    Environment,
    Repository,
    Concept,
    Setting,
}

impl EntityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Technology => "technology",
            Self::Service => "service",
            Self::Environment => "environment",
            Self::Repository => "repository",
            Self::Concept => "concept",
            Self::Setting => "setting",
        }
    }

    pub fn normalized_label(&self, quote: &str) -> String {
        let label = quote.split_whitespace().collect::<Vec<_>>().join(" ");
        if matches!(self, Self::Technology | Self::Concept) {
            label.to_lowercase()
        } else {
            label
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MentionSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub quote: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MentionCandidate {
    pub key: String,
    pub kind: EntityKind,
    pub span: MentionSpan,
    pub confidence: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasCandidate {
    pub left: String,
    pub right: String,
    pub span: MentionSpan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationCandidate {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub span: MentionSpan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCandidates {
    pub schema: String,
    pub adapter_revision: String,
    pub source_version_id: Uuid,
    pub language: String,
    pub mentions: Vec<MentionCandidate>,
    #[serde(default)]
    pub aliases: Vec<AliasCandidate>,
    #[serde(default)]
    pub relations: Vec<RelationCandidate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateFailure {
    InvalidJson,
    InvalidSchema,
    ForeignInput,
    UnsupportedLanguage,
    InputLimit,
    InvalidKey,
    InvalidSpan,
    InvalidConfidence,
    InvalidEndpoint,
    IncompatibleAlias,
}

fn valid_span(source: &str, span: &MentionSpan, mention: bool) -> bool {
    span.byte_start < span.byte_end
        && !span.quote.trim().is_empty()
        && (!mention || span.quote.len() <= 256)
        && source.get(span.byte_start..span.byte_end) == Some(span.quote.as_str())
}

fn covered(span: &MentionSpan, endpoint: &MentionCandidate) -> bool {
    span.byte_start <= endpoint.span.byte_start && span.byte_end >= endpoint.span.byte_end
}

/// All offsets are UTF-8 bytes. A model cannot select a destination Brain or
/// manufacture a source span for a paraphrase. Return the whole validated batch
/// or a static failure; aliases still need semantic/scope acceptance downstream.
pub fn source_candidates(
    expected_version: Uuid,
    immutable_source: &str,
    output: &[u8],
) -> Result<SourceCandidates, CandidateFailure> {
    if immutable_source.len() > MAX_SOURCE_BYTES || output.len() > MAX_OUTPUT_BYTES {
        return Err(CandidateFailure::InputLimit);
    }
    let value =
        crate::strict_json::from_slice(output).map_err(|_| CandidateFailure::InvalidJson)?;
    let candidates: SourceCandidates =
        serde_json::from_value(value).map_err(|_| CandidateFailure::InvalidSchema)?;
    if candidates.schema != SOURCE_SCHEMA
        || candidates.adapter_revision.trim().is_empty()
        || candidates.adapter_revision.len() > 80
        || candidates.adapter_revision.chars().any(char::is_control)
    {
        return Err(CandidateFailure::InvalidSchema);
    }
    if candidates.source_version_id != expected_version {
        return Err(CandidateFailure::ForeignInput);
    }
    if candidates.language != "en" {
        return Err(CandidateFailure::UnsupportedLanguage);
    }
    if candidates.mentions.len() > 128
        || candidates.aliases.len() > 128
        || candidates.relations.len() > 256
    {
        return Err(CandidateFailure::InputLimit);
    }
    let mut endpoints = BTreeMap::new();
    let mut occurrences = BTreeSet::new();
    for mention in &candidates.mentions {
        if mention.key.is_empty()
            || mention.key.len() > 80
            || !mention
                .key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
            || endpoints.insert(mention.key.as_str(), mention).is_some()
        {
            return Err(CandidateFailure::InvalidKey);
        }
        if !valid_span(immutable_source, &mention.span, true)
            || !occurrences.insert((
                mention.span.byte_start,
                mention.span.byte_end,
                serde_json::to_string(&mention.kind)
                    .map_err(|_| CandidateFailure::InvalidSchema)?,
            ))
        {
            return Err(CandidateFailure::InvalidSpan);
        }
        if !mention.confidence.is_finite() || !(0.0..=1.0).contains(&mention.confidence) {
            return Err(CandidateFailure::InvalidConfidence);
        }
    }
    let pair = |a: &str,
                b: &str,
                span: &MentionSpan|
     -> Result<(&MentionCandidate, &MentionCandidate), CandidateFailure> {
        if a == b {
            return Err(CandidateFailure::InvalidEndpoint);
        }
        let left = *endpoints.get(a).ok_or(CandidateFailure::InvalidEndpoint)?;
        let right = *endpoints.get(b).ok_or(CandidateFailure::InvalidEndpoint)?;
        if !valid_span(immutable_source, span, false)
            || !covered(span, left)
            || !covered(span, right)
        {
            return Err(CandidateFailure::InvalidSpan);
        }
        Ok((left, right))
    };
    for alias in &candidates.aliases {
        let (left, right) = pair(&alias.left, &alias.right, &alias.span)?;
        if left.kind != right.kind {
            return Err(CandidateFailure::IncompatibleAlias);
        }
    }
    for relation in &candidates.relations {
        pair(&relation.from, &relation.to, &relation.span)?;
        if !matches!(
            relation.relation.as_str(),
            "uses" | "configured_by" | "depends_on" | "located_in" | "implements" | "related_to"
        ) {
            return Err(CandidateFailure::InvalidSchema);
        }
    }
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn envelope(id: Uuid) -> Value {
        json!({"schema":SOURCE_SCHEMA,"adapter_revision":"synthetic-control-1","source_version_id":id,"language":"en","mentions":[
            {"key":"vault","kind":"technology","span":{"byte_start":11,"byte_end":16,"quote":"Vault"},"confidence":0.99},
            {"key":"prod","kind":"environment","span":{"byte_start":22,"byte_end":26,"quote":"PROD"},"confidence":0.98}
        ],"relations":[{"from":"vault","to":"prod","relation":"located_in","span":{"byte_start":11,"byte_end":26,"quote":"Vault uses PROD"}}]})
    }
    const TEXT: &str = "🔐 café Vault uses PROD.";

    #[test]
    fn utf8_exact_source_and_endpoint_control() {
        let id = Uuid::new_v4();
        let output = envelope(id);
        let parsed = source_candidates(id, TEXT, &serde_json::to_vec(&output).unwrap()).unwrap();
        assert_eq!(parsed.mentions.len(), 2);
        assert_eq!(parsed.relations.len(), 1);
        assert_eq!(
            source_candidates(Uuid::new_v4(), TEXT, &serde_json::to_vec(&output).unwrap())
                .unwrap_err(),
            CandidateFailure::ForeignInput
        );
    }

    #[test]
    fn malformed_spans_missing_endpoints_and_unknown_destinations_reject_atomically() {
        let id = Uuid::new_v4();
        for (pointer, value, expected) in [
            (
                "/mentions/0/span/byte_start",
                json!(3),
                CandidateFailure::InvalidSpan,
            ),
            (
                "/mentions/0/span/byte_end",
                json!(14),
                CandidateFailure::InvalidSpan,
            ),
            (
                "/mentions/0/span/quote",
                json!("Redis"),
                CandidateFailure::InvalidSpan,
            ),
            (
                "/mentions/0/confidence",
                json!(1.1),
                CandidateFailure::InvalidConfidence,
            ),
            (
                "/mentions/1/key",
                json!("vault"),
                CandidateFailure::InvalidKey,
            ),
            (
                "/relations/0/to",
                json!("absent"),
                CandidateFailure::InvalidEndpoint,
            ),
            (
                "/relations/0/to",
                json!("vault"),
                CandidateFailure::InvalidEndpoint,
            ),
            (
                "/relations/0/span/byte_end",
                json!(15),
                CandidateFailure::InvalidSpan,
            ),
            (
                "/mentions/0/span/byte_start",
                json!(true),
                CandidateFailure::InvalidSchema,
            ),
            (
                "/mentions/0/span/byte_start",
                json!(-1),
                CandidateFailure::InvalidSchema,
            ),
            (
                "/language",
                json!("sv"),
                CandidateFailure::UnsupportedLanguage,
            ),
        ] {
            let mut output = envelope(id);
            *output.pointer_mut(pointer).unwrap() = value;
            assert_eq!(
                source_candidates(id, TEXT, &serde_json::to_vec(&output).unwrap()).unwrap_err(),
                expected,
                "{pointer}"
            );
        }
        let mut output = envelope(id);
        output["brain_id"] = json!(Uuid::new_v4());
        assert_eq!(
            source_candidates(id, TEXT, &serde_json::to_vec(&output).unwrap()).unwrap_err(),
            CandidateFailure::InvalidSchema
        );
        let raw = serde_json::to_string(&envelope(id)).unwrap();
        let duplicate = raw.replacen("{", "{\"language\":\"en\",", 1);
        assert_eq!(
            source_candidates(id, TEXT, duplicate.as_bytes()).unwrap_err(),
            CandidateFailure::InvalidJson
        );
    }

    #[test]
    fn incompatible_aliases_and_limits_are_not_accepted_by_high_confidence() {
        let id = Uuid::new_v4();
        let mut output = envelope(id);
        output["aliases"] = json!([{"left":"vault","right":"prod","span":{"byte_start":11,"byte_end":26,"quote":"Vault uses PROD"}}]);
        assert_eq!(
            source_candidates(id, TEXT, &serde_json::to_vec(&output).unwrap()).unwrap_err(),
            CandidateFailure::IncompatibleAlias
        );
        assert_eq!(
            source_candidates(id, &"x".repeat(MAX_SOURCE_BYTES + 1), b"{}").unwrap_err(),
            CandidateFailure::InputLimit
        );
        assert_eq!(
            source_candidates(id, TEXT, &vec![b' '; MAX_OUTPUT_BYTES + 1]).unwrap_err(),
            CandidateFailure::InputLimit
        );
    }
}
