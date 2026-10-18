//! No-call freeze of both actual extraction prompts and an optional vector arm.
//! No paid execution entrypoint is compiled into this proposal preparer.
#[allow(dead_code)]
#[path = "mapping_qualification/support/ledger.rs"]
mod ledger;
#[allow(dead_code)]
#[path = "mapping_extractor_benchmark.rs"]
pub(crate) mod source_facts;
use source_facts::support;

use anyhow::{Context, Result, ensure};
use recollect_protocol::LearningRun;
use recollect_server::{autonomous, config::Config, model_gateway as gateway};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use support::{FRAMING, MAX_BODY, MAX_OUTPUT, private_json, sha256};
use uuid::Uuid;

const CONTROLS: &str = "docs/research/automatic-reconciliation-native-controls-v3-2026-10-18.json";
pub(crate) const FRESH: &str = "docs/research/graph-organization-fresh-heldout-v1-2026-10-18.json";
pub(crate) const FRESH_HASH: &str =
    "a01e61e95454d02aec7d4446530a13d16900dbe50550495c7be052f89bea3d67";
const EMBEDDING_PRICE: &str = "docs/research/mapping-embedding-pricing-2026-10-18.json";

/// Setup-only title and declared scope never enter input0.data or provider gold.
pub(crate) fn schedule(root: &Path, config: &Config) -> Result<Vec<Value>> {
    let (_, mut schedule, _) = source_facts::recipe(root, config)?;
    let extra: Value = serde_json::from_slice(&fs::read(
        root.join("docs/research/graph-organization-adversarial-fixtures-2026-10-18.json"),
    )?)?;
    for item in schedule
        .iter_mut()
        .filter(|d| d["corpus"] == "adversarial20")
    {
        let doc = extra["documents"]
            .as_array()
            .context("Frozen old20")?
            .iter()
            .find(|d| d["id"] == item["id"])
            .context("Frozen old20 ID")?;
        item["scope"] = doc["scope"].clone();
        item["title"] = doc
            .get("title")
            .cloned()
            .unwrap_or_else(|| json!(format!("Synthetic source {}", doc["id"].as_str().unwrap())));
    }
    let bytes = fs::read(root.join(FRESH))?;
    ensure!(
        sha256(&bytes)? == FRESH_HASH,
        "Fresh untouched held-out corpus changed"
    );
    let fresh: Value = serde_json::from_slice(&bytes)?;
    let docs = fresh["documents"]
        .as_array()
        .context("Fresh held-out docs")?;
    ensure!(
        docs.len() == 12
            && docs
                .iter()
                .all(|d| d["partition"] == "held_out" && d["language"] == "en"),
        "Preserve12fresh English held-out inputs"
    );
    for d in docs {
        let text = d["text"].as_str().context("Fresh exact text")?;
        schedule.push(json!({"corpus":"fresh12","id":d["id"],"partition":d["partition"],"title":d["title"],"scope":d["scope"],"text":text,"text_sha256":sha256(text.as_bytes())?}));
    }
    Ok(schedule)
}

