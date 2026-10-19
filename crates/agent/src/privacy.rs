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

/// Identity fields gathered while streaming the start of a bundle.
#[derive(Default)]
struct Partial {
    endpoint: Option<String>,
    device_id: Option<Uuid>,
    brain_id: Option<Uuid>,
    repository_id: Option<Uuid>,
    publication_id: Option<Uuid>,
    revision: Option<String>,
    captured_at: Option<DateTime<Utc>>,
}
impl Partial {
    fn finish(self) -> Option<BundleHeader> {
        Some(BundleHeader {
            endpoint: self.endpoint?,
            device_id: self.device_id?,
            brain_id: self.brain_id?,
            repository_id: self.repository_id?,
            input: InputHeader {
                publication_id: self.publication_id?,
                revision: self.revision?,
                captured_at: self.captured_at?,
            },
        })
    }
    fn complete(&self) -> bool {
        self.endpoint.is_some()
            && self.device_id.is_some()
            && self.brain_id.is_some()
            && self.repository_id.is_some()
            && self.publication_id.is_some()
            && self.revision.is_some()
            && self.captured_at.is_some()
    }
}
/// Reads identity fields and stops as soon as all are known, so a truncated
/// prefix of a large bundle is enough. Stopping is signalled with an error;
/// the caller trusts the cell only when it is complete.
struct HeaderSeed<'a>(&'a std::cell::RefCell<Partial>, bool);
impl<'de> serde::de::DeserializeSeed<'de> for HeaderSeed<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de> serde::de::Visitor<'de> for HeaderSeed<'_> {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a prepared bundle")
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        mut map: A,
    ) -> std::result::Result<(), A::Error> {
        use serde::de::{Error, IgnoredAny};
        let Self(cell, nested) = self;
        while let Some(key) = map.next_key::<String>()? {
            match (nested, key.as_str()) {
                (false, "endpoint") => cell.borrow_mut().endpoint = Some(map.next_value()?),
                (false, "device_id") => cell.borrow_mut().device_id = Some(map.next_value()?),
                (false, "brain_id") => cell.borrow_mut().brain_id = Some(map.next_value()?),
                (false, "repository_id") => {
                    cell.borrow_mut().repository_id = Some(map.next_value()?)
                }
                (false, "input") => map.next_value_seed(HeaderSeed(cell, true))?,
                (true, "publication_id") => {
                    cell.borrow_mut().publication_id = Some(map.next_value()?)
                }
                (true, "revision") => cell.borrow_mut().revision = Some(map.next_value()?),
                (true, "captured_at") => cell.borrow_mut().captured_at = Some(map.next_value()?),
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
            if cell.borrow().complete() {
                return Err(A::Error::custom("header complete"));
            }
        }
        Ok(())
    }
}
fn header_from_prefix(prefix: &[u8]) -> Option<BundleHeader> {
    use serde::de::DeserializeSeed;
    let cell = std::cell::RefCell::new(Partial::default());
    let mut json = serde_json::Deserializer::from_slice(prefix);
    let _ = HeaderSeed(&cell, false).deserialize(&mut json);
    cell.into_inner().finish()
}
/// Identity of one prepared bundle. Only the first bytes are read when the
/// identity fields lead the file (they do for bundles this version writes);
/// otherwise fall back to the bounded full read so old layouts still clean up.
async fn bundle_header(path: &Path) -> Result<Option<BundleHeader>> {
    use tokio::io::AsyncReadExt;
    const PREFIX: usize = 16 * 1024;
    let limit = PUBLICATION_MAX_BYTES + 8192;
    let meta = fs::symlink_metadata(path)
        .await
        .map_err(|_| anyhow!("Enola did not produce a required artifact."))?;
    ensure!(
        meta.is_file() && !meta.is_symlink() && meta.len() <= limit as u64,
        "Invalid or oversized extraction artifact."
    );
    let mut prefix = Vec::with_capacity(PREFIX.min(meta.len() as usize));
    fs::File::open(path)
        .await?
        .take(PREFIX as u64)
        .read_to_end(&mut prefix)
        .await?;
    if let Some(header) = header_from_prefix(&prefix) {
        return Ok(Some(header));
    }
    if meta.len() as usize <= prefix.len() {
        return Ok(None);
    }
    let bytes = publication::artifact(path, limit).await?;
    Ok(serde_json::from_slice(&bytes).ok())
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
        let header = bundle_header(&file.path()).await?.ok_or_else(|| anyhow!("A prepared bundle has unreadable identity metadata; cleanup remains unacknowledged."))?;
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
        let header = bundle_header(&file.path()).await?.ok_or_else(|| {
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
    removed += crate::plugin_coordination::apply_deletion(&root, endpoint, device, brain)?;
    Ok(removed)
}
pub async fn synchronize(
    client: &Client,
    device: &StoredDevice,
    brain: Uuid,
    root: &Path,
) -> Result<Cleanup> {
    let path = format!("/api/brains/{brain}/privacy-sync");
    let fetched: Result<PrivacyDeviceSync> = async {
        decode(
            client
                .send(Method::GET, &path, Some(device.token), None)
                .await?,
        )
        .await
    }
    .await;
    let sync = match fetched {
        Ok(sync) => sync,
        Err(error) => {
            // Offline expiry must not wait for the server: apply the last known
            // position once, then report the failed synchronization.
            let _ = cleanup_known(client, device, brain, root).await;
            return Err(error);
        }
    };
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
    // One pass under the new position; the former position is never looser.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn header(extra: &str) -> String {
        let id = Uuid::new_v4();
        format!(
            r#"{{"endpoint":"http://127.0.0.1:8787","device_id":"{}","brain_id":"{}","repository_id":"{}","input":{{"publication_id":"{id}","operation_id":"{}","origin":"x","revision":"{}","branch":null,"dirty":false,"captured_at":"2026-10-19T12:00:00Z",{extra}"#,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "a".repeat(40)
        )
    }

    #[tokio::test]
    async fn bundle_identity_is_read_from_the_leading_bytes_only() {
        let root = publication::project_root()
            .join(".cache")
            .join(format!("privacy-header-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).await.unwrap();
        // Valid identity followed by far more than a prefix of unparseable bytes:
        // only a header-only read can succeed.
        let leading = root.join("leading.json");
        let mut body = header(r#""files":["#);
        body.push_str(&"!".repeat(512 * 1024));
        fs::write(&leading, body).await.unwrap();
        let found = bundle_header(&leading).await.unwrap().unwrap();
        assert_eq!(found.input.revision, "a".repeat(40));
        assert_eq!(found.endpoint, "http://127.0.0.1:8787");

        // An older layout with the identity after a large payload still resolves
        // through the bounded full read.
        let trailing = root.join("trailing.json");
        let complete = header(r#""files":[]}}"#);
        let body = format!(
            r#"{{"filler":"{}",{}"#,
            "z".repeat(64 * 1024),
            &complete[1..]
        );
        fs::write(&trailing, body).await.unwrap();
        let tail = bundle_header(&trailing).await.unwrap().unwrap();
        assert_eq!(
            tail.input.captured_at.to_rfc3339(),
            "2026-10-19T12:00:00+00:00"
        );

        // Neither layout: unreadable, not a panic or a silent match.
        let broken = root.join("broken.json");
        fs::write(&broken, b"{\"endpoint\":\"x\"}").await.unwrap();
        assert!(bundle_header(&broken).await.unwrap().is_none());
        fs::remove_dir_all(root).await.unwrap();
    }
}
