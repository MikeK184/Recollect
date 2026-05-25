use super::*;
use crate::{config::Config, privacy_journal};
use anyhow::{Context, ensure};
use sqlx::PgPool;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    pub installation_id: Uuid,
    pub id: Uuid,
    pub brain_id: Uuid,
    pub report_id: Uuid,
    pub job_id: Uuid,
    pub lease_token: Uuid,
    pub privacy_sequence: i64,
    pub created_at: DateTime<Utc>,
    pub deadline: DateTime<Utc>,
    pub closed: bool,
}
impl Entry {
    pub fn graph(&self) -> String {
        format!(
            "recollect_analytics_{}_{}",
            self.installation_id.simple(),
            self.id.simple()
        )
    }
    pub fn fence(&self) -> String {
        format!("analytics:{}", self.id)
    }
    pub fn metadata(&self) -> Value {
        json!({"recollect_installation":self.installation_id,"recollect_analytics_attempt":self.id})
    }
    fn path(&self, config: &Config) -> PathBuf {
        directory(config, self.installation_id).join(format!("{}.json", self.id))
    }
}
fn directory(config: &Config, installation: Uuid) -> PathBuf {
    Path::new(&config.erasure_journal).join(format!("analytics-{installation}"))
}
async fn identity(pool: &PgPool) -> anyhow::Result<Uuid> {
    let (id, initialized): (Uuid, bool) =
        sqlx::query_as("SELECT id,initialized AND analytics_initialized FROM privacy_installation")
            .fetch_one(pool)
            .await?;
    ensure!(
        initialized,
        "Initialize the privacy journal before graph analytics"
    );
    Ok(id)
}
async fn marker(config: &Config, id: Uuid) -> anyhow::Result<()> {
    let path = directory(config, id).join("installation.json");
    let saved: Uuid = serde_json::from_slice(
        &tokio::fs::read(path)
            .await
            .context("Restore the analytical ownership journal before serving")?,
    )?;
    ensure!(
        saved == id,
        "Analytical ownership journal belongs to another installation"
    );
    Ok(())
}
pub async fn initialize(pool: &PgPool, config: &Config) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    let (id, initialized): (Uuid, bool) =
        sqlx::query_as("SELECT id,analytics_initialized FROM privacy_installation FOR UPDATE")
            .fetch_one(&mut *tx)
            .await?;
    if initialized {
        marker(config, id).await?;
    } else {
        let dir = directory(config, id);
        tokio::fs::create_dir_all(&dir).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).await?;
        }
        let path = dir.join("installation.json");
        if tokio::fs::try_exists(&path).await? {
            marker(config, id).await?;
        } else {
            privacy_journal::durable_write(&path, &serde_json::to_vec(&id)?).await?;
        }
        #[cfg(unix)]
        tokio::fs::File::open(&config.erasure_journal)
            .await?
            .sync_all()
            .await?;
        sqlx::query("UPDATE privacy_installation SET analytics_initialized=true")
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
async fn entries(config: &Config, pool: &PgPool) -> anyhow::Result<Vec<Entry>> {
    let installation = identity(pool).await?;
    marker(config, installation).await?;
    let dir = directory(config, installation);
    let mut dir = tokio::fs::read_dir(dir).await?;
    let mut result = vec![];
    while let Some(file) = dir.next_entry().await? {
        let name = file.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".tmp") || name == "installation.json" {
            continue;
        }
        ensure!(
            result.len() < 1000 && file.file_type().await?.is_file(),
            "Analytics journal is outside its bounded envelope"
        );
        let id: Uuid = name
            .strip_suffix(".json")
            .context("Unexpected analytics journal entry")?
            .parse()?;
        ensure!(
            file.metadata().await?.len() <= 8192,
            "Oversized analytics journal entry"
        );
        let entry: Entry = serde_json::from_slice(&tokio::fs::read(file.path()).await?)?;
        ensure!(
            entry.installation_id == installation
                && entry.id == id
                && entry.privacy_sequence >= 0
                && entry.deadline > entry.created_at
                && entry.deadline - entry.created_at <= chrono::Duration::seconds(120),
            "Invalid analytics journal ownership"
        );
        result.push(entry);
    }
    let expected: Vec<Json<Value>> =
        sqlx::query_scalar("SELECT to_jsonb(a) FROM recollect_analytics_expected_entries() a")
            .fetch_all(pool)
            .await?;
    ensure!(
        expected.len() <= 1000,
        "Too many retained analytical ownership records"
    );
    for Json(mut a) in expected {
        let id: Uuid = serde_json::from_value(a["id"].clone())?;
        if let Some(found) = result.iter().find(|e| e.id == id) {
            ensure!(
                a["brain_id"] == json!(found.brain_id)
                    && a["report_id"] == json!(found.report_id)
                    && a["job_id"] == json!(found.job_id)
                    && a["lease_token"] == json!(found.lease_token)
                    && a["privacy_sequence"] == json!(found.privacy_sequence)
                    && serde_json::from_value::<DateTime<Utc>>(a["created_at"].clone())?
                        == found.created_at
                    && serde_json::from_value::<DateTime<Utc>>(a["deadline"].clone())?
                        == found.deadline,
                "Analytical ownership journal disagrees with its canonical attempt"
            );
        } else {
            let deadline: DateTime<Utc> = serde_json::from_value(a["deadline"].clone())?;
            // A restore can put an old unclosed attempt back after its durable
            // ledger was retired. Its expired native creation deadline makes
            // exact reconstruction safe; unexpired missing ownership is an error.
            ensure!(
                deadline <= Utc::now(),
                "An unexpired analytical attempt is missing from its retained journal"
            );
            a.as_object_mut().unwrap().remove("cleaned_at");
            a["installation_id"] = json!(installation);
            a["closed"] = json!(false);
            let restored: Entry = serde_json::from_value(a)?;
            privacy_journal::durable_write(&restored.path(config), &serde_json::to_vec(&restored)?)
                .await?;
            result.push(restored);
        }
    }
    result.sort_by_key(|e| (e.deadline, e.id));
    Ok(result)
}
pub(super) async fn arm(state: &AppState, tx: &mut Tx<'_>, job: &ClaimedJob) -> Result<Entry> {
    let installation: Uuid = sqlx::query_scalar(
        "SELECT id FROM privacy_installation WHERE initialized AND analytics_initialized",
    )
    .fetch_one(&mut **tx)
    .await?;
    let created_at: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    let e = Entry {
        installation_id: installation,
        id: Uuid::new_v4(),
        brain_id: job.brain_id,
        report_id: job.target_id,
        job_id: job.id,
        lease_token: job.lease_token,
        privacy_sequence: privacy_sequence(tx, job.brain_id).await?,
        created_at,
        deadline: created_at + chrono::Duration::seconds(120),
        closed: false,
    };
    let write = async {
        marker(&state.config, installation).await?;
        privacy_journal::durable_write(&e.path(&state.config), &serde_json::to_vec(&e)?).await?;
        privacy_journal::mirror::publish_file(
            &state.config,
            installation,
            &format!("analytics/{}.json.age", e.id),
            &serde_json::to_vec(&e)?,
        )
        .await?;
        #[cfg(unix)]
        tokio::fs::File::open(&state.config.erasure_journal)
            .await?
            .sync_all()
            .await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    write.map_err(|_| {
        failure(
            "analytics_journal_unavailable",
            "The durable analytical ownership journal could not be recorded.",
        )
    })?;
    sqlx::query("INSERT INTO analytics_attempts(id,brain_id,report_id,job_id,lease_token,created_at,deadline,privacy_sequence) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
      .bind(e.id).bind(e.brain_id).bind(e.report_id).bind(e.job_id).bind(e.lease_token).bind(e.created_at).bind(e.deadline).bind(e.privacy_sequence).execute(&mut **tx).await?;
    Ok(e)
}
pub(super) async fn close(
    config: &Config,
    pool: &PgPool,
    http: &reqwest::Client,
    entry: &Entry,
) -> Result<()> {
    native::cleanup(config, http, entry).await?;
    let mut closed = entry.clone();
    closed.closed = true;
    privacy_journal::durable_write(
        &closed.path(config),
        &serde_json::to_vec(&closed).map_err(|_| changed())?,
    )
    .await
    .map_err(|_| {
        failure(
            "analytics_journal_unavailable",
            "Closed analysis ownership could not be persisted. Cleanup will retry.",
        )
    })?;
    privacy_journal::mirror::publish_file(
        config,
        closed.installation_id,
        &format!("analytics/{}.json.age", closed.id),
        &serde_json::to_vec(&closed).map_err(|_| changed())?,
    )
    .await
    .map_err(|_| {
        failure(
            "analytics_journal_unavailable",
            "Closed analysis ownership mirror remains pending.",
        )
    })?;
    sqlx::query("SELECT recollect_analytics_cleaned($1,$2)")
        .bind(entry.id)
        .bind(entry.lease_token)
        .execute(pool)
        .await?;
    if entry.deadline <= Utc::now() {
        privacy_journal::mirror::retire_analytics(config, entry.installation_id, entry.id)
            .await
            .map_err(|_| {
                failure(
                    "analytics_journal_unavailable",
                    "Closed analytical mirror could not be retired.",
                )
            })?;
        match tokio::fs::remove_file(entry.path(config)).await {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(failure(
                    "analytics_journal_unavailable",
                    "Closed analytics ownership could not be retired.",
                ));
            }
        }
        #[cfg(unix)]
        tokio::fs::File::open(directory(config, entry.installation_id))
            .await
            .map_err(|_| changed())?
            .sync_all()
            .await
            .map_err(|_| changed())?;
    }
    Ok(())
}
async fn cleanup_entries(
    config: &Config,
    pool: &PgPool,
    force: bool,
    privacy: Option<(Uuid, i64)>,
) -> anyhow::Result<usize> {
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let mut cleaned = 0;
    let entries = entries(config, pool).await?;
    let installation = identity(pool).await?;
    let native=native::control(config,&http,"CALL gds.graph.list() YIELD graphName WHERE graphName STARTS WITH $prefix RETURN graphName",json!({"prefix":format!("recollect_analytics_{}_",installation.simple())}),&["graphName"]).await.map_err(|_|anyhow::anyhow!("Analytical catalog ownership could not be checked"))?;
    ensure!(
        native
            .iter()
            .all(|r| entries.iter().any(|e| r[0] == json!(e.graph()))),
        "A native analytical graph has no retained ownership entry; restore the journal before cleanup or new allocation"
    );
    for e in entries {
        if let Some((brain, sequence)) = privacy {
            if e.brain_id != brain || e.privacy_sequence >= sequence {
                continue;
            }
        } else {
            let live: bool = sqlx::query_scalar("SELECT recollect_analytics_attempt_live($1,$2)")
                .bind(e.job_id)
                .bind(e.lease_token)
                .fetch_one(pool)
                .await?;
            if live {
                continue;
            }
            if e.closed
                && !force
                && e.deadline > Utc::now()
                && !native.iter().any(|r| r[0] == json!(e.graph()))
            {
                continue;
            }
        }
        close(config, pool, &http, &e).await.map_err(|error| {
            anyhow::anyhow!(
                "Analytical scratch cleanup is pending: {} ({})",
                error.1,
                error.2
            )
        })?;
        cleaned += 1;
    }
    Ok(cleaned)
}
pub async fn reconcile(config: &Config, pool: &PgPool) -> anyhow::Result<usize> {
    tokio::time::timeout(
        Duration::from_secs(30),
        cleanup_entries(config, pool, true, None),
    )
    .await
    .context("Analytics cleanup deadline exceeded")?
}
pub async fn erase(
    config: &Config,
    pool: &PgPool,
    brain: Uuid,
    sequence: i64,
    request: Uuid,
) -> anyhow::Result<()> {
    sqlx::query("SELECT recollect_analytics_remove_request($1)")
        .bind(request)
        .execute(pool)
        .await?;
    tokio::time::timeout(
        Duration::from_secs(30),
        cleanup_entries(config, pool, true, Some((brain, sequence))),
    )
    .await
    .context("Analytics erasure cleanup deadline exceeded")??;
    Ok(())
}
pub async fn run_once(state: &AppState) -> anyhow::Result<usize> {
    sqlx::query("SELECT recollect_analytics_maintain()")
        .execute(&state.pool)
        .await?;
    tokio::time::timeout(
        Duration::from_secs(20),
        cleanup_entries(&state.config, &state.pool, false, None),
    )
    .await
    .context("Analytics maintenance deadline exceeded")?
}
