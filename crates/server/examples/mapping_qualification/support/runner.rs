//! Guarded combined execution. All provider traffic stays on the public gateway.
use crate::{
    evaluation,
    ledger::{Arm, Ledger},
    native,
    proposal::{self, source_facts::support as legacy},
    reconciliation,
    scopes::{Imported, Scopes},
};
use anyhow::{Context, Result, ensure};
use recollect_server::{
    AppState, autonomous, db, knowledge_mapping::production, model_gateway as gateway, semantic,
    worker,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    time::Instant,
};
use uuid::Uuid;

fn api<T>(r: recollect_server::error::Result<T>) -> Result<T> {
    r.map_err(|e| anyhow::anyhow!("Canonical operation failed ({})", e.1))
}
fn row(file: &mut File, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
async fn policy(state: &AppState, login: &legacy::Login, brain: Uuid, mode: &str) -> Result<Uuid> {
    let path = format!("/api/brains/{brain}/models/policy");
    let (current, _) = legacy::http(state, Some(login), "GET", &path, Value::Null).await?;
    let mut p = current["current"]["policy"].clone();
    p["enabled"] = json!(true);
    p["purposes"] = if mode == "vectors" {
        json!(["embedding"])
    } else {
        json!(["extraction", "synthesis"])
    };
    p["content_classes"] = json!(["document", "claim", "raw_session", "support_excerpt"]);
    p["max_input_bytes"] = json!(16384);
    p["max_output_tokens"] = json!(4096);
    p["max_concurrent"] = json!(1);
    p["daily_token_limit"] = json!(10_000_000);
    p["automatic_learning"] = json!(false);
    p["autonomous_memory"] = json!(mode == "reconciliation");
    p["automatic_embedding"] = json!(mode == "vectors");
    p["acceptance"] = Value::Null;
    let (p, _) = legacy::http(
        state,
        Some(login),
        "PUT",
        &path,
        json!({"base_change":current["current"]["change_id"],"policy":p}),
    )
    .await?;
    Ok(p["change_id"]
        .as_str()
        .context("Owned policy ID missing")?
        .parse()?)
}
async fn brain(state: &AppState, login: &legacy::Login, name: &str) -> Result<Uuid> {
    let (b, _) = legacy::http(
        state,
        Some(login),
        "POST",
        "/api/brains",
        json!({"name":name}),
    )
    .await?;
    Ok(b["id"]
        .as_str()
        .context("Owned Brain ID missing")?
        .parse()?)
}
async fn source(state: &AppState, login: &legacy::Login, brain: Uuid, text: &str) -> Result<Uuid> {
    let (s,_)=legacy::http(state,Some(login),"POST",&format!("/api/brains/{brain}/sources"),json!({"title":"Synthetic benchmark source","media_type":"text/plain","content":text,"retain_content":true})).await?;
    Ok(s["version"]["id"]
        .as_str()
        .context("Owned source version missing")?
        .parse()?)
}
async fn archive(state: &AppState, login: &legacy::Login, brain: Uuid) -> Result<()> {
    legacy::http(
        state,
        Some(login),
        "PATCH",
        &format!("/api/brains/{brain}"),
        json!({"archived":true}),
    )
    .await?;
    Ok(())
}
fn adapt(text: &str, version: Uuid, value: &Value, outcome: &mut Value) {
    outcome["raw_output"] = value.clone();
    if value.get("mapping").is_none_or(Value::is_null) {
        outcome["state"] = json!("mapping_absent");
        return;
    }
    match production::adapt(version, text, &value["mapping"]) {
        Ok(c) => {
            outcome["state"] = json!("validated");
            outcome["candidates"] = serde_json::to_value(c).unwrap();
        }
        Err(e) => {
            outcome["state"] = json!("adapter_rejected");
            outcome["adapter_error"] = json!(format!("{e:?}"));
        }
    }
}
async fn invoke(
    out: &Path,
    state: &AppState,
    admin: &PgPool,
    ledger: &mut Ledger,
    arm: Arm,
    key: &str,
    context: gateway::Context,
    inv: gateway::Invocation,
    preview: gateway::Preview,
    text: &str,
    version: Uuid,
    mut outcome: Value,
) -> Result<Value> {
    let operation = inv.operation;
    let bytes = serde_json::to_vec(&preview.body)?;
    ensure!(bytes.len() <= 65536, "Complete request exceeds frozen cap");
    let body_hash = legacy::sha256(&bytes)?;
    let body_artifact = format!("outgoing-bodies/{operation}.json");
    legacy::private_json(&out.join(&body_artifact), &preview.body)?;
    outcome["brain_id"] = json!(context.brain);
    outcome["source_version_id"] = json!(version);
    outcome["policy_id"] = json!(preview.policy_id);
    outcome["canonical_input_refs"] = serde_json::to_value(&inv.inputs)?;
    outcome["prompt_label"] = json!(inv.prompt_label);
    outcome["schema_label"] = json!(inv.schema_label);
    outcome["request_body_artifact"] = json!(body_artifact);
    ledger.reserve(arm,key,operation,json!({"actual_body_sha256":body_hash,"request_bytes":bytes.len(),"policy_id":preview.policy_id,"brain_id":context.brain,"source_version_id":version,"prompt_label":inv.prompt_label,"schema_label":inv.schema_label,"request_body_artifact":body_artifact}))?;
    let start = Instant::now();
    let response = gateway::invoke_previewed_for_policy(state, context, inv, &preview).await;
    outcome["operation"] = json!(operation);
    outcome["actual_body_sha256"] = json!(body_hash);
    outcome["latency_ms"] = json!(start.elapsed().as_millis());
    let mut uncertain = false;
    match response {
        Ok(response) => {
            let violation = response.request.input_tokens.unwrap_or(0) > 67584
                || response.request.output_tokens.unwrap_or(0) > 4096;
            outcome["request"] = serde_json::to_value(&response.request)?;
            outcome["usage_bound_violation"] = json!(violation);
            uncertain |=
                violation || matches!(response.request.state.as_str(), "running" | "uncertain");
            match response.output {
                Some(gateway::Output::Json(value)) => adapt(text, version, &value, &mut outcome),
                _ => outcome["state"] = json!("output_unavailable"),
            }
        }
        Err(error) => {
            outcome["state"] = json!("gateway_failed");
            outcome["error_code"] = json!(error.1);
            let status: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT state,error_code FROM model_requests WHERE brain_id=$1 AND operation_id=$2",
            )
            .bind(context.brain)
            .bind(operation)
            .fetch_optional(admin)
            .await?;
            uncertain = status
                .as_ref()
                .is_none_or(|r| matches!(r.0.as_str(), "running" | "uncertain"));
            outcome["request_state"] = serde_json::to_value(status)?;
        }
    }
    ledger.finish(key,json!({"state":outcome["state"],"request":outcome["request"],"error_code":outcome["error_code"]}),uncertain)?;
    Ok(outcome)
}

