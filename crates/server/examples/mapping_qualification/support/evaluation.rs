//! Explicit unpublished native comparisons after guarded extraction. Real
//! vectors remain in the same owned PostgreSQL database throughout evaluation.
use crate::{
    proposal::{self, source_facts::support as legacy},
    scopes::Imported,
};
use anyhow::{Context, Result, ensure};
use axum::{extract::FromRequestParts, http::Request};
use recollect_server::{
    AppState,
    auth::Auth,
    graph::analytics::shadow::{self, Mode, ShadowSource},
    knowledge_mapping::source_features::HybridParameters,
    model_gateway, worker,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

const OLD20: &str = "docs/research/graph-organization-adversarial-fixtures-2026-10-18.json";
const OLD20_HASH: &str = "a0ecfe7dd692c2517f00035c0be7205841e27e7b17551f7a21580edc6d1c916c";
pub fn arm_definitions() -> Value {
    json!({
     "native_hybrid":"Shared hybrid native weighted Leiden plus exact eligible term facets",
     "native_weighted":"Entity IDF native weighted Leiden plus the same exact eligible term facets",
     "entity_overlap":"Exact eligible term facets only",
     "metadata":"Overlapping actual canonical repository, environment and area groups only",
     "eligible_vector":"Retained vector only native weighted Leiden; no entity facets; selected hybrid neighbors/cosine, uniform scale and no-entity median1"
    })
}
fn api<T>(r: recollect_server::error::Result<T>) -> Result<T> {
    r.map_err(|e| anyhow::anyhow!("Unpublished evaluation failed ({})", e.1))
}
async fn evaluator(state: &AppState, login: &legacy::Login) -> Result<Auth> {
    let (mut parts, _) = Request::builder()
        .uri("/api/auth/me")
        .header("cookie", &login.cookie)
        .body(())?
        .into_parts();
    let auth = api(Auth::from_request_parts(&mut parts, state).await)?;
    api(auth.require_browser())?;
    Ok(auth)
}
async fn isolate_heavy(
    state: &AppState,
    admin: &PgPool,
    login: &legacy::Login,
    dir: &Path,
) -> Result<()> {
    let owner:Option<String>=sqlx::query_scalar("SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database()").fetch_one(admin).await?;
    ensure!(
        owner.as_deref() == Some(legacy::OWNER),
        "Refuse projection cancellation outside the exact owned synthetic database"
    );
    let initial:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'brain_id',brain_id,'target_id',target_id,'kind',kind,'state',state,'actor_id',actor_id) FROM jobs WHERE lane='heavy' AND state IN('queued','running') ORDER BY created_at,id").fetch_all(admin).await?;
    ensure!(
        initial.iter().all(|job| job["state"] == "queued"
            && matches!(
                job["kind"].as_str(),
                Some("graph.project" | "knowledge.fields")
            )),
        "Unexpected heavy work prevents isolated evaluation; no unrelated task executed or cancelled"
    );
    legacy::private_json(
        &dir.join("initial-owned-projection-jobs.json"),
        &json!({"owner":owner,"jobs":initial,"only_preexisting_snapshot":true,"model_jobs_changed":false}),
    )?;
    for job in &initial {
        legacy::http(
            state,
            Some(login),
            "POST",
            &format!(
                "/api/brains/{}/jobs/{}/cancel",
                job["brain_id"].as_str().context("Owned job Brain")?,
                job["id"].as_str().context("Owned job ID")?
            ),
            Value::Null,
        )
        .await?;
    }
    Ok(())
}
struct Scorer<'a> {
    root: &'a Path,
    sha256: &'a str,
}
impl Scorer<'_> {
    async fn python(&self, args: &[String]) -> Result<()> {
        ensure!(
            legacy::sha256(&fs::read(self.root.join("scripts/score-real-topics.py"))?)?
                == self.sha256,
            "Frozen scorer changed before subprocess; no comparison or selection admitted"
        );
        let result = tokio::time::timeout(
            Duration::from_secs(20),
            tokio::process::Command::new("python3")
                .arg(self.root.join("scripts/score-real-topics.py"))
                .args(args)
                .current_dir(self.root)
                .kill_on_drop(true)
                .output(),
        )
        .await
        .context("Bounded offline scoring timed out")??;
        ensure!(
            result.status.success(),
            "Offline scalar comparison rejected receipt: {}",
            String::from_utf8_lossy(&result.stderr)
                .chars()
                .take(600)
                .collect::<String>()
        );
        Ok(())
    }
}
fn path(path: &Path) -> Result<String> {
    Ok(path
        .to_str()
        .context("Local artifact path encoding")?
        .into())
}
async fn scored(
    scorer: &Scorer<'_>,
    dir: &Path,
    name: &str,
    corpus: &Path,
    hash: &str,
    receipt: &Value,
    selection: Option<&Path>,
) -> Result<PathBuf> {
    let input = dir.join(format!("{name}-receipt.json"));
    let output = dir.join(format!("{name}-score.json"));
    legacy::private_json(&input, receipt)?;
    let mut args = vec![
        "--corpus".into(),
        path(corpus)?,
        "--receipt".into(),
        path(&input)?,
        "--expected-corpus-sha256".into(),
        hash.into(),
        "--output".into(),
        path(&output)?,
    ];
    if let Some(selection) = selection {
        args.extend(["--selection".into(), path(selection)?]);
    }
    scorer.python(&args).await?;
    Ok(output)
}
#[path = "evaluation_failure.rs"]
mod failure;
async fn arm(
    state: &AppState,
    admin: &PgPool,
    auth: &Auth,
    inputs: &BTreeMap<Uuid, Vec<ShadowSource>>,
    mode: Mode,
    params: HybridParameters,
    gds: &str,
) -> Result<Value> {
    let mut reports = vec![];
    let mut failures = vec![];
    for (brain, sources) in inputs {
        let report = match Box::pin(shadow::queue(
            state, auth, *brain, sources, mode, params, gds,
        ))
        .await
        {
            Ok(id) => id,
            Err(e) => {
                failures.push(json!({"brain_id":brain,"state":"failed","error_code":e.1}));
                continue;
            }
        };
        let expected_job:Uuid=sqlx::query_scalar("SELECT job_id FROM analytics_reports WHERE brain_id=$1 AND id=$2 AND purpose='organization_shadow'").bind(brain).bind(report).fetch_one(admin).await?;
        let job = worker::claim(&state.pool, "heavy")
            .await?
            .context("Expected bounded shadow heavy job missing")?;
        ensure!(
            job.id == expected_job
                && job.brain_id == *brain
                && job.target_id == report
                && job.actor_id == auth.user.id
                && job.kind == "graph.analyze",
            "Refuse to execute any unrelated heavy job"
        );
        if let Err(failure) = Box::pin(worker::execute(state, &job)).await {
            worker::fail(&state.pool, &job, worker::Failure::Unsupported).await?;
            let cancelled =
                failure::cancel_failed(state, admin, auth, *brain, report, expected_job).await?;
            failures.push(json!({"brain_id":brain,"report_id":report,"state":"failed","worker_failure":format!("{failure:?}"),"owned_job_cleanup":cancelled}));
            continue;
        }
        match shadow::result(state, auth, *brain, report).await {
            Ok(result) => reports.push(serde_json::to_value(result)?),
            Err(e) => {
                let cancelled =
                    failure::cancel_failed(state, admin, auth, *brain, report, expected_job)
                        .await?;
                failures.push(json!({"brain_id":brain,"report_id":report,"state":"failed","error_code":e.1,"owned_job_cleanup":cancelled}));
            }
        }
    }
    Ok(
        json!({"name":mode,"state":if failures.is_empty(){"completed"}else{"failed"},"reports":reports,"failures":failures}),
    )
}
/// No calls to a provider, learning executor, support assessor, profile selector
/// or production publisher occur here. Admission independently verifies the
/// already completed gateway request's immutable mapping digest/source binding.
pub async fn execute(
    state: &AppState,
    admin: &PgPool,
    login: &legacy::Login,
    actor: Uuid,
    recipe: &Value,
    out: &Path,
    all: &[(Value, Imported)],
) -> Result<Value> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let dir = out.join("real-organization");
    fs::create_dir(&dir)?;
    let auth = evaluator(state, login).await?;
    ensure!(auth.user.id == actor, "Owned browser evaluator changed");
    isolate_heavy(state, admin, login, &dir).await?;
    ensure!(
        recipe["evaluation"]["arm_definitions"] == arm_definitions(),
        "Frozen comparison arms drifted"
    );
    let gds = recipe["evaluation"]["expected_gds_version"]
        .as_str()
        .context("Frozen native engine version required")?;
    let recipe_hash = legacy::sha256(&serde_json::to_vec(recipe)?)?;
    let scorer = Scorer {
        root: &root,
        sha256: recipe["code_sha256"]["scripts/score-real-topics.py"]
            .as_str()
            .context("Frozen scalar scorer hash required")?,
    };
    let oldpath = root.join(OLD20);
    ensure!(
        legacy::sha256(&fs::read(&oldpath)?)? == OLD20_HASH,
        "Original diagnostic/development corpus changed"
    );
    // Only the old development corpus is opened before selection. The fresh
    // corpus and fresh model outputs are not read by this evaluator until its
    // create_new+fsync selection file has fixed the tuple (or recorded failure).
    let old: Value = serde_json::from_slice(&fs::read(&oldpath)?)?;
    let development: Vec<_> = old["documents"]
        .as_array()
        .context("Original development cohort")?
        .iter()
        .filter(|d| d["partition"] == "development")
        .map(|d| d["id"].as_str().unwrap().to_owned())
        .collect();
    ensure!(
        development.len() == 12,
        "Exactly12original development documents required"
    );
    let dev = Box::pin(prepare(state, admin, &auth, all, |doc| {
        doc["corpus"] == "adversarial20" && development.iter().any(|id| doc["id"] == *id)
    }))
    .await?;
    let mut grid = vec![];
    for k in [3, 5, 8] {
        for c in [0.65, 0.75, 0.85] {
            for s in [0.25, 0.5, 1.0] {
                let params = HybridParameters {
                    neighbors: k,
                    minimum_cosine: c,
                    vector_scale: s,
                };
                let baseline = arm(
                    state,
                    admin,
                    &auth,
                    &dev.inputs,
                    Mode::NativeHybrid,
                    params,
                    gds,
                )
                .await?;
                let receipt = json!({"schema":"recollect-real-organization-evaluation-1","input_mode":"actual_unpublished_candidates_with_retained_vectors","phase":"development_grid","recipe_sha256":recipe_hash,"corpus_sha256":OLD20_HASH,"parameters":params,"arm_definitions":arm_definitions(),"sources":dev.records,"baselines":[baseline]});
                grid.push(
                    scored(
                        &scorer,
                        &dir,
                        &format!("dev-{k}-{c}-{s}"),
                        &oldpath,
                        OLD20_HASH,
                        &receipt,
                        None,
                    )
                    .await?,
                );
            }
        }
    }
    let selection = dir.join("development-selection.json");
    let mut args = vec!["--select-grid".into()];
    for score in &grid {
        args.push(path(score)?);
    }
    args.extend(["--output".into(), path(&selection)?]);
    scorer.python(&args).await?;
    let selected: Value = serde_json::from_slice(&fs::read(&selection)?)?;
    let selection_hash = legacy::sha256(&fs::read(&selection)?)?;
    if selected["selected"] != true {
        let receipt = json!({"state":"development_precision_gate_failed","development_selection_sha256":selection_hash,"development_trials":27,"diagnostic20":"not_measured","fresh12":"not_opened_or_evaluated_by_evaluator","request_preparation":"Proposal scheduling and guarded provider calls already used frozen fresh source text; no fresh model output or grouping metric influenced development selection","production_recipe_qualified":false,"raw_vectors_exported":false});
        legacy::private_json(&dir.join("summary.json"), &receipt)?;
        return Ok(receipt);
    }
    let params: HybridParameters = serde_json::from_value(selected["parameters"].clone())?;
    // The frozen tuple now exists durably; only now open untouched held-out gold
    // and prepare its actual request outputs for admission and five-arm scoring.
    let freshpath = root.join(proposal::FRESH);
    ensure!(
        legacy::sha256(&fs::read(&freshpath)?)? == proposal::FRESH_HASH,
        "Untouched held-out corpus changed"
    );
    let mut summaries = vec![];
    for (phase, corpus, hash, kind) in [
        ("diagnostic20", oldpath, OLD20_HASH, "adversarial20"),
        ("fresh12", freshpath, proposal::FRESH_HASH, "fresh12"),
    ] {
        let prepared = Box::pin(prepare(state, admin, &auth, all, |doc| {
            doc["corpus"] == kind
        }))
        .await?;
        let mut baselines = vec![];
        for mode in [
            Mode::NativeHybrid,
            Mode::NativeWeighted,
            Mode::EntityOverlap,
            Mode::Metadata,
            Mode::EligibleVector,
        ] {
            baselines.push(arm(state, admin, &auth, &prepared.inputs, mode, params, gds).await?);
        }
        let receipt = json!({"schema":"recollect-real-organization-evaluation-1","input_mode":"actual_unpublished_candidates_with_retained_vectors","phase":phase,"recipe_sha256":recipe_hash,"corpus_sha256":hash,"parameters":params,"arm_definitions":arm_definitions(),"sources":prepared.records,"baselines":baselines,"development_selection_sha256":selection_hash});
        let score = scored(
            &scorer,
            &dir,
            phase,
            &corpus,
            hash,
            &receipt,
            Some(&selection),
        )
        .await?;
        summaries.push(json!({"phase":phase,"score_artifact":score.file_name().unwrap(),"score_sha256":legacy::sha256(&fs::read(&score)?)?}));
    }
    let receipt = json!({"state":"comparison_complete_manual_quality_review_required","development_selection_sha256":selection_hash,"development_trials":27,"phases":summaries,"production_recipe_qualified":false,"raw_vectors_exported":false});
    legacy::private_json(&dir.join("summary.json"), &receipt)?;
    Ok(receipt)
}
struct Cohort {
    inputs: BTreeMap<Uuid, Vec<ShadowSource>>,
    records: Vec<Value>,
}
async fn prepare(
    state: &AppState,
    admin: &PgPool,
    auth: &Auth,
    all: &[(Value, Imported)],
    select: impl Fn(&Value) -> bool,
) -> Result<Cohort> {
    let mut inputs = BTreeMap::<Uuid, Vec<ShadowSource>>::new();
    let mut records = vec![];
    for (doc, source) in all.iter().filter(|(doc, _)| select(doc)) {
        let outcome = &doc["extraction_outcome"];
        let request = outcome["request"]["id"]
            .as_str()
            .map(str::parse::<Uuid>)
            .transpose()?;
        let candidate =
            if outcome["state"] == "validated" && outcome["request"]["state"] == "succeeded" {
                Some(api(shadow::admit(
                    state,
                    auth,
                    model_gateway::Context {
                        brain: source.brain,
                        actor: auth.user.id,
                        device: Some(source.device),
                    },
                    request.context("Validated candidate request missing")?,
                    source.version,
                    &outcome["raw_output"]["mapping"],
                )
                .await)?)
            } else {
                None
            };
        inputs.entry(source.brain).or_default().push(ShadowSource {
            source_version_id: source.version,
            candidate_id: candidate,
        });
        // Metadata only. Exact retained vectors/centroids never leave PostgreSQL.
        let entries:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('entry_id',e.id,'profile_id',e.profile_id,'source_chunk_id',c.id,'source_version_id',e.source_version_id,'request_id',e.request_id,'ordinal',c.ordinal,'byte_start',c.byte_start,'byte_end',c.byte_end,'chunk_sha256',encode(sha256(convert_to(c.content,'UTF8')),'hex'),'state',e.state,'truncated',e.truncated,'compatible',e.profile_id=h.profile_id,'embedding_sha256',CASE WHEN e.embedding IS NULL THEN NULL ELSE encode(sha256(convert_to(e.embedding::text,'UTF8')),'hex') END) FROM source_chunks c JOIN semantic_entries e ON e.brain_id=c.brain_id AND e.input_id=c.id AND e.source_version_id=c.version_id AND e.kind='source_chunk' JOIN semantic_heads h ON h.brain_id=e.brain_id WHERE c.brain_id=$1 AND c.version_id=$2 AND e.profile_id=h.profile_id ORDER BY c.ordinal").bind(source.brain).bind(source.version).fetch_all(admin).await?;
        records.push(json!({"id":doc["id"],"brain_id":source.brain,"source_id":source.source,"source_version_id":source.version,"title":doc["title"],"text_sha256":doc["text_sha256"],"canonical_selection":source.selection,"extraction":{"request_id":request,"state":outcome["request"]["state"],"mapping_state":outcome["state"],"prompt_label":outcome["prompt_label"],"schema_label":outcome["schema_label"]},"vector_entries":entries}));
    }
    records.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    for source in inputs.values_mut() {
        source.sort_by_key(|s| s.source_version_id);
    }
    Ok(Cohort { inputs, records })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn changed_scorer_is_rejected_before_python_can_execute() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../.cache/automatic-mapping/scorer-drift-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(root.join("scripts")).unwrap();
        let script = root.join("scripts/score-real-topics.py");
        let frozen = b"raise SystemExit(19)\n";
        fs::write(&script, frozen).unwrap();
        let sha256 = legacy::sha256(frozen).unwrap();
        fs::write(
            &script,
            "from pathlib import Path\nPath('unapproved-execution').touch()\n",
        )
        .unwrap();
        let scorer = Scorer {
            root: &root,
            sha256: &sha256,
        };
        let error = scorer
            .python(&[])
            .await
            .expect_err("Drifted Python must never run");
        assert!(error.to_string().contains("Frozen scorer changed"));
        assert!(!root.join("unapproved-execution").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
