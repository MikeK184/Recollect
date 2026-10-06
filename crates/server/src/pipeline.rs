use crate::{AppState, auth::Auth, db, error::Result};
use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Duration, Utc};
use recollect_protocol::{PipelineFeed, PipelineGraph, PipelineItem};
use sqlx::types::Json as DbJson;
use uuid::Uuid;

#[utoipa::path(get, path="/api/brains/{brain}/pipeline", operation_id="brainPipeline",
    params(("brain"=Uuid,Path)), responses((status=200,body=PipelineFeed)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<PipelineFeed>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    // Existing RLS remains in force. No private operation bindings, raw MCP
    // execution payloads, prompts, model requests or source bodies are selected.
    let rows: Vec<DbJson<PipelineItem>> = sqlx::query_scalar(include_str!("pipeline.sql"))
        .bind(brain)
        .bind(auth.user.id)
        .bind(auth.user.installation_owner)
        .fetch_all(&mut *tx)
        .await?;
    let has_more = rows.len() > 30;
    let mut items: Vec<PipelineItem> = rows.into_iter().take(30).map(|r| r.0).collect();
    let graph: Option<DbJson<PipelineGraph>> = sqlx::query_scalar(
        "SELECT jsonb_build_object('id',g.id,'state',g.state,'input_epoch',g.input_epoch,
         'current_epoch',coalesce((SELECT epoch FROM memory_epochs WHERE brain_id=$1),0),
         'published_at',g.published_at,'node_count',g.node_count,'edge_count',g.edge_count)
         FROM graph_generations g WHERE g.brain_id=$1 AND g.kind='knowledge'
         AND g.state NOT IN ('removed','superseded') ORDER BY g.created_at DESC,g.id DESC LIMIT 1",
    )
    .bind(brain)
    .fetch_optional(&mut *tx)
    .await?;
    let observed_at: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    // Recheck deadlines after the query, including content that expired during
    // this read. Client validity is deliberately short and never renewed locally.
    items.retain(|item| item.expires_at.is_none_or(|at| at > observed_at));
    let valid_until = items
        .iter()
        .filter_map(|i| i.expires_at)
        .fold(observed_at + Duration::seconds(6), |a, b| a.min(b));
    tx.commit().await?;
    Ok(Json(PipelineFeed {
        brain_id: brain,
        observed_at,
        valid_until,
        items,
        has_more,
        graph: graph.map(|g| g.0),
    }))
}
