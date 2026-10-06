use crate::error::Result;
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use std::time::Duration;
use uuid::Uuid;

pub async fn pool(url: &str) -> std::result::Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET statement_timeout = '10s'")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(url)
        .await
}

pub const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_platform",
        include_str!("../migrations/001_platform.sql"),
    ),
    (
        "002_durable_work",
        include_str!("../migrations/002_durable_work.sql"),
    ),
    (
        "003_team_access",
        include_str!("../migrations/003_team_access.sql"),
    ),
    (
        "004_device_pairing",
        include_str!("../migrations/004_device_pairing.sql"),
    ),
    (
        "005_evidence_collections",
        include_str!("../migrations/005_evidence_collections.sql"),
    ),
    (
        "006_workspace_scope",
        include_str!("../migrations/006_workspace_scope.sql"),
    ),
    (
        "007_repository_publication",
        include_str!("../migrations/007_repository_publication.sql"),
    ),
    (
        "008_claims_and_time",
        include_str!("../migrations/008_claims_and_time.sql"),
    ),
    (
        "009_review_and_corrections",
        include_str!("../migrations/009_review_and_corrections.sql"),
    ),
    (
        "010_retention_and_erasure",
        include_str!("../migrations/010_retention_and_erasure.sql"),
    ),
    (
        "011_provider_learning",
        include_str!("../migrations/011_provider_learning.sql"),
    ),
    (
        "012_procedures_and_handovers",
        include_str!("../migrations/012_procedures_and_handovers.sql"),
    ),
    (
        "013_autonomous_memory",
        include_str!("../migrations/013_autonomous_memory.sql"),
    ),
    (
        "014_session_capture",
        include_str!("../migrations/014_session_capture.sql"),
    ),
    (
        "015_exact_lexical_retrieval",
        include_str!("../migrations/015_exact_lexical_retrieval.sql"),
    ),
    (
        "016_capture_reconciliation",
        include_str!("../migrations/016_capture_reconciliation.sql"),
    ),
    (
        "017_semantic_retrieval",
        include_str!("../migrations/017_semantic_retrieval.sql"),
    ),
    (
        "018_graph_projection",
        include_str!("../migrations/018_graph_projection.sql"),
    ),
    (
        "019_combined_graphs",
        include_str!("../migrations/019_combined_graphs.sql"),
    ),
    (
        "020_graph_analytics",
        include_str!("../migrations/020_graph_analytics.sql"),
    ),
    (
        "021_mcp_catalogue",
        include_str!("../migrations/021_mcp_catalogue.sql"),
    ),
    (
        "022_mcp_runtime",
        include_str!("../migrations/022_mcp_runtime.sql"),
    ),
    (
        "023_private_runners",
        include_str!("../migrations/023_private_runners.sql"),
    ),
    (
        "024_mcp_observations",
        include_str!("../migrations/024_mcp_observations.sql"),
    ),
    (
        "025_recovery_journal_order",
        include_str!("../migrations/025_recovery_journal_order.sql"),
    ),
    (
        "026_brain_admission",
        include_str!("../migrations/026_brain_admission.sql"),
    ),
    (
        "027_temporary_answers",
        include_str!("../migrations/027_temporary_answers.sql"),
    ),
    (
        "028_managed_memory_setup",
        include_str!("../migrations/028_managed_memory_setup.sql"),
    ),
    (
        "029_brain_deletion",
        include_str!("../migrations/029_brain_deletion.sql"),
    ),
    (
        "030_plugin_session_memory",
        include_str!("../migrations/030_plugin_session_memory.sql"),
    ),
    (
        "031_devices_host_kind",
        include_str!("../migrations/031_devices_host_kind.sql"),
    ),
    (
        "032_brain_icons",
        include_str!("../migrations/032_brain_icons.sql"),
    ),
    (
        "033_brain_agent_usage",
        include_str!("../migrations/033_brain_agent_usage.sql"),
    ),
    (
        "034_brain_model_selection",
        include_str!("../migrations/034_brain_model_selection.sql"),
    ),
];

pub fn compatible(applied: &[String], complete: bool) -> anyhow::Result<()> {
    anyhow::ensure!(
        applied.len() <= MIGRATIONS.len()
            && (!complete || applied.len() == MIGRATIONS.len())
            && applied.iter().zip(MIGRATIONS).all(|(a, (b, _))| a == b),
        "Database migration history is unknown, incomplete or newer than this executable"
    );
    Ok(())
}

pub async fn ready(pool: &PgPool) -> anyhow::Result<()> {
    let applied: Vec<String> =
        sqlx::query_scalar("SELECT name FROM recollect_migrations ORDER BY name")
            .fetch_all(pool)
            .await?;
    compatible(&applied, true)
}

