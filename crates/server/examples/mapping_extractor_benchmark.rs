//! Frozen extractor-only campaign. Default is a zero-network, zero-DB dry run.
//! A future paid run needs an explicit approval file bound to its frozen recipe.
//! Uses the public production gateway and adapter, never a direct provider SDK.
#[path = "mapping_extractor_benchmark/support.rs"]
pub(crate) mod support;

use anyhow::{Context, Result, ensure};
use recollect_server::{
    config::Config, db, knowledge_mapping::production, model_gateway as gateway,
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use support::*;
use uuid::Uuid;

const ORIGINAL: &str = "docs/research/automatic-knowledge-mapping-fixtures-2026-10-09.json";
const EXTRA: &str = "docs/research/graph-organization-adversarial-fixtures-2026-10-18.json";
const ORIGINAL_HASH: &str = "80e2040b63efa5738f888f99cae3234adfae6b0ad484ccd256da189d6bdd0dbc";
const EXTRA_HASH: &str = "a0ecfe7dd692c2517f00035c0be7205841e27e7b17551f7a21580edc6d1c916c";
const PRICING: &str = "docs/research/mapping-extractor-pricing-2026-10-18.json";

pub(crate) fn recipe(
    root: &std::path::Path,
    config: &Config,
) -> Result<(Value, Vec<Value>, Value)> {
    let mut schedule = vec![];
    let mut corpora = vec![];
    for (name, path, expected, count) in [
        ("original40", ORIGINAL, ORIGINAL_HASH, 40),
        ("adversarial20", EXTRA, EXTRA_HASH, 20),
    ] {
        let bytes = fs::read(root.join(path))?;
        ensure!(sha256(&bytes)? == expected, "Frozen corpus changed: {name}");
        let data: Value = serde_json::from_slice(&bytes)?;
        let docs = data["documents"].as_array().context("Missing documents")?;
        ensure!(docs.len() == count, "Frozen corpus count changed");
        let mut partitions = serde_json::Map::new();
        for d in docs {
            let partition = d["partition"].as_str().context("Missing partition")?;
            ensure!(
                ["development", "held_out"].contains(&partition),
                "Unknown partition"
            );
            *partitions.entry(partition).or_insert(json!(0)) = json!(
                partitions
                    .get(partition)
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    + 1
            );
            let text = d["text"].as_str().context("Missing text")?;
            schedule.push(json!({"corpus":name,"id":d["id"],"partition":partition,
                "text":text,"text_sha256":sha256(text.as_bytes())?}));
        }
        corpora.push(json!({"name":name,"path":path,"sha256":expected,"documents":count,
            "partitions":partitions,"annotations":if name=="original40" {"entity-span-type only"}else{"entity-span-type; alias/relation endpoints; identity/topic gold retained for separate canonical evaluation"}}));
    }
    // All development precedes held-out; no tuning or second attempt is built in.
    schedule.sort_by_key(|d| {
        (
            d["partition"] == "held_out",
            d["corpus"].to_string(),
            d["id"].to_string(),
        )
    });
    let inv = gateway::extraction(Uuid::nil(), Uuid::nil());
    let schema = gateway::extraction_schema();
    let price_bytes = fs::read(root.join(PRICING))?;
    let pricing: Value = serde_json::from_slice(&price_bytes)?;
    validate_pricing(&pricing, &config.models.text_model)?;
    let cost = worst_case(&pricing, schedule.len() as u64)?;
    let mut code = serde_json::Map::new();
    for path in [
        "crates/server/examples/mapping_extractor_benchmark.rs",
        "crates/server/examples/mapping_extractor_benchmark/support.rs",
        "crates/server/examples/mapping_extractor_benchmark/proof.rs",
        "crates/server/src/model_gateway.rs",
        "crates/server/src/knowledge_mapping/production.rs",
        "crates/server/src/knowledge_mapping.rs",
        "crates/server/src/model_policy.rs",
        "crates/server/src/db.rs",
        "Cargo.lock",
        "crates/server/src/config.rs",
        "scripts/score-extractor.py",
    ] {
        code.insert(path.into(), json!(sha256(&fs::read(root.join(path))?)?));
    }
    let mut migrations = serde_json::Map::new();
    for (name, sql) in db::MIGRATIONS {
        migrations.insert((*name).into(), json!(sha256(sql.as_bytes())?));
    }
    let recipe = json!({"version":1,"kind":"production-extractor-only","provider":"openai",
        "model":config.models.text_model,"endpoint":config.models.endpoint,
        "prompt_label":inv.prompt_label,"schema_label":inv.schema_label,"adapter_revision":production::ADAPTER,
        "prompt_sha256":sha256(inv.instructions.as_bytes())?,"schema_sha256":sha256(&serde_json::to_vec(&schema)?)?,
        "corpora":corpora,"schedule_sha256":sha256(&serde_json::to_vec(&schedule)?)?,"code_sha256":code,"migration_sha256":migrations,
        "pricing_path":PRICING,"pricing_sha256":sha256(&price_bytes)?,"pricing":pricing,
        "limits":{"calls":60,"input_bytes":16384,"complete_request_bytes":MAX_BODY,"input_token_framing":FRAMING,
            "output_tokens":MAX_OUTPUT,"concurrent":1,"retries":0,"worst_case_nano_usd":cost,
            "worst_case_usd":dollars(cost)},
        "payload":{"sent":["unchanged production instructions and JSON schema","one exact synthetic document as canonical source input0","gateway-resolved provenance"],
            "never_sent":["gold annotations","partition","expected aliases/relations/identities/topics","benchmark metric targets","other documents or examples"],
            "tools":false,"answer_judging":false,"new_embeddings":false},
        "evaluation":{"original40":"entity-span-type only; missing endpoint annotations are not empty gold",
            "adversarial20":"entity-span-type and alias/relation endpoint coverage; negative/history/hypothesis separately reported",
            "partitions":{"development":42,"held_out":18},"unsupported_language_documents":1,
            "failures":"Every document remains in outcomes; English failed/missing/truncated outputs retain all expected mentions as FN",
            "not_measured":["canonical identity/realm normalization or merging","alias acceptance disposition classifier","canonical/native topic membership or label quality","production provider account availability"]}});
    Ok((
        recipe,
        schedule,
        json!({"instructions":inv.instructions,"schema":schema}),
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut output = None;
    let mut approval = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                output = Some(PathBuf::from(args.get(i).context("--out needs a path")?));
            }
            "--execute-approval" => {
                i += 1;
                approval = Some(PathBuf::from(
                    args.get(i).context("Approval path required")?,
                ));
            }
            _ => anyhow::bail!(
                "Usage: mapping_extractor_benchmark [--out .cache/automatic-mapping/NAME] [--execute-approval FILE]"
            ),
        }
        i += 1;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let config = Config::from_env()?;
    let (mut recipe, schedule, prompt) = recipe(&root, &config)?;
    recipe["compiled_runner_sha256"] = json!(sha256(&fs::read(std::env::current_exe()?)?)?);
    let recipe_hash = sha256(&serde_json::to_vec(&recipe)?)?;
    let out = output.unwrap_or_else(|| {
        root.join(format!(
            ".cache/automatic-mapping/extractor-{}",
            Uuid::new_v4()
        ))
    });
    let out = safe_output(&root, &out, approval.is_some())?;
    if let Some(approval) = approval {
        let frozen: Value = serde_json::from_slice(&fs::read(out.join("recipe.json"))?)?;
        ensure!(
            frozen == recipe,
            "Recipe/code/config/pricing drifted; refresh the same authorized scope before execution"
        );
        let approved: Value = serde_json::from_slice(&fs::read(approval)?)?;
        validate_approval(&approved, &recipe_hash, &recipe)?;
        execute(&root, &out, config, &recipe, &schedule, &approved).await?;
    } else {
        fs::create_dir(&out).context("Dry run never overwrites an existing campaign directory")?;
        private_json(&out.join("recipe.json"), &recipe)?;
        private_json(&out.join("schedule.json"), &json!(schedule))?;
        private_json(&out.join("production-prompt-schema.json"), &prompt)?;
        private_json(
            &out.join("approval-template.json"),
            &json!({"approve_paid_calls":false,
            "recipe_sha256":recipe_hash,"pricing_sha256":recipe["pricing_sha256"],"maximum_calls":60,
            "maximum_output_tokens":MAX_OUTPUT,"budget_usd":"UNAPPROVED","authorization_reference":"UNAPPROVED"}),
        )?;
        private_json(
            &out.join("dry-run-receipt.json"),
            &json!({"status":"dry_run_only","recipe_sha256":recipe_hash,
            "created_at":chrono::Utc::now(),"network_calls":0,"model_calls":0,"database_connections":0,
            "database_writes":0,"policy_changes":0,"documents":schedule.len(),"partition_counts":{"development":42,"held_out":18},
            "maximum_cost_usd":recipe["limits"]["worst_case_usd"],"limits":recipe["limits"],
            "payload":recipe["payload"],"not_measured":recipe["evaluation"]["not_measured"],
            "execute_status":"Not authorized; requires separate explicit user budget approval bound to recipe and fresh pricing"}),
        )?;
    }
    println!(
        "{}",
        json!({"output":out,"recipe_sha256":recipe_hash,"mode":if out.join("outcomes.jsonl").exists(){"execution-recorded"}else{"dry-run"},"maximum_cost_usd":recipe["limits"]["worst_case_usd"]})
    );
    Ok(())
}
