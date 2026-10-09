use super::*;
use crate::{auth::Auth, db, retrieval};
use axum::{
    Json as ResponseJson,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::sync::Semaphore;

pub(super) static CAPACITY: Semaphore = Semaphore::const_new(2);

pub(super) struct Selected {
    pub(super) windowed: bool,
    pub preparation_epoch: i64,
    pub view: GraphView,
    pub nodes: BTreeMap<String, GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub deadline: Option<DateTime<Utc>>,
    pub(super) projections: Vec<(GraphGeneration, Descriptor)>,
}
async fn projection(
    tx: &mut Tx<'_>,
    brain: Uuid,
    kind: &str,
    snapshot: Option<Uuid>,
    inputs: &[Uuid],
) -> Result<(GraphGeneration, Descriptor)> {
    let row:Option<(Json<GraphGeneration>,Json<Descriptor>)>=sqlx::query_as("SELECT to_jsonb(g)-'descriptor',descriptor FROM graph_generations g WHERE brain_id=$1 AND kind=$2 AND snapshot_id IS NOT DISTINCT FROM $3 AND input_snapshot_ids=$4 AND state='ready' AND (kind<>'combined' OR (input_epoch=(SELECT graph_link_epoch FROM brains WHERE id=$1) AND adapter=$5)) ORDER BY created_at DESC,id DESC LIMIT 1")
        .bind(brain).bind(kind).bind(snapshot).bind(inputs).bind(combined::LINKER).fetch_optional(&mut **tx).await?;
    row.map(|(Json(g),Json(d))|(g,d)).ok_or_else(||failure("graph_projection_missing","This exact input has no ready graph generation. Inspect its processing status or rebuild it."))
}
pub(super) async fn select(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    scope: GraphSelection,
    node_limit: usize,
    edge_limit: usize,
) -> Result<Selected> {
    select_at(
        state,
        tx,
        auth,
        brain,
        scope,
        (node_limit, edge_limit),
        None,
    )
    .await
}

pub(super) async fn select_at(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    scope: GraphSelection,
    limits: (usize, usize),
    at: Option<DateTime<Utc>>,
) -> Result<Selected> {
    select_inner(state, tx, auth, brain, scope, limits, at, None).await
}

pub(super) enum Window {
    Page { offset: usize, limit: usize },
    Neighbors { center: String, direction: String },
}

pub(super) async fn select_windowed(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    scope: GraphSelection,
    window: Window,
) -> Result<Selected> {
    select_inner(
        state,
        tx,
        auth,
        brain,
        scope,
        (NODE_LIMIT, EDGE_LIMIT),
        None,
        Some(window),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn select_inner(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    mut scope: GraphSelection,
    limits: (usize, usize),
    at: Option<DateTime<Utc>>,
    window: Option<Window>,
) -> Result<Selected> {
    let (node_limit, edge_limit) = limits;
    if !matches!(scope.kind.as_str(), "repository" | "knowledge" | "combined")
        || !matches!(
            scope.mode.as_str(),
            "investigation" | "strict_accepted" | "strict_operational"
        )
        || (scope.kind != "repository" && scope.snapshot_id.is_some())
        || (scope.kind == "combined"
            && (scope.manifest_revision_id.is_none() || scope.selection.environment_id.is_none()))
        || scope.relations.len() > 20
        || scope.relations.iter().any(|r| {
            !STRUCTURAL.contains(&r.as_str())
                && !KNOWLEDGE.contains(&r.as_str())
                && r != "terraform_module"
        })
    {
        return Err(Error::invalid(
            "Choose a supported graph kind, mode and relation selection.",
        ));
    }
    scope.relations.sort();
    scope.relations.dedup();
    db::require_role(tx, brain, false).await?;
    let prepared_at = preparation_epoch(tx, brain).await?;
    retrieval::graph::authorize(tx, auth, brain, &scope).await?;
    if scope.kind == "repository" && scope.snapshot_id.is_none() {
        if scope.selection.repository_ids.len() != 1 {
            return Err(Error::invalid(
                "Select one exact snapshot or one repository for this structural view.",
            ));
        }
        let repo = scope.selection.repository_ids[0];
        scope.snapshot_id = if let Some(id) = scope.manifest_revision_id {
            crate::memory::manifest(tx, brain, id)
                .await?
                .entries
                .into_iter()
                .find(|e| e.repository_id == repo)
                .and_then(|e| e.snapshot_id)
        } else if scope.selection.environment_id.is_none() {
            sqlx::query_scalar("SELECT id FROM repository_snapshots WHERE brain_id=$1 AND repository_id=$2 AND created_at<=coalesce($3,clock_timestamp()) ORDER BY created_at DESC,id DESC LIMIT 1")
                .bind(brain).bind(repo).bind(at).fetch_optional(&mut **tx).await?
        } else {
            None
        };
        if scope.snapshot_id.is_none() {
            return Err(failure(
                "graph_input_unavailable",
                "The exact selected repository input is unavailable.",
            ));
        }
    }
    if let Some(snapshot) = scope.snapshot_id {
        crate::publication::snapshot(tx, brain, snapshot).await?;
    }
    let ids = if scope.kind == "combined" {
        combined::manifest_inputs(
            tx,
            brain,
            scope.manifest_revision_id.ok_or_else(Error::missing)?,
        )
        .await?
    } else {
        vec![]
    };
    let (g, d) = projection(tx, brain, &scope.kind, scope.snapshot_id, &ids).await?;
    let mut projections = vec![];
    if scope.kind == "combined" {
        let bytes:i64=sqlx::query_scalar("SELECT coalesce(sum(descriptor_bytes),0)::bigint FROM (
          SELECT DISTINCT ON (g.snapshot_id) g.descriptor_bytes,g.snapshot_id FROM graph_generations g JOIN repository_snapshots s ON s.brain_id=g.brain_id AND s.id=g.snapshot_id
          WHERE g.brain_id=$1 AND g.kind='repository' AND g.state='ready' AND s.id=ANY($2) AND (cardinality($3::uuid[])=0 OR s.repository_id=ANY($3))
          ORDER BY g.snapshot_id,g.created_at DESC,g.id DESC) inputs")
            .bind(brain).bind(&ids).bind(&scope.selection.repository_ids).fetch_one(&mut **tx).await?;
        let combined_bytes: i32 = sqlx::query_scalar(
            "SELECT descriptor_bytes FROM graph_generations WHERE brain_id=$1 AND id=$2",
        )
        .bind(brain)
        .bind(g.id)
        .fetch_one(&mut **tx)
        .await?;
        if bytes + i64::from(combined_bytes) > 64 * 1024 * 1024 {
            return Err(Error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "graph_read_input_too_large",
                "The selected graph descriptors exceed the 64 MiB read budget. Select fewer repositories.",
            ));
        }
        let snapshots:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM repository_snapshots WHERE brain_id=$1 AND id=ANY($2) AND (cardinality($3::uuid[])=0 OR repository_id=ANY($3)) ORDER BY id")
            .bind(brain).bind(&ids).bind(&scope.selection.repository_ids).fetch_all(&mut **tx).await?;
        for snapshot in snapshots {
            projections.push(projection(tx, brain, "repository", Some(snapshot), &[]).await?);
        }
    }
    projections.push((g.clone(), d));
    let mut entities: Vec<_> = projections
        .iter()
        .flat_map(|(_, d)| d.nodes.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let windowed = window.is_some();
    let center = match &window {
        Some(Window::Neighbors { center, .. }) => Some(center.clone()),
        _ => None,
    };
    let mut next_offset = None;
    if let Some(window) = window {
        entities = match window {
            Window::Page { offset, limit } => {
                let next = offset.checked_add(limit).ok_or_else(|| {
                    Error::invalid("The graph page offset is outside the supported view.")
                })?;
                next_offset = (next < entities.len()).then_some(next);
                entities.into_iter().skip(offset).take(limit).collect()
            }
            Window::Neighbors { center, direction } => {
                let entity = entities
                    .iter()
                    .find(|e| e.key() == center)
                    .cloned()
                    .ok_or_else(|| {
                        Error::invalid(
                            "The center must be an eligible entity in this exact graph view.",
                        )
                    })?;
                let seed = retrieval::graph::qualify(
                    state,
                    tx,
                    auth,
                    brain,
                    &mut scope,
                    std::slice::from_ref(&entity),
                    (node_limit, at),
                )
                .await?;
                if !seed.nodes.contains_key(&center) {
                    return Err(Error::invalid(
                        "The center must be an eligible entity in this exact graph view.",
                    ));
                }
                let keys = adapter::neighbor_keys(
                    state,
                    &projections,
                    &scope.relations,
                    &center,
                    &direction,
                    250,
                )
                .await?;
                // Keep the seed even when its key sorts after a full page of
                // neighbors. Qualification and reachability both require it.
                let neighbors = entities
                    .into_iter()
                    .filter(|e| e.key() != center && keys.contains(&e.key()))
                    .take(249);
                std::iter::once(entity).chain(neighbors).collect()
            }
        };
        let keys: BTreeSet<_> = entities.iter().map(Entity::key).collect();
        let mut candidates: Vec<_> = projections
            .iter()
            .flat_map(|(_, d)| d.edges.iter())
            .filter(|e| {
                keys.contains(&e.from)
                    && keys.contains(&e.to)
                    && (scope.relations.is_empty() || scope.relations.contains(&e.relation))
            })
            .collect();
        candidates.sort_by_key(|e| {
            (
                center.as_ref().is_some_and(|c| e.from != *c && e.to != *c),
                e.id,
            )
        });
        let edge_ids: BTreeSet<_> = candidates.into_iter().take(1000).map(|e| e.id).collect();
        for (_, descriptor) in &mut projections {
            descriptor.nodes.retain(|e| keys.contains(&e.key()));
            descriptor.edges.retain(|e| edge_ids.contains(&e.id));
        }
    }
    let mut qualified = retrieval::graph::qualify(
        state,
        tx,
        auth,
        brain,
        &mut scope,
        &entities,
        (node_limit, at),
    )
    .await?;
    let mut issues = vec![];
    // Window qualification is deliberately partial even when it happens to
    // contain every candidate. It cannot establish global path absence.
    if windowed {
        note(&mut qualified.coverage, "graph_window");
    }
    let mut deadline = qualified.deadline;
    if g.kind == "combined" {
        // Retention policy can change after projection. Every exact input is
        // required, even when repository/path selection hides all of its nodes.
        let current: Option<DateTime<Utc>> = sqlx::query_scalar("SELECT min(recollect_retention_deadline(brain_id,'repository',created_at)) FROM repository_snapshots WHERE brain_id=$1 AND id=ANY($2)")
            .bind(brain).bind(&g.input_snapshot_ids).fetch_one(&mut **tx).await?;
        deadline = [deadline, current].into_iter().flatten().min();
    }
    for (_, d) in &mut projections {
        issues.extend(
            d.issues
                .iter()
                .filter(|i| qualified.nodes.contains_key(&i.source))
                .cloned(),
        );
        d.nodes.retain(|n| qualified.nodes.contains_key(&n.key()));
        d.edges.retain(|e| {
            qualified.nodes.contains_key(&e.from)
                && qualified.nodes.contains_key(&e.to)
                && (scope.relations.is_empty() || scope.relations.contains(&e.relation))
        });
    }
    let edges: Vec<_> = projections
        .iter()
        .flat_map(|(_, d)| d.edges.iter().cloned())
        .collect();
    if qualified.nodes.len() > node_limit || edges.len() > edge_limit {
        return Err(selection_limit(node_limit));
    }
    let memory_epoch = epoch(tx, brain).await?;
    if g.kind == "knowledge" && g.input_epoch != memory_epoch {
        note(&mut qualified.coverage, "knowledge_generation_stale");
    }
    for (input, _) in &projections {
        if input.unresolved > 0 && input.kind != "combined" {
            note(
                &mut qualified.coverage,
                if input.kind == "knowledge" {
                    "unavailable_knowledge_supports"
                } else {
                    "unresolved_extraction_targets"
                },
            );
        }
        if input.ambiguous > 0 {
            note(&mut qualified.coverage, "ambiguous_extraction_targets");
        }
        if input.unsupported > 0 {
            note(&mut qualified.coverage, "unsupported_extraction_relations");
        }
    }
    if !issues.is_empty() {
        note(&mut qualified.coverage, "unresolved_cross_repository_links");
    }
    if qualified.nodes.len() < entities.len() {
        note(
            &mut qualified.coverage,
            "projection_inputs_outside_current_view",
        );
    }
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM graph_generations WHERE brain_id=$1 AND kind=$2 AND snapshot_id IS NOT DISTINCT FROM $3 AND state IN ('queued','running') AND created_at>$4 AND input_snapshot_ids=$5)")
        .bind(brain).bind(&g.kind).bind(g.snapshot_id).bind(g.created_at).bind(&g.input_snapshot_ids).fetch_one(&mut **tx).await?;
    if pending {
        note(&mut qualified.coverage, "graph_rebuild_pending");
    }
    let relations: BTreeSet<_> = edges.iter().map(|e| e.relation.clone()).collect();
    let view = GraphView {
        brain_id: brain,
        expires_at: deadline,
        scope,
        scope_id: qualified.scope_id,
        inputs: projections
            .iter()
            .filter(|(g, _)| g.kind != "combined")
            .map(|(g, _)| g.clone())
            .collect(),
        link_issues_total: issues.len(),
        link_issues: issues.into_iter().take(100).collect(),
        generation: g,
        state: if qualified.coverage.partial {
            "partial"
        } else {
            "ready"
        }
        .into(),
        memory_epoch,
        nodes: vec![],
        total_nodes: qualified.nodes.len(),
        total_edges: edges.len(),
        offset: 0,
        next_offset,
        relations: relations.into_iter().collect(),
        coverage: qualified.coverage,
    };
    Ok(Selected {
        windowed,
        preparation_epoch: prepared_at,
        view,
        nodes: qualified.nodes,
        edges,
        deadline,
        projections,
    })
}
pub(super) async fn final_gate(
    tx: &mut Tx<'_>,
    auth: &Auth,
    selection: &Selected,
) -> Result<DateTime<Utc>> {
    db::require_role(tx, selection.view.brain_id, false).await?;
    preparation_unchanged(tx, selection.view.brain_id, selection.preparation_epoch).await?;
    retrieval::graph::authorize(tx, auth, selection.view.brain_id, &selection.view.scope).await?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    if selection.deadline.is_some_and(|t| t <= now) {
        return Err(failure(
            "graph_retention_changed",
            "An input reached its retention deadline during this graph request. Read the current view again.",
        ));
    }
    Ok(now)
}

/// Partial pages can survive unrelated captures only after every buffered value
/// is requalified under the fresh canonical lock. Full reach/path consumers keep
/// the strict epoch gate: a page cannot prove absence or whole-graph stability.
pub(super) async fn interactive_final_gate(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    selection: &mut Selected,
) -> Result<DateTime<Utc>> {
    let brain = selection.view.brain_id;
    db::require_role(tx, brain, false).await?;
    let current = preparation_epoch(tx, brain).await?;
    if !selection.windowed || current == selection.preparation_epoch {
        return final_gate(tx, auth, selection).await;
    }
    let changed = || {
        failure(
            "graph_preparation_changed",
            "Selected graph inputs changed during preparation. Read the current view again.",
        )
    };
    retrieval::graph::authorize(tx, auth, brain, &selection.view.scope).await?;
    for (generation, descriptor) in &selection.projections {
        let row: Option<(Json<GraphGeneration>, bool)> = sqlx::query_as(
            "SELECT to_jsonb(g)-'descriptor',descriptor @> jsonb_build_object('nodes',$3::jsonb,'edges',$4::jsonb) FROM graph_generations g WHERE brain_id=$1 AND id=$2 AND state='ready'",
        ).bind(brain).bind(generation.id).bind(Json(&descriptor.nodes)).bind(Json(&descriptor.edges))
            .fetch_optional(&mut **tx).await?;
        let Some((Json(actual), true)) = row else {
            return Err(changed());
        };
        if serde_json::to_value(actual).map_err(|_| changed())?
            != serde_json::to_value(generation).map_err(|_| changed())?
        {
            return Err(changed());
        }
        if generation.kind != "knowledge" {
            descriptor::available(tx, generation).await?;
        }
    }
    let entities: Vec<_> = selection
        .projections
        .iter()
        .flat_map(|(_, d)| d.nodes.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut scope = selection.view.scope.clone();
    let qualified = retrieval::graph::qualify(
        state,
        tx,
        auth,
        brain,
        &mut scope,
        &entities,
        (NODE_LIMIT, None),
    )
    .await?;
    if qualified.scope_id != selection.view.scope_id
        || serde_json::to_value(&scope).map_err(|_| changed())?
            != serde_json::to_value(&selection.view.scope).map_err(|_| changed())?
        || serde_json::to_value(&qualified.nodes).map_err(|_| changed())?
            != serde_json::to_value(&selection.nodes).map_err(|_| changed())?
    {
        return Err(changed());
    }
    selection.deadline = [selection.deadline, qualified.deadline]
        .into_iter()
        .flatten()
        .min();
    selection.view.expires_at = selection.deadline;
    selection.view.memory_epoch = epoch(tx, brain).await?;
    if selection.view.generation.kind == "knowledge"
        && selection.view.generation.input_epoch != selection.view.memory_epoch
    {
        note(&mut selection.view.coverage, "knowledge_generation_stale");
        selection.view.state = "partial".into();
    }
    selection.preparation_epoch = current;
    final_gate(tx, auth, selection).await
}

pub(super) async fn verify(state: &AppState, selected: &Selected) -> Result<()> {
    for (g, d) in &selected.projections {
        adapter::verify_generation(state, g).await?;
        for batch in d.nodes.chunks(500) {
            adapter::verify_nodes(state, g, batch).await?;
        }
        for batch in d.edges.chunks(500) {
            adapter::verify_edges(state, g, batch).await?;
        }
    }
    Ok(())
}

#[utoipa::path(post,path="/api/brains/{brain}/graph/view",operation_id="graphView",params(("brain"=Uuid,Path)),request_body=GraphViewRequest,responses((status=200,body=GraphView)))]
pub async fn view(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    ResponseJson(input): ResponseJson<GraphViewRequest>,
) -> Result<ResponseJson<GraphView>> {
    let _permit = CAPACITY
        .try_acquire()
        .map_err(|_| failure("graph_busy", "Graph reads are busy. Try again shortly."))?;
    tokio::time::timeout(Duration::from_secs(15), async {
        if !input.windowed && input.offset > NODE_LIMIT {
            return Err(Error::invalid(
                "The graph page offset is outside the supported view.",
            ));
        }
        let mut tx = auth.preparation_tx(&state.pool).await?;
        sqlx::query("SET LOCAL statement_timeout='2s'")
            .execute(&mut *tx)
            .await?;
        let mut selected = if input.windowed {
            select_windowed(
                &state,
                &mut tx,
                &auth,
                brain,
                input.scope,
                Window::Page {
                    offset: input.offset,
                    limit: 100,
                },
            )
            .await?
        } else {
            select(
                &state,
                &mut tx,
                &auth,
                brain,
                input.scope,
                NODE_LIMIT,
                EDGE_LIMIT,
            )
            .await?
        };
        verify(&state, &selected).await?;
        tx.commit().await?;
        let mut tx = auth.publication_tx(&state.pool).await?;
        sqlx::query("SET LOCAL statement_timeout='2s'")
            .execute(&mut *tx)
            .await?;
        interactive_final_gate(&state, &mut tx, &auth, &mut selected).await?;
        selected.view.nodes = selected
            .nodes
            .into_values()
            .skip(if input.windowed { 0 } else { input.offset })
            .take(100)
            .collect();
        selected.view.offset = input.offset;
        tx.commit().await?;
        Ok(ResponseJson(selected.view))
    })
    .await
    .map_err(|_| {
        failure(
            "graph_timeout",
            "This graph view exceeded its time limit. Select a smaller scope.",
        )
    })?
}

#[utoipa::path(post,path="/api/brains/{brain}/graph/path",operation_id="graphPath",params(("brain"=Uuid,Path)),request_body=GraphPathRequest,responses((status=200,body=GraphPath)))]
pub async fn path(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    ResponseJson(input): ResponseJson<GraphPathRequest>,
) -> Result<ResponseJson<GraphPath>> {
    let _permit = CAPACITY
        .try_acquire()
        .map_err(|_| failure("graph_busy", "Graph reads are busy. Try again shortly."))?;
    tokio::time::timeout(Duration::from_secs(15), async {
        if !(1..=8).contains(&input.max_hops)
            || !matches!(input.direction.as_str(), "outgoing" | "incoming" | "both")
            || input.start.len() > 100
            || input.end.len() > 100
        {
            return Err(Error::invalid(
                "Choose exact graph entities, one to eight hops and a supported direction.",
            ));
        }
        let mut tx = auth.preparation_tx(&state.pool).await?;
        sqlx::query("SET LOCAL statement_timeout='2s'")
            .execute(&mut *tx)
            .await?;
        let selected = select(
            &state,
            &mut tx,
            &auth,
            brain,
            input.scope.clone(),
            NODE_LIMIT,
            EDGE_LIMIT,
        )
        .await?;
        if !selected.nodes.contains_key(&input.start) || !selected.nodes.contains_key(&input.end) {
            return Err(Error::invalid(
                "Both path endpoints must be eligible members of this exact graph view.",
            ));
        }
        // Verify the selected physical input before a no-path response can be
        // mistaken for absence in an erased, missing or damaged projection.
        verify(&state, &selected).await?;
        let found = if input.start == input.end {
            Some((vec![input.start.clone()], vec![]))
        } else {
            adapter::path(
                &state,
                &selected.view.generation,
                &selected
                    .projections
                    .iter()
                    .map(|(g, _)| g.id)
                    .collect::<Vec<_>>(),
                &input,
                &selected.nodes.keys().cloned().collect::<Vec<_>>(),
                &selected.edges,
            )
            .await?
        };
        let mut nodes = vec![];
        let mut edges = vec![];
        let status = if let Some((keys, ids)) = found {
            if keys.len() != ids.len() + 1
                || ids.len() > input.max_hops
                || keys.first() != Some(&input.start)
                || keys.last() != Some(&input.end)
                || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
            {
                return Err(failure(
                    "graph_response_invalid",
                    "The returned path did not match the requested bounds.",
                ));
            }
            for key in &keys {
                nodes.push(selected.nodes.get(key).cloned().ok_or_else(|| {
                    failure(
                        "graph_response_invalid",
                        "A returned path entity was outside the canonical view.",
                    )
                })?);
            }
            for (position, id) in ids.into_iter().enumerate() {
                let edge = selected.edges.iter().find(|e| e.id == id).ok_or_else(|| {
                    failure(
                        "graph_response_invalid",
                        "A returned edge was outside the canonical view.",
                    )
                })?;
                let forward = edge.from == keys[position] && edge.to == keys[position + 1];
                let backward = edge.to == keys[position] && edge.from == keys[position + 1];
                if !match input.direction.as_str() {
                    "outgoing" => forward,
                    "incoming" => backward,
                    _ => forward || backward,
                } {
                    return Err(failure(
                        "graph_response_invalid",
                        "The returned path direction did not match its canonical edges.",
                    ));
                }
                edges.push(edge.clone());
            }
            "path"
        } else {
            "no_path_within_bound"
        };
        tx.commit().await?;
        let mut tx = auth.publication_tx(&state.pool).await?;
        sqlx::query("SET LOCAL statement_timeout='2s'")
            .execute(&mut *tx)
            .await?;
        final_gate(&mut tx, &auth, &selected).await?;
        tx.commit().await?;
        Ok(ResponseJson(GraphPath {
            view: selected.view,
            status: status.into(),
            direction: input.direction,
            max_hops: input.max_hops,
            nodes,
            edges,
        }))
    })
    .await
    .map_err(|_| {
        failure(
            "graph_timeout",
            "This graph path exceeded its time limit. Select a smaller scope.",
        )
    })?
}
