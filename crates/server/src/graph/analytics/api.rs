use super::*;
use crate::{commands, publication};
use axum::{
    Json as ResponseJson,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use std::time::Duration;
use tokio::sync::Semaphore;

static READS: Semaphore = Semaphore::const_new(2);

#[utoipa::path(post,path="/api/brains/{brain}/graph/analytics",operation_id="queueGraphAnalytics",params(("brain"=Uuid,Path)),request_body=AnalyticsRequest,responses((status=200,body=AnalyticsReport)))]
pub async fn queue(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    ResponseJson(input): ResponseJson<AnalyticsRequest>,
) -> Result<ResponseJson<AnalyticsReport>> {
    let _permit = READS.try_acquire().map_err(|_| {
        failure(
            "analytics_busy",
            "Analysis input reads are busy. Try again shortly.",
        )
    })?;
    tokio::time::timeout(Duration::from_secs(20),async {
        let config=validate(&input)?;
        let key=commands::key(&headers)?;
        let mut tx=auth.tx(&state.pool).await?;
        db::require_writer(&mut tx,brain).await?;
        crate::retrieval::graph::authorize(&mut tx,&auth,brain,&input.scope).await?;
        if let Some(saved)=commands::reserve(&mut tx,key.as_deref(),"graph.analyze",json!({"brain":brain,"input":input})).await? {
            tx.commit().await?;return Ok(ResponseJson(saved));
        }
        let (total,pending):(i64,i64)=sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE state IN ('queued','running')) FROM analytics_reports WHERE brain_id=$1")
          .bind(brain).fetch_one(&mut *tx).await?;
        if pending>=20 {return Err(Error(StatusCode::TOO_MANY_REQUESTS,"analytics_queue_full","This Brain already has 20 pending analyses. Wait for work to finish."));}
        if total>=100 {
            let removed=sqlx::query("DELETE FROM analytics_reports WHERE brain_id=$1 AND id IN (SELECT r.id FROM analytics_reports r WHERE r.brain_id=$1 AND r.state NOT IN ('queued','running') AND NOT EXISTS(SELECT 1 FROM analytics_attempts a WHERE a.report_id=r.id AND a.cleaned_at IS NULL) ORDER BY created_at,id LIMIT $2)")
              .bind(brain).bind(total-99).execute(&mut *tx).await?;
            if removed.rows_affected()<(total-99) as u64 {return Err(failure("analytics_cleanup_pending","Retained analysis cleanup must finish before another report can be queued."));}
        }
        let input_graph=selected(&state,&mut tx,&auth,brain,input.scope).await?;
        if input_graph.nodes.is_empty(){return Err(Error(StatusCode::UNPROCESSABLE_ENTITY,"analytics_empty","No eligible vertices exist in this selection. Choose a ready input with retained evidence."));}
        let id=Uuid::new_v4();
        let audit=db::audit(&mut tx,auth.user.id,brain,"graph.analyze",id,"queued").await?;
        let job=jobs::enqueue_work(&mut tx,auth.user.id,brain,audit,id,"graph.analyze","heavy").await?;
        let epoch=analytics_epoch(&mut tx,brain).await?;
        let sequence=privacy_sequence(&mut tx,brain).await?;
        let provenance=AnalyticsProvenance{generation:input_graph.view.generation.clone(),inputs:input_graph.view.inputs.clone(),memory_epoch:input_graph.view.memory_epoch,coverage:input_graph.view.coverage.clone()};
        sqlx::query("INSERT INTO analytics_reports(id,brain_id,actor_id,job_id,algorithm,direction,selection,provenance,descriptor,parameters,analytics_epoch,privacy_sequence,node_count,edge_count,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)")
          .bind(id).bind(brain).bind(auth.user.id).bind(job).bind(input.algorithm).bind(input.direction).bind(Json(&input_graph.view.scope)).bind(Json(provenance))
          .bind(Json(Frozen::from_selected(&input_graph))).bind(Json(config)).bind(epoch).bind(sequence).bind(input_graph.nodes.len() as i32).bind(input_graph.edges.len() as i32).bind(input_graph.deadline)
          .execute(&mut *tx).await?;
        let r=report(&mut tx,brain,id).await?;
        commands::finish(&mut tx,key.as_deref(),brain,&r).await?;
        deadline(&mut tx,input_graph.deadline).await?;
        tx.commit().await?;Ok(ResponseJson(r))
    }).await.map_err(|_|failure("analytics_timeout","This input read exceeded its time limit. Select a smaller scope."))?
}

