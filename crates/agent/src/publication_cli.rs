use crate::{Client, StoredDevice, decode, publication::*};
use anyhow::{Result, anyhow, bail, ensure};
use recollect_protocol::*;
use reqwest::Method;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const USAGE: &str = "repository publish BRAIN REPOSITORY TASK CHECKOUT [--revision REF] [--retain-file PATH] | repository resume BRAIN BUNDLE_UUID | repository cleanup BRAIN [--offline] | repository snapshots BRAIN REPOSITORY | repository snapshot BRAIN SNAPSHOT | repository manifests BRAIN | repository manifest BRAIN MANIFEST. Repeat --retain-file to explicitly retain permitted text.";
fn value(args: &[String], i: usize) -> Result<&str> {
    args.get(i)
        .map(String::as_str)
        .ok_or_else(|| anyhow!(USAGE))
}
fn id(args: &[String], i: usize) -> Result<Uuid> {
    value(args, i)?
        .parse()
        .map_err(|_| anyhow!("Use an explicit UUID. {USAGE}"))
}
async fn get<T: serde::de::DeserializeOwned>(
    client: &Client,
    device: &StoredDevice,
    path: &str,
) -> Result<T> {
    decode(
        client
            .send(Method::GET, path, Some(device.token), None)
            .await?,
    )
    .await
}
async fn upload(
    client: &Client,
    device: &StoredDevice,
    root: &Path,
    bundle: PreparedPublication,
) -> Result<Value> {
    ensure!(
        bundle.endpoint == client.endpoint && bundle.device_id == device.device_id,
        "Resume this bundle with the original endpoint and paired device."
    );
    client.whoami(device).await?;
    let id = bundle.input.publication_id;
    crate::privacy::synchronize(client, device, bundle.brain_id, root).await?;
    ensure!(
        tokio::fs::try_exists(root.join(format!("{id}.json"))).await?,
        "This prepared publication expired or was erased; its controlled local copy was removed."
    );
    let path = format!(
        "/api/brains/{}/repositories/{}/snapshots",
        bundle.brain_id, bundle.repository_id
    );
    let result: Result<PublicationResult> = async {
        decode(
            client
                .send(
                    Method::POST,
                    &path,
                    Some(device.token),
                    Some(serde_json::to_value(bundle.input)?),
                )
                .await?,
        )
        .await
    }
    .await;
    match result {
        Ok(reply) => {
            remove_bundle(root, id).await?;
            Ok(serde_json::to_value(reply)?)
        }
        Err(error) => {
            // An erasure may commit between policy sync and upload admission.
            let _ = crate::privacy::synchronize(client, device, bundle.brain_id, root).await;
            if !tokio::fs::try_exists(root.join(format!("{id}.json"))).await? {
                bail!(
                    "{error} The controlled prepared copy was removed by current privacy policy."
                );
            }
            bail!(
                "{error} Prepared bundle retained. Resume with: repository resume {} {id}",
                bundle.brain_id
            )
        }
    }
}
pub async fn run(client: &Client, device: &StoredDevice, args: &[String]) -> Result<Value> {
    let action = value(args, 0)?;
    let brain = id(args, 1)?;
    let base = format!("/api/brains/{brain}");
    let root = std::env::var_os("RECOLLECT_PUBLICATION_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root().join(".data/publications"));
    match action {
        "publish" => {
            crate::privacy::synchronize(client, device, brain, &root).await?;
            let repository_id = id(args, 2)?;
            let task_id = id(args, 3)?;
            let checkout = Path::new(value(args, 4)?);
            let mut revision = "HEAD";
            let mut retain_files = Vec::new();
            let mut i = 5;
            while i < args.len() {
                match value(args, i)? {
                    "--revision" => revision = value(args, i + 1)?,
                    "--retain-file" => retain_files.push(value(args, i + 1)?.to_owned()),
                    _ => bail!(USAGE),
                }
                i += 2;
            }
            let identity = client.whoami(device).await?;
            let catalogue: WorkspaceCatalogue =
                get(client, device, &format!("{base}/workspace")).await?;
            let repository = catalogue
                .repositories
                .iter()
                .find(|r| r.id == repository_id)
                .ok_or_else(|| anyhow!("Register this repository in the selected Brain first."))?;
            let policy: RepositoryPolicy =
                get(client, device, &format!("{base}/repositories/policy")).await?;
            let operation: OperationBinding = decode(
                client
                    .send(
                        Method::POST,
                        &format!("{base}/workspace/tasks/{task_id}/operations"),
                        Some(device.token),
                        Some(json!({"kind":"capture"})),
                    )
                    .await?,
            )
            .await?;
            ensure!(
                operation.actor_id == identity.user.id
                    && operation.device_id == Some(device.device_id),
                "Capture binding does not match the paired device."
            );
            let enola = std::env::var_os("RECOLLECT_ENOLA_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(default_enola);
            let input = prepare(Preparation {
                checkout,
                revision,
                repository,
                operation: &operation,
                retain_files,
                allow_file_content: policy.allow_file_content,
                enola: &enola,
                configured_secrets: configured_secrets(),
                staging_root: &root,
            })
            .await?;
            let bundle = PreparedPublication {
                endpoint: client.endpoint.clone(),
                device_id: device.device_id,
                brain_id: brain,
                repository_id,
                input,
            };
            save_bundle(&root, &bundle).await?;
            upload(client, device, &root, bundle).await
        }
        "resume" => {
            ensure!(args.len() == 3, "{USAGE}");
            crate::privacy::synchronize(client, device, brain, &root).await?;
            let bundle = load_bundle(&root, id(args, 2)?).await?;
            ensure!(
                bundle.brain_id == brain,
                "Prepared bundle belongs to a different Brain."
            );
            upload(client, device, &root, bundle).await
        }
        "cleanup" => {
            ensure!(
                args.len() == 2 || (args.len() == 3 && args[2] == "--offline"),
                "{USAGE}"
            );
            let report = if args.len() == 3 {
                crate::privacy::cleanup_known(client,device,brain,&root).await?.ok_or_else(||anyhow!("No known retention policy for this Brain. Run cleanup while connected first."))?
            } else {
                crate::privacy::synchronize(client, device, brain, &root).await?
            };
            Ok(serde_json::to_value(report)?)
        }
        "snapshots" | "snapshot" | "manifest" => {
            ensure!(args.len() == 3, "{USAGE}");
            let resource = id(args, 2)?;
            let path = match action {
                "snapshots" => format!("{base}/repositories/{resource}/snapshots"),
                "snapshot" => format!("{base}/repository-snapshots/{resource}"),
                _ => format!("{base}/revision-manifests/{resource}"),
            };
            get(client, device, &path).await
        }
        "manifests" => {
            ensure!(args.len() == 2, "{USAGE}");
            get(client, device, &format!("{base}/revision-manifests")).await
        }
        _ => bail!(USAGE),
    }
}
