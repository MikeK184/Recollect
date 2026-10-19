//! Source contributor navigation has its own cursor; existing entity detail
//! phases keep their original meaning and response-wide 80-row bound.
use super::*;
use crate::db::admission::{self, BrainAccess};
use std::collections::BTreeMap;

#[derive(Clone, FromRow)]
pub(super) struct Row {
    pub topic_id: Uuid,
    pub input_id: Uuid,
    pub version: Uuid,
    pub reason: String,
    pub weight: f64,
    pinned: bool,
    change_id: Uuid,
    topic_kind: String,
    label: Option<String>,
    basis: Option<Uuid>,
    report_deadline: Option<DateTime<Utc>>,
}
pub(super) struct Window {
    pub rows: Vec<Row>,
    pub cards: Vec<CardRow>,
    pub deadline: Option<DateTime<Utc>>,
}

pub(super) async fn window(
    tx: &mut Tx<'_>,
    brain: Uuid,
    versions: &[Uuid],
    topic: Option<Uuid>,
    nodes: &BTreeMap<String, GraphNode>,
    mapping_cards: &[CardRow],
    topic_ids: Option<&[Uuid]>,
) -> Result<Window> {
    let candidates:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM knowledge_source_inputs WHERE brain_id=$1 AND source_version_id=ANY($2) AND context=recollect_organization_source_context(brain_id,source_version_id,source_id) ORDER BY id LIMIT 251")
        .bind(brain).bind(versions).fetch_all(&mut **tx).await?;
    if candidates.len() > 250 {
        return Err(changed());
    }
    let ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT input_id FROM recollect_organization_current_source_ids($1,$2)")
            .bind(brain)
            .bind(candidates)
            .fetch_all(&mut **tx)
            .await?;
    let rows:Vec<Row>=sqlx::query_as(&format!("WITH current_topics AS MATERIALIZED ({CURRENT_TOPICS}) SELECT m.topic_id,m.input_id,i.source_version_id version,m.reason,m.weight,t.pinned,t.change_id,t.kind AS topic_kind,l.label,l.basis_input basis,t.organization_expires_at report_deadline FROM knowledge_topic_sources m JOIN current_topics t ON t.brain_id=m.brain_id AND t.id=m.topic_id JOIN knowledge_source_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id LEFT JOIN knowledge_topic_source_labels l ON l.brain_id=t.brain_id AND l.topic_id=t.id WHERE m.brain_id=$1 AND i.id=ANY($2) AND ($3::uuid IS NULL OR m.topic_id=$3) AND ($4::uuid[] IS NULL OR m.topic_id=ANY($4)) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions e WHERE e.brain_id=m.brain_id AND e.topic_id=m.topic_id AND e.input_id=m.input_id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_splits s WHERE s.brain_id=m.brain_id AND s.original_topic_id=m.topic_id AND s.input_id=m.input_id) ORDER BY m.topic_id,m.input_id LIMIT 10001"))
        .bind(brain).bind(ids).bind(topic).bind(topic_ids).fetch_all(&mut **tx).await?;
    if rows.len() > 10000 {
        return Err(Error::invalid(
            "The complete source membership window exceeds its bounded envelope.",
        ));
    }
    let deadline = rows.iter().filter_map(|r| r.report_deadline).min();
    let mut grouped = BTreeMap::<Uuid, Vec<&Row>>::new();
    for row in &rows {
        grouped.entry(row.topic_id).or_default().push(row);
    }
    let mut cards = vec![];
    let mapping_cards: BTreeMap<_, _> = mapping_cards.iter().map(|c| (c.id, c)).collect();
    for (id, rows) in grouped {
        let basis = rows.iter().find(|r| r.basis == Some(r.input_id));
        let mapped = mapping_cards.get(&id).copied();
        let representative = basis
            .copied()
            .or_else(|| {
                mapped.and_then(|c| {
                    rows.iter()
                        .copied()
                        .find(|r| r.version == c.representative_source)
                })
            })
            .unwrap_or_else(|| {
                rows.iter()
                    .copied()
                    .min_by_key(|r| r.version)
                    .expect("Nonempty source topic")
            });
        let label = basis
            .and_then(|r| r.label.clone())
            .or_else(|| mapped.map(|c| c.label.clone()))
            .or_else(|| {
                nodes
                    .get(&source_key(representative.version))
                    .map(|n| n.evidence.label.clone())
            })
            .ok_or_else(changed)?;
        cards.push(CardRow {
            id,
            kind: "topic".into(),
            topic_kind: Some(representative.topic_kind.clone()),
            label,
            source_count: rows
                .iter()
                .map(|r| r.version)
                .collect::<BTreeSet<_>>()
                .len() as i64,
            entity_count: mapped.map_or(0, |c| c.entity_count),
            pinned: representative.pinned,
            change_id: Some(representative.change_id),
            representative_source: if basis.is_some() {
                representative.version
            } else {
                mapped.map_or(representative.version, |c| c.representative_source)
            },
            representative_mention: if basis.is_some() {
                Uuid::nil()
            } else {
                mapped.map_or(Uuid::nil(), |c| c.representative_mention)
            },
            provenance_kind: if basis.is_some() {
                "authenticated_source_label".into()
            } else {
                mapped.map_or_else(
                    || "canonical_source_title".into(),
                    |c| c.provenance_kind.clone(),
                )
            },
        });
    }
    cards.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then(b.source_count.cmp(&a.source_count))
            .then(a.label.cmp(&b.label))
            .then(a.id.cmp(&b.id))
    });
    Ok(Window {
        rows,
        cards,
        deadline,
    })
}

