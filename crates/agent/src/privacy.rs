use crate::{Client, StoredDevice, decode, publication};
use anyhow::{Result, anyhow, ensure};
use chrono::{DateTime, Utc};
use recollect_protocol::{
    PUBLICATION_MAX_BYTES, PrivacyDeviceAck, PrivacyDeviceReceipt, PrivacyDeviceSync, git_object_id,
};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CachedPrivacy {
    pub endpoint: String,
    pub device_id: Uuid,
    pub sync: PrivacyDeviceSync,
    pub checked_at: DateTime<Utc>,
}
#[derive(Debug, Serialize)]
pub struct Cleanup {
    pub brain_id: Uuid,
    pub sequence: i64,
    pub last_synchronized_at: DateTime<Utc>,
    pub removed: usize,
    pub retained: usize,
    pub acknowledged: bool,
}
/// One applied Brain-deletion fence, reported separately from central
/// completion so offline copies stay visibly incomplete.
#[derive(Debug, Serialize)]
pub struct DeletionFence {
    pub brain_id: Uuid,
    pub deletion_id: Uuid,
    pub sequence: i64,
    pub removed_bundles: usize,
    pub removed_events: usize,
}
/// The server supplies the fence identity only. Cleanup counts belong to the
/// local application of that fence and cannot be required from the wire.
#[derive(Deserialize)]
struct RemoteDeletionFence {
    brain_id: Uuid,
    deletion_id: Uuid,
    sequence: i64,
}
#[derive(Deserialize)]
struct BundleHeader {
    endpoint: String,
    device_id: Uuid,
    brain_id: Uuid,
    repository_id: Uuid,
    input: InputHeader,
}
#[derive(Deserialize)]
struct InputHeader {
    publication_id: Uuid,
    revision: String,
    captured_at: DateTime<Utc>,
}
async fn cache_path(root: &Path, device: Uuid, brain: Uuid) -> Result<PathBuf> {
    let dir = publication::private_root(&root.join(".privacy")).await?;
    Ok(dir.join(format!("{device}-{brain}.json")))
}
pub async fn cached(
    root: &Path,
    endpoint: &str,
    device: Uuid,
    brain: Uuid,
) -> Result<Option<CachedPrivacy>> {
    let path = cache_path(root, device, brain).await?;
    if !fs::try_exists(&path).await? {
        return Ok(None);
    }
    let saved: CachedPrivacy =
        serde_json::from_slice(&publication::artifact(&path, 8 * 1024 * 1024).await?).map_err(
            |_| anyhow!("Saved retention policy is unreadable; synchronize it before resuming."),
        )?;
    ensure!(
        saved.endpoint == endpoint && saved.device_id == device && saved.sync.brain_id == brain,
        "Saved retention policy belongs to another profile."
    );
    Ok(Some(saved))
}
pub async fn save(root: &Path, saved: &CachedPrivacy) -> Result<()> {
    let path = cache_path(root, saved.device_id, saved.sync.brain_id).await?;
    let tmp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    publication::write_private(&tmp, &serde_json::to_vec(saved)?).await?;
    fs::rename(tmp, &path).await?;
    fs::File::open(path.parent().expect("cache directory"))
        .await?
        .sync_all()
        .await?;
    Ok(())
}
/// Read only the identity header; privacy cleanup must also remove old content
/// that a newer sanitizer would refuse to deserialize as a publication.
pub async fn cleanup(root: &Path, saved: &CachedPrivacy) -> Result<Cleanup> {
    let root = publication::private_root(root).await?;
    let mut files = fs::read_dir(&root).await?;
    let mut report = Cleanup {
        brain_id: saved.sync.brain_id,
        sequence: saved.sync.sequence,
        last_synchronized_at: saved.checked_at,
        removed: 0,
        retained: 0,
        acknowledged: false,
    };
    while let Some(file) = files.next_entry().await? {
        let name = file.file_name();
        let name = name.to_string_lossy();
        let Some(id) = name
            .strip_suffix(".json")
            .and_then(|s| s.parse::<Uuid>().ok())
        else {
            continue;
        };
        let bytes = publication::artifact(&file.path(), PUBLICATION_MAX_BYTES + 8192).await?;
        let header:BundleHeader=serde_json::from_slice(&bytes).map_err(|_|anyhow!("A prepared bundle has unreadable identity metadata; cleanup remains unacknowledged."))?;
        ensure!(
            header.input.publication_id == id && git_object_id(&header.input.revision),
            "Prepared bundle identity is invalid; cleanup remains unacknowledged."
        );
        if header.endpoint != saved.endpoint
            || header.device_id != saved.device_id
            || header.brain_id != saved.sync.brain_id
        {
            continue;
        }
        let fenced = saved.sync.publication_fences.iter().any(|f| {
            f.repository_id == header.repository_id && f.revision == header.input.revision
        });
        let expired = saved.sync.policy.repository_days.is_some_and(|days| {
            header.input.captured_at + chrono::Duration::days(i64::from(days)) <= Utc::now()
        });
        if fenced || expired {
            fs::remove_file(file.path()).await?;
            report.removed += 1;
        } else {
            report.retained += 1;
        }
    }
    fs::File::open(&root).await?.sync_all().await?;
    let capture_root = crate::capture::profile_root(&root, saved.device_id);
    if fs::try_exists(capture_root.join("inbox.sqlite")).await? {
        let mut inbox =
            crate::capture::Inbox::open(&capture_root, &saved.endpoint, saved.device_id)?;
        report.removed += inbox.apply_privacy(&saved.sync, Utc::now())?;
        report.retained += inbox.status()?.pending as usize;
    }
    Ok(report)
}
pub async fn cleanup_known(
    client: &Client,
    device: &StoredDevice,
    brain: Uuid,
    root: &Path,
) -> Result<Option<Cleanup>> {
    if let Some(saved) = cached(root, &client.endpoint, device.device_id, brain).await? {
        cleanup(root, &saved).await.map(Some)
    } else {
        Ok(None)
    }
}
/// Fetch the deletion fence for one Brain on the next check-in. Absence of a
/// fence (live Brain, or no visibility) is a normal 404, not an error.
pub async fn deletion_fence(
    client: &Client,
    device: &StoredDevice,
    brain: Uuid,
) -> Result<Option<DeletionFence>> {
    let response = client
        .send(
            Method::GET,
            &format!("/api/brains/{brain}/deletions/fence"),
            Some(device.token),
            None,
        )
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let fence: RemoteDeletionFence = decode(response).await?;
    ensure!(
        fence.brain_id == brain,
        "Deletion fence belongs to another Brain."
    );
    Ok(Some(DeletionFence {
        brain_id: fence.brain_id,
        deletion_id: fence.deletion_id,
        sequence: fence.sequence,
        removed_bundles: 0,
        removed_events: 0,
    }))
}
/// Remove only product-owned local files for a deleted Brain: the cached
/// privacy position and prepared publication bundles. Git objects and customer
/// working trees are never touched; unreadable bundle identity is an error so
/// incomplete cleanup stays visible.
pub async fn apply_deletion(
    root: &Path,
    endpoint: &str,
    device: Uuid,
    brain: Uuid,
) -> Result<usize> {
    let cache = cache_path(root, device, brain).await?;
    if fs::try_exists(&cache).await? {
        fs::remove_file(cache).await?;
    }
    let root = publication::private_root(root).await?;
    let mut files = fs::read_dir(&root).await?;
    let mut removed = 0;
    while let Some(file) = files.next_entry().await? {
        let name = file.file_name();
        let name = name.to_string_lossy();
        let Some(id) = name
            .strip_suffix(".json")
            .and_then(|s| s.parse::<Uuid>().ok())
        else {
            continue;
        };
        let bytes = publication::artifact(&file.path(), PUBLICATION_MAX_BYTES + 8192).await?;
        let header: BundleHeader = serde_json::from_slice(&bytes).map_err(|_| {
            anyhow!(
                "A prepared bundle has unreadable identity metadata; deletion cleanup remains incomplete."
            )
        })?;
        ensure!(
            header.input.publication_id == id && git_object_id(&header.input.revision),
            "Prepared bundle identity is invalid; deletion cleanup remains incomplete."
        );
        if header.endpoint != endpoint || header.device_id != device || header.brain_id != brain {
            continue;
        }
        fs::remove_file(file.path()).await?;
        removed += 1;
    }
    fs::File::open(&root).await?.sync_all().await?;
    Ok(removed)
}
pub async fn synchronize(
    client: &Client,
    device: &StoredDevice,
    brain: Uuid,
    root: &Path,
) -> Result<Cleanup> {
    cleanup_known(client, device, brain, root).await?;
    let path = format!("/api/brains/{brain}/privacy-sync");
    let sync: PrivacyDeviceSync = decode(
        client
            .send(Method::GET, &path, Some(device.token), None)
            .await?,
    )
    .await?;
    ensure!(
        sync.brain_id == brain,
        "Retention response belongs to another Brain."
    );
    if let Some(old) = cached(root, &client.endpoint, device.device_id, brain).await? {
        ensure!(
            sync.sequence >= old.sync.sequence,
            "Server deletion position moved backwards; reconcile the server before continuing."
        );
    }
    let saved = CachedPrivacy {
        endpoint: client.endpoint.clone(),
        device_id: device.device_id,
        sync,
        checked_at: Utc::now(),
    };
    save(root, &saved).await?;
    let mut report = cleanup(root, &saved).await?;
    let _: PrivacyDeviceReceipt = decode(
        client
            .send(
                Method::POST,
                &path,
                Some(device.token),
                Some(serde_json::to_value(PrivacyDeviceAck {
                    sequence: saved.sync.sequence,
                })?),
            )
            .await?,
    )
    .await?;
    report.acknowledged = true;
    Ok(report)
}
