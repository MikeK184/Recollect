use crate::{Client, StoredDevice, decode, workspace};
use anyhow::{Result, anyhow, bail, ensure};
use recollect_protocol::{
    CheckoutRefreshResult, CreateTask, OperationBinding, RecallGraphOptions, RecallRequest,
    ScopeSelection, WorkspaceCatalogue,
};
use reqwest::Method;
use serde_json::{Value, json};
use std::path::Path;
use uuid::Uuid;

pub const USAGE: &str = "workspace discover|refresh|list [directory]; scope start BRAIN LABEL [--repository ID --area ID --environment ID --workspace ID]; scope fork BRAIN PARENT LABEL [scope options]; scope change BRAIN TASK BASE_SCOPE [scope options]; scope begin BRAIN TASK KIND; scope recall BRAIN CONTEXT_OPERATION QUERY [--channels exact,lexical,semantic,graph --manifest ID --graph-kind knowledge|repository|combined --graph-direction both|incoming|outgoing --graph-hops 1..3 --graph-relations RELATION,... --source-diversity true|false]; scope inspect|close BRAIN TASK. Repeat --repository and --area. IDs are UUIDs; omitted scope dimensions mean Brain-wide.";

fn value(args: &[String], index: usize) -> Result<&str> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| anyhow!("{USAGE}"))
}
fn id(args: &[String], index: usize) -> Result<Uuid> {
    value(args, index)?
        .parse()
        .map_err(|_| anyhow!("Use a Brain/task/resource UUID. {USAGE}"))
}

