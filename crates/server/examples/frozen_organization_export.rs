//! Controlled toy native export conformance only. No paid/external-model branch.
#[path = "frozen_organization_export/export.rs"]
mod export;
#[path = "frozen_organization_export/extraction.rs"]
mod extraction;
#[path = "frozen_organization_export/io.rs"]
mod io;
#[path = "frozen_organization_export/native.rs"]
mod native;
#[path = "frozen_organization_export/owned.rs"]
mod owned;
#[path = "frozen_organization_export/scopes.rs"]
mod scopes;
#[path = "frozen_organization_export/toy.rs"]
mod toy;
#[path = "frozen_organization_export/vectors.rs"]
mod vectors;
use anyhow::{Context, Result, ensure};
use recollect_server::config::Config;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::Duration,
};
const FREEZE: &str = "ffc4f4251823bde4060c2d1158482728b4913d955e508a1fef9b8cf0cb6c6431";
const SELECTION: &str = "084c075eb5f0a300861e2c95cd6c75ad17506d2c2b49b342070dbab45457056d";
const SCORER: &str = "0fc5da6216cf00acb82d4eabb8213f37021e0e00b737587dfe1a7a9917800a94";
const BASE: &str = ".cache/automatic-mapping/native-export-conformance-2026-10-18";
fn recipe() -> Value {
    json!({"schema":"recollect-offline-organization-recipe-1","candidate_freeze_sha256":FREEZE,"development_selection_sha256":SELECTION,"producer":{"prompt_label":"source-facts-4","schema_label":"source-facts-4","adapter_revision":"learning-quote-anchors-3","mapping_schema":"source-mentions-1"},"parameters":export::parameters(),"modes":export::MODES})
}
fn pins(root: &Path) -> Result<Value> {
    let freeze_path=root.join(".cache/automatic-mapping/revision4-qualification-preparation-2026-10-18/candidate-freeze-v2.json");
    ensure!(
        io::hash_file(&freeze_path)? == FREEZE,
        "Candidate freeze changed"
    );
    let freeze: Value = serde_json::from_slice(&fs::read(freeze_path)?)?;
    for (field, count) in [("source_sha256", 198), ("migration_sha256", 64)] {
        let table = freeze[field].as_object().context("Missing product pins")?;
        ensure!(table.len() == count, "Product pin count differs");
        for (path, hash) in table {
            let p = Path::new(path);
            ensure!(
                !p.is_absolute()
                    && !p
                        .components()
                        .any(|c| !matches!(c, std::path::Component::Normal(_))),
                "Product pin path invalid"
            );
            ensure!(
                io::hash_file(&root.join(p))? == hash.as_str().context("Pin encoding")?,
                "Product candidate source drift"
            );
        }
    }
    let selection_path = root.join(
        freeze["grouping"]["selection_path"]
            .as_str()
            .context("Selection path")?,
    );
    ensure!(
        io::hash_file(&selection_path)? == SELECTION,
        "Selected tuple binding drift"
    );
    let selection: Value = serde_json::from_slice(&fs::read(selection_path)?)?;
    ensure!(
        selection["parameters"] == serde_json::to_value(export::parameters())?,
        "Selected tuple differs"
    );
    ensure!(
        io::hash_file(&root.join("scripts/score_frozen_source_corpus.py"))? == SCORER,
        "Independent scorer changed"
    );
    let mut bindings = BTreeMap::new();
    let mut files = vec![root.join("crates/server/examples/frozen_organization_export.rs")];
    for p in fs::read_dir(root.join("crates/server/examples/frozen_organization_export"))? {
        let p = p?.path();
        ensure!(
            p.extension().is_some_and(|e| e == "rs"),
            "Unexpected exporter support file"
        );
        files.push(p);
    }
    for path in files {
        bindings.insert(
            path.strip_prefix(root)?.to_string_lossy().to_string(),
            io::hash_file(&path)?,
        );
    }
    let mut build = BTreeMap::new();
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "crates/server/Cargo.toml",
        "crates/protocol/Cargo.toml",
        "crates/agent/Cargo.toml",
        "crates/mcp-runtime/Cargo.toml",
    ] {
        build.insert(p, io::hash_file(&root.join(p))?);
    }
    Ok(
        json!({"candidate_freeze":FREEZE,"development_selection":SELECTION,"scorer":SCORER,"exporter_executable":io::hash_file(&std::env::current_exe()?)?,"exporter_sources":bindings,"build_inputs":build,"toy_annotation_author":io::hash_file(&root.join(BASE).join("author_toy_annotations.py"))?,"loader":io::hash_file(&root.join("scripts/frozen_source_corpus.py"))?,"dependency_guidance":{"source":"https://docs.rs/sqlx/latest/sqlx/struct.Pool.html","verified":"2026-10-18","finding":"Await pool.close then bounded exact DB session absence before non-FORCE drop"}}),
    )
}
fn exact_output(root: &Path, arg: &str) -> Result<PathBuf> {
    ensure!(
        !arg.split('/').any(|s| s == ".." || s == "."),
        "Output traversal rejected"
    );
    let p = Path::new(arg);
    let p = if p.is_absolute() {
        p.into()
    } else {
        root.join(p)
    };
    ensure!(
        p.parent() == Some(root.join(BASE).as_path())
            && p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("run-")),
        "Use a new named owned proof run"
    );
    io::directory(p.parent().context("Output parent")?)?;
    Ok(p)
}
async fn toy_annotations(root: &Path, out: &Path) -> Result<Value> {
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new("python3")
            .args(["-B"])
            .arg(root.join(BASE).join("author_toy_annotations.py"))
            .arg(out)
            .current_dir(root)
            .kill_on_drop(true)
            .output(),
    )
    .await??;
    ensure!(
        result.status.success(),
        "Independent toy annotation admission failed"
    );
    Ok(serde_json::from_slice(&fs::read(
        out.join("toy-bindings.json"),
    )?)?)
}
async fn body(
    o: &owned::Owned,
    wire: &toy::Transport,
    root: &Path,
    out: &Path,
    input: Value,
    start: chrono::DateTime<chrono::Utc>,
) -> Result<Value> {
    let toys = toy::fixtures();
    let auth = o.auth().await?;
    let mut scopes = scopes::Scopes::new(&o.state, &o.login).await?;
    let mut imported = BTreeMap::new();
    for t in &toys {
        if !t.text.is_empty() && t.id != "incomplete" {
            let s =
                Box::pin(scopes.import(&o.state, &o.login, &o.admin, t.scope, t.title, &t.text))
                    .await?;
            imported.insert(t.id, s);
        }
    }
    let real: Vec<_> = imported.values().cloned().collect();
    let phase1 = out.join("vector-phase1");
    io::new_directory(&phase1)?;
    vectors::prepare(o, &real, &phase1)
        .await
        .context("stage:short_vectors")?;
    let t = toys
        .iter()
        .find(|t| t.id == "incomplete")
        .context("Incomplete fixture")?;
    let late =
        Box::pin(scopes.import(&o.state, &o.login, &o.admin, t.scope, t.title, &t.text)).await?;
    imported.insert(t.id, late.clone());
    let phase2 = out.join("vector-phase2");
    io::new_directory(&phase2)?;
    vectors::prepare(o, &[late], &phase2)
        .await
        .context("stage:long_vectors")?;
    let mut observed = vec![];
    for t in toys {
        let s = imported.remove(t.id);
        let stage = format!("stage:extract:{}", t.id);
        observed.push(
            Box::pin(extraction::extract(o, &auth, t, s, wire))
                .await
                .with_context(|| stage)?,
        );
    }
    export::isolate(o, out).await?;
    let mut modes = vec![];
    let mut mode_provenance = vec![];
    let mut native_results = vec![];
    for mode in export::MODES {
        let (scalar, proof, results) =
            Box::pin(export::mode(o, &auth, &observed, mode, "2026.08.1")).await?;
        modes.push(scalar);
        mode_provenance.push(proof);
        native_results.extend(results);
    }
    let actual_hybrid: Vec<_> = native_results
        .iter()
        .filter(|r| r.mode == recollect_server::graph::analytics::shadow::Mode::NativeHybrid)
        .cloned()
        .collect();
    let rejects = |trial: &[recollect_server::graph::analytics::shadow::ShadowResult]| {
        export::scalar(
            recollect_server::graph::analytics::shadow::Mode::NativeHybrid,
            &observed,
            trial,
            &std::collections::BTreeSet::new(),
        )
        .is_err()
    };
    let mut wrong = actual_hybrid.clone();
    wrong[0].parameters.neighbors = 5;
    let wrong_tuple = rejects(&wrong);
    let mut wrong = actual_hybrid.clone();
    let row = wrong
        .iter_mut()
        .flat_map(|r| &mut r.sources)
        .find(|s| s.candidate_id.is_some())
        .context("Actual candidate probe")?;
    row.candidate_id = Some(uuid::Uuid::new_v4());
    let wrong_candidate = rejects(&wrong);
    let mut wrong = actual_hybrid.clone();
    let row = wrong
        .iter_mut()
        .flat_map(|r| &mut r.sources)
        .find(|s| s.request_id.is_some())
        .context("Actual request probe")?;
    row.request_id = Some(uuid::Uuid::new_v4());
    let wrong_request = rejects(&wrong);
    let mut wrong = actual_hybrid.clone();
    wrong[0].sources[0].source_version_id = uuid::Uuid::new_v4();
    let stale_version = rejects(&wrong);
    let mut wrong = actual_hybrid.clone();
    let foreign = wrong[1].sources[0].source_version_id;
    wrong[0].groups[0].source_versions.push(foreign);
    let cross_brain_member = rejects(&wrong);
    ensure!(
        wrong_tuple && wrong_candidate && wrong_request && stale_version && cross_brain_member,
        "Actual native result corruption admitted"
    );
    io::json(
        &out.join("native-result-rejection-probes.json"),
        &json!({"based_on_actual_fresh_reports":actual_hybrid.iter().map(|r|r.report_id).collect::<Vec<_>>(),"wrong_tuple_rejected":wrong_tuple,"wrong_candidate_rejected":wrong_candidate,"wrong_request_rejected":wrong_request,"stale_source_version_rejected":stale_version,"cross_brain_group_member_rejected":cross_brain_member,"corrupt_trials_never_exported_as_native_observations":true}),
    )?;
    let corpus_bindings: Value = serde_json::from_slice(&fs::read(out.join("toy-bindings.json"))?)?;
    let recipe_sha = io::hash_file(&out.join("recipe.json"))?;
    let sources:Vec<_>=observed.iter().map(|s|Ok(json!({"document_id":s.toy.id,"text_sha256":io::sha(s.toy.text.as_bytes())?,"partition":"held_out","source_version_id":s.imported.as_ref().map(|i|i.version),"extraction":s.extraction}))).collect::<Result<_>>()?;
    let outcomes = json!({"schema":"recollect-offline-organization-outcomes-1","corpus_bindings":corpus_bindings,"recipe_sha256":recipe_sha,"sources":sources,"modes":modes});
    io::json(&out.join("outcomes.json"), &outcomes)?;
    let mut source_proof = vec![];
    for s in &observed {
        let mut p = s.provenance.clone();
        p["document_id"] = json!(s.toy.id);
        if let Some(i) = &s.imported {
            p["vectors"] = vectors::provenance(o, i).await?;
        }
        source_proof.push(p);
    }
    let mut input = input;
    input["recipe"] = json!(recipe_sha);
    input["toy_inputs"] = json!(io::hash_file(&out.join("toy-inputs.json"))?);
    input["toy_corpus"] = corpus_bindings;
    let provenance = json!({"schema":"recollect-native-organization-export-1","input_sha256":input,"output_sha256":{"outcomes":io::hash_file(&out.join("outcomes.json"))?},"run":{"mode":"controlled_toy_conformance","ownership_nonce_sha256":o.nonce_hash,"database_name":o.database,"started_at":start,"finished_at":chrono::Utc::now(),"controlled_transport_status":"loopback_only_controlled_responses","text_requests":wire.calls.load(Ordering::SeqCst),"embedding_requests":wire.vectors.load(Ordering::SeqCst),"external_model_requests":0,"normal_worker_started":false},"sources":source_proof,"modes":mode_provenance,"not_measured":{"canonical_identity":"NOT_MEASURED","accepted_alias_dispositions":"NOT_MEASURED","support_publication":"NOT_MEASURED","label_usefulness":"NOT_MEASURED","producer_qualification":"NOT_MEASURED","recipe_qualification":"NOT_MEASURED"}});
    io::json(&out.join("provenance.json"), &provenance)?;
    // Wrong engine is an additional preserved failure trial; the five equal-cohort
    // outcomes above remain immutable. Production cleanup closes each exact attempt.
    let (failed, proof, _) = Box::pin(export::mode(
        o,
        &auth,
        &observed,
        recollect_server::graph::analytics::shadow::Mode::NativeHybrid,
        "2099.00.0",
    ))
    .await?;
    io::json(
        &out.join("engine-version-failure.json"),
        &json!({"scalar":failed,"provenance":proof}),
    )?;
    ensure!(
        failed["sources"]
            .as_array()
            .context("Failure cohort")?
            .iter()
            .all(|s| matches!(s["state"].as_str(), Some("failed" | "ineligible"))),
        "Wrong engine returned successful membership"
    );
    ensure!(
        proof["reports"]
            .as_array()
            .context("Engine failure provenance")?
            .iter()
            .all(|r| r["report"]["error_code"] == "organization_engine_version_changed"),
        "Wrong engine failed for unrelated reason"
    );
    // Change only an owned existing profile's representation after immutable proof.
    // Fresh result must reject this mismatch; restore it exactly before cleanup.
    let result = native_results
        .iter()
        .find(|r| r.vector_profile_id.is_some())
        .context("Retained vector result unavailable")?;
    let profile = result.vector_profile_id.context("Profile missing")?;
    let spare: uuid::Uuid = sqlx::query_scalar("SELECT id FROM semantic_profiles WHERE brain_id=$1 AND id<>$2 ORDER BY created_at,id LIMIT 1").bind(result.brain_id).bind(profile).fetch_one(&o.admin).await?;
    let changed =
        sqlx::query("UPDATE semantic_heads SET profile_id=$3 WHERE brain_id=$1 AND profile_id=$2")
            .bind(result.brain_id)
            .bind(profile)
            .bind(spare)
            .execute(&o.admin)
            .await?;
    ensure!(
        changed.rows_affected() == 1,
        "Profile probe did not switch exact head"
    );
    let observed_head: uuid::Uuid =
        sqlx::query_scalar("SELECT profile_id FROM semantic_heads WHERE brain_id=$1")
            .bind(result.brain_id)
            .fetch_one(&o.admin)
            .await?;
    ensure!(observed_head == spare, "Profile probe head not observed");
    let rejected = recollect_server::graph::analytics::shadow::result(
        &o.state,
        &auth,
        result.brain_id,
        result.report_id,
    )
    .await
    .is_err();
    let restored =
        sqlx::query("UPDATE semantic_heads SET profile_id=$3 WHERE brain_id=$1 AND profile_id=$2")
            .bind(result.brain_id)
            .bind(spare)
            .bind(profile)
            .execute(&o.admin)
            .await?;
    ensure!(
        restored.rows_affected() == 1,
        "Profile probe did not restore exact head"
    );
    let observed_head: uuid::Uuid =
        sqlx::query_scalar("SELECT profile_id FROM semantic_heads WHERE brain_id=$1")
            .bind(result.brain_id)
            .fetch_one(&o.admin)
            .await?;
    ensure!(
        observed_head == profile,
        "Restored profile head not observed"
    );
    io::json(
        &out.join("profile-mismatch-probe.json"),
        &json!({"profile_id":profile,"brain_id":result.brain_id,"report_id":result.report_id,"wrong_profile_rejected":rejected,"exact_head_restored":true,"actual_product_created_spare_profile_id":spare}),
    )?;
    ensure!(rejected, "Wrong profile accepted as fresh");
    // All reported feature claims must be observations, including incomplete vectors.
    let hybrid = outcomes["modes"]
        .as_array()
        .context("Modes missing")?
        .first()
        .context("Mode missing")?;
    let rows = hybrid["sources"].as_array().context("Sources missing")?;
    for id in ["missing", "rejected"] {
        let row = rows
            .iter()
            .find(|r| r["document_id"] == id)
            .context("Missing feature control omitted")?;
        ensure!(
            row["ner_feature_available"] == false && row["vector_feature_available"] == true,
            "Independent available vector control failed"
        );
    }
    let incomplete = rows
        .iter()
        .find(|r| r["document_id"] == "incomplete")
        .context("Incomplete vector omitted")?;
    ensure!(
        incomplete["vector_feature_available"] == false,
        "Incomplete vectors were fabricated complete"
    );
    let summary = json!({"schema":"recollect-native-conformance-receipt-1","sources":observed.len(),"modes":5,"mode_source_outcomes":observed.len()*5,"outcomes_sha256":io::hash_file(&out.join("outcomes.json"))?,"provenance_sha256":io::hash_file(&out.join("provenance.json"))?,"exact_empty_controls":2,"overlapping_unicode_occurrences":true,"missing_rejected_ner_with_independent_vectors":true,"incomplete_vector_control":true,"wrong_tuple_digest_device_span_profile_engine_rejected":true,"external_model_requests":0,"real_quality":"NOT_MEASURED"});
    io::json(&out.join("conformance-receipt.json"), &summary)?;
    let _ = root;
    Ok(summary)
}
async fn conformance(root: &Path, out: &Path) -> Result<Value> {
    let input = pins(root)?;
    io::new_directory(out)?;
    io::json(
        &out.join("execution-lock.json"),
        &json!({"mode":"controlled_toy_conformance","started_at":chrono::Utc::now(),"external_calls_permitted":false}),
    )?;
    io::json(
        &out.join("toy-inputs.json"),
        &toy::fixture_json(&toy::fixtures()),
    )?;
    io::json(&out.join("recipe.json"), &recipe())?;
    toy_annotations(root, out).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let wire = toy::Transport::new();
    let router = wire.router();
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let mut config = Config::from_env()?;
    config.models.endpoint = format!("http://{address}/v1");
    config.models.openrouter_endpoint = format!("http://{address}/v1");
    config.models.key = Some("controlled-native-toy-only".into());
    config.models.openrouter_key = None;
    ensure!(
        config.models.embedding_dimensions == 3072
            && config.models.embedding_model == "text-embedding-3-large"
            && config.models.text_model.as_deref() == Some("gpt-5.6-luna"),
        "Frozen installed model configuration drift"
    );
    let native = native::Native::verify(&config, root, out, "2026.08.1").await?;
    let mut o = owned::Owned::setup(out, config).await?;
    let (native, state, observer) = native.observe(&o.state).await?;
    o.state = state;
    let result = Box::pin(body(&o, &wire, root, out, input, chrono::Utc::now())).await;
    if let Err(error) = &result {
        io::json(
            &out.join("failure.json"),
            &json!({"state":"failed_owned_artifacts_retained","error":"controlled native conformance failed","diagnostic_contexts":error.chain().map(|e|e.to_string()).filter(|s|s.starts_with("stage:")||s.starts_with("drop:")).collect::<Vec<_>>(),"source_denominator":9,"mode_denominator":5,"external_model_requests":0,"quality":"NOT_MEASURED"}),
        )?;
    }
    // Preserve proof before cleanup on success and failure; a failed cleanup leaves
    // exact owned state intact and never broadens deletion or kills PG sessions.
    let cleanup = native::cleanup_owned(&native, &o.state, &o.admin, out, &o.marker).await;
    io::json(
        &out.join("native-transport-observation.json"),
        &observer.receipt(),
    )?;
    observer.stop().await;
    server.abort();
    let _ = server.await;
    match cleanup {
        Ok(_) => {
            if let Err(error) = o.drop_database(out).await {
                io::json(
                    &out.join("drop-failure.json"),
                    &json!({"error":"owned database drop failed", "contexts":error.chain().map(|e|e.to_string()).filter(|s|s.starts_with("drop:")).collect::<Vec<_>>(),"database_error_code":error.downcast_ref::<sqlx::Error>().and_then(|e|e.as_database_error()).and_then(|e|e.code()).map(|c|c.into_owned())}),
                )?;
                return Err(error);
            }
        }
        Err(_) => {
            io::json(
                &out.join("cleanup-failure.json"),
                &json!({"state":"retained_exact_owned_database","database":o.database,"no_force_drop":true,"neighbor_state_changed":false}),
            )?;
            anyhow::bail!("Native cleanup incomplete")
        }
    }
    result
}
async fn entry() {
    let args: Vec<_> = std::env::args().collect();
    let result = async {
        ensure!(
            args.len() == 4 && args[1] == "controlled-toy-conformance" && args[2] == "--out",
            "Arguments invalid"
        );
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()?;
        let out = exact_output(&root, &args[3])?;
        conformance(&root, &out).await
    }
    .await;
    match result {
        Ok(value) => println!("{}", value),
        Err(_) => {
            eprintln!("native organization export conformance failed");
            std::process::exit(1)
        }
    }
}

fn main() {
    // Before constructing threads, config, app state, or any subprocess.
    for name in ["OPENAI_API_KEY", "OPENROUTER_API_KEY"] {
        unsafe {
            std::env::remove_var(name);
        }
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("controlled runtime");
    runtime.block_on(entry());
}
