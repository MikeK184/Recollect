//! Graph discovery for the canonical recall handler. Traversal stays native;
//! this adapter only qualifies inputs and validates/attributes its witnesses.
use super::*;
use crate::auth::Auth;
use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};

const CANDIDATES: usize = 100;
const PHASE: Duration = Duration::from_secs(15);

pub(crate) fn validate(input: &RecallRequest) -> Result<()> {
    let options = input.graph.clone().unwrap_or_default();
    if input.knowledge_at.is_some() || input.mode == "history" {
        return Err(Error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "graph_history_unavailable",
            "Graph recall has no historical projection. Remove graph or choose current knowledge.",
        ));
    }
    if input.channels.len() < 2
        || !(1..=3).contains(&options.max_hops)
        || !matches!(options.direction.as_str(), "incoming" | "outgoing" | "both")
        || !matches!(
            options.kind.as_str(),
            "repository" | "combined" | "knowledge"
        )
        || (options.kind == "repository" && input.selection.repository_ids.len() != 1)
        || (options.kind == "combined"
            && (input.manifest_revision_id.is_none() || input.selection.environment_id.is_none()))
        || options.relations.len() > 20
        || options.relations.iter().any(|r| {
            !STRUCTURAL.contains(&r.as_str())
                && !KNOWLEDGE.contains(&r.as_str())
                && r != "terraform_module"
        })
    {
        return Err(Error::invalid(
            "Graph recall needs another anchor channel, a supported kind/direction, one to three hops, and supported relations. Repository needs one repository; combined needs an environment and manifest.",
        ));
    }
    Ok(())
}

pub(crate) struct Admission {
    selected: read::Selected,
    options: RecallGraphOptions,
    spent: Duration,
}
pub(crate) struct Output {
    admission: Admission,
    pub items: Vec<RecallItem>,
    pub status: RecallGraph,
}
pub(crate) async fn prepare(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    input: &RecallRequest,
    at: DateTime<Utc>,
    previous: Option<Admission>,
) -> Result<Admission> {
    let _permit = read::CAPACITY
        .try_acquire()
        .map_err(|_| failure("graph_busy", "Graph reads are busy. Try again shortly."))?;
    let started = Instant::now();
    let spent = previous.map_or(Duration::ZERO, |a| a.spent);
    let options = input.graph.clone().unwrap_or_default();
    let scope = GraphSelection {
        kind: options.kind.clone(),
        snapshot_id: None,
        selection: input.selection.clone(),
        operation_id: input.operation_id,
        collection_id: input.collection_id,
        manifest_revision_id: input.manifest_revision_id,
        fact_at: input.fact_at,
        mode: input.mode.clone(),
        relations: options.relations.clone(),
    };
    let selected = tokio::time::timeout(
        PHASE.saturating_sub(spent),
        read::select_at(
            state,
            tx,
            auth,
            brain,
            scope,
            (NODE_LIMIT, EDGE_LIMIT),
            Some(at),
        ),
    )
    .await
    .map_err(|_| timeout())??;
    Ok(Admission {
        selected,
        options,
        spent: spent + started.elapsed(),
    })
}
fn timeout() -> Error {
    failure(
        "graph_timeout",
        "The graph recall phase exceeded its time limit. Narrow the scope or hop bound.",
    )
}

pub(crate) async fn preflight(state: &AppState, mut admission: Admission) -> Result<Admission> {
    let _permit = read::CAPACITY
        .try_acquire()
        .map_err(|_| failure("graph_busy", "Graph reads are busy. Try again shortly."))?;
    let started = Instant::now();
    tokio::time::timeout(
        PHASE.saturating_sub(admission.spent),
        read::verify(state, &admission.selected),
    )
    .await
    .map_err(|_| timeout())??;
    admission.spent += started.elapsed();
    Ok(admission)
}

