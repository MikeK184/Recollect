//! Scoped semantic cards and exact contributor navigation. No model/native I/O
//! runs in the publication transaction, and semantic IDs are never claim IDs.
use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    graph,
    memory_evidence::Tx,
    publication, retrieval,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde_json::json;
use sqlx::FromRow;
use std::{collections::BTreeSet, time::Duration};
use uuid::Uuid;

mod labels;
mod sources;
use crate::db::admission::{self, BrainAccess};
pub use sources::{__path_view as __path_source_view, view as source_view};
const SOURCE_WINDOW: usize = 250;
const DETAIL_WINDOW: usize = 80;

// Reader epochs also change for harmless human overrides. Automatic community
// freshness instead follows its exact semantic outbox revision and selected
// qualified recipe. Facets and explicit split groups have no report dependency.
const CURRENT_TOPICS: &str = "SELECT t.*,CASE WHEN t.kind='community' THEN r.expires_at END AS organization_expires_at FROM knowledge_topics t
 LEFT JOIN analytics_reports r ON r.brain_id=t.brain_id AND r.id=t.organization_report_id
 LEFT JOIN knowledge_organization_calculations c ON c.brain_id=t.brain_id AND c.report_id=r.id
 LEFT JOIN knowledge_organization_outbox o ON o.brain_id=t.brain_id
 LEFT JOIN knowledge_organization_heads h ON h.brain_id=t.brain_id
 LEFT JOIN knowledge_organization_recipes p ON p.brain_id=t.brain_id AND p.change_id=h.change_id
 WHERE t.brain_id=$1 AND (t.kind<>'community' OR
  (r.purpose='organization' AND r.state='ready' AND (r.expires_at IS NULL OR r.expires_at>clock_timestamp())
   AND c.outbox_revision=o.revision AND c.recipe_change_id=h.change_id AND p.quality_state='qualified'))";

// Every aggregate/detail stream joins the same input IDs whose exact selected
// producer and immutable source were qualified once in this read-only snapshot.
// Candidate staging alone is never visible.
const ELIGIBLE: &str = "eligible AS MATERIALIZED (
 SELECT i.id,i.brain_id,i.source_version_id,i.context_id FROM knowledge_mapping_inputs i
 WHERE i.brain_id=$1 AND i.state='published' AND i.id=ANY($2)
), visible_mentions AS (
 SELECT m.*,i.source_version_id,e.kind FROM knowledge_mentions m
 JOIN eligible i ON i.id=m.input_id
 JOIN knowledge_entities e ON e.brain_id=m.brain_id AND e.id=m.entity_id
 WHERE m.brain_id=$1
), visible_memberships AS (
 SELECT m.*,i.source_version_id FROM knowledge_topic_memberships m
 JOIN eligible i ON i.id=m.input_id JOIN current_topics t ON t.id=m.topic_id AND t.brain_id=m.brain_id WHERE m.brain_id=$1
 AND NOT EXISTS(SELECT 1 FROM knowledge_topic_exclusions x WHERE x.brain_id=m.brain_id AND x.topic_id=m.topic_id AND x.input_id=m.input_id)
 AND NOT EXISTS(SELECT 1 FROM knowledge_topic_splits x WHERE x.brain_id=m.brain_id AND x.original_topic_id=m.topic_id AND x.input_id=m.input_id)
 AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions x JOIN knowledge_source_inputs si ON si.brain_id=x.brain_id AND si.id=x.input_id JOIN knowledge_mapping_contexts mc ON mc.brain_id=i.brain_id AND mc.id=i.context_id AND mc.context=si.context WHERE x.brain_id=m.brain_id AND x.topic_id=m.topic_id AND si.source_version_id=i.source_version_id)
 AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_splits x JOIN knowledge_source_inputs si ON si.brain_id=x.brain_id AND si.id=x.input_id JOIN knowledge_mapping_contexts mc ON mc.brain_id=i.brain_id AND mc.id=i.context_id AND mc.context=si.context WHERE x.brain_id=m.brain_id AND x.original_topic_id=m.topic_id AND si.source_version_id=i.source_version_id)
) ";

fn eligible_sql() -> String {
    format!("WITH current_topics AS MATERIALIZED ({CURRENT_TOPICS}), {ELIGIBLE}")
}

