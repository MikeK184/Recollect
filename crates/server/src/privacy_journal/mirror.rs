//! Optional encrypted journal replication. Canonical privacy decisions and
//! acknowledgement still belong to the existing journal writer.
use crate::config::Config;
use anyhow::{Context, ensure};
use serde::Deserialize;
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mirror {
    host: String,
    port: u16,
    user: String,
    remote_root: String,
    identity_file: PathBuf,
    known_hosts_file: PathBuf,
    recipients: Vec<String>,
    age_binary: PathBuf,
    sftp_binary: PathBuf,
}
impl Mirror {
    async fn load(path: &str) -> anyhow::Result<Self> {
        let bytes = tokio::fs::read(path)
            .await
            .context("Erasure mirror configuration is unavailable")?;
        ensure!(
            bytes.len() <= 16_384,
            "Erasure mirror configuration is too large"
        );
        let value: Self = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("Erasure mirror configuration is invalid"))?;
        let name = |s: &str, max: usize| {
            !s.is_empty()
                && s.len() <= max
                && s.as_bytes()[0].is_ascii_alphanumeric()
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        };
        ensure!(
            name(&value.host, 253) && name(&value.user, 64) && value.port != 0,
            "Erasure mirror host identity is invalid"
        );
        ensure!(
            value.remote_root.starts_with('/')
                && value.remote_root.len() <= 512
                && value
                    .remote_root
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"/._-".contains(&c))
                && value
                    .remote_root
                    .split('/')
                    .all(|s| !matches!(s, "." | "..")),
            "Erasure mirror requires a fixed absolute remote root"
        );
        ensure!(
            (1..=8).contains(&value.recipients.len())
                && value.recipients.iter().all(|r| r.starts_with("age1")
                    && r.len() == 62
                    && r.bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())),
            "Erasure mirror requires explicit X25519 age recipients"
        );
        for path in [
            &value.identity_file,
            &value.known_hosts_file,
            &value.age_binary,
            &value.sftp_binary,
        ] {
            ensure!(
                path.is_absolute() && tokio::fs::metadata(path).await.is_ok_and(|m| m.is_file()),
                "Erasure mirror tool or credential reference is unavailable"
            );
        }
        Ok(value)
    }

    fn directory(&self, installation: Uuid) -> String {
        format!(
            "{}/{}",
            self.remote_root.trim_end_matches('/'),
            installation
        )
    }

    async fn sftp(&self, temporary: &Upload, batch: &str) -> anyhow::Result<()> {
        let batch_file = temporary.0.join("commands.sftp");
        tokio::fs::write(&batch_file, batch).await?;
        let mut command = Command::new(&self.sftp_binary);
        command
            .args(["-q", "-F", "/dev/null", "-b"])
            .arg(&batch_file)
            .args(["-P", &self.port.to_string(), "-i"])
            .arg(&self.identity_file)
            .arg("-o")
            .arg(format!(
                "UserKnownHostsFile={}",
                quote(&self.known_hosts_file)?
            ))
            .args([
                "-o",
                "GlobalKnownHostsFile=/dev/null",
                "-o",
                "StrictHostKeyChecking=yes",
                "-o",
                "BatchMode=yes",
                "-o",
                "IdentitiesOnly=yes",
                "-o",
                "IdentityAgent=none",
                "-o",
                "ForwardAgent=no",
                "-o",
                "ClearAllForwardings=yes",
                "-o",
                "ProxyCommand=none",
                "-o",
                "ProxyJump=none",
                "-o",
                "ConnectTimeout=5",
                "-o",
                "ServerAliveInterval=5",
                "-o",
                "ServerAliveCountMax=2",
            ])
            .arg(format!("{}@{}", self.user, self.host));
        bounded(command).await
    }

    async fn check(&self, temporary: &Upload, installation: Uuid) -> anyhow::Result<()> {
        let owner_file = temporary.0.join("owner.json");
        self.sftp(
            temporary,
            &format!(
                "get {}/owner.json {}\n",
                self.directory(installation),
                quote(&owner_file)?
            ),
        )
        .await?;
        let bytes = tokio::fs::read(owner_file).await?;
        ensure!(
            bytes.len() <= 1024,
            "Erasure mirror ownership marker is invalid"
        );
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Owner {
            installation_id: Uuid,
            kind: String,
        }
        let owner: Owner = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("Erasure mirror ownership marker is invalid"))?;
        ensure!(
            owner.installation_id == installation && owner.kind == "recollect-recovery",
            "Erasure mirror belongs to another installation"
        );
        Ok(())
    }
}

