//! Human-adopted cohorts outlive provisional machine calculations. Every use
//! still qualifies their exact immutable sources, contexts and authority.
use super::source_features::{self, Features, SourceInput};
use crate::{
    auth::Auth,
    db,
    error::{Error, Result},
    graph,
    memory_evidence::Tx,
    AppState,
};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use recollect_protocol::{GraphSelection, TopicOverride};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::types::Json;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const INPUTS: usize = 10_000;
const ROWS: usize = 50_000;
const BYTES: usize = 8 * 1024 * 1024;

fn changed() -> Error {
    Error(
        StatusCode::CONFLICT,
        "organization_changed",
        "The organization inputs changed; prepare the command again.",
    )
}
fn capacity() -> Error {
    Error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "organization_overlay_capacity",
        "The complete human organization intent exceeds its bounded preparation envelope.",
    )
}
fn conflict() -> Error {
    Error(
        StatusCode::CONFLICT,
        "organization_intent_conflict",
        "Different legacy split destinations share one immutable source context; preserve the existing intent before replacing its inputs.",
    )
}

#[derive(Clone, Serialize)]
struct Topic {
    id: Uuid,
    change: Uuid,
    kind: String,
}
#[derive(Clone, Serialize)]
struct Mapping {
    topic: Uuid,
    input: Uuid,
    entity: Uuid,
}
#[derive(Clone, Serialize)]
struct Member {
    topic: Uuid,
    input: Uuid,
}
#[derive(Clone)]
struct RawMapping {
    topic: Uuid,
    input: Uuid,
    entity: Uuid,
    source: Uuid,
    version: Uuid,
    context: Value,
}
#[derive(Clone)]
struct RawSource {
    topic: Uuid,
    input: Uuid,
    source: Uuid,
    version: Uuid,
    context: Value,
}

/// Opaque preparation; IDs in `sources` can be provisional until atomic apply.
#[derive(Clone, Serialize)]
pub struct Intent {
    pub epoch: i64,
    pub deadline: Option<DateTime<Utc>>,
    topics: Vec<Topic>,
    sources: Vec<SourceInput>,
    members: Vec<Member>,
    mappings: Vec<Mapping>,
    control_rows: usize,
}
impl Intent {
    pub fn is_overlay(&self, topic: Uuid) -> bool {
        self.topics
            .iter()
            .any(|t| t.id == topic && t.kind == "manual_overlay")
    }
    pub fn topic_ids(&self) -> Vec<Uuid> {
        self.topics.iter().map(|t| t.id).collect()
    }
    pub fn versions(&self) -> BTreeMap<Uuid, BTreeSet<Uuid>> {
        let versions: BTreeMap<_, _> = self.sources.iter().map(|s| (s.id, s.version)).collect();
        let mut out = BTreeMap::<Uuid, BTreeSet<Uuid>>::new();
        for m in &self.members {
            out.entry(m.topic).or_default().insert(versions[&m.input]);
        }
        out
    }
    pub fn source_ids(&self) -> BTreeMap<Uuid, BTreeSet<Uuid>> {
        let mut out = BTreeMap::<Uuid, BTreeSet<Uuid>>::new();
        for m in &self.members {
            out.entry(m.topic).or_default().insert(m.input);
        }
        out
    }
    fn bounded(&self) -> Result<()> {
        let split_topics: BTreeSet<_> = self
            .topics
            .iter()
            .filter(|t| t.kind == "split")
            .map(|t| t.id)
            .collect();
        if self.topics.len() > INPUTS
            || self
                .sources
                .iter()
                .map(|s| s.id)
                .chain(self.mappings.iter().map(|m| m.input))
                .collect::<BTreeSet<_>>()
                .len()
                > INPUTS
            || 2 * (self
                .members
                .iter()
                .filter(|m| !split_topics.contains(&m.topic))
                .count()
                + self
                    .mappings
                    .iter()
                    .filter(|m| !split_topics.contains(&m.topic))
                    .count())
                + self.control_rows
                > ROWS
            || serde_json::to_vec(self).map_err(|_| capacity())?.len() > BYTES
        {
            return Err(capacity());
        }
        Ok(())
    }
    fn retain_topics(&mut self, ids: &BTreeSet<Uuid>) {
        self.topics.retain(|t| ids.contains(&t.id));
        self.members.retain(|m| ids.contains(&m.topic));
        self.mappings.retain(|m| ids.contains(&m.topic));
        let used: BTreeSet<_> = self.members.iter().map(|m| m.input).collect();
        self.sources.retain(|s| used.contains(&s.id));
    }
}

async fn read<'a>(state: &'a AppState, auth: &Auth, brain: Uuid) -> Result<Tx<'a>> {
    let mut tx = db::preparation_tx(&state.pool, auth.user.id, auth.device_id).await?;
    if !matches!(
        db::require_role(&mut tx, brain, false).await?.as_str(),
        "writer" | "admin"
    ) {
        return Err(Error::forbidden());
    }
    sqlx::query("SET LOCAL statement_timeout='4000ms'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL jit=off").execute(&mut *tx).await?;
    sqlx::query("SET LOCAL recollect.bulk_qualification=on")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

async fn topics(tx: &mut Tx<'_>, brain: Uuid, selected: Option<Uuid>) -> Result<Vec<Topic>> {
    let rows:Vec<(Uuid,Uuid,String)>=sqlx::query_as("SELECT t.id,t.change_id,t.kind FROM knowledge_topics t WHERE t.brain_id=$1 AND ($2::uuid IS NULL OR t.id=$2) AND (t.kind IN('manual_overlay','split') OR(t.kind='community' AND ($2::uuid IS NOT NULL OR t.pinned OR EXISTS(SELECT 1 FROM knowledge_topic_labels x WHERE x.topic_id=t.id) OR EXISTS(SELECT 1 FROM knowledge_topic_source_labels x WHERE x.topic_id=t.id) OR EXISTS(SELECT 1 FROM knowledge_topic_exclusions x WHERE x.topic_id=t.id) OR EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions x WHERE x.topic_id=t.id) OR EXISTS(SELECT 1 FROM knowledge_topic_splits x WHERE x.original_topic_id=t.id) OR EXISTS(SELECT 1 FROM knowledge_topic_source_splits x WHERE x.original_topic_id=t.id) OR EXISTS(SELECT 1 FROM knowledge_topic_intent_sources x WHERE x.topic_id=t.id) OR EXISTS(SELECT 1 FROM knowledge_topic_intent_mappings x WHERE x.topic_id=t.id)))) ORDER BY t.id LIMIT 10001")
        .bind(brain).bind(selected).fetch_all(&mut **tx).await?;
    if rows.len() > INPUTS {
        return Err(capacity());
    }
    Ok(rows
        .into_iter()
        .map(|(id, change, kind)| Topic { id, change, kind })
        .collect())
}

