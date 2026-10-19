//! Native source, preview, semantic and export path. Both families share one
//! canonical import/vector basis per cohort; no production catalogue writes.
use super::{
    controls, export,
    guard::{self, Arm},
    inputs::{self, Cohort, Source},
    io, native,
    owned::{Owned, api, http},
    scopes::{Imported, Scopes},
    support,
    wire::{self, Active, Wire},
};
use anyhow::{Context, Result, ensure};
use recollect_protocol::LearningRun;
use recollect_server::{
    autonomous, config::Config, db, graph::analytics::shadow, knowledge_mapping::production,
    model_gateway as gateway, semantic, worker,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::atomic::Ordering,
};
use uuid::Uuid;
#[derive(Clone)]
pub struct Observed {
    pub source: Source,
    pub imported: Option<Imported>,
    pub extraction: Value,
    pub candidate: Option<Uuid>,
    pub provenance: Value,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub key: String,
    pub cohort: String,
    pub family: String,
    pub document_id: String,
    pub operation: Uuid,
    pub source: Imported,
    pub run: Option<LearningRun>,
    pub body: Value,
    pub body_sha256: String,
    pub policy0: Uuid,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub key: String,
    pub brain: Uuid,
    pub actor: Uuid,
    pub batch: Uuid,
    pub job: Uuid,
    pub policy: Uuid,
    pub entries: Vec<Uuid>,
    pub body: Value,
    pub body_sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortPlan {
    pub cohort: Cohort,
    pub imports: BTreeMap<String, Option<Imported>>,
    pub text: Vec<Text>,
    pub batches: Vec<Batch>,
    pub policies: Value,
    pub chunks: Value,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlan {
    pub case_id: String,
    pub brain: Uuid,
    pub run: LearningRun,
    pub device: Uuid,
    pub offered: Vec<Uuid>,
    pub prior_claim: Uuid,
    pub prior_revision: Uuid,
    pub canonical_id_map: Value,
    pub prior_workflow_receipt: Value,
    pub body: Value,
    pub body_sha256: String,
    pub template: Value,
    pub controlled_payload: Value,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub cohorts: Vec<CohortPlan>,
    pub controls: Vec<ControlPlan>,
}
pub fn config(base: &Config, address: std::net::SocketAddr) -> Config {
    let mut c = base.clone();
    c.models.endpoint = format!("http://{address}/v1");
    c.models.openrouter_endpoint = format!("http://{address}/v1");
    c.models.key = Some("revision4-loopback-transport-only".into());
    c.models.openrouter_key = None;
    c
}
pub fn configured(c: &Config) -> Result<()> {
    ensure!(
        c.models.text_model.as_deref() == Some("gpt-5.6-luna")
            && c.models.embedding_model == "text-embedding-3-large"
            && c.models.embedding_dimensions == 3072,
        "Frozen installed models/dimensions drift"
    );
    Ok(())
}
pub fn inv(t: &Text) -> gateway::Invocation {
    if let Some(r) = &t.run {
        autonomous::invocation(r)
    } else {
        gateway::extraction(t.operation, t.source.version)
    }
}
pub fn cx(t: &Text, actor: Uuid) -> gateway::Context {
    gateway::Context {
        brain: t.source.brain,
        actor,
        device: Some(t.source.device),
    }
}
async fn cancel_job(o: &Owned, brain: Uuid, job: Uuid) -> Result<()> {
    let auth = o.auth().await?;
    api(recollect_server::jobs::cancel(
        axum::extract::State(o.state.clone()),
        auth,
        axum::extract::Path((brain, job)),
    )
    .await)?;
    Ok(())
}
async fn change(o: &Owned, brain: Uuid, embedding: bool) -> Result<(Uuid, Uuid)> {
    let path = format!("/api/brains/{brain}/models/policy");
    let (before, _) = http(&o.state, Some(&o.login), "GET", &path, Value::Null).await?;
    let mut p = before["current"]["policy"].clone();
    ensure!(
        p["automatic_learning"] == false && p["autonomous_memory"] == false,
        "Unexpected automatic work"
    );
    p["automatic_embedding"] = json!(embedding);
    let (after, _) = http(
        &o.state,
        Some(&o.login),
        "PUT",
        &path,
        json!({"base_change":before["current"]["change_id"],"policy":p}),
    )
    .await?;
    Ok((
        id(&before["current"]["change_id"])?,
        id(&after["change_id"])?,
    ))
}
fn id(v: &Value) -> Result<Uuid> {
    v.as_str()
        .context("Canonical ID")?
        .parse()
        .context("Canonical UUID")
}
fn embedding_inv(b: &Batch) -> gateway::Invocation {
    gateway::Invocation {
        operation: b.batch,
        purpose: "embedding".into(),
        inputs: b
            .entries
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
    }
}
pub async fn prepare_cohort(
    o: &Owned,
    c: Cohort,
    out: &Path,
    controlled: bool,
) -> Result<CohortPlan> {
    let mut scopes = Scopes::new(&o.state, &o.login).await?;
    let mut imports = BTreeMap::new();
    for s in &c.sources {
        let imported = if s.text.is_empty() {
            None
        } else {
            Some(
                Box::pin(scopes.import(&o.state, &o.login, &o.admin, &s.scope, &s.title, &s.text))
                    .await?,
            )
        };
        imports.insert(s.document_id.clone(), imported);
    }
    let mut text = vec![];
    for s in &c.sources {
        let Some(i) = &imports[&s.document_id] else {
            continue;
        };
        for family in inputs::FAMILIES {
            let run = if family == "source-reconciliation-4" {
                let mut r = Box::pin(scopes.learning(&o.state, i)).await?;
                let mut tx = api(db::preparation_tx(&o.state.pool, o.actor, Some(i.device)).await)?;
                let offered = api(autonomous::targets(&o.state, &mut tx, &r, 16384).await)?;
                ensure!(offered.is_empty(), "Fresh source selector must be empty");
                tx.commit().await?;
                r.reconciliation_inputs = offered;
                cancel_job(o, i.brain, r.job_id).await?;
                Some(r)
            } else {
                None
            };
            let operation = run.as_ref().map_or_else(Uuid::new_v4, |r| r.id);
            let mut t = Text {
                key: format!("heldout/{}/{}/{}", c.name, family, s.document_id),
                cohort: c.name.clone(),
                family: family.into(),
                document_id: s.document_id.clone(),
                operation,
                source: i.clone(),
                run,
                body: Value::Null,
                body_sha256: String::new(),
                policy0: i.policy,
            };
            let p =
                api(
                    gateway::preview_for_policy(&o.state, cx(&t, o.actor), &inv(&t), i.policy)
                        .await,
                )?;
            let bytes = serde_json::to_vec(&p.body)?;
            ensure!(bytes.len() <= 65536, "Complete P0 text body bound");
            t.body_sha256 = io::sha(&bytes)?;
            t.body = p.body;
            super::transport::freeze(
                &out.join("bodies").join(format!("{operation}.json")),
                &bytes,
                false,
            )?;
            text.push(t);
        }
    }
    if controlled {
        preview_probes(o, &text, out).await?;
    }
    let brains: BTreeSet<_> = imports.values().flatten().map(|s| s.brain).collect();
    let mut policies = vec![];
    let mut batches = vec![];
    for brain in brains {
        let (p0, p1) = change(o, brain, true).await?;
        if controlled {
            let (profile, _) = http(
                &o.state,
                Some(&o.login),
                "POST",
                &format!("/api/brains/{brain}/semantic/reindex"),
                json!({"base_profile":null}),
            )
            .await?;
            http(
                &o.state,
                Some(&o.login),
                "POST",
                &format!("/api/brains/{brain}/semantic/reindex"),
                json!({"base_profile":profile["id"]}),
            )
            .await?;
        }

        let expected: i64 =
            sqlx::query_scalar("SELECT count(*) FROM source_chunks WHERE brain_id=$1")
                .bind(brain)
                .fetch_one(&o.admin)
                .await?;
        ensure!(
            expected > 0 && expected <= 1000,
            "Native source chunk coverage bound"
        );
        let mut complete = false;
        let mut discovery_passes = 0;
        for _ in 0..1001 {
            discovery_passes += 1;
            api(semantic::maintain_brain(&o.state, brain, o.actor).await)?;
            let(found,pending):(i64,i64)=sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE state='pending') FROM semantic_entries WHERE brain_id=$1 AND kind='source_chunk'").bind(brain).fetch_one(&o.admin).await?;
            ensure!(found <= expected, "Foreign semantic entries");
            if found == expected && pending == 0 {
                complete = true;
                break;
            }
        }
        ensure!(complete, "Complete discovery cannot be reduced to prefix");
        let incomplete:i64=sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE brain_id=$1 AND (kind<>'source_chunk' OR state<>'queued' OR truncated)").bind(brain).fetch_one(&o.admin).await?;
        ensure!(
            incomplete == 0,
            "Every complete canonical chunk must be queued once"
        );
        let rows:Vec<(Uuid,Uuid,Uuid,Uuid)>=sqlx::query_as("SELECT id,job_id,actor_id,policy_id FROM semantic_batches WHERE brain_id=$1 AND retry_of IS NULL AND automatic_attempt=0 AND state='queued' ORDER BY created_at,id").bind(brain).fetch_all(&o.admin).await?;
        let mut total = 0;
        let mut unique = BTreeSet::new();
        for (batch, job, actor, policy) in rows {
            ensure!(
                actor == o.actor && policy == p1,
                "Batch actor/policy differs"
            );
            let entries:Vec<Uuid>=sqlx::query_scalar("SELECT entry_id FROM semantic_batch_inputs WHERE brain_id=$1 AND batch_id=$2 ORDER BY ordinal").bind(brain).bind(batch).fetch_all(&o.admin).await?;
            ensure!(
                !entries.is_empty()
                    && entries.len() <= 20
                    && entries.iter().all(|id| unique.insert(*id)),
                "Batch partition overlap"
            );
            total += entries.len();
            let mut b = Batch {
                key: format!("embedding/{}/{batch}", c.name),
                brain,
                actor,
                batch,
                job,
                policy,
                entries,
                body: Value::Null,
                body_sha256: String::new(),
            };
            let preview = api(gateway::preview_for_policy(
                &o.state,
                gateway::Context {
                    brain,
                    actor,
                    device: None,
                },
                &embedding_inv(&b),
                p1,
            )
            .await)?;
            let bytes = serde_json::to_vec(&preview.body)?;
            b.body_sha256 = io::sha(&bytes)?;
            b.body = preview.body;
            super::transport::freeze(
                &out.join("bodies").join(format!("{batch}.json")),
                &bytes,
                true,
            )?;
            batches.push(b);
        }
        ensure!(
            total == expected as usize,
            "Full batch/chunk coverage required"
        );
        policies
            .push(json!({"brain_id":brain,"p0":p0,"p1":p1,"p2":null,"complete_chunks":expected,"discovery_passes":discovery_passes}));
    }
    let chunks:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('brain_id',c.brain_id,'source_version_id',c.version_id,'chunk_id',c.id,'ordinal',c.ordinal,'byte_start',c.byte_start,'byte_end',c.byte_end,'complete_chunk_sha256',encode(sha256(convert_to(c.content,'UTF8')),'hex'),'entry_id',e.id,'profile_id',e.profile_id,'batch_id',e.batch_id,'truncated',e.truncated) FROM source_chunks c JOIN semantic_entries e ON e.brain_id=c.brain_id AND e.input_id=c.id AND e.kind='source_chunk' ORDER BY c.brain_id,c.version_id,c.ordinal").fetch_all(&o.admin).await?;
    let p = CohortPlan {
        cohort: c,
        imports,
        text,
        batches,
        policies: json!(policies),
        chunks: json!(chunks),
    };
    io::json(&out.join("cohort-plan.json"), &serde_json::to_value(&p)?)?;
    Ok(p)
}
pub async fn prepare_control(
    o: &Owned,
    case: &Value,
    out: &Path,
    controlled: bool,
) -> Result<ControlPlan> {
    let p = Box::pin(controls::setup_case(&o.state, &o.admin, case)).await?;
    let inv = autonomous::invocation(&p.run);
    ensure!(
        serde_json::to_vec(&p.preview.body)?.len() <= 65536,
        "Control complete body bound"
    );
    let current = api(gateway::preview_for_policy(
        &o.state,
        gateway::Context {
            brain: p.brain_id,
            actor: p.actor_id,
            device: Some(p.device_id),
        },
        &inv,
        p.policy_id,
    )
    .await)?;
    ensure!(
        current.body == p.preview.body,
        "Control canonical builder differs"
    );
    let template = wire::template(&support::invocation(&p.run, Uuid::nil()))?;
    let payload = if controlled {
        let mut claims = case["gold"]["claims_required"]
            .as_array()
            .context("Public controlled claims")?
            .clone();
        for claim in &mut claims {
            claim["rationale"] =
                json!("Explicitly established by the exact controlled cited source lines.");
            if !claim["replaces_revision"].is_null() {
                claim["replaces_revision"] = json!(p.prior_revision_id);
            }
        }
        let retirements=case["gold"]["retirements_required"].as_array().context("Public controlled retirements")?.iter().map(|r|json!({"revision_id":p.prior_revision_id,"reason":r["semantic_reason"],"line_from":r["line_from"],"line_to":r["line_to"]})).collect::<Vec<_>>();
        json!({"claims":claims,"retirements":retirements,"mapping":null})
    } else {
        Value::Null
    };
    let plan = ControlPlan {
        case_id: p.case_id,
        brain: p.brain_id,
        run: p.run,
        device: p.device_id,
        offered: p.offered,
        prior_claim: p.prior_claim_id,
        prior_revision: p.prior_revision_id,
        canonical_id_map: p.canonical_id_map,
        prior_workflow_receipt: p.prior_workflow_receipt,
        body_sha256: io::value_hash(&current.body)?,
        body: current.body,
        template,
        controlled_payload: payload,
    };
    super::transport::freeze(
        &out.join("bodies").join(format!("{}.json", plan.run.id)),
        &serde_json::to_vec(&plan.body)?,
        false,
    )?;
    io::json(
        &out.join("control-plan.json"),
        &serde_json::to_value(&plan)?,
    )?;
    Ok(plan)
}

pub async fn execute_cohort(
    o: &Owned,
    p: &CohortPlan,
    w: &Wire,
    out: &Path,
) -> Result<Vec<Vec<Observed>>> {
    let mut pending: BTreeMap<_, _> = p.batches.iter().map(|b| (b.job, b)).collect();
    let mut lease_proved = false;
    while !pending.is_empty() && !w.stopped().await {
        let actual: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM jobs WHERE lane='model' AND state IN('queued','running') ORDER BY id",
        )
        .fetch_all(&o.admin)
        .await?;
        let expected: Vec<_> = pending.keys().copied().collect();
        ensure!(actual == expected, "Unexpected queued model job");
        let job = worker::claim(&o.state.pool, "model")
            .await?
            .context("Exact semantic job")?;
        let b = pending
            .remove(&job.id)
            .context("Unscheduled semantic claim")?;
        ensure!(
            job.brain_id == b.brain
                && job.target_id == b.batch
                && job.actor_id == b.actor
                && job.device_id.is_none()
                && job.kind == "semantic.generate",
            "Semantic lease/owner drift"
        );
        let fresh = api(gateway::preview_for_policy(
            &o.state,
            gateway::Context {
                brain: b.brain,
                actor: b.actor,
                device: None,
            },
            &embedding_inv(b),
            b.policy,
        )
        .await)?;
        ensure!(
            fresh.body == b.body && io::value_hash(&fresh.body)? == b.body_sha256,
            "Frozen batch drift"
        );
        if w.simulated() && !lease_proved {
            lease_guard_probe(o, &job, b, out).await?;
            lease_proved = true;
        }
        w.register(Active {
            key: b.key.clone(),
            operation: b.batch,
            body: b.body.clone(),
            arm: Arm::Embedding,
            payload: Value::Null,
            control: None,
            lease: Some(wire::Lease {
                admin: o.admin.clone(),
                job: job.clone(),
            }),
        })
        .await?;
        let result = Box::pin(semantic::execute_previewed(&o.state, &job, &fresh)).await;
        w.clear().await;
        if result.is_err() {
            if let Some(l) = w.ledger.lock().await.as_mut() {
                l.stop()?;
            }
            let state: String = sqlx::query_scalar("SELECT state FROM jobs WHERE id=$1")
                .bind(job.id)
                .fetch_one(&o.admin)
                .await?;
            if state == "running" {
                worker::fail(&o.state.pool, &job, worker::Failure::Unsupported).await?;
            }
            cancel_job(o, b.brain, b.job).await?;
        }
    }
    for b in pending.values() {
        cancel_job(o, b.brain, b.job).await?;
    }
    let mut policies = vec![];
    let mut heads = BTreeMap::new();
    for row in p.policies.as_array().context("Policy manifest")? {
        let brain = id(&row["brain_id"])?;
        let (p1, p2) = change(o, brain, false).await?;
        ensure!(json!(p1) == row["p1"], "P1 policy drift");
        heads.insert(brain, p2);
        let mut v = row.clone();
        v["p2"] = json!(p2);
        policies.push(v);
    }
    io::json(
        &out.join("policy-transition.json"),
        &json!({"policies":policies,"permitted_change":"automatic_embedding_only","all_batches_frozen_before_first_attempt":true,"rediscovery_after_execution":false}),
    )?;
    let mut families = vec![];
    for family in inputs::FAMILIES {
        let mut all = vec![];
        for source in &p.cohort.sources {
            let imported = p.imports[&source.document_id].clone();
            let mut observed = Observed {
                source: source.clone(),
                imported: imported.clone(),
                extraction: json!({"state":"not_attempted","failure":null,"candidates":null}),
                candidate: None,
                provenance: json!({"document_id":source.document_id,"text_sha256":source.text_sha256,"byte_length":source.text.len()}),
            };
            if let Some(i) = imported {
                let t = p
                    .text
                    .iter()
                    .find(|t| t.family == family && t.document_id == source.document_id)
                    .context("Frozen text schedule missing")?;
                let fresh = api(gateway::preview_for_policy(
                    &o.state,
                    cx(t, o.actor),
                    &inv(t),
                    heads[&i.brain],
                )
                .await)?;
                ensure!(
                    fresh.body == t.body && io::value_hash(&fresh.body)? == t.body_sha256,
                    "P0/P2 byte equality required before text transmission"
                );
                observed.provenance = json!({"document_id":source.document_id,"brain_id":i.brain,"source_id":i.source,"source_version_id":i.version,"import_operation_id":i.operation,"device_id":i.device,"import_policy_id":i.policy,"p0_policy_id":t.policy0,"p2_policy_id":fresh.policy_id,"canonical_selection":i.selection,"text_sha256":source.text_sha256,"byte_length":source.text.len(),"complete_body_sha256":t.body_sha256,"request":null});
                if !w.stopped().await {
                    if let Some(run) = &t.run {
                        let mut tx =
                            api(db::preparation_tx(&o.state.pool, o.actor, Some(i.device)).await)?;
                        let offered =
                            api(autonomous::targets(&o.state, &mut tx, run, 16384).await)?;
                        ensure!(
                            offered.is_empty(),
                            "Fresh reconciliation target drift before forwarding"
                        );
                        tx.commit().await?;
                    }
                    let payload = toy_payload(source, family)?;
                    w.register(Active {
                        key: t.key.clone(),
                        operation: t.operation,
                        body: fresh.body.clone(),
                        arm: Arm::Heldout,
                        payload,
                        control: None,
                        lease: None,
                    })
                    .await?;
                    let result = gateway::invoke_previewed_for_policy(
                        &o.state,
                        cx(t, o.actor),
                        inv(t),
                        &fresh,
                    )
                    .await;
                    w.clear().await;
                    match result {
                        Ok(response) => {
                            let request:Value=sqlx::query_scalar("SELECT jsonb_build_object('request_id',id,'operation_id',operation_id,'policy_id',policy_id,'purpose',purpose,'state',state,'provider',provider,'requested_model',model,'returned_model',returned_model,'prompt_label',prompt_label,'schema_label',schema_label,'admitted_mapping_digest',mapping_output_sha256) FROM model_requests WHERE brain_id=$1 AND id=$2").bind(i.brain).bind(response.request.id).fetch_one(&o.admin).await?;
                            observed.provenance["request"] = request;
                            if let Some(gateway::Output::Json(value)) = response.output {
                                if value.as_object().is_some_and(|o| o.contains_key("mapping"))
                                    && value["mapping"].is_null()
                                {
                                    observed.extraction =
                                        json!({"state":"absent","failure":null,"candidates":null});
                                } else {
                                    match production::adapt_for_request(
                                        i.version,
                                        &source.text,
                                        &value["mapping"],
                                        &response.request.prompt_label,
                                        &response.request.schema_label,
                                    ) {
                                        Ok(dto) => {
                                            let auth = o.auth().await?;
                                            match shadow::admit(
                                                &o.state,
                                                &auth,
                                                cx(t, o.actor),
                                                response.request.id,
                                                i.version,
                                                &value["mapping"],
                                            )
                                            .await
                                            {
                                                Ok(candidate) => {
                                                    if w.simulated() {
                                                        admission_probes(
                                                            o,
                                                            &auth,
                                                            t,
                                                            response.request.id,
                                                            &value["mapping"],
                                                            &source.text,
                                                        )
                                                        .await?;
                                                    }
                                                    observed.candidate = Some(candidate);
                                                    observed.extraction = json!({"state":"validated","failure":null,"candidates":dto});
                                                }
                                                Err(_) => {
                                                    observed.extraction = json!({"state":"failed","failure":"UpstreamFailure","candidates":null});
                                                    if let Some(l) = w.ledger.lock().await.as_mut()
                                                    {
                                                        l.stop()?;
                                                    }
                                                }
                                            }
                                        }
                                        Err(f) => {
                                            observed.extraction = json!({"state":"rejected","failure":format!("{f:?}"),"candidates":null})
                                        }
                                    }
                                }
                            } else {
                                observed.extraction = json!({"state":"failed","failure":"UpstreamFailure","candidates":null});
                                if let Some(l) = w.ledger.lock().await.as_mut() {
                                    l.stop()?;
                                }
                            }
                        }
                        Err(error) => {
                            let request:Option<Value>=sqlx::query_scalar("SELECT jsonb_build_object('request_id',id,'operation_id',operation_id,'policy_id',policy_id,'purpose',purpose,'state',state,'provider',provider,'requested_model',model,'returned_model',returned_model,'prompt_label',prompt_label,'schema_label',schema_label,'admitted_mapping_digest',mapping_output_sha256,'error_code',error_code) FROM model_requests WHERE brain_id=$1 AND operation_id=$2").bind(i.brain).bind(t.operation).fetch_optional(&o.admin).await?;
                            observed.provenance["request"] = request.unwrap_or(Value::Null);
                            let quality =
                                error.1 == "provider_shape" && w.intact(t.operation).await;
                            observed.extraction = json!({"state":"failed","failure":"UpstreamFailure","candidates":null});
                            if !quality {
                                if let Some(l) = w.ledger.lock().await.as_mut() {
                                    l.stop()?;
                                }
                            }
                        }
                    }
                }
            } else {
                observed.provenance["reason"] = json!("exact_empty_import_ineligible");
            }
            all.push(observed);
        }
        families.push(all);
    }
    Ok(families)
}
pub fn toy_payload(s: &Source, family: &str) -> Result<Value> {
    if s.document_id.ends_with("_missing") {
        return Ok(json!({"__controlled_raw_text":"{"}));
    }
    let text = &s.text;
    let label = if text.contains("Orion") {
        "Orion"
    } else {
        "Vela"
    };
    let span = json!({"quote":label,"line_from":1,"line_to":1,"occurrence":1});
    let mut mapping = if text.contains("ABSTAIN") {
        Value::Null
    } else {
        json!({"language":"en","mentions":[{"key":"service","kind":"service","confidence":0.9,"span":span}],"aliases":[],"relations":[]})
    };
    if s.document_id.ends_with("_rejected") {
        mapping["mentions"][0]["span"]["quote"] = json!("unoffered occurrence");
    }
    let mut v = json!({"claims":[],"mapping":mapping});
    if family == "source-reconciliation-4" {
        v["retirements"] = json!([]);
    }
    Ok(v)
}
pub async fn execute_control(o: &Owned, p: &ControlPlan, w: &Wire, out: &Path) -> Result<Value> {
    let before = snapshot(o, p).await?;
    if w.stopped().await {
        let v = json!({"id":p.case_id,"state":"not_attempted","before":before,"after":before});
        io::json(&out.join("canonical-outcome.json"), &v)?;
        return Ok(v);
    }
    let queued: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM jobs WHERE lane='model' AND state IN('queued','running') ORDER BY id",
    )
    .fetch_all(&o.admin)
    .await?;
    ensure!(
        queued == vec![p.run.job_id],
        "Only exact control job may be claimed"
    );
    let cx = gateway::Context {
        brain: p.brain,
        actor: p.run.actor_id,
        device: Some(p.device),
    };
    let fresh = api(gateway::preview_for_policy(
        &o.state,
        cx,
        &autonomous::invocation(&p.run),
        p.run.policy_id,
    )
    .await)?;
    ensure!(
        fresh.body == p.body && io::value_hash(&fresh.body)? == p.body_sha256,
        "Control upfront bytes drift"
    );
    let job = worker::claim(&o.state.pool, "model")
        .await?
        .context("Control job")?;
    ensure!(
        job.id == p.run.job_id
            && job.brain_id == p.brain
            && job.target_id == p.run.id
            && job.actor_id == p.run.actor_id
            && job.device_id == p.run.device_id,
        "Control lease/target drift"
    );
    let c = wire::Control {
        state: o.state.clone(),
        admin: o.admin.clone(),
        brain: p.brain,
        run: p.run.id,
        source: p.run.source_version_id,
        actor: p.run.actor_id,
        device: p.run.device_id,
        policy: p.run.policy_id,
        offered: p.offered.clone(),
        template: p.template.clone(),
    };
    w.register(Active {
        key: format!("control/{}", p.case_id),
        operation: p.run.id,
        body: fresh.body,
        arm: Arm::Control,
        payload: p.controlled_payload.clone(),
        control: Some(c),
        lease: Some(wire::Lease {
            admin: o.admin.clone(),
            job: job.clone(),
        }),
    })
    .await?;
    // The actual worker reselects targets, creates the normalized stage, and invokes
    // conditional support through the guard before native canonical publication.
    let result = Box::pin(worker::execute(&o.state, &job)).await;
    w.clear().await;
    let after = snapshot(o, p).await?;
    let acceptable = after["run"]["state"] == "succeeded";
    if !acceptable || result.is_err() {
        if let Some(l) = w.ledger.lock().await.as_mut() {
            l.stop()?;
        }
        let s: String = sqlx::query_scalar("SELECT state FROM jobs WHERE id=$1")
            .bind(job.id)
            .fetch_one(&o.admin)
            .await?;
        if matches!(s.as_str(), "running" | "queued") {
            cancel_job(o, p.brain, job.id).await?;
        }
    }
    let v = json!({"id":p.case_id,"state":if acceptable{"native_terminal"}else{"failed_retained"},"canonical_id_map":p.canonical_id_map,"prior_workflow_receipt":p.prior_workflow_receipt,"offered_revision_ids":p.offered,"before":before,"after":after,"canonical_identity_and_publication":"ACTUAL_OBSERVATIONS_ONLY","producer_quality":"NOT_MEASURED"});
    io::json(&out.join("canonical-outcome.json"), &v)?;
    Ok(v)
}
async fn snapshot(o: &Owned, p: &ControlPlan) -> Result<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('run',(SELECT to_jsonb(r) FROM learning_runs r WHERE r.brain_id=$1 AND r.id=$2),'claims',coalesce((SELECT jsonb_agg(jsonb_build_object('claim_id',c.id,'current_revision',c.current_revision,'revision',r.revision) ORDER BY c.id) FROM claims c JOIN claim_revisions r ON r.id=c.current_revision AND r.brain_id=c.brain_id WHERE c.brain_id=$1),'[]'::jsonb),'stage',(SELECT to_jsonb(s) FROM learning_support_stages s WHERE s.brain_id=$1 AND s.run_id=$2),'assessments',coalesce((SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM memory_support_assessments a WHERE brain_id=$1),'[]'::jsonb),'revisions',coalesce((SELECT jsonb_agg(revision ORDER BY recorded_at,id) FROM claim_revisions WHERE brain_id=$1),'[]'::jsonb),'requests',coalesce((SELECT jsonb_agg(jsonb_build_object('request_id',id,'operation_id',operation_id,'state',state,'prompt_label',prompt_label,'schema_label',schema_label,'provider',provider,'requested_model',model,'returned_model',returned_model,'input_tokens',input_tokens,'output_tokens',output_tokens) ORDER BY created_at,id) FROM model_requests WHERE brain_id=$1),'[]'::jsonb))").bind(p.brain).bind(p.run.id).fetch_one(&o.admin).await?)
}
pub async fn vectors(o: &Owned, s: &Imported) -> Result<Value> {
    let rows:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('entry_id',e.id,'brain_id',e.brain_id,'profile_id',e.profile_id,'source_version_id',c.version_id,'chunk_id',c.id,'ordinal',c.ordinal,'byte_start',c.byte_start,'byte_end',c.byte_end,'complete_chunk_sha256',encode(sha256(convert_to(c.content,'UTF8')),'hex'),'state',e.state,'truncated',e.truncated,'request_id',e.request_id,'batch_id',e.batch_id,'embedding_available',e.embedding IS NOT NULL,'embedding_sha256',CASE WHEN e.embedding IS NULL THEN NULL ELSE encode(sha256(convert_to(e.embedding::text,'UTF8')),'hex') END) FROM source_chunks c LEFT JOIN semantic_entries e ON e.brain_id=c.brain_id AND e.input_id=c.id AND e.source_version_id=c.version_id AND e.kind='source_chunk' WHERE c.brain_id=$1 AND c.version_id=$2 ORDER BY c.ordinal").bind(s.brain).bind(s.version).fetch_all(&o.admin).await?;
    Ok(json!({"entries":rows,"vector_values_or_centroids_exported":false}))
}
pub async fn export_family(
    o: &Owned,
    p: &CohortPlan,
    all: &[Observed],
    family: &str,
    out: &Path,
    bindings: &Value,
    controlled: bool,
) -> Result<Value> {
    let recipe = p
        .cohort
        .families
        .iter()
        .find(|f| f.producer == family)
        .context("Family recipe")?;
    let auth = o.auth().await?;
    export::isolate(o, out).await?;
    let mut modes = vec![];
    let mut provenance = vec![];
    let mut actual_results = vec![];
    for mode in export::MODES {
        let (scalar, proof, results) =
            Box::pin(export::mode(o, &auth, all, mode, "2026.08.1")).await?;
        modes.push(scalar);
        provenance.push(proof);
        actual_results.extend(results);
    }
    let sources:Vec<_>=all.iter().map(|s|json!({"document_id":s.source.document_id,"text_sha256":s.source.text_sha256,"partition":s.source.partition,"source_version_id":s.imported.as_ref().map(|i|i.version),"extraction":s.extraction})).collect();
    let outcomes = json!({"schema":"recollect-offline-organization-outcomes-1","corpus_bindings":p.cohort.corpus_bindings,"recipe_sha256":recipe.recipe_sha256,"sources":sources,"modes":modes});
    io::write(&out.join("recipe.json"), &recipe.recipe_bytes)?;
    ensure!(
        io::hash_file(&out.join("recipe.json"))? == recipe.recipe_sha256,
        "Exact raw scalar recipe bytes differ"
    ); // Preserve exact original recipe bytes via hash-bound copy by orchestrator.
    io::json(&out.join("outcomes.json"), &outcomes)?;
    let mut source_proof = vec![];
    for s in all {
        let mut v = s.provenance.clone();
        v["extraction_state"] = s.extraction["state"].clone();
        v["extraction_failure"] = s.extraction["failure"].clone();
        if let Some(i) = &s.imported {
            v["vectors"] = vectors(o, i).await?;
        }
        source_proof.push(v);
    }
    io::json(
        &out.join("native-provenance.json"),
        &json!({"schema":"recollect-native-organization-export-1","input_sha256":{"frozen_campaign":bindings,"corpus":p.cohort.corpus_bindings,"source_feed":recipe.source_feed_sha256,"preparation":recipe.preparation_sha256,"recipe":recipe.recipe_sha256},"output_sha256":{"outcomes":io::hash_file(&out.join("outcomes.json"))?},"run":{"mode":if controlled{"controlled_toy_conformance"}else{"revision4_bounded_campaign"},"database_name":o.database,"ownership_nonce_sha256":o.nonce_hash,"controlled_transport":controlled,"normal_worker_started":false},"sources":source_proof,"modes":provenance,"not_measured":{"canonical_identity":"NOT_MEASURED","accepted_alias_dispositions":"NOT_MEASURED","support_publication":"NOT_MEASURED","label_usefulness":"NOT_MEASURED","producer_qualification":"NOT_MEASURED","recipe_qualification":"NOT_MEASURED"}}),
    )?;
    if controlled {
        let result = actual_results
            .iter()
            .find(|r| r.vector_profile_id.is_some())
            .context("Actual retained profile proof")?;
        let profile = result.vector_profile_id.unwrap();
        let spare:Uuid=sqlx::query_scalar("SELECT id FROM semantic_profiles WHERE brain_id=$1 AND id<>$2 ORDER BY created_at,id LIMIT 1").bind(result.brain_id).bind(profile).fetch_one(&o.admin).await?;
        sqlx::query("UPDATE semantic_heads SET profile_id=$3 WHERE brain_id=$1 AND profile_id=$2")
            .bind(result.brain_id)
            .bind(profile)
            .bind(spare)
            .execute(&o.admin)
            .await?;
        let rejected = shadow::result(&o.state, &auth, result.brain_id, result.report_id)
            .await
            .is_err();
        sqlx::query("UPDATE semantic_heads SET profile_id=$3 WHERE brain_id=$1 AND profile_id=$2")
            .bind(result.brain_id)
            .bind(spare)
            .bind(profile)
            .execute(&o.admin)
            .await?;
        ensure!(
            rejected,
            "Fresh native result accepted wrong active profile"
        );
        io::json(
            &out.join("profile-drift-proof.json"),
            &json!({"actual_report_id":result.report_id,"wrong_profile_rejected":true,"actual_prior_profile_restored":true}),
        )?;
        let (failed, proof, _) = Box::pin(export::mode(
            o,
            &auth,
            all,
            shadow::Mode::NativeHybrid,
            "2099.00.0",
        ))
        .await?;
        io::json(
            &out.join("engine-version-failure.json"),
            &json!({"scalar":failed,"provenance":proof,"expected_deliberate_failure":true,"empty_projection_success":"engine_not_called"}),
        )?;
        let reports = proof["reports"]
            .as_array()
            .context("Wrong-engine reports")?;
        ensure!(
            reports.iter().any(|r| r["state"] == "failed"
                && r["report"]["error_code"] == "organization_engine_version_changed"
                && r["native_attempts"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty()))
                && reports.iter().all(|r| (r["state"] == "failed"
                    && r["report"]["error_code"] == "organization_engine_version_changed"
                    && r["native_attempts"]
                        .as_array()
                        .is_some_and(|a| !a.is_empty()))
                    || (r["state"] == "freshly_qualified"
                        && r["report"]["returned_gds_version"].is_null()
                        && r["result"]["gds_version"].is_null()
                        && r["native_attempts"].as_array().is_some_and(Vec::is_empty))),
            "Wrong engine did not fail actual native work or was attributed successful native output"
        );
    }
    Ok(
        json!({"sources":all.len(),"mode_source_rows":all.len()*5,"outcomes_sha256":io::hash_file(&out.join("outcomes.json"))?,"provenance_sha256":io::hash_file(&out.join("native-provenance.json"))?}),
    )
}

