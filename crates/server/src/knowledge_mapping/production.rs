//! Optional source mapping rides an already authorized learning extraction.
//! Production visibility requires a separately qualified canonical profile;
//! missing legacy outputs never trigger another provider request.
use super::{SOURCE_SCHEMA, SourceCandidates, organization, source_candidates, storage};
use crate::db::admission::{self, BrainAccess, Role};
use crate::{
    AppState,
    error::{Error, Result},
    graph, jobs,
    memory_evidence::Tx,
    model_gateway::SourceText,
    model_policy,
    worker::{ClaimedJob, Failure},
};
use chrono::{DateTime, Utc};
use recollect_protocol::LearningRun;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

mod overlapping;

pub const ADAPTER: &str = "learning-quote-anchors-3";

/// Select coordinates only from the actual successful request's exact pair.
/// Historical responses retain their original coordinate semantics.
pub fn mapping_adapter(prompt: &str, schema: &str) -> Option<&'static str> {
    if prompt != schema {
        return None;
    }
    match prompt {
        "source-facts-2" | "source-reconciliation-2" => Some("learning-quote-anchors-1"),
        "source-facts-3" | "source-reconciliation-3" => Some("learning-quote-anchors-2"),
        "source-facts-4" | "source-reconciliation-4" => Some(ADAPTER),
        _ => None,
    }
}

fn mapping_identity(prompt: &str, schema: &str, adapter: &str) -> bool {
    mapping_adapter(prompt, schema) == Some(adapter)
}

