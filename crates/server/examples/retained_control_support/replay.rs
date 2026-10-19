//! Exact retained extraction replay through NEW native canonical workflows.
use crate::{ORIGINAL, OUTCOMES_SHA, common, fixture, freshness, guard, hash, read, retained};
use anyhow::{Context, Result, ensure};
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use recollect_server::{AppState, config::Config, db, model_gateway as gateway, worker};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;
use uuid::Uuid;

fn id(v: &Value) -> Result<Uuid> {
    Ok(v.as_str().context("Canonical UUID missing")?.parse()?)
}
fn typed(value: &mut Value, map: &BTreeMap<String, String>) {
    if let Some(s) = value.as_str().and_then(|s| map.get(s)) {
        *value = json!(s)
    }
}
fn identity_map(map: &Value) -> Result<BTreeMap<String, String>> {
    map.as_object()
        .context("Declared canonical map")?
        .values()
        .map(|v| {
            Ok((
                v["actual"].as_str().context("Actual identity")?.into(),
                v["frozen"].as_str().context("Frozen identity")?.into(),
            ))
        })
        .collect()
}
fn content(mut value: Value, map: &BTreeMap<String, String>) -> Value {
    if let Some(o) = value.as_object_mut() {
        if let Some(s) = o.get_mut("selection") {
            if let Some(v) = s.get_mut("environment_id") {
                typed(v, map)
            }
            for k in ["repository_ids", "area_ids"] {
                if let Some(a) = s.get_mut(k).and_then(Value::as_array_mut) {
                    for v in a {
                        typed(v, map)
                    }
                }
            }
        }
        if let Some(v) = o.get_mut("manifest_revision_id") {
            typed(v, map)
        }
        if let Some(a) = o.get_mut("supports").and_then(Value::as_array_mut) {
            for s in a {
                if let Some(v) = s.get_mut("id") {
                    typed(v, map)
                }
            }
        }
    }
    value
}
fn payload(mut value: Value, map: &BTreeMap<String, String>) -> Value {
    if let Some(a) = value["claims"].as_array_mut() {
        for c in a {
            c["content"] = content(c["content"].clone(), map);
            for k in ["replaces_revision", "reuses_revision"] {
                if let Some(v) = c.get_mut(k) {
                    typed(v, map)
                }
            }
        }
    }
    if let Some(a) = value["retirements"].as_array_mut() {
        for c in a {
            if let Some(v) = c.get_mut("revision_id") {
                typed(v, map)
            }
        }
    }
    value
}
fn source_provenance(value: &Value, map: &BTreeMap<String, String>) -> Result<Value> {
    let mut p = value.clone();
    for k in ["source_id", "version_id", "contributor_id"] {
        if let Some(v) = p.get_mut(k) {
            typed(v, map)
        }
    }
    for k in ["observed_at", "recorded_at"] {
        if let Some(v) = p.get_mut(k).filter(|v| !v.is_null()) {
            chrono::DateTime::parse_from_rfc3339(v.as_str().context("Canonical time")?)?;
            *v = json!({"new_canonical_runtime_timestamp":k});
        }
    }
    if let Some(c) = p.get_mut("capture").filter(|v| !v.is_null()) {
        for k in ["event_id", "binding_id", "operation_id"] {
            if let Some(v) = c.get_mut(k).filter(|v| !v.is_null()) {
                id(v)?;
                *v = json!({"new_canonical_capture_identity":k});
            }
        }
        for k in ["actor_id", "device_id"] {
            if let Some(v) = c.get_mut(k) {
                typed(v, map)
            }
        }
        for k in ["captured_at", "received_at"] {
            if let Some(v) = c.get_mut(k).filter(|v| !v.is_null()) {
                chrono::DateTime::parse_from_rfc3339(
                    v.as_str().context("Canonical capture time")?,
                )?;
                *v = json!({"new_canonical_runtime_timestamp":k});
            }
        }
        if let Some(v) = c.get_mut("selection") {
            *v = content(json!({"selection":v.clone()}), map)["selection"].clone();
        }
    }
    Ok(p)
}
fn replay_output(old: &Value, new_map: &Value) -> Result<(Value, Value)> {
    let mut output = old["raw_output"].clone();
    let old_target = old["canonical_id_map"]["prior_revision"]["actual"]
        .as_str()
        .context("Old prior target")?;
    let new_target = new_map["prior_revision"]["actual"]
        .as_str()
        .context("New prior target")?;
    let mut changed = vec![];
    for (n, c) in output["claims"]
        .as_array_mut()
        .context("All raw claims")?
        .iter_mut()
        .enumerate()
    {
        if c["replaces_revision"].as_str() == Some(old_target) {
            c["replaces_revision"] = json!(new_target);
            changed.push(format!("/claims/{n}/replaces_revision"));
        }
    }
    for (n, c) in output["retirements"]
        .as_array_mut()
        .context("All raw retirements")?
        .iter_mut()
        .enumerate()
    {
        if c["revision_id"].as_str() == Some(old_target) {
            c["revision_id"] = json!(new_target);
            changed.push(format!("/retirements/{n}/revision_id"));
        }
    }
    // Unknown/unoffered UUIDs stay untouched. Never do free string replacement.
    let mut inverse = output.clone();
    for p in &changed {
        *inverse.pointer_mut(p).context("Typed changed pointer")? = json!(old_target);
    }
    ensure!(
        inverse == old["raw_output"],
        "Replay altered free text, mapping or an undeclared identity"
    );
    let remap = json!({"original_raw_output_sha256":hash(&old["raw_output"])? ,"remapped_raw_output_sha256":hash(&output)?,"typed_changed_pointers":changed,"old_target":old_target,"new_target":new_target,"unknown_targets_preserved":true});
    Ok((output, remap))
}

