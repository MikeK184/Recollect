//! Reviewed no-proxy loopback guard. Public forwarding is constructed only after
//! the exact new approval and execution lock are admitted by the caller.
use super::{
    guard::{self, Arm, Ledger},
    io,
    owned::{Owned, api},
    support,
};
use anyhow::{Context, Result, ensure};
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use recollect_protocol::LearningRun;
use recollect_server::{AppState, model_gateway as gateway};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::Mutex;
use uuid::Uuid;
#[derive(Clone)]
pub struct Active {
    pub key: String,
    pub operation: Uuid,
    pub body: Value,
    pub arm: Arm,
    pub payload: Value,
    pub control: Option<Control>,
    pub lease: Option<Lease>,
}
#[derive(Clone)]
pub struct Lease {
    pub admin: sqlx::PgPool,
    pub job: recollect_server::worker::ClaimedJob,
}
#[derive(Clone)]
pub struct Control {
    pub state: AppState,
    pub admin: sqlx::PgPool,
    pub brain: Uuid,
    pub run: Uuid,
    pub source: Uuid,
    pub actor: Uuid,
    pub device: Option<Uuid>,
    pub policy: Uuid,
    pub offered: Vec<Uuid>,
    pub template: Value,
}
#[derive(Clone)]
pub struct Wire {
    pub active: Arc<Mutex<Option<Active>>>,
    pub ledger: Arc<Mutex<Option<Ledger>>>,
    pub external: Arc<AtomicUsize>,
    pub controlled: Arc<AtomicUsize>,
    pub outcomes: Arc<Mutex<std::collections::BTreeMap<Uuid, Value>>>,
    pub out: PathBuf,
    client: Option<reqwest::Client>,
    key: Option<String>,
}
impl Wire {
    pub fn controlled(out: &Path) -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
            ledger: Arc::new(Mutex::new(None)),
            external: Arc::new(AtomicUsize::new(0)),
            controlled: Arc::new(AtomicUsize::new(0)),
            outcomes: Arc::new(Mutex::new(std::collections::BTreeMap::new())),
            out: out.into(),
            client: None,
            key: None,
        }
    }
    pub fn approved(out: &Path, key: String, ledger: Ledger) -> Result<Self> {
        ensure!(
            !key.trim().is_empty(),
            "Configured public provider key required"
        );
        let mut w = Self::controlled(out);
        w.client = Some(
            reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .timeout(Duration::from_secs(45))
                .build()?,
        );
        w.key = Some(key);
        w.ledger = Arc::new(Mutex::new(Some(ledger)));
        Ok(w)
    }
    pub fn router(&self) -> Router {
        Router::new()
            .route("/v1/responses", post(handle))
            .route("/v1/embeddings", post(embedding))
            .layer(DefaultBodyLimit::disable())
            .with_state(self.clone())
    }
    pub async fn register(&self, a: Active) -> Result<()> {
        let mut v = self.active.lock().await;
        ensure!(v.is_none(), "One current operation only");
        *v = Some(a);
        Ok(())
    }
    pub async fn clear(&self) {
        *self.active.lock().await = None;
    }
    pub async fn intact(&self, operation: Uuid) -> bool {
        self.outcomes
            .lock()
            .await
            .get(&operation)
            .is_some_and(|v| v["acceptable_envelope_usage_model"] == true)
    }
    pub fn simulated(&self) -> bool {
        self.client.is_none()
    }
    pub async fn stopped(&self) -> bool {
        self.ledger
            .lock()
            .await
            .as_ref()
            .is_some_and(Ledger::stopped)
    }
}
async fn handle(State(w): State<Wire>, request: axum::extract::Request) -> Response {
    bounded(w, request, false).await
}
async fn embedding(State(w): State<Wire>, request: axum::extract::Request) -> Response {
    bounded(w, request, true).await
}
async fn bounded(w: Wire, request: axum::extract::Request, embedding: bool) -> Response {
    match axum::body::to_bytes(request.into_body(), 65537).await {
        Ok(bytes) => route(w, bytes, embedding).await,
        Err(_) => {
            if let Some(l) = w.ledger.lock().await.as_mut() {
                let _ = l.stop();
            }
            (StatusCode::PAYLOAD_TOO_LARGE,axum::Json(json!({"error":{"code":"revision4_body_limit","message":"Frozen request body bound exceeded"}}))).into_response()
        }
    }
}
async fn route(w: Wire, b: Bytes, embedding: bool) -> Response {
    match inner(&w, &b, embedding).await {
        Ok(v) => (StatusCode::OK, axum::Json(v)).into_response(),
        Err(error) => {
            let _ = io::json(
                &w.out.join(format!("guard-refusal-{}.json", Uuid::new_v4())),
                &json!({"diagnostic":error.to_string(),"body_sha256":io::sha(&b).ok(),"bytes":b.len(),"external_calls":w.external.load(Ordering::SeqCst)}),
            );
            if let Some(l) = w.ledger.lock().await.as_mut() {
                let _ = l.stop();
            }
            (StatusCode::BAD_REQUEST,axum::Json(json!({"error":{"code":"revision4_guard_refused","message":"Frozen campaign guard refused request"}}))).into_response()
        }
    }
}
async fn inner(w: &Wire, bytes: &[u8], embedding: bool) -> Result<Value> {
    ensure!(bytes.len() <= 65536, "Request bound");
    let body = io::strict(bytes)?;
    let mut locked = w.active.lock().await;
    let a = locked.as_mut().context("No active frozen operation")?;
    let is_support = body["text"]["format"]["name"] == "source_support";
    let (key, operation, arm, payload) = if is_support {
        let c = a.control.as_ref().context("Unscheduled support")?;
        ensure!(
            !embedding && a.body.is_null(),
            "Support must follow exactly one extraction"
        );
        let stage:Value=sqlx::query_scalar("SELECT jsonb_build_object('operation',s.assessment_operation_id,'payload',s.payload,'run',to_jsonb(r),'inputs',(SELECT jsonb_agg(jsonb_build_object('kind',i.kind,'id',i.input_id) ORDER BY i.kind,i.input_id) FROM model_request_inputs i JOIN model_requests q ON q.id=i.request_id AND q.brain_id=i.brain_id WHERE q.brain_id=s.brain_id AND q.operation_id=s.assessment_operation_id)) FROM learning_support_stages s JOIN learning_runs r ON r.id=s.run_id AND r.brain_id=s.brain_id WHERE s.brain_id=$1 AND s.run_id=$2 AND s.verifier_version='source-support-3' AND s.verdicts IS NULL AND s.privacy_state='active'").bind(c.brain).bind(c.run).fetch_one(&c.admin).await?;
        let run: LearningRun = serde_json::from_value(stage["run"].clone())?;
        ensure!(
            run.id == c.run
                && run.brain_id == c.brain
                && run.actor_id == c.actor
                && run.device_id == c.device
                && run.source_version_id == c.source
                && run.policy_id == c.policy
                && run.reconciliation_inputs == c.offered,
            "Canonical control dependency drift"
        );
        let op = stage["operation"]
            .as_str()
            .context("Support operation")?
            .parse()?;
        let inv = support::invocation(&run, op);
        let template = template(&inv)?;
        ensure!(template == c.template, "Support builder/template drift");
        let mut expected = serde_json::to_value(&inv.inputs)?
            .as_array()
            .context("Inputs")?
            .clone();
        expected.sort_by_key(|v| v["kind"].to_string() + &v["id"].to_string());
        ensure!(
            stage["inputs"] == json!(expected),
            "Canonical support linked inputs differ"
        );
        let p = api(gateway::preview_for_policy(
            &c.state,
            gateway::Context {
                brain: c.brain,
                actor: c.actor,
                device: c.device,
            },
            &inv,
            c.policy,
        )
        .await)?;
        ensure!(
            bytes == serde_json::to_vec(&p.body)?,
            "Actual worker support bytes differ from canonical preview"
        );
        let key = format!("{}/support", a.key);
        let mut path = w.out.join("actual-support");
        if !path.exists() {
            io::new_directory(&path)?;
        }
        path.push(format!("{op}.json"));
        super::transport::freeze(&path, bytes, false)?;
        io::json(
            &w.out
                .join("actual-support")
                .join(format!("{op}-binding.json")),
            &json!({"body_sha256":io::sha(bytes)?,"operation":op,"brain":c.brain,"run":c.run,"source":c.source,"offered":c.offered,"template_sha256":io::value_hash(&c.template)?}),
        )?;
        let count = stage["payload"]["claims"].as_array().map_or(0, Vec::len)
            + stage["payload"]["retirements"]
                .as_array()
                .map_or(0, Vec::len);
        ensure!((1..=20).contains(&count), "Empty/oversized support stage");
        let mut tx =
            api(recollect_server::db::preparation_tx(&c.state.pool, c.actor, c.device).await)?;
        let source = api(gateway::source(&c.state, &mut tx, c.brain, c.source, 16384).await)?;
        tx.commit().await?;
        let lines: Vec<_> = source.text.lines().collect();
        let mut actions = vec![];
        for claim in stage["payload"]["claims"]
            .as_array()
            .context("Staged claims")?
        {
            actions.push((
                claim["content"]["supports"][0]["line_from"]
                    .as_u64()
                    .context("Claim cited line")?,
                claim["content"]["supports"][0]["line_to"]
                    .as_u64()
                    .context("Claim cited line")?,
            ));
        }
        for retirement in stage["payload"]["retirements"]
            .as_array()
            .context("Staged retirements")?
        {
            actions.push((
                retirement["line_from"]
                    .as_u64()
                    .context("Retirement cited line")?,
                retirement["line_to"]
                    .as_u64()
                    .context("Retirement cited line")?,
            ));
        }
        let assessments=actions.iter().enumerate().map(|(index,(from,to))|{ensure!(*from>0&&*to>=*from&&*to<=lines.len()as u64,"Controlled support exact lines");Ok(json!({"index":index,"citation_support":true,"evidence_quote":lines[*from as usize-1..*to as usize].join("\n"),"disposition":"supported","reason":"Exact public controlled cited evidence establishes the declared indexed action."}))}).collect::<Result<Vec<_>>>()?;
        let payload = json!({"assessments":assessments});
        (key, op, Arm::Support, payload)
    } else {
        ensure!(
            !a.body.is_null()
                && bytes == serde_json::to_vec(&a.body)?
                && matches!(a.arm, Arm::Embedding) == embedding,
            "Frozen upfront worker bytes differ"
        );
        (a.key.clone(), a.operation, a.arm, a.payload.clone())
    };
    if let Some(lease) = &a.lease {
        let j = &lease.job;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE id=$1 AND brain_id=$2 AND target_id=$3 AND actor_id=$4 AND device_id IS NOT DISTINCT FROM $5 AND lease_token=$6 AND lease_until>clock_timestamp() AND state='running')").bind(j.id).bind(j.brain_id).bind(j.target_id).bind(j.actor_id).bind(j.device_id).bind(j.lease_token).fetch_one(&lease.admin).await?;
        ensure!(
            valid,
            "Exact worker lease or target drift before reservation"
        );
    }
    let canonical_hash = io::sha(bytes)?;
    let provider_bytes = super::transport::provider(bytes, embedding)?;
    let hash = io::sha(&provider_bytes)?;
    // Exclusive durable provider capture and dual-hash binding precede every reservation.
    let provider_path = w.out.join(format!("provider-body-{operation}.json"));
    io::write(&provider_path, &provider_bytes)?;
    io::json(
        &w.out.join(format!("provider-binding-{operation}.json")),
        &json!({"operation_id":operation,"canonical_sha256":canonical_hash,"provider_sha256":hash,"canonical_bytes":bytes.len(),"provider_bytes":provider_bytes.len(),"provider_path":provider_path.file_name(),"rule":super::transport::rule(),"rule_sha256":io::value_hash(&super::transport::rule())?}),
    )?;
    let mut ledger = w.ledger.lock().await;
    if let Some(l) = ledger.as_mut() {
        l.reserve(arm, &key, operation, &provider_bytes, &hash)?;
    }
    let response: Result<Value> = if let (Some(client), Some(secret)) = (&w.client, &w.key) {
        w.external.fetch_add(1, Ordering::SeqCst);
        fetch(w, client, secret, operation, embedding, &provider_bytes).await
    } else {
        w.controlled.fetch_add(1, Ordering::SeqCst);
        Ok(if embedding {
            let inputs = body["input"].as_array().context("Embedding batch")?;
            let mut v = vec![0.0; 3072];
            v[0] = 1.0;
            json!({"object":"list","model":body["model"],"data":inputs.iter().enumerate().map(|(i,_)|json!({"object":"embedding","index":i,"embedding":v})).collect::<Vec<_>>(),"usage":{"prompt_tokens":10,"total_tokens":10}})
        } else {
            let mut response = json!({"service_tier":"default","model":body["model"],"status":"completed","output":[{"type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":payload["__controlled_raw_text"].as_str().map(str::to_owned).unwrap_or_else(||payload.to_string())}]}],"usage":{"input_tokens":10,"output_tokens":10,"total_tokens":20}});
            // Controlled-only protocol fault fixture. The approved fetch branch above
            // cannot select or modify a returned provider tier.
            if let Some(tier) = payload.get("__controlled_response_tier") {
                if tier.is_null() {
                    response
                        .as_object_mut()
                        .context("Controlled object")?
                        .remove("service_tier");
                } else {
                    response["service_tier"] = tier.clone();
                }
            }
            response
        })
    };
    if w.client.is_none() {
        if let Ok(v) = &response {
            let raw = serde_json::to_vec(v)?;
            io::write(
                &w.out
                    .join("raw-responses")
                    .join(format!("{operation}.json")),
                &raw,
            )?;
            io::json(
                &w.out
                    .join("raw-response-bindings")
                    .join(format!("{operation}.json")),
                &json!({"operation_id":operation,"raw_sha256":io::sha(&raw)?,"retained_bytes":raw.len(),"status":200,"complete":true,"controlled":true}),
            )?;
        }
    }
    let acceptable = response.as_ref().is_ok_and(|r| {
        guard::response_ok(r, embedding)
            && (!embedding
                || r["data"].as_array().map(Vec::len) == body["input"].as_array().map(Vec::len))
    });
    if let Some(l) = ledger.as_mut() {
        l.finish(&key,acceptable,json!({"response_sha256":io::hash_file(&w.out.join("raw-responses").join(format!("{operation}.json"))).ok(),"usage":response.as_ref().ok().map(|v|v["usage"].clone()),"returned_model":response.as_ref().ok().map(|v|v["model"].clone()),"delivery_may_have_occurred":w.client.is_some()}))?;
    }
    let observation = json!({"operation_id":operation,"body_sha256":hash,"canonical_body_sha256":canonical_hash,"provider_body_sha256":hash,"transport_rule_sha256":io::value_hash(&super::transport::rule())?,"authorized_schedule_key":key,"acceptable_envelope_usage_model":acceptable,"response_sha256":io::hash_file(&w.out.join("raw-responses").join(format!("{operation}.json"))).ok(),"parsed_candidate_content":response.as_ref().ok().and_then(|r|r["output"].as_array()).is_some(),"embedding":embedding});
    io::json(
        &w.out
            .join("wire-outcomes")
            .join(format!("{operation}.json")),
        &observation,
    )?;
    w.outcomes.lock().await.insert(operation, observation);
    ensure!(acceptable, "Response usage/model/shape failed");
    if is_support {
        a.control = None;
    } else {
        a.body = Value::Null;
    }
    response
}
pub fn template(inv: &gateway::Invocation) -> Result<Value> {
    let gateway::Format::Json { name, schema } = &inv.format else {
        anyhow::bail!("Support format")
    };
    Ok(
        json!({"instructions":inv.instructions,"prompt_label":inv.prompt_label,"schema_label":inv.schema_label,"format_name":name,"schema":schema,"maximum_calls":6,"canonical_dependencies_only":true,"actual_body_freeze":"before_each_transmission"}),
    )
}

