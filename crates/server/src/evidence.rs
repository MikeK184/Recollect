use crate::{
    AppState, artifacts,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    jobs,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Deserialize, Default)]
pub struct CatalogueQuery {
    pub operation_id: Option<Uuid>,
    pub q: Option<String>,
    pub collection: Option<Uuid>,
    pub area: Option<Uuid>,
    pub environment: Option<Uuid>,
    pub offset: Option<i64>,
}
#[derive(Deserialize, Default)]
pub struct ContentQuery {
    pub operation_id: Option<Uuid>,
}
#[derive(Deserialize, Default)]
pub struct PageQuery {
    pub offset: Option<i64>,
}
#[derive(sqlx::FromRow)]
struct GroupRow {
    id: Uuid,
    brain_id: Uuid,
    kind: String,
    name: String,
    description: String,
    source_count: i64,
}
impl From<GroupRow> for EvidenceGroup {
    fn from(r: GroupRow) -> Self {
        Self {
            id: r.id,
            brain_id: r.brain_id,
            kind: r.kind,
            name: r.name,
            description: r.description,
            source_count: r.source_count,
        }
    }
}
#[derive(sqlx::FromRow)]
struct VersionRow {
    id: Uuid,
    source_id: Uuid,
    brain_id: Uuid,
    title: String,
    media_type: String,
    source_uri: Option<String>,
    observed_at: Option<DateTime<Utc>>,
    created_by: Uuid,
    contributor: String,
    device_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
    artifact_id: Option<Uuid>,
    byte_length: i32,
    processing: String,
    job_state: Option<String>,
    retention_class: String,
    expires_at: Option<DateTime<Utc>>,
    privacy_state: String,
    excerpt: Option<sqlx::types::Json<ExcerptProvenance>>,
}
impl VersionRow {
    async fn dto(&self, root: &str) -> SourceVersion {
        let privacy = crate::retention::effective_state(&self.privacy_state, self.expires_at);
        let removed = privacy != "active";
        let availability = if removed {
            privacy
        } else {
            artifacts::availability(root, self.brain_id, self.artifact_id, self.byte_length).await
        };
        let privacy = crate::retention::effective_state(&self.privacy_state, self.expires_at);
        let removed = privacy != "active";
        let availability = if removed { privacy } else { availability };
        let processing = match self.job_state.as_deref() {
            Some(s @ ("queued" | "running" | "failed" | "cancelled")) => s,
            _ => &self.processing,
        };
        SourceVersion {
            id: self.id,
            source_id: self.source_id,
            brain_id: self.brain_id,
            title: if removed {
                format!(
                    "{} source",
                    if privacy == "erased" {
                        "Erased"
                    } else {
                        "Expired"
                    }
                )
            } else {
                self.title.clone()
            },
            media_type: self.media_type.clone(),
            source_uri: if removed {
                None
            } else {
                self.source_uri.clone()
            },
            observed_at: if removed { None } else { self.observed_at },
            created_by: self.created_by,
            contributor: self.contributor.clone(),
            device_id: self.device_id,
            created_at: self.created_at,
            recorded_at: self.recorded_at,
            retained: !removed && self.artifact_id.is_some(),
            byte_length: if removed { 0 } else { self.byte_length },
            processing: if removed {
                privacy.into()
            } else {
                processing.into()
            },
            availability: availability.into(),
            retention_class: self.retention_class.clone(),
            expires_at: self.expires_at,
            privacy_state: privacy.into(),
            excerpt: self.excerpt.as_ref().map(|e| e.0.clone()),
        }
    }
}
#[derive(sqlx::FromRow)]
struct SourceRow {
    source_identity: Uuid,
    source_creator: Uuid,
    source_created: DateTime<Utc>,
    version_count: i64,
    group_ids: Vec<Uuid>,
    #[sqlx(flatten)]
    version: VersionRow,
}
impl SourceRow {
    async fn dto(self, root: &str) -> SourceSummary {
        let version = self.version.dto(root).await;
        SourceSummary {
            id: self.source_identity,
            brain_id: version.brain_id,
            created_by: self.source_creator,
            created_at: self.source_created,
            version,
            version_count: self.version_count,
            group_ids: self.group_ids,
        }
    }
}
const VERSION_FIELDS: &str = "v.*,a.username AS contributor,recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) AS expires_at,(SELECT jsonb_build_object('source_id',e.parent_source_id,'version_id',e.parent_version_id,'first_line',e.first_line,'last_line',e.last_line,'captured_at',e.captured_at) FROM source_excerpts e WHERE e.brain_id=v.brain_id AND e.version_id=v.id) AS excerpt,(SELECT j.state FROM jobs j WHERE j.brain_id=v.brain_id AND j.target_id=v.id AND j.kind='source.process' ORDER BY j.created_at DESC,j.id DESC LIMIT 1) AS job_state";
fn source_select() -> String {
    format!(
        "SELECT s.id AS source_identity,s.created_by AS source_creator,s.created_at AS source_created,(SELECT count(*) FROM source_versions vv WHERE vv.source_id=s.id) AS version_count,ARRAY(SELECT m.group_id FROM evidence_memberships m WHERE m.source_id=s.id ORDER BY m.group_id) AS group_ids,{VERSION_FIELDS} FROM sources s JOIN recollect_source_knowledge v ON v.id=s.current_version JOIN accounts a ON a.id=v.created_by"
    )
}
const GROUP_SELECT: &str = "SELECT g.*,(SELECT count(*) FROM evidence_memberships m WHERE m.group_id=g.id) AS source_count FROM evidence_groups g";
fn conflict(message: &'static str) -> Error {
    Error(StatusCode::CONFLICT, "evidence_conflict", message)
}
fn name(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 || value.chars().any(char::is_control) {
        return Err(Error::invalid(
            "Use a name or title between 1 and 120 printable characters.",
        ));
    }
    Ok(value.into())
}
fn description(value: &str) -> Result<()> {
    if value.chars().count() > 2000
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(Error::invalid(
            "Keep descriptions within 2,000 text characters.",
        ));
    }
    Ok(())
}
fn offset(value: Option<i64>) -> Result<i64> {
    let n = value.unwrap_or(0);
    if !(0..=1_000_000).contains(&n) {
        return Err(Error::invalid(
            "Use a nonnegative page offset within 1,000,000.",
        ));
    }
    Ok(n)
}
async fn write_tx<'a>(
    state: &'a AppState,
    auth: &Auth,
    brain: Uuid,
) -> Result<Transaction<'a, Postgres>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    Ok(tx)
}
async fn groups_valid(
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    groups: &mut Vec<Uuid>,
) -> Result<()> {
    if groups.len() > 100 {
        return Err(Error::invalid("Use at most 100 associations per source."));
    }
    groups.sort_unstable();
    groups.dedup();
    let found: i64 =
        sqlx::query_scalar("SELECT count(*) FROM evidence_groups WHERE brain_id=$1 AND id=ANY($2)")
            .bind(brain)
            .bind(&*groups)
            .fetch_one(&mut **tx)
            .await?;
    if found != groups.len() as i64 {
        return Err(Error::missing());
    }
    Ok(())
}
async fn source_exists(
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    source: Uuid,
) -> Result<Uuid> {
    sqlx::query_scalar("SELECT current_version FROM sources WHERE brain_id=$1 AND id=$2")
        .bind(brain)
        .bind(source)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(Error::missing)
}
async fn summary(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    source: Uuid,
) -> Result<SourceSummary> {
    let row: SourceRow = sqlx::query_as(&format!(
        "{} WHERE s.brain_id=$1 AND s.id=$2",
        source_select()
    ))
    .bind(brain)
    .bind(source)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(Error::missing)?;
    Ok(row.dto(&state.config.artifact_dir).await)
}

