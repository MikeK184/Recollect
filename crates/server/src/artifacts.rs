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
/// Verify the stored artifact has its recorded length, then read only the byte
/// range `[from,to)`. Missing/invalid/unavailable map as in `read_bounded`; a
/// span outside the file is invalid. The caller compares the returned bytes
/// with the canonical chunk text instead of loading the whole artifact.
pub async fn read_span(
    root: &str,
    brain: Uuid,
    artifact: Uuid,
    expected: i32,
    from: usize,
    to: usize,
) -> Result<Vec<u8>, ReadFailure> {
    use tokio::io::AsyncSeekExt;
    let mut file = tokio::fs::File::open(path(root, brain, artifact))
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ReadFailure::Missing
            } else {
                ReadFailure::Unavailable
            }
        })?;
    let metadata = file
        .metadata()
        .await
        .map_err(|_| ReadFailure::Unavailable)?;
    if !metadata.is_file() {
        return Err(ReadFailure::Unavailable);
    }
    let length = metadata.len();
    if expected < 0
        || length != expected as u64
        || length as usize > MAX_TEXT
        || from > to
        || to as u64 > length
    {
        return Err(ReadFailure::Invalid);
    }
    file.seek(std::io::SeekFrom::Start(from as u64))
        .await
        .map_err(|_| ReadFailure::Unavailable)?;
    let mut bytes = vec![0; to - from];
    file.read_exact(&mut bytes)
        .await
        .map_err(|_| ReadFailure::Unavailable)?;
    Ok(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn span_reads_verify_length_and_return_only_the_requested_bytes() {
        let root = std::env::temp_dir().join(format!("recollect-span-{}", Uuid::new_v4()));
        let root_str = root.to_string_lossy().into_owned();
        let (brain, artifact) = (Uuid::new_v4(), Uuid::new_v4());
        let text = "alpha beta\ngamma";
        write(&root_str, brain, artifact, text).await.unwrap();
        let length = text.len() as i32;
        assert_eq!(
            read_span(&root_str, brain, artifact, length, 6, 10).await,
            Ok(b"beta".to_vec())
        );
        assert_eq!(
            read_span(&root_str, brain, artifact, length, 0, 0).await,
            Ok(Vec::new())
        );
        // A recorded length that no longer matches, or a span beyond the file.
        assert_eq!(
            read_span(&root_str, brain, artifact, length + 1, 0, 4).await,
            Err(ReadFailure::Invalid)
        );
        assert_eq!(
            read_span(&root_str, brain, artifact, length, 10, 99).await,
            Err(ReadFailure::Invalid)
        );
        assert_eq!(
            read_span(&root_str, brain, artifact, length, 5, 2).await,
            Err(ReadFailure::Invalid)
        );
        assert_eq!(
            read_span(&root_str, brain, Uuid::new_v4(), length, 0, 1).await,
            Err(ReadFailure::Missing)
        );
        // Changed bytes of the same length are returned for the caller to compare.
        tokio::fs::write(path(&root_str, brain, artifact), vec![0xff; text.len()])
            .await
            .unwrap();
        assert_eq!(
            read_span(&root_str, brain, artifact, length, 6, 10).await,
            Ok(vec![0xff; 4])
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