async fn support_lineage(
    pool: &PgPool,
    brain: Uuid,
    version: Uuid,
    map: &BTreeMap<String, String>,
) -> Result<Value> {
    let mut lineage:Value=sqlx::query_scalar("SELECT jsonb_build_object('version_id',v.id,'source_id',v.source_id,'media_type',v.media_type,'retention_class',v.retention_class,'processing',v.processing,'privacy_state',v.privacy_state,'byte_length',v.byte_length,'text_sha256',(SELECT encode(sha256(convert_to(string_agg(c.content,'' ORDER BY c.ordinal),'UTF8')),'hex') FROM source_chunks c WHERE c.brain_id=v.brain_id AND c.version_id=v.id),'automatic_excerpt',(SELECT jsonb_build_object('parent_version_id',e.parent_version_id,'parent_source_id',(SELECT parent.source_id FROM source_versions parent WHERE parent.brain_id=e.brain_id AND parent.id=e.parent_version_id),'first_line',e.first_line,'last_line',e.last_line,'selection',e.selection,'manifest_revision_id',e.manifest_revision_id,'original_class',e.original_class,'source_role',e.provenance->'role','privacy_state',e.privacy_state) FROM automatic_support_excerpts e WHERE e.brain_id=v.brain_id AND e.version_id=v.id)) FROM source_versions v WHERE v.brain_id=$1 AND v.id=$2")
        .bind(brain).bind(version).fetch_one(pool).await?;
    let actual = json!({"version_id":lineage["version_id"],"source_id":lineage["source_id"]});
    lineage
        .as_object_mut()
        .context("Evidence lineage")?
        .remove("version_id");
    lineage
        .as_object_mut()
        .context("Evidence lineage")?
        .remove("source_id");
    if let Some(e) = lineage
        .get_mut("automatic_excerpt")
        .filter(|e| !e.is_null())
    {
        typed(&mut e["parent_version_id"], map);
        typed(&mut e["parent_source_id"], map);
        e["selection"] = content(json!({"selection":e["selection"]}), map)["selection"].clone();
        typed(&mut e["manifest_revision_id"], map);
    }
    Ok(json!({"actual":actual,"semantics":lineage}))
}

async fn original_snapshot(root: &Path, rows: &[Value]) -> Result<Value> {
    let state = read(&root.join(ORIGINAL).join("owned-state.json"))?;
    let name = state["database"]
        .as_str()
        .context("Original owned database")?;
    ensure!(
        name.starts_with("recollect_extractor_")
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "Original owned database identity"
    );
    let pool = db::pool(&common::database_url(
        &std::env::var("DATABASE_ADMIN_URL")?,
        name,
    )?)
    .await?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await?;
    let owner:Option<String>=sqlx::query_scalar("SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database()").fetch_one(&mut *tx).await?;
    ensure!(
        owner.as_deref() == Some(common::OWNER),
        "Original owned database marker changed"
    );
    let mut records = vec![];
    for r in rows {
        let brain = id(&r["brain_id"])?;
        let run = id(&r["canonical_id_map"]["run"]["actual"])?;
        let safe:bool=sqlx::query_scalar("SELECT b.archived AND r.state='cancelled' AND j.state='cancelled' AND NOT EXISTS(SELECT1 FROM learning_support_stages WHERE brain_id=b.id AND run_id=r.id) FROM brains b JOIN learning_runs r ON r.brain_id=b.id JOIN jobs j ON j.id=r.job_id WHERE b.id=$1 AND r.id=$2".replace("SELECT1","SELECT 1").as_str()).bind(brain).bind(run).fetch_one(&mut *tx).await?;
        ensure!(
            safe,
            "Original completed controls must remain archived/cancelled without stages"
        );
        let prior = id(&r["canonical_id_map"]["prior_revision"]["actual"])?;
        let revision: Value =
            sqlx::query_scalar("SELECT revision FROM claim_revisions WHERE brain_id=$1 AND id=$2")
                .bind(brain)
                .bind(prior)
                .fetch_one(&mut *tx)
                .await?;
        let mut declared = r["canonical_id_map"].clone();
        let mut lineages = vec![];
        for (n, support) in revision["content"]["supports"]
            .as_array()
            .context("Original prior supports")?
            .iter()
            .enumerate()
        {
            if support["kind"] != "source_version" {
                continue;
            }
            let version = id(&support["id"])?;
            if identity_map(&declared)?.contains_key(&version.to_string()) {
                continue;
            }
            let lineage = support_lineage(&pool, brain, version, &identity_map(&declared)?).await?;
            ensure!(
                !lineage["semantics"]["automatic_excerpt"].is_null(),
                "Undeclared prior support lacks exact canonical excerpt lineage"
            );
            declared[format!("prior_support_version_{n}")] =
                json!({"actual":version,"frozen":format!("prior-support-excerpt-version-{n}")});
            declared[format!("prior_support_source_{n}")] = json!({"actual":lineage["actual"]["source_id"],"frozen":format!("prior-support-excerpt-source-{n}")});
            lineages.push(json!({"index":n,"lineage":lineage}));
        }
        let map = identity_map(&declared)?;
        records.push(json!({"id":r["id"],"canonical_id_map":declared,"prior_revision":revision,"prior_support_lineages":lineages,"prior_content_semantics":content(revision["content"].clone(),&map),"actual_offered_revision_ids":r["actual_offered_revision_ids"],"original_run_stays_cancelled":true}));
    }
    tx.commit().await?;
    pool.close().await;
    Ok(
        json!({"schema":"retained-control-original-canonical-snapshot-1","control_outcomes_sha256":OUTCOMES_SHA,"cases":records,"read_only":true,"original_database_mutated":false}),
    )
}