#[utoipa::path(get,path="/api/brains/{brain}/sources/{source}",operation_id="sourceIdentity",params(("brain"=Uuid,Path),("source"=Uuid,Path)),responses((status=200,body=SourceSummary)))]
pub async fn identity(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, source)): Path<(Uuid, Uuid)>,
) -> Result<Json<SourceSummary>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let value = summary(&state, &mut tx, brain, source).await?;
    tx.commit().await?;
    Ok(Json(value))
}
async fn version_row(
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    source: Uuid,
    version: Uuid,
) -> Result<VersionRow> {
    sqlx::query_as(&format!("SELECT {VERSION_FIELDS} FROM recollect_source_knowledge v JOIN accounts a ON a.id=v.created_by WHERE v.brain_id=$1 AND v.source_id=$2 AND v.id=$3"))
        .bind(brain).bind(source).bind(version).fetch_optional(&mut **tx).await?.ok_or_else(Error::missing)
}
async fn group_result(
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    group: Uuid,
) -> Result<EvidenceGroup> {
    let row: GroupRow = sqlx::query_as(&format!("{GROUP_SELECT} WHERE g.brain_id=$1 AND g.id=$2"))
        .bind(brain)
        .bind(group)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(Error::missing)?;
    Ok(row.into())
}
async fn changed(
    tx: &mut Transaction<'_, Postgres>,
    auth: &Auth,
    brain: Uuid,
    target: Uuid,
    action: &str,
    disposition: &str,
) -> Result<()> {
    crate::memory_rules::advance_epoch(tx, brain).await?;
    let audit = db::audit(tx, auth.user.id, brain, action, target, disposition).await?;
    jobs::enqueue(tx, auth.user.id, brain, audit).await?;
    Ok(())
}

