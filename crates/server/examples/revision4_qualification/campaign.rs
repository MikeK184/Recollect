use super::{
    guard::{self, Ledger},
    inputs::{self, Admitted},
    io, native,
    owned::Owned,
    pipeline::{self, CohortPlan, Observed, Plan},
    wire::Wire,
};
use anyhow::{Context, Result, ensure};
use recollect_server::config::Config;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, atomic::Ordering},
};
use tokio::sync::Mutex;
use uuid::Uuid;
async fn server(w: &Wire) -> Result<(std::net::SocketAddr, tokio::task::JoinHandle<()>)> {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let a = l.local_addr()?;
    let r = w.router();
    Ok((
        a,
        tokio::spawn(async move {
            let _ = axum::serve(l, r).await;
        }),
    ))
}
fn dbpath(out: &Path, name: &str) -> std::path::PathBuf {
    out.join(format!("owned-{name}"))
}
fn new_owned_dir(out: &Path) -> Result<()> {
    io::new_directory(out)?;
    io::new_directory(&out.join("bodies"))?;
    Ok(())
}
fn schedule(plan: &Plan) -> Result<Value> {
    let mut rows = vec![];
    for c in &plan.cohorts {
        for b in &c.batches {
            rows.push(json!({"transport":super::transport::binding(&std::path::PathBuf::from(format!("owned-{}/bodies/{}.json",c.cohort.name,b.batch)),&serde_json::to_vec(&b.body)?,true)?,"arm":"embeddings","key":b.key,"operation":b.batch,"body_sha256":b.body_sha256,"body_bytes":serde_json::to_vec(&b.body).unwrap().len(),"brain_id":b.brain,"policy_id":b.policy,"entry_ids":b.entries}));
        }
        for t in &c.text {
            rows.push(json!({"transport":super::transport::binding(&std::path::PathBuf::from(format!("owned-{}/bodies/{}.json",c.cohort.name,t.operation)),&serde_json::to_vec(&t.body)?,false)?,"arm":"heldout_text","key":t.key,"operation":t.operation,"body_sha256":t.body_sha256,"body_bytes":serde_json::to_vec(&t.body).unwrap().len(),"brain_id":t.source.brain,"source_version_id":t.source.version,"device_id":t.source.device,"policy_id":t.policy0}));
        }
    }
    for c in &plan.controls {
        rows.push(json!({"transport":super::transport::binding(&std::path::PathBuf::from(format!("owned-{}/bodies/{}.json",c.case_id,c.run.id)),&serde_json::to_vec(&c.body)?,false)?,"arm":"control_text","key":format!("control/{}",c.case_id),"operation":c.run.id,"body_sha256":c.body_sha256,"body_bytes":serde_json::to_vec(&c.body).unwrap().len(),"brain_id":c.brain,"source_version_id":c.run.source_version_id,"device_id":c.device,"offered":c.offered,"conditional_support_slots":1}));
    }
    Ok(
        json!({"schema":"recollect-revision4-campaign-schedule-1","transport_rule":super::transport::rule(),"upfront":rows,"conditional_support":{"maximum_calls":6,"unknown_until_normalization":true,"actual_body_freeze":"before_each_transmission","canonical_dependencies_only":true}}),
    )
}
pub async fn prepare(root: &Path, out: &Path, a: Admitted, controlled: bool) -> Result<Value> {
    io::new_directory(out)?;
    io::json(
        &out.join("preparation-lock.json"),
        &json!({"schema":"recollect-revision4-preparation-lock-1","started_at":chrono::Utc::now(),"controlled":controlled,"external_calls_permitted":false,"input_bindings":a.bindings}),
    )?;
    let config = Config::from_env()?;
    pipeline::configured(&config)?;
    ensure!(
        a.model_configuration
            == json!({"provider":"openai","endpoint":"https://api.openai.com/v1","text_model":"gpt-5.6-luna","embedding_model":"text-embedding-3-large","embedding_dimensions":3072}),
        "Public frozen model config drift"
    );
    let preflight = out.join("native-preflight");
    io::new_directory(&preflight)?;
    native::Native::verify(&config, root, &preflight, "2026.08.1").await?;
    let w = Wire::controlled(out);
    let (address, server) = server(&w).await?;
    let config = pipeline::config(&config, address);
    let mut cohorts = vec![];
    for cohort in a.cohorts {
        let path = dbpath(out, &cohort.name);
        new_owned_dir(&path)?;
        let o = Owned::setup(&path, config.clone()).await?;
        let result = Box::pin(pipeline::prepare_cohort(&o, cohort, &path, controlled)).await;
        match result {
            Ok(p) => cohorts.push(p),
            Err(e) => {
                let _ = io::json(
                    &path.join("preparation-failure.json"),
                    &json!({"state":"failed_owned_state_retained","diagnostic":"cohort_native_preparation_failed"}),
                );
                o.close().await?;
                server.abort();
                return Err(e);
            }
        }
        o.close().await?;
    }
    let controls = io::read_json(
        &root.join("docs/research/automatic-reconciliation-native-controls-v3-2026-10-18.json"),
        256 * 1024,
    )?;
    ensure!(
        io::hash_file(
            &root.join("docs/research/automatic-reconciliation-native-controls-v3-2026-10-18.json")
        )? == inputs::CONTROLS,
        "Original controls drift"
    );
    let cases = controls["cases"].as_array().context("Six controls")?;
    ensure!(cases.len() == 6, "Six control denominator");
    let mut control_plans = vec![];
    for case in cases {
        let id = case["id"].as_str().context("Control ID")?;
        ensure!(
            ["rc01", "rc02", "rc03", "rc04", "rc05", "rc06"].contains(&id),
            "Control ID"
        );
        let path = dbpath(out, id);
        new_owned_dir(&path)?;
        let o = Owned::setup(&path, config.clone()).await?;
        let result = Box::pin(pipeline::prepare_control(&o, case, &path, controlled)).await;
        match result {
            Ok(p) => control_plans.push(p),
            Err(e) => {
                let _ = io::json(
                    &path.join("preparation-failure.json"),
                    &json!({"state":"failed_owned_state_retained","diagnostic":"control_native_preparation_failed"}),
                );
                o.close().await?;
                server.abort();
                return Err(e);
            }
        }
        o.close().await?;
    }
    server.abort();
    ensure!(
        w.external.load(Ordering::SeqCst) == 0 && w.controlled.load(Ordering::SeqCst) == 0,
        "Preparation never transmits campaign requests"
    );
    let plan = Plan {
        schema: "recollect-revision4-native-campaign-plan-1".into(),
        cohorts,
        controls: control_plans,
    };
    let schedule = schedule(&plan)?;
    let mut templates = plan.controls.iter().map(|p| p.template.clone());
    let template = templates.next().context("Support template")?;
    ensure!(
        templates.all(|p| p == template),
        "Support template varies by control"
    );
    let count: u64 = plan
        .cohorts
        .iter()
        .map(|c| c.cohort.sources.len() as u64)
        .sum();
    let empty: u64 = plan
        .cohorts
        .iter()
        .flat_map(|c| &c.cohort.sources)
        .filter(|s| s.text.is_empty())
        .count() as u64;
    let batches: u64 = plan.cohorts.iter().map(|c| c.batches.len() as u64).sum();
    let maximum_calls = if controlled {
        let held = 2 * (count - empty);
        json!({"heldout_text":held,"control_text":plan.controls.len(),"embeddings":batches,"support":6,"total":held+plan.controls.len()as u64+batches+6})
    } else {
        ensure!(count == 46, "Full actual heldout denominator");
        guard::caps(empty, batches, plan.controls.len() as u64)?
    };
    io::json(&out.join("plan.json"), &serde_json::to_value(&plan)?)?;
    io::json(&out.join("schedule.json"), &schedule)?;
    io::json(&out.join("support-template.json"), &template)?;
    io::json(&out.join("pricing.json"), &a.pricing)?; // Recipe binds the canonical pricing value and original input raw digest separately.
    let price_sha = io::hash_file(&out.join("pricing.json"))?;
    let recipe = json!({"schema":"recollect-revision4-campaign-recipe-1","kind":guard::KIND,"controlled":controlled,"candidate_freeze_sha256":inputs::FREEZE,"development_selection_sha256":inputs::SELECTION,"input_bindings":a.bindings,"pricing":a.pricing,"pricing_sha256":price_sha,"input_pricing_sha256":a.pricing_sha256,"executable_sha256":io::hash_file(&std::env::current_exe()?)?,"code":inputs::code(root)?,"schedule_sha256":io::hash_file(&out.join("schedule.json"))?,"plan_sha256":io::hash_file(&out.join("plan.json"))?,"support_template_sha256":io::hash_file(&out.join("support-template.json"))?,"support_template_value_sha256":io::value_hash(&template)?,"controls_sha256":inputs::CONTROLS,"parameters":inputs::parameters(),"modes":inputs::MODES,"maximum_calls":maximum_calls,"maximum_input_tokens":guard::INPUT,"maximum_output_tokens":guard::OUTPUT,"budget_nano_usd":guard::cost(&maximum_calls)?,"exact_empty_source_count":empty,"canonical_embedding_batches":batches,"source_observations":count*2,"source_mode_rows":count*10,"public_upstreams":["https://api.openai.com/v1/responses","https://api.openai.com/v1/embeddings"],"provider":"openai","text_model":"gpt-5.6-luna","embedding_model":"text-embedding-3-large","dimensions":3072,"provider_transport":{"rule":super::transport::rule(),"helper_path":"crates/server/examples/revision4_qualification/transport.rs","helper_sha256":io::hash_file(&root.join("crates/server/examples/revision4_qualification/transport.rs"))?},"support_actual_calls_unknown":true,"scorer_sha256":inputs::SCORER,"label_usefulness":"NOT_MEASURED","producer_qualification":"NOT_MEASURED","recipe_qualification":"NOT_MEASURED"});
    io::json(&out.join("campaign-recipe.json"), &recipe)?;
    let hash = io::hash_file(&out.join("campaign-recipe.json"))?;
    io::json(
        &out.join("approval-template.json"),
        &guard::template(&recipe, &hash),
    )?;
    let receipt = json!({"schema":"recollect-revision4-campaign-preparation-1","recipe_sha256":hash,"schedule_sha256":recipe["schedule_sha256"],"executable_sha256":recipe["executable_sha256"],"maximum_calls":maximum_calls,"budget_nano_usd":recipe["budget_nano_usd"],"exact_empty_sources":empty,"complete_canonical_embedding_batches":batches,"cohort_gold_readiness":plan.cohorts.iter().map(|c|(c.cohort.name.clone(),json!({"complete_topic_gold_readiness":c.cohort.readiness,"unknown_topic_pairs":a.bindings["reviewed_cohort_counts"][&c.cohort.name]["topic_pairs_unknown"]}))).collect::<BTreeMap<_,_>>(),"source_observations":count*2,"source_mode_rows":count*10,"external_model_calls":0,"support_actual_calls_unknown_until_normalization":true,"paid_approval":false,"controlled_conformance":controlled,"quality":"NOT_MEASURED"});
    io::json(&out.join("preparation-receipt.json"), &receipt)?;
    Ok(receipt)
}
fn frozen(root: &Path, out: &Path) -> Result<(Value, Plan, String)> {
    let recipe = io::read_json(&out.join("campaign-recipe.json"), 2 * 1024 * 1024)?;
    let hash = io::hash_file(&out.join("campaign-recipe.json"))?;
    ensure!(
        recipe["executable_sha256"] == io::hash_file(&std::env::current_exe()?)?
            && recipe["code"] == inputs::code(root)?
            && recipe["schedule_sha256"] == io::hash_file(&out.join("schedule.json"))?
            && recipe["plan_sha256"] == io::hash_file(&out.join("plan.json"))?
            && recipe["pricing_sha256"] == io::hash_file(&out.join("pricing.json"))?
            && recipe["support_template_sha256"]
                == io::hash_file(&out.join("support-template.json"))?,
        "Frozen transitive binding drift"
    );
    let plan: Plan =
        serde_json::from_value(io::read_json(&out.join("plan.json"), 96 * 1024 * 1024)?)?;
    ensure!(
        plan.schema == "recollect-revision4-native-campaign-plan-1"
            && schedule(&plan)? == io::read_json(&out.join("schedule.json"), 2 * 1024 * 1024)?,
        "Frozen schedule drift"
    );
    validate_freeze(root, out, &recipe, &plan)?;
    Ok((recipe, plan, hash))
}
pub async fn execute(
    root: &Path,
    out: &Path,
    approval_path: Option<&Path>,
    controlled: bool,
) -> Result<Value> {
    let (recipe, plan, hash) = frozen(root, out)?;
    ensure!(recipe["controlled"] == controlled, "Execution mode differs");
    let a = if let Some(path) = approval_path {
        io::read_json(path, 65536)?
    } else {
        ensure!(controlled, "Human approval required");
        guard::template(&recipe, &hash)
    };
    if !controlled {
        guard::approval(&a, &recipe, &hash)?;
    } // Must precede a forwarding client, provider key use, lock or DB open.
    let ledger = Ledger::begin(out, &recipe, &hash, &a, controlled)?;
    let mut config = Config::from_env()?;
    pipeline::configured(&config)?;
    let secret = if controlled {
        None
    } else {
        Some(
            config
                .models
                .key
                .take()
                .context("Configured public provider key")?,
        )
    };
    config.models.key = None;
    config.models.openrouter_key = None;
    io::new_directory(&out.join("raw-responses"))?;
    io::new_directory(&out.join("raw-response-bindings"))?;
    io::new_directory(&out.join("wire-outcomes"))?;
    let w = if let Some(key) = secret {
        Wire::approved(out, key, ledger)?
    } else {
        let mut w = Wire::controlled(out);
        w.ledger = Arc::new(Mutex::new(Some(ledger)));
        w
    };
    let (address, server) = server(&w).await?;
    let config = pipeline::config(&config, address);
    let preflight = out.join("execute-native-preflight");
    io::new_directory(&preflight)?;
    native::Native::verify(&config, root, &preflight, "2026.08.1").await?;
    for c in &plan.cohorts {
        let path = dbpath(out, &c.cohort.name);
        let o = Owned::reopen(&path, config.clone()).await?;
        let result = Box::pin(pipeline::execute_cohort(&o, c, &w, &path)).await;
        match result {
            Ok(families) => {
                for (family, all) in inputs::FAMILIES.iter().zip(families) {
                    let rows:Vec<_>=all.iter().map(|s|json!({"document_id":s.source.document_id,"extraction":s.extraction,"candidate_id":s.candidate,"provenance":s.provenance})).collect();
                    io::json(
                        &path.join(format!("{family}-observations.json")),
                        &json!(rows),
                    )?;
                }
            }
            Err(_) => {
                if let Some(l) = w.ledger.lock().await.as_mut() {
                    l.stop()?;
                }
                io::json(
                    &path.join("execution-failure.json"),
                    &json!({"state":"failed_retained","remaining_sources":"not_attempted"}),
                )?;
            }
        }
        o.close().await?;
    }
    let mut observations = BTreeMap::new();
    for c in &plan.cohorts {
        for family in inputs::FAMILIES {
            let path = dbpath(out, &c.cohort.name).join(format!("{family}-observations.json"));
            if path.exists() {
                observations.insert(
                    path.strip_prefix(out)?.to_string_lossy().to_string(),
                    io::hash_file(&path)?,
                );
            }
        }
    }
    let mut canonical = vec![];
    for c in &plan.controls {
        let path = dbpath(out, &c.case_id);
        let o = Owned::reopen(&path, config.clone()).await?;
        let result = Box::pin(pipeline::execute_control(&o, c, &w, &path)).await;
        match result {
            Ok(v) => canonical.push(v),
            Err(_) => {
                if let Some(l) = w.ledger.lock().await.as_mut() {
                    l.stop()?;
                }
                io::json(
                    &path.join("execution-failure.json"),
                    &json!({"state":"failed_retained","remaining_sources":"not_attempted"}),
                )?;
            }
        }
        o.close().await?;
    }
    server.abort();
    let receipt = json!({"schema":"recollect-revision4-execution-1","recipe_sha256":hash,"controlled":controlled,"external_model_calls":w.external.load(Ordering::SeqCst),"controlled_model_calls":w.controlled.load(Ordering::SeqCst),"ledger":w.ledger.lock().await.as_ref().context("Ledger")?.summary(),"observation_sha256":observations,"quality":"NOT_MEASURED","remaining_unattempted_preserved":true,"source_observations":recipe["source_observations"],"source_mode_rows":recipe["source_mode_rows"]});
    io::json(&out.join("execution-receipt.json"), &receipt)?;
    io::json(&out.join("canonical-observations.json"), &json!(canonical))?;
    Ok(receipt)
}
pub async fn export(root: &Path, out: &Path) -> Result<Value> {
    let (recipe, plan, hash) = frozen(root, out)?;
    let execution = io::read_json(&out.join("execution-receipt.json"), 65536)?;
    let mut config = Config::from_env()?;
    config.models.endpoint = "http://127.0.0.1:9/v1".into();
    config.models.openrouter_endpoint = config.models.endpoint.clone();
    config.models.key = Some("revision4-export-no-model-transport".into());
    config.models.openrouter_key = None;
    let target = out.join("exports");
    io::new_directory(&target)?;
    let preflight = target.join("native-preflight");
    io::new_directory(&preflight)?;
    let native = native::Native::verify(&config, root, &preflight, "2026.08.1").await?;
    let mut summaries = vec![];
    for c in &plan.cohorts {
        let path = dbpath(out, &c.cohort.name);
        let mut o = Owned::reopen(&path, config.clone()).await?;
        let (_, observed_state, observer) = native.observe(&o.state).await?;
        o.state = observed_state;
        for family in inputs::FAMILIES {
            let file = path.join(format!("{family}-observations.json"));
            let rel = file.strip_prefix(out)?.to_string_lossy().to_string();
            let rows = if file.exists() {
                ensure!(
                    execution["observation_sha256"][&rel] == io::hash_file(&file)?,
                    "Immutable execution observation hash drift"
                );
                io::read_json(&file, 96 * 1024 * 1024)?
                    .as_array()
                    .context("Observation rows")?
                    .clone()
            } else {
                vec![]
            };
            let expected: std::collections::BTreeSet<_> = c
                .cohort
                .sources
                .iter()
                .map(|s| s.document_id.as_str())
                .collect();
            let mut actual = std::collections::BTreeSet::new();
            for r in &rows {
                io::fields(
                    r,
                    &["document_id", "extraction", "candidate_id", "provenance"],
                )?;
                let doc = r["document_id"].as_str().context("Observation identity")?;
                ensure!(
                    expected.contains(doc) && actual.insert(doc),
                    "Foreign/duplicate observation identity"
                );
            }
            ensure!(
                rows.is_empty() || rows.len() == c.cohort.sources.len(),
                "Partial observation cohort refused"
            );
            let mut all = vec![];
            for source in &c.cohort.sources {
                let row = rows.iter().find(|r| r["document_id"] == source.document_id);
                all.push(Observed{source:source.clone(),imported:c.imports[&source.document_id].clone(),extraction:row.map(|r|r["extraction"].clone()).unwrap_or(json!({"state":"not_attempted","failure":null,"candidates":null})),candidate:row.and_then(|r|r["candidate_id"].as_str()).map(str::parse).transpose()?,provenance:row.map(|r|r["provenance"].clone()).unwrap_or(json!({"document_id":source.document_id,"reason":if source.text.is_empty(){"exact_empty_import_ineligible"}else{"campaign_stopped_unattempted"}}))});
            }
            let dir = target.join(format!("{}-{family}", c.cohort.name));
            io::new_directory(&dir)?;
            let summary=Box::pin(pipeline::export_family(&o,c,&all,family,&dir,&json!({"recipe_sha256":hash,"code":recipe["code"],"executable_sha256":recipe["executable_sha256"]}),recipe["controlled"]==true)).await?;
            summaries.push(summary);
        }
        io::json(
            &path.join("native-transport-observation.json"),
            &observer.receipt(),
        )?;
        observer.stop().await;
        o.close().await?;
    }
    let receipt = json!({"schema":"recollect-revision4-exports-1","artifacts":summaries,"source_observations":recipe["source_observations"],"source_mode_rows":recipe["source_mode_rows"],"external_model_calls":0,"canonical_native_scalar_label_gates_separate":true,"quality":"NOT_MEASURED"});
    io::json(&out.join("export-receipt.json"), &receipt)?;
    Ok(receipt)
}
pub async fn cleanup(root: &Path, out: &Path) -> Result<Value> {
    let _ = root;
    let mut config = Config::from_env()?;
    config.models.endpoint = "http://127.0.0.1:9/v1".into();
    config.models.openrouter_endpoint = config.models.endpoint.clone();
    config.models.key = Some("revision4-owned-cleanup-no-model-transport".into());
    config.models.openrouter_key = None;
    let command = out.join(format!("cleanup-{}", Uuid::new_v4()));
    io::new_directory(&command)?;
    let preflight = command.join("native-preflight");
    io::new_directory(&preflight)?;
    let native = native::Native::verify(&config, root, &preflight, "2026.08.1").await?;
    let mut cleaned = 0;
    for name in [
        "private24",
        "prospective22",
        "rc01",
        "rc02",
        "rc03",
        "rc04",
        "rc05",
        "rc06",
        "incomplete-vectors",
    ] {
        let path = dbpath(out, name);
        if !path.join("ownership.json").exists() {
            continue;
        }
        let owner = io::read_json(&path.join("ownership.json"), 16384)?;
        let dbname = owner["database"].as_str().context("Owned DB")?;
        let admin_url = std::env::var("DATABASE_ADMIN_URL")?;
        let pool = recollect_server::db::pool(&super::owned::database_url(&admin_url, "postgres")?)
            .await?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)")
                .bind(dbname)
                .fetch_one(&pool)
                .await?;
        pool.close().await;
        if !exists {
            continue;
        }
        let dir = command.join(name);
        io::new_directory(&dir)?;
        let mut o = Owned::reopen(&path, config.clone()).await?;
        o.state = native.attach(&o.state);
        native::cleanup_owned(&native, &o.state, &o.admin, &dir, &o.marker).await?;
        sanitize_snapshot(&o, &dir).await?;
        let credential_redaction = io::remove_private_file(&path.join("state/credentials.json"))?;
        io::json(
            &dir.join("credential-redaction.json"),
            &json!({"credentials_file":credential_redaction,"only_exact_owned_state":true}),
        )?;
        let inventory = state_inventory(&path.join("state"))?;
        io::json(&dir.join("state-inventory.json"), &inventory)?;
        snapshot_state(&path.join("state"), &dir, &inventory)?;
        snapshot_database(root, &o, &dir).await?;
        io::json(
            &dir.join("state-snapshot.json"),
            &json!({"state_preserved":true,"exact_owner_nonce_sha256":o.nonce_hash,"database_snapshot_sha256":io::hash_file(&dir.join("database-snapshot.dump"))?,"native_journal_closed":true,"owned_scratch_absent":true,"brain_native_rows_removed":true}),
        )?;
        o.drop_database(&dir).await?;
        cleaned += 1;
    }
    let v = json!({"schema":"recollect-revision4-cleanup-1","owned_databases_removed":cleaned,"drop_without_force":true,"postgres_sessions_terminated":0,"owned_snapshots_and_exports_preserved":true,"neighboring_state_changed":false});
    io::json(&command.join("cleanup-receipt.json"), &v)?;
    Ok(v)
}
async fn snapshot_database(root: &Path, o: &Owned, out: &Path) -> Result<()> {
    o.verify().await?;
    let id_output = tokio::process::Command::new(root.join("scripts/docker.sh"))
        .args([
            "ps",
            "--filter",
            "label=com.docker.compose.project=recollect",
            "--filter",
            "label=com.docker.compose.service=postgres",
            "--format",
            "{{.ID}}",
        ])
        .kill_on_drop(true)
        .output()
        .await?;
    ensure!(
        id_output.status.success(),
        "Owned PostgreSQL snapshot service"
    );
    let ids = String::from_utf8(id_output.stdout)?;
    let ids: Vec<_> = ids.split_whitespace().collect();
    ensure!(
        ids.len() == 1 && ids[0].len() >= 12 && ids[0].bytes().all(|b| b.is_ascii_hexdigit()),
        "One owned PostgreSQL service"
    );
    let metadata = tokio::process::Command::new(root.join("scripts/docker.sh"))
        .args(["inspect", "--format", "{{json .Config.Labels}}", ids[0]])
        .kill_on_drop(true)
        .output()
        .await?;
    ensure!(metadata.status.success(), "Owned PostgreSQL labels");
    let labels = io::strict(&metadata.stdout)?;
    ensure!(
        labels["com.docker.compose.project"] == "recollect"
            && labels["com.docker.compose.service"] == "postgres"
            && labels["com.docker.compose.project.working_dir"] == root.to_string_lossy().as_ref(),
        "Snapshot container ownership drift"
    );
    let actual_installation: uuid::Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(&o.admin)
        .await?;
    let probe=tokio::process::Command::new(root.join("scripts/docker.sh")).args(["exec",ids[0],"psql","--no-psqlrc","--username","recollect_admin","--dbname",&o.database,"--tuples-only","--no-align","--command","SELECT current_database()||'|'||shobj_description(d.oid,'pg_database')||'|'||p.id::text FROM pg_database d CROSS JOIN privacy_installation p WHERE d.datname=current_database()"] ).kill_on_drop(true).output().await?;
    ensure!(
        probe.status.success()
            && String::from_utf8(probe.stdout)?.trim()
                == format!("{}|{}|{}", o.database, o.marker, actual_installation),
        "Container/local exact snapshot database binding differs"
    );
    let file = io::exclusive(&out.join("database-snapshot.dump"))?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new(root.join("scripts/docker.sh"))
            .args([
                "exec",
                ids[0],
                "pg_dump",
                "--format=custom",
                "--no-owner",
                "--no-privileges",
                "--username",
                "recollect_admin",
                &o.database,
            ])
            .stdout(std::process::Stdio::from(file.try_clone()?))
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .status(),
    )
    .await??;
    file.sync_all()?;
    ensure!(
        result.success()
            && file.metadata()?.len() > 0
            && file.metadata()?.len() <= 512 * 1024 * 1024,
        "Validated owned DB snapshot failed"
    );
    let input = io::input_file(&out.join("database-snapshot.dump"), 512 * 1024 * 1024)?;
    let validation = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new(root.join("scripts/docker.sh"))
            .args(["exec", "--interactive", ids[0], "pg_restore", "--list"])
            .stdin(std::process::Stdio::from(input))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .status(),
    )
    .await??;
    ensure!(
        validation.success(),
        "Owned sanitized snapshot archive validation failed"
    );
    io::json(
        &out.join("database-snapshot-validation.json"),
        &json!({"archive_sha256":io::hash_file(&out.join("database-snapshot.dump"))?,"format":"postgresql_custom","pg_restore_list_passed":true,"container_local_database_marker_installation_match":true,"capabilities_sanitized":true,"restored_runtime":"NOT_TESTED"}),
    )?;
    Ok(())
}

