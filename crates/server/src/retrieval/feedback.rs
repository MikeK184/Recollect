//! Explicit utility with canonical delivery/attribution. Nothing in this module
//! changes production retrieval scores, truth, identity or topic membership.
use super::*;
use crate::{commands, db, graph as topology};
use axum::http::HeaderMap;
use sqlx::FromRow;

#[derive(FromRow)]
struct Stored {
    id: Uuid,
    task_id: Uuid,
    scope_id: Uuid,
    query_context_id: Uuid,
    scope: SqlJson<GraphSelection>,
    input_kind: String,
    revision_id: Uuid,
    change_id: Option<Uuid>,
    vote: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}
impl Stored {
    fn result(&self) -> Result<FeedbackResult> {
        Ok(FeedbackResult {
            context_id: self.id,
            task_id: self.task_id,
            query_context_id: self.query_context_id,
            change_id: self.change_id,
            vote: self
                .vote
                .as_ref()
                .map(|v| serde_json::from_value(json!(v)))
                .transpose()
                .map_err(|_| Error::invalid("The saved utility vote is invalid."))?,
            expires_at: self.expires_at,
        })
    }
    fn request(&self) -> FeedbackContextRequest {
        FeedbackContextRequest {
            scope: self.scope.0.clone(),
            query_context_id: self.query_context_id,
            input_kind: self.input_kind.clone(),
            revision_id: self.revision_id,
        }
    }
}
const STORED: &str = "SELECT c.*,v.vote FROM retrieval_feedback_contexts c LEFT JOIN retrieval_feedback_changes v ON v.id=c.change_id AND v.brain_id=c.brain_id WHERE c.brain_id=$1 AND c.id=$2 AND c.actor_id=recollect_actor() AND c.device_id IS NOT DISTINCT FROM recollect_device()";
fn changed() -> Error {
    Error(
        StatusCode::CONFLICT,
        "feedback_changed",
        "The delivery context or vote changed. Inspect the current input before recording feedback.",
    )
}
fn normalize(input: &mut FeedbackContextRequest) -> Result<()> {
    if input.scope.kind != "knowledge"
        || input.scope.snapshot_id.is_some()
        || input.scope.operation_id.is_none()
        || input.query_context_id.is_nil()
        || input.revision_id.is_nil()
        || !matches!(input.input_kind.as_str(), "claim" | "source_version")
    {
        return Err(Error::invalid(
            "Select an exact source version or claim under an owned retrieval operation and non-nil query context.",
        ));
    }
    let mut recall = RecallRequest {
        query: "Explicit utility delivery".into(),
        selection: input.scope.selection.clone(),
        operation_id: input.scope.operation_id,
        collection_id: input.scope.collection_id,
        manifest_revision_id: input.scope.manifest_revision_id,
        fact_at: input.scope.fact_at,
        mode: input.scope.mode.clone(),
        ..Default::default()
    };
    validate(&mut recall)?;
    input.scope.selection = recall.selection;
    input.scope.relations.clear();
    Ok(())
}

/// Also checked with a row lock during publication: a task scope change cannot
/// race a recorded delivery/vote. Devices cannot borrow another task's context.
async fn attribution(
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    scope: &GraphSelection,
    lock: bool,
) -> Result<(Uuid, Uuid)> {
    let operation =
        workspace::bound_operation(tx, brain, scope.operation_id.ok_or_else(Error::forbidden)?)
            .await?;
    if operation.actor_id != auth.user.id
        || operation.device_id != auth.device_id
        || !matches!(operation.kind.as_str(), "context" | "retrieval")
        || !operation.scope_valid
        || operation.scope.selection != scope.selection
    {
        return Err(Error::forbidden());
    }
    let query = if lock {
        "SELECT current_scope FROM workspace_tasks WHERE brain_id=$1 AND id=$2 AND account_id=$3 AND device_id IS NOT DISTINCT FROM $4 FOR SHARE"
    } else {
        "SELECT current_scope FROM workspace_tasks WHERE brain_id=$1 AND id=$2 AND account_id=$3 AND device_id IS NOT DISTINCT FROM $4"
    };
    let current: Option<Option<Uuid>> = sqlx::query_scalar(query)
        .bind(brain)
        .bind(operation.task_id)
        .bind(auth.user.id)
        .bind(auth.device_id)
        .fetch_optional(&mut **tx)
        .await?;
    if current.flatten() != Some(operation.scope.id) {
        return Err(changed());
    }
    Ok((operation.task_id, operation.scope.id))
}

