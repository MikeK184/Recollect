use crate::{
    AppState, artifacts,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    jobs, worker, workspace,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction, types::Json as SqlJson};
use std::collections::BTreeSet;
use uuid::Uuid;
type Tx<'a> = Transaction<'a, Postgres>;
fn conflict(message: &'static str) -> Error {
    Error(StatusCode::CONFLICT, "publication_conflict", message)
}
fn storage() -> Error {
    Error(
        StatusCode::SERVICE_UNAVAILABLE,
        "artifact_unavailable",
        "Repository artifacts are unavailable. Restore storage and retry.",
    )
}
fn capacity() -> Error {
    Error(
        StatusCode::TOO_MANY_REQUESTS,
        "publication_capacity",
        "Repository publication capacity reached.",
    )
}
#[derive(Default, Deserialize)]
pub struct Page {
    pub offset: Option<i64>,
}
pub(crate) fn offset(page: &Page) -> Result<i64> {
    let n = page.offset.unwrap_or(0);
    if !(0..=1_000_000).contains(&n) {
        Err(Error::invalid("Invalid page offset."))
    } else {
        Ok(n)
    }
}
/// Literal list filtering; never interpreted as SQL wildcard/search syntax.
pub(crate) fn list_query(query: Option<&str>) -> Result<String> {
    let value = query.unwrap_or_default().trim();
    if value.len() > 200 || value.chars().any(char::is_control) {
        return Err(Error::invalid(
            "Use a list search within 200 UTF-8 bytes without control characters.",
        ));
    }
    Ok(value.to_owned())
}
async fn write_tx<'a>(state: &'a AppState, auth: &Auth, brain: Uuid) -> Result<Tx<'a>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    Ok(tx)
}
pub(crate) async fn operation(
    tx: &mut Tx<'_>,
    brain: Uuid,
    id: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
    selection: &ScopeSelection,
    require_open: bool,
) -> Result<OperationBinding> {
    let op = workspace::bound_operation(tx, brain, id).await?;
    if op.actor_id != actor
        || op.device_id != device
        || !op.scope_valid
        || !matches!(op.kind.as_str(), "capture" | "write")
        || !op.scope.selection.repository_ids.is_empty()
            && !selection
                .repository_ids
                .iter()
                .all(|r| op.scope.selection.repository_ids.contains(r))
        || op.scope.selection.environment_id.is_some()
            && op.scope.selection.environment_id != selection.environment_id
    {
        return Err(Error::forbidden());
    }
    if require_open {
        let closed: bool = sqlx::query_scalar("SELECT closed FROM workspace_tasks WHERE id=$1")
            .bind(op.task_id)
            .fetch_one(&mut **tx)
            .await?;
        if closed {
            return Err(conflict("This task is closed. Start a new capture task."));
        }
    }
    Ok(op)
}
pub(crate) fn configured_secrets(state: &AppState) -> Vec<String> {
    let mut values = vec![
        state.config.owner_password.clone(),
        state.config.neo4j_password.clone(),
    ];
    if let Ok(url) = reqwest::Url::parse(&state.config.database_url)
        && let Some(p) = url.password()
    {
        values.push(p.into());
    }
    if let Some(oidc) = &state.config.oidc {
        values.push(oidc.client_secret.clone());
    }
    if let Some(key) = &state.config.models.key {
        values.push(key.clone());
    }
    values.extend(std::env::vars().filter_map(|(key, value)| {
        let key = key.to_ascii_uppercase();
        (value.len() >= 4
            && ["KEY", "TOKEN", "PASSWORD", "SECRET"]
                .iter()
                .any(|p| key.contains(p)))
        .then_some(value)
    }));
    values
}
pub(crate) fn safe_payload(state: &AppState, value: &Value) -> Result<()> {
    fn inspect(value: &Value, secrets: &[String]) -> bool {
        match value {
            Value::String(s) => repository_secret(s, secrets),
            Value::Array(v) => v.iter().any(|v| inspect(v, secrets)),
            Value::Object(v) => v
                .iter()
                .any(|(k, v)| repository_secret(k, secrets) || inspect(v, secrets)),
            _ => false,
        }
    }
    if inspect(value, &configured_secrets(state)) {
        return Err(Error::invalid(
            "Remove credential values or private key material before publication.",
        ));
    }
    Ok(())
}
#[derive(sqlx::FromRow)]
struct SnapshotRow {
    id: Uuid,
    brain_id: Uuid,
    repository_id: Uuid,
    revision: String,
    adapter: String,
    adapter_build: String,
    extractor_version: String,
    settings: SqlJson<Value>,
    coverage: SqlJson<Value>,
    file_count: i64,
    fact_count: i64,
    retained_file_count: i64,
    created_at: DateTime<Utc>,
}
pub(crate) async fn snapshot(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<RepositorySnapshot> {
    let permitted:bool=sqlx::query_scalar("SELECT recollect_content_state(brain_id,'repository',privacy_state,created_at)='active' FROM repository_snapshots WHERE brain_id=$1 AND id=$2").bind(brain).bind(id).fetch_optional(&mut **tx).await?.ok_or_else(Error::missing)?;
    if !permitted {
        return Err(crate::retention::unavailable());
    }
    let r = sqlx::query_as::<_, SnapshotRow>(
        "SELECT * FROM repository_snapshots WHERE brain_id=$1 AND id=$2",
    )
    .bind(brain)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(Error::missing)?;
    let job = sqlx::query_as::<_,jobs::JobRow>("SELECT * FROM jobs WHERE brain_id=$1 AND target_id=$2 AND kind='repository.process' ORDER BY created_at DESC,id DESC LIMIT 1").bind(brain).bind(id).fetch_optional(&mut **tx).await?.map(Job::from);
    let processing = match job.as_ref().map(|j| j.state.as_str()) {
        Some("succeeded") => "ready",
        Some(s) => s,
        None => "missing",
    }
    .into();
    Ok(RepositorySnapshot {
        id: r.id,
        brain_id: r.brain_id,
        repository_id: r.repository_id,
        revision: r.revision,
        adapter: r.adapter,
        adapter_build: r.adapter_build,
        extractor_version: r.extractor_version,
        settings: r.settings.0,
        coverage: r.coverage.0,
        file_count: r.file_count,
        fact_count: r.fact_count,
        retained_file_count: r.retained_file_count,
        processing,
        job,
        created_at: r.created_at,
    })
}
#[derive(sqlx::FromRow)]
struct FileRow {
    id: Uuid,
    path: String,
    object_id: String,
    mode: String,
    size: Option<i64>,
    status: String,
    extraction: String,
    artifact_id: Option<Uuid>,
    byte_length: i32,
}
impl FileRow {
    async fn dto(&self, state: &AppState, brain: Uuid) -> RepositoryFile {
        let availability = if self.artifact_id.is_none() {
            "not_retained"
        } else {
            artifacts::availability(
                &state.config.artifact_dir,
                brain,
                self.artifact_id,
                self.byte_length,
            )
            .await
        };
        RepositoryFile {
            id: self.id,
            path: self.path.clone(),
            object_id: self.object_id.clone(),
            mode: self.mode.clone(),
            size: self.size,
            status: self.status.clone(),
            extraction: self.extraction.clone(),
            availability: availability.into(),
        }
    }
}
async fn raw_artifact(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    snapshot: Uuid,
    kind: &str,
) -> Result<Value> {
    let (id,length): (Uuid,i32) = sqlx::query_as("SELECT id,byte_length FROM repository_artifacts WHERE brain_id=$1 AND snapshot_id=$2 AND kind=$3").bind(brain).bind(snapshot).bind(kind).fetch_optional(&mut **tx).await?.ok_or_else(storage)?;
    let bytes = artifacts::read_bounded(
        &state.config.artifact_dir,
        brain,
        id,
        length,
        FACTS_MAX_BYTES,
    )
    .await
    .map_err(|_| storage())?;
    serde_json::from_str(&bytes).map_err(|_| storage())
}
fn coverage(input: &PublicationInput) -> Value {
    let ids: BTreeSet<_> = input
        .facts
        .iter()
        .filter_map(|f| f.get("id").and_then(Value::as_str))
        .collect();
    let mut unresolved = 0;
    for fact in &input.facts {
        if let Some(relations) = fact.get("relations").and_then(Value::as_array) {
            for rel in relations {
                if rel
                    .get("target_id")
                    .and_then(Value::as_str)
                    .is_none_or(|id| !ids.contains(id))
                {
                    unresolved += 1;
                }
            }
        }
    }
    let no_facts: BTreeSet<_> = input
        .files
        .iter()
        .filter(|f| f.status == "materialized" && f.extraction == "no_facts")
        .map(|f| {
            std::path::Path::new(&f.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("no extension")
        })
        .collect();
    json!({"materialized_files":input.files.iter().filter(|f| f.status=="materialized").count(), "files_with_facts":input.files.iter().filter(|f| f.extraction=="facts_emitted").count(), "excluded_files":input.files.iter().filter(|f| f.status!="materialized").count(), "extensions_without_facts":no_facts, "unresolved_relations":unresolved, "quality":input.receipt.get("quality"), "limits":["Static extraction does not prove deployed state.","Kubernetes YAML semantic extraction is unsupported; inventory and permitted retained text remain available."]})
}
async fn equivalent(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    id: Uuid,
    input: &PublicationInput,
) -> Result<bool> {
    for (kind, expected) in [
        ("facts", json!(input.facts)),
        ("insights", input.insights.clone()),
        ("receipt", input.receipt.clone()),
    ] {
        if raw_artifact(state, tx, brain, id, kind).await? != expected {
            return Ok(false);
        }
    }
    let files = sqlx::query_as::<_, FileRow>(
        "SELECT * FROM repository_files WHERE brain_id=$1 AND snapshot_id=$2 ORDER BY path",
    )
    .bind(brain)
    .bind(id)
    .fetch_all(&mut **tx)
    .await?;
    if files.len() != input.files.len() {
        return Ok(false);
    }
    for (saved, incoming) in files.iter().zip(&input.files) {
        let content = if let Some(artifact) = saved.artifact_id {
            Some(
                artifacts::read(
                    &state.config.artifact_dir,
                    brain,
                    artifact,
                    saved.byte_length,
                )
                .await
                .map_err(|_| storage())?,
            )
        } else {
            None
        };
        if saved.path != incoming.path
            || saved.object_id != incoming.object_id
            || saved.mode != incoming.mode
            || saved.size != incoming.size.map(|n| n as i64)
            || saved.status != incoming.status
            || saved.extraction != incoming.extraction
            || content != incoming.content
        {
            return Ok(false);
        }
    }
    Ok(true)
}
async fn enqueue(
    tx: &mut Tx<'_>,
    actor: Uuid,
    brain: Uuid,
    snapshot: Uuid,
    audit: Uuid,
    operation: Option<Uuid>,
    selection: &ScopeSelection,
) -> Result<Uuid> {
    let job = jobs::enqueue_work(
        tx,
        actor,
        brain,
        audit,
        snapshot,
        "repository.process",
        "heavy",
    )
    .await?;
    sqlx::query("INSERT INTO repository_jobs(job_id,brain_id,snapshot_id,operation_id,selection) VALUES($1,$2,$3,$4,$5)").bind(job).bind(brain).bind(snapshot).bind(operation).bind(SqlJson(selection)).execute(&mut **tx).await?;
    Ok(job)
}
#[utoipa::path(get,path="/api/brains/{brain}/repositories/policy",operation_id="repositoryPolicy",params(("brain"=Uuid,Path)),responses((status=200,body=RepositoryPolicy)))]
pub async fn policy(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<RepositoryPolicy>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let allowed = sqlx::query_scalar("SELECT allow_repository_content FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(RepositoryPolicy {
        allow_file_content: allowed,
    }))
}
#[utoipa::path(put,path="/api/brains/{brain}/repositories/policy",operation_id="setRepositoryPolicy",params(("brain"=Uuid,Path)),request_body=RepositoryPolicy,responses((status=200,body=RepositoryPolicy)))]
pub async fn set_policy(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<RepositoryPolicy>,
) -> Result<Json<RepositoryPolicy>> {
    let mut tx = write_tx(&state, &auth, brain).await?;
    db::require_role(&mut tx, brain, true).await?;
    sqlx::query("UPDATE brains SET allow_repository_content=$2 WHERE id=$1")
        .bind(brain)
        .bind(input.allow_file_content)
        .execute(&mut *tx)
        .await?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "repository.policy",
        brain,
        if input.allow_file_content {
            "content_allowed"
        } else {
            "content_disabled"
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(input))
}
#[utoipa::path(post,path="/api/brains/{brain}/repositories/{repository}/snapshots",operation_id="publishRepository",params(("brain"=Uuid,Path),("repository"=Uuid,Path)),request_body=PublicationInput,responses((status=200,body=PublicationResult)))]
pub async fn publish(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, repository)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(mut input): Json<PublicationInput>,
) -> Result<Json<PublicationResult>> {
    let device = auth.device_id.ok_or_else(Error::forbidden)?;
    input.files.sort_by(|a, b| a.path.cmp(&b.path));
    input.receipt = stable_receipt(&input.receipt);
    validate_publication(&input).map_err(Error::invalid)?;
    let body = serde_json::to_value(&input).map_err(|_| Error::invalid("Invalid publication."))?;
    safe_payload(&state, &body)?;
    if input.facts.iter().any(|f| {
        f.get("repo")
            .is_some_and(|r| r.as_str() != Some(repository.to_string().as_str()))
    }) {
        return Err(Error::invalid(
            "Extractor repository label does not match the publication repository.",
        ));
    }
    let key = input.publication_id.to_string();
    if commands::key(&headers)?.is_some_and(|k| k != key) {
        return Err(Error::invalid("Idempotency-Key must match publication_id."));
    }
    let mut tx = write_tx(&state, &auth, brain).await?;
    let origin: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM repository_origins WHERE brain_id=$1 AND repository_id=$2 AND origin=$3)").bind(brain).bind(repository).bind(&input.origin).fetch_one(&mut *tx).await?;
    if !origin {
        return Err(Error::missing());
    }
    let fenced:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM privacy_publication_fences WHERE brain_id=$1 AND repository_id=$2 AND revision=$3) OR EXISTS(SELECT 1 FROM repository_snapshots WHERE brain_id=$1 AND repository_id=$2 AND revision=$3 AND recollect_content_state(brain_id,'repository',privacy_state,created_at)<>'active')").bind(brain).bind(repository).bind(&input.revision).fetch_one(&mut *tx).await?;
    if fenced {
        return Err(crate::retention::unavailable());
    }
    let op = workspace::bound_operation(&mut tx, brain, input.operation_id).await?;
    let selection = ScopeSelection {
        repository_ids: vec![repository],
        area_ids: op.scope.selection.area_ids.clone(),
        environment_id: op.scope.selection.environment_id,
    };
    let op = operation(
        &mut tx,
        brain,
        input.operation_id,
        auth.user.id,
        Some(device),
        &selection,
        true,
    )
    .await?;
    if op.kind != "capture" {
        return Err(Error::forbidden());
    }
    let retained = input.files.iter().filter(|f| f.content.is_some()).count();
    let allowed: bool =
        sqlx::query_scalar("SELECT allow_repository_content FROM brains WHERE id=$1")
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
    if retained > 0 && !allowed {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "repository_content_disabled",
            "This Brain does not allow new repository file retention.",
        ));
    }
    if let Some(reply) = commands::reserve::<PublicationResult>(
        &mut tx,
        Some(&key),
        "repository.publish",
        json!({"brain":brain,"repository":repository,"input":body}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(reply));
    }
    let candidate: Option<Uuid> = sqlx::query_scalar("SELECT id FROM repository_snapshots WHERE brain_id=$1 AND repository_id=$2 AND revision=$3 AND adapter=$4 AND adapter_build=$5 AND extractor_version=$6 AND settings=$7").bind(brain).bind(repository).bind(&input.revision).bind(&input.adapter).bind(&input.adapter_build).bind(&input.extractor_version).bind(&input.settings).fetch_optional(&mut *tx).await?;
    let reused = candidate.is_some();
    if let Some(id) = candidate
        && !equivalent(&state, &mut tx, brain, id, &input).await?
    {
        return Err(conflict(
            "This revision and extraction setting already has different immutable artifacts. Publish a distinct setting or inspect the contributor history.",
        ));
    }
    let snapshot_id = candidate.unwrap_or_else(Uuid::new_v4);
    let contribution_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM repository_contributions WHERE snapshot_id=$1")
            .bind(snapshot_id)
            .fetch_one(&mut *tx)
            .await?;
    if contribution_count >= 1000 {
        return Err(capacity());
    }
    if !reused {
        let (brain_count,repo_count):(i64,i64) = sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE repository_id=$2) FROM repository_snapshots WHERE brain_id=$1").bind(brain).bind(repository).fetch_one(&mut *tx).await?;
        if brain_count >= 4000 || repo_count >= 200 {
            return Err(capacity());
        }
    }
    jobs::capacity(&mut tx, brain).await?;
    let mut written = Vec::new();
    let result: Result<PublicationResult> = async {
        if !reused {
            sqlx::query("INSERT INTO repository_snapshots(id,brain_id,repository_id,revision,adapter,adapter_build,extractor_version,settings,coverage,file_count,fact_count,retained_file_count,created_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
                .bind(snapshot_id).bind(brain).bind(repository).bind(&input.revision).bind(&input.adapter).bind(&input.adapter_build).bind(&input.extractor_version).bind(&input.settings).bind(coverage(&input)).bind(input.files.len() as i64).bind(input.facts.len() as i64).bind(retained as i64).bind(auth.user.id).execute(&mut *tx).await?;
            for (kind,value) in [("facts",json!(input.facts)),("insights",input.insights.clone()),("receipt",input.receipt.clone())] {
                let content = serde_json::to_string(&value).map_err(|_| storage())?;
                let artifact_id = Uuid::new_v4();
                artifacts::write(&state.config.artifact_dir,brain,artifact_id,&content).await.map_err(|_|storage())?;
                written.push(artifact_id);
                sqlx::query("INSERT INTO repository_artifacts(id,brain_id,snapshot_id,kind,byte_length) VALUES($1,$2,$3,$4,$5)").bind(artifact_id).bind(brain).bind(snapshot_id).bind(kind).bind(content.len() as i32).execute(&mut *tx).await?;
            }
            let mut rows = Vec::new();
            for file in &input.files {
                let artifact_id = if let Some(content) = &file.content {
                    let id = Uuid::new_v4(); artifacts::write(&state.config.artifact_dir,brain,id,content).await.map_err(|_|storage())?; written.push(id); Some(id)
                } else { None };
                rows.push((Uuid::new_v4(),file,artifact_id));
            }
            for chunk in rows.chunks(500) {
                let mut q = sqlx::QueryBuilder::<Postgres>::new("INSERT INTO repository_files(id,brain_id,snapshot_id,path,object_id,mode,size,status,extraction,artifact_id,byte_length) ");
                q.push_values(chunk,|mut row,(id,file,artifact_id)| { row.push_bind(*id).push_bind(brain).push_bind(snapshot_id).push_bind(&file.path).push_bind(&file.object_id).push_bind(&file.mode).push_bind(file.size.map(|n|n as i64)).push_bind(&file.status).push_bind(&file.extraction).push_bind(*artifact_id).push_bind(file.content.as_ref().map_or(0,|s|s.len() as i32)); });
                q.build().execute(&mut *tx).await?;
            }
        }
        let existing = sqlx::query_as::<_,ContributionRow>("SELECT * FROM repository_contributions WHERE snapshot_id=$1 AND operation_id=$2").bind(snapshot_id).bind(input.operation_id).fetch_optional(&mut *tx).await?;
        if existing.as_ref().is_some_and(|old| old.origin != input.origin || old.branch != input.branch || old.dirty != input.dirty || old.captured_at != input.captured_at) {
            return Err(conflict("This capture operation already has different contributor provenance. Start a new capture operation."));
        }
        let contribution_id = existing.as_ref().map(|old|old.id).unwrap_or_else(Uuid::new_v4);
        if existing.is_none() {
            sqlx::query("INSERT INTO repository_contributions(id,brain_id,snapshot_id,actor_id,device_id,operation_id,scope,origin,branch,dirty,captured_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)").bind(contribution_id).bind(brain).bind(snapshot_id).bind(auth.user.id).bind(device).bind(input.operation_id).bind(SqlJson(&op.scope)).bind(&input.origin).bind(&input.branch).bind(input.dirty).bind(input.captured_at).execute(&mut *tx).await?;
        }
        crate::memory_rules::advance_epoch(&mut tx,brain).await?;
        let audit = db::audit(&mut tx,auth.user.id,brain,"repository.publish",snapshot_id,if reused {"contribution_reused"} else {"snapshot_accepted"}).await?;
        if !reused { enqueue(&mut tx,auth.user.id,brain,snapshot_id,audit,Some(op.id),&selection).await?; }
        let reply = PublicationResult { snapshot:snapshot(&mut tx,brain,snapshot_id).await?, contribution_id, reused };
        commands::finish(&mut tx,Some(&key),brain,&reply).await?;
        Ok(reply)
    }.await;
    match result {
        Ok(reply) => {
            tx.commit().await?;
            Ok(Json(reply))
        }
        Err(error) => {
            let _ = tx.rollback().await;
            for id in written {
                let _ =
                    tokio::fs::remove_file(artifacts::path(&state.config.artifact_dir, brain, id))
                        .await;
            }
            Err(error)
        }
    }
}

