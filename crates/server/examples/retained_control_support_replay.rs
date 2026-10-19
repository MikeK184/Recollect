//! Separate retained-output replay diagnostic. Default is a zero-call freeze.
//! No old run, request identity, stage, verdict or lease is resumed or copied.
#[allow(dead_code)]
#[path = "mapping_extractor_benchmark/support.rs"]
mod common;
#[allow(dead_code)]
#[path = "mapping_qualification/support/reconciliation.rs"]
mod fixture;
#[path = "retained_control_support/guard.rs"]
mod guard;
#[path = "retained_control_support/replay.rs"]
mod replay;
// The shared benchmark utility's existing cfg(test) proof refers to its owning
// example recipe. Keep that test-only interface without changing frozen helpers.
#[cfg(test)]
#[path = "mapping_extractor_benchmark.rs"]
mod original_example;
#[cfg(test)]
fn recipe(root: &Path, config: &Config) -> Result<(Value, Vec<Value>, Value)> {
    original_example::recipe(root, config)
}

use anyhow::{Context, Result, ensure};
use recollect_server::{config::Config, db};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const KIND: &str = "control-support6-replay-1";
const ORIGINAL: &str = ".cache/automatic-mapping/extractor-remaining111-current-v4-2026-10-18";
const ORIGINAL_RECIPE: &str = "19a6a7b429c1eb1f911e1e0df6ddb2615c0422e4b83953a5b40102413ecc5bb5";
const OUTCOMES_SHA: &str = "b931745d9b600a0a0b1e887a21e7f9d0402bb52b5d73c4ef98bd2826810b08fe";
const PRICE: &str = ".cache/automatic-mapping/source-support6-preparation-2026-10-18/support-pricing-2026-10-18.json";

fn read(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn hash(v: &Value) -> Result<String> {
    common::sha256(&serde_json::to_vec(v)?)
}
fn root() -> Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn retained(root: &Path) -> Result<(Value, Vec<Value>)> {
    let base = root.join(ORIGINAL);
    ensure!(
        hash(&read(&base.join("recipe.json"))?)? == ORIGINAL_RECIPE,
        "Immutable original recipe changed"
    );
    ensure!(
        read(&base.join("execution-receipt.json"))?["status"] == "completed",
        "Original campaign is not complete"
    );
    let bytes = fs::read(base.join("control-outcomes.jsonl"))?;
    ensure!(
        common::sha256(&bytes)? == OUTCOMES_SHA,
        "Immutable six real outputs changed"
    );
    let rows: Vec<Value> = bytes
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(serde_json::from_slice)
        .collect::<std::result::Result<_, _>>()?;
    let cases = read(&root.join(fixture::CONTROLS_PATH))?;
    ensure!(
        common::sha256(&fs::read(root.join(fixture::CONTROLS_PATH))?)? == fixture::CONTROLS_SHA256,
        "Frozen controls changed"
    );
    ensure!(
        rows.len() == 6
            && rows
                .iter()
                .zip(cases["cases"].as_array().context("Six controls")?)
                .all(|(r, c)| r["id"] == c["id"] && r["request"]["state"] == "succeeded"),
        "All six real outcomes required"
    );
    ensure!(
        rows.iter()
            .map(|r| r["raw_output"]["claims"].as_array().map_or(0, Vec::len))
            .sum::<usize>()
            == 10,
        "Retain all ten raw claims"
    );
    Ok((cases, rows))
}

fn code_bindings(root: &Path) -> Result<Value> {
    fn walk(root: &Path, path: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
        for e in fs::read_dir(path)? {
            let p = e?.path();
            if p.is_dir() {
                walk(root, &p, paths)?;
            } else if p.extension().is_some_and(|s| s == "rs") {
                paths.push(p.strip_prefix(root)?.into());
            }
        }
        Ok(())
    }
    let mut paths = vec![];
    for p in [
        "crates/server/src",
        "crates/protocol/src",
        "crates/server/examples/retained_control_support",
    ] {
        walk(root, &root.join(p), &mut paths)?;
    }
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "crates/server/Cargo.toml",
        "crates/protocol/Cargo.toml",
        "crates/server/examples/retained_control_support_replay.rs",
        "crates/server/examples/mapping_extractor_benchmark/support.rs",
        "crates/server/examples/mapping_qualification/support/reconciliation.rs",
    ] {
        paths.push(p.into());
    }
    paths.sort();
    paths.dedup();
    let bindings: BTreeMap<String, String> = paths
        .into_iter()
        .map(|p| {
            Ok((
                p.to_string_lossy().into(),
                common::sha256(&fs::read(root.join(p))?)?,
            ))
        })
        .collect::<Result<_>>()?;
    Ok(json!(bindings))
}
fn migrations(root: &Path) -> Result<Value> {
    let mut result = serde_json::Map::new();
    for (name, compiled) in db::MIGRATIONS {
        let raw = fs::read(root.join(format!("crates/server/migrations/{name}.sql")))?;
        ensure!(
            raw == compiled.as_bytes(),
            "Migration source differs from compiled executable"
        );
        result.insert((*name).into(), json!(common::sha256(&raw)?));
    }
    Ok(json!(result))
}

