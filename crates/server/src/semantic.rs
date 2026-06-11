//! Canonical semantic projections. Representation IDs identify inputs and rebuilds;
//! vectors never own scope, eligibility, evidence or deletion authority.
use crate::{
    AppState,
    error::{Error, Result},
    memory, memory_evidence,
    memory_evidence::Tx,
    model_gateway::{self as gateway, InputRef},
    model_policy, publication,
};
use recollect_protocol::SemanticProfile;
use serde_json::json;
use sqlx::types::Json;
use uuid::Uuid;

pub const REPRESENTATION: &str = "engineering-text-1";
pub const DIMENSIONS: i32 = 3072;
pub const INPUT_BYTES: usize = 6000;

mod queue;
mod work;
pub use queue::{
    __path_get, __path_reindex, __path_retry, get, maintain_brain, reindex, retry, run_once,
};
pub use work::execute;

#[derive(sqlx::FromRow)]
pub(crate) struct Entry {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub kind: String,
    pub input_id: Uuid,
    pub source_version_id: Option<Uuid>,
    pub state: String,
}

pub(crate) struct Representation {
    pub text: String,
    pub class: String,
    pub dependencies: Vec<InputRef>,
    pub truncated: bool,
}

pub(crate) async fn profile(tx: &mut Tx<'_>, brain: Uuid) -> Result<Option<SemanticProfile>> {
    let profile: Option<Json<SemanticProfile>> = sqlx::query_scalar(
        "SELECT to_jsonb(p) FROM semantic_profiles p JOIN semantic_heads h
         ON h.profile_id=p.id AND h.brain_id=p.brain_id WHERE p.brain_id=$1",
    )
    .bind(brain)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(profile.map(|p| p.0))
}

pub(crate) fn compatible(state: &AppState, profile: &SemanticProfile) -> bool {
    profile.provider == "openai"
        && profile.model == state.config.models.embedding_model
        && profile.dimensions == DIMENSIONS
        && profile.dimensions == state.config.models.embedding_dimensions
        && profile.representation == REPRESENTATION
}

pub(crate) async fn entry(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<Entry> {
    sqlx::query_as(
        "SELECT id,profile_id,kind,input_id,source_version_id,state FROM semantic_entries WHERE brain_id=$1 AND id=$2",
    )
    .bind(brain)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(Error::missing)
}

pub(crate) async fn representation(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    id: Uuid,
) -> Result<Representation> {
    let entry = entry(tx, brain, id).await?;
    if entry.state == "removed" {
        return Err(crate::retention::unavailable());
    }
    let profile = profile(tx, brain).await?.ok_or_else(Error::missing)?;
    if profile.id != entry.profile_id || !compatible(state, &profile) {
        return Err(model_policy::failure(
            "semantic_profile_mismatch",
            "The semantic profile changed or is incompatible. Reindex using the approved model.",
        ));
    }
    let mut dependencies = vec![InputRef {
        kind: "semantic_entry".into(),
        id: entry.id,
    }];
    let (mut text, class) = match entry.kind.as_str() {
        "source_chunk" => {
            let version = entry.source_version_id.ok_or_else(Error::missing)?;
            let source =
                gateway::source(state, tx, brain, version, crate::artifacts::MAX_TEXT).await?;
            let (start, end, indexed): (i32, i32, String) = sqlx::query_as(
                "SELECT byte_start,byte_end,content FROM source_chunks WHERE brain_id=$1 AND id=$2 AND version_id=$3",
            )
            .bind(brain)
            .bind(entry.input_id)
            .bind(version)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(Error::missing)?;
            let retained = usize::try_from(start)
                .ok()
                .zip(usize::try_from(end).ok())
                .and_then(|(start, end)| source.text.get(start..end))
                .filter(|text| !text.is_empty() && text.len() <= 4096 && *text == indexed);
            let retained = retained.ok_or_else(|| {
                model_policy::failure(
                    "semantic_source_span_unavailable",
                    "The indexed span does not match its retained source. Reprocess the source.",
                )
            })?;
            dependencies.push(InputRef {
                kind: "source_version".into(),
                id: version,
            });
            (
                format!(
                    "Title: {}\nRole: {}\nText:\n{}",
                    source.provenance["title"].as_str().unwrap_or(""),
                    source.provenance["role"]
                        .as_str()
                        .unwrap_or("source_document"),
                    retained
                ),
                source.class,
            )
        }
        "claim_revision" => {
            let current: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM claims WHERE brain_id=$1 AND current_revision=$2)
                   AND NOT EXISTS(SELECT 1 FROM model_claim_fences WHERE brain_id=$1 AND revision_id=$2)",
            )
            .bind(brain)
            .bind(entry.input_id)
            .fetch_one(&mut **tx)
            .await?;
            if !current {
                return Err(model_policy::denied());
            }
            let revision = crate::procedures::exact(tx, brain, entry.input_id)
                .await?
                .ok_or_else(crate::retention::unavailable)?;
            let view = memory::view(state, tx, revision, chrono::Utc::now(), None).await?;
            if !view.eligibility.investigation
                || !view.eligibility.rule_ids.is_empty()
                || view.contributions.iter().any(|c| c.revision.is_none())
            {
                return Err(model_policy::denied());
            }
            let c = view.revision.content;
            dependencies.push(InputRef {
                kind: "claim_revision".into(),
                id: entry.input_id,
            });
            (
                format!(
                    "Kind: {}\nSubject: {}\nProperty: {}\nValue: {}\nRationale: {}\nOperational: {}\nProcedure: {}\nHandover: {}",
                    c.kind,
                    c.subject,
                    c.predicate,
                    c.value,
                    c.rationale,
                    c.operational,
                    c.procedure
                        .map(|p| json!(p).to_string())
                        .unwrap_or_default(),
                    c.handover.map(|h| json!(h).to_string()).unwrap_or_default()
                ),
                "claim".into(),
            )
        }
        "repository_fact" | "manifest_revision" => {
            let row = memory_evidence::row(tx, brain, &entry.kind, entry.input_id).await?;
            if row.privacy() != "active" {
                return Err(crate::retention::unavailable());
            }
            dependencies.push(InputRef {
                kind: entry.kind.clone(),
                id: entry.input_id,
            });
            (
                format!("Kind: {}\nData: {}", entry.kind, row.data),
                "repository".into(),
            )
        }
        _ => return Err(Error::invalid("Unsupported semantic input.")),
    };
    // Sanitize/check the entire candidate before limiting the transmitted text.
    publication::safe_payload(state, &json!(&text))?;
    let truncated = text.len() > INPUT_BYTES;
    if truncated {
        let mut end = INPUT_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    Ok(Representation {
        text,
        class,
        dependencies,
        truncated,
    })
}
