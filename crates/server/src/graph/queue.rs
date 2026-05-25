use super::*;
use crate::{auth::Auth, commands, db, jobs, publication};
use axum::{
    Json as ResponseJson,
    extract::{Path, Query, State},
    http::HeaderMap,
};

pub(super) async fn enqueue(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    input: &GraphRebuild,
) -> Result<GraphGeneration> {
    if !matches!(input.kind.as_str(), "repository" | "knowledge" | "combined")
        || (input.kind == "repository") != input.snapshot_id.is_some()
        || (input.kind == "combined") != input.manifest_revision_id.is_some()
    {
        return Err(Error::invalid(
            "Choose repository with a snapshot, combined with an exact manifest, or knowledge without either.",
        ));
    }
    if let Some(snapshot) = input.snapshot_id {
        publication::snapshot(tx, brain, snapshot).await?;
    }
    let (input_epoch, snapshot_ids, adapter) = if let Some(manifest) = input.manifest_revision_id {
        (
            combined::origin_epoch(tx, brain).await?,
            combined::manifest_inputs(tx, brain, manifest).await?,
            combined::LINKER,
        )
    } else {
        (epoch(tx, brain).await?, vec![], "canonical-graph-1")
    };
    let id = Uuid::new_v4();
    let audit = db::audit(tx, actor, brain, "graph.rebuild", id, "queued").await?;
    let job = jobs::enqueue_work(tx, actor, brain, audit, id, "graph.project", "heavy").await?;
    let Json(g):Json<GraphGeneration>=sqlx::query_scalar("INSERT INTO graph_generations(id,brain_id,kind,snapshot_id,input_epoch,actor_id,job_id,input_snapshot_ids,adapter) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING to_jsonb(graph_generations)-'descriptor'")
        .bind(id).bind(brain).bind(&input.kind).bind(input.snapshot_id).bind(input_epoch).bind(actor).bind(job).bind(snapshot_ids).bind(adapter).fetch_one(&mut **tx).await?;
    descriptor::available(tx, &g).await?;
    Ok(g)
}

