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

async fn input_gate(tx: &mut Tx<'_>, brain: Uuid, version: Uuid) -> Result<Option<DateTime<Utc>>> {
    let deadline: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
        "SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at)
         FROM recollect_source_knowledge v JOIN sources s ON s.brain_id=v.brain_id AND s.current_version=v.id
         WHERE v.brain_id=$1 AND v.id=$2 AND v.processing='ready'
          AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
          AND NOT EXISTS(SELECT 1 FROM model_input_fences f WHERE f.brain_id=v.brain_id AND f.source_version_id=v.id)",
    ).bind(brain).bind(version).fetch_optional(&mut **tx).await?;
    deadline.map(|d| d.0).ok_or_else(retention::unavailable)
}

async fn context(tx: &mut Tx<'_>, brain: Uuid, version: Uuid, source: Uuid) -> Result<Value> {
    let Json(context): Json<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object(
          'selection',coalesce((SELECT selection FROM source_import_scopes WHERE brain_id=$1 AND version_id=$2),'{}'::jsonb),
          'areas',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=$1 AND m.source_id=$3 AND g.kind='area' ORDER BY g.id),
          'environments',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=$1 AND m.source_id=$3 AND g.kind='environment' ORDER BY g.id))",
    ).bind(brain).bind(version).bind(source).fetch_one(&mut **tx).await?;
    Ok(context)
}

fn realm(kind: &EntityKind, source: Uuid, context: Uuid) -> Value {
    if matches!(kind, EntityKind::Technology | EntityKind::Concept) {
        return json!({"kind":"brain_term"});
    }
    // A canonical origin remains part of an instance realm even when context
    // exists: context similarity is not equivalence proof. Reconciliation can
    // later link independently retained originals with explicit support.
    json!({"kind":"source_instance","source_id":source,"context_id":context})
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
    let candidates = source_candidates(version, &source.text, output).map_err(|_| {
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
    let payload = serde_json::to_value(&prepared.candidates).map_err(|_| changed())?;
    // Canonical context equality is resolved inside the Brain writer boundary.
    // Never index arbitrarily large JSON values or use a hash as identity.
    let context_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM knowledge_mapping_contexts WHERE brain_id=$1 AND source_id=$2 AND context=$3 ORDER BY id LIMIT 1")
        .bind(prepared.brain).bind(prepared.source_id).bind(Json(&prepared.context)).fetch_optional(&mut *tx).await?;
    let context_id = if let Some(id) = context_id {
        id
    } else {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_mapping_contexts(id,brain_id,source_id,context) VALUES($1,$2,$3,$4)")
            .bind(id).bind(prepared.brain).bind(prepared.source_id).bind(Json(&prepared.context)).execute(&mut *tx).await?;
        id
    };
    let previous: Option<(Uuid, Json<Value>)> = sqlx::query_as(
        "SELECT id,payload FROM knowledge_mapping_inputs WHERE brain_id=$1 AND source_version_id=$2 AND schema_revision=$3 AND adapter_revision=$4 AND context_id=$5",
    ).bind(prepared.brain).bind(prepared.candidates.source_version_id).bind(&prepared.candidates.schema).bind(&prepared.candidates.adapter_revision).bind(context_id).fetch_optional(&mut *tx).await?;
    if let Some((id, Json(previous))) = previous {
        if previous != payload {
            return Err(changed());
        }
        tx.commit().await?;
        return Ok(id);
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_mapping_inputs(id,brain_id,source_version_id,schema_revision,adapter_revision,preparation_epoch,payload,created_by,context_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(id).bind(prepared.brain).bind(prepared.candidates.source_version_id).bind(&prepared.candidates.schema).bind(&prepared.candidates.adapter_revision).bind(prepared.epoch).bind(Json(&payload)).bind(auth.user.id).bind(context_id).execute(&mut *tx).await?;
    let mut mentions = BTreeMap::new();
    for mention in &prepared.candidates.mentions {
        let entity: Uuid = sqlx::query_scalar("INSERT INTO knowledge_entities(id,brain_id,kind,realm,normalized_label) VALUES($1,$2,$3,$4,$5) ON CONFLICT(brain_id,kind,realm,normalized_label) DO UPDATE SET normalized_label=excluded.normalized_label RETURNING id")
            .bind(Uuid::new_v4()).bind(prepared.brain).bind(mention.kind.as_str()).bind(Json(realm(&mention.kind,prepared.source_id,context_id))).bind(mention.kind.normalized_label(&mention.span.quote)).fetch_one(&mut *tx).await?;
        let mention_id = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_mentions(id,brain_id,input_id,entity_id,local_key,byte_start,byte_end,quote,confidence) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(mention_id).bind(prepared.brain).bind(id).bind(entity).bind(&mention.key).bind(mention.span.byte_start as i32).bind(mention.span.byte_end as i32).bind(&mention.span.quote).bind(mention.confidence).execute(&mut *tx).await?;
        mentions.insert(mention.key.as_str(), mention_id);
    }
    let relations = prepared
        .candidates
        .relations
        .iter()
        .map(|r| (r.from.as_str(), r.to.as_str(), r.relation.as_str(), &r.span))
        .chain(
            prepared
                .candidates
                .aliases
                .iter()
                .map(|r| (r.left.as_str(), r.right.as_str(), "alias", &r.span)),
        );
    for (from, to, relation, span) in relations {
        sqlx::query("INSERT INTO knowledge_relation_candidates(id,brain_id,input_id,from_mention,to_mention,relation,byte_start,byte_end,quote) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(Uuid::new_v4()).bind(prepared.brain).bind(id).bind(mentions[from]).bind(mentions[to]).bind(relation).bind(span.byte_start as i32).bind(span.byte_end as i32).bind(&span.quote).execute(&mut *tx).await?;
    }
    db::audit(
        &mut tx,
        auth.user.id,
        prepared.brain,
        "knowledge.mapping.stage",
        id,
        "source_validated_candidates",
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