#[derive(Clone)]
struct Routing {
    active: Arc<Mutex<Option<Active>>>,
    out: PathBuf,
    admin: PgPool,
    mode: Mode,
    ledger: Arc<Mutex<guard::Ledger>>,
    client: reqwest::Client,
    root: PathBuf,
    config: Config,
    state: AppState,
}
#[derive(Clone)]
enum Mode {
    Controlled,
    Paid {
        recipe: Arc<Value>,
        key: Arc<String>,
    },
}
struct Active {
    case_id: String,
    brain: Uuid,
    run: Uuid,
    source: Uuid,
    offered: Vec<Uuid>,
    map: Value,
    source_text: String,
    source_class: String,
    source_provenance: Value,
    replay: Value,
    expected_extraction: Value,
    remap_receipt: Value,
    extracted: bool,
    assessed: bool,
    record: Option<Value>,
    expected: Option<Value>,
}

fn wire(model: &Value, output: Value) -> Value {
    json!({"model":model,"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":output.to_string()}]}],"usage":{"input_tokens":23,"output_tokens":17,"total_tokens":40}})
}
async fn route(State(r): State<Routing>, bytes: Bytes) -> Response {
    match route_inner(&r, &bytes).await {
        Ok((status, body)) => (status, axum::Json(body)).into_response(),
        Err(error) => {
            let name = format!("router-rejection-{}.json", Uuid::new_v4());
            let ledger = r.ledger.lock().await.summary();
            let _ = common::private_json(
                &r.out.join(name),
                &json!({"error":error.to_string(),"body_sha256":common::sha256(&bytes).ok(),"bytes":bytes.len(),"ledger_at_failure":ledger,"transmission_status":"See durable ledger; a post-reservation failure may have transmitted and must never be replayed"}),
            );
            (StatusCode::BAD_REQUEST,axum::Json(json!({"error":{"code":"support6_router_denied","message":"Exact owned support replay preflight rejected"}}))).into_response()
        }
    }
}