#[utoipa::path(get,path="/api/brains/{brain}/graph",operation_id="graphStatus",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=GraphStatus)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(page): Query<publication::Page>,
) -> Result<ResponseJson<GraphStatus>> {
    let offset = publication::offset(&page)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let generations:Vec<Json<GraphGeneration>>=sqlx::query_scalar("SELECT (to_jsonb(g)-'descriptor') || CASE WHEN (g.kind='repository' AND NOT EXISTS(SELECT 1 FROM repository_snapshots s WHERE s.brain_id=g.brain_id AND s.id=g.snapshot_id AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active')) OR (g.kind='combined' AND EXISTS(SELECT 1 FROM unnest(g.input_snapshot_ids) x WHERE NOT EXISTS(SELECT 1 FROM repository_snapshots s WHERE s.brain_id=g.brain_id AND s.id=x AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active'))) THEN jsonb_build_object('state','removed','error_code','graph_input_unavailable') ELSE '{}'::jsonb END FROM graph_generations g WHERE brain_id=$1 ORDER BY created_at DESC,id DESC LIMIT 20 OFFSET $2")
        .bind(brain).bind(offset).fetch_all(&mut *tx).await?;
    let total = sqlx::query_scalar("SELECT count(*) FROM graph_generations WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    let jobs:Vec<jobs::JobRow>=sqlx::query_as("SELECT * FROM jobs WHERE brain_id=$1 AND kind='graph.project' ORDER BY created_at DESC,id DESC LIMIT 20")
        .bind(brain).fetch_all(&mut *tx).await?;
    let memory_epoch = epoch(&mut tx, brain).await?;
    let link_epoch = combined::origin_epoch(&mut tx, brain).await?;
    tx.commit().await?;
    Ok(ResponseJson(GraphStatus {
        generations: generations.into_iter().map(|g| g.0).collect(),
        jobs: jobs.into_iter().map(Into::into).collect(),
        total,
        offset,
        memory_epoch,
        link_epoch,
    }))
}

#[utoipa::path(post,path="/api/brains/{brain}/graph/rebuild",operation_id="rebuildGraph",params(("brain"=Uuid,Path)),request_body=GraphRebuild,responses((status=200,body=GraphGeneration)))]
pub async fn rebuild(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    ResponseJson(input): ResponseJson<GraphRebuild>,
) -> Result<ResponseJson<GraphGeneration>> {
    auth.require_browser()?;
    let request_key = commands::key(&headers)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    if let Some(saved) = commands::reserve(
        &mut tx,
        request_key.as_deref(),
        "graph.rebuild",
        json!({"brain":brain,"input":input}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(ResponseJson(saved));
    }
    let g = enqueue(&mut tx, brain, auth.user.id, &input).await?;
    commands::finish(&mut tx, request_key.as_deref(), brain, &g).await?;
    tx.commit().await?;
    Ok(ResponseJson(g))
}

pub async fn maintain_brain(state: &AppState, brain: Uuid, actor: Uuid) -> Result<usize> {
    let mut tx = db::actor_tx(&state.pool, actor).await?;
    db::require_writer(&mut tx, brain).await?;
    let first: i16 = sqlx::query_scalar("UPDATE brains SET graph_scanned_at=clock_timestamp(),graph_discovery_cursor=(graph_discovery_cursor+1)%3 WHERE id=$1 RETURNING (graph_discovery_cursor+2)::smallint%3::smallint")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("UPDATE jobs SET state='cancelled',error_code='graph_input_unavailable',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE brain_id=$1 AND kind='graph.project' AND state IN ('queued','running') AND target_id IN (SELECT g.id FROM graph_generations g JOIN repository_snapshots s ON s.brain_id=g.brain_id AND s.id=g.snapshot_id WHERE g.brain_id=$1 AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)<>'active')")
        .bind(brain).execute(&mut *tx).await?;
    sqlx::query("UPDATE graph_generations g SET state='removed',error_code='graph_input_unavailable' FROM repository_snapshots s WHERE g.brain_id=$1 AND s.brain_id=g.brain_id AND s.id=g.snapshot_id AND g.state<>'removed' AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)<>'active'")
        .bind(brain).execute(&mut *tx).await?;
    combined::remove_unavailable(&mut tx, brain).await?;
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM jobs WHERE brain_id=$1 AND state IN ('queued','running')",
    )
    .bind(brain)
    .fetch_one(&mut *tx)
    .await?;
    let room = (500 - pending).clamp(0, 10) as usize;
    let snapshots:Vec<Uuid>=sqlx::query_scalar("SELECT s.id FROM repository_snapshots s WHERE s.brain_id=$1
      AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active'
      AND EXISTS(SELECT 1 FROM repository_jobs r JOIN jobs j ON j.id=r.job_id WHERE r.brain_id=s.brain_id AND r.snapshot_id=s.id AND j.state='succeeded')
      AND NOT EXISTS(SELECT 1 FROM graph_generations g WHERE g.brain_id=s.brain_id AND g.snapshot_id=s.id)
      ORDER BY s.created_at,s.id LIMIT 7")
        .bind(brain).fetch_all(&mut *tx).await?;
    let current = epoch(&mut tx, brain).await?;
    let needed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM claims WHERE brain_id=$1) AND NOT EXISTS(SELECT 1 FROM graph_generations WHERE brain_id=$1 AND kind='knowledge' AND input_epoch=$2 AND state NOT IN ('superseded','removed') AND (error_code IS NULL OR error_code<>'graph_input_expired') AND (descriptor->>'expires_at' IS NULL OR (descriptor->>'expires_at')::timestamptz>clock_timestamp()))")
        .bind(brain).bind(current).fetch_one(&mut *tx).await?;
    let mut count = 0;
    for offset in 0..3 {
        match (usize::try_from(first).unwrap_or(0) + offset) % 3 {
            0 => {
                for snapshot in snapshots.iter().take((room - count).min(7)) {
                    enqueue(
                        &mut tx,
                        brain,
                        actor,
                        &GraphRebuild {
                            kind: "repository".into(),
                            snapshot_id: Some(*snapshot),
                            manifest_revision_id: None,
                        },
                    )
                    .await?;
                    count += 1;
                }
            }
            1 if needed && count < room => {
                enqueue(
                    &mut tx,
                    brain,
                    actor,
                    &GraphRebuild {
                        kind: "knowledge".into(),
                        snapshot_id: None,
                        manifest_revision_id: None,
                    },
                )
                .await?;
                count += 1;
            }
            2 => count += combined::discover(&mut tx, brain, actor, (room - count).min(2)).await?,
            _ => {}
        }
    }
    let cleanup:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM graph_generations WHERE brain_id=$1 AND state IN ('superseded','removed') AND cleaned_at IS NULL ORDER BY created_at,id LIMIT 2")
        .bind(brain).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    for id in cleanup {
        if !adapter::cleanup(state, brain, id).await? {
            continue;
        }
        let mut tx = db::actor_tx(&state.pool, actor).await?;
        db::require_writer(&mut tx, brain).await?;
        sqlx::query("UPDATE graph_generations SET cleaned_at=clock_timestamp() WHERE brain_id=$1 AND id=$2 AND state IN ('superseded','removed')")
            .bind(brain).bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    Ok(count)
}
pub async fn run_once(state: &AppState) -> Result<usize> {
    let brains: Vec<(Uuid, Uuid)> = sqlx::query_as("SELECT * FROM recollect_graph_brains()")
        .fetch_all(&state.pool)
        .await?;
    let mut count = 0;
    for (brain, actor) in brains {
        match maintain_brain(state, brain, actor).await {
            Ok(n) => count += n,
            Err(_) => tracing::warn!(brain_id=%brain,"Graph maintenance remains pending"),
        }
    }
    Ok(count)
}
