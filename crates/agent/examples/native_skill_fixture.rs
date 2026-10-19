//! Isolated installed-host proof driver. Never resolves credentials or connects
//! to a canonical service; accepts synthetic validated bundles over stdin.
use anyhow::{Result, bail};
use recollect_agent::{plugin_skills::native, plugin_storage::Config};
use recollect_protocol::ManagedSkillBundle;
use serde::Deserialize;
use std::{io::Read, path::PathBuf};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    action: String,
    root: PathBuf,
    workspace: PathBuf,
    host: String,
    config: Config,
    environment: Option<Uuid>,
    task: Uuid,
    #[serde(default)]
    persistent: bool,
    #[serde(default)]
    bundles: Vec<ManagedSkillBundle>,
    id: Option<Uuid>,
}

fn main() -> Result<()> {
    let mut body = String::new();
    std::io::stdin()
        .take(2 * 1024 * 1024 + 1)
        .read_to_string(&mut body)?;
    let request: Request = serde_json::from_str(&body)?;
    let cache = recollect_agent::publication::project_root().join(".cache/native-skills-proof");
    let cache = cache.canonicalize()?;
    let workspace = request.workspace.canonicalize()?;
    if !workspace.starts_with(&cache) || !request.root.starts_with(&cache) {
        bail!("fixture_requires_owned_native_skills_proof_directory");
    }
    let scope = native::NativeScope {
        root: &request.root,
        config: &request.config,
        workspace: &workspace,
        host: native::NativeHost::parse(&request.host)?,
        environment: request.environment,
    };
    let result = match request.action.as_str() {
        "install" => native::install(&scope, request.task, &request.bundles, request.persistent),
        "sync" => native::sync(&scope, request.task, &request.bundles, request.persistent),
        "release" => native::release_task(&scope, request.task),
        "uninstall" => native::uninstall(
            &scope,
            request.id,
            (!request.persistent).then_some(request.task),
        ),
        "status" => native::status(&scope),
        _ => bail!("unknown_fixture_action"),
    }?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
