//! Default: freeze only. Paid execution requires a fresh exact-scope approval.
#[path = "mapping_qualification/support/evaluation.rs"]
mod evaluation;
#[path = "mapping_qualification/support/ledger.rs"]
mod ledger;
#[path = "mapping_qualification/support/native.rs"]
mod native;
#[allow(dead_code)]
#[path = "mapping_qualification_proposal.rs"]
mod proposal;
#[allow(dead_code)]
#[path = "mapping_qualification/support/reconciliation.rs"]
mod reconciliation;
#[path = "mapping_qualification/support/reused.rs"]
mod reused;
#[path = "mapping_qualification/support/runner.rs"]
mod runner;
#[path = "mapping_qualification/support/scopes.rs"]
mod scopes;
use anyhow::{Context, Result, ensure};
use proposal::source_facts::support as common;
use recollect_server::{config::Config, db};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const BLIND_LABEL_RUBRIC: &str =
    "docs/research/graph-organization-blind-label-rubric-v1-2026-10-18.json";
const BLIND_LABEL_RUBRIC_SHA256: &str =
    "bb45993468ed83f0d92ca438b7fef2f4058d13c755def32529d23c481e4f2a02";

fn current(root: &Path, config: &Config) -> Result<(Value, Value, Value, Vec<Value>, Value)> {
    let (mut recipe, prompts, chunks) = proposal::freeze(root, config)?;
    let bytes = fs::read(root.join(reconciliation::CONTROLS_PATH))?;
    ensure!(
        common::sha256(&bytes)? == reconciliation::CONTROLS_SHA256,
        "Reviewed native controls changed"
    );
    let controls: Value = serde_json::from_slice(&bytes)?;
    ensure!(
        recipe["reconciliation"]["controls_sha256"] == reconciliation::CONTROLS_SHA256,
        "Proposal/nativecontrols revision mismatch"
    );
    let receipt: Value = serde_json::from_slice(
        &fs::read(root.join(".cache/automatic-mapping/reconciliation-canonical-preflight.json"))
            .context("Actual six-control canonical preflight required before final freeze")?,
    )?;
    ensure!(
        receipt["controls_sha256"] == reconciliation::CONTROLS_SHA256
            && receipt["eligible_machine_memories_offered"] == 5
            && receipt["reviewed_manual_memories_offered"] == 0
            && receipt["paid_requests"] == 0
            && receipt["actual_worker_preview_body_equality"] == true,
        "Canonical control selector/body preflight failed or changed"
    );
    recipe["campaign_kind"] = json!("combined-source-facts-reconciliation-vectors-1");
    recipe["mode"] = json!("guarded_combined_extraction_only_with_optional_canonical_vectors");
    recipe["evaluation"] = json!({"mode":"actual_unpublished_candidates_with_retained_vectors","arm_definitions":evaluation::arm_definitions(),"expected_gds_version":"2026.08.1","development_grid":{"neighbors":[3,5,8],"minimum_cosine":[0.65,0.75,0.85],"vector_scale":[0.25,0.5,1.0]},"selection_rule":"precision>=0.95; maximizeF1,coverage; lexicographically smallest tuple","selection_scope":"original12development_only; observed8heldout_diagnostic_only; fresh12once_after_durable_selection","missing_features":"completecanonicalcohort_and_allfailed_denominators","provider_calls":0,"raw_vectors_exported":false,"production_recipe_qualified":false});
    recipe["evaluation"]["observation_policy"] = json!({"all_outcomes":"Retain all requested documents and controls, including rejected, absent, failed, uncertain and unattempted rows; no rescued predictions or denominator pruning","development_only":"Select only from the declared27tuple grid using original12development documents; preserve every trial and failed selection","fresh_heldout":"Never inspect fresh12gold or provider outputs for implementation or parameter selection; compare once only after durable successful development selection","after_failure":"No source-facts rerun, retry, model substitution, relabeling, prompt revision or additional tuning within this frozen campaign; changed recipes need independent untouched evidence and new scope authorization","qualified_or_activated":false});
    let rubric_bytes = fs::read(root.join(BLIND_LABEL_RUBRIC))?;
    ensure!(
        common::sha256(&rubric_bytes)? == BLIND_LABEL_RUBRIC_SHA256,
        "Frozen independent blind label rubric changed; no campaign may be admitted"
    );
    let rubric: Value = serde_json::from_slice(&rubric_bytes)?;
    recipe["evaluation"]["blind_label_review"] = json!({"path":BLIND_LABEL_RUBRIC,"sha256":BLIND_LABEL_RUBRIC_SHA256,"rubric":rubric,"status":"not_measured","automated_provenance_is_not_semantic_usefulness":true,"execution":"Separate independent blind review; no provider call or paid judge","activation":false});
    recipe["evaluation"]["native_runtime"] = json!({"configured_url":config.neo4j_url,"required_url":"http://127.0.0.1:57474","required_daemon":"local_unix_socket","compose_project":"recollect","compose_service":"neo4j","working_directory":root,"compose_file":root.join("compose.yaml"),"runtime_preflight":"exactDockerownership_binding_running_and_actualGDS_version_before_anyproviderrequest","credentials_recorded":false});
    recipe["compiled_runner_sha256"] = json!(common::sha256(&fs::read(std::env::current_exe()?)?)?);
    recipe["reconciliation"]["canonical_selector_preflight"] = receipt;
    recipe["reconciliation"]["prior_memory_provenance"] = json!(
        "Actual canonical learning+independent support with controlled loopback earlier outputs; manual prior through actual browser review; no prior model history insertion"
    );
    recipe["paid_guard"]["execution_entrypoint"] =
        json!("mapping_qualification_runner --out FROZEN --execute-approval NEW_COMBINED_APPROVAL");
    for path in [
        "crates/server/examples/mapping_qualification_runner.rs",
        "crates/server/examples/mapping_qualification/support/runner.rs",
        "crates/server/examples/mapping_qualification/support/scopes.rs",
        "crates/server/examples/mapping_qualification/support/ledger.rs",
        "crates/server/examples/mapping_qualification/support/reused.rs",
        "crates/server/examples/mapping_qualification/support/evaluation.rs",
        "crates/server/examples/mapping_qualification/support/evaluation_failure.rs",
        "crates/server/examples/mapping_qualification/support/native.rs",
        "crates/server/examples/mapping_qualification/support/reconciliation.rs",
        "scripts/score-reconciliation-controls.py",
        "scripts/score-real-topics.py",
        BLIND_LABEL_RUBRIC,
        "crates/server/src/graph/analytics/shadow.rs",
        "crates/server/src/graph/analytics/shadow/groups.rs",
        "crates/server/src/graph/queue.rs",
        "crates/protocol/src/graph.rs",
        "crates/server/src/knowledge_mapping/source_features.rs",
        "crates/server/src/knowledge_mapping/affinity.rs",
        "crates/server/src/knowledge_mapping.rs",
        "crates/server/src/knowledge_mapping/manual_intent.rs",
        "crates/server/src/knowledge_mapping/organization.rs",
        "crates/server/src/knowledge_mapping/organization/sources.rs",
        "crates/server/src/privacy.rs",
        "crates/server/src/model_gateway.rs",
        "crates/server/src/semantic.rs",
        "crates/server/src/semantic/work.rs",
        "crates/server/src/db.rs",
        "compose.yaml",
        "infra/product/compose.yaml",
        "scripts/docker.sh",
    ] {
        recipe["code_sha256"][path] = json!(common::sha256(&fs::read(root.join(path))?)?);
    }
    let mut migrations = serde_json::Map::new();
    for (name, sql) in db::MIGRATIONS {
        migrations.insert((*name).into(), json!(common::sha256(sql.as_bytes())?));
    }
    recipe["migration_sha256"] = json!(migrations);
    let schedule = proposal::schedule(root, config)?;
    Ok((recipe, prompts, chunks, schedule, controls))
}
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut output = None;
    let mut approval = None;
    let mut reuse = None;
    let mut n = 0;
    while n < args.len() {
        match args[n].as_str() {
            "--out" => {
                n += 1;
                output = Some(PathBuf::from(args.get(n).context("Output path missing")?));
            }
            "--reuse-source-facts" => {
                n += 1;
                reuse = Some(PathBuf::from(
                    args.get(n).context("Completed60 directory missing")?,
                ));
            }
            "--execute-approval" => {
                n += 1;
                approval = Some(PathBuf::from(
                    args.get(n).context("Combined approval path missing")?,
                ));
            }
            _ => anyhow::bail!(
                "Usage: mapping_qualification_runner --out .cache/automatic-mapping/extractor-combined-NAME [--reuse-source-facts COMPLETED60] [--execute-approval NEW_SCOPE_APPROVAL]"
            ),
        };
        n += 1;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let config = Config::from_env()?;
    let (mut recipe, prompts, chunks, schedule, controls) = current(&root, &config)?;
    if let Some(directory) = reuse {
        reused::apply(&root, &mut recipe, &directory)?;
    }
    let hash = common::sha256(&serde_json::to_vec(&recipe)?)?;
    let out = common::safe_output(
        &root,
        &output.context("Explicit owned output directory required")?,
        approval.is_some(),
    )?;
    if let Some(path) = approval {
        let frozen: Value = serde_json::from_slice(&fs::read(out.join("recipe.json"))?)?;
        ensure!(
            frozen == recipe,
            "Code/config/pricing/corpora/canonical preflight drifted; re-freeze and obtain new approval"
        );
        let approved: Value = serde_json::from_slice(&fs::read(path)?)?;
        let budget = ledger::validate_approval(&approved, &recipe, &hash)?;
        ensure!(
            config.models.key.is_some() && config.models.endpoint == "https://api.openai.com/v1",
            "Current installed official provider credential required; no model/provider substitution"
        );
        let mut ledger = ledger::Ledger::begin(&out, &recipe, &hash, &approved, budget)?;
        if Box::pin(runner::execute(
            &out,
            config,
            &recipe,
            &schedule,
            &controls,
            &mut ledger,
        ))
        .await
        .is_err()
        {
            common::private_json(
                &out.join("failure-receipt.json"),
                &json!({"status":"stopped_review_required","ledger":ledger.summary(),"automatic_resume_or_retry":false,"normal_data_touched":false,"details":"Inspect only this campaign's owned metadata; no uncertain attempt may be replayed"}),
            )?;
            anyhow::bail!(
                "Owned campaign stopped; review its receipt. Automatic retry/resume is forbidden."
            );
        }
        println!(
            "Combined campaign finished. Independently inspect outcomes and retained synthetic vectors; no producer or organization recipe was qualified or activated."
        );
    } else {
        fs::create_dir(&out).context("Never overwrite a frozen combined campaign")?;
        common::private_json(&out.join("recipe.json"), &recipe)?;
        common::private_json(&out.join("production-prompts-schemas.json"), &prompts)?;
        common::private_json(&out.join("source-chunk-manifest.json"), &chunks)?;
        common::private_json(&out.join("schedule.json"), &json!(schedule))?;
        common::private_json(
            &out.join("approval-template.json"),
            &json!({"approve_paid_calls":false,"campaign_kind":recipe["campaign_kind"],"reused_source_facts_sha256":recipe["reused_source_facts"]["artifact_sha256"],"recipe_sha256":hash,"text_pricing_sha256":recipe["source_facts"]["pricing_sha256"],"embedding_pricing_sha256":recipe["optional_vectors"]["pricing_sha256"],"control_sha256":recipe["reconciliation"]["controls_sha256"],"chunk_manifest_sha256":recipe["optional_vectors"]["source_chunk_manifest_sha256"],"maximum_requests":recipe["limits"]["requests"],"maximum_source_facts_requests":recipe["limits"]["source_facts_requests"],"maximum_text_requests":recipe["limits"]["text_requests"],"maximum_embedding_requests":recipe["limits"]["embedding_requests"],"maximum_output_tokens":4096,"budget_usd":"UNAPPROVED","authorization_reference":"UNAPPROVED"}),
        )?;
        common::private_json(
            &out.join("dry-run-receipt.json"),
            &json!({"status":"dry_run_only","recipe_sha256":hash,"created_at":chrono::Utc::now(),"model_calls":0,"database_connections":0,"database_writes":0,"normal_policy_changes":0,"canonical_setup_preflight":"separate zero-paid native receipts required and bound","limits":recipe["limits"],"approval":"Not authorized; old60call approval cannot apply","not_measured":["actual extractor quality","real support assessment or current canonical learning mutation quality","real vector/native grouping baseline","profile qualification/activation"]}),
        )?;
        println!(
            "Frozen guarded combined campaign:{}maximumrequests,{}USDmaximum;0model calls/DBconnections. No paid approval.",
            recipe["limits"]["requests"],
            recipe["limits"]["total_usd"].as_str().unwrap()
        );
    }
    Ok(())
}