struct Prepared {
    task: Uuid,
    scope: Uuid,
    epoch: i64,
    deadline: Option<DateTime<Utc>>,
    node: GraphNode,
}
async fn prepare(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    input: &FeedbackContextRequest,
) -> Result<Prepared> {
    let mut tx = auth.preparation_tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let epoch = topology::preparation_epoch(&mut tx, brain).await?;
    let (task, scope) = attribution(&mut tx, auth, brain, &input.scope, false).await?;
    let logical: Option<Uuid> = if input.input_kind == "source_version" {
        sqlx::query_scalar("SELECT v.source_id FROM source_versions v JOIN sources s ON s.brain_id=v.brain_id AND s.id=v.source_id AND s.current_version=v.id WHERE v.brain_id=$1 AND v.id=$2 AND NOT recollect_source_model_fenced(v.brain_id,v.id)")
            .bind(brain).bind(input.revision_id).fetch_optional(&mut *tx).await?
    } else {
        sqlx::query_scalar("WITH RECURSIVE ancestors(id) AS (
          SELECT id FROM claim_revisions WHERE brain_id=$1 AND id=$2
          UNION SELECT c.input_revision_id FROM claim_contributions c JOIN ancestors a ON a.id=c.revision_id WHERE c.brain_id=$1)
          SELECT claim_id FROM claim_revisions WHERE brain_id=$1 AND id=$2
          AND EXISTS(SELECT 1 FROM claims c WHERE c.brain_id=$1 AND c.id=claim_id AND c.current_revision=$2)
          AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=$1 AND f.revision_id IN(SELECT id FROM ancestors))
          AND NOT EXISTS(SELECT 1 FROM claim_supports s WHERE s.brain_id=$1 AND s.revision_id IN(SELECT id FROM ancestors) AND s.source_version_id IS NOT NULL AND recollect_source_model_fenced($1,s.source_version_id))")
            .bind(brain).bind(input.revision_id).fetch_optional(&mut *tx).await?
    };
    let entity = topology::Entity {
        kind: input.input_kind.clone(),
        id: logical.ok_or_else(Error::missing)?,
        revision_id: input.revision_id,
    };
    let mut selection = input.scope.clone();
    let mut qualified = graph::qualify(
        state,
        &mut tx,
        auth,
        brain,
        &mut selection,
        std::slice::from_ref(&entity),
        (1, None),
    )
    .await?;
    let node = qualified
        .nodes
        .remove(&entity.key())
        .ok_or_else(Error::missing)?;
    tx.commit().await?;
    Ok(Prepared {
        task,
        scope,
        epoch,
        deadline: qualified.deadline,
        node,
    })
}
async fn final_gate(
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    input: &FeedbackContextRequest,
    p: &Prepared,
    expires: Option<DateTime<Utc>>,
) -> Result<()> {
    if topology::preparation_epoch(tx, brain).await? != p.epoch
        || attribution(tx, auth, brain, &input.scope, true).await? != (p.task, p.scope)
    {
        return Err(changed());
    }
    let deadline = [p.deadline, expires].into_iter().flatten().min();
    let valid: bool = sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
        .bind(deadline)
        .fetch_one(&mut **tx)
        .await?;
    if !valid {
        return Err(changed());
    }
    Ok(())
}