const RAW_MAPPINGS: &str = "WITH members AS(SELECT topic_id,input_id,entity_id FROM knowledge_topic_memberships WHERE brain_id=$1 AND topic_id=ANY($2) UNION SELECT topic_id,input_id,entity_id FROM knowledge_topic_intent_mappings WHERE brain_id=$1 AND topic_id=ANY($2)) SELECT m.topic_id,m.input_id,m.entity_id,v.source_id,v.id,c.context FROM members m JOIN knowledge_mapping_inputs i ON i.brain_id=$1 AND i.id=m.input_id JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.source_version_id JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id WHERE EXISTS(SELECT 1 FROM knowledge_mentions n WHERE n.brain_id=i.brain_id AND n.input_id=i.id AND n.entity_id=m.entity_id) ORDER BY m.topic_id,m.input_id,m.entity_id LIMIT 50001";
const RAW_SOURCES: &str = "WITH members AS(SELECT topic_id,input_id FROM knowledge_topic_sources WHERE brain_id=$1 AND topic_id=ANY($2) UNION SELECT topic_id,input_id FROM knowledge_topic_intent_sources WHERE brain_id=$1 AND topic_id=ANY($2)) SELECT m.topic_id,i.id,i.source_id,i.source_version_id,i.context FROM members m JOIN knowledge_source_inputs i ON i.brain_id=$1 AND i.id=m.input_id ORDER BY m.topic_id,i.id LIMIT 50001";

// Reject oversized intermediate cohorts before PostgreSQL sends any repeated
// contexts to Rust. Conservative per-row IDs/JSON overhead is intentional;
// preparation capacity protects both the stored result and transient buffers.
async fn context_budget(
    tx: &mut Tx<'_>,
    brain: Uuid,
    ids: &[Uuid],
    query: &str,
) -> Result<(usize, usize)> {
    let (rows, bytes):(i64,i64)=sqlx::query_as(&format!("SELECT count(*),coalesce(sum(octet_length(context::text)::bigint+256),0)::bigint FROM ({query}) prepared"))
        .bind(brain).bind(ids).fetch_one(&mut **tx).await?;
    if rows as usize > ROWS || bytes as usize > BYTES {
        return Err(capacity());
    }
    Ok((rows as usize, bytes as usize))
}

async fn retained_scope(tx: &mut Tx<'_>, brain: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>> {
    let scope:Vec<Uuid>=sqlx::query_scalar("WITH RECURSIVE links AS MATERIALIZED(SELECT original_topic_id parent,split_topic_id child FROM knowledge_topic_splits WHERE brain_id=$1 UNION SELECT original_topic_id,split_topic_id FROM knowledge_topic_source_splits WHERE brain_id=$1), retained(id) AS(SELECT unnest($2::uuid[]) UNION SELECT l.child FROM retained r JOIN links l ON l.parent=r.id) SELECT id FROM retained LIMIT 10001")
        .bind(brain).bind(ids).fetch_all(&mut **tx).await?;
    if scope.len() > INPUTS {
        return Err(capacity());
    }
    Ok(scope)
}
async fn retained_control_rows(tx: &mut Tx<'_>, brain: Uuid, ids: &[Uuid]) -> Result<usize> {
    let scope = retained_scope(tx, brain, ids).await?;
    let originals: BTreeSet<_> = ids.iter().copied().collect();
    // Split topics keep their own stored memberships. They are capacity roots,
    // not automatic communities whose entire cohort is restored twice. Count
    // their physical membership/intent rows once, including arbitrary depth.
    let split_topics:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM knowledge_topics WHERE brain_id=$1 AND id=ANY($2) AND kind='split' LIMIT 10001")
        .bind(brain).bind(&scope).fetch_all(&mut **tx).await?;
    let split_topics: BTreeSet<_> = split_topics.into_iter().collect();
    let children: Vec<_> = scope
        .iter()
        .copied()
        .filter(|id| !originals.contains(id) || split_topics.contains(id))
        .collect();
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM(SELECT 1 FROM knowledge_topic_labels WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 1 FROM knowledge_topic_source_labels WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 1 FROM knowledge_topic_exclusions WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 1 FROM knowledge_topic_source_exclusions WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 1 FROM knowledge_topic_splits WHERE brain_id=$1 AND original_topic_id=ANY($2) UNION ALL SELECT 1 FROM knowledge_topic_source_splits WHERE brain_id=$1 AND original_topic_id=ANY($2) UNION ALL SELECT 1 FROM knowledge_topic_sources WHERE brain_id=$1 AND topic_id=ANY($3) UNION ALL SELECT 1 FROM knowledge_topic_memberships WHERE brain_id=$1 AND topic_id=ANY($3) UNION ALL SELECT 1 FROM knowledge_topic_intent_sources WHERE brain_id=$1 AND topic_id=ANY($3) UNION ALL SELECT 1 FROM knowledge_topic_intent_mappings WHERE brain_id=$1 AND topic_id=ANY($3) LIMIT 50001)x")
        .bind(brain).bind(scope).bind(children).fetch_one(&mut **tx).await?;
    if count as usize > ROWS {
        return Err(capacity());
    }
    Ok(count as usize)
}