async fn fetch(
    w: &Wire,
    client: &reqwest::Client,
    secret: &str,
    operation: Uuid,
    embedding: bool,
    bytes: &[u8],
) -> Result<Value> {
    let url = if embedding {
        "https://api.openai.com/v1/embeddings"
    } else {
        "https://api.openai.com/v1/responses"
    };
    let mut raw = Vec::new();
    let mut status = None;
    let mut complete = false;
    let mut oversized = false;
    let fetched: Result<()> = async {
        let mut r = client
            .post(url)
            .bearer_auth(secret)
            .header("content-type", "application/json")
            .body(bytes.to_vec())
            .send()
            .await?;
        status = Some(r.status().as_u16());
        ensure!(r.url().as_str() == url, "Fixed upstream changed");
        while let Some(chunk) = r.chunk().await? {
            let remaining = (2 * 1024 * 1024usize).saturating_sub(raw.len());
            raw.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
            if chunk.len() > remaining {
                oversized = true;
                anyhow::bail!("Response exceeded bound");
            }
        }
        complete = true;
        Ok(())
    }
    .await;
    io::write(
        &w.out
            .join("raw-responses")
            .join(format!("{operation}.json")),
        &raw,
    )?;
    io::json(
        &w.out
            .join("raw-response-bindings")
            .join(format!("{operation}.json")),
        &json!({"operation_id":operation,"raw_sha256":io::sha(&raw)?,"retained_bytes":raw.len(),"status":status,"complete":complete,"oversized_truncated":oversized,"delivery_may_have_occurred":true,"transport_completed":fetched.is_ok()}),
    )?;
    fetched?;
    ensure!(
        status.is_some_and(|s| (200..300).contains(&s)),
        "Provider status failed"
    );
    io::strict(&raw)
}