#[utoipa::path(get,path="/api/brains/{brain}/evidence",operation_id="evidenceCatalogue",params(("brain"=Uuid,Path),("collection"=Option<Uuid>,Query),("area"=Option<Uuid>,Query),("environment"=Option<Uuid>,Query),("q"=Option<String>,Query),("offset"=Option<i64>,Query),("operation_id"=Option<Uuid>,Query)),responses((status=200,body=EvidenceCatalogue)))]
pub async fn catalogue(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<EvidenceCatalogue>> {
    let offset = offset(query.offset)?;
    let search = crate::publication::list_query(query.q.as_deref())?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let mut filters = vec![];
    for (id, kind) in [
        (query.collection, "collection"),
        (query.area, "area"),
        (query.environment, "environment"),
    ] {
        if let Some(id) = id {
            let found:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_groups WHERE brain_id=$1 AND id=$2 AND kind=$3)").bind(brain).bind(id).bind(kind).fetch_one(&mut *tx).await?;
            if !found {
                return Err(Error::missing());
            }
            filters.push(id);
        }
    }
    let selection = read_selection(&mut tx, &auth, brain, query.operation_id).await?;
    let condition = "($5::jsonb IS NULL OR recollect_recall_scope(coalesce((SELECT a.selection FROM automatic_support_excerpts a WHERE a.version_id=v.id AND a.brain_id=v.brain_id),(SELECT i.selection FROM source_import_scopes i WHERE i.version_id=v.id),(SELECT b.selection FROM capture_events e JOIN capture_bindings b ON b.id=e.binding_id WHERE e.source_id=s.id AND e.brain_id=s.brain_id LIMIT 1),'{}'::jsonb),$5)) AND s.brain_id=$1 AND NOT EXISTS(SELECT 1 FROM unnest($2::uuid[]) f WHERE NOT EXISTS(SELECT 1 FROM evidence_memberships m WHERE m.source_id=s.id AND m.group_id=f)) AND ($3='' OR (recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active' AND position(lower($3) in lower(v.title))>0))";
    let total: i64 =
        sqlx::query_scalar(&format!("SELECT count(*) FROM sources s JOIN source_versions v ON v.id=s.current_version AND v.brain_id=s.brain_id WHERE {condition} AND $4::bigint>=0"))
            .bind(brain)
            .bind(&filters)
            .bind(&search)
            .bind(offset)
            .bind(&selection)
            .fetch_one(&mut *tx)
            .await?;
    let rows: Vec<SourceRow> = sqlx::query_as(&format!(
        "{} WHERE {condition} ORDER BY s.updated_at DESC,s.id LIMIT 50 OFFSET $4",
        source_select()
    ))
    .bind(brain)
    .bind(&filters)
    .bind(&search)
    .bind(offset)
    .bind(&selection)
    .fetch_all(&mut *tx)
    .await?;
    let mut sources = Vec::with_capacity(rows.len());
    for row in rows {
        sources.push(row.dto(&state.config.artifact_dir).await);
    }
    let groups: Vec<GroupRow> = sqlx::query_as(&format!(
        "{GROUP_SELECT} WHERE g.brain_id=$1 ORDER BY g.kind,lower(g.name),g.id"
    ))
    .bind(brain)
    .fetch_all(&mut *tx)
    .await?;
    let allow_document_content =
        sqlx::query_scalar("SELECT allow_document_content FROM brains WHERE id=$1")
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(Json(EvidenceCatalogue {
        groups: groups.into_iter().map(Into::into).collect(),
        sources,
        policy: EvidencePolicy {
            allow_document_content,
        },
        total,
        offset,
    }))
}
#[utoipa::path(put,path="/api/brains/{brain}/evidence/policy",operation_id="evidencePolicy",params(("brain"=Uuid,Path)),request_body=EvidencePolicy,responses((status=200,body=EvidencePolicy)))]
pub async fn policy(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<EvidencePolicy>,
) -> Result<Json<EvidencePolicy>> {
    let mut tx = write_tx(&state, &auth, brain).await?;
    db::require_role(&mut tx, brain, true).await?;
    sqlx::query("UPDATE brains SET allow_document_content=$2 WHERE id=$1")
        .bind(brain)
        .bind(input.allow_document_content)
        .execute(&mut *tx)
        .await?;
    changed(
        &mut tx,
        &auth,
        brain,
        brain,
        "evidence.policy",
        if input.allow_document_content {
            "retained_import_allowed"
        } else {
            "reference_only_import"
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(input))
}
#[utoipa::path(post,path="/api/brains/{brain}/evidence/groups",operation_id="createEvidenceGroup",params(("brain"=Uuid,Path)),request_body=CreateEvidenceGroup,responses((status=200,body=EvidenceGroup)))]
pub async fn create_group(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(mut input): Json<CreateEvidenceGroup>,
) -> Result<Json<EvidenceGroup>> {
    input.name = name(&input.name)?;
    description(&input.description)?;
    if !matches!(input.kind.as_str(), "collection" | "area" | "environment") {
        return Err(Error::invalid("Choose collection, area or environment."));
    }
    let key = commands::key(&headers)?;
    let mut tx = write_tx(&state, &auth, brain).await?;
    if let Some(saved) = commands::reserve::<EvidenceGroup>(
        &mut tx,
        key.as_deref(),
        &format!("evidence.group.create:{brain}"),
        serde_json::to_value(&input).expect("group serialization"),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(saved));
    }
    let (count,duplicate):(i64,bool)=sqlx::query_as("SELECT count(*),coalesce(bool_or(lower(name)=lower($3)),false) FROM evidence_groups WHERE brain_id=$1 AND kind=$2").bind(brain).bind(&input.kind).bind(&input.name).fetch_one(&mut *tx).await?;
    if duplicate {
        return Err(conflict("That name already exists in this kind of view."));
    }
    if count >= 250 {
        return Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "group_capacity",
            "This Brain already has 250 views of this kind.",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO evidence_groups(id,brain_id,kind,name,description,created_by) VALUES($1,$2,$3,$4,$5,$6)").bind(id).bind(brain).bind(&input.kind).bind(&input.name).bind(&input.description).bind(auth.user.id).execute(&mut *tx).await?;
    changed(
        &mut tx,
        &auth,
        brain,
        id,
        "evidence.group.create",
        "created",
    )
    .await?;
    let result = group_result(&mut tx, brain, id).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(patch,path="/api/brains/{brain}/evidence/groups/{group}",operation_id="updateEvidenceGroup",params(("brain"=Uuid,Path),("group"=Uuid,Path)),request_body=UpdateEvidenceGroup,responses((status=200,body=EvidenceGroup)))]
pub async fn update_group(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, group)): Path<(Uuid, Uuid)>,
    Json(mut input): Json<UpdateEvidenceGroup>,
) -> Result<Json<EvidenceGroup>> {
    input.name = name(&input.name)?;
    description(&input.description)?;
    let mut tx = write_tx(&state, &auth, brain).await?;
    let old = group_result(&mut tx, brain, group).await?;
    let duplicate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_groups WHERE brain_id=$1 AND kind=$2 AND lower(name)=lower($3) AND id<>$4)").bind(brain).bind(&old.kind).bind(&input.name).bind(group).fetch_one(&mut *tx).await?;
    if duplicate {
        return Err(conflict("That name already exists in this kind of view."));
    }
    sqlx::query("UPDATE evidence_groups SET name=$3,description=$4 WHERE brain_id=$1 AND id=$2")
        .bind(brain)
        .bind(group)
        .bind(input.name)
        .bind(input.description)
        .execute(&mut *tx)
        .await?;
    changed(
        &mut tx,
        &auth,
        brain,
        group,
        "evidence.group.update",
        "updated",
    )
    .await?;
    let result = group_result(&mut tx, brain, group).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(delete,path="/api/brains/{brain}/evidence/groups/{group}",operation_id="removeEvidenceGroup",params(("brain"=Uuid,Path),("group"=Uuid,Path)),responses((status=204)))]