#[utoipa::path(get,path="/api/brains/{brain}/repositories/{repository}/snapshots",operation_id="repositorySnapshots",params(("brain"=Uuid,Path),("repository"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=SnapshotPage)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, repository)): Path<(Uuid, Uuid)>,
    Query(page): Query<Page>,
) -> Result<Json<SnapshotPage>> {
    let offset = offset(&page)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM repositories WHERE id=$1 AND brain_id=$2)")
            .bind(repository)
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
    if !exists {
        return Err(Error::missing());
    }
    let total = sqlx::query_scalar(
        "SELECT count(*) FROM repository_snapshots WHERE brain_id=$1 AND repository_id=$2 AND recollect_content_state(brain_id,'repository',privacy_state,created_at)='active'",
    )
    .bind(brain)
    .bind(repository)
    .fetch_one(&mut *tx)
    .await?;
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM repository_snapshots WHERE brain_id=$1 AND repository_id=$2 AND recollect_content_state(brain_id,'repository',privacy_state,created_at)='active' ORDER BY created_at DESC,id LIMIT 20 OFFSET $3").bind(brain).bind(repository).bind(offset).fetch_all(&mut *tx).await?;
    let mut items = Vec::new();
    for id in ids {
        items.push(snapshot(&mut tx, brain, id).await?);
    }
    tx.commit().await?;
    Ok(Json(SnapshotPage {
        items,
        total,
        offset,
    }))
}
#[derive(sqlx::FromRow)]
struct ContributionRow {
    id: Uuid,
    #[sqlx(default)]
    actor_name: String,
    actor_id: Uuid,
    device_id: Uuid,
    operation_id: Uuid,
    scope: SqlJson<ScopeSnapshot>,
    origin: String,
    branch: Option<String>,
    dirty: bool,
    captured_at: DateTime<Utc>,
    accepted_at: DateTime<Utc>,
}
#[utoipa::path(get,path="/api/brains/{brain}/repository-snapshots/{snapshot}",operation_id="repositorySnapshot",params(("brain"=Uuid,Path),("snapshot"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=SnapshotDetail)))]
pub async fn detail(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(page): Query<Page>,
) -> Result<Json<SnapshotDetail>> {
    let offset = offset(&page)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    db::lock_brain(&mut tx, brain, false).await?;
    let snapshot = snapshot(&mut tx, brain, id).await?;
    let total = sqlx::query_scalar(
        "SELECT count(*) FROM repository_contributions WHERE snapshot_id=$1 AND brain_id=$2",
    )
    .bind(id)
    .bind(brain)
    .fetch_one(&mut *tx)
    .await?;
    let rows = sqlx::query_as::<_,ContributionRow>("SELECT c.*,a.username AS actor_name FROM repository_contributions c JOIN accounts a ON a.id=c.actor_id WHERE c.snapshot_id=$1 AND c.brain_id=$2 ORDER BY c.accepted_at DESC,c.id LIMIT 20 OFFSET $3").bind(id).bind(brain).bind(offset).fetch_all(&mut *tx).await?;
    let contributors = rows
        .into_iter()
        .map(|r| RepositoryContribution {
            id: r.id,
            actor_id: r.actor_id,
            actor_name: r.actor_name,
            device_id: r.device_id,
            operation_id: r.operation_id,
            scope: r.scope.0,
            origin: r.origin,
            branch: r.branch,
            dirty: r.dirty,
            captured_at: r.captured_at,
            accepted_at: r.accepted_at,
        })
        .collect();
    let observed_at: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let deadline: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT recollect_retention_deadline(brain_id,'repository',created_at) FROM repository_snapshots WHERE brain_id=$1 AND id=$2")
        .bind(brain).bind(id).fetch_one(&mut *tx).await?;
    if deadline.is_some_and(|at| at <= observed_at) {
        return Err(Error::missing());
    }
    let valid_until = deadline.map_or(observed_at + chrono::Duration::seconds(6), |at| {
        at.min(observed_at + chrono::Duration::seconds(6))
    });
    tx.commit().await?;
    Ok(Json(SnapshotDetail {
        observed_at,
        valid_until,
        snapshot,
        contributors,
        contributor_total: total,
        contributor_offset: offset,
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/repository-snapshots/{snapshot}/files",operation_id="repositoryFiles",params(("brain"=Uuid,Path),("snapshot"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=RepositoryFilePage)))]
pub async fn files(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(page): Query<Page>,
) -> Result<Json<RepositoryFilePage>> {
    let offset = offset(&page)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let snapshot = snapshot(&mut tx, brain, id).await?;
    let rows = sqlx::query_as::<_,FileRow>("SELECT * FROM repository_files WHERE brain_id=$1 AND snapshot_id=$2 ORDER BY path LIMIT 100 OFFSET $3").bind(brain).bind(id).bind(offset).fetch_all(&mut *tx).await?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.dto(&state, brain).await);
    }
    tx.commit().await?;
    Ok(Json(RepositoryFilePage {
        items,
        total: snapshot.file_count,
        offset,
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/repository-snapshots/{snapshot}/files/{file}",operation_id="repositoryFileContent",params(("brain"=Uuid,Path),("snapshot"=Uuid,Path),("file"=Uuid,Path)),responses((status=200,body=RepositoryFileContent)))]
pub async fn file(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id, file)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<RepositoryFileContent>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    db::lock_brain(&mut tx, brain, false).await?;
    snapshot(&mut tx, brain, id).await?;
    let row = sqlx::query_as::<_, FileRow>(
        "SELECT * FROM repository_files WHERE brain_id=$1 AND snapshot_id=$2 AND id=$3",
    )
    .bind(brain)
    .bind(id)
    .bind(file)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(Error::missing)?;
    let mut file = row.dto(&state, brain).await;
    let content = if let Some(artifact) = row.artifact_id {
        match artifacts::read(&state.config.artifact_dir, brain, artifact, row.byte_length).await {
            Ok(content) => Some(content),
            Err(reason) => {
                file.availability = match reason {
                    artifacts::ReadFailure::Missing => "missing",
                    artifacts::ReadFailure::Invalid => "unreadable",
                    _ => "unavailable",
                }
                .into();
                None
            }
        }
    } else {
        None
    };
    tx.commit().await?;
    Ok(Json(RepositoryFileContent { file, content }))
}
#[derive(sqlx::FromRow)]
struct FactRow {
    id: Uuid,
    ordinal: i32,
    record: SqlJson<Value>,
}
#[utoipa::path(get,path="/api/brains/{brain}/repository-snapshots/{snapshot}/facts",operation_id="repositoryFacts",params(("brain"=Uuid,Path),("snapshot"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=RepositoryFactPage)))]
pub async fn facts(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(page): Query<Page>,
) -> Result<Json<RepositoryFactPage>> {
    let offset = offset(&page)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let snapshot = snapshot(&mut tx, brain, id).await?;
    let items = sqlx::query_as::<_,FactRow>("SELECT id,ordinal,record FROM repository_facts WHERE brain_id=$1 AND snapshot_id=$2 ORDER BY ordinal LIMIT 100 OFFSET $3").bind(brain).bind(id).bind(offset).fetch_all(&mut *tx).await?.into_iter().map(|r| RepositoryFact {id:r.id,ordinal:r.ordinal,record:r.record.0}).collect();
    tx.commit().await?;
    Ok(Json(RepositoryFactPage {
        items,
        total: snapshot.fact_count,
        offset,
        processing: snapshot.processing,
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/repository-snapshots/{snapshot}/artifacts/{kind}",operation_id="repositoryArtifact",params(("brain"=Uuid,Path),("snapshot"=Uuid,Path),("kind"=String,Path)),responses((status=200,body=RepositoryArtifact)))]
pub async fn artifact(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id, kind)): Path<(Uuid, Uuid, String)>,
) -> Result<Json<RepositoryArtifact>> {
    if !matches!(kind.as_str(), "facts" | "insights" | "receipt") {
        return Err(Error::missing());
    }
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    snapshot(&mut tx, brain, id).await?;
    let row: Option<(Uuid,i32)> = sqlx::query_as("SELECT id,byte_length FROM repository_artifacts WHERE brain_id=$1 AND snapshot_id=$2 AND kind=$3").bind(brain).bind(id).bind(&kind).fetch_optional(&mut *tx).await?;
    let (availability, content) = if let Some((artifact, length)) = row {
        match artifacts::read_bounded(
            &state.config.artifact_dir,
            brain,
            artifact,
            length,
            FACTS_MAX_BYTES,
        )
        .await
        {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(v) => ("retained", Some(v)),
                Err(_) => ("unreadable", None),
            },
            Err(artifacts::ReadFailure::Missing) => ("missing", None),
            Err(artifacts::ReadFailure::Invalid) => ("unreadable", None),
            Err(_) => ("unavailable", None),
        }
    } else {
        ("missing", None)
    };
    tx.commit().await?;
    Ok(Json(RepositoryArtifact {
        kind,
        availability: availability.into(),
        content,
    }))
}
#[utoipa::path(post,path="/api/brains/{brain}/repository-snapshots/{snapshot}/process",operation_id="processRepositorySnapshot",params(("brain"=Uuid,Path),("snapshot"=Uuid,Path)),responses((status=200,body=RepositorySnapshot)))]
pub async fn process(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<RepositorySnapshot>> {
    let mut tx = write_tx(&state, &auth, brain).await?;
    let current = snapshot(&mut tx, brain, id).await?;
    let key = commands::key(&headers)?;
    if let Some(reply) = commands::reserve::<RepositorySnapshot>(
        &mut tx,
        key.as_deref(),
        "repository.process",
        json!({"brain":brain,"snapshot":id}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(reply));
    }
    if current
        .job
        .as_ref()
        .is_none_or(|j| !matches!(j.state.as_str(), "queued" | "running"))
    {
        let audit = db::audit(
            &mut tx,
            auth.user.id,
            brain,
            "repository.reprocess",
            id,
            "queued",
        )
        .await?;
        enqueue(
            &mut tx,
            auth.user.id,
            brain,
            id,
            audit,
            None,
            &ScopeSelection {
                repository_ids: vec![current.repository_id],
                ..Default::default()
            },
        )
        .await?;
    }
    let reply = snapshot(&mut tx, brain, id).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &reply).await?;
    tx.commit().await?;
    Ok(Json(reply))
}

pub async fn project(
    state: &AppState,
    tx: &mut Tx<'_>,
    job: &worker::ClaimedJob,
) -> std::result::Result<&'static str, worker::Failure> {
    let saved: Option<(Option<Uuid>,SqlJson<ScopeSelection>)> = sqlx::query_as("SELECT operation_id,selection FROM repository_jobs WHERE brain_id=$1 AND snapshot_id=$2 AND job_id=$3").bind(job.brain_id).bind(job.target_id).bind(job.id).fetch_optional(&mut **tx).await?;
    let Some((op, selection)) = saved else {
        return Err(worker::Failure::Missing);
    };
    if !workspace::selection_valid(tx, job.brain_id, &selection.0)
        .await
        .map_err(|_| worker::Failure::Revoked)?
    {
        return Err(worker::Failure::Revoked);
    }
    if let Some(op) = op {
        operation(
            tx,
            job.brain_id,
            op,
            job.actor_id,
            job.device_id,
            &selection.0,
            false,
        )
        .await
        .map_err(|_| worker::Failure::Revoked)?;
    }
    let row = snapshot(tx, job.brain_id, job.target_id)
        .await
        .map_err(|_| worker::Failure::Missing)?;
    if selection.0.repository_ids != vec![row.repository_id] {
        return Err(worker::Failure::Revoked);
    }
    let facts: Vec<Value> = serde_json::from_value(
        raw_artifact(state, tx, job.brain_id, job.target_id, "facts")
            .await
            .map_err(|_| worker::Failure::Storage)?,
    )
    .map_err(|_| worker::Failure::Storage)?;
    let files = sqlx::query_as::<_, FileRow>(
        "SELECT * FROM repository_files WHERE brain_id=$1 AND snapshot_id=$2 ORDER BY path",
    )
    .bind(job.brain_id)
    .bind(job.target_id)
    .fetch_all(&mut **tx)
    .await?;
    let files: Vec<_> = files
        .into_iter()
        .map(|f| ExtractedFile {
            path: f.path,
            object_id: f.object_id,
            mode: f.mode,
            size: f.size.map(|n| n as u64),
            status: f.status,
            extraction: f.extraction,
            content: None,
        })
        .collect();
    validate_repository_facts(&facts, &files).map_err(|_| worker::Failure::Storage)?;
    if facts.len() as i64 != row.fact_count {
        return Err(worker::Failure::Storage);
    }
    // One bounded bulk insert preserves original UUIDs on reprocessing and duplicate upstream IDs by ordinal.
    sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) SELECT gen_random_uuid(),$1,$2,(ordinality-1)::integer,value FROM jsonb_array_elements($3) WITH ORDINALITY ON CONFLICT(snapshot_id,ordinal) DO NOTHING").bind(job.brain_id).bind(job.target_id).bind(json!(facts)).execute(&mut **tx).await?;
    let mismatches:i64 = sqlx::query_scalar("SELECT count(*) FROM (SELECT ordinal,record FROM repository_facts WHERE brain_id=$1 AND snapshot_id=$2) f FULL JOIN jsonb_array_elements($3) WITH ORDINALITY r ON f.ordinal=r.ordinality-1 WHERE f.record IS DISTINCT FROM r.value").bind(job.brain_id).bind(job.target_id).bind(json!(facts)).fetch_one(&mut **tx).await?;
    if mismatches != 0 {
        return Err(worker::Failure::Storage);
    }
    Ok("facts_materialized")
}
