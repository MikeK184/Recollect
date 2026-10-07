use crate::{AppState, config::Config, privacy::Manifest};
use anyhow::{Context, ensure};
use chrono::{DateTime, Utc};
use recollect_protocol::ErasureTarget;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, types::Json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub(crate) mod mirror;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub installation_id: Uuid,
    pub id: Uuid,
    pub brain_id: Uuid,
    pub sequence: i64,
    pub target: ErasureTarget,
    pub cause: String,
    pub manifest: Manifest,
    pub created_at: DateTime<Utc>,
}
fn directory(config: &Config, id: Uuid) -> PathBuf {
    Path::new(&config.erasure_journal).join(id.to_string())
}
async fn identity(pool: &PgPool) -> anyhow::Result<(Uuid, bool)> {
    Ok(
        sqlx::query_as("SELECT id,initialized FROM privacy_installation")
            .fetch_one(pool)
            .await?,
    )
}
pub(crate) async fn durable_write(path: &Path, body: &[u8]) -> anyhow::Result<()> {
    let parent = path.parent().context("Journal path has no directory")?;
    let tmp = parent.join(format!("{}.tmp", Uuid::new_v4()));
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&tmp).await?;
    file.write_all(body).await?;
    file.flush().await?;
    file.sync_all().await?;
    tokio::fs::rename(&tmp, path).await?;
    #[cfg(unix)]
    tokio::fs::File::open(parent).await?.sync_all().await?;
    Ok(())
}
async fn marker(config: &Config, id: Uuid) -> anyhow::Result<()> {
    let path = directory(config, id).join("installation.json");
    let saved: Uuid = serde_json::from_slice(&tokio::fs::read(path).await.context(
        "Erasure journal is unavailable; restore the retained journal before serving content",
    )?)?;
    ensure!(
        saved == id,
        "Erasure journal belongs to another installation"
    );
    Ok(())
}
/// Initialization is explicit migration/setup work, never a serving fallback.
pub async fn initialize(admin: &PgPool, config: &Config) -> anyhow::Result<()> {
    let mut tx = admin.begin().await?;
    let (id, initialized): (Uuid, bool) =
        sqlx::query_as("SELECT id,initialized FROM privacy_installation FOR UPDATE")
            .fetch_one(&mut *tx)
            .await?;
    if initialized {
        marker(config, id).await?;
    } else {
        let dir = directory(config, id);
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join("installation.json");
        match tokio::fs::try_exists(&path).await? {
            true => marker(config, id).await?,
            false => durable_write(&path, &serde_json::to_vec(&id)?).await?,
        }
        #[cfg(unix)]
        {
            tokio::fs::File::open(&config.erasure_journal)
                .await?
                .sync_all()
                .await?;
            let parent = Path::new(&config.erasure_journal)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            tokio::fs::File::open(parent).await?.sync_all().await?;
        }
        sqlx::query("UPDATE privacy_installation SET initialized=true")
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    crate::graph::analytics::initialize(admin, config).await?;
    Ok(())
}
async fn export(pool: &PgPool, config: &Config, entry: &Entry) -> anyhow::Result<()> {
    marker(config, entry.installation_id).await?;
    let path = directory(config, entry.installation_id).join(format!("{}.json", entry.id));
    if tokio::fs::try_exists(&path).await? {
        let old: Entry = serde_json::from_slice(&tokio::fs::read(&path).await?)?;
        ensure!(
            old == *entry,
            "Erasure journal entry differs from canonical request"
        );
    } else {
        durable_write(&path, &serde_json::to_vec(entry)?).await?;
    }
    mirror::publish(pool, config, entry).await?;
    Ok(())
}
async fn entries(config: &Config, id: Uuid) -> anyhow::Result<Vec<Entry>> {
    marker(config, id).await?;
    let mut dir = tokio::fs::read_dir(directory(config, id)).await?;
    let mut entries = Vec::new();
    while let Some(file) = dir.next_entry().await? {
        let name = file.file_name();
        let name = name.to_string_lossy();
        if name == "installation.json" || name.ends_with(".tmp") {
            continue;
        }
        ensure!(
            file.file_type().await?.is_file(),
            "Unexpected journal storage entry"
        );
        let request: Uuid = name
            .strip_suffix(".json")
            .context("Unexpected journal filename")?
            .parse()?;
        let entry: Entry = serde_json::from_slice(&tokio::fs::read(file.path()).await?)?;
        ensure!(
            entry.installation_id == id
                && entry.id == request
                && entry.sequence > 0
                && matches!(entry.cause.as_str(), "erase" | "expire"),
            "Invalid journal identity"
        );
        entries.push(entry);
    }
    entries.sort_by_key(|e| e.sequence);
    Ok(entries)
}
pub async fn remove_artifact(config: &Config, brain: Uuid, id: Uuid) -> anyhow::Result<()> {
    let path = crate::artifacts::path(&config.artifact_dir, brain, id);
    let parent = path.parent().context("Missing artifact directory")?;
    match tokio::fs::symlink_metadata(parent).await {
        Ok(meta) => ensure!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "Artifact directory unavailable"
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    }
    match tokio::fs::symlink_metadata(&path).await {
        Ok(meta) => ensure!(
            meta.is_file() && !meta.file_type().is_symlink(),
            "Artifact storage is not a regular file"
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    }
    match tokio::fs::remove_file(&path).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    }
    #[cfg(unix)]
    tokio::fs::File::open(parent).await?.sync_all().await?;
    Ok(())
}
#[derive(sqlx::FromRow)]
struct Pending {
    id: Uuid,
    brain_id: Uuid,
    sequence: i64,
    target: Json<ErasureTarget>,
    cause: String,
    manifest: Json<Manifest>,
    created_at: DateTime<Utc>,
}
/// Export and clean only already-authorized canonical requests.
pub async fn maintain(state: &AppState) -> anyhow::Result<usize> {
    let (installation_id, initialized) = identity(&state.pool).await?;
    ensure!(
        initialized,
        "Initialize the erasure journal with the migration command"
    );
    let pending: Vec<Pending> = sqlx::query_as("SELECT * FROM recollect_privacy_pending()")
        .fetch_all(&state.pool)
        .await?;
    let count = pending.len();
    for r in pending {
        let entry = Entry {
            installation_id,
            id: r.id,
            brain_id: r.brain_id,
            sequence: r.sequence,
            target: r.target.0,
            cause: r.cause,
            manifest: r.manifest.0,
            created_at: r.created_at,
        };
        let exported = export(&state.pool, &state.config, &entry).await.is_ok();
        let mut removed = Vec::new();
        let mut failure = (!exported).then_some("journal_unavailable");
        // Tombstones already deny reads. Preserve artifacts for retry if durable export failed.
        if exported {
            if crate::graph::analytics::erase(
                &state.config,
                &state.pool,
                entry.brain_id,
                entry.sequence,
                entry.id,
            )
            .await
            .is_ok()
                && crate::graph::erase(&state.config, entry.brain_id, &entry.manifest)
                    .await
                    .is_ok()
            {
                sqlx::query("SELECT recollect_privacy_graph_done($1)")
                    .bind(entry.id)
                    .execute(&state.pool)
                    .await?;
            } else {
                failure = Some("graph_unavailable");
            }
            for id in &entry.manifest.artifacts {
                if remove_artifact(&state.config, entry.brain_id, *id)
                    .await
                    .is_ok()
                {
                    removed.push(*id)
                } else {
                    failure = Some("artifact_unavailable");
                }
            }
        }
        sqlx::query("SELECT recollect_privacy_progress($1,$2,$3,$4)")
            .bind(entry.id)
            .bind(exported)
            .bind(removed)
            .bind(failure)
            .execute(&state.pool)
            .await?;
    }
    Ok(count)
}
pub async fn run_once(state: &AppState) -> anyhow::Result<usize> {
    crate::support_excerpts::reconcile_artifacts(state, None).await?;
    for _ in 0..20 {
        let request: Option<Uuid> = sqlx::query_scalar("SELECT recollect_expire_one()")
            .fetch_one(&state.pool)
            .await?;
        if request.is_none() {
            break;
        }
    }
    sqlx::query("SELECT recollect_expire_audit()")
        .execute(&state.pool)
        .await?;
    sqlx::query("SELECT recollect_expire_model_details()")
        .execute(&state.pool)
        .await?;
    maintain(state).await
}
/// Replaying supplied closures requires the operator database role.
pub async fn reconcile(admin: &PgPool, config: &Config) -> anyhow::Result<()> {
    let (id, initialized) = identity(admin).await?;
    ensure!(initialized, "Journal has not been initialized");
    mirror::check(config, id).await?;
    for entry in entries(config, id).await? {
        export(admin, config, &entry).await?;
        sqlx::query("SELECT recollect_privacy_replay($1)")
            .bind(Json(&entry))
            .execute(admin)
            .await?;
        crate::graph::analytics::erase(config, admin, entry.brain_id, entry.sequence, entry.id)
            .await?;
        crate::graph::erase(config, entry.brain_id, &entry.manifest)
            .await
            .map_err(|_| anyhow::anyhow!("Graph privacy replay did not complete"))?;
        sqlx::query("SELECT recollect_privacy_graph_done($1)")
            .bind(entry.id)
            .execute(admin)
            .await?;
        for artifact in &entry.manifest.artifacts {
            remove_artifact(config, entry.brain_id, *artifact).await?;
        }
        sqlx::query("SELECT recollect_privacy_progress($1,true,$2,NULL)")
            .bind(entry.id)
            .bind(&entry.manifest.artifacts)
            .execute(admin)
            .await?;
    }
    crate::graph::analytics::reconcile(config, admin).await?;
    Ok(())
}
/// Startup refuses an older database until its retained journal has been replayed.
pub async fn barrier(pool: &PgPool, config: &Config) -> anyhow::Result<()> {
    let (id, initialized) = identity(pool).await?;
    ensure!(initialized, "Initialize the erasure journal before serving");
    mirror::check(config, id).await?;
    let entries = entries(config, id).await?;
    let positions: Vec<(Uuid, i64, bool)> =
        sqlx::query_as("SELECT * FROM recollect_privacy_positions()")
            .fetch_all(pool)
            .await?;
    let canonical: BTreeMap<_, _> = positions
        .iter()
        .map(|(id, seq, journaled)| (*id, (*seq, *journaled)))
        .collect();
    let retained: BTreeMap<_, _> = entries.iter().map(|e| (e.id, e.sequence)).collect();
    for (id, sequence, journaled) in positions {
        ensure!(
            !journaled || retained.get(&id) == Some(&sequence),
            "Retained erasure journal is incomplete; restore it before serving"
        );
    }
    for entry in entries {
        ensure!(
            canonical
                .get(&entry.id)
                .is_some_and(|(seq, _)| *seq == entry.sequence),
            "Database predates the erasure journal; run privacy-reconcile before serving"
        );
        // This also synchronizes pre-existing local acknowledgements when an
        // operator explicitly enables a new remote recovery mirror.
        mirror::publish(pool, config, &entry).await?;
        // A restored artifact copy must not survive merely because its DB request was complete.
        crate::graph::analytics::erase(config, pool, entry.brain_id, entry.sequence, entry.id)
            .await?;
        crate::graph::erase(config, entry.brain_id, &entry.manifest)
            .await
            .map_err(|_| anyhow::anyhow!("Graph privacy replay did not complete"))?;
        for artifact in entry.manifest.artifacts {
            remove_artifact(config, entry.brain_id, artifact).await?;
        }
    }
    crate::graph::analytics::reconcile(config, pool).await?;
    Ok(())
}