pub async fn remove_group(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, group)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    let mut tx = write_tx(&state, &auth, brain).await?;
    group_result(&mut tx, brain, group).await?;
    let has_manifests: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM revision_manifests WHERE brain_id=$1 AND environment_id=$2)",
    )
    .bind(brain)
    .bind(group)
    .fetch_one(&mut *tx)
    .await?;
    if has_manifests {
        return Err(conflict(
            "This environment has revision manifest history. Preserve it until the explicit erasure lifecycle is available.",
        ));
    }
    let has_mcp: bool = sqlx::query_scalar("SELECT recollect_mcp_environment_used($1,$2)")
        .bind(brain)
        .bind(group)
        .fetch_one(&mut *tx)
        .await?;
    if has_mcp {
        return Err(conflict(
            "This environment is referenced by MCP configuration. Reassign its connections and profiles before removing it.",
        ));
    }
    sqlx::query("DELETE FROM evidence_groups WHERE brain_id=$1 AND id=$2")
        .bind(brain)
        .bind(group)
        .execute(&mut *tx)
        .await?;
    changed(
        &mut tx,
        &auth,
        brain,
        group,
        "evidence.group.remove",
        "sources_preserved",
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
fn validate_input(input: &mut SourceInput) -> Result<()> {
    input.title = name(&input.title)?;
    if !matches!(
        input.media_type.as_str(),
        "text/plain" | "text/markdown" | "application/json" | "text/yaml" | "application/toml"
    ) {
        return Err(Error::invalid(
            "Import UTF-8 plain text, Markdown, JSON, YAML or TOML. Binary/PDF extraction is unavailable.",
        ));
    }
    input.source_uri = input
        .source_uri
        .take()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty());
    if let Some(uri) = &input.source_uri {
        if uri.chars().count() > 2000 || uri.chars().any(char::is_control) {
            return Err(Error::invalid(
                "Use a source reference within 2,000 printable characters.",
            ));
        }
        if let Ok(url) = reqwest::Url::parse(uri)
            && (!url.username().is_empty() || url.password().is_some())
        {
            return Err(Error::invalid(
                "Source references must not contain credentials.",
            ));
        }
    }
    if let Some(content) = &input.content {
        if !input.retain_content {
            return Err(Error::invalid(
                "Choose retained content explicitly before importing text.",
            ));
        }
        if content.len() > artifacts::MAX_TEXT {
            return Err(Error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "source_too_large",
                "Keep each text document within 1 MiB.",
            ));
        }
        if content.trim().is_empty() || content.contains('\0') {
            return Err(Error::invalid(
                "Import nonempty UTF-8 text without binary NUL bytes.",
            ));
        }
    } else if input.retain_content || input.source_uri.is_none() {
        return Err(Error::invalid(
            "A reference-only version needs a source reference and no retained text.",
        ));
    }
    Ok(())
}
#[utoipa::path(post,path="/api/brains/{brain}/sources",operation_id="importSource",params(("brain"=Uuid,Path),("Idempotency-Key"=Option<String>,Header)),request_body=SourceInput,responses((status=200,body=SourceSummary)))]
pub async fn import(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<SourceInput>,
) -> Result<Json<SourceSummary>> {
    save_version(&state, &auth, brain, None, &headers, input)
        .await
        .map(Json)
}
#[utoipa::path(post,path="/api/brains/{brain}/sources/{source}/versions",operation_id="appendSourceVersion",params(("brain"=Uuid,Path),("source"=Uuid,Path),("Idempotency-Key"=Option<String>,Header)),request_body=SourceInput,responses((status=200,body=SourceSummary)))]
pub async fn append(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, source)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<SourceInput>,
) -> Result<Json<SourceSummary>> {
    save_version(&state, &auth, brain, Some(source), &headers, input)
        .await
        .map(Json)
}
async fn save_version(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    existing: Option<Uuid>,
    headers: &HeaderMap,
    mut input: SourceInput,
) -> Result<SourceSummary> {
    validate_input(&mut input)?;
    if input
        .retention_class
        .as_deref()
        .is_some_and(|c| !matches!(c, "document" | "raw_session" | "tool_output"))
    {
        return Err(Error::invalid(
            "Choose document, raw_session or tool_output. Supporting excerpts use the explicit excerpt operation.",
        ));
    }
    let key = commands::key(headers)?;
    let mut tx = write_tx(state, auth, brain).await?;
    crate::publication::safe_payload(
        state,
        &serde_json::to_value(&input).expect("source serialization"),
    )?;
    let import_scope = if let Some(operation) = input.operation_id {
        let bound = crate::workspace::bound_operation(&mut tx, brain, operation).await?;
        if bound.actor_id != auth.user.id
            || bound.device_id != auth.device_id
            || !bound.scope_valid
            || bound.kind != "write"
        {
            return Err(Error::forbidden());
        }
        Some((
            Some(operation),
            serde_json::to_value(bound.scope.selection).expect("selection"),
        ))
    } else if let Some(id) = existing {
        sqlx::query_as::<_,(Option<Uuid>,serde_json::Value)>("SELECT i.operation_id,i.selection FROM source_import_scopes i JOIN sources s ON s.current_version=i.version_id WHERE s.brain_id=$1 AND s.id=$2").bind(brain).bind(id).fetch_optional(&mut *tx).await?
    } else {
        None
    };
    let retention_class = if let Some(id) = existing {
        let (class,current_state):(String,String)=sqlx::query_as("SELECT s.retention_class,v.privacy_state FROM sources s JOIN source_versions v ON v.id=s.current_version WHERE s.brain_id=$1 AND s.id=$2").bind(brain).bind(id).fetch_optional(&mut *tx).await?.ok_or_else(Error::missing)?;
        if current_state == "erased" {
            return Err(crate::retention::unavailable());
        }
        if class == "support_excerpt" || input.retention_class.as_ref().is_some_and(|c| *c != class)
        {
            return Err(Error::invalid(
                "A source's retention class is immutable; excerpts require a new explicit selection.",
            ));
        }
        class
    } else {
        input
            .retention_class
            .clone()
            .unwrap_or_else(|| "document".into())
    };
    let input_text = [
        input.title.as_str(),
        input.source_uri.as_deref().unwrap_or(""),
        input.content.as_deref().unwrap_or(""),
    ]
    .join("\n");
    let database = reqwest::Url::parse(&state.config.database_url).ok();
    let secrets = [
        Some(state.config.owner_password.as_str()),
        Some(state.config.neo4j_password.as_str()),
        database.as_ref().and_then(|url| url.password()),
        state.config.oidc.as_ref().map(|o| o.client_secret.as_str()),
    ];
    if secrets
        .into_iter()
        .flatten()
        .any(|secret| !secret.is_empty() && input_text.contains(secret))
        || input_text
            .lines()
            .any(|line| line.contains("-----BEGIN ") && line.contains("PRIVATE KEY-----"))
    {
        return Err(Error::invalid(
            "Remove credential values or private key material before importing this source.",
        ));
    }
    if existing.is_some() && !input.group_ids.is_empty() {
        return Err(Error::invalid(
            "Change existing source associations through Organize.",
        ));
    }
    groups_valid(&mut tx, brain, &mut input.group_ids).await?;
    let permitted: bool =
        sqlx::query_scalar("SELECT allow_document_content FROM brains WHERE id=$1")
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
    if input.retain_content && !permitted {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "capture_denied",
            "This Brain permits reference-only imports. Ask a Brain admin to change its capture policy.",
        ));
    }
    let operation = existing
        .map(|id| format!("source.append:{brain}:{id}"))
        .unwrap_or_else(|| format!("source.import:{brain}"));
    if let Some(saved) = commands::reserve::<SourceSummary>(
        &mut tx,
        key.as_deref(),
        &operation,
        serde_json::to_value(&input).expect("source serialization"),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(saved);
    }
    jobs::capacity(&mut tx, brain).await?;
    let source = existing.unwrap_or_else(Uuid::new_v4);
    if let Some(source) = existing {
        let current = source_exists(&mut tx, brain, source).await?;
        if input.base_version.is_some_and(|base| base != current) {
            return Err(conflict(
                "This source changed while you were editing. Reload the latest version before saving.",
            ));
        }
    } else {
        if input.base_version.is_some() {
            return Err(Error::invalid("A new source has no previous version."));
        }
        sqlx::query(
            "INSERT INTO sources(id,brain_id,created_by,retention_class) VALUES($1,$2,$3,$4)",
        )
        .bind(source)
        .bind(brain)
        .bind(auth.user.id)
        .bind(&retention_class)
        .execute(&mut *tx)
        .await?;
    }
    let version = Uuid::new_v4();
    let artifact = input.content.as_ref().map(|_| Uuid::new_v4());
    let bytes = input.content.as_ref().map_or(0, |v| v.len()) as i32;
    sqlx::query("INSERT INTO source_versions(id,source_id,brain_id,title,media_type,source_uri,observed_at,artifact_id,byte_length,created_by,processing,retention_class,device_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
        .bind(version).bind(source).bind(brain).bind(&input.title).bind(&input.media_type).bind(&input.source_uri).bind(input.observed_at).bind(artifact).bind(bytes).bind(auth.user.id).bind(if artifact.is_some(){"queued"}else{"reference_only"}).bind(retention_class).bind(auth.device_id).execute(&mut *tx).await?;
    if let Some((operation, selection)) = import_scope {
        sqlx::query("INSERT INTO source_import_scopes(version_id,brain_id,operation_id,selection) VALUES($1,$2,$3,$4)").bind(version).bind(brain).bind(operation).bind(selection).execute(&mut *tx).await?;
    }
    sqlx::query(
        "UPDATE sources SET current_version=$3,updated_at=now() WHERE brain_id=$1 AND id=$2",
    )
    .bind(brain)
    .bind(source)
    .bind(version)
    .execute(&mut *tx)
    .await?;
    for group in &input.group_ids {
        sqlx::query(
            "INSERT INTO evidence_memberships(brain_id,source_id,group_id) VALUES($1,$2,$3)",
        )
        .bind(brain)
        .bind(source)
        .bind(group)
        .execute(&mut *tx)
        .await?;
    }
    crate::memory_rules::advance_epoch(&mut tx, brain).await?;
    let audit = db::audit(
        &mut tx,
        auth.user.id,
        brain,
        if existing.is_some() {
            "source.update"
        } else {
            "source.import"
        },
        version,
        if artifact.is_some() {
            "retained"
        } else {
            "reference_only"
        },
    )
    .await?;
    if artifact.is_some() {
        jobs::enqueue_work(
            &mut tx,
            auth.user.id,
            brain,
            audit,
            version,
            "source.process",
            "capture",
        )
        .await?;
    } else {
        jobs::enqueue(&mut tx, auth.user.id, brain, audit).await?;
    }
    if let (Some(artifact), Some(content)) = (artifact, &input.content) {
        artifacts::write(&state.config.artifact_dir, brain, artifact, content)
            .await
            .map_err(|_| {
                Error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "artifact_unavailable",
                    "Document storage is unavailable. Check the artifact volume and retry.",
                )
            })?;
    }
    let response = summary(state, &mut tx, brain, source).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &response).await?;
    tx.commit().await?;
    Ok(response)
}