/// Same UTF-8 and newline boundaries as canonical evidence::process. A later
/// executor must compare this frozen manifest with actual source_chunks first.
fn chunks(text: &str) -> Vec<(usize, usize)> {
    let mut chunks = vec![];
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + 4096).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end < text.len()
            && let Some(relative) = text[start..end].rfind('\n')
            && relative >= 2048
        {
            end = start + relative + 1;
        }
        chunks.push((start, end));
        start = end;
    }
    chunks
}
pub(crate) fn run(source: Uuid, offered: Vec<Uuid>) -> Result<LearningRun> {
    Ok(serde_json::from_value(
        json!({"id":Uuid::nil(),"brain_id":Uuid::nil(),"source_version_id":source,
      "policy_id":Uuid::nil(),"actor_id":Uuid::nil(),"device_id":null,"operation_id":null,
      "selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"manifest_revision_id":null,
      "automatic":true,"job_id":Uuid::nil(),"state":"queued","error_code":null,"request_id":null,
      "claim_ids":[],"proposed":0,"accepted":0,"blocked":0,"conflicting":0,"reused":0,"revised":0,"retired":0,
      "reconciliation_inputs":offered,"automatic_attempt":0,"retry_of":null,
      "created_at":"2026-10-18T00:00:00Z","finished_at":null}),
    )?)
}
fn invocation(run: &LearningRun) -> Result<Value> {
    let inv = autonomous::invocation(run);
    let gateway::Format::Json { name, schema } = inv.format else {
        anyhow::bail!("Reconciliation changed from JSON");
    };
    Ok(
        json!({"prompt_label":inv.prompt_label,"schema_label":inv.schema_label,"instructions":inv.instructions,
      "format_name":name,"schema":schema,"canonical_inputs":inv.inputs,
      "instructions_sha256":sha256(inv.instructions.as_bytes())?,"schema_sha256":sha256(&serde_json::to_vec(&schema)?)?}),
    )
}
fn price_embedding(price: &Value, config: &Config, calls: u64) -> Result<u64> {
    ensure!(
        price["provider"] == "openai"
            && price["model"] == config.models.embedding_model
            && price["endpoint"] == config.models.endpoint,
        "Fresh exact-model embedding price missing; do not substitute"
    );
    let verified = chrono::DateTime::parse_from_rfc3339(
        price["verified_at"]
            .as_str()
            .context("Price verification time")?,
    )?;
    let age = chrono::Utc::now()
        .signed_duration_since(verified)
        .num_seconds();
    ensure!(
        (-300..=86400).contains(&age) && price["valid_for_hours"].as_u64().is_some_and(|v| v <= 24),
        "Embedding pricing is stale; re-verify and re-freeze"
    );
    let url = reqwest::Url::parse(
        price["source_url"]
            .as_str()
            .context("Official embedding source")?,
    )?;
    ensure!(
        url.scheme() == "https"
            && url.host_str() == Some("developers.openai.com")
            && url.path().ends_with(&config.models.embedding_model),
        "Embedding source is not exact-model official pricing"
    );
    ensure!(
        calls <= 33,
        "Embedding request bound exceeds reviewed proposal"
    );
    (MAX_BODY + FRAMING)
        .checked_mul(
            price["input_nano_usd_per_token"]
                .as_u64()
                .context("Embedding rate")?,
        )
        .and_then(|p| p.checked_mul(calls))
        .context("Embedding cost overflow")
}
pub(crate) fn freeze(root: &Path, config: &Config) -> Result<(Value, Value, Value)> {
    let (facts, _, facts_prompt) = source_facts::recipe(root, config)?;
    let schedule = schedule(root, config)?;
    let control_bytes = fs::read(root.join(CONTROLS))
        .context("Independent six-control fixture must be frozen first")?;
    ensure!(
        sha256(&control_bytes)?
            == "d15e0171a691e1656dcd4e3955eca39859c6b73a19b8257402c692e69101cf81",
        "Independent control file changed after readback; re-review before freezing"
    );
    let controls: Value = serde_json::from_slice(&control_bytes)?;
    let cases = controls["cases"]
        .as_array()
        .context("Missing independent control cases")?;
    ensure!(cases.len() == 6, "Expected six independent controls");
    ensure!(
        cases
            .iter()
            .filter(|c| c["partition"] == "development")
            .count()
            == 3
            && cases
                .iter()
                .filter(|c| c["partition"] == "held_out")
                .count()
                == 3,
        "Preserve independent3development/3heldout split"
    );
    let no_offers = invocation(&run(Uuid::nil(), vec![])?)?;
    let mut invocations = vec![];
    for case in cases {
        let memories = case["canonical_memories"]
            .as_array()
            .context("Control canonical memories missing")?;
        let offered = case["offered_memory_symbols"]
            .as_array()
            .context("Control offered memories missing")?;
        let mut ids = vec![];
        let mut payload = vec![];
        for symbol in offered {
            let memory = memories
                .iter()
                .find(|m| m["symbol"] == *symbol)
                .context("Offered control memory missing")?;
            ensure!(
                memory["eligible_to_offer"] == true,
                "Ineligible/manual memory cannot be offered"
            );
            ids.push(
                memory["revision_id"]
                    .as_str()
                    .context("Control revision missing")?
                    .parse()?,
            );
            // Only canonical setup text/identity is admissible; gold and labels
            // describing the test expectation are deliberately not copied.
            payload.push(json!({"symbol":symbol,"revision_id":memory["revision_id"],"claim_id":memory["claim_id"],
              "subject":memory["subject"],"predicate":memory["predicate"],"value":memory["value"],
              "origin":memory.get("origin").unwrap_or(&controls["common"]["machine_memory_defaults"]["origin"]),"support":memory["support"],"lineage":memory["lineage"]}));
        }
        let source = case["source"]["version_id"]
            .as_str()
            .context("Control source version missing")?
            .parse()?;
        invocations.push(
            json!({"id":case["id"],"partition":case["partition"],"source":case["source"],
          "offered_canonical_setup":payload,"actual_invocation":invocation(&run(source,ids)?)?}),
        );
    }
    let mut manifest = vec![];
    for doc in schedule.iter().filter(|d| d["corpus"] != "original40") {
        let text = doc["text"].as_str().context("Frozen source text missing")?;
        for (ordinal, (start, end)) in chunks(text).into_iter().enumerate() {
            manifest.push(json!({"document_id":doc["id"],"partition":doc["partition"],"ordinal":ordinal,
              "byte_start":start,"byte_end":end,"chunk_sha256":sha256(text[start..end].as_bytes())?}));
        }
    }
    ensure!(
        manifest.len() == 33,
        "Unchanged20 plus fresh12 must yield33full canonical chunks"
    );
    let mut preflights = vec![];
    for (name, path) in [
        ("adversarial20", "canonical-chunk-preflight.json"),
        ("fresh12", "canonical-fresh-chunk-preflight.json"),
    ] {
        let preflight: Value = serde_json::from_slice(
            &fs::read(root.join(".cache/automatic-mapping").join(path))
                .context("Actual full canonical chunk preflight required")?,
        )?;
        let ids: Vec<_> = schedule
            .iter()
            .filter(|d| d["corpus"] == name)
            .map(|d| d["id"].clone())
            .collect();
        let expected: Vec<_> = manifest
            .iter()
            .filter(|m| ids.contains(&m["document_id"]))
            .cloned()
            .collect();
        ensure!(
            preflight["status"] == "passed"
                && preflight["model_calls"] == 0
                && preflight["manifest"] == json!(expected),
            "Canonical complete chunk preflight changed"
        );
        ensure!(
            preflight["source_splitter_sha256"]
                == sha256(&fs::read(root.join("crates/server/src/evidence.rs"))?)?,
            "Source splitter changed since native preflight"
        );
        preflights.push(preflight);
    }
    let price_bytes = fs::read(root.join(EMBEDDING_PRICE))?;
    let price: Value = serde_json::from_slice(&price_bytes)?;
    let text60 = support::worst_case(&facts["pricing"], 60)?;
    let recon_count = schedule.len() as u64 + cases.len() as u64;
    let text_recon = support::worst_case(&facts["pricing"], 1)?
        .checked_mul(recon_count)
        .context("Reconciliation cost overflow")?;
    let text_requests = 60 + recon_count;
    let request_count = text_requests + manifest.len() as u64;
    let vectors = price_embedding(&price, config, manifest.len() as u64)?;
    let total = text60
        .checked_add(text_recon)
        .and_then(|v| v.checked_add(vectors))
        .context("Combined budget overflow")?;
    let mut code = serde_json::Map::new();
    for path in [
        "crates/server/examples/mapping_qualification_proposal.rs",
        "crates/server/src/autonomous.rs",
        "crates/server/src/learning.rs",
        "crates/server/src/evidence.rs",
        "crates/server/src/semantic.rs",
        "crates/server/src/semantic/queue.rs",
        "crates/server/src/semantic/work.rs",
        "crates/server/src/model_gateway.rs",
        "crates/server/src/knowledge_mapping/production.rs",
        "crates/server/src/knowledge_mapping/producer.rs",
        "scripts/score-extractor.py",
        "crates/server/examples/mapping_qualification/support/ledger.rs",
    ] {
        code.insert(path.into(), json!(sha256(&fs::read(root.join(path))?)?));
    }
    let recipe = json!({"version":1,"kind":"combined-extractor-reconciliation-optional-vectors-proposal",
      "mode":"freeze_only_no_execution_entrypoint","config":{"provider":"openai","text_model":config.models.text_model,"embedding_model":config.models.embedding_model,"embedding_dimensions":config.models.embedding_dimensions,"endpoint":config.models.endpoint},
      "source_facts":facts,"source_facts_recipe_sha256":sha256(&serde_json::to_vec(&facts)?)?,
      "reconciliation":{"prompt_label":autonomous::PROMPT,"source_corpora_unchanged":true,"documents":recon_count,"partitions":{"development":45,"held_out":33},
        "source_schedule_sha256":sha256(&serde_json::to_vec(&schedule)?)?,"corpus_documents":schedule.len(),"independent_controls":6,"fresh_heldout":{"path":FRESH,"sha256":FRESH_HASH,"documents":12,"endpoint_annotations":"NOT_ANNOTATED; no alias/relation negatives inferred","development_selection":"Never tune on these inputs/gold/outputs"},"controls_path":CONTROLS,"controls_sha256":sha256(&control_bytes)?,
        "actual_no_offer_invocation":no_offers,"control_invocations_sha256":sha256(&serde_json::to_vec(&invocations)?)?,
        "canonical_setup":"Every control in an independent owned Brain/environment; current exact-lineage canonical machine memories through real targets()/memory::view; manual memories excluded. Verify all actual offered IDs and full gateway payload hashes before transmitting. Never serialize fixture/gold metadata into model inputs.",
        "prior_memory_provenance":"Synthetic canonical baseline, never a real measured provider output or qualification receipt. Setup implementation and independent proof required before execution.",
        "request_bounds":{"complete_request_bytes":MAX_BODY,"framing_input_tokens":FRAMING,"source_and_offered_input_bytes":16384,"output_tokens":MAX_OUTPUT,"retry":0}},
      "optional_vectors":{"documents":32,"source_chunks":manifest.len(),"maximum_requests":manifest.len(),"representation":"engineering-text-1","schema":"embedding-float-1","source_chunk_manifest_sha256":sha256(&serde_json::to_vec(&manifest)?)?,"canonical_preflight":preflights,
        "pricing_path":EMBEDDING_PRICE,"pricing_sha256":sha256(&price_bytes)?,"pricing":price,
        "preflight":"Actual capture processing and canonical semantic representation must match every frozen byte range/hash, preserve old20+fresh12 and both h06chunks; 33maxrequests, no queryembedding, truncate, silentdrop, syntheticvector or retry.",
        "comparison":"Same actual extracted canonical32sources; old20diagnostic190pairs and fresh12independent66pairs scored separately, retain exact vector/profile/request provenance; any missing/incompatible/failed vector stays visible; native grouping/vector scorer separately frozen before output.",
        "complete_request_bytes":MAX_BODY,"framing_input_tokens":FRAMING,"canonical_gateway_input_bytes":8000,"canonical_representation_bytes":6000},
      "limits":{"source_facts_requests":60,"reconciliation_requests":recon_count,"text_requests":text_requests,"embedding_requests":manifest.len(),"requests":request_count,"concurrent":1,"retries":0,"source_facts_nano_usd":text60,"reconciliation_nano_usd":text_recon,"optional_vectors_nano_usd":vectors,"total_nano_usd":total,"total_usd":support::dollars(total)},
      "paid_guard":{"approved":false,"original_60_call_approval_cannot_apply":true,"new_combined_recipe_pricing_corpus_chunk_hash_approval_required":true,"one_durable_campaign_lock":true,"one_common_cost_ledger":true,"reserve_worst_cost_before_each_attempt":true,"uncertain_stop_no_resume":true,"normal_data_or_policy_mutations":false,"provider_calls_during_freeze":0,"answer_judging":false},
      "code_sha256":code,"acceptance":{"source40":"Span/type only; no endpoint annotation is not empty gold","adversarial20":"Exact spans/types and independently annotated endpoint/disposition/identity/topic metrics remain separate","fresh12":"Exact46manualmentions scored independently; endpointgold not annotated; fresh66groupingpairs require separate actual-candidate scorer", "controls6":"Every required/forbidden replace/retire/reuse/new-claim outcome including manual exclusion remains in denominator","unsupported_language":"Observe actual output; no gold-driven skip or manufactured unsupported success","not_qualified_by_proposal":["real provider extraction quality","canonical mutation behavior","topic or label usefulness","retained vector baseline","release-owned producer or recipe activation"]}});
    Ok((
        recipe,
        json!({"source_facts":facts_prompt,"reconciliation_no_offer":no_offers,"controls":invocations}),
        json!(manifest),
    ))
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 2 && args[0] == "--out",
        "Usage: mapping_qualification_proposal --out .cache/automatic-mapping/extractor-combined-NAME; no execution option exists"
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let config = Config::from_env()?;
    let (recipe, prompts, manifest) = freeze(&root, &config)?;
    let out = support::safe_output(&root, &PathBuf::from(&args[1]), false)?;
    fs::create_dir(&out).context("Never overwrite a frozen proposal")?;
    private_json(&out.join("recipe.json"), &recipe)?;
    private_json(&out.join("production-prompts-schemas.json"), &prompts)?;
    private_json(&out.join("source-chunk-manifest.json"), &manifest)?;
    private_json(
        &out.join("dry-run-receipt.json"),
        &json!({"status":"no_call_proposal_only","recipe_sha256":sha256(&serde_json::to_vec(&recipe)?)?,"created_at":chrono::Utc::now(),"model_calls":0,"database_connections":0,"database_writes":0,"normal_policy_changes":0,"limits":recipe["limits"],"canonical_chunk_preflight":"passed_actual32documents33chunks","control_setup_execution":"separate_actual_native_receipt","paid_execution":"not_authorized_no_entrypoint"}),
    )?;
    println!(
        "Frozen proposal only; 0model calls,0DB connections. Maximum {}USD /{}requests. Execution remains unavailable.",
        recipe["limits"]["total_usd"].as_str().unwrap(),
        recipe["limits"]["requests"]
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_chunk_boundaries_preserve_unicode_newlines_and_tail() {
        let text = format!("{}{}Z", "x".repeat(4095), "ü");
        assert_eq!(chunks(&text), vec![(0, 4095), (4095, 4098)]);
        let text = format!("{}\n{}", "x".repeat(3000), "y".repeat(3000));
        assert_eq!(chunks(&text), vec![(0, 3001), (3001, 6001)]);
    }
    #[test]
    fn actual_invocation_cannot_offer_an_unlisted_revision() {
        let id = Uuid::from_u128(1);
        let inv = invocation(&run(Uuid::nil(), vec![id]).unwrap()).unwrap();
        assert_eq!(inv["prompt_label"], autonomous::PROMPT);
        assert_eq!(
            inv["schema"]["properties"]["claims"]["items"]["properties"]["replaces_revision"]["enum"],
            json!([null, id])
        );
        assert_eq!(
            inv["schema"]["properties"]["retirements"]["items"]["properties"]["revision_id"]["enum"],
            json!([id])
        );
    }
}