async fn raw(
    tx: &mut Tx<'_>,
    brain: Uuid,
    ids: &[Uuid],
) -> Result<(Vec<RawMapping>, Vec<RawSource>)> {
    split_conflict(tx, brain, ids).await?;
    let scope = retained_scope(tx, brain, ids).await?;
    let (mapping_rows, mapping_bytes) = context_budget(tx, brain, &scope, RAW_MAPPINGS).await?;
    let (source_rows, source_bytes) = context_budget(tx, brain, &scope, RAW_SOURCES).await?;
    if mapping_rows + source_rows > ROWS || mapping_bytes + source_bytes > BYTES {
        return Err(capacity());
    }
    let input_count:i64=sqlx::query_scalar(&format!("SELECT count(*) FROM (SELECT input_id FROM ({RAW_MAPPINGS}) m UNION SELECT id FROM ({RAW_SOURCES}) s UNION SELECT basis_input FROM knowledge_topic_labels WHERE brain_id=$1 AND topic_id=ANY($2) UNION SELECT basis_input FROM knowledge_topic_source_labels WHERE brain_id=$1 AND topic_id=ANY($2) UNION SELECT input_id FROM knowledge_topic_exclusions WHERE brain_id=$1 AND topic_id=ANY($2) UNION SELECT input_id FROM knowledge_topic_source_exclusions WHERE brain_id=$1 AND topic_id=ANY($2) UNION SELECT input_id FROM knowledge_topic_splits WHERE brain_id=$1 AND original_topic_id=ANY($2) UNION SELECT input_id FROM knowledge_topic_source_splits WHERE brain_id=$1 AND original_topic_id=ANY($2) LIMIT 10001) retained"))
        .bind(brain).bind(&scope).fetch_one(&mut **tx).await?;
    if input_count as usize > INPUTS {
        return Err(capacity());
    }
    let mappings: Vec<(Uuid, Uuid, Uuid, Uuid, Uuid, Json<Value>)> = sqlx::query_as(RAW_MAPPINGS)
        .bind(brain)
        .bind(ids)
        .fetch_all(&mut **tx)
        .await?;
    let sources: Vec<(Uuid, Uuid, Uuid, Uuid, Json<Value>)> = sqlx::query_as(RAW_SOURCES)
        .bind(brain)
        .bind(ids)
        .fetch_all(&mut **tx)
        .await?;
    let controls = retained_control_rows(tx, brain, ids).await?;
    let split_ids:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM knowledge_topics WHERE brain_id=$1 AND id=ANY($2) AND kind='split' LIMIT 10001")
        .bind(brain).bind(ids).fetch_all(&mut **tx).await?;
    let split_ids: BTreeSet<_> = split_ids.into_iter().collect();
    if mappings
        .iter()
        .filter(|m| !split_ids.contains(&m.0))
        .count()
        + sources.iter().filter(|m| !split_ids.contains(&m.0)).count()
        + controls
        > ROWS
    {
        return Err(capacity());
    }
    Ok((
        mappings
            .into_iter()
            .map(
                |(topic, input, entity, source, version, Json(context))| RawMapping {
                    topic,
                    input,
                    entity,
                    source,
                    version,
                    context,
                },
            )
            .collect(),
        sources
            .into_iter()
            .map(|(topic, input, source, version, Json(context))| RawSource {
                topic,
                input,
                source,
                version,
                context,
            })
            .collect(),
    ))
}

async fn qualify(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    topics: Vec<Topic>,
    raw_mapping: Vec<RawMapping>,
    raw_sources: Vec<RawSource>,
    capture_split: Option<Uuid>,
) -> Result<Intent> {
    let epoch = graph::preparation_epoch(tx, brain).await?;
    let mapping_ids: Vec<_> = raw_mapping
        .iter()
        .map(|r| r.input)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let raw_inputs: BTreeSet<_> = mapping_ids
        .iter()
        .copied()
        .chain(raw_sources.iter().map(|r| r.input))
        .collect();
    if raw_inputs.len() > INPUTS {
        return Err(capacity());
    }
    let mut eligible_mapping = BTreeSet::new();
    for page in mapping_ids.chunks(INPUTS) {
        let ids: Vec<Uuid> =
            sqlx::query_scalar("SELECT input_id FROM recollect_mapping_current_input_ids($1,$2)")
                .bind(brain)
                .bind(page)
                .fetch_all(&mut **tx)
                .await?;
        eligible_mapping.extend(ids);
    }
    let source_ids: Vec<_> = raw_sources
        .iter()
        .map(|r| r.input)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut eligible_sources = BTreeSet::new();
    for page in source_ids.chunks(250) {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT input_id FROM recollect_organization_current_source_ids($1,$2)",
        )
        .bind(brain)
        .bind(page)
        .fetch_all(&mut **tx)
        .await?;
        eligible_sources.extend(ids);
    }
    let mut raw_mapping: Vec<_> = raw_mapping
        .into_iter()
        .filter(|r| eligible_mapping.contains(&r.input))
        .collect();
    let mut raw_sources: Vec<_> = raw_sources
        .into_iter()
        .filter(|r| eligible_sources.contains(&r.input))
        .collect();
    let versions: BTreeMap<_, _> = raw_mapping
        .iter()
        .map(|r| (r.version, r.source))
        .chain(raw_sources.iter().map(|r| (r.version, r.source)))
        .collect();
    if versions.len() > INPUTS {
        return Err(capacity());
    }
    let mut scope: GraphSelection =
        serde_json::from_value(json!({"kind":"knowledge"})).map_err(|_| changed())?;
    crate::retrieval::graph::authorize(tx, auth, brain, &scope).await?;
    let at: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    let mut valid = BTreeSet::new();
    let mut deadline:Option<DateTime<Utc>>=sqlx::query_scalar("SELECT least(CASE WHEN a.auth_kind<>'local' THEN a.membership_until END,d.expires_at) FROM accounts a LEFT JOIN devices d ON d.id=$2 AND d.account_id=a.id WHERE a.id=$1")
        .bind(auth.user.id).bind(auth.device_id).fetch_one(&mut **tx).await?;
    let entities: Vec<_> = versions
        .iter()
        .map(|(version, source)| graph::Entity {
            kind: "source_version".into(),
            id: *source,
            revision_id: *version,
        })
        .collect();
    for page in entities.chunks(250) {
        let q = crate::retrieval::graph::qualify(
            state,
            tx,
            auth,
            brain,
            &mut scope,
            page,
            (250, Some(at)),
        )
        .await?;
        for e in page {
            if q.nodes.contains_key(&e.key()) {
                valid.insert(e.revision_id);
            }
        }
        deadline = [deadline, q.deadline].into_iter().flatten().min();
    }
    let contexts:Vec<(Uuid,Json<Value>)>=sqlx::query_as("SELECT v.id,recollect_organization_source_context(v.brain_id,v.id,v.source_id) FROM source_versions v WHERE v.brain_id=$1 AND v.id=ANY($2)")
        .bind(brain).bind(valid.iter().copied().collect::<Vec<_>>()).fetch_all(&mut **tx).await?;
    let contexts: BTreeMap<_, _> = contexts.into_iter().map(|(v, Json(c))| (v, c)).collect();
    raw_mapping.retain(|r| contexts.get(&r.version) == Some(&r.context));
    raw_sources.retain(|r| contexts.get(&r.version) == Some(&r.context));
    // Qualification and global bounds cover every manual split root, but only
    // the selected human edit adopts a new split snapshot. Do not manufacture
    // provisional source IDs or duplicate cohorts for unrelated split groups.
    let adopted: BTreeSet<_> = topics
        .iter()
        .filter(|t| t.kind != "split" || Some(t.id) == capture_split)
        .map(|t| t.id)
        .collect();
    raw_mapping.retain(|r| adopted.contains(&r.topic));
    raw_sources.retain(|r| adopted.contains(&r.topic));
    let existing_query = "SELECT id,source_version_id,context FROM knowledge_source_inputs WHERE brain_id=$1 AND source_version_id=ANY($2) ORDER BY id LIMIT 10001";
    let (existing_rows, _) = context_budget(
        tx,
        brain,
        &valid.iter().copied().collect::<Vec<_>>(),
        existing_query,
    )
    .await?;
    if existing_rows > INPUTS {
        return Err(capacity());
    }
    let existing:Vec<(Uuid,Uuid,Json<Value>)>=sqlx::query_as("SELECT id,source_version_id,context FROM knowledge_source_inputs WHERE brain_id=$1 AND source_version_id=ANY($2) ORDER BY id LIMIT 10001")
        .bind(brain).bind(valid.iter().copied().collect::<Vec<_>>()).fetch_all(&mut **tx).await?;
    if existing.len() > INPUTS {
        return Err(capacity());
    }
    let mut chosen = BTreeMap::<Uuid, SourceInput>::new();
    for r in &raw_sources {
        chosen.entry(r.version).or_insert(SourceInput {
            id: r.input,
            source: r.source,
            version: r.version,
            context: r.context.clone(),
        });
    }
    for r in &raw_mapping {
        chosen.entry(r.version).or_insert_with(|| SourceInput {
            id: existing
                .iter()
                .find(|(_, v, c)| *v == r.version && c.0 == r.context)
                .map(|r| r.0)
                .unwrap_or_else(Uuid::new_v4),
            source: r.source,
            version: r.version,
            context: r.context.clone(),
        });
    }
    let members: BTreeSet<_> = raw_sources
        .iter()
        .map(|r| (r.topic, chosen[&r.version].id))
        .chain(raw_mapping.iter().map(|r| (r.topic, chosen[&r.version].id)))
        .collect();
    let control_rows =
        retained_control_rows(tx, brain, &topics.iter().map(|t| t.id).collect::<Vec<_>>()).await?;
    let out = Intent {
        epoch,
        deadline,
        topics,
        sources: chosen.into_values().collect(),
        members: members
            .into_iter()
            .map(|(topic, input)| Member { topic, input })
            .collect(),
        control_rows,
        mappings: raw_mapping
            .into_iter()
            .map(|r| Mapping {
                topic: r.topic,
                input: r.input,
                entity: r.entity,
            })
            .collect(),
    };
    out.bounded()?;
    Ok(out)
}

