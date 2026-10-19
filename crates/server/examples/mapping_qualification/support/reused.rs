//! A completed standalone60 receipt is diagnostics only. It never authorizes
//! a second source-facts call, or supplies reconciliation/native candidates.
use super::common;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
pub const KIND: &str = "remaining-reconciliation-vectors-1";
const GRANTED_RECIPE: &str = "0db65e0d88161ae0e61778edbebd23a06174f0d7ec11401d68e2fd949a4822a0";
const COMPLETED_OUTCOMES: &str = "d95e51c107db34e5db3b2222e157be0c819f2bcd6f616c8717387459d847ce17";
const COMPLETED_EXECUTION: &str =
    "b397109e476f9eff75b5007dde2691f72793bd777a693ee4a3ba5e6c986f9c41";
const VERIFIED_CLEANUP: &str = "11ff2be39d2e633b03319caccebb01f5b3428280285fa1e27bb65ab88cf449af";

pub fn apply(root: &Path, recipe: &mut Value, directory: &Path) -> Result<()> {
    let directory = common::safe_output(root, directory, true)?;
    let mut files = BTreeMap::new();
    for name in [
        "recipe.json",
        "schedule.json",
        "outcomes.jsonl",
        "execution-receipt.json",
        "cleanup-receipt.json",
    ] {
        let bytes = fs::read(directory.join(name))
            .with_context(|| format!("Completed60 evidence missing: {name}"))?;
        files.insert(name, bytes);
    }
    let prior: Value = serde_json::from_slice(&files["recipe.json"])?;
    let receipt: Value = serde_json::from_slice(&files["execution-receipt.json"])?;
    let cleanup: Value = serde_json::from_slice(&files["cleanup-receipt.json"])?;
    let schedule: Vec<Value> = serde_json::from_slice(&files["schedule.json"])?;
    let outcomes: Vec<Value> = std::str::from_utf8(&files["outcomes.jsonl"])?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;
    ensure!(
        common::sha256(&serde_json::to_vec(&prior)?)? == GRANTED_RECIPE
            && common::sha256(&files["outcomes.jsonl"])? == COMPLETED_OUTCOMES
            && common::sha256(&files["execution-receipt.json"])? == COMPLETED_EXECUTION
            && common::sha256(&files["cleanup-receipt.json"])? == VERIFIED_CLEANUP,
        "Only the exact immutable granted/completed60 baseline may be reused; never substitute or rerun"
    );
    ensure!(
        prior["model"] == recipe["config"]["text_model"]
            && prior["endpoint"] == recipe["config"]["endpoint"],
        "Remaining campaign must preserve the installed approved model/provider choice"
    );
    // Later reconciliation schema/adapter corrections do not reinterpret or
    // invalidate this historical failed diagnostic source-facts baseline.
    validate(
        &prior,
        &prior,
        &receipt,
        &cleanup,
        &schedule,
        &outcomes,
        &files["outcomes.jsonl"],
    )?;
    let mut hashes = serde_json::Map::new();
    for (name, bytes) in files {
        hashes.insert(name.into(), json!(common::sha256(&bytes)?));
    }
    let recon = recipe["limits"]["reconciliation_requests"]
        .as_u64()
        .context("Reconciliation cap")?;
    let embeds = recipe["limits"]["embedding_requests"]
        .as_u64()
        .context("Embedding cap")?;
    let total = recipe["limits"]["reconciliation_nano_usd"]
        .as_u64()
        .context("Reconciliation maximum")?
        .checked_add(
            recipe["limits"]["optional_vectors_nano_usd"]
                .as_u64()
                .context("Embedding maximum")?,
        )
        .context("Cost overflow")?;
    ensure!(
        recon == 78 && embeds == 33,
        "This remaining scope is exactly78reconciliation/33maximumembedding requests"
    );
    recipe["campaign_kind"] = json!(KIND);
    recipe["kind"] = json!("remaining-reconciliation-optional-vectors-proposal");
    recipe["mode"] = json!("remaining_only_no_source_facts_import_or_call");
    recipe["limits"]["source_facts_requests"] = json!(0);
    recipe["limits"]["source_facts_nano_usd"] = json!(0);
    recipe["limits"]["text_requests"] = json!(recon);
    recipe["limits"]["requests"] = json!(recon + embeds);
    recipe["limits"]["total_nano_usd"] = json!(total);
    recipe["limits"]["total_usd"] = json!(common::dollars(total));
    recipe["reused_source_facts"] = json!({"directory":directory,"artifact_sha256":hashes,"documents":60,"model":prior["model"],"scope":"immutable completed60diagnostics only; no import/call/candidate reuse","quality_or_production_qualification":false,"granted_recipe_sha256":GRANTED_RECIPE,"historical_prompt_label":prior["prompt_label"],"historical_schema_label":prior["schema_label"],"historical_adapter_revision":prior["adapter_revision"],"current_source_facts_component_executes":false,"cleanup_status":cleanup["status"]});
    recipe["paid_guard"]["execution_entrypoint"] = json!(
        "mapping_qualification_runner --reuse-source-facts COMPLETED60 --out FROZEN --execute-approval NEW_REMAINING_APPROVAL"
    );
    Ok(())
}
fn validate(
    current: &Value,
    prior: &Value,
    receipt: &Value,
    cleanup: &Value,
    schedule: &[Value],
    outcomes: &[Value],
    bytes: &[u8],
) -> Result<()> {
    ensure!(
        prior["kind"] == "production-extractor-only" && prior["limits"]["calls"] == 60,
        "Only original60 source-facts scope may be reused"
    );
    for field in [
        "model",
        "endpoint",
        "prompt_label",
        "schema_label",
        "adapter_revision",
        "prompt_sha256",
        "schema_sha256",
        "corpora",
        "schedule_sha256",
    ] {
        ensure!(
            prior[field] == current[field] && !prior[field].is_null(),
            "Completed60 scope differs: {field}"
        );
    }
    ensure!(
        receipt["status"] == "completed"
            && receipt["outcome_rows"] == 60
            && receipt["corpus_documents"] == 60
            && receipt["retries"] == 0
            && receipt["model"] == prior["model"],
        "Completed original60 document receipt required; never resume failed/uncertain runs"
    );
    ensure!(
        receipt["recipe_sha256"] == common::sha256(&serde_json::to_vec(prior)?)?
            && receipt["outcomes_sha256"] == common::sha256(bytes)?
            && receipt["compiled_runner_sha256"] == prior["compiled_runner_sha256"]
            && prior["compiled_runner_sha256"]
                .as_str()
                .is_some_and(|s| s.len() == 64),
        "Completed60 recipe/outcomes/executable binding mismatch"
    );
    ensure!(
        cleanup["status"] == "cleaned"
            && cleanup["database_absence_verified"] == true
            && cleanup["drop_without_force"] == true,
        "Owned original60 cleanup proof required; retained data needs explicit independent review"
    );
    ensure!(
        schedule.len() == 60
            && outcomes.len() == 60
            && common::sha256(&serde_json::to_vec(schedule)?)? == prior["schedule_sha256"],
        "All60 frozen source-facts denominators required"
    );
    let keys = |rows: &[Value]| -> Result<BTreeMap<(String, String), (Value, Value)>> {
        let mut map = BTreeMap::new();
        for row in rows {
            let key = (
                row["corpus"].as_str().context("Corpus")?.to_owned(),
                row["id"].as_str().context("Document ID")?.to_owned(),
            );
            ensure!(
                map.insert(key, (row["text_sha256"].clone(), row["partition"].clone()))
                    .is_none(),
                "Duplicate original60 outcome"
            );
        }
        Ok(map)
    };
    ensure!(
        keys(schedule)? == keys(outcomes)?,
        "Original60 outcome scope/hash/partition mismatch"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Value, Value, Value, Vec<Value>, Vec<Value>, Vec<u8>) {
        let schedule:Vec<Value>=(0..60).map(|id|json!({"corpus":"synthetic","id":id.to_string(),"partition":"development","text_sha256":"a".repeat(64)})).collect();
        let outcomes = schedule.clone();
        let bytes = outcomes
            .iter()
            .map(|r| r.to_string() + "\n")
            .collect::<String>()
            .into_bytes();
        let prior = json!({"kind":"production-extractor-only","limits":{"calls":60},"model":"standing","endpoint":"https://api.openai.com/v1","prompt_label":"source-facts-2","schema_label":"source-facts-2","adapter_revision":"fixed","prompt_sha256":"prompt","schema_sha256":"schema","corpora":[],"schedule_sha256":common::sha256(&serde_json::to_vec(&schedule).unwrap()).unwrap(),"compiled_runner_sha256":"b".repeat(64)});
        let receipt = json!({"status":"completed","outcome_rows":60,"corpus_documents":60,"retries":0,"model":"standing","recipe_sha256":common::sha256(&serde_json::to_vec(&prior).unwrap()).unwrap(),"outcomes_sha256":common::sha256(&bytes).unwrap(),"compiled_runner_sha256":prior["compiled_runner_sha256"]});
        let cleanup =
            json!({"status":"cleaned","database_absence_verified":true,"drop_without_force":true});
        (prior, receipt, cleanup, schedule, outcomes, bytes)
    }
    #[test]
    fn immutable60_receipt_cannot_be_rebound_to_duplicate_or_different_scope() {
        let (prior, receipt, cleanup, schedule, mut outcomes, bytes) = fixture();
        assert!(
            validate(
                &prior, &prior, &receipt, &cleanup, &schedule, &outcomes, &bytes
            )
            .is_ok()
        );
        outcomes[59] = outcomes[0].clone();
        assert!(
            validate(
                &prior, &prior, &receipt, &cleanup, &schedule, &outcomes, &bytes
            )
            .is_err()
        );
        let (prior, receipt, cleanup, schedule, outcomes, bytes) = fixture();
        let mut different = prior.clone();
        different["prompt_sha256"] = json!("different");
        assert!(
            validate(
                &different, &prior, &receipt, &cleanup, &schedule, &outcomes, &bytes
            )
            .is_err()
        );
        let mut changed = bytes;
        changed.push(b' ');
        assert!(
            validate(
                &prior, &prior, &receipt, &cleanup, &schedule, &outcomes, &changed
            )
            .is_err()
        );
    }
    #[test]
    fn uncertain_or_unverified_cleanup_never_passes_as_completed_diagnostics() {
        let (prior, mut receipt, mut cleanup, schedule, outcomes, bytes) = fixture();
        receipt["status"] = json!("stopped_review_required");
        assert!(
            validate(
                &prior, &prior, &receipt, &cleanup, &schedule, &outcomes, &bytes
            )
            .is_err()
        );
        receipt["status"] = json!("completed");
        cleanup["database_absence_verified"] = json!(false);
        assert!(
            validate(
                &prior, &prior, &receipt, &cleanup, &schedule, &outcomes, &bytes
            )
            .is_err()
        );
    }
}
