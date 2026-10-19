//! Brain-owned curated guidance, immutable history and current eligibility.
use crate::{
    AppState,
    auth::Auth,
    db,
    error::{Error, Result},
    workspace,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use recollect_protocol::*;
use serde::Deserialize;
use sqlx::{Postgres, Transaction, types::Json as DbJson};
use uuid::Uuid;

type Tx<'a> = Transaction<'a, Postgres>;

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    environment_id: Option<Uuid>,
    operation_id: Option<Uuid>,
    query: Option<String>,
    #[serde(default)]
    include_disabled: bool,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FetchQuery {
    environment_id: Option<Uuid>,
    operation_id: Option<Uuid>,
    version: Option<i32>,
    path: Option<String>,
    #[serde(default)]
    entrypoint_only: bool,
}
fn conflict() -> Error {
    Error(
        StatusCode::CONFLICT,
        "skill_version_conflict",
        "The name already exists or this skill changed. Reload its current version and preserve your draft.",
    )
}
fn mutation_error(error: sqlx::Error) -> Error {
    if error
        .as_database_error()
        .is_some_and(|e| e.is_unique_violation())
    {
        conflict()
    } else {
        error.into()
    }
}
async fn transaction<'a>(
    state: &'a AppState,
    auth: &Auth,
    brain: Uuid,
    write: bool,
) -> Result<(Tx<'a>, String)> {
    if write {
        auth.require_browser()?;
    }
    let mut tx = auth.publication_tx(&state.pool).await?;
    let role = db::require_role(&mut tx, brain, write).await?;
    let archived: bool = sqlx::query_scalar("SELECT archived FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
    if archived && (write || auth.device_id.is_some() || role != "admin") {
        return Err(Error(
            StatusCode::CONFLICT,
            "brain_archived",
            "Reopen this Brain before publishing or consuming shared guidance.",
        ));
    }
    Ok((tx, role))
}
async fn environment(tx: &mut Tx<'_>, brain: Uuid, id: Option<Uuid>) -> Result<()> {
    if !workspace::selection_valid(
        tx,
        brain,
        &ScopeSelection {
            environment_id: id,
            ..Default::default()
        },
    )
    .await?
    {
        return Err(Error::missing());
    }
    Ok(())
}
async fn selection(
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    selected: Option<Uuid>,
    operation: Option<Uuid>,
) -> Result<Option<Uuid>> {
    let selected = match operation {
        Some(id) => {
            let bound = workspace::bound_operation(tx, brain, id).await?;
            let closed: bool = sqlx::query_scalar(
                "SELECT closed FROM workspace_tasks WHERE brain_id=$1 AND id=$2",
            )
            .bind(brain)
            .bind(bound.task_id)
            .fetch_one(&mut **tx)
            .await?;
            if bound.actor_id != auth.user.id
                || bound.device_id != auth.device_id
                || !bound.scope_valid
                || closed
                || !matches!(bound.kind.as_str(), "context" | "retrieval")
                || selected.is_some_and(|env| Some(env) != bound.scope.selection.environment_id)
            {
                return Err(Error::forbidden());
            }
            bound.scope.selection.environment_id
        }
        None if auth.device_id.is_some() => return Err(Error::forbidden()),
        None => selected,
    };
    environment(tx, brain, selected).await?;
    Ok(selected)
}
fn applies(environment: Option<Uuid>, selected: Option<Uuid>) -> bool {
    environment.is_none() || environment == selected
}
async fn current(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<ManagedSkillSummary> {
    let summary: DbJson<ManagedSkillSummary> =
        sqlx::query_scalar("SELECT summary FROM managed_skills WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(Error::missing)?;
    Ok(summary.0)
}

#[utoipa::path(get,path="/api/brains/{brain}/skills",operation_id="managedSkills",params(("brain"=Uuid,Path),("environment_id"=Option<Uuid>,Query),("operation_id"=Option<Uuid>,Query),("query"=Option<String>,Query),("include_disabled"=Option<bool>,Query)),responses((status=200,body=Vec<ManagedSkillSummary>)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ManagedSkillSummary>>> {
    let (mut tx, role) = transaction(&state, &auth, brain, false).await?;
    let selected = selection(
        &mut tx,
        &auth,
        brain,
        query.environment_id,
        query.operation_id,
    )
    .await?;
    if query.include_disabled && (role != "admin" || auth.device_id.is_some()) {
        return Err(Error::forbidden());
    }
    let filter = query.query.unwrap_or_default();
    if filter.len() > 200 || filter.chars().any(char::is_control) {
        return Err(Error::invalid(
            "Search at most 200 UTF-8 bytes without control characters.",
        ));
    }
    let summaries: Vec<DbJson<ManagedSkillSummary>> = sqlx::query_scalar("SELECT summary FROM managed_skills WHERE brain_id=$1 AND (environment_id IS NULL OR environment_id=$2) AND (enabled OR $3) ORDER BY name,id")
        .bind(brain).bind(selected).bind(query.include_disabled).fetch_all(&mut *tx).await?;
    let filter = filter.to_lowercase();
    let rows = summaries
        .into_iter()
        .map(|r| r.0)
        .filter(|r| {
            filter.is_empty()
                || r.name.to_lowercase().contains(&filter)
                || r.description.to_lowercase().contains(&filter)
                || r.tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&filter))
        })
        .collect();
    tx.commit().await?;
    Ok(Json(rows))
}