/// Stale bases return no new capture so exact command replays still reach the
/// command receipt. New stale commands are rejected by the existing writer.
pub async fn prepare_capture(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    topic: Uuid,
    base_change: Uuid,
) -> Result<Option<Intent>> {
    let mut tx = read(state, auth, brain).await?;
    let selected = topics(&mut tx, brain, Some(topic)).await?;
    if selected.first().is_none_or(|t| t.change != base_change) {
        return Ok(None);
    }
    // The aggregate bound includes already saved human intent, so repeated
    // individually small edits cannot grow an unpublishable hidden cohort.
    let mut all = topics(&mut tx, brain, None).await?;
    if !all.iter().any(|t| t.id == topic) {
        all.extend(selected);
        all.sort_by_key(|t| t.id);
    }
    let ids: Vec<_> = all.iter().map(|t| t.id).collect();
    let (m, s) = raw(&mut tx, brain, &ids).await?;
    let mut out = qualify(state, &mut tx, auth, brain, all, m, s, Some(topic)).await?;
    out.retain_topics(&BTreeSet::from([topic]));
    tx.commit().await?;
    Ok(Some(out))
}

// kind, topic, input/basis, entity, split destination. These are content-free
// prospective rows, including already retained child memberships.
type PlanRow = (String, Uuid, Option<Uuid>, Option<Uuid>, Option<Uuid>);
fn row(kind: &str, topic: Uuid, input: Uuid, entity: Option<Uuid>, split: Option<Uuid>) -> PlanRow {
    (kind.into(), topic, Some(input), entity, split)
}

