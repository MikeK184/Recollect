//! Authenticated candidate staging, not semantic publication. Pending aliases
//! never equate identities and these tables are not an unchecked read API.
use super::{EntityKind, SourceCandidates, source_candidates};
use crate::{
    AppState,
    auth::Auth,
    db,
    error::{Error, Result},
    graph,
    memory_evidence::Tx,
    model_gateway, retention,
};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::types::Json;
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct PreparedSource {
    brain: Uuid,
    actor: Uuid,
    source_id: Uuid,
    epoch: i64,
    deadline: Option<DateTime<Utc>>,
    context: Value,
    candidates: SourceCandidates,
}

fn changed() -> Error {
    Error(
        StatusCode::CONFLICT,
        "mapping_input_changed",
        "Mapping inputs changed. Prepare the exact current source again.",
    )
}

pub(super) async fn input_gate(
    tx: &mut Tx<'_>,
    brain: Uuid,
    version: Uuid,
) -> Result<Option<DateTime<Utc>>> {
    let deadline: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
        "SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at)
         FROM recollect_source_knowledge v JOIN sources s ON s.brain_id=v.brain_id AND s.current_version=v.id
         WHERE v.brain_id=$1 AND v.id=$2 AND v.processing='ready'
          AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
          AND NOT recollect_source_model_fenced(v.brain_id,v.id)",
    ).bind(brain).bind(version).fetch_optional(&mut **tx).await?;
    deadline.map(|d| d.0).ok_or_else(retention::unavailable)
}

pub(super) async fn context(
    tx: &mut Tx<'_>,
    brain: Uuid,
    version: Uuid,
    source: Uuid,
) -> Result<Value> {
    let Json(context): Json<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object(
          'selection',coalesce((SELECT selection FROM source_import_scopes WHERE brain_id=$1 AND version_id=$2),'{}'::jsonb),
          'areas',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=$1 AND m.source_id=$3 AND g.kind='area' ORDER BY g.id),
          'environments',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=$1 AND m.source_id=$3 AND g.kind='environment' ORDER BY g.id))",
    ).bind(brain).bind(version).bind(source).fetch_one(&mut **tx).await?;
    Ok(context)
}

fn realm(
    kind: &EntityKind,
    source: Uuid,
    context: Uuid,
    version: Uuid,
    span: &super::MentionSpan,
) -> Value {
    if matches!(kind, EntityKind::Technology | EntityKind::Concept) {
        return json!({"kind":"brain_term"});
    }
    // Unknown prose instances remain occurrence-scoped until equivalence is
    // supported. Repeated DEV/PROD or same-name people in one input cannot merge
    // just because the document shares canonical context.
    json!({"kind":"source_occurrence","source_id":source,"context_id":context,
        "version_id":version,"byte_start":span.byte_start,"byte_end":span.byte_end})
}

/// No Brain lock is held while loading/parsing immutable text. Only signed-in
/// Brain writers can prepare a batch; models never choose authorization/scope.
pub async fn prepare_source(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    version: Uuid,
    output: &[u8],
) -> Result<PreparedSource> {
    auth.require_browser()?;
    let mut tx = auth.preparation_tx(&state.pool).await?;
    if !matches!(
        db::require_role(&mut tx, brain, false).await?.as_str(),
        "writer" | "admin"
    ) {
        return Err(Error::forbidden());
    }
    let epoch = graph::preparation_epoch(&mut tx, brain).await?;
    let deadline = input_gate(&mut tx, brain, version).await?;
    let source = model_gateway::source(state, &mut tx, brain, version, 2 * 1024 * 1024).await?;
    let candidates = source_candidates(version, &source.text, output).map_err(|failure| {
        tracing::warn!(brain_id=%brain, version_id=%version, failure=failure.code(), "Mapping candidates were rejected");
        Error::invalid(
            "Mapping candidates must reference exact source spans and valid local endpoints.",
        )
    })?;
    let context = context(&mut tx, brain, version, source.source_id).await?;
    tx.commit().await?;
    Ok(PreparedSource {
        brain,
        actor: auth.user.id,
        source_id: source.source_id,
        epoch,
        deadline,
        context,
        candidates,
    })
}