async fn community_status(tx: &mut Tx<'_>, brain: Uuid) -> Result<OrganizationCommunityStatus> {
    let (state, report_id, vector_coverage): (String, Option<Uuid>, Option<String>) = sqlx::query_as("SELECT CASE
      WHEN h.change_id IS NULL THEN 'unconfigured'
      WHEN p.quality_state<>'qualified' OR p.quality_receipt_sha256 IS NULL THEN 'blocked'
      WHEN o.brain_id IS NULL THEN 'pending'
      WHEN o.state='blocked_recipe' OR (o.state='empty' AND o.report_id IS NULL) THEN 'blocked'
      WHEN o.state='failed' OR r.state IN ('failed','cancelled','removed') THEN 'failed'
      WHEN o.state='pending' OR o.report_id IS NULL THEN 'pending'
      WHEN c.recipe_change_id IS DISTINCT FROM h.change_id OR c.outbox_revision IS DISTINCT FROM o.revision THEN 'stale'
      WHEN r.state IN ('queued','running') THEN 'queued'
      WHEN r.purpose='organization' AND r.state='ready' AND (r.expires_at IS NULL OR r.expires_at>clock_timestamp())
       AND c.recipe_change_id=h.change_id AND c.outbox_revision=o.revision THEN 'ready'
      ELSE 'stale' END AS state, o.report_id, o.vector_coverage
      FROM brains b LEFT JOIN knowledge_organization_heads h ON h.brain_id=b.id
      LEFT JOIN knowledge_organization_recipes p ON p.brain_id=b.id AND p.change_id=h.change_id
      LEFT JOIN knowledge_organization_outbox o ON o.brain_id=b.id
      LEFT JOIN analytics_reports r ON r.brain_id=b.id AND r.id=o.report_id
      LEFT JOIN knowledge_organization_calculations c ON c.brain_id=b.id AND c.report_id=r.id
      WHERE b.id=$1").bind(brain).fetch_one(&mut **tx).await?;
    Ok(OrganizationCommunityStatus {
        state: serde_json::from_value(json!(state)).map_err(|_| changed())?,
        report_id,
        vector_features: vector_coverage.map(|v| {
            if v == "complete" {
                VectorFeatureReadiness::Ready
            } else if v.starts_with("partial_features:") {
                VectorFeatureReadiness::Partial
            } else {
                VectorFeatureReadiness::Unavailable
            }
        }),
    })
}

async fn producer_status(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
) -> Result<ProducerReadiness> {
    let current = crate::model_policy::current(state, tx, brain).await?;
    let p = current.policy;
    let prompt = if p.autonomous_memory {
        crate::autonomous::PROMPT
    } else {
        crate::model_gateway::EXTRACT_PROMPT
    };
    let schema = if p.autonomous_memory {
        crate::autonomous::PROMPT
    } else {
        crate::model_gateway::EXTRACT_SCHEMA
    };
    let exact: Option<bool> = sqlx::query_scalar("SELECT f.quality_state='qualified' AND f.quality_receipt_sha256 IS NOT NULL
      AND f.provider=$2 AND f.requested_model=$3 AND f.prompt_label=$4 AND f.schema_label=$5 AND f.adapter_revision=$6 AND f.mapping_schema=$7
      FROM knowledge_mapping_producer_heads h JOIN knowledge_mapping_producer_profiles f ON f.brain_id=h.brain_id AND f.change_id=h.change_id WHERE h.brain_id=$1")
        .bind(brain).bind(&p.provider).bind(&p.text_model).bind(prompt).bind(schema)
        .bind(super::production::ADAPTER).bind(super::SOURCE_SCHEMA).fetch_optional(&mut **tx).await?;
    Ok(match exact {
        None => ProducerReadiness::Unconfigured,
        Some(true)
            if p.enabled
                && p.purposes.iter().any(|v| v == "extraction")
                && crate::model_policy::matches_installation(state, &p) =>
        {
            ProducerReadiness::Ready
        }
        _ => ProducerReadiness::Blocked,
    })
}

#[derive(FromRow)]
struct Mention {
    id: Uuid,
    entity_id: Uuid,
    input_id: Uuid,
    source_version_id: Uuid,
    byte_start: i32,
    byte_end: i32,
    quote: String,
}
#[derive(FromRow)]
struct Membership {
    topic_id: Uuid,
    entity_id: Uuid,
    input_id: Uuid,
    source_version_id: Uuid,
    reason: String,
    weight: f64,
}
#[derive(FromRow)]
struct Connection {
    id: Uuid,
    from_entity: Uuid,
    to_entity: Uuid,
    relation: String,
    source_version_id: Uuid,
    byte_start: i32,
    byte_end: i32,
    quote: String,
}
#[derive(Clone, FromRow)]
struct CardRow {
    id: Uuid,
    kind: String,
    topic_kind: Option<String>,
    label: String,
    source_count: i64,
    entity_count: i64,
    pinned: bool,
    change_id: Option<Uuid>,
    representative_source: Uuid,
    representative_mention: Uuid,
    #[sqlx(default)]
    provenance_kind: String,
}
impl CardRow {
    fn card(&self) -> OrganizationCard {
        OrganizationCard {
            id: self.id,
            kind: self.kind.clone(),
            topic_kind: match self.topic_kind.as_deref() {
                Some("facet") => Some(OrganizationTopicKind::Facet),
                Some("community") => Some(OrganizationTopicKind::Community),
                Some("split") => Some(OrganizationTopicKind::Split),
                Some("manual_overlay") => Some(OrganizationTopicKind::ManualOverlay),
                _ => None,
            },
            label: self.label.clone(),
            source_count: self.source_count as usize,
            entity_count: self.entity_count as usize,
            pinned: self.pinned,
            change_id: self.change_id,
        }
    }
}
fn changed() -> Error {
    Error(
        StatusCode::CONFLICT,
        "organization_changed",
        "Organization inputs changed. Read the current page again.",
    )
}
fn query_error(phase: &'static str, error: sqlx::Error) -> Error {
    // SQLSTATE and a fixed phase are diagnostic metadata, never driver text,
    // SQL, parameters, source content or credentials.
    let code = error
        .as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned());
    tracing::warn!(phase, sqlstate = code, "Bounded organization query failed");
    error.into()
}
fn source_key(id: Uuid) -> String {
    graph::key("source_version", id)
}

/// Both source streams must select the identical canonical window, including
/// a topic center. A pinned cursor never widens to unrelated source IDs.
/// This is a structural candidate page. Exact producer origins are qualified
/// once in bounded batches below, never through a scalar call for every source.
async fn candidate_sources(
    tx: &mut Tx<'_>,
    brain: Uuid,
    after: Option<Uuid>,
    through: Option<Uuid>,
    topic: Option<Uuid>,
    entity: Option<Uuid>,
) -> Result<Vec<(Uuid, Uuid)>> {
    let rows:Vec<(Uuid,Uuid)>=sqlx::query_as(&format!(
        "WITH current_topics AS MATERIALIZED ({CURRENT_TOPICS})
         SELECT s.id,v.id FROM sources s JOIN source_versions v ON v.brain_id=s.brain_id AND v.id=s.current_version
         WHERE s.brain_id=$1 AND ($2::uuid IS NULL OR s.id>$2) AND ($6::uuid IS NULL OR s.id<=$6)
          AND ($3::uuid IS NULL OR EXISTS(SELECT 1 FROM knowledge_topic_memberships m JOIN knowledge_mapping_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id JOIN current_topics t ON t.brain_id=m.brain_id AND t.id=m.topic_id
           WHERE m.brain_id=$1 AND m.topic_id=$3 AND i.source_version_id=v.id AND i.state='published'
            AND NOT EXISTS(SELECT 1 FROM knowledge_topic_exclusions x WHERE x.brain_id=m.brain_id AND x.topic_id=m.topic_id AND x.input_id=m.input_id)
            AND NOT EXISTS(SELECT 1 FROM knowledge_topic_splits x WHERE x.brain_id=m.brain_id AND x.original_topic_id=m.topic_id AND x.input_id=m.input_id)
            AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions x JOIN knowledge_source_inputs si ON si.brain_id=x.brain_id AND si.id=x.input_id JOIN knowledge_mapping_contexts mc ON mc.brain_id=i.brain_id AND mc.id=i.context_id AND mc.context=si.context WHERE x.brain_id=m.brain_id AND x.topic_id=m.topic_id AND si.source_version_id=i.source_version_id)
            AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_splits x JOIN knowledge_source_inputs si ON si.brain_id=x.brain_id AND si.id=x.input_id JOIN knowledge_mapping_contexts mc ON mc.brain_id=i.brain_id AND mc.id=i.context_id AND mc.context=si.context WHERE x.brain_id=m.brain_id AND x.original_topic_id=m.topic_id AND si.source_version_id=i.source_version_id)) OR EXISTS(SELECT 1 FROM knowledge_topic_sources m JOIN knowledge_source_inputs i ON i.id=m.input_id AND i.brain_id=m.brain_id JOIN current_topics t ON t.id=m.topic_id AND t.brain_id=m.brain_id WHERE m.brain_id=$1 AND m.topic_id=$3 AND i.source_version_id=v.id AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions x WHERE x.topic_id=m.topic_id AND x.input_id=m.input_id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_splits x WHERE x.original_topic_id=m.topic_id AND x.input_id=m.input_id)))
          AND ($4::uuid IS NULL OR EXISTS(SELECT 1 FROM knowledge_mentions m JOIN knowledge_mapping_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id
           WHERE m.brain_id=$1 AND m.entity_id=$4 AND i.source_version_id=v.id AND i.state='published'))
         ORDER BY s.id LIMIT $5"))
        .bind(brain).bind(after).bind(topic).bind(entity).bind((SOURCE_WINDOW+1) as i64).bind(through).fetch_all(&mut **tx).await?;
    Ok(rows)
}