/// Plan the entire post-command representation before entering the writer.
/// Set arithmetic counts existing rows once and includes exact split children,
/// rather than reserving a fixed number of inputs while missing their entities.
pub async fn reserve_command(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    topic: Uuid,
    intent: &Intent,
    change: &TopicOverride,
) -> Result<()> {
    let mut tx = read(state, auth, brain).await?;
    if graph::preparation_epoch(&mut tx, brain).await? != intent.epoch {
        return Err(changed());
    }
    let mut ids: Vec<_> = topics(&mut tx, brain, None)
        .await?
        .into_iter()
        .map(|t| t.id)
        .collect();
    if !ids.contains(&topic) {
        ids.push(topic);
    }
    let scope = retained_scope(&mut tx, brain, &ids).await?;
    let rows:Vec<PlanRow>=sqlx::query_as("SELECT 'm',topic_id,input_id,entity_id,NULL::uuid FROM knowledge_topic_memberships WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 's',topic_id,input_id,NULL,NULL FROM knowledge_topic_sources WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 'im',topic_id,input_id,entity_id,NULL FROM knowledge_topic_intent_mappings WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 'is',topic_id,input_id,NULL,NULL FROM knowledge_topic_intent_sources WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 'l',topic_id,basis_input,NULL,NULL FROM knowledge_topic_labels WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 'sl',topic_id,basis_input,NULL,NULL FROM knowledge_topic_source_labels WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 'e',topic_id,input_id,NULL,NULL FROM knowledge_topic_exclusions WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 'se',topic_id,input_id,NULL,NULL FROM knowledge_topic_source_exclusions WHERE brain_id=$1 AND topic_id=ANY($2) UNION ALL SELECT 'x',original_topic_id,input_id,NULL,split_topic_id FROM knowledge_topic_splits WHERE brain_id=$1 AND original_topic_id=ANY($2) UNION ALL SELECT 'sx',original_topic_id,input_id,NULL,split_topic_id FROM knowledge_topic_source_splits WHERE brain_id=$1 AND original_topic_id=ANY($2) LIMIT 50001")
        .bind(brain).bind(&scope).fetch_all(&mut *tx).await?;
    if rows.len() > ROWS || scope.len() > INPUTS {
        return Err(capacity());
    }
    let mut post: BTreeSet<_> = rows.into_iter().collect();
    let mapping_ids: Vec<_> = post
        .iter()
        .filter(|r| matches!(r.0.as_str(), "m" | "im" | "l" | "e" | "x"))
        .filter_map(|r| r.2)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let current_inputs: BTreeSet<_> = post.iter().filter_map(|r| r.2).collect();
    if current_inputs.len() > INPUTS {
        return Err(capacity());
    }
    let meta_query = "SELECT i.id,i.source_version_id,c.context FROM knowledge_mapping_inputs i JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id WHERE i.brain_id=$1 AND i.id=ANY($2) ORDER BY i.id LIMIT 10001";
    let (_, meta_bytes) = context_budget(&mut tx, brain, &mapping_ids, meta_query).await?;
    if meta_bytes + serde_json::to_vec(intent).map_err(|_| capacity())?.len() > BYTES {
        return Err(capacity());
    }
    let meta:Vec<(Uuid,Uuid,Json<Value>)>=sqlx::query_as("SELECT i.id,i.source_version_id,c.context FROM knowledge_mapping_inputs i JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id WHERE i.brain_id=$1 AND i.id=ANY($2)")
        .bind(brain).bind(mapping_ids).fetch_all(&mut *tx).await?;
    let map_source: BTreeMap<_, _> = meta
        .into_iter()
        .filter_map(|(input, version, Json(context))| {
            intent
                .sources
                .iter()
                .find(|s| s.version == version && s.context == context)
                .map(|s| (input, s.id))
        })
        .collect();
    post.retain(|r| r.1 != topic || !matches!(r.0.as_str(), "is" | "im"));
    for m in &intent.members {
        post.insert(row("is", m.topic, m.input, None, None));
    }
    for m in &intent.mappings {
        post.insert(row("im", m.topic, m.input, Some(m.entity), None));
    }
    let future = Uuid::new_v4();
    let mut new_topic = false;
    match change {
        TopicOverride::Rename { basis_input, .. } => {
            post.retain(|r| r.1 != topic || r.0 != "l");
            post.insert(row("l", topic, *basis_input, None, None));
            if let Some(source) = map_source.get(basis_input) {
                post.retain(|r| r.1 != topic || r.0 != "sl");
                post.insert(row("sl", topic, *source, None, None));
            }
        }
        TopicOverride::RenameSource { basis_input, .. } => {
            post.retain(|r| r.1 != topic || r.0 != "sl");
            post.insert(row("sl", topic, *basis_input, None, None));
        }
        TopicOverride::Pin { .. } => {}
        TopicOverride::Exclude {
            input_ids,
            excluded,
        } => {
            for input in input_ids {
                let source = map_source.get(input);
                if *excluded {
                    post.insert(row("e", topic, *input, None, None));
                    if let Some(s) = source {
                        post.insert(row("se", topic, *s, None, None));
                    }
                } else {
                    post.remove(&row("e", topic, *input, None, None));
                    if let Some(s) = source {
                        post.remove(&row("se", topic, *s, None, None));
                        post.retain(|r| {
                            !(r.1 == topic
                                && r.0 == "e"
                                && r.2.and_then(|i| map_source.get(&i)) == Some(s))
                        });
                    }
                }
            }
        }
        TopicOverride::ExcludeSources {
            input_ids,
            excluded,
        } => {
            for input in input_ids {
                if *excluded {
                    post.insert(row("se", topic, *input, None, None));
                } else {
                    post.remove(&row("se", topic, *input, None, None));
                    post.retain(|r| {
                        !(r.1 == topic
                            && r.0 == "e"
                            && r.2.and_then(|i| map_source.get(&i)) == Some(input))
                    });
                }
            }
        }
        TopicOverride::Split { input_ids } => {
            new_topic = true;
            let members: Vec<_> = post
                .iter()
                .filter(|r| {
                    r.0 == "m" && r.1 == topic && r.2.is_some_and(|i| input_ids.contains(&i))
                })
                .cloned()
                .collect();
            for mut m in members {
                m.1 = future;
                post.insert(m);
            }
            for input in input_ids {
                post.insert(row("x", topic, *input, None, Some(future)));
            }
        }
        TopicOverride::SplitSources { input_ids } => {
            new_topic = true;
            for input in input_ids {
                post.insert(row("sx", topic, *input, None, Some(future)));
                post.insert(row("s", future, *input, None, None));
            }
        }
    }
    // Bridge both pre-existing and pending exact legacy controls. A second
    // destination for one canonical source is a conflict, never first-wins.
    let legacy: Vec<_> = post
        .iter()
        .filter(|r| r.1 == topic && matches!(r.0.as_str(), "e" | "x"))
        .cloned()
        .collect();
    for r in legacy {
        if let Some(source) = r.2.and_then(|i| map_source.get(&i)) {
            if r.0 == "e" {
                post.insert(row("se", topic, *source, None, None));
            } else {
                post.insert(row("sx", topic, *source, None, r.4));
                if let Some(child) = r.4 {
                    post.insert(row("s", child, *source, None, None));
                }
            }
        }
    }
    let mut destinations = BTreeMap::<Uuid, BTreeSet<Uuid>>::new();
    for r in post.iter().filter(|r| r.1 == topic && r.0 == "sx") {
        if let (Some(input), Some(child)) = (r.2, r.4) {
            destinations.entry(input).or_default().insert(child);
        }
    }
    if destinations.values().any(|v| v.len() > 1) {
        return Err(conflict());
    }
    // Capture can bridge an already stored human label as well as a new rename.
    let labels: Vec<_> = post
        .iter()
        .filter(|r| r.1 == topic && r.0 == "l")
        .cloned()
        .collect();
    for l in labels {
        if !post.iter().any(|r| r.1 == topic && r.0 == "sl") {
            if let Some(source) = l.2.and_then(|i| map_source.get(&i)) {
                post.insert(row("sl", topic, *source, None, None));
            }
        }
    }
    let inputs: BTreeSet<_> = post.iter().filter_map(|r| r.2).collect();
    if post.len() > ROWS
        || inputs.len() > INPUTS
        || scope.len() + usize::from(new_topic) > INPUTS
        || serde_json::to_vec(&(intent, &post, change))
            .map_err(|_| capacity())?
            .len()
            > BYTES
    {
        return Err(capacity());
    }
    tx.commit().await?;
    Ok(())
}

async fn fresh(tx: &mut Tx<'_>, brain: Uuid, intent: &Intent) -> Result<()> {
    db::require_writer(tx, brain).await?;
    if graph::preparation_epoch(tx, brain).await? != intent.epoch {
        return Err(changed());
    }
    let current: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id,change_id FROM knowledge_topics WHERE brain_id=$1 AND id=ANY($2) ORDER BY id",
    )
    .bind(brain)
    .bind(intent.topic_ids())
    .fetch_all(&mut **tx)
    .await?;
    if current
        != intent
            .topics
            .iter()
            .map(|t| (t.id, t.change))
            .collect::<Vec<_>>()
    {
        return Err(changed());
    }
    if let Some(deadline) = intent.deadline {
        let valid: bool = sqlx::query_scalar("SELECT clock_timestamp()<$1")
            .bind(deadline)
            .fetch_one(&mut **tx)
            .await?;
        if !valid {
            return Err(changed());
        }
    }
    Ok(())
}

async fn persist_sources(tx: &mut Tx<'_>, brain: Uuid, actor: Uuid, intent: &Intent) -> Result<()> {
    for page in intent.sources.chunks(250) {
        source_features::persist(
            tx,
            brain,
            actor,
            &Features {
                sources: page.to_vec(),
                vectors: vec![],
                coverage: "human_intent".into(),
                deadline: intent.deadline,
            },
        )
        .await?;
    }
    Ok(())
}