/// Session capture uses the same artifact, chunks and durable processing lane as
/// document imports. Its independent policy has already authorized this content.
pub(crate) async fn capture_source(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    actor: Uuid,
    event_id: Uuid,
    host: &str,
    event: &CapturedHook,
) -> Result<(Uuid, Uuid)> {
    let content = event
        .content
        .as_deref()
        .ok_or_else(|| Error::invalid("Capture has no retained text."))?;
    jobs::capacity(tx, brain).await?;
    let source = Uuid::new_v4();
    let version = Uuid::new_v4();
    let artifact = Uuid::new_v4();
    let class = if event.kind == "tool_result" {
        "tool_output"
    } else {
        "raw_session"
    };
    sqlx::query("INSERT INTO sources(id,brain_id,created_by,retention_class,created_at) VALUES($1,$2,$3,$4,$5)")
        .bind(source).bind(brain).bind(actor).bind(class).bind(event.captured_at).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO source_versions(id,source_id,brain_id,title,media_type,source_uri,observed_at,artifact_id,byte_length,created_by,processing,retention_class,created_at) VALUES($1,$2,$3,$4,'text/plain',$5,$6,$7,$8,$9,'queued',$10,$6)")
        .bind(version).bind(source).bind(brain).bind(format!("{host} {}",event.kind))
        .bind(format!("recollect:capture:{event_id}")).bind(event.captured_at).bind(artifact)
        .bind(content.len() as i32).bind(actor).bind(class).execute(&mut **tx).await?;
    sqlx::query("UPDATE sources SET current_version=$2 WHERE id=$1")
        .bind(source)
        .bind(version)
        .execute(&mut **tx)
        .await?;
    crate::memory_rules::advance_epoch(tx, brain).await?;
    let audit = db::audit(tx, actor, brain, "source.capture", version, "retained").await?;
    jobs::enqueue_work(
        tx,
        actor,
        brain,
        audit,
        version,
        "source.process",
        "capture",
    )
    .await?;
    artifacts::write(&state.config.artifact_dir, brain, artifact, content)
        .await
        .map_err(|_| {
            Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "artifact_unavailable",
                "Captured source storage is unavailable. Retry shortly.",
            )
        })?;
    Ok((source, version))
}

