//! Explicit operator recovery. Normal startup never clears a restore hold.
use crate::{AppState, artifacts, config::Config, db, privacy_journal};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hold {
    pub restore_id: Uuid,
    pub backup_id: Uuid,
    pub installation_id: Uuid,
    pub source_origin: String,
}

pub fn hold_path(config: &Config) -> PathBuf {
    Path::new(&config.erasure_journal).join("recovery-hold.json")
}

pub async fn guard(config: &Config) -> anyhow::Result<()> {
    // Broken links and unreadable markers also hold the installation.
    match tokio::fs::symlink_metadata(hold_path(config)).await {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => anyhow::bail!(
            "Installation is held for recovery; complete its explicit restore before startup"
        ),
    }
}

pub async fn held(config: &Config, pool: &PgPool) -> anyhow::Result<Hold> {
    let path = hold_path(config);
    let metadata = tokio::fs::symlink_metadata(&path).await?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 1024,
        "Recovery marker is invalid"
    );
    let hold: Hold = serde_json::from_slice(&tokio::fs::read(path).await?)?;
    let identity: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(pool)
        .await?;
    ensure!(
        hold.installation_id == identity,
        "Recovery marker belongs to another installation"
    );
    Ok(hold)
}

pub async fn artifacts_available(admin: &PgPool, config: &Config) -> anyhow::Result<usize> {
    let rows: Vec<(Uuid, Uuid, i32)> = sqlx::query_as(
        "SELECT brain_id,artifact_id,byte_length FROM source_versions WHERE artifact_id IS NOT NULL AND privacy_state='active'
         UNION SELECT brain_id,id,byte_length FROM repository_artifacts
         UNION SELECT brain_id,artifact_id,byte_length FROM repository_files WHERE artifact_id IS NOT NULL")
        .fetch_all(admin).await?;
    for (brain, id, length) in &rows {
        let path = artifacts::path(&config.artifact_dir, *brain, *id);
        let meta = tokio::fs::symlink_metadata(path)
            .await
            .context("Required recovery artifact is unavailable")?;
        ensure!(
            meta.is_file() && !meta.file_type().is_symlink() && meta.len() == *length as u64,
            "Required recovery artifact has an invalid length or type"
        );
    }
    Ok(rows.len())
}

pub async fn checkpoint(admin: &PgPool, config: &Config) -> anyhow::Result<()> {
    let pool = db::pool(&config.database_url).await?;
    let state = AppState::new(pool, config.clone())?;
    // A bounded pass can create up to twenty expiry requests. Continue only
    // while work progresses; an unavailable graph/mirror cannot be reported as
    // a completed privacy checkpoint.
    for _ in 0..1000 {
        let count = privacy_journal::run_once(&state).await?;
        let incomplete: i64 =
            sqlx::query_scalar("SELECT count(*) FROM privacy_requests WHERE state<>'complete'")
                .fetch_one(admin)
                .await?;
        ensure!(
            incomplete == 0,
            "Privacy maintenance is pending; backup/recovery checkpoint is unavailable"
        );
        if count == 0 {
            privacy_journal::barrier(&state.pool, config).await?;
            artifacts_available(admin, config).await?;
            return Ok(());
        }
    }
    anyhow::bail!(
        "Privacy maintenance exceeded its checkpoint bound; continue maintenance before retrying"
    )
}

/// Run only against a stopped, held destination. The transaction is repeatable:
/// terminal outcomes and existing audit records are never rewritten on retry.
pub async fn prepare(admin: &PgPool, config: &Config) -> anyhow::Result<()> {
    let hold = held(config, admin).await?;
    privacy_journal::reconcile(admin, config).await?;
    checkpoint(admin, config).await?;
    let mut tx = admin.begin().await?;
    sqlx::raw_sql(include_str!("recovery_prepare.sql"))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let outbox = recollect_mcp_runtime::outbox::Outbox::relocate(
        crate::mcp::runtime::central::receipt_directory(config),
        format!("central:{}", hold.source_origin),
        format!("central:{}", config.public_origin),
    )
    .await?;
    let ids = outbox.identities(std::time::SystemTime::now()).await?;
    let removed: Vec<Uuid> =
        sqlx::query_scalar("SELECT * FROM recollect_mcp_receipt_removals('central',$1)")
            .bind(ids)
            .fetch_all(admin)
            .await?;
    outbox.remove(removed).await?;
    // Keep the marker: the operator still has to verify the target and record a
    // completed restore. A crash after this command cannot start the server.
    Ok(())
}

pub async fn release(admin: &PgPool, config: &Config) -> anyhow::Result<()> {
    let path = hold_path(config);
    let completed = path.with_file_name("recovery-released.json");
    if !tokio::fs::try_exists(&path).await? && tokio::fs::try_exists(&completed).await? {
        // An atomic marker move can complete before the operator's status write.
        // This acknowledgement is identity-only; no restore step is repeated.
        let bytes = tokio::fs::read(&completed).await?;
        ensure!(bytes.len() <= 1024, "Recovery completion marker is invalid");
        let hold: Hold = serde_json::from_slice(&bytes)?;
        let identity: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
            .fetch_one(admin)
            .await?;
        ensure!(
            hold.installation_id == identity,
            "Recovery completion marker differs"
        );
        return Ok(());
    }
    held(config, admin).await?;
    db::ready(admin).await?;
    let outstanding: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM mcp_calls WHERE state IN ('queued','starting','running')) + (SELECT count(*) FROM model_requests WHERE state='running') + (SELECT count(*) FROM mcp_instances WHERE state IN ('starting','ready','draining'))")
        .fetch_one(admin).await?;
    ensure!(
        outstanding == 0,
        "Recovered execution ownership has not been fenced"
    );
    let pool = db::pool(&config.database_url).await?;
    privacy_journal::barrier(&pool, config).await?;
    artifacts_available(admin, config).await?;
    tokio::fs::rename(&path, completed).await?;
    #[cfg(unix)]
    tokio::fs::File::open(path.parent().unwrap())
        .await?
        .sync_all()
        .await?;
    Ok(())
}