async fn save(tx: &mut Tx<'_>, brain: Uuid, intent: &Intent) -> Result<()> {
    let topics = intent.topic_ids();
    sqlx::query(
        "DELETE FROM knowledge_topic_intent_mappings WHERE brain_id=$1 AND topic_id=ANY($2)",
    )
    .bind(brain)
    .bind(&topics)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "DELETE FROM knowledge_topic_intent_sources WHERE brain_id=$1 AND topic_id=ANY($2)",
    )
    .bind(brain)
    .bind(&topics)
    .execute(&mut **tx)
    .await?;
    sqlx::query("INSERT INTO knowledge_topic_intent_sources(id,brain_id,topic_id,input_id) SELECT gen_random_uuid(),$1,topic,input FROM jsonb_to_recordset($2) x(topic uuid,input uuid) ON CONFLICT DO NOTHING")
        .bind(brain).bind(Json(&intent.members)).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_intent_mappings(id,brain_id,topic_id,input_id,entity_id) SELECT gen_random_uuid(),$1,topic,input,entity FROM jsonb_to_recordset($2) x(topic uuid,input uuid,entity uuid) ON CONFLICT DO NOTHING")
        .bind(brain).bind(Json(&intent.mappings)).execute(&mut **tx).await?;
    Ok(())
}

pub async fn apply_capture(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    intent: &Intent,
) -> Result<()> {
    fresh(tx, brain, intent).await?;
    persist_sources(tx, brain, actor, intent).await?;
    save(tx, brain, intent).await?;
    copy_controls(tx, brain, &intent.topic_ids()).await?;
    copy_labels(tx, brain, &intent.topic_ids(), false).await?;
    Ok(())
}

/// A human name can gain a durable source basis only through the exact same
/// source version/context adopted by that human, never a replacement input.
pub async fn copy_legacy_label(
    tx: &mut Tx<'_>,
    brain: Uuid,
    topic: Uuid,
    replace: bool,
) -> Result<()> {
    copy_labels(tx, brain, &[topic], replace).await
}
async fn copy_labels(tx: &mut Tx<'_>, brain: Uuid, topics: &[Uuid], replace: bool) -> Result<()> {
    sqlx::query("INSERT INTO knowledge_topic_source_labels(brain_id,topic_id,basis_input,label) SELECT DISTINCT ON(l.topic_id) l.brain_id,l.topic_id,s.id,l.label FROM knowledge_topic_labels l JOIN knowledge_mapping_inputs i ON i.brain_id=l.brain_id AND i.id=l.basis_input JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id JOIN knowledge_topic_intent_sources m ON m.brain_id=l.brain_id AND m.topic_id=l.topic_id JOIN knowledge_source_inputs s ON s.brain_id=m.brain_id AND s.id=m.input_id AND s.source_version_id=i.source_version_id AND s.context=c.context WHERE l.brain_id=$1 AND l.topic_id=ANY($2) AND (NOT EXISTS(SELECT 1 FROM knowledge_topic_exclusions e WHERE e.brain_id=l.brain_id AND e.topic_id=l.topic_id AND e.input_id=l.basis_input) OR EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions e WHERE e.brain_id=l.brain_id AND e.topic_id=l.topic_id AND e.input_id=s.id)) AND (NOT EXISTS(SELECT 1 FROM knowledge_topic_splits x WHERE x.brain_id=l.brain_id AND x.original_topic_id=l.topic_id AND x.input_id=l.basis_input) OR EXISTS(SELECT 1 FROM knowledge_topic_source_splits x JOIN knowledge_topic_splits legacy ON legacy.brain_id=x.brain_id AND legacy.original_topic_id=x.original_topic_id AND legacy.split_topic_id=x.split_topic_id WHERE x.brain_id=l.brain_id AND x.original_topic_id=l.topic_id AND x.input_id=s.id AND legacy.input_id=l.basis_input)) ORDER BY l.topic_id,s.id ON CONFLICT(topic_id) DO UPDATE SET basis_input=excluded.basis_input,label=excluded.label WHERE $3")
        .bind(brain).bind(topics).bind(replace).execute(&mut **tx).await?;
    Ok(())
}

pub async fn copy_legacy_controls(tx: &mut Tx<'_>, brain: Uuid, topic: Uuid) -> Result<()> {
    copy_controls(tx, brain, &[topic]).await
}
async fn copy_controls(tx: &mut Tx<'_>, brain: Uuid, topics: &[Uuid]) -> Result<()> {
    split_conflict(tx, brain, topics).await?;
    sqlx::query("INSERT INTO knowledge_topic_source_exclusions(brain_id,topic_id,input_id) SELECT DISTINCT e.brain_id,e.topic_id,s.id FROM knowledge_topic_exclusions e JOIN knowledge_mapping_inputs i ON i.brain_id=e.brain_id AND i.id=e.input_id JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id JOIN knowledge_topic_intent_sources m ON m.brain_id=e.brain_id AND m.topic_id=e.topic_id JOIN knowledge_source_inputs s ON s.brain_id=m.brain_id AND s.id=m.input_id AND s.source_version_id=i.source_version_id AND s.context=c.context WHERE e.brain_id=$1 AND e.topic_id=ANY($2) ON CONFLICT DO NOTHING")
        .bind(brain).bind(topics).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_source_splits(brain_id,original_topic_id,split_topic_id,input_id) SELECT DISTINCT e.brain_id,e.original_topic_id,e.split_topic_id,s.id FROM knowledge_topic_splits e JOIN knowledge_mapping_inputs i ON i.brain_id=e.brain_id AND i.id=e.input_id JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id JOIN knowledge_topic_intent_sources m ON m.brain_id=e.brain_id AND m.topic_id=e.original_topic_id JOIN knowledge_source_inputs s ON s.brain_id=m.brain_id AND s.id=m.input_id AND s.source_version_id=i.source_version_id AND s.context=c.context WHERE e.brain_id=$1 AND e.original_topic_id=ANY($2) ON CONFLICT DO NOTHING")
        .bind(brain).bind(topics).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT e.brain_id,e.split_topic_id,e.input_id,'explicit_source_split',1 FROM knowledge_topic_source_splits e JOIN knowledge_topic_intent_sources m ON m.brain_id=e.brain_id AND m.topic_id=e.original_topic_id AND m.input_id=e.input_id WHERE e.brain_id=$1 AND e.original_topic_id=ANY($2) ON CONFLICT DO NOTHING")
        .bind(brain).bind(topics).execute(&mut **tx).await?;
    Ok(())
}