pub fn mapping_schema() -> Value {
    let span = json!({"type":"object","description":"An exact physical raw-source occurrence; pair evidence must enclose both selected mention occurrences.","properties":{"quote":{"type":"string","minLength":1,"maxLength":4096,"description":"Exact contiguous raw-source substring, preserving whitespace and Unicode; no provenance or line prefixes."},"line_from":{"type":"integer","minimum":1,"description":"First one-based physical raw-source line in the inclusive search window."},"line_to":{"type":"integer","minimum":1,"description":"Last inclusive physical raw-source line in the search window."},"occurrence":{"type":["integer","null"],"minimum":1,"description":"One-based ordinal of every exact substring match, including overlaps, from left to right in this window; null only if exactly one match exists."}},"required":["quote","line_from","line_to","occurrence"],"additionalProperties":false});
    let key =
        json!({"type":"string","minLength":1,"maxLength":80,"pattern":super::LOCAL_KEY_PATTERN});
    let mut mention_span = span.clone();
    mention_span["properties"]["quote"]["maxLength"] = json!(256);
    json!({"type":["object","null"],"properties":{
        "language":{"type":"string","enum":["en"]},
        "mentions":{"type":"array","maxItems":128,"items":{"type":"object","properties":{
            "key":key,"kind":{"type":"string","enum":["person","technology","service","environment","repository","concept","setting"]},
            "span":mention_span,"confidence":{"type":"number","minimum":0,"maximum":1}},"required":["key","kind","span","confidence"],"additionalProperties":false}},
        "aliases":{"type":"array","maxItems":128,"items":{"type":"object","properties":{"left":key,"right":key,"span":span},"required":["left","right","span"],"additionalProperties":false}},
        "relations":{"type":"array","maxItems":256,"items":{"type":"object","properties":{"from":key,"to":key,"relation":{"type":"string","enum":super::RELATION_KINDS},"span":span},"required":["from","to","relation","span"],"additionalProperties":false}}
    },"required":["language","mentions","aliases","relations"],"additionalProperties":false})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OfferedMapping {
    language: String,
    mentions: Vec<AnchoredMention>,
    #[serde(default)]
    aliases: Vec<AnchoredAlias>,
    #[serde(default)]
    relations: Vec<AnchoredRelation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Anchor {
    quote: String,
    line_from: usize,
    line_to: usize,
    #[serde(default)]
    occurrence: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnchoredMention {
    key: String,
    kind: super::EntityKind,
    span: Anchor,
    confidence: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnchoredAlias {
    left: String,
    right: String,
    span: Anchor,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnchoredRelation {
    from: String,
    to: String,
    relation: String,
    span: Anchor,
}

fn resolve_anchor(
    source: &str,
    lines: &[(usize, usize)],
    anchor: Anchor,
    adapter: &str,
) -> std::result::Result<super::MentionSpan, super::CandidateFailure> {
    use super::CandidateFailure::InvalidSpan;
    if anchor.quote.is_empty() || anchor.line_from == 0 || anchor.line_to < anchor.line_from {
        return Err(InvalidSpan);
    }
    let from = lines.get(anchor.line_from - 1).ok_or(InvalidSpan)?.0;
    let to = lines.get(anchor.line_to - 1).ok_or(InvalidSpan)?.1;
    if anchor.quote.len() > 4096 {
        return Err(InvalidSpan);
    }
    let window = &source[from..to];
    let relative = if adapter == ADAPTER {
        overlapping::resolve(window, &anchor.quote, anchor.occurrence).ok_or(InvalidSpan)?
    } else {
        // Adapter1/2 retain str::match_indices' original disjoint semantics.
        let mut matches = window.match_indices(&anchor.quote).map(|(start, _)| start);
        match anchor.occurrence {
            Some(n) if n > 0 => matches.nth(n - 1).ok_or(InvalidSpan)?,
            None => {
                let first = matches.next().ok_or(InvalidSpan)?;
                if matches.next().is_some() {
                    return Err(InvalidSpan);
                }
                first
            }
            _ => return Err(InvalidSpan),
        }
    };
    let start = from + relative;
    if !source.is_char_boundary(start) || !source.is_char_boundary(start + anchor.quote.len()) {
        return Err(InvalidSpan);
    }
    Ok(super::MentionSpan {
        byte_start: start,
        byte_end: start + anchor.quote.len(),
        quote: anchor.quote,
    })
}

/// Deterministic coordinate adapter. It resolves exact source-local anchors,
/// never fuzzy text, another offered input, or model-chosen destination IDs.
pub fn adapt(
    version: Uuid,
    source: &str,
    mapping: &Value,
) -> std::result::Result<SourceCandidates, super::CandidateFailure> {
    adapt_for_request(
        version,
        source,
        mapping,
        crate::model_gateway::EXTRACT_PROMPT,
        crate::model_gateway::EXTRACT_SCHEMA,
    )
}

/// Adapt an actual retained response without relabeling it as the current builder.
pub fn adapt_for_request(
    version: Uuid,
    source: &str,
    mapping: &Value,
    prompt: &str,
    schema: &str,
) -> std::result::Result<SourceCandidates, super::CandidateFailure> {
    use super::CandidateFailure::{InputLimit, InvalidSchema};
    let adapter = mapping_adapter(prompt, schema).ok_or(InvalidSchema)?;
    if source.len() > 2 * 1024 * 1024
        || serde_json::to_vec(mapping)
            .map_err(|_| InvalidSchema)?
            .len()
            > 256 * 1024
    {
        return Err(InputLimit);
    }
    let raw: OfferedMapping = serde_json::from_value(mapping.clone()).map_err(|_| InvalidSchema)?;
    if raw.mentions.len() > 128 || raw.aliases.len() > 128 || raw.relations.len() > 256 {
        return Err(InputLimit);
    }
    let mut offset = 0;
    let lines: Vec<_> = source
        .split_inclusive('\n')
        .map(|line| {
            let start = offset;
            offset += line.len();
            (start, offset)
        })
        .collect();
    let candidates = SourceCandidates {
        schema: SOURCE_SCHEMA.into(),
        adapter_revision: adapter.into(),
        source_version_id: version,
        language: raw.language,
        mentions: raw
            .mentions
            .into_iter()
            .map(|m| {
                Ok(super::MentionCandidate {
                    key: m.key,
                    kind: m.kind,
                    span: resolve_anchor(source, &lines, m.span, adapter)?,
                    confidence: m.confidence,
                })
            })
            .collect::<std::result::Result<_, super::CandidateFailure>>()?,
        aliases: raw
            .aliases
            .into_iter()
            .map(|m| {
                Ok(super::AliasCandidate {
                    left: m.left,
                    right: m.right,
                    span: resolve_anchor(source, &lines, m.span, adapter)?,
                })
            })
            .collect::<std::result::Result<_, super::CandidateFailure>>()?,
        relations: raw
            .relations
            .into_iter()
            .map(|m| {
                Ok(super::RelationCandidate {
                    from: m.from,
                    to: m.to,
                    relation: m.relation,
                    span: resolve_anchor(source, &lines, m.span, adapter)?,
                })
            })
            .collect::<std::result::Result<_, super::CandidateFailure>>()?,
    };
    source_candidates(
        version,
        source,
        &serde_json::to_vec(&candidates).map_err(|_| InvalidSchema)?,
    )
}

pub(crate) async fn missing_legacy(
    tx: &mut Tx<'_>,
    run: &LearningRun,
    request: Uuid,
) -> Result<()> {
    sqlx::query("INSERT INTO knowledge_mapping_coverage(run_id,brain_id,request_id,state) VALUES($1,$2,$3,'missing') ON CONFLICT(run_id) DO NOTHING")
        .bind(run.id).bind(run.brain_id).bind(request).execute(&mut **tx).await?;
    Ok(())
}
async fn coverage(
    tx: &mut Tx<'_>,
    run: &LearningRun,
    request: Uuid,
    state: &str,
    input: Option<Uuid>,
    failure: Option<&str>,
) -> Result<()> {
    sqlx::query("INSERT INTO knowledge_mapping_coverage(run_id,brain_id,request_id,state,input_id,failure) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(run_id) DO UPDATE SET state=excluded.state,input_id=excluded.input_id,failure=excluded.failure WHERE knowledge_mapping_coverage.request_id=excluded.request_id")
        .bind(run.id).bind(run.brain_id).bind(request).bind(state).bind(input).bind(failure).execute(&mut **tx).await?;
    Ok(())
}

pub(crate) enum PreparedMapping {
    Missing,
    /// Static reason: a `CandidateFailure::code()` or an operational cause.
    Invalid(&'static str),
    Validated {
        candidates: SourceCandidates,
        source: Uuid,
        context: Value,
        deadline: Option<DateTime<Utc>>,
    },
}

/// Freeze server-owned scope before the existing provider invocation. A later
/// group/context move cannot retroactively assign its output to a new realm.
pub(crate) async fn learning_context(
    state: &AppState,
    job: &ClaimedJob,
    run: &LearningRun,
) -> Result<Value> {
    let mut tx = admission::brain_preparation_tx(
        &state.pool,
        job.actor_id,
        job.device_id,
        run.brain_id,
        BrainAccess::READ.with_role(Role::Writer),
    )
    .await?
    .tx;
    let source: Uuid =
        sqlx::query_scalar("SELECT source_id FROM source_versions WHERE brain_id=$1 AND id=$2")
            .bind(run.brain_id)
            .bind(run.source_version_id)
            .fetch_one(&mut *tx)
            .await?;
    let context = storage::context(&mut tx, run.brain_id, run.source_version_id, source).await?;
    tx.commit().await?;
    Ok(context)
}

/// Read immutable input0 and resolve all quote anchors outside writer admission.
/// A separate fresh stage transaction decides whether this provisional batch
/// still has the exact current source/scope and publication provenance.
pub(crate) async fn prepare_learning(
    state: &AppState,
    job: &ClaimedJob,
    run: &LearningRun,
    request: &recollect_protocol::ModelRequest,
    limit: usize,
    mapping: Option<&Value>,
    expected_context: &Value,
) -> Result<PreparedMapping> {
    let Some(mapping) = mapping.filter(|v| !v.is_null()) else {
        return Ok(PreparedMapping::Missing);
    };
    if mapping_adapter(&request.prompt_label, &request.schema_label).is_none()
        || serde_json::to_vec(mapping).map_or(true, |bytes| bytes.len() > 256 * 1024)
    {
        return Ok(PreparedMapping::Invalid("adapter_or_size_rejected"));
    }
    let mut tx = admission::brain_preparation_tx(
        &state.pool,
        job.actor_id,
        job.device_id,
        run.brain_id,
        BrainAccess::READ.with_role(Role::Writer),
    )
    .await?
    .tx;
    sqlx::query("SET LOCAL statement_timeout='2s'")
        .execute(&mut *tx)
        .await?;
    let deadline = match storage::input_gate(&mut tx, run.brain_id, run.source_version_id).await {
        Ok(deadline) => deadline,
        Err(error) if error.1 == "content_removed" => {
            return Ok(PreparedMapping::Invalid("content_removed"));
        }
        Err(error) => return Err(error),
    };
    let source =
        crate::model_gateway::source(state, &mut tx, run.brain_id, run.source_version_id, limit)
            .await?;
    let context = storage::context(
        &mut tx,
        run.brain_id,
        run.source_version_id,
        source.source_id,
    )
    .await?;
    tx.commit().await?;
    if &context != expected_context {
        return Ok(PreparedMapping::Invalid("context_changed"));
    }
    Ok(
        match adapt_for_request(
            run.source_version_id,
            &source.text,
            mapping,
            &request.prompt_label,
            &request.schema_label,
        ) {
            Ok(candidates) => PreparedMapping::Validated {
                candidates,
                source: source.source_id,
                context,
                deadline,
            },
            Err(failure) => PreparedMapping::Invalid(failure.code()),
        },
    )
}

/// Called beside the normalized claim stage, after fresh learning authority and
/// exact immutable source checks. Models cannot choose profile identities.
pub(crate) async fn stage_learning(
    tx: &mut Tx<'_>,
    run: &LearningRun,
    request: &recollect_protocol::ModelRequest,
    source: &SourceText,
    prepared: PreparedMapping,
) -> Result<()> {
    let (candidates, prepared_source, prepared_context, deadline) = match prepared {
        PreparedMapping::Missing => {
            return coverage(tx, run, request.id, "missing", None, None).await;
        }
        PreparedMapping::Invalid(reason) => {
            return coverage(tx, run, request.id, "invalid", None, Some(reason)).await;
        }
        PreparedMapping::Validated {
            candidates,
            source,
            context,
            deadline,
        } => (candidates, source, context, deadline),
    };
    let context =
        storage::context(tx, run.brain_id, run.source_version_id, source.source_id).await?;
    if prepared_source != source.source_id
        || candidates.source_version_id != run.source_version_id
        || prepared_context != context
        || deadline.is_some_and(|at| at <= Utc::now())
    {
        return coverage(
            tx,
            run,
            request.id,
            "invalid",
            None,
            Some("context_changed"),
        )
        .await;
    }
    let current_deadline = match storage::input_gate(tx, run.brain_id, run.source_version_id).await
    {
        Ok(deadline) => deadline,
        Err(error) if error.1 == "content_removed" => {
            return coverage(
                tx,
                run,
                request.id,
                "invalid",
                None,
                Some("content_removed"),
            )
            .await;
        }
        Err(error) => return Err(error),
    };
    let deadline = [deadline, current_deadline].into_iter().flatten().min();
    let superseded: bool = sqlx::query_scalar("SELECT recollect_mapping_superseded($1,$2,$3,$4)")
        .bind(run.brain_id)
        .bind(request.id)
        .bind(run.source_version_id)
        .bind(sqlx::types::Json(&context))
        .fetch_one(&mut **tx)
        .await?;
    if superseded {
        coverage(tx, run, request.id, "superseded", None, None).await?;
        sqlx::query("DELETE FROM knowledge_mapping_stages WHERE brain_id=$1 AND run_id=$2")
            .bind(run.brain_id)
            .bind(run.id)
            .execute(&mut **tx)
            .await?;
        return Ok(());
    }
    if candidates.schema != SOURCE_SCHEMA
        || !mapping_identity(
            &request.prompt_label,
            &request.schema_label,
            &candidates.adapter_revision,
        )
    {
        return Err(Error::invalid(
            "Retained mapping adapter does not match its original request identity.",
        ));
    }
    let _: Option<Uuid> = sqlx::query_scalar("SELECT recollect_mapping_bootstrap($1)")
        .bind(run.brain_id)
        .fetch_one(&mut **tx)
        .await?;
    let profile: Option<Uuid> = sqlx::query_scalar("SELECT f.change_id FROM knowledge_mapping_producer_heads h JOIN knowledge_mapping_producer_profiles f ON f.change_id=h.change_id AND f.brain_id=h.brain_id WHERE h.brain_id=$1 AND f.quality_state='qualified' AND f.quality_receipt_sha256 IS NOT NULL AND f.provider=$2 AND f.requested_model=$3 AND f.returned_model=$4 AND f.prompt_label=$5 AND f.schema_label=$6 AND f.adapter_revision=$7 AND f.mapping_schema=$8")
        .bind(run.brain_id).bind(&request.provider).bind(&request.model).bind(&request.returned_model)
        .bind(&request.prompt_label).bind(&request.schema_label).bind(&candidates.adapter_revision).bind(&candidates.schema).fetch_optional(&mut **tx).await?;
    let Some(profile) = profile else {
        let retained =
            super::producer::retain(tx, run, request, &context, &candidates, deadline).await?;
        return coverage(
            tx,
            run,
            request.id,
            if retained {
                "blocked_profile"
            } else {
                "blocked_capacity"
            },
            None,
            None,
        )
        .await;
    };
    storage::input_gate(tx, run.brain_id, run.source_version_id).await?;
    let epoch = graph::preparation_epoch(tx, run.brain_id).await?;
    let input = storage::stage_validated(
        tx,
        run.brain_id,
        run.actor_id,
        source.source_id,
        epoch,
        &context,
        &candidates,
        Some((request.id, run.policy_id, profile)),
    )
    .await?;
    sqlx::query("INSERT INTO knowledge_mapping_origins(input_id,brain_id,run_id,request_id,policy_id,producer_change_id,offered_inputs) SELECT $1,$2,$3,$4,$5,$6,coalesce(jsonb_agg(jsonb_build_object('kind',kind,'id',input_id) ORDER BY kind,input_id),'[]'::jsonb) FROM model_request_inputs WHERE brain_id=$2 AND request_id=$4 ON CONFLICT(input_id) DO NOTHING")
        .bind(input).bind(run.brain_id).bind(run.id).bind(request.id).bind(run.policy_id).bind(profile).execute(&mut **tx).await?;
    coverage(tx, run, request.id, "staged", Some(input), None).await
}

/// Queue exactly this retained source stage in the successful learning commit,
/// including a learning outcome with no accepted claim candidates.
pub(crate) async fn enqueue_learning(
    tx: &mut Tx<'_>,
    job: &ClaimedJob,
    run: &LearningRun,
    audit: Uuid,
) -> Result<()> {
    let input: Option<Uuid> = sqlx::query_scalar("SELECT input_id FROM knowledge_mapping_coverage WHERE brain_id=$1 AND run_id=$2 AND state='staged'")
        .bind(run.brain_id).bind(run.id).fetch_optional(&mut **tx).await?.flatten();
    let Some(input) = input else {
        return Ok(());
    };
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE brain_id=$1 AND target_id=$2 AND kind='knowledge.map' AND state IN ('queued','running','succeeded'))")
        .bind(job.brain_id).bind(input).fetch_one(&mut **tx).await?;
    if !exists {
        let id = jobs::enqueue_work(
            tx,
            job.actor_id,
            job.brain_id,
            audit,
            input,
            "knowledge.map",
            "heavy",
        )
        .await?;
        sqlx::query("UPDATE jobs SET device_id=$2 WHERE id=$1")
            .bind(id)
            .bind(job.device_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn lease(tx: &mut Tx<'_>, job: &ClaimedJob) -> Result<()> {
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE brain_id=$1 AND id=$2 AND kind='knowledge.map' AND target_id=$3 AND actor_id=$4 AND device_id IS NOT DISTINCT FROM $5 AND state='running' AND lease_token=$6 AND lease_until>clock_timestamp())")
        .bind(job.brain_id).bind(job.id).bind(job.target_id).bind(job.actor_id).bind(job.device_id).bind(job.lease_token).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(model_policy::failure(
            "mapping_lease_lost",
            "This mapping worker no longer owns publication.",
        ));
    }
    Ok(())
}
async fn eligible(
    state: &AppState,
    tx: &mut Tx<'_>,
    job: &ClaimedJob,
) -> Result<(Uuid, Uuid, Option<DateTime<Utc>>)> {
    lease(tx, job).await?;
    let superseded:bool=sqlx::query_scalar("SELECT coalesce((SELECT recollect_mapping_superseded(i.brain_id,i.producer_request_id,i.source_version_id,c.context) FROM knowledge_mapping_inputs i JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id WHERE i.brain_id=$1 AND i.id=$2),false)")
        .bind(job.brain_id).bind(job.target_id).fetch_one(&mut **tx).await?;
    if superseded {
        return Err(model_policy::failure(
            "mapping_superseded",
            "A newer interpretation of this exact source and context is already published.",
        ));
    }
    let row: Option<(Uuid,Uuid,Option<DateTime<Utc>>)> = sqlx::query_as("SELECT v.source_id,v.id,CASE WHEN o.published_at IS NULL THEN recollect_retention_deadline(o.brain_id,'audit',q.created_at) END FROM knowledge_mapping_inputs i JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.source_version_id JOIN knowledge_mapping_origins o ON o.brain_id=i.brain_id AND o.input_id=i.id JOIN learning_runs r ON r.id=o.run_id AND r.brain_id=o.brain_id JOIN model_requests q ON q.brain_id=o.brain_id AND q.id=o.request_id WHERE i.brain_id=$1 AND i.id=$2 AND r.actor_id=$3 AND r.device_id IS NOT DISTINCT FROM $4 AND recollect_mapping_origin_current(i.brain_id,i.id)")
        .bind(job.brain_id).bind(job.target_id).bind(job.actor_id).bind(job.device_id).fetch_optional(&mut **tx).await?;
    let (source, version, audit_deadline) = row.ok_or_else(crate::retention::unavailable)?;
    let policy = model_policy::current(state, tx, job.brain_id).await?;
    if !model_policy::matches_installation(state, &policy.policy) {
        return Err(model_policy::denied());
    }
    let deadline = storage::input_gate(tx, job.brain_id, version).await?;
    Ok((
        source,
        version,
        [deadline, audit_deadline].into_iter().flatten().min(),
    ))
}

async fn publish(state: &AppState, job: &ClaimedJob) -> Result<()> {
    let mut tx = admission::brain_preparation_tx(
        &state.pool,
        job.actor_id,
        job.device_id,
        job.brain_id,
        BrainAccess::READ.with_role(Role::Writer),
    )
    .await?
    .tx;
    sqlx::query("SET LOCAL statement_timeout='2s'")
        .execute(&mut *tx)
        .await?;
    let (source, version, deadline) = eligible(state, &mut tx, job).await?;
    let epoch = graph::preparation_epoch(&mut tx, job.brain_id).await?;
    let context = storage::context(&mut tx, job.brain_id, version, source).await?;
    tx.commit().await?;
    let mut tx = admission::brain_tx_as(
        &state.pool,
        job.actor_id,
        job.device_id,
        job.brain_id,
        BrainAccess::WRITE
            .with_statement_timeout(std::time::Duration::from_secs(1))
            .with_lock_timeout(std::time::Duration::from_millis(250)),
    )
    .await?
    .tx;
    // Lock the exact lease before any derivative is moved or published.
    sqlx::query("SELECT id FROM jobs WHERE id=$1 AND lease_token=$2 FOR UPDATE")
        .bind(job.id)
        .bind(job.lease_token)
        .fetch_optional(&mut *tx)
        .await?;
    graph::preparation_unchanged(&mut tx, job.brain_id, epoch).await?;
    let (current_source, current_version, _) = eligible(state, &mut tx, job).await?;
    if source != current_source
        || version != current_version
        || context != storage::context(&mut tx, job.brain_id, version, source).await?
        || deadline.is_some_and(|at| at <= Utc::now())
    {
        return Err(crate::retention::unavailable());
    }
    organization::publish_input(&mut tx, job.brain_id, job.actor_id, job.target_id).await?;
    replace_same_input(&mut tx, job).await?;
    sqlx::query(
        "UPDATE knowledge_mapping_coverage SET state='published' WHERE brain_id=$1 AND input_id=$2",
    )
    .bind(job.brain_id)
    .bind(job.target_id)
    .execute(&mut *tx)
    .await?;
    eligible(state, &mut tx, job).await?;
    let retained: bool =
        sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
            .bind(deadline)
            .fetch_one(&mut *tx)
            .await?;
    if !retained {
        return Err(crate::retention::unavailable());
    }
    let done = sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE id=$1 AND state='running' AND lease_token=$2 AND lease_until>clock_timestamp()")
        .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
    if done.rows_affected() != 1 {
        return Err(model_policy::failure(
            "mapping_lease_lost",
            "This mapping worker lost publication.",
        ));
    }
    tx.commit().await?;
    Ok(())
}

/// Human intent follows a rebuild of the same immutable source/context only.
/// Source correction, scope change and erasure never transfer these overrides.
async fn replace_same_input(tx: &mut Tx<'_>, job: &ClaimedJob) -> Result<()> {
    let old: Vec<Uuid> = sqlx::query_scalar("SELECT old.id FROM knowledge_mapping_inputs old JOIN knowledge_mapping_inputs new ON new.brain_id=old.brain_id AND new.source_version_id=old.source_version_id AND new.context_id=old.context_id WHERE new.brain_id=$1 AND new.id=$2 AND old.id<>new.id AND old.producer_request_id IS NOT NULL AND old.state='published'")
        .bind(job.brain_id).bind(job.target_id).fetch_all(&mut **tx).await?;
    if old.is_empty() {
        return Ok(());
    }
    super::manual_intent::copy_before_replacement(tx, job.brain_id, &old).await?;
    sqlx::query("UPDATE knowledge_topic_labels SET basis_input=$2 WHERE brain_id=$1 AND basis_input=ANY($3)")
        .bind(job.brain_id).bind(job.target_id).bind(&old).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_exclusions(brain_id,topic_id,input_id,actor_id) SELECT brain_id,topic_id,$2,actor_id FROM knowledge_topic_exclusions WHERE brain_id=$1 AND input_id=ANY($3) ON CONFLICT DO NOTHING")
        .bind(job.brain_id).bind(job.target_id).bind(&old).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_splits(brain_id,original_topic_id,split_topic_id,input_id,actor_id) SELECT brain_id,original_topic_id,split_topic_id,$2,actor_id FROM knowledge_topic_splits WHERE brain_id=$1 AND input_id=ANY($3) ON CONFLICT DO NOTHING")
        .bind(job.brain_id).bind(job.target_id).bind(&old).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) SELECT brain_id,topic_id,entity_id,$2,reason,weight FROM knowledge_topic_memberships old WHERE brain_id=$1 AND input_id=ANY($3) AND reason='explicit_split' AND EXISTS(SELECT 1 FROM knowledge_mentions current WHERE current.brain_id=old.brain_id AND current.input_id=$2 AND current.entity_id=old.entity_id) ON CONFLICT DO NOTHING")
        .bind(job.brain_id).bind(job.target_id).bind(&old).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM knowledge_mapping_inputs WHERE brain_id=$1 AND id=ANY($2)")
        .bind(job.brain_id)
        .bind(&old)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn execute(state: &AppState, job: &ClaimedJob) -> std::result::Result<(), Failure> {
    match tokio::time::timeout(std::time::Duration::from_secs(5), publish(state, job)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) if error.1 == "mapping_superseded" => {
            match tokio::time::timeout(
                std::time::Duration::from_millis(1500),
                discard_superseded(state, job),
            )
            .await
            {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) if error.1 == "mapping_lease_lost" => Err(Failure::LostLease),
                _ => Err(Failure::Database),
            }
        }
        Ok(Err(error)) if error.1 == "mapping_lease_lost" => Err(Failure::LostLease),
        Ok(Err(error))
            if matches!(
                error.0,
                axum::http::StatusCode::FORBIDDEN | axum::http::StatusCode::UNAUTHORIZED
            ) =>
        {
            Err(Failure::Revoked)
        }
        Ok(Err(error)) if error.1 == "content_removed" => Err(Failure::Missing),
        _ => Err(Failure::Database),
    }
}

async fn discard_superseded(state: &AppState, job: &ClaimedJob) -> Result<()> {
    let mut tx = admission::brain_tx_as(
        &state.pool,
        job.actor_id,
        job.device_id,
        job.brain_id,
        BrainAccess::WRITE
            .with_statement_timeout(std::time::Duration::from_millis(750))
            .with_lock_timeout(std::time::Duration::from_millis(250)),
    )
    .await?
    .tx;
    lease(&mut tx, job).await?;
    let still_superseded:bool=sqlx::query_scalar("SELECT coalesce((SELECT recollect_mapping_superseded(i.brain_id,i.producer_request_id,i.source_version_id,c.context) FROM knowledge_mapping_inputs i JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id WHERE i.brain_id=$1 AND i.id=$2),false)")
        .bind(job.brain_id).bind(job.target_id).fetch_one(&mut *tx).await?;
    if !still_superseded {
        return Err(model_policy::failure(
            "mapping_input_changed",
            "The interpretation winner changed; retry publication.",
        ));
    }
    let entities: Vec<Uuid> = sqlx::query_scalar(
        "SELECT entity_id FROM knowledge_mentions WHERE brain_id=$1 AND input_id=$2",
    )
    .bind(job.brain_id)
    .bind(job.target_id)
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query("UPDATE knowledge_mapping_coverage SET state='superseded',input_id=NULL WHERE brain_id=$1 AND input_id=$2").bind(job.brain_id).bind(job.target_id).execute(&mut *tx).await?;
    sqlx::query(
        "DELETE FROM knowledge_mapping_inputs WHERE brain_id=$1 AND id=$2 AND state='staged'",
    )
    .bind(job.brain_id)
    .bind(job.target_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM knowledge_entities e WHERE e.brain_id=$1 AND e.id=ANY($2) AND NOT EXISTS(SELECT 1 FROM knowledge_mentions m WHERE m.brain_id=e.brain_id AND m.entity_id=e.id)").bind(job.brain_id).bind(&entities).execute(&mut *tx).await?;
    let changed=sqlx::query("UPDATE jobs SET state='cancelled',error_code='mapping_superseded',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE id=$1 AND state='running' AND lease_token=$2 AND lease_until>clock_timestamp()")
        .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
    if changed.rows_affected() != 1 {
        return Err(model_policy::failure(
            "mapping_lease_lost",
            "This mapping worker lost publication.",
        ));
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn one(quote: &str, occurrence: Value) -> Value {
        json!({"language":"en","mentions":[{"key":"m","kind":"technology","span":{"quote":quote,"line_from":1,"line_to":1,"occurrence":occurrence},"confidence":1.0}],"aliases":[],"relations":[]})
    }
    #[test]
    fn exact_unicode_anchors_resolve_server_coordinates_and_reject_ambiguity() {
        let id = Uuid::new_v4();
        let source = "é Rust then Rust\nPostgreSQL\n";
        let first = adapt(id, source, &one("Rust", json!(1))).unwrap();
        let second = adapt(id, source, &one("Rust", json!(2))).unwrap();
        assert_eq!(
            first.mentions[0].span.byte_start,
            source.find("Rust").unwrap()
        );
        assert_eq!(
            second.mentions[0].span.byte_start,
            source.rfind("Rust").unwrap()
        );
        assert!(adapt(id, source, &one("Rust", Value::Null)).is_err());
        assert!(adapt(id, source, &one("Rust", json!(3))).is_err());
        assert!(adapt(id, source, &one("PostgreSQL", Value::Null)).is_err());
        assert!(adapt(id, source, &one("Other input", Value::Null)).is_err());
        let mut foreign = one("Rust", json!(1));
        foreign["source_version_id"] = json!(Uuid::new_v4());
        assert!(adapt(id, source, &foreign).is_err());
        let empty = json!({"language":"en","mentions":[],"aliases":[],"relations":[]});
        assert!(adapt(id, source, &empty).unwrap().mentions.is_empty());
    }
    #[test]
    fn relation_quote_must_cover_exact_anchored_endpoints() {
        let id = Uuid::new_v4();
        let source = "Rust uses PostgreSQL.\n";
        let mut value = json!({"language":"en","mentions":[
            {"key":"a","kind":"technology","span":{"quote":"Rust","line_from":1,"line_to":1,"occurrence":null},"confidence":1.0},
            {"key":"b","kind":"technology","span":{"quote":"PostgreSQL","line_from":1,"line_to":1,"occurrence":null},"confidence":1.0}],"aliases":[],
            "relations":[{"from":"a","to":"b","relation":"uses","span":{"quote":"Rust uses PostgreSQL.","line_from":1,"line_to":1,"occurrence":null}}]});
        assert_eq!(adapt(id, source, &value).unwrap().relations.len(), 1);
        value["relations"][0]["span"]["quote"] = json!("uses");
        assert!(adapt(id, source, &value).is_err());
    }
    #[test]
    fn provider_contract_and_native_vocabulary_agree_without_semantic_coercion() {
        let schema = mapping_schema();
        assert_eq!(
            schema["properties"]["relations"]["items"]["properties"]["relation"]["enum"],
            json!(super::super::RELATION_KINDS)
        );
        assert_eq!(
            schema["properties"]["mentions"]["items"]["properties"]["key"]["pattern"],
            super::super::LOCAL_KEY_PATTERN
        );
        assert_eq!(
            schema["properties"]["mentions"]["items"]["properties"]["span"]["properties"]["quote"]
                ["maxLength"],
            256
        );
        assert_eq!(
            schema["properties"]["relations"]["items"]["properties"]["span"]["properties"]["quote"]
                ["maxLength"],
            4096
        );
        for relation in super::super::RELATION_KINDS {
            let source = if *relation == "does_not_use" {
                "Orion does not use Redis."
            } else if *relation == "has_owner" {
                "Orion.owner = Zoë"
            } else {
                "Orion uses Redis."
            };
            let right = if *relation == "has_owner" {
                "Zoë"
            } else {
                "Redis"
            };
            let mut value = json!({"language":"en","mentions":[
                {"key":"m1","kind":"service","confidence":1.0,"span":{"quote":"Orion","line_from":1,"line_to":1,"occurrence":null}},
                {"key":"m2","kind":"technology","confidence":1.0,"span":{"quote":right,"line_from":1,"line_to":1,"occurrence":null}}],"aliases":[],
                "relations":[{"from":"m1","to":"m2","relation":relation,"span":{"quote":source,"line_from":1,"line_to":1,"occurrence":null}}]});
            let parsed = adapt(Uuid::new_v4(), source, &value).unwrap();
            assert_eq!(parsed.relations[0].relation, *relation);
            let mut legacy = serde_json::to_value(&parsed).unwrap();
            legacy["adapter_revision"] = json!("learning-quote-anchors-1");
            let old = source_candidates(
                parsed.source_version_id,
                source,
                &serde_json::to_vec(&legacy).unwrap(),
            );
            if ["does_not_use", "has_owner"].contains(relation) {
                assert_eq!(
                    old.unwrap_err(),
                    super::super::CandidateFailure::InvalidSchema
                );
            } else {
                assert_eq!(old.unwrap().adapter_revision, "learning-quote-anchors-1");
            }
            value["relations"][0]["relation"] = json!("invented_relation");
            assert_eq!(
                adapt(Uuid::new_v4(), source, &value).unwrap_err(),
                super::super::CandidateFailure::InvalidSchema
            );
        }
    }
    #[test]
    fn revision4_overlaps_preserve_historical_coordinate_semantics() {
        for (source, quote, second) in [("ababa", "aba", 2), ("ééé", "éé", 2)] {
            let id = Uuid::new_v4();
            assert!(adapt(id, source, &one(quote, Value::Null)).is_err());
            assert_eq!(
                adapt(id, source, &one(quote, json!(2))).unwrap().mentions[0]
                    .span
                    .byte_start,
                second
            );
            for n in [0, 3, usize::MAX] {
                assert!(adapt(id, source, &one(quote, json!(n))).is_err());
            }
            for prompt in [
                "source-facts-2",
                "source-reconciliation-2",
                "source-facts-3",
                "source-reconciliation-3",
            ] {
                let legacy =
                    adapt_for_request(id, source, &one(quote, Value::Null), prompt, prompt)
                        .unwrap();
                assert_eq!(
                    legacy.adapter_revision,
                    mapping_adapter(prompt, prompt).unwrap()
                );
                assert_eq!(legacy.mentions[0].span.byte_start, 0);
                assert!(
                    adapt_for_request(id, source, &one(quote, json!(2)), prompt, prompt).is_err()
                );
            }
        }
    }

    #[test]
    fn revision4_physical_window_and_endpoint_occurrences_are_exact() {
        let source = "Elm uses Ink.\r\nElm uses Ink.\r\n";
        let id = Uuid::new_v4();
        let mut value = json!({"language":"en","mentions":[
            {"key":"a","kind":"service","span":{"quote":"Elm","line_from":1,"line_to":2,"occurrence":2},"confidence":1.0},
            {"key":"b","kind":"technology","span":{"quote":"Ink","line_from":2,"line_to":2,"occurrence":null},"confidence":1.0}],"aliases":[],
            "relations":[{"from":"a","to":"b","relation":"uses","span":{"quote":"Elm uses Ink.","line_from":1,"line_to":2,"occurrence":2}}]});
        let parsed = adapt(id, source, &value).unwrap();
        assert_eq!(parsed.mentions[0].span.byte_start, 15);
        assert_eq!(parsed.relations[0].span.byte_start, 15);
        value["relations"][0]["span"]["occurrence"] = json!(1);
        assert!(adapt(id, source, &value).is_err());
        value["relations"][0]["span"]["line_from"] = json!(2);
        value["relations"][0]["span"]["occurrence"] = Value::Null;
        assert!(adapt(id, source, &value).is_ok());
        value["mentions"][0]["span"]["line_to"] = json!(1);
        assert!(adapt(id, source, &value).is_err());
    }

    #[test]
    fn revision4_exact_tuple_truth_table_rejects_every_crossed_or_unknown_pair() {
        let labels = [
            "source-facts-2",
            "source-reconciliation-2",
            "source-facts-3",
            "source-reconciliation-3",
            "source-facts-4",
            "source-reconciliation-4",
            "unknown",
        ];
        let adapters = [
            "learning-quote-anchors-1",
            "learning-quote-anchors-2",
            "learning-quote-anchors-3",
            "unknown",
        ];
        for (index, prompt) in labels.iter().enumerate() {
            for schema in labels {
                for adapter in adapters {
                    let expected = prompt == &schema && index < 6 && adapters[index / 2] == adapter;
                    assert_eq!(
                        mapping_identity(prompt, schema, adapter),
                        expected,
                        "{prompt}/{schema}/{adapter}"
                    );
                }
                let parsed = adapt_for_request(
                    Uuid::new_v4(),
                    "ababa",
                    &one("aba", json!(1)),
                    prompt,
                    schema,
                );
                assert_eq!(parsed.is_ok(), prompt == &schema && index < 6);
            }
        }
    }

    #[test]
    fn current_and_legacy_adapter_request_pairs_cannot_be_reinterpreted() {
        for prompt in ["source-facts-2", "source-reconciliation-2"] {
            assert!(mapping_identity(prompt, prompt, "learning-quote-anchors-1"));
            assert!(!mapping_identity(prompt, prompt, ADAPTER));
        }
        for prompt in [
            crate::model_gateway::EXTRACT_PROMPT,
            crate::autonomous::PROMPT,
        ] {
            assert!(mapping_identity(prompt, prompt, ADAPTER));
            assert!(!mapping_identity(
                prompt,
                prompt,
                "learning-quote-anchors-1"
            ));
            assert!(!mapping_identity(prompt, "other_schema", ADAPTER));
        }
        let parsed = adapt(Uuid::new_v4(), "Rust", &one("Rust", Value::Null)).unwrap();
        let mut legacy = serde_json::to_value(&parsed).unwrap();
        legacy["adapter_revision"] = json!("learning-quote-anchors-1");
        let decoded = source_candidates(
            parsed.source_version_id,
            "Rust",
            &serde_json::to_vec(&legacy).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded.adapter_revision, "learning-quote-anchors-1");
        assert_eq!(decoded.schema, "source-mentions-1");
    }
}