async fn request(
    client: &Client,
    device: &StoredDevice,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value> {
    decode(client.send(method, path, Some(device.token), body).await?).await
}
fn selection(args: &[String]) -> Result<(Option<ScopeSelection>, Option<Uuid>)> {
    let mut scope = ScopeSelection::default();
    let mut selected = false;
    let mut workspace = None;
    let mut index = 0;
    while index < args.len() {
        let option = value(args, index)?;
        let resource = id(args, index + 1)?;
        match option {
            "--repository" => {
                scope.repository_ids.push(resource);
                selected = true;
            }
            "--area" => {
                scope.area_ids.push(resource);
                selected = true;
            }
            "--environment" => {
                ensure!(
                    scope.environment_id.is_none(),
                    "Select only one environment."
                );
                scope.environment_id = Some(resource);
                selected = true;
            }
            "--workspace" => {
                ensure!(workspace.is_none(), "Select only one workspace.");
                workspace = Some(resource);
            }
            _ => bail!("Unknown scope option. {USAGE}"),
        }
        index += 2;
    }
    Ok((selected.then_some(scope), workspace))
}

pub async fn run(
    client: &Client,
    device: &StoredDevice,
    command: &str,
    args: &[String],
) -> Result<Value> {
    if command == "workspace" {
        let action = value(args, 0)?;
        ensure!(
            matches!(action, "refresh" | "list") && args.len() <= 2,
            "{USAGE}"
        );
        let directory = Path::new(args.get(1).map(String::as_str).unwrap_or("."));
        let (root, selector) = workspace::selector(directory).await?;
        let brains = client.brains(device).await?;
        let matches: Vec<_> = brains
            .iter()
            .filter(|b| b.id.to_string() == selector || b.name == selector)
            .collect();
        ensure!(
            matches.len() == 1,
            "Workspace selector must match exactly one accessible Brain. Use its UUID if names are ambiguous."
        );
        let brain = matches[0].id;
        let path = format!("/api/brains/{brain}/workspace");
        if action == "refresh" {
            let report = workspace::discover(directory).await?;
            ensure!(
                report.brain_selector == selector
                    && Path::new(&report.refresh.workspace_root) == root,
                "Workspace selector changed during discovery. Retry explicitly."
            );
            let response: CheckoutRefreshResult = decode(
                client
                    .send(
                        Method::POST,
                        &format!("{path}/checkouts"),
                        Some(device.token),
                        Some(serde_json::to_value(report.refresh)?),
                    )
                    .await?,
            )
            .await?;
            let inventory: WorkspaceCatalogue = decode(
                client
                    .send(
                        Method::GET,
                        &format!("{path}?workspace_id={}", response.workspace.id),
                        Some(device.token),
                        None,
                    )
                    .await?,
            )
            .await?;
            return Ok(
                json!({"refresh":response,"catalogue":inventory,"excluded_workspaces":report.excluded_workspaces}),
            );
        }
        let mut inventory: WorkspaceCatalogue = decode(
            client
                .send(Method::GET, &path, Some(device.token), None)
                .await?,
        )
        .await?;
        let root = root.to_string_lossy().replace('\\', "/");
        let selected = inventory
            .workspaces
            .iter()
            .find(|workspace| workspace.device_id == device.device_id && workspace.root == root)
            .map(|workspace| workspace.id);
        if let Some(selected) = selected {
            if inventory.selected_workspace != Some(selected) {
                inventory = decode(
                    client
                        .send(
                            Method::GET,
                            &format!("{path}?workspace_id={selected}"),
                            Some(device.token),
                            None,
                        )
                        .await?,
                )
                .await?;
            }
        } else {
            inventory.selected_workspace = None;
            inventory.checkouts.clear();
            inventory.checkout_total = 0;
        }
        return Ok(serde_json::to_value(inventory)?);
    }
    let action = value(args, 0)?;
    let brain = id(args, 1)?;
    let base = format!("/api/brains/{brain}/workspace");
    match action {
        "start" | "fork" => {
            let label_index = if action == "fork" { 3 } else { 2 };
            let label = value(args, label_index)?.to_owned();
            let (selection, workspace_id) = selection(&args[label_index + 1..])?;
            let parent_task_id = if action == "fork" {
                Some(id(args, 2)?)
            } else {
                None
            };
            request(
                client,
                device,
                Method::POST,
                &format!("{base}/tasks"),
                Some(serde_json::to_value(CreateTask {
                    label,
                    parent_task_id,
                    workspace_id,
                    selection,
                })?),
            )
            .await
        }
        "change" => {
            let task = id(args, 2)?;
            let base_scope = id(args, 3)?;
            let (selection, workspace) = selection(&args[4..])?;
            ensure!(
                workspace.is_none(),
                "A task's workspace does not change with its scope. Start a new task instead."
            );
            request(
                client,
                device,
                Method::PUT,
                &format!("{base}/tasks/{task}/scope"),
                Some(json!({"base_scope":base_scope,"selection":selection.unwrap_or_default()})),
            )
            .await
        }
        "recall" => {
            ensure!(args.len() >= 4 && args.len().is_multiple_of(2), "{USAGE}");
            let mut input = RecallRequest {
                query: value(args, 3)?.into(),
                ..Default::default()
            };
            let mut graph = RecallGraphOptions::default();
            let mut graph_options = false;
            let mut seen = std::collections::BTreeSet::new();
            for pair in args[4..].chunks_exact(2) {
                ensure!(
                    seen.insert(pair[0].as_str()),
                    "Repeated recall option. {USAGE}"
                );
                graph_options |= pair[0].starts_with("--graph-");
                match pair[0].as_str() {
                    "--channels" => {
                        input.channels = pair[1].split(',').map(str::to_string).collect()
                    }
                    "--manifest" => input.manifest_revision_id = Some(pair[1].parse()?),
                    "--graph-kind" => graph.kind = pair[1].clone(),
                    "--graph-direction" => graph.direction = pair[1].clone(),
                    "--graph-hops" => graph.max_hops = pair[1].parse()?,
                    "--graph-relations" => {
                        graph.relations = pair[1]
                            .split(',')
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .collect()
                    }
                    "--source-diversity" => input.source_diversity = pair[1].parse()?,
                    _ => bail!("Unknown recall option. {USAGE}"),
                }
            }
            if graph_options || input.channels.iter().any(|c| c == "graph") {
                input.graph = Some(graph);
            }
            input.semantic_request_id = input
                .channels
                .iter()
                .any(|c| c == "semantic")
                .then(Uuid::new_v4);
            let operation_id = id(args, 2)?;
            let operation: OperationBinding = serde_json::from_value(
                request(
                    client,
                    device,
                    Method::GET,
                    &format!("{base}/operations/{operation_id}"),
                    None,
                )
                .await?,
            )?;
            input.operation_id = Some(operation_id);
            input.selection = operation.scope.selection;
            request(
                client,
                device,
                Method::POST,
                &format!("/api/brains/{brain}/recall"),
                Some(serde_json::to_value(input)?),
            )
            .await
        }
        "begin" => {
            let task = id(args, 2)?;
            let kind = value(args, 3)?;
            ensure!(args.len() == 4, "{USAGE}");
            request(
                client,
                device,
                Method::POST,
                &format!("{base}/tasks/{task}/operations"),
                Some(json!({"kind":kind})),
            )
            .await
        }
        "inspect" | "close" => {
            let task = id(args, 2)?;
            ensure!(args.len() == 3, "{USAGE}");
            let path = format!("{base}/tasks/{task}");
            if action == "close" {
                request(client, device, Method::POST, &format!("{path}/close"), None).await
            } else {
                request(client, device, Method::GET, &path, None).await
            }
        }
        _ => bail!("{USAGE}"),
    }
}