pub(super) fn details(
    window: &Window,
    after: Option<(Uuid, Uuid)>,
    limit: usize,
) -> (Vec<OrganizationSourceMembership>, bool) {
    let rows: Vec<_> = window
        .rows
        .iter()
        .filter(|r| after.is_none_or(|a| (r.topic_id, r.input_id) > a))
        .take(limit + 1)
        .collect();
    let more = rows.len() > limit;
    (
        rows.into_iter()
            .take(limit)
            .map(|r| OrganizationSourceMembership {
                topic_id: r.topic_id,
                input_id: r.input_id,
                source_key: source_key(r.version),
                reason: r.reason.clone(),
                weight: r.weight,
            })
            .collect(),
        more,
    )
}
pub(super) fn representatives(
    window: &Window,
    cards: &[CardRow],
) -> Vec<OrganizationRepresentativeSource> {
    cards
        .iter()
        .filter_map(|c| {
            window
                .rows
                .iter()
                .find(|r| r.topic_id == c.id && r.version == c.representative_source)
                .map(|r| OrganizationRepresentativeSource {
                    topic_id: c.id,
                    input_id: r.input_id,
                    source_key: source_key(r.version),
                    provenance_kind: c.provenance_kind.clone(),
                })
        })
        .collect()
}

async fn prepare_sources(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    input: &OrganizationSourcesRequest,
) -> Result<OrganizationSourcesView> {
    if input.scope.kind != "knowledge" {
        return Err(Error::invalid("Choose a knowledge scope."));
    }
    let mut recall = RecallRequest {
        query: "Source contributor selection".into(),
        selection: input.scope.selection.clone(),
        operation_id: input.scope.operation_id,
        collection_id: input.scope.collection_id,
        manifest_revision_id: input.scope.manifest_revision_id,
        fact_at: input.scope.fact_at,
        mode: input.scope.mode.clone(),
        ..Default::default()
    };
    retrieval::validate(&mut recall)?;
    let mut scope = input.scope.clone();
    scope.selection = recall.selection;
    let mut tx = auth.preparation_tx(&state.pool).await?;
    sqlx::query("SET LOCAL recollect.bulk_qualification=on")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL jit=off").execute(&mut *tx).await?;
    sqlx::query("SET LOCAL statement_timeout='1500ms'")
        .execute(&mut *tx)
        .await?;
    retrieval::graph::authorize(&mut tx, auth, brain, &scope).await?;
    let epoch = graph::preparation_epoch(&mut tx, brain).await?;
    if let Some(c) = &input.cursor {
        if c.brain_id != brain
            || c.epoch != epoch
            || c.topic_id != input.topic_id
            || serde_json::to_value(&c.scope).ok() != serde_json::to_value(&scope).ok()
            || c.expires_at.is_some_and(|d| d <= Utc::now())
            || c.after_input.is_some() != c.after_topic.is_some()
            || (c.after_input.is_some() && c.through_source.is_none())
            || c.after_source
                .zip(c.through_source)
                .is_some_and(|(a, b)| a >= b)
        {
            return Err(changed());
        }
    }
    let after_source = input.cursor.as_ref().and_then(|c| c.after_source);
    let through_source = input.cursor.as_ref().and_then(|c| c.through_source);
    let mut candidates = candidate_sources(
        &mut tx,
        brain,
        after_source,
        through_source,
        input.topic_id,
        None,
    )
    .await?;
    let more_sources = candidates.len() > 250;
    candidates.truncate(250);
    let entities: Vec<_> = candidates
        .iter()
        .map(|(s, v)| graph::Entity {
            kind: "source_version".into(),
            id: *s,
            revision_id: *v,
        })
        .collect();
    let mut qualified = retrieval::graph::qualify(
        state,
        &mut tx,
        auth,
        brain,
        &mut scope,
        &entities,
        (250, None),
    )
    .await?;
    let versions: Vec<_> = entities
        .iter()
        .filter(|e| qualified.nodes.contains_key(&e.key()))
        .map(|e| e.revision_id)
        .collect();
    // Match overview's model fence and exact published-origin qualification.
    let fenced: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM source_versions WHERE brain_id=$1 AND id=ANY($2) AND recollect_source_model_fenced(brain_id,id)")
        .bind(brain).bind(&versions).fetch_all(&mut *tx).await?;
    for id in fenced {
        qualified.nodes.remove(&source_key(id));
    }
    let versions: Vec<_> = versions
        .into_iter()
        .filter(|v| qualified.nodes.contains_key(&source_key(*v)))
        .collect();
    let mapping_inputs = labels::current_inputs(&mut tx, brain, &versions).await?;
    let mapping_cards = labels::cards(
        &mut tx,
        brain,
        &mapping_inputs,
        input.topic_id,
        None,
        &qualified.nodes,
    )
    .await?;
    let w = window(
        &mut tx,
        brain,
        &versions,
        input.topic_id,
        &qualified.nodes,
        &mapping_cards,
        None,
    )
    .await?;
    let deadline = [
        qualified.deadline,
        w.deadline,
        input.cursor.as_ref().and_then(|c| c.expires_at),
    ]
    .into_iter()
    .flatten()
    .min();
    let after = input
        .cursor
        .as_ref()
        .and_then(|c| c.after_topic.zip(c.after_input));
    let (rows, more) = details(&w, after, 80);
    let next_cursor = if more {
        let last = rows.last().ok_or_else(changed)?;
        Some(OrganizationSourceCursor {
            brain_id: brain,
            after_source,
            through_source: through_source.or_else(|| candidates.last().map(|c| c.0)),
            after_topic: Some(last.topic_id),
            after_input: Some(last.input_id),
            epoch,
            scope: scope.clone(),
            topic_id: input.topic_id,
            expires_at: deadline,
        })
    } else if more_sources
        || (through_source.is_some()
            && !candidate_sources(
                &mut tx,
                brain,
                candidates.last().map(|c| c.0).or(through_source),
                None,
                input.topic_id,
                None,
            )
            .await?
            .is_empty())
    {
        Some(OrganizationSourceCursor {
            brain_id: brain,
            after_source: candidates.last().map(|c| c.0).or(through_source),
            through_source: None,
            after_topic: None,
            after_input: None,
            epoch,
            scope: scope.clone(),
            topic_id: input.topic_id,
            expires_at: deadline,
        })
    } else {
        None
    };
    // Source response cards are endpoints for this page only, not an unbounded
    // extra overview. Source identities remain canonical; origin describes the
    // same qualified term, authenticated name or title selected for overview.
    let endpoint: BTreeSet<_> = rows.iter().map(|r| r.topic_id).collect();
    let cards: Vec<_> = w
        .cards
        .iter()
        .filter(|c| endpoint.contains(&c.id))
        .cloned()
        .collect();
    let reps = representatives(&w, &cards);
    let evidence_ids: BTreeSet<_> = rows
        .iter()
        .map(|r| r.source_key.clone())
        .chain(reps.iter().map(|r| r.source_key.clone()))
        .collect();
    let result = OrganizationSourcesView {
        brain_id: brain,
        scope,
        epoch,
        expires_at: deadline,
        state: if next_cursor.is_some() {
            "partial"
        } else if rows.is_empty() {
            "unassigned"
        } else {
            "ready"
        }
        .into(),
        topics: cards.iter().map(CardRow::card).collect(),
        evidence: qualified
            .nodes
            .into_iter()
            .filter(|(k, _)| evidence_ids.contains(k))
            .map(|(_, n)| n)
            .collect(),
        source_memberships: rows,
        representative_sources: reps,
        coverage: OrganizationCoverage {
            examined_sources: candidates.len(),
            qualified_sources: versions.len(),
            mapped_sources: w
                .rows
                .iter()
                .map(|r| r.version)
                .collect::<BTreeSet<_>>()
                .len(),
            unassigned_sources: versions.len().saturating_sub(
                w.rows
                    .iter()
                    .map(|r| r.version)
                    .collect::<BTreeSet<_>>()
                    .len(),
            ),
            counts_are_lower_bounds: input.cursor.is_some() || next_cursor.is_some(),
        },
        community_status: community_status(&mut tx, brain).await?,
        next_cursor,
    };
    tx.commit().await?;
    Ok(result)
}