fn validate_freeze(root: &Path, out: &Path, r: &Value, p: &Plan) -> Result<()> {
    io::fields(
        r,
        &[
            "schema",
            "kind",
            "controlled",
            "candidate_freeze_sha256",
            "development_selection_sha256",
            "input_bindings",
            "pricing",
            "pricing_sha256",
            "input_pricing_sha256",
            "executable_sha256",
            "code",
            "schedule_sha256",
            "plan_sha256",
            "support_template_sha256",
            "support_template_value_sha256",
            "controls_sha256",
            "parameters",
            "modes",
            "maximum_calls",
            "maximum_input_tokens",
            "maximum_output_tokens",
            "budget_nano_usd",
            "exact_empty_source_count",
            "canonical_embedding_batches",
            "source_observations",
            "source_mode_rows",
            "public_upstreams",
            "provider",
            "text_model",
            "embedding_model",
            "dimensions",
            "support_actual_calls_unknown",
            "provider_transport",
            "scorer_sha256",
            "label_usefulness",
            "producer_qualification",
            "recipe_qualification",
        ],
    )?;
    ensure!(
        r["schema"] == "recollect-revision4-campaign-recipe-1"
            && r["kind"] == guard::KIND
            && r["candidate_freeze_sha256"] == inputs::FREEZE
            && r["development_selection_sha256"] == inputs::SELECTION
            && r["controls_sha256"] == inputs::CONTROLS
            && r["scorer_sha256"] == inputs::SCORER
            && r["parameters"] == inputs::parameters()
            && r["modes"] == json!(inputs::MODES)
            && r["maximum_input_tokens"] == guard::INPUT
            && r["maximum_output_tokens"] == guard::OUTPUT
            && r["provider"] == "openai"
            && r["text_model"] == "gpt-5.6-luna"
            && r["embedding_model"] == "text-embedding-3-large"
            && r["dimensions"] == 3072
            && r["provider_transport"]
                == json!({"rule":super::transport::rule(),"helper_path":"crates/server/examples/revision4_qualification/transport.rs","helper_sha256":io::hash_file(&root.join("crates/server/examples/revision4_qualification/transport.rs"))?})
            && r["public_upstreams"]
                == json!([
                    "https://api.openai.com/v1/responses",
                    "https://api.openai.com/v1/embeddings"
                ]),
        "Fixed recipe scope differs"
    );
    ensure!(
        p.cohorts.len() == 2 && p.controls.len() == 6,
        "Complete cohorts/controls required"
    );
    let mut names = std::collections::BTreeSet::new();
    let mut versions = std::collections::BTreeSet::new();
    let mut operations = std::collections::BTreeSet::new();
    let mut n = 0u64;
    let mut e = 0u64;
    let mut b = 0u64;
    let mut template = None;
    for c in &p.cohorts {
        ensure!(
            ["private24", "prospective22"].contains(&c.cohort.name.as_str())
                && names.insert(&c.cohort.name)
                && c.cohort.families.len() == 2
                && c.cohort.sources.len() <= 250,
            "Cohort identity/count drift"
        );
        if r["controlled"] == false {
            ensure!(
                c.cohort.sources.len() == if c.cohort.name == "private24" { 24 } else { 22 },
                "Full heldout denominator required"
            );
        }
        let docs: std::collections::BTreeSet<_> = c
            .cohort
            .sources
            .iter()
            .map(|s| s.document_id.as_str())
            .collect();
        ensure!(
            docs.len() == c.cohort.sources.len()
                && c.imports.len() == docs.len()
                && c.imports.keys().all(|k| docs.contains(k.as_str())),
            "Import/source bijection differs"
        );
        n += c.cohort.sources.len() as u64;
        for s in &c.cohort.sources {
            ensure!(
                io::sha(s.text.as_bytes())? == s.text_sha256,
                "Source text digest drift"
            );
            if s.text.is_empty() {
                e += 1;
                ensure!(c.imports[&s.document_id].is_none(), "Empty source imported");
            } else {
                let i = c.imports[&s.document_id]
                    .as_ref()
                    .context("Missing nonempty canonical import")?;
                ensure!(
                    versions.insert(i.version),
                    "Source versions shared across cohorts"
                );
            }
        }
        ensure!(
            c.text.len() == 2 * c.imports.values().filter(|i| i.is_some()).count(),
            "Both text family schedules required"
        );
        let mut pairs = std::collections::BTreeSet::new();
        let evaluator = io::read_json(&dbpath(out, &c.cohort.name).join("evaluator.json"), 16384)?;
        let actor: Uuid = evaluator["actor_id"]
            .as_str()
            .context("Frozen evaluator")?
            .parse()?;
        for t in &c.text {
            ensure!(
                t.cohort == c.cohort.name
                    && t.key == format!("heldout/{}/{}/{}", c.cohort.name, t.family, t.document_id)
                    && pairs.insert((t.document_id.clone(), t.family.clone()))
                    && c.imports
                        .get(&t.document_id)
                        .and_then(Option::as_ref)
                        .is_some_and(|i| serde_json::to_value(i).ok()
                            == serde_json::to_value(&t.source).ok()),
                "Text schedule pair/source bijection drift"
            );
            match (&t.run, t.family.as_str()) {
                (None, "source-facts-4") => {}
                (Some(run), "source-reconciliation-4") => ensure!(
                    run.id == t.operation
                        && run.brain_id == t.source.brain
                        && run.source_version_id == t.source.version
                        && run.actor_id == actor
                        && run.device_id == Some(t.source.device)
                        && run.operation_id == Some(t.source.operation)
                        && run.selection == t.source.selection
                        && run.reconciliation_inputs.is_empty(),
                    "Reconciliation canonical context drift"
                ),
                _ => anyhow::bail!("Producer invocation tuple drift"),
            }
            super::transport::verify(
                &dbpath(out, &c.cohort.name)
                    .join("bodies")
                    .join(format!("{}.json", t.operation)),
                &serde_json::to_vec(&t.body)?,
                false,
            )?;
            ensure!(
                inputs::FAMILIES.contains(&t.family.as_str())
                    && docs.contains(t.document_id.as_str())
                    && operations.insert(t.operation)
                    && io::value_hash(&t.body)? == t.body_sha256
                    && io::hash_file(
                        &dbpath(out, &c.cohort.name)
                            .join("bodies")
                            .join(format!("{}.json", t.operation))
                    )? == t.body_sha256,
                "Frozen upfront text body/operation drift"
            );
        }
        let mut entries = std::collections::BTreeSet::new();
        let chunks = c.chunks.as_array().context("Canonical chunks")?;
        let chunk_entries = chunks
            .iter()
            .map(|row| {
                row["entry_id"]
                    .as_str()
                    .context("Chunk entry ID")?
                    .parse::<Uuid>()
                    .map_err(Into::into)
            })
            .collect::<Result<std::collections::BTreeSet<_>>>()?;
        for batch in &c.batches {
            super::transport::verify(
                &dbpath(out, &c.cohort.name)
                    .join("bodies")
                    .join(format!("{}.json", batch.batch)),
                &serde_json::to_vec(&batch.body)?,
                true,
            )?;
            ensure!(
                operations.insert(batch.batch)
                    && !batch.entries.is_empty()
                    && batch.entries.len() <= 20
                    && batch.entries.iter().all(|e| entries.insert(*e))
                    && io::value_hash(&batch.body)? == batch.body_sha256
                    && io::hash_file(
                        &dbpath(out, &c.cohort.name)
                            .join("bodies")
                            .join(format!("{}.json", batch.batch))
                    )? == batch.body_sha256,
                "Frozen complete embedding partition drift"
            );
        }
        ensure!(
            entries == chunk_entries
                && entries.len() == chunks.len()
                && chunks.iter().all(|row| c
                    .batches
                    .iter()
                    .any(|b| row["batch_id"] == json!(b.batch)
                        && row["brain_id"] == json!(b.brain)
                        && b.entries.iter().any(|e| row["entry_id"] == json!(e)))),
            "Complete shared chunk coverage required"
        );
        b += c.batches.len() as u64;
        let mut families = std::collections::BTreeSet::new();
        for f in &c.cohort.families {
            inputs::validate_recipe(&f.recipe, &f.producer)?;
            ensure!(
                families.insert(f.producer.as_str())
                    && io::sha(&f.recipe_bytes)? == f.recipe_sha256
                    && io::strict(&f.recipe_bytes)? == f.recipe,
                "Scalar recipe raw bytes drift"
            );
        }
    }
    let mut cases = std::collections::BTreeSet::new();
    for c in &p.controls {
        super::transport::verify(
            &dbpath(out, &c.case_id)
                .join("bodies")
                .join(format!("{}.json", c.run.id)),
            &serde_json::to_vec(&c.body)?,
            false,
        )?;
        ensure!(
            ["rc01", "rc02", "rc03", "rc04", "rc05", "rc06"].contains(&c.case_id.as_str())
                && cases.insert(c.case_id.as_str())
                && operations.insert(c.run.id)
                && io::value_hash(&c.body)? == c.body_sha256
                && io::hash_file(
                    &dbpath(out, &c.case_id)
                        .join("bodies")
                        .join(format!("{}.json", c.run.id))
                )? == c.body_sha256,
            "Control schedule/body drift"
        );
        if let Some(t) = &template {
            ensure!(t == &c.template, "Support templates differ");
        } else {
            template = Some(c.template.clone());
        }
    }
    let held = 2 * (n - e);
    let maximum =
        json!({"heldout_text":held,"control_text":6,"embeddings":b,"support":6,"total":held+b+12});
    ensure!(
        r["maximum_calls"] == maximum
            && r["budget_nano_usd"] == guard::cost(&maximum)?
            && r["exact_empty_source_count"] == e
            && r["canonical_embedding_batches"] == b
            && r["source_observations"] == n * 2
            && r["source_mode_rows"] == n * 10,
        "Programmatically recomputed maximum scope differs"
    );
    let t = template.context("Support template")?;
    ensure!(
        r["support_template_value_sha256"] == io::value_hash(&t)?
            && io::read_json(&out.join("support-template.json"), 65536)? == t,
        "Conditional support template drift"
    );
    if r["controlled"] == false {
        let (_, freeze) = inputs::bound(
            root,
            &r["input_bindings"]["candidate_freeze_file"],
            256 * 1024,
        )?;
        let (_, selection) = inputs::bound(
            root,
            &r["input_bindings"]["development_selection_file"],
            256 * 1024,
        )?;
        inputs::pins(root, &freeze, &selection)?;
        let manifest = &r["input_bindings"]["manifest_file"];
        let path = io::contained(
            root,
            manifest["path"]
                .as_str()
                .context("Original input manifest")?,
        )?;
        let admitted = inputs::admit_frozen(root, &path, io::h(&manifest["sha256"])?)?;
        for cohort in &p.cohorts {
            let original = admitted
                .cohorts
                .iter()
                .find(|c| c.name == cohort.cohort.name)
                .context("Reviewed cohort")?;
            ensure!(
                serde_json::to_value(original)? == serde_json::to_value(&cohort.cohort)?,
                "Frozen campaign source-only cohort differs from reviewed inputs"
            );
        }
    }
    Ok(())
}