async fn entity_cards(
    tx: &mut Tx<'_>,
    brain: Uuid,
    versions: &[Uuid],
    center: Option<Uuid>,
    ids: Option<&[Uuid]>,
    limit: usize,
    offset: usize,
) -> Result<Vec<CardRow>> {
    let eligible = eligible_sql();
    let sql=format!("{eligible} SELECT entity_id AS id,kind,NULL::text AS topic_kind,(array_agg(quote ORDER BY id))[1] AS label,
        count(DISTINCT source_version_id)::bigint AS source_count,1::bigint AS entity_count,
        false AS pinned,NULL::uuid AS change_id,(array_agg(source_version_id ORDER BY id))[1] AS representative_source,(array_agg(id ORDER BY id))[1] AS representative_mention
        FROM visible_mentions WHERE ($3::uuid IS NULL OR entity_id=$3) AND ($4::uuid[] IS NULL OR entity_id=ANY($4))
        GROUP BY entity_id,kind ORDER BY source_count DESC,label,entity_id LIMIT $5 OFFSET $6");
    Ok(sqlx::query_as(&sql)
        .bind(brain)
        .bind(versions)
        .bind(center)
        .bind(ids)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| query_error("entity_cards", e))?)
}

async fn prepare(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    input: &OrganizationRequest,
) -> Result<OrganizationView> {
    if input.scope.kind != "knowledge" || !(1..=50).contains(&input.limit) {
        return Err(Error::invalid(
            "Choose a knowledge scope and 1 to 50 organization cards.",
        ));
    }
    let mut normalized = input.clone();
    let mut recall = RecallRequest {
        query: "Organization selection".into(),
        selection: input.scope.selection.clone(),
        operation_id: input.scope.operation_id,
        collection_id: input.scope.collection_id,
        manifest_revision_id: input.scope.manifest_revision_id,
        fact_at: input.scope.fact_at,
        mode: input.scope.mode.clone(),
        ..Default::default()
    };
    retrieval::validate(&mut recall)?;
    normalized.scope.selection = recall.selection;
    let input = &normalized;
    let mut tx = auth.preparation_tx(&state.pool).await?;
    sqlx::query("SET LOCAL recollect.bulk_qualification='on'")
        .execute(&mut *tx)
        .await?;
    // These bounded interactive reads do too little work to amortize LLVM
    // compilation, particularly before newly populated tables are analyzed.
    sqlx::query("SET LOCAL jit=off").execute(&mut *tx).await?;
    sqlx::query("SET LOCAL statement_timeout='1500ms'")
        .execute(&mut *tx)
        .await?;
    retrieval::graph::authorize(&mut tx, auth, brain, &input.scope).await?;
    let epoch = graph::preparation_epoch(&mut tx, brain).await?;
    if let Some(c) = &input.cursor {
        let malformed = matches!(&c.row_cursor, OrganizationRowCursor::Memberships{after_topic,after_entity,after_input}
            if [after_topic.is_some(),after_entity.is_some(),after_input.is_some()].iter().any(|v|*v)
            && ![after_topic.is_some(),after_entity.is_some(),after_input.is_some()].iter().all(|v|*v));
        if c.epoch != epoch
            || c.center != input.center
            || i64::try_from(c.card_offset).is_err()
            || c.card_offset.checked_add(input.limit).is_none()
            || malformed
            || serde_json::to_value(&c.scope).ok() != serde_json::to_value(&input.scope).ok()
            || c.expires_at.is_some_and(|at| at <= Utc::now())
        {
            return Err(changed());
        }
    }
    let offset = input.cursor.as_ref().map_or(0, |c| c.card_offset);
    let after_source = input.cursor.as_ref().and_then(|c| c.after_source);
    let mut row_cursor = input
        .cursor
        .as_ref()
        .map(|c| c.row_cursor.clone())
        .unwrap_or(OrganizationRowCursor::Mentions { after_id: None });
    let (topic, entity) = match input.center {
        Some(OrganizationCenter::Topic { id }) => (Some(id), None),
        Some(OrganizationCenter::Entity { id }) => (None, Some(id)),
        None => (None, None),
    };
    let mut candidates =
        candidate_sources(&mut tx, brain, after_source, None, topic, entity).await?;
    let more_sources = candidates.len() > SOURCE_WINDOW;
    candidates.truncate(SOURCE_WINDOW);
    let sources: Vec<_> = candidates
        .iter()
        .map(|(id, revision_id)| graph::Entity {
            kind: "source_version".into(),
            id: *id,
            revision_id: *revision_id,
        })
        .collect();
    let mut scope = input.scope.clone();
    let mut qualified = retrieval::graph::qualify(
        state,
        &mut tx,
        auth,
        brain,
        &mut scope,
        &sources,
        (SOURCE_WINDOW, None),
    )
    .await?;
    let versions: Vec<_> = sources
        .iter()
        .filter(|e| qualified.nodes.contains_key(&e.key()))
        .map(|e| e.revision_id)
        .collect();
    let fenced:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM source_versions WHERE brain_id=$1 AND id=ANY($2) AND recollect_source_model_fenced(brain_id,id)")
        .bind(brain).bind(&versions).fetch_all(&mut *tx).await?;
    for id in fenced {
        qualified.nodes.remove(&source_key(id));
    }
    let versions: Vec<_> = sources
        .iter()
        .filter(|e| qualified.nodes.contains_key(&e.key()))
        .map(|e| e.revision_id)
        .collect();
    // Evaluate production provenance once per bounded cohort in this immutable
    // preparation. Every aggregate/detail read reuses exactly this qualified
    // set; publication still rechecks the epoch and earliest source deadline.
    let input_ids = labels::current_inputs(&mut tx, brain, &versions).await?;
    // Counts and names use all eligible rows in the qualified source window,
    // independently of whichever detail prefix happens to fit this response.
    let mut entities = entity_cards(
        &mut tx,
        brain,
        &input_ids,
        entity,
        None,
        input.limit + 1,
        offset,
    )
    .await?;
    let mut all_topics =
        labels::cards(&mut tx, brain, &input_ids, topic, entity, &qualified.nodes).await?;
    let label_topic_ids: Option<Vec<_>> = entity.map(|_| all_topics.iter().map(|c| c.id).collect());
    let label_window = sources::window(
        &mut tx,
        brain,
        &versions,
        topic,
        &qualified.nodes,
        &all_topics,
        label_topic_ids.as_deref(),
    )
    .await?;
    // Labels use the same qualified human/source composition for every center.
    // An entity center still emits only its original centered detail streams.
    let source_window = entity.is_none().then_some(&label_window);
    {
        let w = &label_window;
        let mut merged: std::collections::BTreeMap<_, _> =
            all_topics.into_iter().map(|c| (c.id, c)).collect();
        merged.extend(w.cards.iter().cloned().map(|c| (c.id, c)));
        all_topics = merged.into_values().collect();
    }
    if all_topics.len() > 10_000 {
        return Err(Error::invalid(
            "The complete topic window exceeds its bounded envelope.",
        ));
    }
    all_topics.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then(b.source_count.cmp(&a.source_count))
            .then(a.label.cmp(&b.label))
            .then(a.id.cmp(&b.id))
    });
    let mut topics: Vec<_> = all_topics
        .iter()
        .skip(offset)
        .take(input.limit + 1)
        .cloned()
        .collect();
    let more_cards = entities.len() > input.limit || topics.len() > input.limit;
    entities.truncate(input.limit);
    topics.truncate(input.limit);
    let mut mentions = Vec::<Mention>::new();
    let mut memberships = Vec::<Membership>::new();
    let mut connections = Vec::<Connection>::new();
    let eligible = eligible_sql();
    loop {
        let remaining = DETAIL_WINDOW - mentions.len() - memberships.len() - connections.len();
        if remaining == 0 || matches!(row_cursor, OrganizationRowCursor::Done) {
            break;
        }
        row_cursor = match row_cursor {
            OrganizationRowCursor::Mentions { after_id } => {
                let sql=format!("{eligible} SELECT id,entity_id,input_id,source_version_id,byte_start,byte_end,quote FROM visible_mentions
                    WHERE ($3::uuid IS NULL OR entity_id=$3) AND ($4::uuid IS NULL OR id>$4) ORDER BY id LIMIT $5");
                let mut rows: Vec<Mention> = sqlx::query_as(&sql)
                    .bind(brain)
                    .bind(&input_ids)
                    .bind(entity)
                    .bind(after_id)
                    .bind((remaining + 1) as i64)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|e| query_error("mentions", e))?;
                let more = rows.len() > remaining;
                rows.truncate(remaining);
                let after = rows.last().map(|r| r.id);
                mentions.extend(rows);
                if more {
                    OrganizationRowCursor::Mentions { after_id: after }
                } else {
                    OrganizationRowCursor::Memberships {
                        after_topic: None,
                        after_entity: None,
                        after_input: None,
                    }
                }
            }
            OrganizationRowCursor::Memberships {
                after_topic,
                after_entity,
                after_input,
            } => {
                let sql=format!("{eligible} SELECT topic_id,entity_id,input_id,source_version_id,reason,weight FROM visible_memberships m
                    WHERE ($3::uuid IS NULL OR topic_id=$3) AND ($4::uuid IS NULL OR entity_id=$4)
                     AND ($5::uuid IS NULL OR (topic_id,entity_id,input_id)>($5,$6,$7))
                     AND EXISTS(SELECT 1 FROM visible_mentions v WHERE v.input_id=m.input_id AND v.entity_id=m.entity_id)
                    ORDER BY topic_id,entity_id,input_id LIMIT $8");
                let mut rows: Vec<Membership> = sqlx::query_as(&sql)
                    .bind(brain)
                    .bind(&input_ids)
                    .bind(topic)
                    .bind(entity)
                    .bind(after_topic)
                    .bind(after_entity)
                    .bind(after_input)
                    .bind((remaining + 1) as i64)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|e| query_error("memberships", e))?;
                let more = rows.len() > remaining;
                rows.truncate(remaining);
                let after = rows.last().map(|r| (r.topic_id, r.entity_id, r.input_id));
                memberships.extend(rows);
                if more {
                    let (t, e, i) = after.expect("nonempty emitted keyset");
                    OrganizationRowCursor::Memberships {
                        after_topic: Some(t),
                        after_entity: Some(e),
                        after_input: Some(i),
                    }
                } else {
                    OrganizationRowCursor::Connections { after_id: None }
                }
            }
            OrganizationRowCursor::Connections { after_id } => {
                // Relations already name exact mention IDs. Indexed endpoint
                // joins avoid scanning the complete mention window twice when
                // freshly populated tables still have sparse planner stats.
                let sql=format!("{eligible} SELECT r.id,a.entity_id AS from_entity,b.entity_id AS to_entity,r.relation,i.source_version_id,r.byte_start,r.byte_end,r.quote
                    FROM knowledge_relation_candidates r JOIN eligible i ON i.id=r.input_id
                    JOIN knowledge_mentions a ON a.brain_id=r.brain_id AND a.input_id=r.input_id AND a.id=r.from_mention
                    JOIN knowledge_mentions b ON b.brain_id=r.brain_id AND b.input_id=r.input_id AND b.id=r.to_mention
                    WHERE r.brain_id=$1 AND r.disposition<>'rejected' AND (r.relation<>'alias' OR r.disposition='accepted')
                     AND ($3::uuid IS NULL OR a.entity_id=$3 OR b.entity_id=$3) AND ($4::uuid IS NULL OR r.id>$4)
                    ORDER BY r.id LIMIT $5");
                let mut rows: Vec<Connection> = sqlx::query_as(&sql)
                    .bind(brain)
                    .bind(&input_ids)
                    .bind(entity)
                    .bind(after_id)
                    .bind((remaining + 1) as i64)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|e| query_error("connections", e))?;
                let more = rows.len() > remaining;
                rows.truncate(remaining);
                let after = rows.last().map(|r| r.id);
                connections.extend(rows);
                if more {
                    OrganizationRowCursor::Connections { after_id: after }
                } else {
                    OrganizationRowCursor::Done
                }
            }
            OrganizationRowCursor::Done => OrganizationRowCursor::Done,
        };
    }
    // Hydrate every emitted endpoint, including those outside the independently
    // ranked overview page. A detail row contributes at most three nodes; with
    // 80 rows plus 100 cards, representative sources and at most 50 extra
    // representative entities is <=490 nodes (usually much less after dedup).
    let remaining_source = DETAIL_WINDOW - mentions.len() - memberships.len() - connections.len();
    let (source_memberships, source_more) = source_window.as_ref().map_or((vec![], false), |w| {
        sources::details(w, None, remaining_source)
    });
    let mut representative_sources = sources::representatives(&label_window, &topics);
    let source_deadline = label_window.deadline;
    qualified.deadline = [qualified.deadline, source_deadline]
        .into_iter()
        .flatten()
        .min();
    let mut source_next_cursor = if source_more {
        let last = source_memberships.last();
        Some(OrganizationSourceCursor {
            brain_id: brain,
            after_source,
            through_source: candidates.last().map(|c| c.0),
            after_topic: last.map(|r| r.topic_id),
            after_input: last.map(|r| r.input_id),
            epoch,
            scope: input.scope.clone(),
            topic_id: topic,
            expires_at: qualified.deadline,
        })
    } else if more_sources && source_window.is_some() {
        candidates.last().map(|c| OrganizationSourceCursor {
            brain_id: brain,
            after_source: Some(c.0),
            through_source: None,
            after_topic: None,
            after_input: None,
            epoch,
            scope: input.scope.clone(),
            topic_id: topic,
            expires_at: qualified.deadline,
        })
    } else {
        None
    };
    let mut endpoint_entities = BTreeSet::new();
    let mut endpoint_topics = BTreeSet::new();
    let mut evidence = BTreeSet::new();
    for m in &mentions {
        endpoint_entities.insert(m.entity_id);
        evidence.insert(m.source_version_id);
    }
    for m in &source_memberships {
        if let Some(r) = source_window.as_ref().and_then(|w| {
            w.rows
                .iter()
                .find(|r| r.topic_id == m.topic_id && r.input_id == m.input_id)
        }) {
            evidence.insert(r.version);
            endpoint_topics.insert(r.topic_id);
        }
    }
    for m in &memberships {
        endpoint_entities.insert(m.entity_id);
        endpoint_topics.insert(m.topic_id);
        evidence.insert(m.source_version_id);
    }
    for c in &connections {
        endpoint_entities.insert(c.from_entity);
        endpoint_entities.insert(c.to_entity);
        evidence.insert(c.source_version_id);
    }
    for c in entities.iter().chain(&topics) {
        evidence.insert(c.representative_source);
    }
    let representative_ids: Vec<_> = entities
        .iter()
        .chain(&topics)
        .map(|c| c.representative_mention)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let representatives:Vec<Mention>=sqlx::query_as(&format!("{eligible} SELECT id,entity_id,input_id,source_version_id,byte_start,byte_end,quote FROM visible_mentions WHERE id=ANY($3) ORDER BY id"))
        .bind(brain).bind(&input_ids).bind(&representative_ids).fetch_all(&mut *tx).await?;
    for mention in &representatives {
        endpoint_entities.insert(mention.entity_id);
    }
    let endpoint_entities: Vec<_> = endpoint_entities.into_iter().collect();
    let endpoint_topics: Vec<_> = endpoint_topics.into_iter().collect();
    let mut context = entity_cards(
        &mut tx,
        brain,
        &input_ids,
        None,
        Some(&endpoint_entities),
        DETAIL_WINDOW * 2 + 100,
        0,
    )
    .await?;
    context.extend(
        all_topics
            .iter()
            .filter(|c| endpoint_topics.contains(&c.id))
            .take(DETAIL_WINDOW)
            .cloned(),
    );
    for card in &context {
        evidence.insert(card.representative_source);
    }
    let overview_ids: BTreeSet<_> = entities.iter().chain(&topics).map(|c| c.id).collect();
    context.retain(|c| !overview_ids.contains(&c.id));
    // A visible contextual human name needs its canonical basis even when
    // that source falls outside the independent 80-row detail prefix.
    let context_topics: Vec<_> = context
        .iter()
        .filter(|c| c.kind == "topic")
        .cloned()
        .collect();
    representative_sources.extend(sources::representatives(&label_window, &context_topics));
    let (mapped_versions,report_deadline):(Vec<Uuid>,Option<DateTime<Utc>>)=sqlx::query_as(&format!("{eligible} SELECT coalesce(array_agg(DISTINCT source_version_id),ARRAY[]::uuid[]),
        (SELECT min(t.organization_expires_at) FROM visible_memberships m JOIN current_topics t ON t.brain_id=m.brain_id AND t.id=m.topic_id)
        FROM eligible WHERE EXISTS(SELECT 1 FROM visible_mentions m WHERE m.input_id=eligible.id)"))
        .bind(brain).bind(&input_ids).fetch_one(&mut *tx).await?;
    let mut mapped_versions: BTreeSet<_> = mapped_versions.into_iter().collect();
    if let Some(w) = &source_window {
        mapped_versions.extend(w.rows.iter().map(|r| r.version));
    }
    let mapped = mapped_versions.len();
    // A native report can expire before a visible subset of its contributors.
    // Bind the entire card/detail cursor window to that deadline, including
    // communities on later card pages, and recheck it before publication.
    qualified.deadline = [
        qualified.deadline,
        report_deadline,
        input.cursor.as_ref().and_then(|c| c.expires_at),
    ]
    .into_iter()
    .flatten()
    .min();
    if let Some(c) = &mut source_next_cursor {
        c.expires_at = qualified.deadline;
    }
    let work_remains = more_cards || !matches!(row_cursor, OrganizationRowCursor::Done);
    let next_cursor = if work_remains {
        Some(OrganizationCursor {
            after_source,
            card_offset: if more_cards {
                offset + input.limit
            } else {
                offset + entities.len().max(topics.len())
            },
            row_cursor,
            epoch,
            scope: input.scope.clone(),
            center: input.center.clone(),
            expires_at: qualified.deadline,
        })
    } else if more_sources {
        candidates.last().map(|(id, _)| OrganizationCursor {
            after_source: Some(*id),
            card_offset: 0,
            row_cursor: OrganizationRowCursor::Mentions { after_id: None },
            epoch,
            scope: input.scope.clone(),
            center: input.center.clone(),
            expires_at: qualified.deadline,
        })
    } else {
        None
    };
    let partial = next_cursor.is_some()
        || source_next_cursor.is_some()
        || after_source.is_some()
        || offset > 0
        || input.cursor.is_some();
    let coverage = OrganizationCoverage {
        examined_sources: candidates.len(),
        qualified_sources: versions.len(),
        mapped_sources: mapped as usize,
        unassigned_sources: versions.len().saturating_sub(mapped as usize),
        counts_are_lower_bounds: partial,
    };
    let view = OrganizationView {
        brain_id: brain,
        scope,
        epoch,
        expires_at: qualified.deadline,
        state: if partial {
            "partial"
        } else if mapped == 0 {
            "unassigned"
        } else {
            "ready"
        }
        .into(),
        topics: topics.iter().map(CardRow::card).collect(),
        entities: entities.iter().map(CardRow::card).collect(),
        context_cards: context.iter().map(CardRow::card).collect(),
        evidence: qualified
            .nodes
            .into_iter()
            .filter(|(k, _)| evidence.iter().any(|id| source_key(*id) == *k))
            .map(|(_, node)| node)
            .collect(),
        mentions: mentions
            .into_iter()
            .map(|m| OrganizationMention {
                id: m.id,
                entity_id: m.entity_id,
                input_id: m.input_id,
                source_key: source_key(m.source_version_id),
                byte_start: m.byte_start,
                byte_end: m.byte_end,
                quote: m.quote,
            })
            .collect(),
        representative_mentions: representatives
            .into_iter()
            .map(|m| OrganizationMention {
                id: m.id,
                entity_id: m.entity_id,
                input_id: m.input_id,
                source_key: source_key(m.source_version_id),
                byte_start: m.byte_start,
                byte_end: m.byte_end,
                quote: m.quote,
            })
            .collect(),
        memberships: memberships
            .into_iter()
            .map(|m| OrganizationMembership {
                topic_id: m.topic_id,
                entity_id: m.entity_id,
                input_id: m.input_id,
                source_key: source_key(m.source_version_id),
                reason: m.reason,
                weight: m.weight,
            })
            .collect(),
        connections: connections
            .into_iter()
            .map(|r| OrganizationConnection {
                id: r.id,
                from: r.from_entity,
                to: r.to_entity,
                interpretation: if r.relation == "alias" {
                    "supported_alias"
                } else {
                    "source_described_navigation"
                }
                .into(),
                relation: r.relation,
                source_key: source_key(r.source_version_id),
                byte_start: r.byte_start,
                byte_end: r.byte_end,
                quote: r.quote,
            })
            .collect(),
        source_memberships,
        representative_sources,
        source_next_cursor,
        coverage,
        community_status: Some(community_status(&mut tx, brain).await?),
        producer_status: Some(producer_status(state, &mut tx, brain).await?),
        next_cursor,
    };
    tx.commit().await?;
    Ok(view)
}

#[utoipa::path(post,path="/api/brains/{brain}/organization/view",operation_id="organizationView",params(("brain"=Uuid,Path)),request_body=OrganizationRequest,responses((status=200,body=OrganizationView)))]
pub async fn view(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<OrganizationRequest>,
) -> Result<Json<OrganizationView>> {
    let _permit = graph::read_capacity().try_acquire().map_err(|_| {
        Error(
            StatusCode::TOO_MANY_REQUESTS,
            "graph_busy",
            "Graph reads are busy. Try again shortly.",
        )
    })?;
    tokio::time::timeout(Duration::from_secs(5), async {
        for attempt in 0..3 {
            let prepared = prepare(&state, &auth, brain, &input).await?;
            let mut tx = admission::brain_tx(
                &state.pool,
                &auth,
                brain,
                BrainAccess::READ
                    .with_statement_timeout(Duration::from_millis(750))
                    .with_session_recheck(),
            )
            .await?
            .tx;
            retrieval::graph::authorize(&mut tx, &auth, brain, &prepared.scope).await?;
            if graph::preparation_epoch(&mut tx, brain).await? != prepared.epoch {
                tx.rollback().await?;
                if attempt < 2 && input.cursor.is_none() {
                    continue;
                }
                return Err(changed());
            }
            let fresh: bool =
                sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
                    .bind(prepared.expires_at)
                    .fetch_one(&mut *tx)
                    .await?;
            if !fresh {
                return Err(changed());
            }
            tx.commit().await?;
            return Ok(Json(prepared));
        }
        Err(changed())
    })
    .await
    .map_err(|_| {
        Error(
            StatusCode::GATEWAY_TIMEOUT,
            "organization_timeout",
            "Organization exceeded its bounded read deadline.",
        )
    })?
}

/// Reuse canonical scope/support/chunk gates before taking the short writer
/// boundary. The epoch/deadline check below makes this immutable preparation.
async fn prepare_topic_change(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    topic: Uuid,
    inputs: &[Uuid],
    require_visible: bool,
) -> Result<(i64, Option<DateTime<Utc>>)> {
    let mut tx = admission::brain_preparation_tx(
        &state.pool,
        auth.user.id,
        auth.device_id,
        brain,
        BrainAccess::READ.with_statement_timeout(Duration::from_millis(1500)),
    )
    .await?
    .tx;
    sqlx::query("SET LOCAL recollect.bulk_qualification='on'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL jit=off").execute(&mut *tx).await?;
    let epoch = graph::preparation_epoch(&mut tx, brain).await?;
    let mut after: Option<Uuid> = None;
    loop {
        let rows:Vec<(Uuid,Uuid,Uuid,Option<DateTime<Utc>>)>=sqlx::query_as(&format!(
        "WITH current_topics AS MATERIALIZED ({CURRENT_TOPICS})
         SELECT DISTINCT i.id,v.source_id,v.id,t.organization_expires_at FROM knowledge_topic_memberships m
         JOIN current_topics t ON t.brain_id=m.brain_id AND t.id=m.topic_id
         JOIN knowledge_mapping_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id
         JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.source_version_id
         WHERE m.brain_id=$1 AND m.topic_id=$2 AND (cardinality($3::uuid[])=0 OR i.id=ANY($3))
          AND i.state='published' AND ($6::uuid IS NULL OR i.id>$6)
          AND (NOT $5 OR (NOT EXISTS(SELECT 1 FROM knowledge_topic_exclusions x WHERE x.brain_id=m.brain_id AND x.topic_id=m.topic_id AND x.input_id=m.input_id)
           AND NOT EXISTS(SELECT 1 FROM knowledge_topic_splits x WHERE x.brain_id=m.brain_id AND x.original_topic_id=m.topic_id AND x.input_id=m.input_id)
           AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions x JOIN knowledge_source_inputs si ON si.brain_id=x.brain_id AND si.id=x.input_id JOIN knowledge_mapping_contexts mc ON mc.brain_id=i.brain_id AND mc.id=i.context_id AND mc.context=si.context WHERE x.brain_id=m.brain_id AND x.topic_id=m.topic_id AND si.source_version_id=i.source_version_id)
           AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_splits x JOIN knowledge_source_inputs si ON si.brain_id=x.brain_id AND si.id=x.input_id JOIN knowledge_mapping_contexts mc ON mc.brain_id=i.brain_id AND mc.id=i.context_id AND mc.context=si.context WHERE x.brain_id=m.brain_id AND x.original_topic_id=m.topic_id AND si.source_version_id=i.source_version_id)))
         ORDER BY i.id LIMIT $4"))
        .bind(brain).bind(topic).bind(inputs).bind(if inputs.is_empty(){250_i64}else{251_i64}).bind(require_visible).bind(after).fetch_all(&mut *tx).await?;
        if rows.len() > 250 || rows.is_empty() {
            return Err(changed());
        }
        let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
        let current: Vec<Uuid> =
            sqlx::query_scalar("SELECT input_id FROM recollect_mapping_current_input_ids($1,$2)")
                .bind(brain)
                .bind(ids)
                .fetch_all(&mut *tx)
                .await?;
        let entities: Vec<_> = rows
            .iter()
            .filter(|r| current.contains(&r.0))
            .map(|(_, source, version, _)| graph::Entity {
                kind: "source_version".into(),
                id: *source,
                revision_id: *version,
            })
            .collect();
        let mut scope = GraphSelection {
            kind: "knowledge".into(),
            ..Default::default()
        };
        let qualified = retrieval::graph::qualify(
            state,
            &mut tx,
            auth,
            brain,
            &mut scope,
            &entities,
            (250, None),
        )
        .await?;
        let eligible: BTreeSet<_> = rows
            .iter()
            .filter(|(id, _, v, _)| {
                current.contains(id) && qualified.nodes.contains_key(&source_key(*v))
            })
            .map(|(i, _, _, _)| *i)
            .collect();
        if eligible.is_empty() || !inputs.iter().all(|i| eligible.contains(i)) {
            if inputs.is_empty() && rows.len() == 250 {
                after = rows.last().map(|r| r.0);
                continue;
            }
            return Err(changed());
        }
        tx.commit().await?;
        let report_deadline = rows.iter().filter_map(|(_, _, _, at)| *at).min();
        return Ok((
            epoch,
            [qualified.deadline, report_deadline]
                .into_iter()
                .flatten()
                .min(),
        ));
    }
}

#[utoipa::path(put,path="/api/brains/{brain}/organization/topics/{id}",operation_id="organizationTopic",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=TopicCommand,responses((status=200,body=TopicCommandResult)))]
pub async fn topic(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<TopicCommand>,
) -> Result<Json<TopicCommandResult>> {
    auth.require_browser()?;
    let key = commands::key(&headers)?
        .ok_or_else(|| Error::invalid("Supply an Idempotency-Key for a topic change."))?;
    let inputs: Vec<Uuid> = match &input.change {
        TopicOverride::Rename { basis_input, .. }
        | TopicOverride::RenameSource { basis_input, .. } => vec![*basis_input],
        TopicOverride::Exclude { input_ids, .. }
        | TopicOverride::Split { input_ids }
        | TopicOverride::ExcludeSources { input_ids, .. }
        | TopicOverride::SplitSources { input_ids } => input_ids.clone(),
        TopicOverride::Pin { .. } => vec![],
    };
    if inputs.len() > 250 || inputs.iter().collect::<BTreeSet<_>>().len() != inputs.len() {
        return Err(Error::invalid("Select at most 250 distinct topic inputs."));
    }
    if matches!(
        &input.change,
        TopicOverride::Split { .. }
            | TopicOverride::Exclude { .. }
            | TopicOverride::SplitSources { .. }
            | TopicOverride::ExcludeSources { .. }
    ) && inputs.is_empty()
    {
        return Err(Error::invalid("Select source memberships to change."));
    }
    let source_change = matches!(
        input.change,
        TopicOverride::RenameSource { .. }
            | TopicOverride::ExcludeSources { .. }
            | TopicOverride::SplitSources { .. }
    );
    let capacity = graph::read_capacity().try_acquire().map_err(|_| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "organization_busy",
            "Organization read capacity is busy.",
        )
    })?;
    let (prepared_epoch, prepared_deadline, prepared_intent) =
        tokio::time::timeout(Duration::from_secs(5), async {
            let (epoch, deadline) = if source_change {
                sources::prepare_change(&state, &auth, brain, id, &inputs, false).await
            } else {
                let prepared = prepare_topic_change(
                    &state,
                    &auth,
                    brain,
                    id,
                    &inputs,
                    matches!(input.change, TopicOverride::Pin { .. }),
                )
                .await;
                match prepared {
                    Err(e)
                        if e.1 == "organization_changed"
                            && matches!(input.change, TopicOverride::Pin { .. }) =>
                    {
                        sources::prepare_change(&state, &auth, brain, id, &[], true).await
                    }
                    other => other,
                }
            }?;
            let intent = super::manual_intent::prepare_capture(
                &state,
                &auth,
                brain,
                id,
                input.base_change_id,
            )
            .await?;
            if intent.as_ref().is_some_and(|i| i.epoch != epoch) {
                return Err(changed());
            }
            if let Some(intent) = &intent {
                super::manual_intent::reserve_command(
                    &state,
                    &auth,
                    brain,
                    id,
                    intent,
                    &input.change,
                )
                .await?;
            }
            let deadline = [deadline, intent.as_ref().and_then(|i| i.deadline)]
                .into_iter()
                .flatten()
                .min();
            Ok((epoch, deadline, intent))
        })
        .await
        .map_err(|_| {
            Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "organization_timeout",
                "Organization preparation exceeded its bounded window.",
            )
        })??;
    drop(capacity);
    tokio::time::timeout(Duration::from_millis(1500), async {
    let mut tx = admission::brain_tx(&state.pool, &auth, brain, BrainAccess::WRITE.with_statement_timeout(Duration::from_millis(750)).with_session_recheck()).await?.tx;
    let deadline_current: bool =
        sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
            .bind(prepared_deadline)
            .fetch_one(&mut *tx)
            .await?;
    if graph::preparation_epoch(&mut tx, brain).await? != prepared_epoch || !deadline_current {
        return Err(changed());
    }
    let payload = json!({"brain_id":brain,"topic_id":id,"command":input});
    if let Some(saved) =
        commands::reserve(&mut tx, Some(&key), "organization.topic", payload).await?
    {
        tx.commit().await?;
        return Ok(Json(saved));
    }
    let base: Option<Uuid> = sqlx::query_scalar(
        "SELECT change_id FROM knowledge_topics WHERE brain_id=$1 AND id=$2 FOR UPDATE",
    )
    .bind(brain)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    if base != Some(input.base_change_id) {
        return Err(changed());
    }
    // Replays have already returned their saved result. A new split may only
    // move currently visible contributors; preparation still qualifies the
    // original contributors so replaying an earlier split stays idempotent.
    if matches!(input.change, TopicOverride::SplitSources { .. }) {
        let visible: i64 = sqlx::query_scalar("SELECT count(*) FROM knowledge_topic_sources m WHERE m.brain_id=$1 AND m.topic_id=$2 AND m.input_id=ANY($3) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions x WHERE x.topic_id=m.topic_id AND x.input_id=m.input_id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_splits x WHERE x.original_topic_id=m.topic_id AND x.input_id=m.input_id)")
            .bind(brain).bind(id).bind(&inputs).fetch_one(&mut *tx).await?;
        if visible != inputs.len() as i64 {
            return Err(changed());
        }
    }
    if let Some(intent) = &prepared_intent {
        super::manual_intent::apply_capture(&mut tx, brain, auth.user.id, intent).await?;
    }
    let mut split_topic_id = None;
    let action = match &input.change {
        TopicOverride::RenameSource { label, basis_input } => {
            let label = label.trim();
            if label.is_empty() || label.len() > 240 || label.chars().any(char::is_control) {
                return Err(Error::invalid(
                    "Use a source-backed topic name of 1 to 240 bytes without control characters.",
                ));
            }
            publication::safe_payload(&state, &json!({"label":label}))?;
            sqlx::query("DELETE FROM knowledge_topic_labels WHERE brain_id=$1 AND topic_id=$2").bind(brain).bind(id).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO knowledge_topic_source_labels(brain_id,topic_id,basis_input,label) VALUES($1,$2,$3,$4) ON CONFLICT(topic_id) DO UPDATE SET basis_input=excluded.basis_input,label=excluded.label").bind(brain).bind(id).bind(basis_input).bind(label).execute(&mut *tx).await?;
            "rename"
        }
        TopicOverride::ExcludeSources { excluded, .. } => {
            if *excluded {
                sqlx::query("INSERT INTO knowledge_topic_source_exclusions(brain_id,topic_id,input_id) SELECT $1,$2,unnest($3::uuid[]) ON CONFLICT DO NOTHING").bind(brain).bind(id).bind(&inputs).execute(&mut *tx).await?;
            } else {
                sqlx::query("DELETE FROM knowledge_topic_source_exclusions WHERE brain_id=$1 AND topic_id=$2 AND input_id=ANY($3)").bind(brain).bind(id).bind(&inputs).execute(&mut *tx).await?;
                // One accepted unexclude clears the same immutable contributor
                // in either representation, without crossing source context.
                sqlx::query("DELETE FROM knowledge_topic_exclusions x USING knowledge_mapping_inputs i JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id JOIN knowledge_source_inputs s ON s.brain_id=i.brain_id AND s.source_version_id=i.source_version_id AND s.context=c.context WHERE x.brain_id=$1 AND x.topic_id=$2 AND x.input_id=i.id AND i.brain_id=x.brain_id AND s.id=ANY($3)")
                    .bind(brain).bind(id).bind(&inputs).execute(&mut *tx).await?;
            }
            "exclude"
        }
        TopicOverride::SplitSources { .. } => {
            let split = Uuid::new_v4();
            let change = Uuid::new_v4();
            sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'split',$3,'human-split-1')").bind(split).bind(brain).bind(change).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT brain_id,$3,input_id,'explicit_source_split',weight FROM knowledge_topic_sources WHERE brain_id=$1 AND topic_id=$2 AND input_id=ANY($4)").bind(brain).bind(id).bind(split).bind(&inputs).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO knowledge_topic_source_splits(brain_id,original_topic_id,split_topic_id,input_id) SELECT $1,$2,$3,unnest($4::uuid[])").bind(brain).bind(id).bind(split).bind(&inputs).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) VALUES($1,$2,$3,$4,'create')").bind(change).bind(brain).bind(split).bind(auth.user.id).execute(&mut *tx).await?;
            split_topic_id = Some(split);
            "split"
        }
        TopicOverride::Rename { label, basis_input } => {
            let label = label.trim();
            if label.is_empty() || label.len() > 256 || label.chars().any(char::is_control) {
                return Err(Error::invalid(
                    "Use a topic name of 1 to 256 bytes without control characters.",
                ));
            }
            publication::safe_payload(&state, &json!({"label":label}))?;
            sqlx::query("DELETE FROM knowledge_topic_source_labels WHERE brain_id=$1 AND topic_id=$2").bind(brain).bind(id).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO knowledge_topic_labels(brain_id,topic_id,basis_input,label,actor_id) VALUES($1,$2,$3,$4,$5) ON CONFLICT(topic_id) DO UPDATE SET basis_input=excluded.basis_input,label=excluded.label,actor_id=excluded.actor_id")
                .bind(brain).bind(id).bind(basis_input).bind(label).bind(auth.user.id).execute(&mut *tx).await?;
            if prepared_intent.is_some() {
                super::manual_intent::copy_legacy_label(&mut tx, brain, id, true).await?;
            }
            "rename"
        }
        TopicOverride::Pin { pinned } => {
            sqlx::query("UPDATE knowledge_topics SET pinned=$3 WHERE brain_id=$1 AND id=$2")
                .bind(brain)
                .bind(id)
                .bind(pinned)
                .execute(&mut *tx)
                .await?;
            "pin"
        }
        TopicOverride::Exclude { excluded, .. } => {
            if *excluded {
                sqlx::query("INSERT INTO knowledge_topic_exclusions(brain_id,topic_id,input_id,actor_id) SELECT $1,$2,unnest($3::uuid[]),$4 ON CONFLICT DO NOTHING")
                    .bind(brain).bind(id).bind(&inputs).bind(auth.user.id).execute(&mut *tx).await?;
                super::manual_intent::copy_legacy_controls(&mut tx, brain, id).await?;
            } else {
                sqlx::query("DELETE FROM knowledge_topic_exclusions x USING knowledge_mapping_inputs i JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id JOIN knowledge_mapping_inputs selected ON selected.brain_id=i.brain_id AND selected.source_version_id=i.source_version_id JOIN knowledge_mapping_contexts selected_context ON selected_context.brain_id=selected.brain_id AND selected_context.id=selected.context_id AND selected_context.context=c.context WHERE x.brain_id=$1 AND x.topic_id=$2 AND x.input_id=i.id AND i.brain_id=x.brain_id AND selected.id=ANY($3)")
                    .bind(brain).bind(id).bind(&inputs).execute(&mut *tx).await?;
                sqlx::query("DELETE FROM knowledge_topic_source_exclusions x USING knowledge_source_inputs s JOIN knowledge_mapping_inputs i ON i.brain_id=s.brain_id AND i.source_version_id=s.source_version_id JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id AND c.context=s.context WHERE x.brain_id=$1 AND x.topic_id=$2 AND x.input_id=s.id AND s.brain_id=x.brain_id AND i.id=ANY($3)")
                    .bind(brain).bind(id).bind(&inputs).execute(&mut *tx).await?;
                if prepared_intent.is_some() {
                    super::manual_intent::copy_legacy_label(&mut tx, brain, id, false).await?;
                }
            }
            "exclude"
        }
        TopicOverride::Split { .. } => {
            let split = Uuid::new_v4();
            let change = Uuid::new_v4();
            sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES($1,$2,'split',$3,'human-split-1')")
                .bind(split).bind(brain).bind(change).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) SELECT brain_id,$3,entity_id,input_id,'explicit_split',weight FROM knowledge_topic_memberships WHERE brain_id=$1 AND topic_id=$2 AND input_id=ANY($4)")
                .bind(brain).bind(id).bind(split).bind(&inputs).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO knowledge_topic_splits(brain_id,original_topic_id,split_topic_id,input_id,actor_id) SELECT $1,$2,$3,unnest($4::uuid[]),$5")
                .bind(brain).bind(id).bind(split).bind(&inputs).bind(auth.user.id).execute(&mut *tx).await?;
            super::manual_intent::copy_legacy_controls(&mut tx, brain, id).await?;
            sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) VALUES($1,$2,$3,$4,'create')")
                .bind(change).bind(brain).bind(split).bind(auth.user.id).execute(&mut *tx).await?;
            split_topic_id = Some(split);
            "split"
        }
    };
    let change_id = Uuid::new_v4();
    sqlx::query("UPDATE knowledge_topics SET change_id=$3 WHERE brain_id=$1 AND id=$2")
        .bind(brain)
        .bind(id)
        .bind(change_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action,previous_change_id) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(change_id).bind(brain).bind(id).bind(auth.user.id).bind(action).bind(input.base_change_id).execute(&mut *tx).await?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "organization.topic",
        id,
        action,
    )
    .await?;
    sqlx::query("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1")
        .bind(brain)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT recollect_organization_override_changed($1)")
        .bind(brain)
        .execute(&mut *tx)
        .await?;
    let result = TopicCommandResult {
        id,
        change_id,
        split_topic_id,
    };
    commands::finish(&mut tx, Some(&key), brain, &result).await?;
    admission::admit_in(&mut tx, brain, &BrainAccess::WRITE).await?;
    let retained:bool=sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()").bind(prepared_deadline).fetch_one(&mut *tx).await?;
    if !retained { return Err(changed()); }
    tx.commit().await?;
    Ok(Json(result))
    }).await.map_err(|_|Error(StatusCode::SERVICE_UNAVAILABLE,"organization_timeout","Organization publication exceeded its bounded window."))?
}

