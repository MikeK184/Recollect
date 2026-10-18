//! Revision4 isolated qualification campaign. Default has no forwarding path.
#[path = "revision4_qualification/campaign.rs"]
mod campaign;
#[path = "revision4_qualification/controls.rs"]
mod controls;
#[path = "revision4_qualification/export.rs"]
mod export;
#[path = "revision4_qualification/guard.rs"]
mod guard;
#[path = "revision4_qualification/inputs.rs"]
mod inputs;
#[path = "revision4_qualification/io.rs"]
mod io;
#[path = "revision4_qualification/native.rs"]
mod native;
#[path = "revision4_qualification/owned.rs"]
mod owned;
#[path = "revision4_qualification/pipeline.rs"]
mod pipeline;
#[path = "revision4_qualification/proof.rs"]
mod proof;
#[path = "revision4_qualification/scopes.rs"]
mod scopes;
#[path = "revision4_qualification/support.rs"]
mod support;
#[path = "revision4_qualification/transport.rs"]
mod transport;
#[path = "revision4_qualification/wire.rs"]
mod wire;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
const BASE: &str = ".cache/automatic-mapping/revision4-campaign-conformance-2026-10-18";
#[derive(Debug, PartialEq)]
struct Args {
    mode: String,
    values: BTreeMap<String, String>,
}
fn args(a: Vec<String>) -> Result<Args> {
    let mut values = BTreeMap::new();
    let mut mode = None;
    let mut i = 0;
    while i < a.len() {
        let flag = &a[i];
        if [
            "--conformance",
            "--prepare",
            "--execute",
            "--export",
            "--cleanup",
        ]
        .contains(&flag.as_str())
        {
            ensure!(mode.is_none(), "One explicit mode");
            mode = Some(flag.clone());
            i += 1;
        } else {
            ensure!(
                [
                    "--inputs",
                    "--inputs-sha256",
                    "--out",
                    "--frozen",
                    "--approval"
                ]
                .contains(&flag.as_str())
                    && i + 1 < a.len()
                    && !a[i + 1].starts_with("--")
                    && values.insert(flag.clone(), a[i + 1].clone()).is_none(),
                "Unknown/repeated/missing argument"
            );
            i += 2;
        }
    }
    let mode = mode.context("Explicit mode required")?;
    let expected: &[&str] = match mode.as_str() {
        "--conformance" => &["--out"],
        "--prepare" => &["--inputs", "--inputs-sha256", "--out"],
        "--execute" => &["--frozen", "--approval"],
        _ => &["--frozen"],
    };
    ensure!(
        values.len() == expected.len() && expected.iter().all(|s| values.contains_key(*s)),
        "Mixed arguments"
    );
    Ok(Args { mode, values })
}
fn root() -> Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}
fn output(root: &Path, text: &str, new: bool) -> Result<PathBuf> {
    let p = io::contained(root, text)?;
    ensure!(
        p.parent() == Some(root.join(BASE).as_path())
            && p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("run-")),
        "Explicit new run path required"
    );
    if new {
        ensure!(!p.exists(), "Existing output refused");
    } else {
        io::private_directory(&p)?;
    }
    Ok(p)
}
async fn run(a: Args) -> Result<Value> {
    let root = root()?;
    match a.mode.as_str() {
        "--conformance" => {
            let out = output(&root, &a.values["--out"], true)?;
            proof::conformance(&root, &out).await
        }
        "--prepare" => {
            let out = output(&root, &a.values["--out"], true)?;
            let input = io::contained(&root, &a.values["--inputs"])?;
            let admitted = inputs::admit(&root, &input, &a.values["--inputs-sha256"])?;
            campaign::prepare(&root, &out, admitted, false).await
        }
        "--execute" => {
            let out = output(&root, &a.values["--frozen"], false)?;
            let approval = io::contained(&root, &a.values["--approval"])?;
            campaign::execute(&root, &out, Some(&approval), false).await
        }
        "--export" => {
            let out = output(&root, &a.values["--frozen"], false)?;
            campaign::export(&root, &out).await
        }
        "--cleanup" => {
            let out = output(&root, &a.values["--frozen"], false)?;
            campaign::cleanup(&root, &out).await
        }
        _ => unreachable!(),
    }
}
#[tokio::main]
async fn main() {
    let a = match args(std::env::args().skip(1).collect()) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("revision4 campaign arguments invalid");
            std::process::exit(2)
        }
    };
    let failure_out = a
        .values
        .get("--out")
        .cloned()
        .or_else(|| a.values.get("--frozen").cloned());
    match Box::pin(run(a)).await {
        Ok(v) => println!("{}", v),
        Err(error) => {
            if let (Ok(root), Some(path)) = (root(), failure_out) {
                if let Ok(out) = output(&root, &path, false) {
                    let contexts: Vec<_> = error
                        .chain()
                        .map(|e| e.to_string())
                        .filter(|s| {
                            s.len() < 240
                                && !s.contains("://")
                                && !s.to_lowercase().contains("password")
                                && !s.to_lowercase().contains("token")
                                && !s.to_lowercase().contains("secret")
                        })
                        .collect();
                    let _ = io::json(
                        &out.join(format!("failure-{}.json", uuid::Uuid::new_v4())),
                        &serde_json::json!({"state":"failed_owned_evidence_retained","diagnostic_contexts":contexts,"resumption_forbidden":true}),
                    );
                }
            }
            eprintln!("revision4 campaign failed");
            std::process::exit(1)
        }
    }
}