#[utoipa::path(post,path="/api/brains/{brain}/excerpts",operation_id="retainExcerpt",params(("brain"=Uuid,Path)),request_body=ExcerptInput,responses((status=200,body=SourceSummary)))]
pub async fn excerpt(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(mut input): Json<ExcerptInput>,
) -> Result<Json<SourceSummary>> {
    input.title = input.title.trim().into();
    if input.title.is_empty()
        || input.title.chars().count() > 120
        || input.title.chars().any(char::is_control)
        || input.first_line < 1
        || input.last_line < input.first_line
    {
        return Err(Error::invalid(
            "Select exact source lines and a title of 1–120 printable characters.",
        ));
    }
    let key = commands::key(&headers)?;
    let mut tx = write_tx(&state, &auth, brain).await?;
    let policy = crate::retention::settings(&mut tx, brain).await?;
    let capture: bool = sqlx::query_scalar("SELECT allow_document_content FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    if !capture || !policy.policy.allow_support_excerpts {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "excerpt_denied",
            "This Brain does not permit new retained excerpts.",
        ));
    }
    if let Some(reply) = commands::reserve(
        &mut tx,
        key.as_deref(),
        "source.excerpt",
        serde_json::json!({"brain":brain,"input":input}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(reply));
    }
    let parent = version_row(&mut tx, brain, input.source_id, input.version_id).await?;
    if crate::retention::effective_state(&parent.privacy_state, parent.expires_at) != "active" {
        return Err(crate::retention::unavailable());
    }
    let artifact = parent
        .artifact_id
        .ok_or_else(|| Error::invalid("An excerpt requires retained source text."))?;
    let text = artifacts::read(
        &state.config.artifact_dir,
        brain,
        artifact,
        parent.byte_length,
    )
    .await
    .map_err(|_| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "artifact_unavailable",
            "The selected source text is unavailable.",
        )
    })?;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    if input.last_line as usize > lines.len() {
        return Err(Error::invalid(
            "The selected lines extend past the retained source.",
        ));
    }
    let content = lines[(input.first_line - 1) as usize..input.last_line as usize].concat();
    if content.len() > 8192 || content.trim().is_empty() {
        return Err(Error::invalid(
            "Select a nonempty excerpt of at most 8 KiB.",
        ));
    }
    crate::publication::safe_payload(
        &state,
        &serde_json::json!({"title":input.title,"text":content}),
    )?;
    if crate::retention::effective_state(&parent.privacy_state, parent.expires_at) != "active" {
        return Err(crate::retention::unavailable());
    }
    jobs::capacity(&mut tx, brain).await?;
    let source = Uuid::new_v4();
    let version = Uuid::new_v4();
    let artifact = Uuid::new_v4();
    sqlx::query("INSERT INTO sources(id,brain_id,created_by,retention_class) VALUES($1,$2,$3,'support_excerpt')").bind(source).bind(brain).bind(auth.user.id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO source_versions(id,source_id,brain_id,title,media_type,artifact_id,byte_length,created_by,processing,retention_class) VALUES($1,$2,$3,$4,'text/plain',$5,$6,$7,'queued','support_excerpt')")
        .bind(version).bind(source).bind(brain).bind(&input.title).bind(artifact).bind(content.len() as i32).bind(auth.user.id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO source_excerpts(brain_id,version_id,parent_source_id,parent_version_id,first_line,last_line) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(brain).bind(version).bind(input.source_id).bind(input.version_id).bind(input.first_line).bind(input.last_line).execute(&mut *tx).await?;
    sqlx::query("UPDATE sources SET current_version=$2 WHERE id=$1 AND brain_id=$3")
        .bind(source)
        .bind(version)
        .bind(brain)
        .execute(&mut *tx)
        .await?;
    crate::memory_rules::advance_epoch(&mut tx, brain).await?;
    let audit = db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "source.excerpt",
        version,
        "retained",
    )
    .await?;
    jobs::enqueue_work(
        &mut tx,
        auth.user.id,
        brain,
        audit,
        version,
        "source.process",
        "capture",
    )
    .await?;
    artifacts::write(&state.config.artifact_dir, brain, artifact, &content)
        .await
        .map_err(|_| {
            Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "artifact_unavailable",
                "Excerpt storage is unavailable. Retry shortly.",
            )
        })?;
    let result = summary(&state, &mut tx, brain, source).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(put,path="/api/brains/{brain}/sources/{source}/groups",operation_id="organizeSource",params(("brain"=Uuid,Path),("source"=Uuid,Path)),request_body=SourceGroups,responses((status=200,body=SourceSummary)))]