async fn route_inner(r: &Routing, bytes: &[u8]) -> Result<(StatusCode, Value)> {
    ensure!(
        bytes.len() <= 65536,
        "Complete current request exceeds64KiB"
    );
    let body: Value = serde_json::from_slice(bytes)?;
    let mut locked = r.active.lock().await;
    let a = locked
        .as_mut()
        .context("No exact owned current control registered")?;
    ensure!(
        body["model"] == r.config.models.text_model,
        "Unexpected model"
    );
    if body["text"]["format"]["name"] == "source_reconciliation" {
        ensure!(
            !a.extracted && !a.assessed && body == a.expected_extraction,
            "Unexpected/duplicate/changed extraction replay body"
        );
        common::private_json(
            &r.out.join(format!("{}-extraction-body.json", a.case_id)),
            &body,
        )?;
        a.extracted = true;
        return Ok((StatusCode::OK, wire(&body["model"], a.replay.clone())));
    }
    ensure!(
        a.extracted && !a.assessed && body["text"]["format"]["name"] == "source_support",
        "Only one current support request after exact replay is allowed"
    );
    let state:Value=sqlx::query_scalar("SELECT jsonb_build_object('payload',s.payload,'assessment_operation',s.assessment_operation_id,'extraction_request',s.extraction_request_id,'run',to_jsonb(r),'request_inputs',(SELECT jsonb_agg(jsonb_build_object('kind',i.kind,'id',i.input_id) ORDER BY i.kind,i.input_id) FROM model_request_inputs i JOIN model_requests q ON q.id=i.request_id AND q.brain_id=i.brain_id WHERE q.brain_id=s.brain_id AND q.operation_id=s.assessment_operation_id)) FROM learning_support_stages s JOIN learning_runs r ON r.brain_id=s.brain_id AND r.id=s.run_id WHERE s.brain_id=$1 AND s.run_id=$2 AND s.verifier_version='source-support-3' AND s.verdicts IS NULL AND s.privacy_state='active'")
        .bind(a.brain).bind(a.run).fetch_one(&r.admin).await?;
    let mut inputs: Vec<Value> = state["request_inputs"]
        .as_array()
        .context("Actual current support input links")?
        .clone();
    inputs.sort_by_key(|v| v["kind"].to_string() + &v["id"].to_string());
    let mut expected = vec![
        json!({"kind":"source_version","id":a.source}),
        json!({"kind":"learning_support_stage","id":a.run}),
    ];
    expected.extend(
        a.offered
            .iter()
            .map(|v| json!({"kind":"claim_revision","id":v})),
    );
    expected.sort_by_key(|v| v["kind"].to_string() + &v["id"].to_string());
    ensure!(
        inputs == expected,
        "Unexpected source/stage/offered target IDs"
    );
    let map = identity_map(&a.map)?;
    let normalized = payload(state["payload"].clone(), &map);
    let target: Value =
        sqlx::query_scalar("SELECT revision FROM claim_revisions WHERE brain_id=$1 AND id=$2")
            .bind(a.brain)
            .bind(id(&a.map["prior_revision"]["actual"])?)
            .fetch_one(&r.admin)
            .await?;
    let semantics = json!({"id":a.case_id,"normalized_payload":normalized,"source_text_sha256":common::sha256(a.source_text.as_bytes())?,"source_class":a.source_class,"source_provenance_semantics":source_provenance(&a.source_provenance,&map)?,"source_selection":content(json!({"selection":state["run"]["selection"]}),&map),"prior_content":content(target["content"].clone(),&map),"offered_prior":!a.offered.is_empty(),"instructions_sha256":common::sha256(body["instructions"].as_str().context("Support prompt missing")?.as_bytes())?,"schema_sha256":hash(&body["text"]["format"]["schema"])?});
    if let Some(e) = &a.expected {
        ensure!(
            e == &semantics,
            "Normalized action/source/target/prompt/schema semantics differ from no-paid freeze"
        );
    }
    let mut invocation_inputs = vec![
        gateway::InputRef {
            kind: "source_version".into(),
            id: a.source,
        },
        gateway::InputRef {
            kind: "learning_support_stage".into(),
            id: a.run,
        },
    ];
    invocation_inputs.extend(a.offered.iter().map(|id| gateway::InputRef {
        kind: "claim_revision".into(),
        id: *id,
    }));
    let invocation = gateway::Invocation {
        operation: id(&state["assessment_operation"])?,
        purpose: "extraction".into(),
        inputs: invocation_inputs,
        query: None,
        instructions: body["instructions"]
            .as_str()
            .context("Support instructions")?
            .into(),
        prompt_label: "source-support-3".into(),
        schema_label: "source-support-3".into(),
        format: gateway::Format::Json {
            name: "source_support".into(),
            schema: body["text"]["format"]["schema"].clone(),
        },
        work_lease: None,
        metadata_replay: false,
        expected_json: None,
    };
    let canonical = gateway::preview_for_policy(
        &r.state,
        gateway::Context {
            brain: a.brain,
            actor: id(&state["run"]["actor_id"])?,
            device: state["run"]["device_id"]
                .as_str()
                .map(str::parse)
                .transpose()?,
        },
        &invocation,
        id(&state["run"]["policy_id"])?,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Fresh canonical support preview failed ({})", e.1))?;
    guard::canonical_body(bytes, &canonical.body)?;
    let mut wire_probes = 0;
    if matches!(r.mode, Mode::Controlled) {
        // Change actual source, candidate and target wire data independently.
        // These must reject before any reservation or transport is permitted.
        for class in ["source", "candidate", "target"] {
            let mut changed = canonical.body.clone();
            let mut inputs: Vec<Value> = changed["input"]
                .as_str()
                .context("Canonical wire input")?
                .lines()
                .map(serde_json::from_str)
                .collect::<std::result::Result<_, _>>()?;
            if class == "source" {
                let source = inputs
                    .iter_mut()
                    .find(|i| i["provenance"]["kind"] == "source_version")
                    .context("Exact source wire record")?;
                source["data"] = json!(format!(
                    "{}\nCONTROLLED_ALTERED_SOURCE",
                    source["data"].as_str().context("Source data")?
                ));
            } else {
                let stage = inputs
                    .iter_mut()
                    .find(|i| i["provenance"]["kind"] == "learning_support_stage")
                    .context("Exact stage wire record")?;
                let mut data: Value =
                    serde_json::from_str(stage["data"].as_str().context("Stage data")?)?;
                ensure!(
                    data["actions"].as_array().is_some_and(|a| !a.is_empty()),
                    "Wire probe needs actual action"
                );
                if class == "candidate" {
                    data["actions"][0]["value"] = json!("CONTROLLED_ALTERED_CANDIDATE");
                } else {
                    data["actions"][0]["replaces_revision"] = json!(Uuid::new_v4());
                }
                stage["data"] = json!(data.to_string());
            }
            changed["input"] = json!(
                inputs
                    .iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            ensure!(
                guard::canonical_body(&serde_json::to_vec(&changed)?, &canonical.body).is_err(),
                "Changed wire input admitted"
            );
            wire_probes += 1;
        }
    }
    let h = common::sha256(bytes)?;
    let instructions = body["instructions"]
        .as_str()
        .context("Support instructions")?;
    guard::support_body(
        bytes,
        &h,
        &r.config.models.text_model,
        instructions,
        &body["text"]["format"]["schema"],
    )?;
    common::private_json(
        &r.out.join(format!("{}-support-body.json", a.case_id)),
        &body,
    )?;
    common::private_json(
        &r.out.join(format!("{}-typed-stage.json", a.case_id)),
        &state,
    )?;
    let op = id(&state["assessment_operation"])?;
    // Read-only freshness is repeated immediately before EVERY reservation/send.
    if let Mode::Paid { recipe, .. } = &r.mode {
        freshness(&r.root, &r.config, recipe)?;
        common::validate_pricing(&recipe["pricing"], &r.config.models.text_model)?;
    }
    r.ledger.lock().await.reserve(&a.case_id,op,bytes,&h,json!({"brain_id":a.brain,"run_id":a.run,"source_version_id":a.source,"semantic_sha256":hash(&semantics)?,"body_artifact":format!("{}-support-body.json",a.case_id)}))?;
    a.assessed = true;
    let mut record = json!({"semantics":semantics,"raw_claim_count":a.replay["claims"].as_array().map_or(0,Vec::len),"normalized_claim_count":state["payload"]["claims"].as_array().map_or(0,Vec::len),"normalized_retirement_count":state["payload"]["retirements"].as_array().map_or(0,Vec::len),"assessment_operation":op,"support_body_sha256":h,"full_body_bytes":bytes.len(),"remap":a.remap_receipt,"typed_stage_created_by":"actual learning executor","source_stage_injected":false,"actual_wire_equals_fresh_canonical_gateway_body":true,"changed_wire_probes_rejected_before_reservation":wire_probes,"exact_source_provenance":a.source_provenance});
    a.record = Some(record.clone());
    let response = match &r.mode {
        Mode::Controlled => {
            let lines: Vec<&str> = a.source_text.lines().collect();
            let claims = state["payload"]["claims"]
                .as_array()
                .context("Normalized claims")?;
            let retirements = state["payload"]["retirements"]
                .as_array()
                .context("Normalized retirements")?;
            let mut assessments = vec![];
            for index in 0..claims.len() + retirements.len() {
                let (first, last) = if index < claims.len() {
                    let s = &claims[index]["content"]["supports"][0];
                    (s["line_from"].as_u64(), s["line_to"].as_u64())
                } else {
                    let s = &retirements[index - claims.len()];
                    (s["line_from"].as_u64(), s["line_to"].as_u64())
                };
                let (first, last) = (
                    first.context("First cited line")? as usize,
                    last.context("Last cited line")? as usize,
                );
                ensure!(
                    first >= 1 && first <= last && last <= lines.len(),
                    "Required cited lines unavailable"
                );
                let quote = lines[first - 1..last].join("\n");
                let disposition = match a.case_id.as_str() {
                    "rc01" => "insufficient",
                    "rc02" | "rc03" => "contradicted",
                    _ => "supported",
                };
                let citation = disposition == "supported";
                let quote = if a.case_id == "rc05" {
                    "CONTROLLED_UNCITED_QUOTE_SENTINEL_NOT_IN_SOURCE".into()
                } else if citation {
                    quote
                } else {
                    String::new()
                };
                assessments.push(json!({"index":index,"disposition":disposition,"citation_support":citation,"evidence_quote":quote,"reason":"Controlled loopback enforcement probe only; never semantic quality evidence"}));
            }
            record["controlled_probe"] = json!(match a.case_id.as_str() {
                "rc01" => "insufficient preserves head",
                "rc02" => "contradicted historical replacement preserves head",
                "rc03" =>
                    "contradicted preserves head; missing required retirement remains failure",
                "rc05" => "invalid quotation downgrades supported to insufficient",
                _ => "supported publication/identity control; no semantic quality claim",
            });
            (
                StatusCode::OK,
                wire(&body["model"], json!({"assessments":assessments})),
            )
        }
        Mode::Paid { recipe, key } => {
            // One send; redirects, retries and environment proxies are disabled.
            let result = r
                .client
                .post(format!(
                    "{}/responses",
                    recipe["config"]["official_forward_endpoint"]
                        .as_str()
                        .context("Official endpoint")?
                ))
                .bearer_auth(key.as_str())
                .header("content-type", "application/json")
                .body(bytes.to_vec())
                .send()
                .await;
            match result {
                Ok(mut response) => {
                    let status = response.status();
                    let data = async {
                        let mut bytes = Vec::new();
                        while let Some(chunk) = response.chunk().await? {
                            ensure!(
                                bytes.len() + chunk.len() <= 1024 * 1024,
                                "Provider response exceeds1MiB"
                            );
                            bytes.extend_from_slice(&chunk);
                        }
                        Ok::<_, anyhow::Error>(bytes)
                    }
                    .await;
                    match data {
                        Ok(data) => {
                            let parsed = serde_json::from_slice::<Value>(&data);
                            let Ok(parsed) = parsed else {
                                r.ledger.lock().await.finish(&a.case_id,true,json!({"state":"completed_http_but_json_unavailable","http_status":status.as_u16(),"response_sha256":common::sha256(&data)?}))?;
                                bail_uncertain()?;
                                unreachable!()
                            };
                            common::private_json(
                                &r.out.join(format!("{}-provider-response.json", a.case_id)),
                                &parsed,
                            )?;
                            let returned = parsed["model"].as_str();
                            let expected = &r.config.models.text_model;
                            // Exact alias or declared dated API snapshot only;
                            // do not let an unrelated prefix/model pass.
                            let model_ok = returned.is_some_and(|m| {
                                m == expected
                                    || m.strip_prefix(&format!("{expected}-")).is_some_and(|d| {
                                        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok()
                                    })
                            });
                            let violation = !guard::response_bounds(&parsed, expected);
                            r.ledger.lock().await.finish(&a.case_id,violation,json!({"http_status":status.as_u16(),"response_sha256":common::sha256(&data)?,"usage":parsed["usage"],"usage_missing_or_bound_violation":violation,"returned_model":returned,"returned_model_matches":model_ok}))?;
                            if violation {
                                a.record = Some(record);
                                return Ok((
                                    StatusCode::BAD_GATEWAY,
                                    json!({"error":{"code":"support6_response_guard","message":"Usage/model unavailable or outside frozen bounds; no continuation or replay"}}),
                                ));
                            }
                            a.record = Some(record);
                            return Ok((StatusCode::from_u16(status.as_u16())?, parsed));
                        }
                        _ => {
                            r.ledger.lock().await.finish(
                                &a.case_id,
                                true,
                                json!({"state":"response_read_unavailable"}),
                            )?;
                            bail_uncertain()?;
                            unreachable!()
                        }
                    }
                }
                Err(_) => {
                    r.ledger.lock().await.finish(
                        &a.case_id,
                        true,
                        json!({"state":"transmission_outcome_uncertain"}),
                    )?;
                    bail_uncertain()?;
                    unreachable!()
                }
            }
        }
    };
    r.ledger.lock().await.finish(
        &a.case_id,
        false,
        json!({"state":"controlled_loopback_completed","semantic_quality":false}),
    )?;
    a.record = Some(record);
    Ok(response)
}
fn bail_uncertain() -> Result<()> {
    anyhow::bail!(
        "Uncertain external attempt retained; no replay/resumption or subsequent transmission"
    )
}

async fn snapshot(admin: &PgPool, brain: Uuid, run: Uuid) -> Result<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('run',(SELECT to_jsonb(r) FROM learning_runs r WHERE r.brain_id=$1 AND r.id=$2),'claims',coalesce((SELECT jsonb_agg(jsonb_build_object('claim_id',c.id,'current_revision',c.current_revision,'revision',r.revision) ORDER BY c.id) FROM claims c JOIN claim_revisions r ON r.brain_id=c.brain_id AND r.id=c.current_revision WHERE c.brain_id=$1),'[]'::jsonb),'revisions',coalesce((SELECT jsonb_agg(revision ORDER BY recorded_at,id) FROM claim_revisions WHERE brain_id=$1),'[]'::jsonb),'assessments',coalesce((SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM memory_support_assessments a WHERE a.brain_id=$1),'[]'::jsonb),'requests',coalesce((SELECT jsonb_agg(to_jsonb(q)-'call_token' ORDER BY q.created_at,q.id) FROM model_requests q WHERE q.brain_id=$1),'[]'::jsonb),'stage',(SELECT to_jsonb(s) FROM learning_support_stages s WHERE s.brain_id=$1 AND s.run_id=$2))")
        .bind(brain).bind(run).fetch_one(admin).await?)
}

