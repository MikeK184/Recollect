use crate::{
    AppState,
    auth::Auth,
    db,
    error::{Error, Result},
    jobs,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde::Deserialize;
use sqlx::{Postgres, Transaction, types::Json as SqlJson};
use uuid::Uuid;

type Tx<'a> = Transaction<'a, Postgres>;
fn conflict(message: &'static str) -> Error {
    Error(StatusCode::CONFLICT, "workspace_conflict", message)
}
fn capacity() -> Error {
    Error(
        StatusCode::TOO_MANY_REQUESTS,
        "workspace_capacity",
        "Workspace capacity reached. Close finished tasks or use a smaller catalogue.",
    )
}
fn offset(value: Option<i64>) -> Result<i64> {
    let value = value.unwrap_or(0);
    if !(0..=1_000_000).contains(&value) {
        return Err(Error::invalid("Invalid page offset."));
    }
    Ok(value)
}
fn printable(value: &str, max: usize) -> bool {
    !value.chars().any(char::is_control) && value.chars().count() <= max
}
fn local_path(value: &str) -> Result<String> {
    let value = value.replace('\\', "/");
    let absolute = value.starts_with('/')
        || (value.len() >= 3
            && value.as_bytes()[0].is_ascii_alphabetic()
            && value.as_bytes()[1] == b':'
            && value.as_bytes()[2] == b'/');
    if !absolute || !printable(&value, 2000) || value.split('/').any(|p| p == "." || p == "..") {
        return Err(Error::invalid(
            "Use an absolute normalized workspace/checkout path.",
        ));
    }
    Ok(
        if value == "/" || value.len() == 3 && value.ends_with(":/") {
            value
        } else {
            value.trim_end_matches('/').into()
        },
    )
}
async fn working_tx<'a>(state: &'a AppState, auth: &Auth, brain: Uuid) -> Result<Tx<'a>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::lock_brain(&mut tx, brain, true).await?;
    let archived: bool = sqlx::query_scalar("SELECT archived FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    if archived {
        return Err(conflict(
            "Reopen this Brain before changing workspace context.",
        ));
    }
    Ok(tx)
}

#[derive(sqlx::FromRow)]
struct RepoRow {
    id: Uuid,
    brain_id: Uuid,
    canonical_origin: String,
    origins: Vec<String>,
    created_at: DateTime<Utc>,
}
impl From<RepoRow> for Repository {
    fn from(r: RepoRow) -> Self {
        Self {
            id: r.id,
            brain_id: r.brain_id,
            canonical_origin: r.canonical_origin,
            origins: r.origins,
            created_at: r.created_at,
        }
    }
}
const REPO_SELECT: &str = "SELECT r.*,ARRAY(SELECT o.origin FROM repository_origins o WHERE o.repository_id=r.id ORDER BY o.origin) AS origins FROM repositories r";
async fn repository(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<Repository> {
    Ok(
        sqlx::query_as::<_, RepoRow>(&format!("{REPO_SELECT} WHERE r.brain_id=$1 AND r.id=$2"))
            .bind(brain)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(Error::missing)?
            .into(),
    )
}

#[derive(Default, Deserialize)]
pub struct RepositoryQuery {
    q: Option<String>,
    offset: Option<i64>,
}
#[utoipa::path(get,path="/api/brains/{brain}/workspace/repositories",operation_id="repositoryCatalogue",params(("brain"=Uuid,Path),("q"=Option<String>,Query),("offset"=Option<i64>,Query)),responses((status=200,body=RepositoryPage)))]
pub async fn repositories(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<RepositoryQuery>,
) -> Result<Json<RepositoryPage>> {
    let page_offset = offset(query.offset)?;
    let search = crate::publication::list_query(query.q.as_deref())?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let condition = "r.brain_id=$1 AND ($2='' OR position(lower($2) in lower(r.canonical_origin))>0 OR EXISTS(SELECT 1 FROM repository_origins o WHERE o.repository_id=r.id AND position(lower($2) in lower(o.origin))>0))";
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM repositories r WHERE {condition}"
    ))
    .bind(brain)
    .bind(&search)
    .fetch_one(&mut *tx)
    .await?;
    let rows = sqlx::query_as::<_, RepoRow>(&format!(
        "{REPO_SELECT} WHERE {condition} ORDER BY r.canonical_origin,r.id LIMIT 50 OFFSET $3"
    ))
    .bind(brain)
    .bind(&search)
    .bind(page_offset)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(RepositoryPage {
        items: rows.into_iter().map(Into::into).collect(),
        total,
        offset: page_offset,
        next_offset: (page_offset + 50 < total).then_some(page_offset + 50),
    }))
}

