//! Complete, bounded label cohorts shared by overview and source detail.
use super::*;
use std::collections::BTreeMap;

const MAX_INPUTS: usize = 10_000;
const MAX_CONTRIBUTORS: usize = 50_000;

pub(super) async fn current_inputs(
    tx: &mut Tx<'_>,
    brain: Uuid,
    versions: &[Uuid],
) -> Result<Vec<Uuid>> {
    let candidates: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM knowledge_mapping_inputs WHERE brain_id=$1 AND state='published' AND source_version_id=ANY($2) ORDER BY id LIMIT 10001")
        .bind(brain).bind(versions).fetch_all(&mut **tx).await?;
    if candidates.len() > MAX_INPUTS {
        return Err(Error::invalid(
            "The complete mapping input window exceeds its bounded envelope.",
        ));
    }
    Ok(
        sqlx::query_scalar("SELECT input_id FROM recollect_mapping_current_input_ids($1,$2)")
            .bind(brain)
            .bind(candidates)
            .fetch_all(&mut **tx)
            .await?,
    )
}

#[derive(FromRow)]
struct Contributor {
    topic_id: Uuid,
    topic_kind: String,
    anchor: Option<Uuid>,
    pinned: bool,
    change_id: Uuid,
    human_label: Option<String>,
    human_basis: Option<Uuid>,
    entity_id: Uuid,
    entity_kind: String,
    normalized_label: String,
    realm: String,
    input_id: Uuid,
    source_version_id: Uuid,
    mention_id: Uuid,
    byte_start: i32,
    byte_end: i32,
    quote: String,
}
impl Contributor {
    fn span_key(&self) -> (Uuid, i32, i32, &str, Uuid) {
        (
            self.source_version_id,
            self.byte_start,
            self.byte_end,
            &self.quote,
            self.mention_id,
        )
    }
}

pub(super) async fn cards(
    tx: &mut Tx<'_>,
    brain: Uuid,
    inputs: &[Uuid],
    topic: Option<Uuid>,
    entity: Option<Uuid>,
    nodes: &std::collections::BTreeMap<String, GraphNode>,
) -> Result<Vec<CardRow>> {
    // Fetch one extra row solely to reject an incomplete cohort. Neither the
    // ranking nor its denominator may depend on a detail/card page prefix.
    let rows: Vec<Contributor> = sqlx::query_as(&format!("{} SELECT t.id topic_id,t.kind topic_kind,t.anchor_entity_id anchor,t.pinned,t.change_id,l.label human_label,l.basis_input human_basis,
        e.id entity_id,e.kind entity_kind,e.normalized_label,e.realm::text realm,m.input_id,m.source_version_id,
        v.id mention_id,v.byte_start,v.byte_end,v.quote
        FROM visible_memberships m JOIN current_topics t ON t.brain_id=m.brain_id AND t.id=m.topic_id
        JOIN knowledge_entities e ON e.brain_id=m.brain_id AND e.id=m.entity_id
        JOIN knowledge_mentions v ON v.brain_id=m.brain_id AND v.input_id=m.input_id AND v.entity_id=m.entity_id
        LEFT JOIN knowledge_topic_labels l ON l.brain_id=t.brain_id AND l.topic_id=t.id
        WHERE ($3::uuid IS NULL OR t.id=$3) AND ($4::uuid IS NULL OR EXISTS(SELECT 1 FROM visible_memberships centered WHERE centered.topic_id=t.id AND centered.entity_id=$4))
        ORDER BY t.id,e.id,m.source_version_id,v.byte_start,v.byte_end,v.quote,v.id LIMIT 50001",eligible_sql()))
        .bind(brain).bind(inputs).bind(topic).bind(entity).fetch_all(&mut **tx).await
        .map_err(|e| query_error("topic_labels",e))?;
    if rows.len() > MAX_CONTRIBUTORS {
        return Err(Error::invalid(
            "The complete topic label window exceeds its bounded envelope.",
        ));
    }
    let mut topics = BTreeMap::<Uuid, Vec<&Contributor>>::new();
    for row in &rows {
        topics.entry(row.topic_id).or_default().push(row);
    }
    if topics.len() > 10_000 {
        return Err(Error::invalid(
            "The complete topic window exceeds its bounded envelope.",
        ));
    }
    let mut cards = Vec::with_capacity(topics.len());
    for (id, rows) in topics {
        let meta = rows[0];
        let human = rows
            .iter()
            .copied()
            .filter(|r| r.human_label.is_some() && r.human_basis == Some(r.input_id))
            .min_by_key(|r| r.span_key());
        // Entity UUID is the aggregation identity: two scoped entities with
        // the same surface label must never pool their source support.
        let mut terms = BTreeMap::<Uuid, Vec<&Contributor>>::new();
        for row in &rows {
            if meta.topic_kind != "community"
                || matches!(row.entity_kind.as_str(), "technology" | "concept")
            {
                terms.entry(row.entity_id).or_default().push(row);
            }
        }
        let anchor = (meta.topic_kind == "facet")
            .then_some(meta.anchor)
            .flatten()
            .and_then(|anchor| terms.get(&anchor));
        let ranked = anchor.or_else(|| {
            terms.values().min_by_key(|rows| {
                let r = rows[0];
                let distinct = rows
                    .iter()
                    .map(|r| r.source_version_id)
                    .collect::<BTreeSet<_>>()
                    .len();
                super::super::topic_labels::term_rank(
                    distinct,
                    (
                        r.entity_kind.as_str(),
                        r.normalized_label.as_str(),
                        r.realm.as_str(),
                        r.entity_id,
                    ),
                )
            })
        });
        let selected = human
            .or_else(|| ranked.and_then(|rows| rows.iter().copied().min_by_key(|r| r.span_key())));
        let (label, version, mention, provenance) = if let Some(r) = selected {
            (
                if human.is_some() {
                    r.human_label.clone().ok_or_else(changed)?
                } else {
                    r.quote.clone()
                },
                r.source_version_id,
                r.mention_id,
                if human.is_some() {
                    "authenticated_mapping_label"
                } else {
                    "eligible_source_term"
                },
            )
        } else {
            let version = rows
                .iter()
                .map(|r| r.source_version_id)
                .min()
                .ok_or_else(changed)?;
            (
                nodes
                    .get(&source_key(version))
                    .ok_or_else(changed)?
                    .evidence
                    .label
                    .clone(),
                version,
                Uuid::nil(),
                "canonical_source_title",
            )
        };
        cards.push(CardRow {
            id,
            kind: "topic".into(),
            topic_kind: Some(meta.topic_kind.clone()),
            label,
            source_count: rows
                .iter()
                .map(|r| r.source_version_id)
                .collect::<BTreeSet<_>>()
                .len() as i64,
            entity_count: rows
                .iter()
                .map(|r| r.entity_id)
                .collect::<BTreeSet<_>>()
                .len() as i64,
            pinned: meta.pinned,
            change_id: Some(meta.change_id),
            representative_source: version,
            representative_mention: mention,
            provenance_kind: provenance.into(),
        });
    }
    Ok(cards)
}