async fn preview_probes(o: &Owned, text: &[Text], out: &Path) -> Result<()> {
    let t = text.first().context("Toy text preview")?;
    let context = cx(t, o.actor);
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&o.admin)
        .await?;
    let wrong_device = gateway::preview_for_policy(
        &o.state,
        gateway::Context {
            device: Some(Uuid::new_v4()),
            ..context
        },
        &inv(t),
        t.policy0,
    )
    .await
    .is_err();
    let mut source = inv(t);
    source.operation = Uuid::new_v4();
    source.inputs[0].id = Uuid::new_v4();
    let wrong_source = gateway::preview_for_policy(&o.state, context, &source, t.policy0)
        .await
        .is_err();
    let mut target = inv(t);
    target.operation = Uuid::new_v4();
    target.inputs.push(gateway::InputRef {
        kind: "claim_revision".into(),
        id: Uuid::new_v4(),
    });
    let wrong_target = gateway::preview_for_policy(&o.state, context, &target, t.policy0)
        .await
        .is_err();
    let mut changed = t.body.clone();
    changed["model"] = json!("unapproved-model");
    let mut invocation = inv(t);
    invocation.operation = Uuid::new_v4();
    let wrong_body = gateway::invoke_previewed_for_policy(
        &o.state,
        context,
        invocation,
        &gateway::Preview {
            policy_id: t.policy0,
            body: changed,
        },
    )
    .await
    .is_err();
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&o.admin)
        .await?;
    ensure!(
        wrong_device && wrong_source && wrong_target && wrong_body && before == after,
        "Native preview/admission drift guard failed"
    );
    io::json(
        &out.join("native-preview-drift-proof.json"),
        &json!({"wrong_device_rejected":wrong_device,"wrong_source_rejected":wrong_source,"wrong_target_rejected":wrong_target,"wrong_body_rejected":wrong_body,"model_requests_created":after-before,"external_calls":0}),
    )?;
    Ok(())
}
async fn admission_probes(
    o: &Owned,
    auth: &recollect_server::auth::Auth,
    t: &Text,
    request: Uuid,
    mapping: &Value,
    text: &str,
) -> Result<()> {
    let wrong_tuple =
        production::adapt_for_request(t.source.version, text, mapping, "source-facts-3", &t.family)
            .is_err();
    let mut wrong = mapping.clone();
    wrong["mentions"][0]["span"]["quote"] = json!("unoffered physical occurrence");
    let wrong_span =
        production::adapt_for_request(t.source.version, text, &wrong, &t.family, &t.family)
            .is_err();
    let mut changed = mapping.clone();
    changed["mentions"][0]["confidence"] = json!(0.2345);
    let wrong_digest = shadow::admit(
        &o.state,
        auth,
        cx(t, o.actor),
        request,
        t.source.version,
        &changed,
    )
    .await
    .is_err();
    let wrong_device = shadow::admit(
        &o.state,
        auth,
        gateway::Context {
            device: Some(Uuid::new_v4()),
            ..cx(t, o.actor)
        },
        request,
        t.source.version,
        mapping,
    )
    .await
    .is_err();
    ensure!(
        wrong_tuple && wrong_span && wrong_digest && wrong_device,
        "Actual request admission drift accepted"
    );
    Ok(())
}
async fn lease_guard_probe(
    o: &Owned,
    job: &worker::ClaimedJob,
    b: &Batch,
    out: &Path,
) -> Result<()> {
    let dir = out.join("lease-guard-proof");
    io::new_directory(&dir)?;
    for name in ["raw-responses", "raw-response-bindings", "wire-outcomes"] {
        io::new_directory(&dir.join(name))?;
    }
    let probe = Wire::controlled(&dir);
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = l.local_addr()?;
    let router = probe.router();
    let server = tokio::spawn(async move {
        let _ = axum::serve(l, router).await;
    });
    let mut rejected = 0;
    for target in [false, true] {
        let mut bad = job.clone();
        if target {
            bad.target_id = Uuid::new_v4()
        } else {
            bad.lease_token = Uuid::new_v4()
        };
        probe
            .register(Active {
                key: format!("controlled_invalid_lease_{target}"),
                operation: b.batch,
                body: b.body.clone(),
                arm: Arm::Embedding,
                payload: Value::Null,
                control: None,
                lease: Some(wire::Lease {
                    admin: o.admin.clone(),
                    job: bad,
                }),
            })
            .await?;
        let response = o
            .state
            .http
            .post(format!("http://{address}/v1/embeddings"))
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&b.body)?)
            .send()
            .await?;
        ensure!(
            !response.status().is_success(),
            "Guard forwarded changed worker lease/target"
        );
        probe.clear().await;
        rejected += 1;
    }
    server.abort();
    ensure!(
        probe.controlled.load(Ordering::SeqCst) == 0 && probe.external.load(Ordering::SeqCst) == 0,
        "Lease probe forwarded"
    );
    io::json(
        &dir.join("receipt.json"),
        &json!({"actual_claimed_job_id":job.id,"wrong_lease_and_target_rejected":rejected,"forwarded_requests":0,"lease_tokens_persisted":false,"original_job_lease_preserved":true}),
    )?;
    Ok(())
}
