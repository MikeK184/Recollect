use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

pub const MAX_TEXT: usize = 1024 * 1024;

#[derive(Debug, PartialEq)]
pub enum ReadFailure {
    Missing,
    Unavailable,
    Invalid,
}

pub fn path(root: &str, brain: Uuid, artifact: Uuid) -> PathBuf {
    Path::new(root)
        .join(brain.to_string())
        .join(format!("{artifact}.txt"))
}
pub async fn write(root: &str, brain: Uuid, artifact: Uuid, content: &str) -> std::io::Result<()> {
    let path = path(root, brain, artifact);
    let parent = path.parent().expect("artifact path always has a parent");
    tokio::fs::create_dir_all(parent).await?;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&path).await?;
    file.write_all(content.as_bytes()).await?;
    file.flush().await?;
    file.sync_all().await?;
    #[cfg(unix)]
    tokio::fs::File::open(parent).await?.sync_all().await?;
    Ok(())
}
pub async fn read(
    root: &str,
    brain: Uuid,
    artifact: Uuid,
    expected: i32,
) -> Result<String, ReadFailure> {
    read_bounded(root, brain, artifact, expected, MAX_TEXT).await
}
pub async fn read_bounded(
    root: &str,
    brain: Uuid,
    artifact: Uuid,
    expected: i32,
    limit: usize,
) -> Result<String, ReadFailure> {
    let file = tokio::fs::File::open(path(root, brain, artifact))
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ReadFailure::Missing
            } else {
                ReadFailure::Unavailable
            }
        })?;
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| ReadFailure::Unavailable)?;
    if bytes.len() > limit || bytes.len() != expected as usize {
        return Err(ReadFailure::Invalid);
    }
    String::from_utf8(bytes).map_err(|_| ReadFailure::Invalid)
}
pub async fn availability(
    root: &str,
    brain: Uuid,
    artifact: Option<Uuid>,
    expected: i32,
) -> &'static str {
    let Some(id) = artifact else {
        return "reference_only";
    };
    match tokio::fs::metadata(path(root, brain, id)).await {
        Ok(meta) if meta.is_file() && meta.len() == expected as u64 => "retained",
        Ok(_) => "unreadable",
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "missing",
        Err(_) => "unavailable",
    }
}