#[utoipa::path(get,path="/api/brains/{brain}/workspace/repositories/{repository}",operation_id="repositoryIdentity",params(("brain"=Uuid,Path),("repository"=Uuid,Path)),responses((status=200,body=Repository)))]
pub async fn repository_identity(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Repository>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let value = repository(&mut tx, brain, id).await?;
    tx.commit().await?;
    Ok(Json(value))
}
#[derive(sqlx::FromRow)]
struct WorkspaceRow {
    id: Uuid,
    brain_id: Uuid,
    device_id: Uuid,
    root: String,
    complete: bool,
    notes: SqlJson<Vec<String>>,
    refreshed_at: DateTime<Utc>,
}
impl From<WorkspaceRow> for WorkspaceRegistration {
    fn from(r: WorkspaceRow) -> Self {
        Self {
            id: r.id,
            brain_id: r.brain_id,
            device_id: r.device_id,
            root: r.root,
            complete: r.complete,
            notes: r.notes.0,
            refreshed_at: r.refreshed_at,
        }
    }
}
async fn registration(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<WorkspaceRegistration> {
    Ok(sqlx::query_as::<_, WorkspaceRow>(
        "SELECT * FROM workspace_registrations WHERE brain_id=$1 AND id=$2",
    )
    .bind(brain)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(Error::missing)?
    .into())
}
#[derive(sqlx::FromRow)]
struct CheckoutRow {
    id: Uuid,
    workspace_id: Uuid,
    repository_id: Option<Uuid>,
    observation: SqlJson<CheckoutObservation>,
    present: bool,
    observed_at: DateTime<Utc>,
}
impl From<CheckoutRow> for CheckoutRegistration {
    fn from(r: CheckoutRow) -> Self {
        Self {
            id: r.id,
            workspace_id: r.workspace_id,
            repository_id: r.repository_id,
            observation: r.observation.0,
            present: r.present,
            observed_at: r.observed_at,
        }
    }
}
#[derive(sqlx::FromRow)]
struct ResourceRow {
    id: Uuid,
    name: String,
}
impl From<ResourceRow> for ScopeResource {
    fn from(r: ResourceRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
        }
    }
}
async fn groups(tx: &mut Tx<'_>, brain: Uuid, kind: &str) -> Result<Vec<ScopeResource>> {
    Ok(sqlx::query_as::<_, ResourceRow>(
        "SELECT id,name FROM evidence_groups WHERE brain_id=$1 AND kind=$2 ORDER BY lower(name),id",
    )
    .bind(brain)
    .bind(kind)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(Into::into)
    .collect())
}
pub(crate) async fn selection_valid(
    tx: &mut Tx<'_>,
    brain: Uuid,
    selection: &ScopeSelection,
) -> Result<bool> {
    Ok(
        sqlx::query_scalar("SELECT recollect_selection_valid($1,$2)")
            .bind(brain)
            .bind(sqlx::types::Json(selection))
            .fetch_one(&mut **tx)
            .await?,
    )
}
async fn snapshot(
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    task: Uuid,
    mut selection: ScopeSelection,
) -> Result<ScopeSnapshot> {
    if selection.repository_ids.len() > 100 || selection.area_ids.len() > 100 {
        return Err(Error::invalid(
            "Select at most 100 repositories and 100 areas.",
        ));
    }
    selection.repository_ids.sort();
    selection.repository_ids.dedup();
    selection.area_ids.sort();
    selection.area_ids.dedup();
    if !selection_valid(tx, brain, &selection).await? {
        return Err(Error::missing());
    }
    let repositories = sqlx::query_as::<_,ResourceRow>("SELECT id,canonical_origin AS name FROM repositories WHERE brain_id=$1 AND id=ANY($2) ORDER BY id").bind(brain).bind(&selection.repository_ids).fetch_all(&mut **tx).await?.into_iter().map(Into::into).collect();
    let areas = sqlx::query_as::<_, ResourceRow>(
        "SELECT id,name FROM evidence_groups WHERE brain_id=$1 AND id=ANY($2) ORDER BY id",
    )
    .bind(brain)
    .bind(&selection.area_ids)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(Into::into)
    .collect();
    let environment = if let Some(id) = selection.environment_id {
        Some(
            sqlx::query_as::<_, ResourceRow>(
                "SELECT id,name FROM evidence_groups WHERE brain_id=$1 AND id=$2",
            )
            .bind(brain)
            .bind(id)
            .fetch_one(&mut **tx)
            .await?
            .into(),
        )
    } else {
        None
    };
    let value = ScopeSnapshot {
        id: Uuid::new_v4(),
        task_id: task,
        brain_id: brain,
        selection,
        repositories,
        areas,
        environment,
        created_at: Utc::now(),
    };
    sqlx::query("INSERT INTO scope_snapshots(id,brain_id,task_id,account_id,snapshot,created_at) VALUES($1,$2,$3,$4,$5,$6)").bind(value.id).bind(brain).bind(task).bind(auth.user.id).bind(SqlJson(&value)).bind(value.created_at).execute(&mut **tx).await?;
    sqlx::query(
        "UPDATE workspace_tasks SET current_scope=$3,updated_at=now() WHERE brain_id=$1 AND id=$2",
    )
    .bind(brain)
    .bind(task)
    .bind(value.id)
    .execute(&mut **tx)
    .await?;
    Ok(value)
}
#[derive(sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    brain_id: Uuid,
    parent_task_id: Option<Uuid>,
    continuation_of_task_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
    account_id: Uuid,
    device_id: Option<Uuid>,
    label: String,
    closed: bool,
    snapshot: SqlJson<ScopeSnapshot>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