fn configuration(config: &Config) -> Result<Value> {
    ensure!(
        config.models.text_model.as_deref() == Some("gpt-5.6-luna")
            && config.models.endpoint == "https://api.openai.com/v1",
        "Exact installed provider/model/endpoint required; no substitution"
    );
    Ok(
        json!({"provider":"openai","model":config.models.text_model,"official_forward_endpoint":config.models.endpoint,"router":"private dynamically bound127.0.0.1 Responses-only; no proxy environment; exact bytes; no redirect/retry","prompt_label":"source-support-3","schema_label":"source-support-3","format_name":"source_support"}),
    )
}

fn pricing(root: &Path, price: &Value) -> Result<()> {
    common::validate_pricing(price, "gpt-5.6-luna")?;
    let retrieval = root.join(
        price["official_retrieval_path"]
            .as_str()
            .context("Official retrieval receipt path")?,
    );
    ensure!(
        common::sha256(&fs::read(&retrieval)?)? == price["official_retrieval_sha256"],
        "Official price retrieval binding changed"
    );
    let receipt = read(&retrieval)?;
    ensure!(
        receipt["verified_at"] == price["verified_at"]
            && receipt["model"] == price["model"]
            && receipt["no_provider_inference_called"] == true
            && receipt["source_sha256"] == price["official_source_sha256"]
            && receipt["conservative_nano_usd_per_token"]["input_max"] == 250
            && receipt["conservative_nano_usd_per_token"]["output_max"] == 1200,
        "Official exact model price receipt differs"
    );
    ensure!(
        common::sha256(&fs::read(
            root.join(
                price["official_source_path"]
                    .as_str()
                    .context("Official markdown source")?
            )
        )?)? == price["official_source_sha256"],
        "Official retrieved source changed"
    );
    Ok(())
}