/// Atomically stage the whole batch under fresh authorization and canonical
/// input fences. The extractor-quality gate and qualified publication remain
/// separate; this does not make pending candidates available to retrieval.
pub async fn stage_source(state: &AppState, auth: &Auth, prepared: PreparedSource) -> Result<Uuid> {
    auth.require_browser()?;
    if prepared.actor != auth.user.id {
        return Err(Error::forbidden());
    }
    let mut tx = auth.publication_tx(&state.pool).await?;
    db::require_writer(&mut tx, prepared.brain).await?;
    if graph::preparation_epoch(&mut tx, prepared.brain).await? != prepared.epoch {
        return Err(changed());
    }
    let deadline = input_gate(
        &mut tx,
        prepared.brain,
        prepared.candidates.source_version_id,
    )
    .await?;
    if [deadline, prepared.deadline]
        .into_iter()
        .flatten()
        .any(|d| d <= Utc::now())
    {
        return Err(retention::unavailable());
    }
    let id = stage_validated(
        &mut tx,
        prepared.brain,
        auth.user.id,
        prepared.source_id,
        prepared.epoch,
        &prepared.context,
        &prepared.candidates,
        None,
    )
    .await?;
    input_gate(
        &mut tx,
        prepared.brain,
        prepared.candidates.source_version_id,
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// Internal typed boundary. `Some` binds server-owned production provenance;
/// neither the HTTP staging command nor the candidate JSON supplies this tuple.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn stage_validated(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    source: Uuid,
    epoch: i64,
    canonical_context: &Value,
    candidates: &SourceCandidates,
    producer: Option<(Uuid, Uuid, Uuid)>,
) -> Result<Uuid> {
    let payload = serde_json::to_value(candidates).map_err(|_| changed())?;
    let existing_context: Option<Uuid> = sqlx::query_scalar("SELECT id FROM knowledge_mapping_contexts WHERE brain_id=$1 AND source_id=$2 AND context=$3 ORDER BY id LIMIT 1")
        .bind(brain).bind(source).bind(Json(canonical_context)).fetch_optional(&mut **tx).await?;
    let context_id = if let Some(id) = existing_context {
        id
    } else {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_mapping_contexts(id,brain_id,source_id,context) VALUES($1,$2,$3,$4)")
            .bind(id).bind(brain).bind(source).bind(Json(canonical_context)).execute(&mut **tx).await?;
        id
    };
    let (request, policy, change) =
        producer.map_or((None, None, None), |(r, p, c)| (Some(r), Some(p), Some(c)));
    type Previous = (Uuid, Json<Value>, Option<Uuid>, Option<Uuid>);
    let previous:Option<Previous>=sqlx::query_as("SELECT id,payload,producer_policy_id,producer_change_id FROM knowledge_mapping_inputs WHERE brain_id=$1 AND source_version_id=$2 AND schema_revision=$3 AND adapter_revision=$4 AND context_id=$5 AND producer_request_id IS NOT DISTINCT FROM $6")
        .bind(brain).bind(candidates.source_version_id).bind(&candidates.schema).bind(&candidates.adapter_revision).bind(context_id).bind(request).fetch_optional(&mut **tx).await?;
    if let Some((id, Json(old), old_policy, old_change)) = previous {
        if old != payload || old_policy != policy || old_change != change {
            return Err(changed());
        }
        return Ok(id);
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_mapping_inputs(id,brain_id,source_version_id,schema_revision,adapter_revision,preparation_epoch,payload,created_by,context_id,producer_request_id,producer_policy_id,producer_change_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
        .bind(id).bind(brain).bind(candidates.source_version_id).bind(&candidates.schema).bind(&candidates.adapter_revision).bind(epoch).bind(Json(&payload)).bind(actor).bind(context_id).bind(request).bind(policy).bind(change).execute(&mut **tx).await?;
    let mut mention_ids = BTreeMap::new();
    let rows:Vec<Value>=candidates.mentions.iter().map(|m| {
        let mention_id=Uuid::new_v4(); mention_ids.insert(m.key.as_str(),mention_id);
        json!({"id":mention_id,"kind":m.kind.as_str(),"realm":realm(&m.kind,source,context_id,candidates.source_version_id,&m.span),
            "normalized_label":m.kind.normalized_label(&m.span.quote),"local_key":m.key,
            "byte_start":m.span.byte_start,"byte_end":m.span.byte_end,"quote":m.span.quote,"confidence":m.confidence})
    }).collect();
    // Three bounded bulk statements replace hundreds of writer round trips.
    sqlx::query("INSERT INTO knowledge_entities(id,brain_id,kind,realm,normalized_label)
        SELECT gen_random_uuid(),$1,kind,realm,normalized_label FROM (
         SELECT DISTINCT kind,realm,normalized_label FROM jsonb_to_recordset($2) AS x(kind text,realm jsonb,normalized_label text)) x
        ON CONFLICT(brain_id,kind,realm,normalized_label) DO NOTHING")
        .bind(brain).bind(Json(&rows)).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_mentions(id,brain_id,input_id,entity_id,local_key,byte_start,byte_end,quote,confidence)
        SELECT x.id,$1,$2,e.id,x.local_key,x.byte_start,x.byte_end,x.quote,x.confidence
        FROM jsonb_to_recordset($3) AS x(id uuid,kind text,realm jsonb,normalized_label text,local_key text,byte_start integer,byte_end integer,quote text,confidence float8)
        JOIN knowledge_entities e ON e.brain_id=$1 AND e.kind=x.kind AND e.realm=x.realm AND e.normalized_label=x.normalized_label")
        .bind(brain).bind(id).bind(Json(&rows)).execute(&mut **tx).await?;
    let relations: Vec<Value> = candidates
        .relations
        .iter()
        .map(|r| (r.from.as_str(), r.to.as_str(), r.relation.as_str(), &r.span))
        .chain(
            candidates
                .aliases
                .iter()
                .map(|r| (r.left.as_str(), r.right.as_str(), "alias", &r.span)),
        )
        .map(|(a, b, kind, span)| {
            json!({"id":Uuid::new_v4(),"a":mention_ids[a],"b":mention_ids[b],"relation":kind,
            "byte_start":span.byte_start,"byte_end":span.byte_end,"quote":span.quote})
        })
        .collect();
    sqlx::query("INSERT INTO knowledge_relation_candidates(id,brain_id,input_id,from_mention,to_mention,relation,byte_start,byte_end,quote)
        SELECT id,$1,$2,a,b,relation,byte_start,byte_end,quote FROM jsonb_to_recordset($3)
         AS x(id uuid,a uuid,b uuid,relation text,byte_start integer,byte_end integer,quote text)")
        .bind(brain).bind(id).bind(Json(&relations)).execute(&mut **tx).await?;
    db::audit(
        tx,
        actor,
        brain,
        "knowledge.mapping.stage",
        id,
        "source_validated_candidates",
    )
    .await?;
    Ok(id)
}