#[utoipa::path(post,path="/api/brains/{brain}/retrieval/feedback/context",operation_id="feedbackContext",params(("brain"=Uuid,Path)),request_body=FeedbackContextRequest,responses((status=200,body=FeedbackContext)))]
pub async fn context(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(mut input): Json<FeedbackContextRequest>,
) -> Result<Json<FeedbackContext>> {
    let key = commands::key(&headers)?
        .ok_or_else(|| Error::invalid("Supply an Idempotency-Key for utility delivery."))?;
    normalize(&mut input)?;
    tokio::time::timeout(Duration::from_secs(15), async {
        let prepared = prepare(&state, &auth, brain, &input).await?;
        tokio::time::timeout(Duration::from_millis(1500), async {
            let mut tx = auth.publication_tx(&state.pool).await?;
            sqlx::query("SET LOCAL statement_timeout='750ms'").execute(&mut *tx).await?;
            db::require_writer(&mut tx, brain).await?;
            final_gate(&mut tx,&auth,brain,&input,&prepared,None).await?;
            let saved: Option<FeedbackResult> = commands::reserve(&mut tx,Some(&key),"retrieval.feedback.context",json!({"brain":brain,"input":input})).await?;
            let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM retrieval_feedback_contexts WHERE brain_id=$1 AND actor_id=$2 AND task_id=$3 AND query_context_id=$4 AND input_kind=$5 AND revision_id=$6")
                .bind(brain).bind(auth.user.id).bind(prepared.task).bind(input.query_context_id).bind(&input.input_kind).bind(input.revision_id).fetch_optional(&mut *tx).await?;
            let id = match id {
                Some(id) => id,
                None => {
                    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM retrieval_feedback_contexts WHERE brain_id=$1").bind(brain).fetch_one(&mut *tx).await?;
                    if count>=10_000 { return Err(Error(StatusCode::TOO_MANY_REQUESTS,"feedback_capacity","Your utility context storage for this Brain has reached its 10,000-context limit.")); }
                    let id=Uuid::new_v4();
                    sqlx::query("INSERT INTO retrieval_feedback_contexts(id,brain_id,actor_id,device_id,task_id,operation_id,scope_id,query_context_id,scope,input_kind,revision_id,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
                        .bind(id).bind(brain).bind(auth.user.id).bind(auth.device_id).bind(prepared.task).bind(input.scope.operation_id).bind(prepared.scope).bind(input.query_context_id).bind(SqlJson(&input.scope)).bind(&input.input_kind).bind(input.revision_id).bind(prepared.deadline).execute(&mut *tx).await?;
                    db::audit(&mut tx,auth.user.id,brain,"retrieval.feedback.context",id,"qualified").await?;
                    id
                }
            };
            let stored: Stored = sqlx::query_as(STORED).bind(brain).bind(id).fetch_optional(&mut *tx).await?.ok_or_else(Error::missing)?;
            if serde_json::to_value(&stored.scope.0).ok()!=serde_json::to_value(&input.scope).ok() || saved.as_ref().is_some_and(|s| s.context_id!=id) { return Err(changed()); }
            let result=stored.result()?;
            // Receipts contain attribution only, never the delivered node body.
            commands::finish(&mut tx,Some(&key),brain,&result).await?;
            final_gate(&mut tx,&auth,brain,&input,&prepared,stored.expires_at).await?;
            tx.commit().await?;
            Ok(Json(FeedbackContext { feedback:result, node:prepared.node }))
        }).await.map_err(|_| Error(StatusCode::SERVICE_UNAVAILABLE,"feedback_timeout","Utility delivery publication exceeded its time limit."))?
    }).await.map_err(|_| Error(StatusCode::SERVICE_UNAVAILABLE,"feedback_timeout","Utility delivery exceeded its time limit."))?
}

#[utoipa::path(post,path="/api/brains/{brain}/retrieval/feedback",operation_id="recordFeedback",params(("brain"=Uuid,Path)),request_body=FeedbackCommand,responses((status=200,body=FeedbackResult)))]
pub async fn record(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(command): Json<FeedbackCommand>,
) -> Result<Json<FeedbackResult>> {
    let key = commands::key(&headers)?
        .ok_or_else(|| Error::invalid("Supply an Idempotency-Key for a utility vote."))?;
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut read=auth.preparation_tx(&state.pool).await?;
        let stored:Stored=sqlx::query_as(STORED).bind(brain).bind(command.context_id).fetch_optional(&mut *read).await?.ok_or_else(Error::missing)?;
        read.commit().await?;
        let input=stored.request();
        let prepared=prepare(&state,&auth,brain,&input).await?;
        tokio::time::timeout(Duration::from_millis(1500), async {
            let mut tx=auth.publication_tx(&state.pool).await?;
            sqlx::query("SET LOCAL statement_timeout='750ms'").execute(&mut *tx).await?;
            db::require_writer(&mut tx,brain).await?;
            let current:Stored=sqlx::query_as(&format!("{STORED} FOR UPDATE OF c")).bind(brain).bind(command.context_id).fetch_optional(&mut *tx).await?.ok_or_else(Error::missing)?;
            if current.scope_id!=prepared.scope || current.task_id!=prepared.task { return Err(changed()); }
            final_gate(&mut tx,&auth,brain,&input,&prepared,current.expires_at).await?;
            let saved:Option<FeedbackResult>=commands::reserve(&mut tx,Some(&key),"retrieval.feedback.vote",json!({"brain":brain,"input":command})).await?;
            if let Some(saved)=saved {
                if saved.change_id!=current.change_id { return Err(changed()); }
                final_gate(&mut tx,&auth,brain,&input,&prepared,current.expires_at).await?;
                tx.commit().await?; return Ok(Json(saved));
            }
            if command.base_change_id!=current.change_id { return Err(changed()); }
            let mut result=current.result()?;
            if result.vote.as_ref()!=Some(&command.vote) {
                let change=Uuid::new_v4();
                sqlx::query("INSERT INTO retrieval_feedback_changes(id,brain_id,context_id,actor_id,device_id,vote,previous_change_id) VALUES($1,$2,$3,$4,$5,$6,$7)")
                    .bind(change).bind(brain).bind(current.id).bind(auth.user.id).bind(auth.device_id).bind(serde_json::to_value(&command.vote).unwrap().as_str()).bind(current.change_id).execute(&mut *tx).await?;
                sqlx::query("UPDATE retrieval_feedback_contexts SET change_id=$3 WHERE brain_id=$1 AND id=$2").bind(brain).bind(current.id).bind(change).execute(&mut *tx).await?;
                db::audit(&mut tx,auth.user.id,brain,"retrieval.feedback.vote",current.id,"recorded").await?;
                result.change_id=Some(change); result.vote=Some(command.vote.clone());
            }
            commands::finish(&mut tx,Some(&key),brain,&result).await?;
            final_gate(&mut tx,&auth,brain,&input,&prepared,current.expires_at).await?;
            tx.commit().await?; Ok(Json(result))
        }).await.map_err(|_| Error(StatusCode::SERVICE_UNAVAILABLE,"feedback_timeout","Utility vote publication exceeded its time limit."))?
    }).await.map_err(|_| Error(StatusCode::SERVICE_UNAVAILABLE,"feedback_timeout","Utility vote exceeded its time limit."))?
}