async fn split_conflict(tx: &mut Tx<'_>, brain: Uuid, topics: &[Uuid]) -> Result<()> {
    let ambiguous:bool=sqlx::query_scalar("WITH destinations AS(SELECT e.original_topic_id topic,i.source_version_id version,c.context,e.split_topic_id child FROM knowledge_topic_splits e JOIN knowledge_mapping_inputs i ON i.brain_id=e.brain_id AND i.id=e.input_id JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id WHERE e.brain_id=$1 AND e.original_topic_id=ANY($2) UNION ALL SELECT e.original_topic_id,i.source_version_id,i.context,e.split_topic_id FROM knowledge_topic_source_splits e JOIN knowledge_source_inputs i ON i.brain_id=e.brain_id AND i.id=e.input_id WHERE e.brain_id=$1 AND e.original_topic_id=ANY($2)) SELECT EXISTS(SELECT 1 FROM destinations GROUP BY topic,version,context HAVING count(DISTINCT child)>1)")
        .bind(brain).bind(topics).fetch_one(&mut **tx).await?;
    if ambiguous {
        return Err(conflict());
    }
    Ok(())
}

/// Ordinary producer replacement must preserve durable human controls before
/// their old mapping FKs cascade. Privacy erasure does not use this path.
pub async fn copy_before_replacement(
    tx: &mut Tx<'_>,
    brain: Uuid,
    old_inputs: &[Uuid],
) -> Result<()> {
    let topics:Vec<Uuid>=sqlx::query_scalar("SELECT DISTINCT m.topic_id FROM knowledge_topic_intent_mappings m WHERE m.brain_id=$1 AND m.input_id=ANY($2) UNION SELECT l.topic_id FROM knowledge_topic_labels l WHERE l.brain_id=$1 AND l.basis_input=ANY($2) UNION SELECT e.topic_id FROM knowledge_topic_exclusions e WHERE e.brain_id=$1 AND e.input_id=ANY($2) UNION SELECT e.original_topic_id FROM knowledge_topic_splits e WHERE e.brain_id=$1 AND e.input_id=ANY($2) ORDER BY 1 LIMIT 10001")
        .bind(brain).bind(old_inputs).fetch_all(&mut **tx).await?;
    if topics.len() > INPUTS {
        return Err(capacity());
    }
    copy_controls(tx, brain, &topics).await?;
    copy_labels(tx, brain, &topics, false).await?;
    Ok(())
}

/// Read-only canonical saved intent used before native identity matching.
pub async fn prepare_saved(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    epoch: i64,
) -> Result<Intent> {
    let mut tx = read(state, auth, brain).await?;
    let topics = topics(&mut tx, brain, None).await?;
    let ids = topics.iter().map(|t| t.id).collect::<Vec<_>>();
    let (m, s) = raw(&mut tx, brain, &ids).await?;
    let mut out = qualify(state, &mut tx, auth, brain, topics, m, s, None).await?;
    if out.epoch != epoch {
        return Err(changed());
    }
    // Global capacity and exact source eligibility include all manual split
    // roots. Their identities/memberships remain human split records; only
    // edited automatic communities and existing overlays enter reconciliation.
    let publication_topics = out
        .topics
        .iter()
        .filter(|t| t.kind != "split")
        .map(|t| t.id)
        .collect();
    out.retain_topics(&publication_topics);
    tx.commit().await?;
    Ok(out)
}

/// Genuine provisional native memberships, already canonically qualified by
/// the publisher. Only a uniquely matched edited topic adopts a refreshed set.
pub struct Cohort {
    pub topic: Uuid,
    pub mappings: Vec<(Uuid, Uuid)>,
    pub sources: Vec<Uuid>,
}
pub struct Publication {
    intent: Intent,
    overlays: Vec<Uuid>,
}
impl Publication {
    pub fn deadline(&self) -> Option<DateTime<Utc>> {
        self.intent.deadline
    }
}