#[utoipa::path(get,path="/api/brains/{brain}/skills/{id}",operation_id="managedSkill",params(("brain"=Uuid,Path),("id"=Uuid,Path),("environment_id"=Option<Uuid>,Query),("operation_id"=Option<Uuid>,Query),("version"=Option<i32>,Query),("path"=Option<String>,Query),("entrypoint_only"=Option<bool>,Query)),responses((status=200,body=ManagedSkillBundle)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(query): Query<FetchQuery>,
) -> Result<Json<ManagedSkillBundle>> {
    if query.version.is_some_and(|v| v < 1)
        || query
            .path
            .as_deref()
            .is_some_and(|p| !validate_managed_skill_path(p))
    {
        return Err(Error::invalid(
            "Select a positive version and exact safe text resource path.",
        ));
    }
    let (mut tx, role) = transaction(&state, &auth, brain, false).await?;
    let selected = selection(
        &mut tx,
        &auth,
        brain,
        query.environment_id,
        query.operation_id,
    )
    .await?;
    let head = current(&mut tx, brain, id).await?;
    if !applies(head.environment_id, selected)
        || (!head.enabled && (auth.device_id.is_some() || role != "admin"))
    {
        return Err(Error::missing());
    }
    let version = query.version.unwrap_or(head.version);
    let row: Option<(DbJson<ManagedSkillSummary>,DbJson<Vec<ManagedSkillFile>>)> = sqlx::query_as("SELECT summary,CASE WHEN $4::text IS NOT NULL OR $5 THEN coalesce((SELECT jsonb_agg(f) FROM jsonb_array_elements(files) f WHERE f->>'path'=coalesce($4,CASE summary->>'kind' WHEN 'skill' THEN 'SKILL.md' ELSE 'AGENTS.md' END)),'[]'::jsonb) ELSE files END FROM managed_skill_versions WHERE brain_id=$1 AND skill_id=$2 AND version=$3")
        .bind(brain).bind(id).bind(version).bind(&query.path).bind(query.entrypoint_only).fetch_optional(&mut *tx).await?;
    let (skill, files) = row.ok_or_else(Error::missing)?;
    if !applies(skill.environment_id, selected) {
        return Err(Error::missing());
    }
    if files.0.is_empty() {
        return Err(Error::missing());
    }
    tx.commit().await?;
    Ok(Json(ManagedSkillBundle {
        skill: skill.0,
        files: files.0,
    }))
}

