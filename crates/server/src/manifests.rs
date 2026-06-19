use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    jobs,
    publication::{self, Page},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use recollect_protocol::*;
use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, Transaction, types::Json as SqlJson};
use std::collections::BTreeSet;
use uuid::Uuid;
type Tx<'a> = Transaction<'a, Postgres>;
fn conflict(message: &'static str) -> Error {
    Error(StatusCode::CONFLICT, "manifest_conflict", message)
}
fn capacity() -> Error {
    Error(
        StatusCode::TOO_MANY_REQUESTS,
        "manifest_capacity",
        "Revision manifest capacity reached.",
    )
}
#[derive(Default, Deserialize)]
pub struct ManifestQuery {
    offset: Option<i64>,
    environment_id: Option<Uuid>,
}
fn validate(input: &mut ManifestInput) -> Result<()> {
    input.name = input.name.trim().into();
    if input.name.is_empty()
        || input.name.chars().count() > 120
        || input.name.chars().any(char::is_control)
        || input.notes.len() > 2048
    {
        return Err(Error::invalid(
            "Use a manifest name of 1–120 printable characters and notes of at most 2 KiB.",
        ));
    }
    if !matches!(
        input.kind.as_str(),
        "committed" | "desired" | "observed_deployed"
    ) {
        return Err(Error::invalid(
            "Select committed, desired or observed-deployed revisions.",
        ));
    }
    if input.kind == "observed_deployed" {
        if input.observed_at.is_none()
            || input
                .observation_reference
                .as_ref()
                .is_none_or(|s| s.trim().is_empty())
        {
            return Err(Error::invalid(
                "Recorded deployment observations require an observation time and supporting reference.",
            ));
        }
    } else if input.observed_at.is_some() || input.observation_reference.is_some() {
        return Err(Error::invalid(
            "Only observed-deployed manifests accept deployment observation metadata.",
        ));
    }
    if input
        .observation_reference
        .as_ref()
        .is_some_and(|s| s.chars().count() > 2000 || s.chars().any(char::is_control))
    {
        return Err(Error::invalid(
            "Use a printable observation reference of at most 2000 characters.",
        ));
    }
    if input.entries.is_empty() || input.entries.len() > 100 {
        return Err(Error::invalid("Select 1–100 distinct repositories."));
    }
    let mut repos = BTreeSet::new();
    for entry in &mut input.entries {
        if !repos.insert(entry.repository_id)
            || !git_object_id(&entry.revision)
            || entry.config_paths.len() > 100
            || entry.config_paths.iter().any(|p| !repository_path(p))
        {
            return Err(Error::invalid(
                "Each entry needs a unique repository, exact commit and up to 100 relative config paths.",
            ));
        }
        entry.config_paths.sort();
        entry.config_paths.dedup();
    }
    input.entries.sort_by_key(|e| e.repository_id);
    Ok(())
}
async fn current(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<ManifestRevision> {
    let row:Option<SqlJson<ManifestRevision>>=sqlx::query_scalar("SELECT CASE WHEN r.privacy_state='active' THEN r.revision END FROM revision_manifests m JOIN manifest_revisions r ON r.id=m.current_revision WHERE m.brain_id=$1 AND m.id=$2").bind(brain).bind(id).fetch_optional(&mut **tx).await?.ok_or_else(Error::missing)?;
    Ok(row.ok_or_else(crate::retention::unavailable)?.0)
}
#[utoipa::path(get,path="/api/brains/{brain}/revision-manifests",operation_id="revisionManifests",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query),("environment_id"=Option<Uuid>,Query)),responses((status=200,body=ManifestPage)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<ManifestQuery>,
) -> Result<Json<ManifestPage>> {
    let offset = publication::offset(&Page {
        offset: query.offset,
    })?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    if let Some(env) = query.environment_id {
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_groups WHERE id=$1 AND brain_id=$2 AND kind='environment')").bind(env).bind(brain).fetch_one(&mut *tx).await?;
        if !exists {
            return Err(Error::missing());
        }
    }
    let total=sqlx::query_scalar("SELECT count(*) FROM revision_manifests m JOIN manifest_revisions r ON r.id=m.current_revision WHERE m.brain_id=$1 AND r.privacy_state='active' AND ($2::uuid IS NULL OR m.environment_id=$2)").bind(brain).bind(query.environment_id).fetch_one(&mut *tx).await?;
    let items=sqlx::query_scalar::<_,SqlJson<ManifestRevision>>("SELECT r.revision FROM revision_manifests m JOIN manifest_revisions r ON r.id=m.current_revision WHERE m.brain_id=$1 AND r.privacy_state='active' AND ($2::uuid IS NULL OR m.environment_id=$2) ORDER BY lower(m.name),m.id LIMIT 20 OFFSET $3").bind(brain).bind(query.environment_id).bind(offset).fetch_all(&mut *tx).await?.into_iter().map(|r|r.0).collect();
    tx.commit().await?;
    Ok(Json(ManifestPage {
        items,
        total,
        offset,
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/revision-manifests/{manifest}",operation_id="revisionManifest",params(("brain"=Uuid,Path),("manifest"=Uuid,Path),("offset"=Option<i64>,Query)),responses((status=200,body=ManifestDetail)))]
pub async fn detail(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(page): Query<Page>,
) -> Result<Json<ManifestDetail>> {
    let offset = publication::offset(&page)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let current = current(&mut tx, brain, id).await?;
    let total = sqlx::query_scalar(
        "SELECT count(*) FROM manifest_revisions WHERE brain_id=$1 AND manifest_id=$2 AND privacy_state='active'",
    )
    .bind(brain)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let history=sqlx::query_scalar::<_,SqlJson<ManifestRevision>>("SELECT revision FROM manifest_revisions WHERE brain_id=$1 AND manifest_id=$2 AND privacy_state='active' ORDER BY created_at DESC,id LIMIT 20 OFFSET $3").bind(brain).bind(id).bind(offset).fetch_all(&mut *tx).await?.into_iter().map(|r|r.0).collect();
    tx.commit().await?;
    Ok(Json(ManifestDetail {
        current,
        history,
        total,
        offset,
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/revision-manifests/{manifest}/revisions/{revision}",operation_id="manifestRevision",params(("brain"=Uuid,Path),("manifest"=Uuid,Path),("revision"=Uuid,Path)),responses((status=200,body=ManifestRevision)))]
pub async fn revision(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, manifest, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<ManifestRevision>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let revision: Option<SqlJson<ManifestRevision>> = sqlx::query_scalar(
        "SELECT CASE WHEN privacy_state='active' THEN revision END FROM manifest_revisions WHERE brain_id=$1 AND manifest_id=$2 AND id=$3",
    )
    .bind(brain)
    .bind(manifest)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(Error::missing)?;
    tx.commit().await?;
    Ok(Json(revision.ok_or_else(crate::retention::unavailable)?.0))
}
#[utoipa::path(post,path="/api/brains/{brain}/revision-manifests",operation_id="createRevisionManifest",params(("brain"=Uuid,Path)),request_body=ManifestInput,responses((status=200,body=ManifestRevision)))]
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<ManifestInput>,
) -> Result<Json<ManifestRevision>> {
    save(&state, &auth, brain, None, &headers, input)
        .await
        .map(Json)
}
#[utoipa::path(put,path="/api/brains/{brain}/revision-manifests/{manifest}",operation_id="updateRevisionManifest",params(("brain"=Uuid,Path),("manifest"=Uuid,Path)),request_body=ManifestInput,responses((status=200,body=ManifestRevision)))]
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ManifestInput>,
) -> Result<Json<ManifestRevision>> {
    save(&state, &auth, brain, Some(id), &headers, input)
        .await
        .map(Json)
}
async fn save(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    existing: Option<Uuid>,
    headers: &HeaderMap,
    mut input: ManifestInput,
) -> Result<ManifestRevision> {
    validate(&mut input)?;
    publication::safe_payload(state, &json!(input))?;
    let key = commands::key(headers)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    let environment:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_groups WHERE brain_id=$1 AND id=$2 AND kind='environment')").bind(brain).bind(input.environment_id).fetch_one(&mut *tx).await?;
    if !environment {
        return Err(Error::missing());
    }
    for entry in &input.entries {
        let repo: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM repositories WHERE id=$1 AND brain_id=$2)",
        )
        .bind(entry.repository_id)
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
        if !repo {
            return Err(Error::missing());
        }
        if let Some(snapshot) = entry.snapshot_id {
            let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM repository_snapshots WHERE id=$1 AND brain_id=$2 AND repository_id=$3 AND revision=$4 AND recollect_content_state(brain_id,'repository',privacy_state,created_at)='active')").bind(snapshot).bind(brain).bind(entry.repository_id).bind(&entry.revision).fetch_one(&mut *tx).await?;
            if !valid {
                return Err(Error::invalid(
                    "Selected snapshot does not match this Brain, repository and exact commit.",
                ));
            }
        }
    }
    let selection = ScopeSelection {
        repository_ids: input.entries.iter().map(|e| e.repository_id).collect(),
        environment_id: Some(input.environment_id),
        ..Default::default()
    };
    let scope = if let Some(op) = input.operation_id {
        Some(
            publication::operation(
                &mut tx,
                brain,
                op,
                auth.user.id,
                auth.device_id,
                &selection,
                true,
            )
            .await?
            .scope,
        )
    } else {
        None
    };
    if let Some(reply) = commands::reserve::<ManifestRevision>(
        &mut tx,
        key.as_deref(),
        "manifest.save",
        json!({"brain":brain,"manifest":existing,"input":input}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(reply);
    }
    let id = if let Some(id) = existing {
        let previous = current(&mut tx, brain, id).await?;
        if input.base_revision != Some(previous.id) {
            return Err(conflict(
                "This manifest changed. Reload the current revision before saving.",
            ));
        }
        if input.environment_id != previous.environment_id || input.name != previous.name {
            return Err(Error::invalid(
                "A named manifest keeps its environment and name. Create another manifest for a different selection.",
            ));
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM manifest_revisions WHERE manifest_id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 1000 {
            return Err(capacity());
        }
        id
    } else {
        if input.base_revision.is_some() {
            return Err(Error::invalid("New manifests do not have a base revision."));
        }
        let duplicate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM revision_manifests WHERE environment_id=$1 AND lower(name)=lower($2))").bind(input.environment_id).bind(&input.name).fetch_one(&mut *tx).await?;
        if duplicate {
            return Err(conflict(
                "This environment already has a manifest with that name.",
            ));
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM revision_manifests WHERE environment_id=$1")
                .bind(input.environment_id)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 100 {
            return Err(capacity());
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO revision_manifests(id,brain_id,environment_id,name) VALUES($1,$2,$3,$4)",
        )
        .bind(id)
        .bind(brain)
        .bind(input.environment_id)
        .bind(&input.name)
        .execute(&mut *tx)
        .await?;
        id
    };
    jobs::capacity(&mut tx, brain).await?;
    let revision = ManifestRevision {
        id: Uuid::new_v4(),
        manifest_id: id,
        brain_id: brain,
        environment_id: input.environment_id,
        name: input.name,
        kind: input.kind,
        entries: input.entries,
        scope,
        actor_id: auth.user.id,
        observed_at: input.observed_at,
        observation_reference: input.observation_reference,
        notes: input.notes,
        created_at: Utc::now(),
    };
    sqlx::query(
        "INSERT INTO manifest_revisions(id,manifest_id,brain_id,revision) VALUES($1,$2,$3,$4)",
    )
    .bind(revision.id)
    .bind(id)
    .bind(brain)
    .bind(SqlJson(&revision))
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE revision_manifests SET current_revision=$3 WHERE brain_id=$1 AND id=$2")
        .bind(brain)
        .bind(id)
        .bind(revision.id)
        .execute(&mut *tx)
        .await?;
    crate::memory_rules::advance_epoch(&mut tx, brain).await?;
    let audit = db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "manifest.revise",
        revision.id,
        "revision_recorded",
    )
    .await?;
    jobs::enqueue(&mut tx, auth.user.id, brain, audit).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &revision).await?;
    tx.commit().await?;
    Ok(revision)
}