#[cfg(test)]
mod native_proof {
    use super::*;
    use axum::{Json, Router, extract::State, routing::post};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    #[derive(Default)]
    struct Wire {
        calls: AtomicUsize,
        embedding_rows: AtomicUsize,
    }
    async fn wire(State(wire): State<Arc<Wire>>, Json(body): Json<Value>) -> Json<Value> {
        wire.calls.fetch_add(1, Ordering::SeqCst);
        if body.get("dimensions").is_some() {
            let dimensions = body["dimensions"].as_u64().unwrap() as usize;
            let data: Vec<_> = body["input"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(n, _)| {
                    // Distinct finite nonzero basis per canonical chunk, across
                    // every batch. No corpus/gold/partition is inspected.
                    let coordinate = wire.embedding_rows.fetch_add(1, Ordering::SeqCst);
                    assert!(coordinate < dimensions);
                    let mut vector = vec![0.0f64; dimensions];
                    vector[coordinate] = 1.0;
                    json!({"index":n,"embedding":vector})
                })
                .collect();
            Json(
                json!({"model":body["model"],"data":data,"usage":{"prompt_tokens":10,"total_tokens":10}}),
            )
        } else {
            let mut output = json!({"claims":[],"mapping":null});
            if body["text"]["format"]["schema"]["properties"]
                .get("retirements")
                .is_some()
            {
                output["retirements"] = json!([]);
            }
            Json(
                json!({"model":body["model"],"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":output.to_string()}]}],"usage":{"input_tokens":10,"output_tokens":4,"total_tokens":14}}),
            )
        }
    }
    async fn controlled_executor_proof(remaining: bool) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let config = Config::from_env().unwrap();
        let (mut recipe, _, _, schedule, controls) = current(&root, &config).unwrap();
        if remaining {
            reused::apply(
                &root,
                &mut recipe,
                &root.join(".cache/automatic-mapping/extractor-approved60-2026-10-18"),
            )
            .unwrap();
            assert_eq!(recipe["limits"]["source_facts_requests"], 0);
            assert_eq!(recipe["limits"]["requests"], 111);
        }
        let calls = Arc::new(Wire::default());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        let app = Router::new()
            .route("/v1/responses", post(wire))
            .route("/v1/embeddings", post(wire))
            .with_state(calls.clone());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let mut fake = config.clone();
        fake.models.endpoint = endpoint;
        fake.models.key = Some("controlled-loopback-only".into());
        fake.models.openrouter_key = None;
        let out = root.join(format!(
            ".cache/automatic-mapping/extractor-{}-loopback-proof-{}",
            if remaining {
                "remaining111"
            } else {
                "combined"
            },
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&out).unwrap();
        let hash = common::sha256(&serde_json::to_vec(&recipe).unwrap()).unwrap();
        let synthetic_approval = json!({"approve_paid_calls":true,"campaign_kind":recipe["campaign_kind"],"reused_source_facts_sha256":recipe["reused_source_facts"]["artifact_sha256"],"recipe_sha256":hash,"text_pricing_sha256":recipe["source_facts"]["pricing_sha256"],"embedding_pricing_sha256":recipe["optional_vectors"]["pricing_sha256"],"control_sha256":recipe["reconciliation"]["controls_sha256"],"chunk_manifest_sha256":recipe["optional_vectors"]["source_chunk_manifest_sha256"],"maximum_requests":recipe["limits"]["requests"],"maximum_source_facts_requests":recipe["limits"]["source_facts_requests"],"maximum_text_requests":recipe["limits"]["text_requests"],"maximum_embedding_requests":recipe["limits"]["embedding_requests"],"maximum_output_tokens":4096,"budget_usd":"3.30","authorization_reference":"UNIT TEST ONLY; no external provider or paid authorization"});
        let budget = ledger::validate_approval(&synthetic_approval, &recipe, &hash).unwrap();
        let mut ledger =
            ledger::Ledger::begin(&out, &recipe, &hash, &synthetic_approval, budget).unwrap();
        Box::pin(runner::execute(
            &out,
            fake,
            &recipe,
            &schedule,
            &controls,
            &mut ledger,
        ))
        .await
        .expect("Controlled loopback executor failed; inspect only this owned fixture");
        assert_eq!(
            ledger.summary()["source_facts_attempts"],
            if remaining { 0 } else { 60 }
        );
        assert_eq!(
            ledger.summary()["reconciliation_attempts"],
            recipe["limits"]["reconciliation_requests"]
        );
        let embedded = ledger.summary()["embedding_attempts"].as_u64().unwrap();
        assert!((1..=33).contains(&embedded));
        assert_eq!(
            calls.calls.load(Ordering::SeqCst),
            recipe["limits"]["text_requests"].as_u64().unwrap() as usize + embedded as usize,
            "Every actual current gateway HTTP operation has one common ledger reservation"
        );
        let vectors: Value = serde_json::from_slice(
            &fs::read(out.join("canonical-vector-provenance.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(vectors["entries"].as_array().unwrap().len(), 33);
        assert!(
            vectors["entries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["state"] == "ready"
                    && e["embedding_available"] == true
                    && e["embedding_sha256"].is_string()
                    && e["truncated"] == false)
        );
        let recon_rows: Vec<Value> = fs::read_to_string(out.join("reconciliation/outcomes.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let vector_docs = vectors["documents"].as_array().unwrap();
        assert_eq!(vector_docs.len(), 32);
        for doc in vector_docs {
            let extraction = recon_rows
                .iter()
                .find(|r| r["corpus"] == doc["corpus"] && r["id"] == doc["document_id"])
                .unwrap();
            assert_eq!(doc["source_version_id"], extraction["source_version_id"]);
            assert_eq!(doc["brain_id"], extraction["brain_id"]);
            assert_eq!(doc["selection"], extraction["canonical_selection"]);
            assert_eq!(doc["extraction_request_id"], extraction["request"]["id"]);
            assert!(doc["extraction_request_id"].is_string());
        }
        assert_eq!(
            fs::read_dir(out.join("outgoing-bodies")).unwrap().count(),
            calls.calls.load(Ordering::SeqCst)
        );
        if remaining {
            assert!(
                !out.join("source-facts").exists(),
                "No repeated source-facts import, outcomes, or requests"
            );
            let execution: Value =
                serde_json::from_slice(&fs::read(out.join("execution-receipt.json")).unwrap())
                    .unwrap();
            assert_eq!(execution["source_facts_documents"], 0);
            assert_eq!(
                execution["reused_source_facts_diagnostics"]["granted_recipe_sha256"],
                "0db65e0d88161ae0e61778edbebd23a06174f0d7ec11401d68e2fd949a4822a0"
            );
        } else {
            assert_eq!(
                fs::read_to_string(out.join("source-facts/outcomes.jsonl"))
                    .unwrap()
                    .lines()
                    .count(),
                60
            );
        }
        assert_eq!(
            fs::read_to_string(out.join("reconciliation/outcomes.jsonl"))
                .unwrap()
                .lines()
                .count(),
            72
        );
        assert_eq!(
            fs::read_to_string(out.join("control-outcomes.jsonl"))
                .unwrap()
                .lines()
                .count(),
            6
        );
        assert_eq!(calls.embedding_rows.load(Ordering::SeqCst), 33);
        let selection: Value = serde_json::from_slice(
            &fs::read(out.join("real-organization/development-selection.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(selection["selected"], false);
        assert_eq!(selection["trials"], 27);
        assert!(!out.join("real-organization/fresh12-receipt.json").exists());
        assert!(
            !out.join("real-organization/diagnostic20-receipt.json")
                .exists()
        );
        let mut trials = 0;
        for entry in fs::read_dir(out.join("real-organization")).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("dev-") && name.ends_with("-score.json") {
                let score: Value =
                    serde_json::from_slice(&fs::read(entry.path()).unwrap()).unwrap();
                assert_eq!(score["baselines"][0]["documents"], 12);
                assert_eq!(score["baselines"][0]["pair_metrics"]["pairs"], 66);
                assert_eq!(score["baselines"][0]["assignment"]["assigned_annotated"], 0);
                assert_eq!(score["baselines"][0]["assignment"]["vector_available"], 12);
                trials += 1;
            }
        }
        assert_eq!(trials, 27);
        let owned: Value =
            serde_json::from_slice(&fs::read(out.join("owned-state.json")).unwrap()).unwrap();
        let database = owned["database"].as_str().unwrap();
        assert!(database.starts_with("recollect_extractor_"));
        let admin = std::env::var("DATABASE_ADMIN_URL").unwrap();
        let connection = db::pool(&common::database_url(&admin, "postgres").unwrap())
            .await
            .unwrap();
        let owner: Option<String> = sqlx::query_scalar(
            "SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=$1",
        )
        .bind(database)
        .fetch_one(&connection)
        .await
        .unwrap();
        assert_eq!(owner.as_deref(), Some(common::OWNER));
        let owned_admin = db::pool(&common::database_url(&admin, database).unwrap())
            .await
            .unwrap();
        let mut cleanup_config = config.clone();
        cleanup_config.database_url = common::database_url(&config.database_url, database).unwrap();
        cleanup_config.erasure_journal = out.join("state/erasure-journal").to_string_lossy().into();
        cleanup_config.erasure_mirror = None;
        cleanup_config.artifact_dir = out.join("state/artifacts").to_string_lossy().into();
        let cleanup_state = recollect_server::AppState::new(
            db::pool(&cleanup_config.database_url).await.unwrap(),
            cleanup_config,
        )
        .unwrap();
        let check_dir = out.join("native-cleanup-preflight");
        fs::create_dir(&check_dir).unwrap();
        let verified = native::Native::verify(&config, &root, &check_dir, "2026.08.1")
            .await
            .expect("Exact native ownership must remain valid before cleanup");
        let cleanup = native::cleanup_owned(&verified, &cleanup_state, &owned_admin, &out)
            .await
            .expect("Only the exact owned native artifacts may be cleaned");
        assert_eq!(cleanup["pending_own_journals"], 0);
        assert_eq!(cleanup["live_own_gds_graphs"], 0);
        assert_eq!(cleanup["remaining_own_native_rows"], 0);
        cleanup_state.pool.close().await;
        owned_admin.close().await;
        sqlx::query(&format!("DROP DATABASE {database}"))
            .execute(&connection)
            .await
            .unwrap();
        connection.close().await;
        fs::remove_dir_all(out.join("state")).unwrap();
        common::private_json(&root.join(format!(".cache/automatic-mapping/{}-loopback-proof-{}.json",if remaining { "remaining111" } else { "combined" },uuid::Uuid::new_v4())),&json!({"status":"passed","campaign_kind":recipe["campaign_kind"],"recipe_sha256":hash,"paid_calls":0,"external_provider_calls":0,"source_facts":if remaining {0}else{60},"reconciliation":78,"embedding_http_requests":embedded,"canonical_vector_entries":33,"current_calls_counted_once":true,"development_trials":27,"failed_selection_preserves12documents66pairs_each":true,"heldout_comparison_not_started":true,"own_journals_gds_and_native_rows_remaining":0,"owned_database_removed":true,"normal_data_copied":false,"quality":"NOT_MEASURED: fixed empty outputs and distinct orthogonal finite nonzero fake chunk vectors","campaign":out})).unwrap();
        server.abort();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Owned PostgreSQL and native GDS; every provider operation is controlled loopback"]
    async fn combined_executor_records_every_attempt_and_publishes_exact_canonical_vectors_without_paid_calls()
     {
        controlled_executor_proof(false).await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Owned PostgreSQL and native GDS; immutable60 diagnostics only, every new provider operation is controlled loopback"]
    async fn remaining111_executor_makes_zero_source_facts_calls_and_preserves_every_denominator() {
        controlled_executor_proof(true).await;
    }
}