async fn vectors(
    state: &AppState,
    admin: &PgPool,
    actor: Uuid,
    recipe: &Value,
    sources: &[(Value, Imported)],
    ledger: &mut Ledger,
    out: &Path,
) -> Result<Value> {
    let expected = recipe["optional_vectors"]["source_chunks"]
        .as_u64()
        .context("Frozen vector count")? as usize;
    let mut versions = vec![];
    let mut actual_manifest = vec![];
    let mut brains = std::collections::BTreeMap::new();
    for (doc, source) in sources {
        let scope:serde_json::Value=sqlx::query_scalar("SELECT selection FROM source_import_scopes WHERE brain_id=$1 AND version_id=$2 AND operation_id=$3").bind(source.brain).bind(source.version).bind(source.operation).fetch_one(admin).await?;
        ensure!(
            scope == serde_json::to_value(&source.selection)?,
            "Source import scope changed before embedding"
        );
        let chunks:Vec<(i32,i32,i32,String)>=sqlx::query_as("SELECT ordinal,byte_start,byte_end,encode(sha256(convert_to(content,'UTF8')),'hex') FROM source_chunks WHERE brain_id=$1 AND version_id=$2 ORDER BY ordinal").bind(source.brain).bind(source.version).fetch_all(admin).await?;
        for (n, a, b, h) in chunks {
            actual_manifest.push(json!({"document_id":doc["id"],"partition":doc["partition"],"ordinal":n,"byte_start":a,"byte_end":b,"chunk_sha256":h}));
        }
        versions.push(json!({"corpus":doc["corpus"],"document_id":doc["id"],"partition":doc["partition"],"brain_id":source.brain,"source_id":source.source,"source_version_id":source.version,"operation_id":source.operation,"selection":source.selection,"policy_id":source.policy,"extraction_request_id":doc["extraction_request_id"]}));
        brains.insert(source.brain, source.policy);
    }
    ensure!(
        actual_manifest.len() == expected
            && legacy::sha256(&serde_json::to_vec(&actual_manifest)?)?
                == recipe["optional_vectors"]["source_chunk_manifest_sha256"],
        "Actual canonical chunks drifted before embedding; no vector request admitted"
    );
    // One discovery pass per Brain, before any failed batches; never re-run the
    // automatic retry scheduler during this one-shot qualification campaign.
    for brain in brains.keys() {
        api(semantic::maintain_brain(state, *brain, actor).await)?;
    }
    let brain_ids: Vec<_> = brains.keys().copied().collect();
    let entries: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM semantic_entries WHERE brain_id=ANY($1) AND kind='source_chunk'",
    )
    .bind(&brain_ids)
    .fetch_one(admin)
    .await?;
    ensure!(
        entries as usize == expected,
        "Complete canonical semantic discovery is required"
    );
    let mut batches = vec![];
    while let Some(job) = worker::claim(&state.pool, "model").await? {
        ensure!(
            brains.contains_key(&job.brain_id) && job.kind == "semantic.generate",
            "Refuse to execute any other model job"
        );
        if ledger.stopped() {
            worker::fail(&state.pool, &job, worker::Failure::Unsupported).await?;
            break;
        }
        let brain = job.brain_id;
        let policy_id = brains[&brain];
        let (batch,expected_policy):(Uuid,Uuid)=sqlx::query_as("SELECT id,policy_id FROM semantic_batches WHERE brain_id=$1 AND job_id=$2 AND retry_of IS NULL AND automatic_attempt=0").bind(brain).bind(job.id).fetch_one(admin).await?;
        ensure!(expected_policy == policy_id, "Vector policy changed");
        let ids:Vec<Uuid>=sqlx::query_scalar("SELECT entry_id FROM semantic_batch_inputs WHERE brain_id=$1 AND batch_id=$2 ORDER BY ordinal").bind(brain).bind(batch).fetch_all(admin).await?;
        let inv = gateway::Invocation {
            operation: batch,
            purpose: "embedding".into(),
            inputs: ids
                .iter()
                .map(|id| gateway::InputRef {
                    kind: "semantic_entry".into(),
                    id: *id,
                })
                .collect(),
            query: None,
            instructions: String::new(),
            prompt_label: semantic::REPRESENTATION.into(),
            schema_label: "embedding-float-1".into(),
            format: gateway::Format::Embedding,
            work_lease: None,
            metadata_replay: false,
            expected_json: None,
        };
        let context = gateway::Context {
            brain,
            actor: job.actor_id,
            device: job.device_id,
        };
        let preview = api(gateway::preview_for_policy(state, context, &inv, policy_id).await)?;
        let body_hash = legacy::sha256(&serde_json::to_vec(&preview.body)?)?;
        let full_body = serde_json::to_vec(&preview.body)?;
        ensure!(
            full_body.len() <= 65536,
            "Complete embedding batch body exceeds frozen bound"
        );
        let body_artifact = format!("outgoing-bodies/{batch}.json");
        legacy::private_json(&out.join(&body_artifact), &preview.body)?;
        let key = format!("vectors/{batch}");
        ledger.reserve(Arm::Embedding,&key,batch,json!({"body_sha256":body_hash,"entry_ids":ids,"policy_id":policy_id,"actual_body_bytes":serde_json::to_vec(&preview.body)?.len(),"request_body_artifact":body_artifact}))?;
        let result = semantic::execute_previewed(state, &job, &preview).await;
        let request:Option<Value>=sqlx::query_scalar("SELECT to_jsonb(q) - 'call_token' FROM model_requests q WHERE brain_id=$1 AND operation_id=$2").bind(brain).bind(batch).fetch_optional(admin).await?;
        let uncertain = request.as_ref().is_none_or(|r| {
            matches!(r["state"].as_str(), Some("running" | "uncertain"))
                || r["input_tokens"].as_u64().unwrap_or(0) > 67584
                || r["output_tokens"].as_u64().unwrap_or(0) > 4096
        });
        ledger.finish(&key,json!({"request":request,"worker_result":result.as_ref().map(|_|"completed").unwrap_or("failed")}),uncertain)?;
        batches.push(json!({"batch_id":batch,"request":request,"entry_ids":ids,"body_sha256":body_hash,"uncertain":uncertain}));
        if uncertain {
            break;
        }
    }
    let vectors:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('entry_id',e.id,'brain_id',e.brain_id,'profile_id',e.profile_id,'source_version_id',e.source_version_id,'source_chunk_id',e.input_id,'state',e.state,'request_id',e.request_id,'error_code',e.error_code,'truncated',e.truncated,'embedding_available',e.embedding IS NOT NULL,'embedding_sha256',CASE WHEN e.embedding IS NULL THEN NULL ELSE encode(sha256(convert_to(e.embedding::text,'UTF8')),'hex') END) FROM semantic_entries e WHERE brain_id=ANY($1) ORDER BY source_version_id,input_id").bind(&brain_ids).fetch_all(admin).await?;
    ensure!(
        vectors.len() == expected,
        "Vector failures remain in complete entry denominators"
    );
    let profiles:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(p) FROM semantic_profiles p JOIN semantic_heads h ON h.profile_id=p.id AND h.brain_id=p.brain_id WHERE p.brain_id=ANY($1) ORDER BY p.brain_id").bind(&brain_ids).fetch_all(admin).await?;
    let receipt = json!({"profiles":profiles,"documents":versions,"manifest":actual_manifest,"batches":batches,"entries":vectors,"document_denominator":sources.len(),"pair_denominators":{"old20_diagnostic":190,"fresh12_untouched":66},"vector_entry_denominator":expected,"same_sources_as_extraction":true,"quality_or_native_baseline":"not_measured_by_vector_generation; separate real-candidate equal-input scorer required; never label gold_upper_bound"});
    legacy::private_json(&out.join("canonical-vector-provenance.json"), &receipt)?;
    Ok(receipt)
}