pub async fn organize(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, source)): Path<(Uuid, Uuid)>,
    Json(mut input): Json<SourceGroups>,
) -> Result<Json<SourceSummary>> {
    let mut tx = write_tx(&state, &auth, brain).await?;
    source_exists(&mut tx, brain, source).await?;
    groups_valid(&mut tx, brain, &mut input.group_ids).await?;
    sqlx::query("DELETE FROM evidence_memberships WHERE brain_id=$1 AND source_id=$2")
        .bind(brain)
        .bind(source)
        .execute(&mut *tx)
        .await?;
    for group in input.group_ids {
        sqlx::query(
            "INSERT INTO evidence_memberships(brain_id,source_id,group_id) VALUES($1,$2,$3)",
        )
        .bind(brain)
        .bind(source)
        .bind(group)
        .execute(&mut *tx)
        .await?;
    }
    changed(
        &mut tx,
        &auth,
        brain,
        source,
        "source.organize",
        "associations_updated",
    )
    .await?;
    let result = summary(&state, &mut tx, brain, source).await?;
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/brains/{brain}/sources/{source}/versions",operation_id="sourceHistory",params(("brain"=Uuid,Path),("source"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=SourceHistory)))]
pub async fn history(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, source)): Path<(Uuid, Uuid)>,
    Query(query): Query<PageQuery>,
) -> Result<Json<SourceHistory>> {
    let offset = offset(query.offset)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    source_exists(&mut tx, brain, source).await?;
    let total = sqlx::query_scalar(
        "SELECT count(*) FROM source_versions WHERE brain_id=$1 AND source_id=$2",
    )
    .bind(brain)
    .bind(source)
    .fetch_one(&mut *tx)
    .await?;
    let rows:Vec<VersionRow>=sqlx::query_as(&format!("SELECT {VERSION_FIELDS} FROM recollect_source_knowledge v JOIN accounts a ON a.id=v.created_by WHERE v.brain_id=$1 AND v.source_id=$2 ORDER BY v.recorded_at DESC,v.id DESC LIMIT 20 OFFSET $3")).bind(brain).bind(source).bind(offset).fetch_all(&mut *tx).await?;
    let mut versions = vec![];
    for row in rows {
        versions.push(row.dto(&state.config.artifact_dir).await);
    }
    tx.commit().await?;
    Ok(Json(SourceHistory {
        versions,
        total,
        offset,
    }))
}
#[derive(sqlx::FromRow)]
struct SpanRow {
    id: Uuid,
    ordinal: i32,
    byte_start: i32,
    byte_end: i32,
    line_start: i32,
    line_end: i32,
}
#[utoipa::path(get,path="/api/brains/{brain}/sources/{source}/versions/{version}",operation_id="sourceContent",params(("brain"=Uuid,Path),("source"=Uuid,Path),("version"=Uuid,Path),("operation_id"=Option<Uuid>,Query)),responses((status=200,body=SourceContent)))]
pub async fn content(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, source, version)): Path<(Uuid, Uuid, Uuid)>,
    Query(query): Query<ContentQuery>,
) -> Result<Json<SourceContent>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let row = version_row(&mut tx, brain, source, version).await?;
    if let Some(selection) = read_selection(&mut tx, &auth, brain, query.operation_id).await? {
        let scope = crate::capture::source_selection(&mut tx, brain, version).await?;
        let allowed: bool = sqlx::query_scalar("SELECT recollect_recall_scope($1,$2)")
            .bind(serde_json::to_value(scope).expect("scope"))
            .bind(selection)
            .fetch_one(&mut *tx)
            .await?;
        if !allowed {
            return Err(Error::missing());
        }
    }
    let mut info = row.dto(&state.config.artifact_dir).await;
    let mut content = if let Some(artifact) =
        row.artifact_id.filter(|_| info.privacy_state == "active")
    {
        match artifacts::read(&state.config.artifact_dir, brain, artifact, row.byte_length).await {
            Ok(text) => Some(text),
            Err(error) => {
                info.availability = match error {
                    artifacts::ReadFailure::Missing => "missing",
                    artifacts::ReadFailure::Unavailable => "unavailable",
                    artifacts::ReadFailure::Invalid => "unreadable",
                }
                .into();
                None
            }
        }
    } else {
        None
    };
    if crate::retention::effective_state(&row.privacy_state, row.expires_at) != "active" {
        info = row.dto(&state.config.artifact_dir).await;
        content = None;
    }
    let spans = if content.is_some() {
        sqlx::query_as::<_,SpanRow>("SELECT id,ordinal,byte_start,byte_end,line_start,line_end FROM source_chunks WHERE brain_id=$1 AND version_id=$2 ORDER BY ordinal").bind(brain).bind(version).fetch_all(&mut *tx).await?
            .into_iter().map(|r|SourceSpan { id:r.id,ordinal:r.ordinal,byte_start:r.byte_start,byte_end:r.byte_end,line_start:r.line_start,line_end:r.line_end }).collect()
    } else {
        vec![]
    };
    tx.commit().await?;
    Ok(Json(SourceContent {
        version: info,
        content,
        spans,
    }))
}
#[utoipa::path(post,path="/api/brains/{brain}/sources/{source}/process",operation_id="processSource",params(("brain"=Uuid,Path),("source"=Uuid,Path)),responses((status=200,body=SourceSummary)))]
pub async fn process(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, source)): Path<(Uuid, Uuid)>,
) -> Result<Json<SourceSummary>> {
    let mut tx = write_tx(&state, &auth, brain).await?;
    let version = source_exists(&mut tx, brain, source).await?;
    let row = version_row(&mut tx, brain, source, version).await?;
    if crate::retention::effective_state(&row.privacy_state, row.expires_at) != "active" {
        return Err(crate::retention::unavailable());
    }
    if row.artifact_id.is_none() {
        return Err(conflict(
            "This source has only a reference. Import a retained version before processing.",
        ));
    }
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE brain_id=$1 AND target_id=$2 AND kind='source.process' AND state IN ('queued','running'))").bind(brain).bind(version).fetch_one(&mut *tx).await?;
    if !pending {
        let audit = db::audit(
            &mut tx,
            auth.user.id,
            brain,
            "source.reprocess",
            version,
            "queued",
        )
        .await?;
        jobs::enqueue_work(
            &mut tx,
            auth.user.id,
            brain,
            audit,
            version,
            "source.process",
            "capture",
        )
        .await?;
    }
    let result = summary(&state, &mut tx, brain, source).await?;
    tx.commit().await?;
    Ok(Json(result))
}