async fn sanitize_snapshot(o: &Owned, out: &Path) -> Result<()> {
    o.verify().await?;
    let busy: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs WHERE state='running'")
        .fetch_one(&o.admin)
        .await?;
    ensure!(busy == 0, "Cannot sanitize active owned workers");
    let mut tx = o.admin.begin().await?;
    let mut actions = BTreeMap::new();
    for (table, statement) in [
        ("sessions", "DELETE FROM sessions"),
        ("device_pairings", "DELETE FROM device_pairings"),
        (
            "devices.token",
            "UPDATE devices SET token=id,revoked_at=coalesce(revoked_at,clock_timestamp())",
        ),
        ("invitations.token", "UPDATE invitations SET token=id"),
        (
            "model_requests.call_token",
            "UPDATE model_requests SET call_token=id",
        ),
        (
            "analytics_attempts.lease_token",
            "UPDATE analytics_attempts SET lease_token=id WHERE cleaned_at IS NOT NULL",
        ),
        (
            "jobs.lease_token",
            "UPDATE jobs SET lease_token=NULL,lease_until=NULL WHERE state<>'running'",
        ),
        (
            "graph_generations.native_attempt_token",
            "UPDATE graph_generations SET native_attempt_token=NULL",
        ),
    ] {
        let count = sqlx::query(statement)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        actions.insert(table, count);
    }
    let unexpected:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM answer_requests)+(SELECT count(*) FROM mcp_calls)+(SELECT count(*) FROM mcp_observation_outbox)+(SELECT count(*) FROM oidc_flows)").fetch_one(&mut *tx).await?; // Only empty unrelated capability surfaces can be snapshotted.
    ensure!(unexpected == 0, "Unexpected unrelated capability data");
    tx.commit().await?;
    io::json(
        &out.join("snapshot-redaction.json"),
        &json!({"schema":"recollect-owned-snapshot-redaction-1","actions":actions,"canonical_identity_columns_changed":false,"source_versions_heads_requests_usage_support_native_provenance_preserved":true,"capability_replacements":"public_row_id_placeholders_and_null_leases","credential_values_read_or_printed":false}),
    )?;
    Ok(())
}
fn state_inventory(root: &Path) -> Result<Value> {
    io::private_directory(root)?;
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeMap::new();
    while let Some(dir) = pending.pop() {
        io::directory(&dir)?;
        for entry in std::fs::read_dir(&dir)? {
            let e = entry?;
            let kind = e.file_type()?;
            let p = e.path();
            ensure!(!kind.is_symlink(), "State snapshot symlink refused");
            if kind.is_dir() {
                pending.push(p)
            } else {
                ensure!(
                    kind.is_file() && files.len() < 10000 && e.file_name() != "credentials.json",
                    "Unexpected secret/state snapshot file"
                );
                files.insert(
                    p.strip_prefix(root)?.to_string_lossy().to_string(),
                    io::hash_file(&p)?,
                );
            }
        }
    }
    Ok(
        json!({"schema":"recollect-owned-state-inventory-1","files":files,"credentials_excluded":true,"state_preserved":true,"snapshot_values_exported":false}),
    )
}