#[utoipa::path(post,path="/api/brains/{brain}/organization/sources/view",operation_id="organizationSources",params(("brain"=Uuid,Path)),request_body=OrganizationSourcesRequest,responses((status=200,body=OrganizationSourcesView)))]
pub async fn view(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<OrganizationSourcesRequest>,
) -> Result<Json<OrganizationSourcesView>> {
    let _permit = graph::read_capacity().try_acquire().map_err(|_| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "organization_busy",
            "Organization read capacity is busy.",
        )
    })?;
    tokio::time::timeout(Duration::from_secs(5), async {
        let prepared = prepare_sources(&state, &auth, brain, &input).await?;
        let mut tx = admission::brain_tx(
            &state.pool,
            &auth,
            brain,
            BrainAccess::READ.with_session_recheck(),
        )
        .await?
        .tx;
        retrieval::graph::authorize(&mut tx, &auth, brain, &prepared.scope).await?;
        let live: bool =
            sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
                .bind(prepared.expires_at)
                .fetch_one(&mut *tx)
                .await?;
        if graph::preparation_epoch(&mut tx, brain).await? != prepared.epoch || !live {
            return Err(changed());
        }
        tx.commit().await?;
        Ok(Json(prepared))
    })
    .await
    .map_err(|_| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "organization_timeout",
            "Organization preparation exceeded its bounded window.",
        )
    })?
}

