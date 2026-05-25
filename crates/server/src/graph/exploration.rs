use super::*;
use crate::auth::Auth;
use axum::{
    Json as ResponseJson,
    extract::{Path, State},
};
use std::time::Duration;

pub(super) const DISPLAY_NODES: usize = 500;
const DISPLAY_EDGES: usize = 2000;

fn display_limit() -> Error {
    Error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "graph_display_too_large",
        "This graph exceeds 500 displayed entities or 2,000 relationships. Select a smaller scope or explore from an entity with fewer hops.",
    )
}
fn invalid_response() -> Error {
    failure(
        "graph_response_invalid",
        "Native reachability did not match the canonical entities, relationships or requested bounds.",
    )
}

// This validates native witnesses; it does not discover paths or calculate
// reachability. The shortest-path selector owns those operations.
pub(super) fn validate_reach(
    selected: &read::Selected,
    input: &GraphExploreRequest,
    reach: &[GraphReach],
    complete_targets: bool,
) -> Result<BTreeSet<String>> {
    let center = input.center.as_ref().ok_or_else(invalid_response)?;
    let edges: BTreeMap<_, _> = selected.edges.iter().map(|e| (e.id, e)).collect();
    let mut included = BTreeSet::from([center.clone()]);
    for route in reach {
        if !included.insert(route.key.clone())
            || route.nodes.len() != route.edges.len() + 1
            || route.edges.is_empty()
            || route.edges.len() > input.max_hops
            || route.nodes.first() != Some(center)
            || route.nodes.last() != Some(&route.key)
            || route.nodes.iter().collect::<BTreeSet<_>>().len() != route.nodes.len()
            || route.edges.iter().collect::<BTreeSet<_>>().len() != route.edges.len()
            || route.nodes.iter().any(|k| !selected.nodes.contains_key(k))
        {
            return Err(invalid_response());
        }
        for (position, id) in route.edges.iter().enumerate() {
            let edge = edges.get(id).ok_or_else(invalid_response)?;
            let forward =
                edge.from == route.nodes[position] && edge.to == route.nodes[position + 1];
            let backward =
                edge.to == route.nodes[position] && edge.from == route.nodes[position + 1];
            if !match input.direction.as_str() {
                "outgoing" => forward,
                "incoming" => backward,
                _ => forward || backward,
            } {
                return Err(invalid_response());
            }
        }
    }
    // Each witness's intermediate entities must themselves be represented.
    if complete_targets
        && reach
            .iter()
            .any(|r| r.nodes.iter().any(|k| !included.contains(k)))
    {
        return Err(invalid_response());
    }
    Ok(included)
}

#[utoipa::path(post,path="/api/brains/{brain}/graph/explore",operation_id="graphExplore",params(("brain"=Uuid,Path)),request_body=GraphExploreRequest,responses((status=200,body=GraphExploration)))]
pub async fn explore(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    ResponseJson(input): ResponseJson<GraphExploreRequest>,
) -> Result<ResponseJson<GraphExploration>> {
    let _permit = read::CAPACITY
        .try_acquire()
        .map_err(|_| failure("graph_busy", "Graph reads are busy. Try again shortly."))?;
    tokio::time::timeout(Duration::from_secs(15), async {
        if !(1..=8).contains(&input.max_hops)
            || !matches!(input.direction.as_str(), "outgoing" | "incoming" | "both")
            || input.center.as_ref().is_some_and(|k| k.is_empty() || k.len() > 100)
        {
            return Err(Error::invalid("Choose an eligible center or overview, one to eight hops and a supported direction."));
        }
        let mut tx = auth.tx(&state.pool).await?;
        sqlx::query("SET LOCAL statement_timeout='2s'").execute(&mut *tx).await?;
        let selected = read::select(&state, &mut tx, &auth, brain, input.scope.clone(), NODE_LIMIT, EDGE_LIMIT).await?;
        if input.center.as_ref().is_some_and(|k| !selected.nodes.contains_key(k)) {
            return Err(Error::invalid("The center must be an eligible entity in this exact graph view."));
        }
        // Bound an overview before spending time verifying its projection.
        if input.center.is_none() && (selected.nodes.len() > DISPLAY_NODES || selected.edges.len() > DISPLAY_EDGES) {
            return Err(display_limit());
        }
        read::verify(&state, &selected).await?;
        let (included, reach) = if let Some(center) = &input.center {
            let mut reach = adapter::reach(&state, &selected, &input, DISPLAY_NODES + 1).await?;
            if reach.len() + 1 > DISPLAY_NODES {
                return Err(display_limit());
            }
            let included = validate_reach(&selected, &input, &reach, true)?;
            reach.insert(0, GraphReach { key: center.clone(), nodes: vec![center.clone()], edges: vec![] });
            (included, reach)
        } else {
            (selected.nodes.keys().cloned().collect(), vec![])
        };
        let edges: Vec<_> = selected.edges.iter().filter(|e| included.contains(&e.from) && included.contains(&e.to)).cloned().collect();
        if edges.len() > DISPLAY_EDGES {
            return Err(display_limit());
        }
        read::final_gate(&mut tx, &auth, &selected).await?;
        tx.commit().await?;
        Ok(ResponseJson(GraphExploration {
            expires_at: selected.deadline,
            view: selected.view,
            center: input.center,
            direction: input.direction,
            max_hops: input.max_hops,
            nodes: selected.nodes.into_values().filter(|n| included.contains(&n.key)).collect(),
            edges,
            reach,
        }))
    }).await.map_err(|_| failure("graph_timeout", "This exploration exceeded its time limit. Select a smaller scope or hop bound."))?
}