#[utoipa::path(get,path="/api/brains/{brain}/graph/analytics",operation_id="listGraphAnalytics",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=AnalyticsStatus)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(page): Query<publication::Page>,
) -> Result<ResponseJson<AnalyticsStatus>> {
    auth.require_browser()?;
    let offset = publication::offset(&page)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let saved:Vec<Json<AnalyticsReport>>=sqlx::query_scalar("SELECT to_jsonb(r)-'descriptor'-'scores' FROM analytics_reports r WHERE brain_id=$1 ORDER BY created_at DESC,id DESC LIMIT 20 OFFSET $2")
      .bind(brain).bind(offset).fetch_all(&mut *tx).await?;
    let epoch = analytics_epoch(&mut tx, brain).await?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let reports = saved
        .into_iter()
        .map(|Json(mut r)| {
            if matches!(r.state.as_str(), "queued" | "running" | "ready")
                && (r.analytics_epoch != epoch || r.expires_at.is_some_and(|at| at <= now))
            {
                r.state = "stale".into();
                r.error_code = Some("analytics_inputs_changed".into());
            }
            r
        })
        .collect();
    let total = sqlx::query_scalar("SELECT count(*) FROM analytics_reports WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    let jobs:Vec<jobs::JobRow>=sqlx::query_as("SELECT * FROM jobs WHERE brain_id=$1 AND kind='graph.analyze' ORDER BY created_at DESC,id DESC LIMIT 20").bind(brain).fetch_all(&mut *tx).await?;
    let cleanup_pending=sqlx::query_scalar("SELECT DISTINCT report_id FROM analytics_attempts WHERE brain_id=$1 AND cleaned_at IS NULL").bind(brain).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(ResponseJson(AnalyticsStatus {
        reports,
        jobs: jobs.into_iter().map(Into::into).collect(),
        cleanup_pending,
        total,
        offset,
    }))
}

#[utoipa::path(post,path="/api/brains/{brain}/graph/analytics/{id}/view",operation_id="viewGraphAnalytics",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=AnalyticsViewRequest,responses((status=200,body=AnalyticsView)))]
pub async fn view(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    ResponseJson(input): ResponseJson<AnalyticsViewRequest>,
) -> Result<ResponseJson<AnalyticsView>> {
    let _permit = READS.try_acquire().map_err(|_| {
        failure(
            "analytics_busy",
            "Analysis reads are busy. Try again shortly.",
        )
    })?;
    tokio::time::timeout(Duration::from_secs(20),async {
        if input.offset>NODES {return Err(Error::invalid("The report offset exceeds its result limit."));}
        let mut tx=auth.tx(&state.pool).await?;
        db::require_role(&mut tx,brain,false).await?;
        let mut r=report(&mut tx,brain,id).await?;
        if let Some(mut scope)=r.selection.clone(){
            scope.operation_id=input.operation_id;
            crate::retrieval::graph::authorize(&mut tx,&auth,brain,&scope).await?;
        }else if auth.device_id.is_some(){return Err(Error::missing());}
        let mut rows=vec![];
        let mut final_deadline=None;
        let mut qualified_scores=false;
        if r.state=="ready" {
            match current(&state,&mut tx,&auth,&r,input.operation_id).await {
                Ok(graph)=>{
                    let scores:Option<Json<Vec<Score>>>=sqlx::query_scalar("SELECT scores FROM analytics_reports WHERE brain_id=$1 AND id=$2").bind(brain).bind(id).fetch_one(&mut *tx).await?;
                    if let Some(Json(scores))=scores {
                        if scores.len()!=graph.nodes.len(){return Err(failure("analytics_result_invalid","The saved result does not match its complete input."));}
                        for value in scores.into_iter().skip(input.offset).take(100){
                            let node=graph.nodes.get(&value.key).cloned().ok_or_else(changed)?;
                            rows.push(AnalyticsResult{node,score:value.score,group:value.group});
                        }
                        qualified_scores=true;
                    }
                    final_deadline=[graph.deadline,r.expires_at].into_iter().flatten().min();
                }
                Err(e) if e.1=="analytics_inputs_changed" || matches!(e.1,"graph_projection_missing"|"graph_input_unavailable"|"graph_retention_changed"|"analytics_scope_too_large")=>{
                    sqlx::query("SELECT recollect_analytics_observed_stale($1,$2)").bind(brain).bind(id).execute(&mut *tx).await?;
                    r.state="stale".into();r.error_code=Some("analytics_inputs_changed".into());
                }
                Err(e)=>return Err(e),
            }
        }
        let job:jobs::JobRow=sqlx::query_as("SELECT * FROM jobs WHERE brain_id=$1 AND id=$2").bind(brain).bind(r.job_id).fetch_one(&mut *tx).await?;
        let cleanup_pending=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM analytics_attempts WHERE brain_id=$1 AND report_id=$2 AND cleaned_at IS NULL)").bind(brain).bind(id).fetch_one(&mut *tx).await?;
        db::require_role(&mut tx,brain,false).await?;
        // Lock the report last: no job locks or stale-state updates may follow.
        // Another reader's observed invalidation does not advance the Brain epoch.
        let latest:Option<Json<AnalyticsReport>>=sqlx::query_scalar("SELECT recollect_analytics_read_final($1,$2)").bind(brain).bind(id).fetch_one(&mut *tx).await?;
        r=latest.ok_or_else(Error::missing)?.0;
        if r.state!="ready" {rows.clear();final_deadline=None;}
        else if !qualified_scores {return Err(changed());}
        let total=if r.state=="ready" {r.node_count}else{0};
        deadline(&mut tx,final_deadline).await?;
        tx.commit().await?;
        Ok(ResponseJson(AnalyticsView{report:r,job:job.into(),cleanup_pending,rows,total,offset:input.offset}))
    }).await.map_err(|_|failure("analytics_timeout","This report read exceeded its time limit. Try a narrower analysis."))?
}