pub(super) async fn prepare_change(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    topic: Uuid,
    inputs: &[Uuid],
    visible: bool,
) -> Result<(i64, Option<DateTime<Utc>>)> {
    let mut tx = admission::brain_preparation_tx(
        &state.pool,
        auth.user.id,
        auth.device_id,
        brain,
        BrainAccess::READ,
    )
    .await?
    .tx;
    sqlx::query("SET LOCAL recollect.bulk_qualification=on")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL jit=off").execute(&mut *tx).await?;
    sqlx::query("SET LOCAL statement_timeout='1500ms'")
        .execute(&mut *tx)
        .await?;
    let epoch = graph::preparation_epoch(&mut tx, brain).await?;
    let limit = if inputs.is_empty() { 250_i64 } else { 251 };
    let mut after: Option<Uuid> = None;
    loop {
        let rows:Vec<(Uuid,Uuid,Uuid,Option<DateTime<Utc>>)>=sqlx::query_as(&format!("WITH current_topics AS MATERIALIZED ({CURRENT_TOPICS}) SELECT i.id,i.source_id,i.source_version_id,t.organization_expires_at FROM knowledge_topic_sources m JOIN current_topics t ON t.brain_id=m.brain_id AND t.id=m.topic_id JOIN knowledge_source_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE m.brain_id=$1 AND m.topic_id=$2 AND (cardinality($3::uuid[])=0 OR i.id=ANY($3)) AND (NOT $4 OR(NOT EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions x WHERE x.topic_id=m.topic_id AND x.input_id=m.input_id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_source_splits x WHERE x.original_topic_id=m.topic_id AND x.input_id=m.input_id))) AND ($6::uuid IS NULL OR i.id>$6) ORDER BY i.id LIMIT $5"))
        .bind(brain).bind(topic).bind(inputs).bind(visible).bind(limit).bind(after).fetch_all(&mut *tx).await?;
        if rows.is_empty() || rows.len() > 250 {
            return Err(changed());
        }
        let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
        let current: Vec<Uuid> = sqlx::query_scalar(
            "SELECT input_id FROM recollect_organization_current_source_ids($1,$2)",
        )
        .bind(brain)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await?;
        let mut scope = GraphSelection {
            kind: "knowledge".into(),
            ..Default::default()
        };
        let entities: Vec<_> = rows
            .iter()
            .filter(|r| current.contains(&r.0))
            .map(|r| graph::Entity {
                kind: "source_version".into(),
                id: r.1,
                revision_id: r.2,
            })
            .collect();
        let q = retrieval::graph::qualify(
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
            .filter(|r| current.contains(&r.0) && q.nodes.contains_key(&source_key(r.2)))
            .map(|r| r.0)
            .collect();
        if !eligible.is_empty() && inputs.iter().all(|i| eligible.contains(i)) {
            let deadline = rows.iter().filter_map(|r| r.3).chain(q.deadline).min();
            tx.commit().await?;
            return Ok((epoch, deadline));
        }
        if !inputs.is_empty() || rows.len() < limit as usize {
            return Err(changed());
        }
        // Pin needs one current witness, not the first immutable input IDs. Stale
        // history must not hide a later valid contributor. Each qualification stays
        // within250 inputs; the caller's whole5s preparation deadline bounds scans.
        after = rows.last().map(|r| r.0);
    }
}