pub async fn project(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    version: Uuid,
) -> std::result::Result<&'static str, crate::worker::Failure> {
    use crate::worker::Failure;
    let row: Option<(Option<Uuid>, i32)> = sqlx::query_as(
        "SELECT artifact_id,byte_length FROM source_versions WHERE brain_id=$1 AND id=$2 AND privacy_state='active' AND coalesce(recollect_retention_deadline(brain_id,retention_class,created_at)>clock_timestamp(),true)",
    )
    .bind(brain)
    .bind(version)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((artifact, length)) = row else {
        return Err(Failure::Missing);
    };
    let (status, text) = if let Some(artifact) = artifact {
        match artifacts::read(&state.config.artifact_dir, brain, artifact, length).await {
            Ok(text) => ("ready", Some(text)),
            Err(artifacts::ReadFailure::Missing | artifacts::ReadFailure::Invalid) => {
                ("missing", None)
            }
            Err(artifacts::ReadFailure::Unavailable) => return Err(Failure::Storage),
        }
    } else {
        ("reference_only", None)
    };
    let mut count = 0i32;
    if let Some(text) = text {
        let mut start = 0;
        let mut line = 1i32;
        while start < text.len() {
            let mut end = (start + 4096).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            if end < text.len()
                && let Some(relative) = text[start..end].rfind('\n')
                && relative >= 2048
            {
                end = start + relative + 1;
            }
            let part = &text[start..end];
            let newlines = part.bytes().filter(|c| *c == b'\n').count() as i32;
            let last = (line + newlines - i32::from(part.ends_with('\n'))).max(line);
            sqlx::query("INSERT INTO source_chunks(id,brain_id,version_id,ordinal,byte_start,byte_end,line_start,line_end,content) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(version_id,ordinal) DO UPDATE SET byte_start=excluded.byte_start,byte_end=excluded.byte_end,line_start=excluded.line_start,line_end=excluded.line_end,content=excluded.content")
                .bind(Uuid::new_v4()).bind(brain).bind(version).bind(count).bind(start as i32).bind(end as i32).bind(line).bind(last).bind(part).execute(&mut **tx).await?;
            line += newlines;
            start = end;
            count += 1;
        }
    }
    sqlx::query("DELETE FROM source_chunks WHERE brain_id=$1 AND version_id=$2 AND ordinal >= $3")
        .bind(brain)
        .bind(version)
        .bind(count)
        .execute(&mut **tx)
        .await?;
    sqlx::query("UPDATE source_versions SET processing=$3 WHERE brain_id=$1 AND id=$2")
        .bind(brain)
        .bind(version)
        .bind(status)
        .execute(&mut **tx)
        .await?;
    Ok(status)
}

async fn read_selection(
    tx: &mut Transaction<'_, Postgres>,
    auth: &Auth,
    brain: Uuid,
    operation: Option<Uuid>,
) -> Result<Option<serde_json::Value>> {
    let Some(id) = operation else {
        return Ok(None);
    };
    let bound = crate::workspace::bound_operation(tx, brain, id).await?;
    if bound.actor_id != auth.user.id
        || bound.device_id != auth.device_id
        || !bound.scope_valid
        || !matches!(bound.kind.as_str(), "context" | "retrieval")
    {
        return Err(Error::forbidden());
    }
    Ok(Some(
        serde_json::to_value(bound.scope.selection).expect("selection"),
    ))
}