pub(crate) async fn expand(
    state: &AppState,
    admission: Admission,
    ranked: &[&RecallItem],
) -> Result<Output> {
    let _permit = read::CAPACITY
        .try_acquire()
        .map_err(|_| failure("graph_busy", "Graph reads are busy. Try again shortly."))?;
    let remaining = PHASE.saturating_sub(admission.spent);
    tokio::time::timeout(remaining, async {
        let selected = &admission.selected;
        let anchors: Vec<_> = ranked
            .iter()
            .map(|r| key(&r.kind, r.revision_id))
            .filter(|k| selected.nodes.contains_key(k))
            .take(3)
            .collect();
        let mut status = RecallGraph {
            view: selected.view.clone(),
            anchors: anchors.clone(),
            candidates: 0,
            state: "no_eligible_anchors".into(),
            expires_at: selected.deadline,
        };
        let mut candidates = BTreeMap::<(String, Uuid), (usize, usize, GraphReach)>::new();
        if !anchors.is_empty() {
            read::verify(state, selected).await?;
            for (ordinal, anchor) in anchors.iter().enumerate() {
                let input = GraphExploreRequest {
                    scope: selected.view.scope.clone(),
                    windowed: false,
                    center: Some(anchor.clone()),
                    direction: admission.options.direction.clone(),
                    max_hops: admission.options.max_hops,
                };
                let reach = adapter::reach(state, selected, &input, CANDIDATES + 1).await?;
                // Prefixes can omit an intermediate target; every intermediate
                // must still be a qualified entity with a valid canonical edge.
                exploration::validate_reach(selected, &input, &reach, false)?;
                if reach.len() > CANDIDATES {
                    note(&mut status.view.coverage, "graph_candidate_limit");
                }
                for route in reach
                    .into_iter()
                    .take(CANDIDATES)
                    .filter(|r| !anchors.contains(&r.key))
                {
                    let node = &selected.nodes[&route.key];
                    let identity = (node.evidence.kind.clone(), node.evidence.id);
                    let priority = (route.edges.len(), ordinal);
                    if candidates
                        .get(&identity)
                        .is_none_or(|old| priority < (old.0, old.1))
                    {
                        candidates.insert(identity, (priority.0, priority.1, route));
                    }
                }
            }
            status.state = if candidates.is_empty() {
                "no_neighbors_within_bound"
            } else {
                "available"
            }
            .into();
        }
        let mut ordered: Vec<_> = candidates.into_iter().collect();
        ordered.sort_by(|a, b| (a.1.0, a.1.1, &a.0).cmp(&(b.1.0, b.1.1, &b.0)));
        if ordered.len() > CANDIDATES {
            note(&mut status.view.coverage, "graph_candidate_limit");
        }
        let edges: BTreeMap<_, _> = selected.edges.iter().map(|e| (e.id, e)).collect();
        let items: Vec<_> = ordered
            .into_iter()
            .take(CANDIDATES)
            .map(|(_, (_, anchor, route))| {
                let mut item = selected.nodes[&route.key].evidence.clone();
                item.qualifications.push("graph_proximity_not_truth".into());
                item.graph_match = Some(RecallGraphMatch {
                    generation_id: selected.view.generation.id,
                    anchor_key: anchors[anchor].clone(),
                    direction: admission.options.direction.clone(),
                    nodes: route
                        .nodes
                        .into_iter()
                        .map(|key| {
                            let r = &selected.nodes[&key].evidence;
                            RecallGraphEntity {
                                key,
                                kind: r.kind.clone(),
                                id: r.id,
                                revision_id: r.revision_id,
                                label: r.label.clone(),
                            }
                        })
                        .collect(),
                    edges: route.edges.iter().map(|id| (*edges[id]).clone()).collect(),
                });
                item
            })
            .collect();
        status.candidates = items.len();
        Ok(Output {
            admission,
            items,
            status,
        })
    })
    .await
    .map_err(|_| timeout())?
}

pub(crate) async fn final_gate(
    tx: &mut Tx<'_>,
    auth: &Auth,
    output: &Output,
) -> Result<DateTime<Utc>> {
    read::final_gate(tx, auth, &output.admission.selected).await
}

/// Requalify only the immutable graph witness; no Neo4j traversal or model
/// call is repeated while an answer is being transmitted/published.
pub(crate) async fn bundle_evidence(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    input: &RecallRequest,
    response: &RecallResponse,
) -> Result<Vec<RecallItem>> {
    let Some(original) = &response.graph else {
        return Ok(vec![]);
    };
    let current = prepare(
        state,
        tx,
        auth,
        response.brain_id,
        input,
        response.knowledge_at,
        None,
    )
    .await?;
    if current.selected.view.generation.id != original.view.generation.id {
        return Err(crate::answers::stale());
    }
    let mut items = BTreeMap::new();
    for item in &response.context.items {
        if let Some(witness) = &item.graph_match {
            for node in &witness.nodes {
                let selected = current
                    .selected
                    .nodes
                    .get(&node.key)
                    .ok_or_else(crate::answers::stale)?;
                if selected.evidence.kind != node.kind
                    || selected.evidence.id != node.id
                    || selected.evidence.revision_id != node.revision_id
                    || selected.evidence.label != node.label
                {
                    return Err(crate::answers::stale());
                }
                items.insert(node.key.clone(), selected.evidence.clone());
            }
            for edge in &witness.edges {
                if !current.selected.edges.iter().any(|current| {
                    serde_json::to_value(current).ok() == serde_json::to_value(edge).ok()
                }) {
                    return Err(crate::answers::stale());
                }
            }
        }
    }
    read::final_gate(tx, auth, &current.selected).await?;
    Ok(items.into_values().collect())
}