async fn preserve_failure(out: &Path, error: &anyhow::Error) -> Result<()> {
    let ids = ["rc01", "rc02", "rc03", "rc04", "rc05", "rc06"];
    let records:Vec<Value>=ids.iter().map(|id|{let p=out.join(format!("{id}-outcome.json"));let executed=out.join(format!("{id}-executed.json"));if p.exists(){read(&p).unwrap_or(json!({"id":id,"state":"outcome_unreadable_retained"}))}else if executed.exists(){read(&executed).unwrap_or(json!({"id":id,"state":"executed_outcome_unreadable_retained"}))}else{json!({"id":id,"state":"failed_or_unattempted_retained","typed_stage_artifact_available":out.join(format!("{id}-typed-stage.json")).exists(),"source_body_artifact_available":out.join(format!("{id}-extraction-body.json")).exists()})}}).collect();
    common::private_json(
        &out.join("failure-receipt.json"),
        &json!({"status":"failed_all_six_denominators_retained","error":error.to_string(),"controls":records,"cleanup":false,"original_runs_resumed":false,"semantic_quality_claim":false,"paid_authority_granted_by_failure":false}),
    )?;
    Ok(())
}
pub async fn conformance(root: &Path, out: &Path, config: Config) -> Result<()> {
    let result = run(root, out, config, None).await;
    if let Err(e) = &result {
        preserve_failure(out, e).await?;
    }
    result
}
pub async fn paid(root: &Path, out: &Path, config: Config, recipe: &Value) -> Result<()> {
    let result = run(root, out, config, Some(recipe.clone())).await;
    if let Err(e) = &result {
        preserve_failure(out, e).await?;
    }
    result
}