const TASK_SELECT: &str =
    "SELECT t.*,s.snapshot FROM workspace_tasks t JOIN scope_snapshots s ON s.id=t.current_scope";
impl TaskRow {
    async fn dto(self, tx: &mut Tx<'_>) -> Result<WorkspaceTask> {
        let scope = self.snapshot.0;
        let scope_valid = selection_valid(tx, self.brain_id, &scope.selection).await?;
        Ok(WorkspaceTask {
            id: self.id,
            brain_id: self.brain_id,
            parent_task_id: self.parent_task_id,
            continuation_of_task_id: self.continuation_of_task_id,
            workspace_id: self.workspace_id,
            created_by: self.account_id,
            device_id: self.device_id,
            label: self.label,
            closed: self.closed,
            scope,
            scope_valid,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
async fn task(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<WorkspaceTask> {
    sqlx::query_as::<_, TaskRow>(&format!("{TASK_SELECT} WHERE t.brain_id=$1 AND t.id=$2"))
        .bind(brain)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(Error::missing)?
        .dto(tx)
        .await
}
#[derive(sqlx::FromRow)]
struct OperationRow {
    id: Uuid,
    brain_id: Uuid,
    task_id: Uuid,
    snapshot: SqlJson<ScopeSnapshot>,
    kind: String,
    account_id: Uuid,
    device_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}
const OP_SELECT: &str =
    "SELECT o.*,s.snapshot FROM operation_bindings o JOIN scope_snapshots s ON s.id=o.scope_id";
impl OperationRow {
    async fn dto(self, tx: &mut Tx<'_>) -> Result<OperationBinding> {
        let scope = self.snapshot.0;
        let scope_valid = selection_valid(tx, self.brain_id, &scope.selection).await?;
        Ok(OperationBinding {
            id: self.id,
            brain_id: self.brain_id,
            task_id: self.task_id,
            scope,
            scope_valid,
            kind: self.kind,
            actor_id: self.account_id,
            device_id: self.device_id,
            created_at: self.created_at,
        })
    }
}
pub async fn bound_operation(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<OperationBinding> {
    sqlx::query_as::<_, OperationRow>(&format!("{OP_SELECT} WHERE o.brain_id=$1 AND o.id=$2"))
        .bind(brain)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(Error::missing)?
        .dto(tx)
        .await
}
fn handoff(task: WorkspaceTask, previous: Option<Uuid>) -> TaskChange {
    TaskChange {
        handoff: ScopeHandoff {
            previous_scope_id: previous,
            current_scope_id: task.scope.id,
            fresh_context_required: true,
            retrieval_available: true,
        },
        task,
    }
}

#[derive(Default, Deserialize)]
pub struct CatalogueQuery {
    workspace_id: Option<Uuid>,
    checkout_offset: Option<i64>,
    task_offset: Option<i64>,
    include_repositories: Option<bool>,
}
#[utoipa::path(get,path="/api/brains/{brain}/workspace",operation_id="workspaceCatalogue",params(("brain"=Uuid,Path),("workspace_id"=Option<Uuid>,Query),("checkout_offset"=Option<i64>,Query),("task_offset"=Option<i64>,Query),("include_repositories"=Option<bool>,Query)),responses((status=200,body=WorkspaceCatalogue)))]
pub async fn catalogue(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<WorkspaceCatalogue>> {
    let checkout_offset = offset(query.checkout_offset)?;
    let task_offset = offset(query.task_offset)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let repositories = if query.include_repositories == Some(false) {
        vec![]
    } else {
        sqlx::query_as::<_, RepoRow>(&format!(
            "{REPO_SELECT} WHERE r.brain_id=$1 ORDER BY r.canonical_origin,r.id"
        ))
        .bind(brain)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(Into::into)
        .collect()
    };
    let areas = groups(&mut tx, brain, "area").await?;
    let environments = groups(&mut tx, brain, "environment").await?;
    let workspaces: Vec<WorkspaceRegistration> = sqlx::query_as::<_, WorkspaceRow>(
        "SELECT * FROM workspace_registrations WHERE brain_id=$1 ORDER BY refreshed_at DESC,id",
    )
    .bind(brain)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(Into::into)
    .collect();
    let selected_workspace = query
        .workspace_id
        .or_else(|| workspaces.first().map(|w| w.id));
    let (checkouts, checkout_total) = if let Some(id) = selected_workspace {
        if !workspaces.iter().any(|w| w.id == id) {
            return Err(Error::missing());
        }
        let total =
            sqlx::query_scalar("SELECT count(*) FROM checkout_registrations WHERE workspace_id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        let rows = sqlx::query_as::<_,CheckoutRow>("SELECT * FROM checkout_registrations WHERE workspace_id=$1 ORDER BY present DESC,local_path,id LIMIT 100 OFFSET $2").bind(id).bind(checkout_offset).fetch_all(&mut *tx).await?;
        (rows.into_iter().map(Into::into).collect(), total)
    } else {
        (vec![], 0)
    };
    let task_total = sqlx::query_scalar("SELECT count(*) FROM workspace_tasks WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    let rows = sqlx::query_as::<_,TaskRow>(&format!("{TASK_SELECT} WHERE t.brain_id=$1 ORDER BY t.closed,t.created_at DESC,t.id DESC LIMIT 20 OFFSET $2")).bind(brain).bind(task_offset).fetch_all(&mut *tx).await?;
    let mut tasks = vec![];
    for row in rows {
        tasks.push(row.dto(&mut tx).await?);
    }
    tx.commit().await?;
    Ok(Json(WorkspaceCatalogue {
        repositories,
        areas,
        environments,
        workspaces,
        selected_workspace,
        checkouts,
        checkout_total,
        checkout_offset,
        tasks,
        task_total,
        task_offset,
    }))
}

#[utoipa::path(post,path="/api/brains/{brain}/workspace/checkouts",operation_id="refreshCheckouts",params(("brain"=Uuid,Path)),request_body=CheckoutRefresh,responses((status=200,body=CheckoutRefreshResult)))]
pub async fn refresh(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(mut input): Json<CheckoutRefresh>,
) -> Result<Json<CheckoutRefreshResult>> {
    let device = auth.device_id.ok_or_else(Error::forbidden)?;
    input.workspace_root = local_path(&input.workspace_root)?;
    if input.checkouts.len() > 200
        || input.notes.len() > 100
        || input.notes.iter().any(|n| !printable(n, 200))
    {
        return Err(Error::invalid("Checkout refresh exceeds its limits."));
    }
    let prefix = format!("{}/", input.workspace_root.trim_end_matches('/'));
    for c in &mut input.checkouts {
        c.local_path = local_path(&c.local_path)?;
        if c.local_path != input.workspace_root && !c.local_path.starts_with(&prefix) {
            return Err(Error::invalid(
                "Checkout path is outside the selected workspace.",
            ));
        }
        c.origin = c
            .origin
            .as_deref()
            .map(canonical_origin)
            .transpose()
            .map_err(Error::invalid)?;
        if c.branch.as_ref().is_some_and(|b| !printable(b, 256))
            || c.head.as_ref().is_some_and(|h| {
                !matches!(h.len(), 40 | 64) || !h.chars().all(|c| c.is_ascii_hexdigit())
            })
            || !matches!(
                c.status.as_str(),
                "available" | "no_origin" | "unsupported_origin" | "git_unavailable"
            )
        {
            return Err(Error::invalid("Invalid checkout observation."));
        }
        if c.status == "available" && c.origin.is_none() {
            return Err(Error::invalid(
                "An available repository needs a canonical origin.",
            ));
        }
    }
    input
        .checkouts
        .sort_by(|a, b| a.local_path.cmp(&b.local_path));
    if input
        .checkouts
        .windows(2)
        .any(|w| w[0].local_path == w[1].local_path)
    {
        return Err(Error::invalid("Duplicate checkout path in refresh."));
    }
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    jobs::capacity(&mut tx, brain).await?;
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM workspace_registrations WHERE brain_id=$1 AND device_id=$2 AND root=$3",
    )
    .bind(brain)
    .bind(device)
    .bind(&input.workspace_root)
    .fetch_optional(&mut *tx)
    .await?;
    let workspace = if let Some(id) = existing {
        id
    } else {
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM workspace_registrations WHERE brain_id=$1")
                .bind(brain)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 50 {
            return Err(capacity());
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO workspace_registrations(id,brain_id,account_id,device_id,root,complete,notes) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(id).bind(brain).bind(auth.user.id).bind(device).bind(&input.workspace_root).bind(input.complete).bind(SqlJson(&input.notes)).execute(&mut *tx).await?;
        id
    };
    sqlx::query(
        "UPDATE workspace_registrations SET complete=$2,notes=$3,refreshed_at=now() WHERE id=$1",
    )
    .bind(workspace)
    .bind(input.complete)
    .bind(SqlJson(&input.notes))
    .execute(&mut *tx)
    .await?;
    if input.complete {
        sqlx::query("UPDATE checkout_registrations SET present=false WHERE workspace_id=$1")
            .bind(workspace)
            .execute(&mut *tx)
            .await?;
    }
    for c in &input.checkouts {
        let repo = if let Some(origin) = &c.origin {
            let current: Option<Uuid> = sqlx::query_scalar(
                "SELECT repository_id FROM repository_origins WHERE brain_id=$1 AND origin=$2",
            )
            .bind(brain)
            .bind(origin)
            .fetch_optional(&mut *tx)
            .await?;
            Some(if let Some(id) = current {
                id
            } else {
                let count: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM repositories WHERE brain_id=$1")
                        .bind(brain)
                        .fetch_one(&mut *tx)
                        .await?;
                if count >= 1000 {
                    return Err(capacity());
                }
                let id = Uuid::new_v4();
                sqlx::query("INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,$3,$4)").bind(id).bind(brain).bind(origin).bind(auth.user.id).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO repository_origins(brain_id,origin,repository_id,created_by) VALUES($1,$2,$3,$4)").bind(brain).bind(origin).bind(id).bind(auth.user.id).execute(&mut *tx).await?;
                id
            })
        } else {
            None
        };
        sqlx::query("INSERT INTO checkout_registrations(id,workspace_id,brain_id,account_id,device_id,repository_id,local_path,observation) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(workspace_id,local_path) DO UPDATE SET repository_id=excluded.repository_id,observation=excluded.observation,present=true,observed_at=now()")
            .bind(Uuid::new_v4()).bind(workspace).bind(brain).bind(auth.user.id).bind(device).bind(repo).bind(&c.local_path).bind(SqlJson(c)).execute(&mut *tx).await?;
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM checkout_registrations WHERE workspace_id=$1")
            .bind(workspace)
            .fetch_one(&mut *tx)
            .await?;
    if count > 1000 {
        return Err(capacity());
    }
    let audit = db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "workspace.refresh",
        workspace,
        if input.complete {
            "complete"
        } else {
            "partial"
        },
    )
    .await?;
    jobs::enqueue(&mut tx, auth.user.id, brain, audit).await?;
    let result = CheckoutRefreshResult {
        workspace: registration(&mut tx, brain, workspace).await?,
        observed_checkouts: input.checkouts.len() as i64,
    };
    tx.commit().await?;
    Ok(Json(result))
}

#[utoipa::path(post,path="/api/brains/{brain}/workspace/repositories/{repository}/origins",operation_id="addRepositoryOrigin",params(("brain"=Uuid,Path),("repository"=Uuid,Path)),request_body=RepositoryOrigin,responses((status=200,body=Repository)))]
pub async fn alias(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<RepositoryOrigin>,
) -> Result<Json<Repository>> {
    let origin = canonical_origin(&input.origin).map_err(Error::invalid)?;
    let mut tx = working_tx(&state, &auth, brain).await?;
    db::require_role(&mut tx, brain, true).await?;
    repository(&mut tx, brain, id).await?;
    let current: Option<Uuid> = sqlx::query_scalar(
        "SELECT repository_id FROM repository_origins WHERE brain_id=$1 AND origin=$2",
    )
    .bind(brain)
    .bind(&origin)
    .fetch_optional(&mut *tx)
    .await?;
    if current.is_some_and(|existing| existing != id) {
        return Err(conflict(
            "That origin already belongs to another repository. Explicit reconciliation is required; identities were not merged.",
        ));
    }
    if current.is_none() {
        jobs::capacity(&mut tx, brain).await?;
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM repository_origins WHERE repository_id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 100 {
            return Err(capacity());
        }
        sqlx::query("INSERT INTO repository_origins(brain_id,origin,repository_id,created_by) VALUES($1,$2,$3,$4)").bind(brain).bind(origin).bind(id).bind(auth.user.id).execute(&mut *tx).await?;
        let audit = db::audit(
            &mut tx,
            auth.user.id,
            brain,
            "repository.alias",
            id,
            "origin_attached",
        )
        .await?;
        jobs::enqueue(&mut tx, auth.user.id, brain, audit).await?;
    }
    let result = repository(&mut tx, brain, id).await?;
    tx.commit().await?;
    Ok(Json(result))
}

#[utoipa::path(post,path="/api/brains/{brain}/workspace/tasks",operation_id="createWorkspaceTask",params(("brain"=Uuid,Path)),request_body=CreateTask,responses((status=200,body=TaskChange)))]
pub async fn create_task(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(mut input): Json<CreateTask>,
) -> Result<Json<TaskChange>> {
    input.label = input.label.trim().into();
    if input.label.is_empty() || !printable(&input.label, 120) {
        return Err(Error::invalid(
            "Task label must contain 1 to 120 printable characters.",
        ));
    }
    let mut tx = working_tx(&state, &auth, brain).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM workspace_tasks WHERE brain_id=$1 AND NOT closed")
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
    if count >= 100 {
        return Err(capacity());
    }
    if let Some(previous) = input.continuation_of_task_id {
        let previous = task(&mut tx, brain, previous).await?;
        if !previous.closed
            || previous.created_by != auth.user.id
            || previous.device_id != auth.device_id
            || input.parent_task_id.is_some()
        {
            return Err(Error::forbidden());
        }
        if input
            .selection
            .as_ref()
            .is_some_and(|s| s != &previous.scope.selection)
        {
            return Err(Error::forbidden());
        }
        input.selection = Some(previous.scope.selection);
        if input.workspace_id.is_none() {
            input.workspace_id = previous.workspace_id;
        }
    }
    if let Some(parent) = input.parent_task_id {
        let parent = task(&mut tx, brain, parent).await?;
        if parent.closed {
            return Err(conflict("This parent task is closed."));
        }
        if input.selection.is_none() {
            input.selection = Some(parent.scope.selection);
        }
        if input.workspace_id.is_none() {
            input.workspace_id = parent.workspace_id;
        }
    }
    if let Some(id) = input.workspace_id {
        let workspace = registration(&mut tx, brain, id).await?;
        if auth.device_id.is_some_and(|d| d != workspace.device_id) {
            return Err(Error::missing());
        }
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO workspace_tasks(id,brain_id,account_id,device_id,parent_task_id,workspace_id,label,continuation_of_task_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(id).bind(brain).bind(auth.user.id).bind(auth.device_id).bind(input.parent_task_id).bind(input.workspace_id).bind(input.label).bind(input.continuation_of_task_id).execute(&mut *tx).await?;
    snapshot(
        &mut tx,
        &auth,
        brain,
        id,
        input.selection.unwrap_or_default(),
    )
    .await?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "task.create",
        id,
        "scope_bound",
    )
    .await?;
    let result = handoff(task(&mut tx, brain, id).await?, None);
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(put,path="/api/brains/{brain}/workspace/tasks/{task}/scope",operation_id="changeTaskScope",params(("brain"=Uuid,Path),("task"=Uuid,Path)),request_body=ChangeScope,responses((status=200,body=TaskChange)))]
pub async fn change_scope(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ChangeScope>,
) -> Result<Json<TaskChange>> {
    let mut tx = working_tx(&state, &auth, brain).await?;
    let current = task(&mut tx, brain, id).await?;
    if current.closed {
        return Err(conflict("This task is closed."));
    }
    if input.base_scope != current.scope.id {
        return Err(conflict(
            "The task scope changed. Reload its current scope before changing it again.",
        ));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM scope_snapshots WHERE task_id=$1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if count >= 1000 {
        return Err(capacity());
    }
    let next = snapshot(&mut tx, &auth, brain, id, input.selection).await?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "task.scope",
        next.id,
        "fresh_context_required",
    )
    .await?;
    let result = handoff(task(&mut tx, brain, id).await?, Some(current.scope.id));
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/brains/{brain}/workspace/tasks/{task}/operations",operation_id="startScopedOperation",params(("brain"=Uuid,Path),("task"=Uuid,Path)),request_body=StartOperation,responses((status=200,body=OperationBinding)))]
pub async fn begin(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<StartOperation>,
) -> Result<Json<OperationBinding>> {
    if !matches!(
        input.kind.as_str(),
        "context" | "retrieval" | "write" | "capture" | "tool"
    ) {
        return Err(Error::invalid("Unknown operation kind."));
    }
    let mut tx = working_tx(&state, &auth, brain).await?;
    if matches!(input.kind.as_str(), "write" | "capture") {
        db::require_writer(&mut tx, brain).await?;
    }
    let current = task(&mut tx, brain, id).await?;
    if current.closed {
        return Err(conflict("This task is closed."));
    }
    if !current.scope_valid {
        return Err(conflict(
            "A selected view is no longer available. Change the task scope explicitly before starting another operation.",
        ));
    }
    if input
        .expected_scope
        .is_some_and(|scope| scope != current.scope.id)
    {
        return Err(conflict(
            "The task scope changed before context refresh. Inspect the current task and refresh explicitly.",
        ));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM operation_bindings WHERE task_id=$1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if count >= 1000 {
        return Err(capacity());
    }
    let operation = Uuid::new_v4();
    sqlx::query("INSERT INTO operation_bindings(id,brain_id,task_id,scope_id,account_id,device_id,kind) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(operation).bind(brain).bind(id).bind(current.scope.id).bind(auth.user.id).bind(auth.device_id).bind(input.kind).execute(&mut *tx).await?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "operation.bind",
        operation,
        "binding_created",
    )
    .await?;
    let result = bound_operation(&mut tx, brain, operation).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/brains/{brain}/workspace/tasks/{task}/close",operation_id="closeWorkspaceTask",params(("brain"=Uuid,Path),("task"=Uuid,Path)),responses((status=200,body=WorkspaceTask)))]
pub async fn close(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<WorkspaceTask>> {
    let mut tx = working_tx(&state, &auth, brain).await?;
    if !task(&mut tx, brain, id).await?.closed {
        sqlx::query(
            "UPDATE workspace_tasks SET closed=true,updated_at=now() WHERE brain_id=$1 AND id=$2",
        )
        .bind(brain)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        db::audit(&mut tx, auth.user.id, brain, "task.close", id, "closed").await?;
    }
    let result = task(&mut tx, brain, id).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[derive(Default, Deserialize)]
pub struct HistoryQuery {
    scope_offset: Option<i64>,
    operation_offset: Option<i64>,
}
#[utoipa::path(get,path="/api/brains/{brain}/workspace/tasks/{task}",operation_id="workspaceTask",params(("brain"=Uuid,Path),("task"=Uuid,Path),("scope_offset"=Option<i64>,Query),("operation_offset"=Option<i64>,Query)),responses((status=200,body=TaskDetail)))]
pub async fn detail(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<TaskDetail>> {
    let scope_offset = offset(query.scope_offset)?;
    let operation_offset = offset(query.operation_offset)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let task = task(&mut tx, brain, id).await?;
    let scope_total = sqlx::query_scalar("SELECT count(*) FROM scope_snapshots WHERE task_id=$1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let scopes=sqlx::query_scalar::<_,SqlJson<ScopeSnapshot>>("SELECT snapshot FROM scope_snapshots WHERE task_id=$1 ORDER BY created_at DESC,id DESC LIMIT 20 OFFSET $2").bind(id).bind(scope_offset).fetch_all(&mut *tx).await?.into_iter().map(|v|v.0).collect();
    let operation_total =
        sqlx::query_scalar("SELECT count(*) FROM operation_bindings WHERE task_id=$1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    let rows = sqlx::query_as::<_, OperationRow>(&format!(
        "{OP_SELECT} WHERE o.task_id=$1 ORDER BY o.created_at DESC,o.id DESC LIMIT 20 OFFSET $2"
    ))
    .bind(id)
    .bind(operation_offset)
    .fetch_all(&mut *tx)
    .await?;
    let mut operations = vec![];
    for row in rows {
        operations.push(row.dto(&mut tx).await?);
    }
    tx.commit().await?;
    Ok(Json(TaskDetail {
        task,
        scopes,
        scope_total,
        scope_offset,
        operations,
        operation_total,
        operation_offset,
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/workspace/operations/{operation}",operation_id="scopedOperation",params(("brain"=Uuid,Path),("operation"=Uuid,Path)),responses((status=200,body=OperationBinding)))]
pub async fn operation(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<OperationBinding>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let result = bound_operation(&mut tx, brain, id).await?;
    tx.commit().await?;
    Ok(Json(result))
}