pub async fn execute(
    out: &Path,
    config: recollect_server::config::Config,
    recipe: &Value,
    schedule: &[Value],
    controls: &Value,
    ledger: &mut Ledger,
) -> Result<()> {
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let native = native::Native::verify(
        &config,
        &repository,
        out,
        recipe["evaluation"]["expected_gds_version"]
            .as_str()
            .context("Frozen GDS version")?,
    )
    .await?;
    let (state, admin, root, database, login, actor, facts_brain, facts_policy) =
        legacy::setup(out, config).await?;
    if recipe["campaign_kind"] == "remaining-reconciliation-vectors-1" {
        ensure!(
            recipe["limits"]["source_facts_requests"] == 0,
            "Remaining campaign cannot enable a source-facts method"
        );
    }
    let facts_dir = out.join("source-facts");
    let recon_dir = out.join("reconciliation");
    let run_facts = recipe["limits"]["source_facts_requests"]
        .as_u64()
        .context("Frozen source-facts cap")?
        > 0;
    ensure!(
        run_facts || !facts_dir.exists(),
        "Remaining campaign refuses any source-facts work directory"
    );
    if run_facts {
        fs::create_dir(&facts_dir)?;
    }
    fs::create_dir(&recon_dir)?;
    fs::create_dir(out.join("outgoing-bodies"))?;
    let mut facts_rows = if run_facts {
        Some(legacy::private_file(&facts_dir.join("outcomes.jsonl"))?)
    } else {
        None
    };
    let mut recon_rows = legacy::private_file(&recon_dir.join("outcomes.jsonl"))?;
    let mut control_rows = legacy::private_file(&out.join("control-outcomes.jsonl"))?;
    for case in controls["cases"].as_array().context("Six frozen cases")? {
        let mut outcome = json!({"id":case["id"],"partition":case["partition"],"state":"not_attempted","candidates":null});
        if !ledger.stopped() {
            let prepared = reconciliation::setup_case(&state, &admin, case).await?;
            let context = gateway::Context {
                brain: prepared.brain_id,
                actor: prepared.actor_id,
                device: Some(prepared.device_id),
            };
            let inv = autonomous::invocation(&prepared.run);
            let actual =
                api(gateway::preview_for_policy(&state, context, &inv, prepared.policy_id).await)?;
            ensure!(
                actual.body == prepared.preview.body,
                "Canonical control body changed since independent setup"
            );
            outcome = invoke(
                out,
                &state,
                &admin,
                ledger,
                Arm::Reconciliation,
                &format!("control/{}", prepared.case_id),
                context,
                inv,
                actual,
                &prepared.source.text,
                prepared.run.source_version_id,
                outcome,
            )
            .await?;
            outcome["canonical_id_map"] = prepared.canonical_id_map.clone();
            outcome["prior_workflow_receipt"] = prepared.prior_workflow_receipt.clone();
            outcome["actual_offered_revision_ids"] = json!(prepared.offered);
            outcome["canonical_current_mutation_quality"] = json!(
                "NOT_MEASURED: current extraction only; no paid support assessment or learning publication"
            );
            row(&mut control_rows, &outcome)?;
            reconciliation::archive_case(&state, &admin, &prepared).await?;
        } else {
            row(&mut control_rows, &outcome)?;
        }
    }
    for doc in schedule
        .iter()
        .filter(|d| run_facts && d["corpus"] != "fresh12")
    {
        let mut outcome = json!({"corpus":doc["corpus"],"id":doc["id"],"partition":doc["partition"],"text_sha256":doc["text_sha256"],"state":"not_attempted","candidates":null});
        if !ledger.stopped() {
            let text = doc["text"].as_str().context("Frozen document text")?;
            let version = source(&state, &login, facts_brain, text).await?;
            let inv = gateway::extraction(Uuid::new_v4(), version);
            let context = gateway::Context {
                brain: facts_brain,
                actor,
                device: None,
            };
            let preview =
                api(gateway::preview_for_policy(&state, context, &inv, facts_policy).await)?;
            outcome = invoke(
                out,
                &state,
                &admin,
                ledger,
                Arm::SourceFacts,
                &format!(
                    "facts/{}/{}",
                    doc["corpus"].as_str().unwrap(),
                    doc["id"].as_str().unwrap()
                ),
                context,
                inv,
                preview,
                text,
                version,
                outcome,
            )
            .await?;
        }
        row(
            facts_rows
                .as_mut()
                .context("Source-facts loop forbidden in remaining campaign")?,
            &outcome,
        )?;
    }
    archive(&state, &login, facts_brain).await?;
    let recon_brain = brain(&state, &login, "Owned no-offer reconciliation corpus").await?;
    let recon_policy = policy(&state, &login, recon_brain, "reconciliation").await?;
    let mut scopes = Scopes::new(&state, &login).await?;
    let mut vector_sources = vec![];
    for doc in schedule {
        let mut outcome = json!({"corpus":doc["corpus"],"id":doc["id"],"partition":doc["partition"],"text_sha256":doc["text_sha256"],"state":"not_attempted","candidates":null});
        if !ledger.stopped() {
            let text = doc["text"].as_str().context("Frozen text")?;
            let imported = if doc["corpus"] != "original40" {
                Some(
                    scopes
                        .import(
                            &state,
                            &login,
                            doc["scope"]
                                .as_str()
                                .context("Declared canonical fixture scope")?,
                            doc["title"].as_str().context("Frozen title")?,
                            text,
                        )
                        .await?,
                )
            } else {
                None
            };
            let (brain, version, policy_id, device) = if let Some(source) = &imported {
                (
                    source.brain,
                    source.version,
                    source.policy,
                    Some(source.device),
                )
            } else {
                (
                    recon_brain,
                    source(&state, &login, recon_brain, text).await?,
                    recon_policy,
                    None,
                )
            };
            while worker::run_once(&state, "capture").await? {}
            let mut run = proposal::run(version, vec![])?;
            run.id = Uuid::new_v4();
            run.brain_id = brain;
            run.actor_id = actor;
            run.policy_id = policy_id;
            run.device_id = device;
            if let Some(source) = &imported {
                run.selection = source.selection.clone();
                run.operation_id = Some(source.operation);
            }
            let mut tx = api(db::preparation_tx(&state.pool, actor, device).await)?;
            let offered = api(autonomous::targets(&state, &mut tx, &run, 16384).await)?;
            ensure!(
                offered.is_empty(),
                "Fresh source corpus must have no prior canonical memories"
            );
            tx.commit().await?;
            let inv = autonomous::invocation(&run);
            let context = gateway::Context {
                brain,
                actor,
                device,
            };
            let preview = api(gateway::preview_for_policy(&state, context, &inv, policy_id).await)?;
            outcome = invoke(
                out,
                &state,
                &admin,
                ledger,
                Arm::Reconciliation,
                &format!(
                    "recon/{}/{}",
                    doc["corpus"].as_str().unwrap(),
                    doc["id"].as_str().unwrap()
                ),
                context,
                inv,
                preview,
                text,
                version,
                outcome,
            )
            .await?;
            outcome["canonical_offered_inputs"] = json!([]);
            outcome["brain_id"] = json!(brain);
            outcome["source_version_id"] = json!(version);
            outcome["canonical_selection"] = serde_json::to_value(&run.selection)?;
            outcome["canonical_operation_id"] = json!(run.operation_id);
            outcome["policy_id"] = json!(policy_id);
            if let Some(source) = imported {
                let mut descriptor = doc.clone();
                descriptor["extraction_request_id"] = outcome["request"]["id"].clone();
                descriptor["extraction_outcome"] = outcome.clone();
                vector_sources.push((descriptor, source));
            }
            outcome["boundary"] =
                json!("Extractor-only request; no learning publication or support-assessment call");
        }
        row(&mut recon_rows, &outcome)?;
    }
    archive(&state, &login, recon_brain).await?;
    let vector_receipt = if !ledger.stopped() {
        Some(vectors(&state, &admin, actor, recipe, &vector_sources, ledger, out).await?)
    } else {
        None
    };
    let evaluation = if !ledger.stopped() {
        let evaluation_state = native.attach(&state);
        // Installation journal initialization belongs to the owned admin setup;
        // the ordinary application role intentionally cannot read this table.
        recollect_server::graph::analytics::initialize(&admin, &evaluation_state.config).await?;
        Some(
            Box::pin(evaluation::execute(
                &evaluation_state,
                &admin,
                &login,
                actor,
                recipe,
                out,
                &vector_sources,
            ))
            .await?,
        )
    } else {
        None
    };
    let summary = json!({"status":if ledger.stopped(){"stopped_review_required"}else{"completed"},"ledger":ledger.summary(),"source_facts_documents":if run_facts {60}else{0},"reused_source_facts_diagnostics":recipe["reused_source_facts"],"reconciliation_documents":schedule.len(),"controls":6,"vector_documents":recipe["optional_vectors"]["documents"],"vector_entries":recipe["optional_vectors"]["source_chunks"],"vectors_attempted":vector_receipt.is_some(),"unpublished_evaluation":evaluation,"paid_support_or_answer_judging":0,"qualified_producer_or_organization_recipe_selected":false,"semantic_profiles":"Only owned canonical embedding profiles from the frozen configured model; normal installation profiles unchanged","normal_data_copied":false,"cleanup":"owned database retained through comparison; explicit owned cleanup after independent review"});
    legacy::private_json(&out.join("execution-receipt.json"), &summary)?;
    // Retain only this explicitly owned synthetic DB for independent vector and
    // control review; do not touch normal installation data or global policies.
    state.pool.close().await;
    admin.close().await;
    root.close().await;
    legacy::private_json(
        &out.join("retained-state.json"),
        &json!({"database":database,"owner":legacy::OWNER,"synthetic_only":true,"reason":"Independent exact vector/control review; explicit owned cleanup after acceptance"}),
    )?;
    Ok(())
}