async fn publish(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    id: Option<Uuid>,
    mut input: ManagedSkillInput,
) -> Result<ManagedSkillBundle> {
    let (mut tx, _) = transaction(state, auth, brain, true).await?;
    validate_managed_skill_bundle(input.kind, &input.name, &input.description, &input.files)
        .map_err(Error::invalid)?;
    if input.profile_ids.len() > 16
        || input
            .profile_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != input.profile_ids.len()
        || input.tags.len() > 32
        || input
            .tags
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != input.tags.len()
        || input.tags.iter().any(|tag| {
            tag.trim().is_empty()
                || tag.chars().count() > 64
                || tag.chars().any(char::is_control)
                || repository_secret(tag, &[])
        })
    {
        return Err(Error::invalid(
            "Select at most 16 distinct same-Brain profiles and 32 distinct nonempty tags of at most 64 characters without secrets or control characters.",
        ));
    }
    environment(&mut tx, brain, input.environment_id).await?;
    for profile in &input.profile_ids {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM mcp_profiles WHERE brain_id=$1 AND id=$2)",
        )
        .bind(brain)
        .bind(profile)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(Error::missing());
        }
    }
    let version = if let Some(id) = id {
        let head = current(&mut tx, brain, id).await?;
        if input.expected_version != Some(head.version) {
            return Err(conflict());
        }
        head.version.checked_add(1).ok_or_else(conflict)?
    } else {
        if input.expected_version.is_some() {
            return Err(Error::invalid(
                "Creating a skill requires an absent expected version.",
            ));
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM managed_skills WHERE brain_id=$1")
                .bind(brain)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 64 {
            return Err(Error(
                StatusCode::TOO_MANY_REQUESTS,
                "skill_catalogue_capacity",
                "This Brain already contains the maximum 64 skill bundles, including disabled bundles.",
            ));
        }
        1
    };
    input.files.sort_by(|a, b| a.path.cmp(&b.path));
    input.profile_ids.sort_unstable();
    input.tags.sort();
    let id = id.unwrap_or_else(Uuid::new_v4);
    let skill = ManagedSkillSummary {
        id,
        brain_id: brain,
        name: input.name,
        description: input.description,
        kind: input.kind,
        version,
        environment_id: input.environment_id,
        profile_ids: input.profile_ids,
        tags: input.tags,
        enabled: input.enabled,
        file_paths: input.files.iter().map(|f| f.path.clone()).collect(),
        updated_at: Utc::now(),
    };
    if serde_json::to_vec(&skill)
        .map_err(|_| Error::invalid("Skill metadata could not be encoded."))?
        .len()
        > 12 * 1024
    {
        return Err(Error::invalid(
            "Keep combined skill metadata and its resource manifest within 12 KiB of JSON.",
        ));
    }
    if version == 1 {
        sqlx::query("INSERT INTO managed_skills(id,brain_id,name,version,environment_id,enabled,summary) VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(id).bind(brain).bind(&skill.name).bind(version).bind(skill.environment_id).bind(skill.enabled).bind(DbJson(&skill)).execute(&mut *tx).await.map_err(mutation_error)?;
    } else {
        sqlx::query("UPDATE managed_skills SET name=$3,version=$4,environment_id=$5,enabled=$6,summary=$7 WHERE id=$1 AND brain_id=$2")
            .bind(id).bind(brain).bind(&skill.name).bind(version).bind(skill.environment_id).bind(skill.enabled).bind(DbJson(&skill)).execute(&mut *tx).await.map_err(mutation_error)?;
    }
    sqlx::query("INSERT INTO managed_skill_versions(id,brain_id,skill_id,version,environment_id,summary,files,created_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(Uuid::new_v4()).bind(brain).bind(id).bind(version).bind(skill.environment_id).bind(DbJson(&skill)).bind(DbJson(&input.files)).bind(auth.user.id).execute(&mut *tx).await?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        if version == 1 {
            "skills.create"
        } else {
            "skills.update"
        },
        id,
        if skill.enabled { "enabled" } else { "disabled" },
    )
    .await?;
    tx.commit().await?;
    Ok(ManagedSkillBundle {
        skill,
        files: input.files,
    })
}
#[utoipa::path(post,path="/api/brains/{brain}/skills",operation_id="createManagedSkill",params(("brain"=Uuid,Path)),request_body=ManagedSkillInput,responses((status=200,body=ManagedSkillBundle)))]
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<ManagedSkillInput>,
) -> Result<Json<ManagedSkillBundle>> {
    publish(&state, &auth, brain, None, input).await.map(Json)
}
#[utoipa::path(put,path="/api/brains/{brain}/skills/{id}",operation_id="updateManagedSkill",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=ManagedSkillInput,responses((status=200,body=ManagedSkillBundle)))]
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ManagedSkillInput>,
) -> Result<Json<ManagedSkillBundle>> {
    publish(&state, &auth, brain, Some(id), input)
        .await
        .map(Json)
}