pub async fn migrate(admin: &PgPool) -> anyhow::Result<()> {
    let mut tx = admin.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(73241001)")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS recollect_migrations (name text PRIMARY KEY, applied_at timestamptz NOT NULL DEFAULT now())")
        .execute(&mut *tx).await?;
    let applied: Vec<String> =
        sqlx::query_scalar("SELECT name FROM recollect_migrations ORDER BY name")
            .fetch_all(&mut *tx)
            .await?;
    compatible(&applied, false)?;
    for (name, sql) in &MIGRATIONS[applied.len()..] {
        sqlx::raw_sql(sql).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO recollect_migrations (name) VALUES ($1)")
            .bind(name)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("GRANT SELECT ON recollect_migrations TO recollect_app")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn bootstrap(admin: &PgPool, username: &str) -> anyhow::Result<Uuid> {
    let mut tx = admin.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(73241002)")
        .execute(&mut *tx)
        .await?;
    let owner: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, username FROM accounts WHERE installation_owner")
            .fetch_optional(&mut *tx)
            .await?;
    let id = if let Some((id, existing)) = owner {
        anyhow::ensure!(
            existing == username,
            "Configured owner username differs from the persisted owner; restore RECOLLECT_OWNER_USERNAME and use owner recovery"
        );
        id
    } else {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO accounts (id, username, installation_owner) VALUES ($1,$2,true)")
            .bind(id)
            .bind(username)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO mutation_audit (id,actor_id,action,target_id,disposition) VALUES ($1,$2,'owner.bootstrap',$2,'created')")
            .bind(Uuid::new_v4()).bind(id).execute(&mut *tx).await?;
        id
    };
    tx.commit().await?;
    Ok(id)
}

pub async fn recover_owner(admin: &PgPool) -> anyhow::Result<()> {
    let mut tx = admin.begin().await?;
    let id: Uuid =
        sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner FOR UPDATE")
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query("DELETE FROM sessions WHERE account_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE accounts SET enabled = true WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO mutation_audit (id,actor_id,action,target_id,disposition) VALUES ($1,$2,'owner.recover',$2,'sessions_revoked')")
        .bind(Uuid::new_v4()).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn actor_tx(pool: &PgPool, actor: Uuid) -> Result<Transaction<'_, Postgres>> {
    let mut tx = pool.begin().await?;
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM accounts WHERE id=$1 FOR SHARE")
            .bind(actor)
            .fetch_optional(&mut *tx)
            .await?;
    if enabled != Some(true) {
        return Err(crate::error::Error::unauthorized());
    }
    sqlx::query("SELECT set_config('recollect.actor',$1,true)")
        .bind(actor.to_string())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

pub async fn device_tx(
    pool: &PgPool,
    actor: Uuid,
    device: Option<Uuid>,
) -> Result<Transaction<'_, Postgres>> {
    let mut tx = actor_tx(pool, actor).await?;
    if let Some(device) = device {
        let valid:Option<bool>=sqlx::query_scalar("SELECT d.claimed AND d.revoked_at IS NULL AND d.expires_at>now() AND (a.auth_kind='local' OR a.membership_until>now()) FROM devices d JOIN accounts a ON a.id=d.account_id WHERE d.id=$1 AND d.account_id=$2 FOR SHARE OF d")
            .bind(device).bind(actor).fetch_optional(&mut *tx).await?;
        if valid != Some(true) {
            return Err(crate::error::Error::unauthorized());
        }
        sqlx::query("SELECT set_config('recollect.device',$1,true)")
            .bind(device.to_string())
            .execute(&mut *tx)
            .await?;
    }
    Ok(tx)
}

pub async fn require_role(
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    admin: bool,
) -> Result<String> {
    lock_brain(tx, brain, admin).await?;
    let role: Option<String> = sqlx::query_scalar("SELECT recollect_role($1)")
        .bind(brain)
        .fetch_one(&mut **tx)
        .await?;
    match role {
        None => Err(crate::error::Error::missing()),
        Some(role) if admin && role != "admin" => Err(crate::error::Error::forbidden()),
        Some(role) => Ok(role),
    }
}

pub async fn lock_brain(
    tx: &mut Transaction<'_, Postgres>,
    brain: Uuid,
    exclusive: bool,
) -> Result<()> {
    let locked: bool = sqlx::query_scalar("SELECT recollect_lock_brain($1,$2)")
        .bind(brain)
        .bind(exclusive)
        .fetch_one(&mut **tx)
        .await?;
    if !locked {
        return Err(crate::error::Error::missing());
    }
    Ok(())
}

pub async fn require_writer(tx: &mut Transaction<'_, Postgres>, brain: Uuid) -> Result<()> {
    lock_brain(tx, brain, true).await?;
    let archived: Option<bool> = sqlx::query_scalar("SELECT archived FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_optional(&mut **tx)
        .await?;
    let archived = archived.ok_or_else(crate::error::Error::missing)?;
    let role = require_role(tx, brain, false).await?;
    if !matches!(role.as_str(), "writer" | "admin") {
        return Err(crate::error::Error::forbidden());
    }
    if archived {
        return Err(crate::error::Error(
            axum::http::StatusCode::CONFLICT,
            "brain_archived",
            "Reopen this Brain before changing its evidence.",
        ));
    }
    Ok(())
}

pub async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    brain: Uuid,
    action: &str,
    target: Uuid,
    disposition: &str,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO mutation_audit (id,brain_id,actor_id,action,target_id,disposition) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(id).bind(brain).bind(actor).bind(action).bind(target).bind(disposition)
        .execute(&mut **tx).await?;
    Ok(id)
}