/// Complete preparation after native identity matching. Saved human source
/// identity, rather than the current machine feature profile, governs overlays.
pub async fn prepare_publication(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    mut saved: Intent,
    cohorts: &[Cohort],
) -> Result<Publication> {
    let selected: BTreeSet<_> = saved
        .topics
        .iter()
        .filter(|t| t.kind == "community")
        .map(|t| t.id)
        .collect();
    let refresh: BTreeSet<_> = cohorts
        .iter()
        .filter(|c| selected.contains(&c.topic))
        .map(|c| c.topic)
        .collect();
    let overlays: Vec<_> = saved
        .topics
        .iter()
        .filter(|t| t.kind == "manual_overlay" || !refresh.contains(&t.id))
        .map(|t| t.id)
        .collect();
    if !refresh.is_empty() {
        let mut tx = read(state, auth, brain).await?;
        let mappings: Vec<_> = cohorts
            .iter()
            .filter(|c| refresh.contains(&c.topic))
            .flat_map(|c| {
                c.mappings.iter().map(
                    move |(input, entity)| json!({"topic":c.topic,"input":input,"entity":entity}),
                )
            })
            .collect();
        let sources: Vec<_> = cohorts
            .iter()
            .filter(|c| refresh.contains(&c.topic))
            .flat_map(|c| {
                c.sources
                    .iter()
                    .map(move |input| json!({"topic":c.topic,"input":input}))
            })
            .collect();
        if mappings.len() + sources.len() > ROWS {
            return Err(capacity());
        }
        let (mapping_bytes,source_bytes):(i64,i64)=sqlx::query_as("SELECT coalesce((SELECT sum(octet_length(c.context::text)::bigint+256)::bigint FROM jsonb_to_recordset($2) x(topic uuid,input uuid,entity uuid) JOIN knowledge_mapping_inputs i ON i.brain_id=$1 AND i.id=x.input JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id),0),coalesce((SELECT sum(octet_length(i.context::text)::bigint+256)::bigint FROM jsonb_to_recordset($3) x(topic uuid,input uuid) JOIN knowledge_source_inputs i ON i.brain_id=$1 AND i.id=x.input),0)")
            .bind(brain).bind(Json(&mappings)).bind(Json(&sources)).fetch_one(&mut *tx).await?;
        if mapping_bytes as usize + source_bytes as usize > BYTES {
            return Err(capacity());
        }
        let m:Vec<(Uuid,Uuid,Uuid,Uuid,Uuid,Json<Value>)>=sqlx::query_as("SELECT DISTINCT x.topic,i.id,x.entity,v.source_id,v.id,c.context FROM jsonb_to_recordset($2) x(topic uuid,input uuid,entity uuid) JOIN knowledge_mapping_inputs i ON i.brain_id=$1 AND i.id=x.input JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.source_version_id JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id WHERE EXISTS(SELECT 1 FROM knowledge_mentions n WHERE n.brain_id=i.brain_id AND n.input_id=i.id AND n.entity_id=x.entity)")
            .bind(brain).bind(Json(&mappings)).fetch_all(&mut *tx).await?;
        let s:Vec<(Uuid,Uuid,Uuid,Uuid,Json<Value>)>=sqlx::query_as("SELECT DISTINCT x.topic,i.id,i.source_id,i.source_version_id,i.context FROM jsonb_to_recordset($2) x(topic uuid,input uuid) JOIN knowledge_source_inputs i ON i.brain_id=$1 AND i.id=x.input")
            .bind(brain).bind(Json(&sources)).fetch_all(&mut *tx).await?;
        if m.len() != mappings.len() || s.len() != sources.len() {
            return Err(changed());
        }
        let new = qualify(
            state,
            &mut tx,
            auth,
            brain,
            saved
                .topics
                .iter()
                .filter(|t| refresh.contains(&t.id))
                .cloned()
                .collect(),
            m.into_iter()
                .map(
                    |(topic, input, entity, source, version, Json(context))| RawMapping {
                        topic,
                        input,
                        entity,
                        source,
                        version,
                        context,
                    },
                )
                .collect(),
            s.into_iter()
                .map(|(topic, input, source, version, Json(context))| RawSource {
                    topic,
                    input,
                    source,
                    version,
                    context,
                })
                .collect(),
            None,
        )
        .await?;
        if new.epoch != saved.epoch {
            return Err(changed());
        }
        // A complete native cohort must remain complete under this second
        // canonical check; it cannot silently become a partial human snapshot.
        let saved_source_members: BTreeSet<_> =
            new.members.iter().map(|m| (m.topic, m.input)).collect();
        if new.mappings.len() != mappings.len()
            || sources.iter().any(|s| {
                !saved_source_members.contains(&(
                    s["topic"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_default(),
                    s["input"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_default(),
                ))
            })
        {
            return Err(changed());
        }
        let retained: BTreeSet<_> = saved
            .topics
            .iter()
            .map(|t| t.id)
            .filter(|t| !refresh.contains(t))
            .collect();
        saved.retain_topics(&retained);
        saved.topics.extend(new.topics);
        saved.sources.extend(new.sources);
        saved.members.extend(new.members);
        saved.mappings.extend(new.mappings);
        saved.deadline = [saved.deadline, new.deadline].into_iter().flatten().min();
        saved.topics.sort_by_key(|t| t.id);
        saved.sources.sort_by_key(|s| s.id);
        saved.sources.dedup_by_key(|s| s.id);
        tx.commit().await?;
    }
    saved.bounded()?;
    Ok(Publication {
        intent: saved,
        overlays,
    })
}

/// The caller owns the authenticated Brain writer and whole1.5s transaction.
/// This runs before community replacement, preserving only freshly prepared
/// human memberships and independently expiring exact label/override bases.
pub async fn apply_publication(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    p: &Publication,
) -> Result<()> {
    let intent = &p.intent;
    if intent.topics.is_empty() {
        return Ok(());
    }
    fresh(tx, brain, intent).await?;
    persist_sources(tx, brain, actor, intent).await?;
    save(tx, brain, intent).await?;
    let topics = intent.topic_ids();
    // Bridge old human names while their original NER basis still exists.
    copy_controls(tx, brain, &topics).await?;
    copy_labels(tx, brain, &topics, false).await?;
    sqlx::query("DELETE FROM knowledge_topic_sources WHERE brain_id=$1 AND(topic_id=ANY($2) OR(topic_id=ANY($3) AND reason='explicit_source_overlay'))")
        .bind(brain).bind(&p.overlays).bind(&topics).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM knowledge_topic_memberships WHERE brain_id=$1 AND topic_id=ANY($2)")
        .bind(brain)
        .bind(&p.overlays)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT $1,topic,input,'explicit_source_overlay',1 FROM jsonb_to_recordset($2) x(topic uuid,input uuid) ON CONFLICT(topic_id,input_id) DO UPDATE SET reason=excluded.reason,weight=excluded.weight")
        .bind(brain).bind(Json(&intent.members)).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) SELECT $1,topic,entity,input,'explicit_overlay',1 FROM jsonb_to_recordset($2) x(topic uuid,input uuid,entity uuid) WHERE topic=ANY($3) ON CONFLICT DO NOTHING")
        .bind(brain).bind(Json(&intent.mappings)).bind(&p.overlays).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM knowledge_topic_labels l WHERE l.brain_id=$1 AND l.topic_id=ANY($2) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_intent_mappings m WHERE m.brain_id=l.brain_id AND m.topic_id=l.topic_id AND m.input_id=l.basis_input)")
        .bind(brain).bind(&topics).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM knowledge_topic_source_labels l WHERE l.brain_id=$1 AND l.topic_id=ANY($2) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_intent_sources m WHERE m.brain_id=l.brain_id AND m.topic_id=l.topic_id AND m.input_id=l.basis_input)")
        .bind(brain).bind(&topics).execute(&mut **tx).await?;
    for (table, column, intent_table) in [
        (
            "knowledge_topic_exclusions",
            "topic_id",
            "knowledge_topic_intent_mappings",
        ),
        (
            "knowledge_topic_splits",
            "original_topic_id",
            "knowledge_topic_intent_mappings",
        ),
        (
            "knowledge_topic_source_exclusions",
            "topic_id",
            "knowledge_topic_intent_sources",
        ),
        (
            "knowledge_topic_source_splits",
            "original_topic_id",
            "knowledge_topic_intent_sources",
        ),
    ] {
        sqlx::query(&format!("DELETE FROM {table} x WHERE x.brain_id=$1 AND x.{column}=ANY($2) AND NOT EXISTS(SELECT 1 FROM {intent_table} m WHERE m.brain_id=x.brain_id AND m.topic_id=x.{column} AND m.input_id=x.input_id)"))
            .bind(brain).bind(&topics).execute(&mut **tx).await?;
    }
    let changes: Vec<_> = intent
        .topics
        .iter()
        .filter(|t| p.overlays.contains(&t.id))
        .map(|t| json!({"topic":t.id,"previous":t.change,"change":Uuid::new_v4()}))
        .collect();
    sqlx::query("UPDATE knowledge_topics t SET kind='manual_overlay',recipe='human-overlay-1',organization_report_id=NULL,change_id=x.change FROM jsonb_to_recordset($2) x(topic uuid,previous uuid,change uuid) WHERE t.brain_id=$1 AND t.id=x.topic AND t.change_id=x.previous")
        .bind(brain).bind(Json(&changes)).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action,previous_change_id) SELECT change,$1,topic,$2,'rebuild',previous FROM jsonb_to_recordset($3) x(topic uuid,previous uuid,change uuid)")
        .bind(brain).bind(actor).bind(Json(&changes)).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM knowledge_topics t WHERE t.brain_id=$1 AND t.id=ANY($2) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_sources m WHERE m.brain_id=t.brain_id AND m.topic_id=t.id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_memberships m WHERE m.brain_id=t.brain_id AND m.topic_id=t.id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_intent_sources m WHERE m.brain_id=t.brain_id AND m.topic_id=t.id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_intent_mappings m WHERE m.brain_id=t.brain_id AND m.topic_id=t.id)")
        .bind(brain).bind(&topics).execute(&mut **tx).await?;
    Ok(())
}