async fn run(root: &Path, out: &Path, config: Config, recipe: Option<Value>) -> Result<()> {
    let (controls, rows) = retained(root)?;
    let originals = if let Some(recipe) = &recipe {
        let proof = Path::new(
            recipe["conformance"]["path"]
                .as_str()
                .context("Proof path")?,
        );
        let p = proof.join("original-canonical-snapshot.json");
        ensure!(
            common::sha256(&fs::read(&p)?)?
                == recipe["conformance"]["semantics"]["original_snapshot_sha256"],
            "Original canonical snapshot changed"
        );
        read(&p)?
    } else {
        original_snapshot(root, &rows).await?
    };
    common::private_json(&out.join("original-canonical-snapshot.json"), &originals)?;
    let real_key = config.models.key.clone();
    let original_config = config.clone();
    let mut isolated = config;
    isolated.models.endpoint = "http://127.0.0.1:9/v1".into();
    isolated.models.key = Some("synthetic-support6-loopback-only".into());
    isolated.models.openrouter_key = None;
    isolated.models.openrouter_endpoint = "http://127.0.0.1:9".into();
    let (state, admin, cluster, database, _login, _actor, _brain, _policy) =
        common::setup(out, isolated).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let mode = if let Some(recipe) = &recipe {
        Mode::Paid {
            recipe: Arc::new(recipe.clone()),
            key: Arc::new(real_key.context("Configured OpenAI key required")?),
        }
    } else {
        Mode::Controlled
    };
    let (per, total) = if let Some(recipe) = &recipe {
        (
            recipe["limits"]["per_request_nano_usd"]
                .as_u64()
                .context("Per-call cost")?,
            recipe["limits"]["total_nano_usd"]
                .as_u64()
                .context("Full cost")?,
        )
    } else {
        (21_811_200, 130_867_200)
    };
    let mut runtime = (*state.config).clone();
    runtime.models.endpoint = format!("http://{address}/v1");
    let state = AppState::new(state.pool.clone(), runtime)?;
    let routing = Routing {
        active: Arc::new(Mutex::new(None)),
        out: out.into(),
        admin: admin.clone(),
        mode,
        ledger: Arc::new(Mutex::new(guard::Ledger::new(
            &out.join("support-cost-ledger.jsonl"),
            per,
            total,
            recipe.is_none(),
        )?)),
        client: reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(120))
            .build()?,
        root: root.into(),
        config: original_config,
        state: state.clone(),
    };
    let server = tokio::spawn(
        axum::serve(
            listener,
            Router::new()
                .route("/v1/responses", post(route))
                .layer(DefaultBodyLimit::max(65537))
                .with_state(routing.clone()),
        )
        .into_future(),
    );
    let mut records = vec![];
    let mut semantics = vec![];
    let mut all = true;
    for (case, old) in controls["cases"]
        .as_array()
        .context("Cases")?
        .iter()
        .zip(&rows)
    {
        if routing.ledger.lock().await.summary()["stopped"] == true {
            records.push(json!({"id":case["id"],"state":"not_attempted_after_uncertain"}));
            all = false;
            continue;
        }
        let mut p = Box::pin(fixture::setup_case(&state, &admin, case)).await?;
        let original = originals["cases"]
            .as_array()
            .context("Original target snapshot")?
            .iter()
            .find(|r| r["id"] == case["id"])
            .context("Original case")?;
        let before = snapshot(&admin, p.brain_id, p.run.id).await?;
        common::private_json(&out.join(format!("{}-before.json", p.case_id)), &before)?;
        let target = before["revisions"]
            .as_array()
            .context("Prior revisions")?
            .iter()
            .find(|r| r["id"] == json!(p.prior_revision_id))
            .context("New canonical target")?;
        for old_lineage in original["prior_support_lineages"]
            .as_array()
            .context("Exact original support lineage")?
        {
            let n = old_lineage["index"].as_u64().context("Evidence index")? as usize;
            let version = id(&target["content"]["supports"][n]["id"])?;
            let lineage = support_lineage(
                &admin,
                p.brain_id,
                version,
                &identity_map(&p.canonical_id_map)?,
            )
            .await?;
            ensure!(
                lineage["semantics"] == old_lineage["lineage"]["semantics"],
                "Corresponding prior support excerpt lineage/text differs"
            );
            p.canonical_id_map[format!("prior_support_version_{n}")] =
                json!({"actual":version,"frozen":format!("prior-support-excerpt-version-{n}")});
            p.canonical_id_map[format!("prior_support_source_{n}")] = json!({"actual":lineage["actual"]["source_id"],"frozen":format!("prior-support-excerpt-source-{n}")});
        }
        ensure!(
            content(
                target["content"].clone(),
                &identity_map(&p.canonical_id_map)?
            ) == original["prior_content_semantics"],
            "Recreated canonical target semantic fields differ from retained original"
        );
        let (output, remap) = replay_output(old, &p.canonical_id_map)?;
        let expected = recipe
            .as_ref()
            .map(|r| {
                r["conformance"]["semantics"]["cases"]
                    .as_array()
                    .context("Frozen cases")
                    .and_then(|a| {
                        a.iter()
                            .find(|c| c["id"] == case["id"])
                            .cloned()
                            .context("Frozen case missing")
                    })
            })
            .transpose()?;
        *routing.active.lock().await = Some(Active {
            case_id: p.case_id.clone(),
            brain: p.brain_id,
            run: p.run.id,
            source: p.run.source_version_id,
            offered: p.offered.clone(),
            map: p.canonical_id_map.clone(),
            source_text: p.source.text.clone(),
            source_class: p.source.class.clone(),
            source_provenance: p.source.provenance.clone(),
            replay: output,
            expected_extraction: p.preview.body.clone(),
            remap_receipt: remap,
            extracted: false,
            assessed: false,
            record: None,
            expected,
        });
        let job = worker::claim(&state.pool, "model")
            .await?
            .context("New exact control job")?;
        ensure!(
            job.id == p.run.job_id
                && job.target_id == p.run.id
                && job.brain_id == p.brain_id
                && job.kind == "source.learn",
            "Unexpected queued model work; no unrelated worker executed"
        );
        let mut wrong = job.clone();
        wrong.lease_token = Uuid::new_v4();
        let state_clone = state.clone();
        let wrong_result =
            tokio::spawn(async move { worker::execute(&state_clone, &wrong).await }).await?;
        ensure!(
            !routing
                .active
                .lock()
                .await
                .as_ref()
                .context("Active")?
                .extracted,
            "Invalid lease transmitted extraction"
        );
        let state_clone = state.clone();
        let exact_job = job.clone();
        let result =
            tokio::spawn(async move { worker::execute(&state_clone, &exact_job).await }).await?;
        let after = snapshot(&admin, p.brain_id, p.run.id).await?;
        let active = routing
            .active
            .lock()
            .await
            .take()
            .context("Completed route")?;
        let mut record = active
            .record
            .unwrap_or(json!({"state":"support_not_reached"}));
        record["id"] = case["id"].clone();
        record["canonical_id_map"] = p.canonical_id_map.clone();
        record["prior_workflow_receipt"] = p.prior_workflow_receipt.clone();
        record["before"] = before.clone();
        record["after"] = after.clone();
        record["worker_result"] = json!(format!("{result:?}"));
        record["invalid_lease_probe_result"] = json!(format!("{wrong_result:?}"));
        record["original_target_semantics_match"] = json!(true);
        record["current_stage_verdicts"] = after["stage"]["verdicts"].clone();
        record["current_canonical_support_assessments"] = json!(
            after["assessments"]
                .as_array()
                .context("Canonical assessment rows")?
                .iter()
                .filter(|s| s["source_stage_id"] == json!(p.run.id))
                .cloned()
                .collect::<Vec<_>>()
        );
        record["prior_canonical_assessment_count"] =
            json!(before["assessments"].as_array().map_or(0, Vec::len));
        record["canonical_learning_dispositions"] = json!({"accepted":after["run"]["accepted"],"proposed":after["run"]["proposed"],"blocked":after["run"]["blocked"],"conflicting":after["run"]["conflicting"],"reused":after["run"]["reused"],"revised":after["run"]["revised"],"retired":after["run"]["retired"],"supported_is_not_human_review":true});
        common::private_json(&out.join(format!("{}-executed.json", p.case_id)), &record)?;
        if let Some(s) = record.get("semantics") {
            semantics.push(s.clone());
        }
        let succeeded = after["run"]["state"] == "succeeded";
        if recipe.is_none() {
            ensure!(
                succeeded,
                "Controlled native replay did not reach terminal publication; retained for review"
            );
            if ["rc01", "rc02", "rc03", "rc05"].contains(&p.case_id.as_str()) {
                ensure!(
                    before["claims"] == after["claims"],
                    "Unsupported/contradicted/uncited action mutated a canonical head"
                );
            }
            if p.case_id == "rc04" {
                ensure!(
                    after["revisions"].as_array().map_or(0, Vec::len)
                        > before["revisions"].as_array().map_or(0, Vec::len),
                    "Grounded owner/usage supported positive control did not publish a revision"
                );
            }
            if p.case_id == "rc06" {
                let current = after["claims"]
                    .as_array()
                    .context("Manual identity")?
                    .iter()
                    .find(|c| c["claim_id"] == json!(p.prior_claim_id))
                    .context("Manual claim still present")?;
                ensure!(
                    current["current_revision"] == json!(p.prior_revision_id),
                    "Manual prior head changed"
                );
            }
            let state_clone = state.clone();
            let done_job = job.clone();
            let _ =
                tokio::spawn(async move { worker::execute(&state_clone, &done_job).await }).await?;
            ensure!(
                snapshot(&admin, p.brain_id, p.run.id).await? == after,
                "Completed lease replay changed stored result"
            );
            record["native_enforcement_checks"] = json!({"invalid_lease_no_requests":true,"completed_run_no_duplicate_request_or_mutation":true,"all_failed_or_missing_semantic_requirements_retained":true,"manual_prior_identity_preserved":p.case_id=="rc06","controlled_only_not_assessor_quality":true});
        }
        common::private_json(&out.join(format!("{}-outcome.json", p.case_id)), &record)?;
        records.push(record);
        all &= succeeded;
        if succeeded {
            Box::pin(fixture::archive_case(&state, &admin, &p)).await?;
        } else {
            break;
        }
    }
    server.abort();
    while records.len() < 6 {
        let i = records.len();
        records.push(
            json!({"id":controls["cases"][i]["id"],"state":"not_attempted_after_terminal_failure"}),
        );
    }
    let manifest = json!({"schema":"support6-normalized-semantic-manifest-1","original_control_outcomes_sha256":OUTCOMES_SHA,"original_snapshot_sha256":common::sha256(&fs::read(out.join("original-canonical-snapshot.json"))?)?,"normalization":"Only typed content selection/support/replacement/reuse/retirement identities use declared frozen map; source provenance top observed_at/recorded_at and capture captured_at/received_at are explicit new runtime timestamps; capture event_id/binding_id/operation_id are new structural identities; no source/assertion/rationale strings edited","cases":semantics});
    common::private_json(&out.join("semantic-manifest.json"), &manifest)?;
    let ledger = routing.ledger.lock().await.summary();
    let gateway_requests: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&admin)
        .await?;
    if recipe.is_none() {
        ensure!(
            gateway_requests == 22,
            "Unexpected model work outside exact10prior+6replay+6support requests"
        );
    }
    let normalized_claims: usize = records
        .iter()
        .map(|r| r["normalized_claim_count"].as_u64().unwrap_or(0) as usize)
        .sum();
    let receipt = json!({"schema":"support6-conformance-or-real-receipt-1","status":if all {"completed"}else{"failed_all_outcomes_retained"},"compiled_runner_sha256":common::sha256(&fs::read(std::env::current_exe()?)?)?,"code_manifest_sha256":hash(&crate::code_bindings(root)?)?,"mode":if recipe.is_none(){"controlled_loopback_enforcement_only"}else{"real_support_on_retained_replay"},"external_requests":if recipe.is_none(){json!(0)}else{ledger["reserved_attempts"].clone()},"gateway_requests":gateway_requests,"all_six_raw_outputs_preserved":true,"raw_claims":10,"raw_retirements":0,"normalized_claims":normalized_claims,"controls":records,"ledger":ledger,"normal_data_copied":false,"source_stages_injected":false,"original_runs_resumed_or_unarchived":false,"semantic_quality_claim":false,"production_activation":false,"cleanup":false,"owned_database":database});
    common::private_json(
        &out.join(if recipe.is_none() {
            "conformance-receipt.json"
        } else {
            "execution-receipt.json"
        }),
        &receipt,
    )?;
    common::private_json(
        &out.join("retained-state.json"),
        &json!({"database":database,"owner":common::OWNER,"reason":"Independent exact audit and explicit cleanup required; no active or failed resource dropped","synthetic_only":true}),
    )?;
    state.pool.close().await;
    admin.close().await;
    cluster.close().await;
    ensure!(
        all,
        "Replay failed; all artifacts/resources retained without retry"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remapping_never_edits_unknown_uuid_or_text() {
        let old = json!({"canonical_id_map":{"prior_revision":{"actual":"11111111-1111-4111-8111-111111111111"}},"raw_output":{"claims":[{"replaces_revision":"22222222-2222-4222-8222-222222222222","rationale":"11111111-1111-4111-8111-111111111111"},{"replaces_revision":"11111111-1111-4111-8111-111111111111"}],"retirements":[],"mapping":{"mentions":[]}}});
        let (new, _) = replay_output(
            &old,
            &json!({"prior_revision":{"actual":"33333333-3333-4333-8333-333333333333"}}),
        )
        .unwrap();
        assert_eq!(new["claims"][0], old["raw_output"]["claims"][0]);
        assert_eq!(
            new["claims"][1]["replaces_revision"],
            "33333333-3333-4333-8333-333333333333"
        );
        assert_eq!(new["mapping"], old["raw_output"]["mapping"]);
    }
}