/// Called only after the production worker checks origin, policy, source,
/// current qualification and lease. Facets are a transparent overlapping
/// baseline, separate from the weighted native community recipe.
pub(crate) async fn publish_input(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    input: Uuid,
) -> Result<()> {
    let updated=sqlx::query("UPDATE knowledge_mapping_inputs SET state='published' WHERE brain_id=$1 AND id=$2 AND producer_request_id IS NOT NULL AND producer_policy_id IS NOT NULL AND producer_change_id IS NOT NULL AND recollect_mapping_origin_current(brain_id,id)")
        .bind(brain).bind(input).execute(&mut **tx).await?;
    if updated.rows_affected() != 1 {
        return Err(changed());
    }
    sqlx::query("UPDATE knowledge_mapping_origins SET published_at=coalesce(published_at,clock_timestamp()) WHERE brain_id=$1 AND input_id=$2")
        .bind(brain).bind(input).execute(&mut **tx).await?;
    sqlx::query("WITH added AS (
        INSERT INTO knowledge_topics(id,brain_id,kind,anchor_entity_id,change_id,recipe)
        SELECT gen_random_uuid(),$1,'facet',e.id,gen_random_uuid(),'entity-facet-1'
        FROM knowledge_entities e WHERE e.brain_id=$1 AND e.kind IN ('technology','concept') AND EXISTS(
         SELECT 1 FROM knowledge_mentions m WHERE m.brain_id=$1 AND m.input_id=$2 AND m.entity_id=e.id)
        ON CONFLICT(brain_id,anchor_entity_id) WHERE kind='facet' DO NOTHING RETURNING id,change_id)
        INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT change_id,$1,id,$3,'create' FROM added")
        .bind(brain).bind(input).bind(actor).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight)
        SELECT DISTINCT $1,t.id,m.entity_id,$2,'entity_facet',1.0
        FROM knowledge_mentions m JOIN knowledge_topics t ON t.brain_id=m.brain_id AND t.anchor_entity_id=m.entity_id AND t.kind='facet'
        WHERE m.brain_id=$1 AND m.input_id=$2
        ON CONFLICT DO NOTHING")
        .bind(brain).bind(input).execute(&mut **tx).await?;
    db::audit(
        tx,
        actor,
        brain,
        "knowledge.mapping.publish",
        input,
        "qualified_entity_facets",
    )
    .await?;
    sqlx::query("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1")
        .bind(brain)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