fn freeze(root: &Path, config: &Config, proof: &Path, guards: &Path) -> Result<Value> {
    let (_, rows) = retained(root)?;
    let receipt = read(&proof.join("conformance-receipt.json"))?;
    ensure!(
        receipt["status"] == "completed"
            && receipt["external_requests"] == 0
            && receipt["all_six_raw_outputs_preserved"] == true,
        "Complete zero-paid native conformance required"
    );
    let semantics = read(&proof.join("semantic-manifest.json"))?;
    ensure!(
        semantics["cases"].as_array().is_some_and(|a| a.len() == 6),
        "All six normalized semantic fingerprints required"
    );
    let executable = common::sha256(&fs::read(std::env::current_exe()?)?)?;
    let code = code_bindings(root)?;
    let code_hash = hash(&code)?;
    let guards_receipt = read(&guards.join("guard-proof-receipt.json"))?;
    ensure!(
        guards_receipt["status"] == "passed"
            && guards_receipt["external_requests"] == 0
            && guards_receipt["compiled_runner_sha256"] == executable
            && guards_receipt["code_manifest_sha256"] == code_hash,
        "Current executable zero-paid guard proof required"
    );
    ensure!(
        receipt["compiled_runner_sha256"] == executable
            && receipt["code_manifest_sha256"] == code_hash,
        "Current executable/source native proof required"
    );
    let prompt = semantics["cases"][0]["instructions_sha256"].clone();
    let schema = semantics["cases"][0]["schema_sha256"].clone();
    ensure!(
        semantics["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["instructions_sha256"] == prompt && c["schema_sha256"] == schema),
        "All six support prompts/schemas must be exact and identical"
    );
    let pricing = read(&root.join(PRICE))?;
    self::pricing(root, &pricing)?;
    let maximum = common::worst_case(&pricing, 6)?;
    ensure!(
        maximum <= 150_000_000,
        "Fresh six-call maximum exceeds reviewed15cent hard cap"
    );
    Ok(
        json!({"version":1,"campaign_kind":KIND,"mode":"explicit_retained_extraction_replay_then_current_support_only","config":configuration(config)?,
        "compiled_runner_sha256":executable,"code_sha256":code,"migration_sha256":migrations(root)?,"support_prompt_sha256":prompt,"support_schema_sha256":schema,
        "original":{"directory":ORIGINAL,"recipe_sha256":ORIGINAL_RECIPE,"control_outcomes_sha256":OUTCOMES_SHA,"controls_path":fixture::CONTROLS_PATH,"controls_sha256":fixture::CONTROLS_SHA256,"raw_outcomes":rows.len(),"raw_claims":10,"raw_retirements":0,"original_request_identities_reused":false},
        "conformance":{"path":proof,"receipt_sha256":common::sha256(&fs::read(proof.join("conformance-receipt.json"))?)?,"semantic_manifest_sha256":common::sha256(&fs::read(proof.join("semantic-manifest.json"))?)?,"semantics":semantics},
        "guards":{"path":guards,"receipt_sha256":common::sha256(&fs::read(guards.join("guard-proof-receipt.json"))?)?},
        "pricing_path":PRICE,"pricing_sha256":common::sha256(&fs::read(root.join(PRICE))?)?,"pricing":pricing,
        "limits":{"external_support_requests":6,"external_extraction_requests":0,"embedding_requests":0,"answer_judging":0,"concurrent":1,"retries":0,"complete_body_bytes":65536,"framing_input_tokens":2048,"maximum_output_tokens":4096,"maximum_input_tokens":67584,"per_request_nano_usd":maximum/6,"total_nano_usd":maximum,"total_usd":common::dollars(maximum),"human_approval_hardcap_nano_usd":150000000},
        "evaluation":{"controlled_replay":"enforcement/plumbing only; synthetic assessor outputs are never semantic quality","real_support":"observed frozen six-control diagnostics, all raw and normalized actions and postconditions; not new untouched verifier qualification","no_tuning":true,"no_activation":true,"required_retirement_missing_remains_failure":true,"server_ids_and_timestamps":"new and separately recorded; only declared typed canonical identity mappings allowed; text untouched"}}),
    )
}

fn approval_template(recipe: &Value, recipe_hash: &str) -> Value {
    json!({"approve_paid_calls":false,"authorization_reference":"UNAPPROVED","campaign_kind":KIND,"recipe_sha256":recipe_hash,"pricing_sha256":recipe["pricing_sha256"],"control_outcomes_sha256":OUTCOMES_SHA,"controls_sha256":fixture::CONTROLS_SHA256,"semantic_manifest_sha256":recipe["conformance"]["semantic_manifest_sha256"],"maximum_support_requests":6,"maximum_external_extraction_requests":0,"maximum_embedding_requests":0,"maximum_output_tokens":4096,"budget_usd":"0.15"})
}

fn freshness(root: &Path, config: &Config, recipe: &Value) -> Result<()> {
    let _ = retained(root)?;
    ensure!(
        recipe["campaign_kind"] == KIND && recipe["config"] == configuration(config)?,
        "Campaign/config changed"
    );
    ensure!(
        recipe["compiled_runner_sha256"] == common::sha256(&fs::read(std::env::current_exe()?)?)?
            && recipe["code_sha256"] == code_bindings(root)?,
        "Executable/source changed"
    );
    ensure!(
        common::sha256(&fs::read(root.join(PRICE))?)? == recipe["pricing_sha256"],
        "Price binding changed"
    );
    pricing(root, &recipe["pricing"])?;
    let proof = Path::new(
        recipe["conformance"]["path"]
            .as_str()
            .context("Conformance path")?,
    );
    ensure!(
        common::sha256(&fs::read(proof.join("semantic-manifest.json"))?)?
            == recipe["conformance"]["semantic_manifest_sha256"]
            && read(&proof.join("semantic-manifest.json"))? == recipe["conformance"]["semantics"],
        "Frozen normalized semantic cohort changed"
    );
    ensure!(
        common::sha256(&fs::read(proof.join("conformance-receipt.json"))?)?
            == recipe["conformance"]["receipt_sha256"],
        "Conformance receipt changed"
    );
    let guards = Path::new(
        recipe["guards"]["path"]
            .as_str()
            .context("Guard proof path")?,
    );
    ensure!(
        common::sha256(&fs::read(guards.join("guard-proof-receipt.json"))?)?
            == recipe["guards"]["receipt_sha256"],
        "Guard proof receipt changed"
    );
    ensure!(
        recipe["migration_sha256"] == migrations(root)?,
        "Migrations changed"
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seen = std::collections::BTreeSet::new();
    let mut n = 0;
    while n < args.len() {
        let key = &args[n];
        ensure!(seen.insert(key.clone()), "Duplicate argument");
        match key.as_str() {
            "--conformance" | "--guard-proof" => n += 1,
            "--out" | "--freeze-from" | "--guards-from" | "--execute-approval" => {
                ensure!(
                    args.get(n + 1).is_some_and(|s| !s.starts_with("--")),
                    "Argument value missing"
                );
                n += 2
            }
            _ => anyhow::bail!("Unknown replay argument"),
        }
    }
    ensure!(
        ["--conformance", "--guard-proof", "--execute-approval"]
            .iter()
            .filter(|s| seen.contains(**s))
            .count()
            <= 1,
        "Mutually exclusive replay modes"
    );
    let value = |key: &str| -> Result<Option<PathBuf>> {
        match args.iter().position(|s| s == key) {
            Some(n) => Ok(Some(args.get(n + 1).context("Missing argument")?.into())),
            None => Ok(None),
        }
    };
    let out = value("--out")?.context("--out required")?;
    let root = root()?;
    let config = Config::from_env()?;
    if args.iter().any(|a| a == "--conformance") {
        ensure!(
            value("--execute-approval")?.is_none(),
            "Conformance cannot enable paid routing"
        );
        ensure!(
            !out.exists(),
            "Use a new owned conformance directory; no resume"
        );
        fs::create_dir_all(&out)?;
        return replay::conformance(&root, &out, config).await;
    }
    if args.iter().any(|a| a == "--guard-proof") {
        ensure!(!out.exists(), "Use a new proof directory");
        fs::create_dir_all(&out)?;
        return guard::proof(&out).await;
    }
    if let Some(grant) = value("--execute-approval")? {
        let recipe = read(&out.join("recipe.json"))?;
        let recipe_hash = hash(&recipe)?;
        let approval = read(&grant)?;
        // Reject false/old approvals before locks, fixture creation or any calls.
        guard::validate_approval(&approval, &recipe, &recipe_hash)?;
        freshness(&root, &config, &recipe)?;
        let _lock = common::private_file(&out.join("execution.lock"))?;
        common::private_json(
            &out.join("authorization-admission-receipt.json"),
            &json!({"recipe_sha256":recipe_hash,"approval_sha256":common::sha256(&fs::read(&grant)?)?,"approval":approval,"maximum_requests":6,"actual_transmissions_at_admission":0,"admitted_at":chrono::Utc::now()}),
        )?;
        return replay::paid(&root, &out, config, &recipe).await;
    }
    let proof = value("--freeze-from")?
        .context("--freeze-from completed conformance directory required")?;
    let guards = value("--guards-from")?.context("--guards-from completed guard proof required")?;
    let recipe = freeze(&root, &config, &proof, &guards)?;
    ensure!(!out.exists(), "Never replace a frozen recipe");
    fs::create_dir_all(&out)?;
    let h = hash(&recipe)?;
    common::private_json(&out.join("recipe.json"), &recipe)?;
    common::private_json(
        &out.join("approval-template.json"),
        &approval_template(&recipe, &h),
    )?;
    common::private_json(
        &out.join("freeze-receipt.json"),
        &json!({"recipe_sha256":h,"paid_authorized":false,"external_requests":0,"database_created":false}),
    )?;
    println!("Frozen false/unapproved support6 recipe {h}; no requests");
    Ok(())
}