/// This directory is created by this invocation, below the configured local
/// journal root, and never contains a decryption identity or copied credential.
struct Upload(PathBuf);
impl Upload {
    async fn new(config: &Config) -> anyhow::Result<Self> {
        let parent = PathBuf::from(&config.erasure_journal).join(".mirror-uploads");
        let mut builder = tokio::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        builder.mode(0o700);
        builder.create(&parent).await?;
        ensure!(
            !tokio::fs::symlink_metadata(&parent)
                .await?
                .file_type()
                .is_symlink(),
            "Erasure mirror staging cannot be a symbolic link"
        );
        let path = parent.join(Uuid::new_v4().to_string());
        let mut builder = tokio::fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
        builder.create(&path).await?;
        Ok(Self(path))
    }
}
impl Drop for Upload {
    fn drop(&mut self) {
        // No foreign path or persisted PID is ever used for cleanup.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn quote(path: &std::path::Path) -> anyhow::Result<String> {
    let value = path.to_str().context("Erasure mirror path must be UTF-8")?;
    ensure!(
        !value.chars().any(char::is_control),
        "Erasure mirror path contains a control character"
    );
    Ok(format!(
        "\"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

async fn bounded(mut command: Command) -> anyhow::Result<()> {
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/local/bin")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let result = tokio::time::timeout(Duration::from_secs(30), command.status())
        .await
        .map_err(|_| anyhow::anyhow!("Erasure mirror operation exceeded its deadline"))?
        .map_err(|_| anyhow::anyhow!("Erasure mirror tool could not run"))?;
    ensure!(
        result.success(),
        "Erasure mirror operation did not complete"
    );
    Ok(())
}

pub(super) async fn publish(
    pool: &sqlx::PgPool,
    config: &Config,
    entry: &super::Entry,
) -> anyhow::Result<()> {
    if config.erasure_mirror.is_none() {
        return Ok(());
    }
    let previous: Option<(Uuid, i64)> = sqlx::query_as("SELECT id,sequence FROM recollect_privacy_positions() WHERE sequence<$1 ORDER BY sequence DESC LIMIT 1")
        .bind(entry.sequence).fetch_optional(pool).await?;
    let body = serde_json::to_vec(&serde_json::json!({"entry":entry,
        "previous":previous.map(|(id,sequence)|serde_json::json!({"id":id,"sequence":sequence}))}))?;
    publish_file(
        config,
        entry.installation_id,
        &format!("journal/{:020}-{}.json.age", entry.sequence, entry.id),
        &body,
    )
    .await
}

pub(crate) async fn publish_file(
    config: &Config,
    installation: Uuid,
    name: &str,
    body: &[u8],
) -> anyhow::Result<()> {
    let Some(path) = &config.erasure_mirror else {
        return Ok(());
    };
    let mirror = Mirror::load(path).await?;
    let temporary = Upload::new(config).await?;
    mirror.check(&temporary, installation).await?;
    let encrypted = temporary.0.join("entry.age");
    let original = temporary.0.join("entry.json");
    tokio::fs::write(&original, body).await?;
    let mut encrypt = Command::new(&mirror.age_binary);
    encrypt.args(["--encrypt", "--output"]).arg(&encrypted);
    for recipient in &mirror.recipients {
        encrypt.arg("--recipient").arg(recipient);
    }
    encrypt.arg(original);
    bounded(encrypt).await?;
    let destination = format!("{}/{}", mirror.directory(installation), name);
    let partial = format!("{destination}.pending-{}", Uuid::new_v4());
    // OpenSSH's flush requests remote fsync before publishing the complete name.
    // Existing entries have stable canonical request/sequence identity; a retry
    // replaces only that same encrypted entry, never another privacy request.
    mirror
        .sftp(
            &temporary,
            &format!(
                "put -f {} {partial}\nrename {partial} {destination}\n",
                quote(&encrypted)?
            ),
        )
        .await
}

pub(super) async fn check(config: &Config, installation: Uuid) -> anyhow::Result<()> {
    let Some(path) = &config.erasure_mirror else {
        return Ok(());
    };
    let mirror = Mirror::load(path).await?;
    let temporary = Upload::new(config).await?;
    mirror.check(&temporary, installation).await
}

pub(crate) async fn retire_analytics(
    config: &Config,
    installation: Uuid,
    id: Uuid,
) -> anyhow::Result<()> {
    let Some(path) = &config.erasure_mirror else {
        return Ok(());
    };
    let mirror = Mirror::load(path).await?;
    let temporary = Upload::new(config).await?;
    mirror.check(&temporary, installation).await?;
    mirror
        .sftp(
            &temporary,
            &format!(
                "rm {}/analytics/{}.json.age\n",
                mirror.directory(installation),
                id
            ),
        )
        .await
}
