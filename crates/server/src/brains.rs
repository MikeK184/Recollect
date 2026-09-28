use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    jobs,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chrono::{DateTime, Utc};
use recollect_protocol::{AuditEvent, Brain, CreateBrain, UpdateBrain};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct BrainRow {
    id: Uuid,
    owner_id: Uuid,
    name: String,
    description: String,
    archived: bool,
    role: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
impl From<BrainRow> for Brain {
    fn from(r: BrainRow) -> Self {
        Self {
            id: r.id,
            owner_id: r.owner_id,
            name: r.name,
            description: r.description,
            archived: r.archived,
            role: r.role,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
fn name(value: String) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 {
        return Err(Error::invalid(
            "Use a Brain name between 1 and 120 characters.",
        ));
    }
    Ok(value.into())
}
fn description(value: String) -> Result<String> {
    if value.chars().count() > 2000 {
        return Err(Error::invalid(
            "Keep the description within 2,000 characters.",
        ));
    }
    Ok(value)
}

#[utoipa::path(get,path="/api/brains",responses((status=200,body=Vec<Brain>)))]
pub async fn list(State(state): State<AppState>, auth: Auth) -> Result<Json<Vec<Brain>>> {
    let mut tx = auth.tx(&state.pool).await?;
    let rows = sqlx::query_as::<_, BrainRow>(
        "SELECT *, recollect_role(id) AS role FROM brains ORDER BY archived, lower(name), id",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post,path="/api/brains",request_body=CreateBrain,params(("Idempotency-Key"=Option<String>,Header)),responses((status=200,body=Brain),(status=400,body=recollect_protocol::ApiError)))]
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    headers: HeaderMap,
    Json(input): Json<CreateBrain>,
) -> Result<Json<Brain>> {
    if input.managed_memory {
        auth.require_browser()?;
    }
    let name = name(input.name)?;
    let description = description(input.description)?;
    let key = commands::key(&headers)?;
    let id = Uuid::new_v4();
    let mut tx = auth.tx(&state.pool).await?;
    // Keep receipts from clients predating the optional preset replayable.
    let mut identity = serde_json::json!({"name":name,"description":description});
    if input.managed_memory {
        identity["managed_memory"] = true.into();
    }
    if let Some(response) =
        commands::reserve::<Brain>(&mut tx, key.as_deref(), "brain.create", identity).await?
    {
        tx.commit().await?;
        return Ok(Json(response));
    }
    sqlx::query("INSERT INTO brains (id,owner_id,name,description) VALUES ($1,$2,$3,$4)")
        .bind(id)
        .bind(auth.user.id)
        .bind(name)
        .bind(description)
        .execute(&mut *tx)
        .await?;
    let audit = db::audit(&mut tx, auth.user.id, id, "brain.create", id, "created").await?;
    jobs::enqueue(&mut tx, auth.user.id, id, audit).await?;
    if input.managed_memory {
        crate::automation::apply(&state, &mut tx, id, auth.user.id).await?;
    }
    let row = sqlx::query_as::<_, BrainRow>(
        "SELECT *,recollect_role(id) AS role FROM brains WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let response: Brain = row.into();
    commands::finish(&mut tx, key.as_deref(), id, &response).await?;
    tx.commit().await?;
    Ok(Json(response))
}

#[utoipa::path(get,path="/api/brains/{id}",params(("id"=Uuid,Path)),responses((status=200,body=Brain),(status=404,body=recollect_protocol::ApiError)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Brain>> {
    let mut tx = auth.tx(&state.pool).await?;
    let row = sqlx::query_as::<_, BrainRow>(
        "SELECT *,recollect_role(id) AS role FROM brains WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(Error::missing)?;
    tx.commit().await?;
    Ok(Json(row.into()))
}

#[utoipa::path(patch,path="/api/brains/{id}",params(("id"=Uuid,Path),("Idempotency-Key"=Option<String>,Header)),request_body=UpdateBrain,responses((status=200,body=Brain),(status=403,body=recollect_protocol::ApiError),(status=404,body=recollect_protocol::ApiError)))]
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<UpdateBrain>,
) -> Result<Json<Brain>> {
    let key = commands::key(&headers)?;
    let name = input.name.map(name).transpose()?;
    let description = input.description.map(description).transpose()?;
    if name.is_none() && description.is_none() && input.archived.is_none() {
        return Err(Error::invalid("Choose a field to update."));
    }
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    if let Some(response) = commands::reserve::<Brain>(
        &mut tx,
        key.as_deref(),
        &format!("brain.update:{id}"),
        serde_json::json!({"name":name,"description":description,"archived":input.archived}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(response));
    }
    let row = sqlx::query_as::<_,BrainRow>("UPDATE brains SET name=coalesce($2,name),description=coalesce($3,description),archived=coalesce($4,archived),updated_at=now(),change_id=gen_random_uuid() WHERE id=$1 RETURNING *,recollect_role(id) AS role")
        .bind(id).bind(name).bind(description).bind(input.archived).fetch_one(&mut *tx).await?;
    let audit = db::audit(&mut tx, auth.user.id, id, "brain.update", id, "updated").await?;
    jobs::enqueue(&mut tx, auth.user.id, id, audit).await?;
    let response: Brain = row.into();
    commands::finish(&mut tx, key.as_deref(), id, &response).await?;
    tx.commit().await?;
    Ok(Json(response))
}

#[utoipa::path(get,path="/api/brains/{id}/audit",params(("id"=Uuid,Path)),responses((status=200,body=Vec<AuditEvent>)))]
pub async fn audit(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<AuditEvent>>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    type Row = (Uuid, Uuid, String, Uuid, String, DateTime<Utc>);
    let rows: Vec<Row> = sqlx::query_as("SELECT id,actor_id,action,target_id,disposition,created_at FROM mutation_audit WHERE brain_id=$1 ORDER BY created_at DESC,id LIMIT 100")
        .bind(id).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(id, actor_id, action, target_id, disposition, created_at)| AuditEvent {
                    id,
                    actor_id,
                    action,
                    target_id,
                    disposition,
                    created_at,
                },
            )
            .collect(),
    ))
}