fn snapshot_state(source: &Path, out: &Path, inventory: &Value) -> Result<()> {
    let target = out.join("state-snapshot");
    io::new_directory(&target)?;
    let files = inventory["files"].as_object().context("State inventory")?;
    for (rel, hash) in files {
        let input = io::contained(source, rel)?;
        let destination = io::contained(&target, rel).or_else(|_| {
            // Create only declared normal directory members under the new private copy.
            let mut current = target.clone();
            let parts: Vec<_> = Path::new(rel).components().collect();
            ensure!(
                parts
                    .iter()
                    .all(|p| matches!(p, std::path::Component::Normal(_))),
                "State member traversal"
            );
            for part in &parts[..parts.len().saturating_sub(1)] {
                current.push(part.as_os_str());
                if !current.exists() {
                    io::new_directory(&current)?;
                } else {
                    io::private_directory(&current)?;
                }
            }
            Ok::<_, anyhow::Error>(target.join(rel))
        })?;
        let bytes = io::read(&input, 96 * 1024 * 1024)?;
        ensure!(
            io::sha(&bytes)? == io::h(hash)?,
            "State changed before private snapshot"
        );
        io::write(&destination, &bytes)?;
        ensure!(
            io::hash_file(&destination)? == io::h(hash)?,
            "State snapshot copy mismatch"
        );
    }
    let actual = state_inventory(&target)?;
    ensure!(
        actual["files"] == inventory["files"],
        "State snapshot member inventory differs"
    );
    io::json(
        &out.join("state-snapshot-validation.json"),
        &json!({"schema":"recollect-owned-state-snapshot-1","format":"exclusive_private_directory_snapshot","inventory_sha256":io::value_hash(&inventory["files"] )?,"copied_members":files.len(),"member_hashes":inventory["files"],"all_members_validated":true,"credentials_present":false,"independent_from_live_state":true,"immutable_create_new_files":true}),
    )?;
    Ok(())
}
