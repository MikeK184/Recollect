//! Native canonical reconciliation controls. This setup has no paid call path.
//! Prior machine state is produced by actual learning + independent support
//! against a private loopback provider, never by inserting model history.
use anyhow::{Context, Result, ensure};
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::State,
    http::Request,
    routing::post,
};
use recollect_protocol::{ClaimView, LearningRun, ScopeSelection};
use recollect_server::{AppState, app, autonomous, db, model_gateway as gateway, worker};
use serde_json::{Value, json};
use sqlx::{PgPool, types::Json as SqlJson};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

pub const CONTROLS_PATH: &str =
    "docs/research/automatic-reconciliation-native-controls-v3-2026-10-18.json";
pub const CONTROLS_SHA256: &str =
    "d15e0171a691e1656dcd4e3955eca39859c6b73a19b8257402c692e69101cf81";
const OWNERS: &[&str] = &[
    "Recollect disposable integration test created by crates/server/tests/platform.rs",
    "Recollect owned extractor benchmark; synthetic input only",
    "Recollect owned qualification benchmark; synthetic input only",
];

/// Actual API-generated IDs are mapped to frozen symbols; source/gold stay frozen.
/// `run.reconciliation_inputs` is the result of actual targets(), prepared in
/// memory only. learning::execute will independently select/persist them again.
pub struct PreparedControl {
    pub case_id: String,
    pub brain_id: Uuid,
    pub actor_id: Uuid,
    pub device_id: Uuid,
    pub environment_id: Uuid,
    pub policy_id: Uuid,
    pub operation_id: Uuid,
    pub run: LearningRun,
    pub prior_claim_id: Uuid,
    pub prior_revision_id: Uuid,
    pub offered: Vec<Uuid>,
    pub source: gateway::SourceText,
    pub canonical_inputs: Vec<Value>,
    pub canonical_id_map: Value,
    pub prior_workflow_receipt: Value,
    pub preview: gateway::Preview,
}

struct Browser {
    cookie: String,
    csrf: String,
    actor: Uuid,
}

fn product<T>(result: recollect_server::error::Result<T>) -> Result<T> {
    result.map_err(|e| anyhow::anyhow!("Canonical fixture operation failed: {}", e.1))
}
fn id(value: &Value) -> Result<Uuid> {
    value
        .as_str()
        .context("Canonical fixture ID missing")?
        .parse()
        .context("Invalid fixture ID")
}

async fn request(
    state: &AppState,
    browser: Option<&Browser>,
    token: Option<&str>,
    method: &str,
    uri: &str,
    body: Value,
) -> Result<(Value, String)> {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("idempotency-key", Uuid::new_v4().to_string());
    if let Some(browser) = browser {
        req = req
            .header("cookie", &browser.cookie)
            .header("x-csrf-token", &browser.csrf);
    }
    if let Some(token) = token {
        req = req.header("authorization", format!("Bearer {token}"));
    }
    let response = app(state.clone())
        .oneshot(req.body(Body::from(body.to_string()))?)
        .await?;
    let status = response.status();
    let cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .to_owned();
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024).await?;
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)?
    };
    ensure!(
        status.is_success(),
        "Canonical fixture HTTP {method} {uri} failed ({status}, code={})",
        body["error"]["code"]
            .as_str()
            .or_else(|| body["code"].as_str())
            .unwrap_or("redacted")
    );
    Ok((body, cookie))
}
async fn browser(state: &AppState) -> Result<Browser> {
    let (body, cookie) = request(
        state,
        None,
        None,
        "POST",
        "/api/auth/login",
        json!({"username":state.config.owner_username,"password":state.config.owner_password}),
    )
    .await?;
    Ok(Browser {
        cookie,
        csrf: body["csrf_token"]
            .as_str()
            .context("Fixture CSRF missing")?
            .into(),
        actor: id(&body["user"]["id"])?,
    })
}
async fn http(
    state: &AppState,
    b: &Browser,
    method: &str,
    uri: &str,
    body: Value,
) -> Result<Value> {
    Ok(request(state, Some(b), None, method, uri, body).await?.0)
}
async fn device(
    state: &AppState,
    token: &str,
    method: &str,
    uri: &str,
    body: Value,
) -> Result<Value> {
    Ok(request(state, None, Some(token), method, uri, body)
        .await?
        .0)
}

async fn guard_owned(state: &AppState, admin: &PgPool) -> Result<()> {
    let (database, owner):(String,Option<String>)=sqlx::query_as("SELECT current_database(),shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database()")
        .fetch_one(admin).await?;
    let app_database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&state.pool)
        .await?;
    ensure!(
        database == app_database
            && (database.starts_with("recollect_test_")
                || database.starts_with("recollect_extractor_")
                || database.starts_with("recollect_qualification_"))
            && owner.as_deref().is_some_and(|v| OWNERS.contains(&v)),
        "Reconciliation setup requires an exact owned empty synthetic database"
    );
    let busy: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM jobs WHERE lane='model' AND state IN ('queued','running')",
    )
    .fetch_one(admin)
    .await?;
    ensure!(
        busy == 0,
        "Finish/archive the preceding control before preparing another; never consume another queued model job through loopback"
    );
    Ok(())
}

#[derive(Clone)]
struct PriorProvider {
    candidate: Value,
    bodies: Arc<Mutex<Vec<Value>>>,
}
async fn prior_wire(State(p): State<PriorProvider>, Json(body): Json<Value>) -> Json<Value> {
    p.bodies.lock().unwrap().push(body.clone());
    let output = if body["text"]["format"]["name"] == "source_support" {
        let stage = body["input"]
            .as_str()
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|v| v["provenance"]["kind"] == "learning_support_stage")
            .unwrap();
        let stage: Value = serde_json::from_str(stage["data"].as_str().unwrap()).unwrap();
        let quote = body["input"]
            .as_str()
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter_map(|v| serde_json::from_str::<Value>(v["data"].as_str()?).ok())
            .find_map(|v| {
                v["cited_spans"].as_array()?.first()?["text"]
                    .as_str()
                    .map(str::to_owned)
            })
            .unwrap_or_default();
        json!({"assessments":stage["actions"].as_array().unwrap().iter().map(|a|
            json!({"index":a["index"],"disposition":"supported","reason":"Controlled synthetic prior support; plumbing only","citation_support":true,"evidence_quote":quote})).collect::<Vec<_>>()})
    } else {
        p.candidate.clone()
    };
    Json(
        json!({"model":body["model"],"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":output.to_string()}]}],
        "usage":{"input_tokens":23,"output_tokens":17,"total_tokens":40}}),
    )
}
struct LocalProvider(tokio::task::JoinHandle<()>);
impl Drop for LocalProvider {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn drain_capture(state: &AppState) -> Result<()> {
    for _ in 0..50 {
        if !run_worker(state, "capture").await? {
            return Ok(());
        }
    }
    anyhow::bail!("Synthetic capture queue did not drain within its bound")
}
// Match production's independently polled lane tasks without increasing the
// thread stack or changing model, lease, source, or publication behavior.
async fn run_worker(state: &AppState, lane: &str) -> Result<bool> {
    let state = state.clone();
    let lane = lane.to_owned();
    Ok(tokio::spawn(async move { worker::run_once(&state, &lane).await }).await??)
}

async fn read_run(admin: &PgPool, brain: Uuid, version: Uuid) -> Result<LearningRun> {
    let SqlJson(run):SqlJson<LearningRun>=sqlx::query_scalar("SELECT to_jsonb(r) FROM learning_runs r WHERE brain_id=$1 AND source_version_id=$2 AND automatic ORDER BY created_at DESC LIMIT 1")
        .bind(brain).bind(version).fetch_one(admin).await?;
    Ok(run)
}
async fn schedule(
    state: &AppState,
    admin: &PgPool,
    brain: Uuid,
    version: Uuid,
) -> Result<LearningRun> {
    product(autonomous::run_once(state).await)?;
    let run = read_run(admin, brain, version).await?;
    ensure!(
        run.automatic && run.automatic_attempt == 0 && run.state == "queued",
        "Expected actual initial automatic learning run"
    );
    Ok(run)
}
async fn claim_view(state: &AppState, b: &Browser, base: &str, claim: Uuid) -> Result<ClaimView> {
    let detail = http(
        state,
        b,
        "GET",
        &format!("{base}/claims/{claim}"),
        Value::Null,
    )
    .await?;
    serde_json::from_value(detail["selected"].clone())
        .context("Expected actual typed canonical ClaimView")
}

/// Setup transmits only earlier support text to the local provider. The current
/// source is stored but never sent anywhere. No current response/model history
/// is fabricated. Callers must independently validate body/schema before any
/// separately authorized paid execution.
pub async fn setup_case(state: &AppState, admin: &PgPool, case: &Value) -> Result<PreparedControl> {
    eprintln!("Canonical control setup phase: entered");
    guard_owned(state, admin).await?;
    let case_id = case["id"].as_str().context("Control ID")?;
    ensure!(
        ["rc01", "rc02", "rc03", "rc04", "rc05", "rc06"].contains(&case_id),
        "Unknown frozen control"
    );
    let memories = case["canonical_memories"]
        .as_array()
        .context("Control prior memory")?;
    ensure!(
        memories.len() == 1,
        "Control must have exactly one prior canonical memory"
    );
    let memory = &memories[0];
    let prior_text = memory["support"]["text"]
        .as_str()
        .context("Prior support text")?;
    let current_text = case["source"]["text"]
        .as_str()
        .context("Current source text")?;
    let candidate = json!({"claims":[{"subject":memory["subject"],"predicate":memory["predicate"],"value":memory["value"],
        "rationale":"Declared by this earlier synthetic source.","kind":"claim","procedure":null,"context_role":null,
        "line_from":1,"line_to":1,"replaces_revision":null}],"retirements":[]});
    let prior_provider = PriorProvider {
        candidate,
        bodies: Arc::new(Mutex::new(vec![])),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    ensure!(
        address.ip().is_loopback(),
        "Prior setup provider must be loopback"
    );
    let provider = Router::new()
        .route("/v1/responses", post(prior_wire))
        .with_state(prior_provider.clone());
    let _local = LocalProvider(tokio::spawn(async move {
        let _ = axum::serve(listener, provider).await;
    }));
    let mut config = (*state.config).clone();
    config.models.key = Some("synthetic-loopback-fixture-only".into());
    config.models.openrouter_key = None;
    config.models.endpoint = format!("http://{address}/v1");
    config.models.openrouter_endpoint = "http://127.0.0.1:9".into();
    let prior_state = AppState::new(state.pool.clone(), config)?;
    let owner = browser(&prior_state).await?;
    eprintln!("Canonical control setup phase: owner_authenticated");
    let brain = http(
        &prior_state,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":format!("Owned native {case_id}")}),
    )
    .await?;
    let brain_id = id(&brain["id"])?;
    let base = format!("/api/brains/{brain_id}");
    let invitation = http(
        &prior_state,
        &owner,
        "POST",
        "/api/team/invitations",
        json!({"username":format!("Cassandra {case_id}")}),
    )
    .await?;
    let (enrolled, cookie) = request(
        &prior_state,
        None,
        None,
        "POST",
        "/api/auth/enroll",
        json!({"token":invitation["token"],"password":Uuid::new_v4().to_string()}),
    )
    .await?;
    let b = Browser {
        cookie,
        csrf: enrolled["csrf_token"]
            .as_str()
            .context("Fixture actor CSRF")?
            .into(),
        actor: id(&enrolled["user"]["id"])?,
    };
    http(
        &prior_state,
        &owner,
        "PUT",
        &format!("{base}/grants/{}", b.actor),
        json!({"role":"admin"}),
    )
    .await?;
    let environment = http(
        &prior_state,
        &b,
        "POST",
        &format!("{base}/evidence/groups"),
        json!({"kind":"environment","name":"Synthetic native control"}),
    )
    .await?;
    let environment_id = id(&environment["id"])?;
    let selection = json!({"repository_ids":[],"area_ids":[],"environment_id":environment_id});
    let (start, _) = request(
        &prior_state,
        None,
        None,
        "POST",
        "/api/devices/pairings",
        json!({"name":"Owned native control device"}),
    )
    .await?;
    http(
        &prior_state,
        &b,
        "POST",
        &format!(
            "/api/devices/pairings/{}/approve",
            start["user_code"].as_str().context("Pairing user code")?
        ),
        json!({"approve":true}),
    )
    .await?;
    let (poll, _) = request(
        &prior_state,
        None,
        None,
        "POST",
        "/api/devices/pairings/poll",
        json!({"device_code":start["device_code"]}),
    )
    .await?;
    let token = poll["token"]
        .as_str()
        .context("Private fixture token")?
        .to_owned();
    let device_id = id(&poll["device"]["id"])?;
    request(
        &prior_state,
        None,
        None,
        "POST",
        "/api/devices/pairings/finish",
        json!({"device_code":start["device_code"]}),
    )
    .await?;
    let task = device(
        &prior_state,
        &token,
        "POST",
        &format!("{base}/workspace/tasks"),
        json!({"label":"Native control source scope","selection":selection}),
    )
    .await?;
    let operation = device(
        &prior_state,
        &token,
        "POST",
        &format!(
            "{base}/workspace/tasks/{}/operations",
            task["task"]["id"].as_str().context("Task ID")?
        ),
        json!({"kind":if case_id=="rc04" {"capture"} else {"write"}}),
    )
    .await?;
    let operation_id = id(&operation["id"])?;
    let settings = http(
        &prior_state,
        &b,
        "GET",
        &format!("{base}/models/policy"),
        Value::Null,
    )
    .await?;
    let mut policy = settings["current"]["policy"].clone();
    policy["enabled"] = json!(true);
    policy["autonomous_memory"] = json!(true);
    policy["automatic_learning"] = json!(false);
    policy["automatic_embedding"] = json!(false);
    policy["provider"] = json!("openai");
    policy["text_model"] = json!(state.config.models.text_model);
    policy["purposes"] = json!(["extraction", "synthesis"]);
    policy["content_classes"] = json!(["document", "raw_session", "claim", "support_excerpt"]);
    // Source+offered-input bytes and complete serialized request bytes have
    // separate campaign bounds; the product policy permits at most 32 KiB.
    policy["max_input_bytes"] = json!(16384);
    policy["max_output_tokens"] = json!(4096);
    policy["daily_token_limit"] = json!(2_000_000);
    policy["max_concurrent"] = json!(1);
    policy["acceptance"] = Value::Null;
    let saved = http(
        &prior_state,
        &b,
        "PUT",
        &format!("{base}/models/policy"),
        json!({"base_change":settings["current"]["change_id"],"policy":policy}),
    )
    .await?;
    let policy_id = id(&saved["change_id"])?;
    eprintln!("Canonical control setup phase: policy_ready");
    let mut binding_id = None;
    let mut old_event_id = None;
    let old_source = if case_id == "rc04" {
        let p = http(
            &prior_state,
            &b,
            "GET",
            &format!("{base}/capture/policy"),
            Value::Null,
        )
        .await?;
        let mut policy = p["policy"].clone();
        policy["enabled"] = json!(true);
        http(
            &prior_state,
            &b,
            "PUT",
            &format!("{base}/capture/policy"),
            json!({"base_change":p["change_id"],"policy":policy}),
        )
        .await?;
        let binding=device(&prior_state,&token,"POST",&format!("{base}/capture/bindings"),json!({"id":Uuid::new_v4(),"operation_id":operation_id,"host":"codex","host_version":"0.154.0"})).await?;
        binding_id = Some(id(&binding["id"])?);
        let event = capture(binding_id.unwrap(), "prior-control", prior_text);
        old_event_id = Some(id(&event["id"])?);
        let s = device(
            &prior_state,
            &token,
            "POST",
            &format!("{base}/capture/events"),
            event,
        )
        .await?;
        json!({"id":s["source_id"],"version":{"id":s["source_version_id"]}})
    } else {
        device(&prior_state,&token,"POST",&format!("{base}/sources"),json!({"operation_id":operation_id,"title":"Cassandra migration note","media_type":"text/plain","content":prior_text,"retain_content":true})).await?
    };
    let old_source_id = id(&old_source["id"])?;
    let old_version = id(&old_source["version"]["id"])?;
    drain_capture(&prior_state).await?;
    let mut prior_run = None;
    let prior_claim = if case_id == "rc06" {
        let content = json!({"kind":"claim","subject":memory["subject"],"predicate":memory["predicate"],"value":memory["value"],"rationale":"Human-authored synthetic control.",
            "selection":selection,"manifest_revision_id":null,"validity":{"kind":"unknown","from":null,"to":null,"precision":"unknown"},
            "freshness":"current","operational":"declared","observed_at":null,"observation":"","supports":[{"kind":"source_version","id":old_version,"line_from":1,"line_to":1}]} );
        let claim = http(
            &prior_state,
            &b,
            "POST",
            &format!("{base}/claims"),
            json!({"base_revision":null,"operation_id":null,"content":content}),
        )
        .await?;
        let claim_id = id(&claim["claim_id"])?;
        http(&prior_state,&b,"POST",&format!("{base}/claims/{claim_id}/review"),json!({"base_revision":claim["id"],"action":"accept","reason":"Independent synthetic human review control.","content":null,"revalidation_basis":null})).await?;
        claim_id
    } else {
        let run = schedule(&prior_state, admin, brain_id, old_version).await?;
        let queued: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id,target_id FROM jobs WHERE lane='model' AND state IN ('queued','running')",
        )
        .fetch_all(admin)
        .await?;
        ensure!(
            queued == vec![(run.job_id, run.id)],
            "Refuse another model job during prior fixture setup"
        );
        ensure!(
            run_worker(&prior_state, "model").await?,
            "Prior learning job was not claimed"
        );
        let result = read_run(admin, brain_id, old_version).await?;
        ensure!(
            result.state == "succeeded" && result.claim_ids.len() == 1,
            "Prior canonical learning/support did not publish exactly one claim"
        );
        let claim = result.claim_ids[0];
        prior_run = Some(result);
        claim
    };
    let before = claim_view(&prior_state, &b, &base, prior_claim).await?;
    eprintln!("Canonical control setup phase: prior_claim_ready");
    ensure!(
        before.revision.content.selection
            == serde_json::from_value::<ScopeSelection>(selection.clone())?
            && before.eligibility.investigation,
        "Prior claim is not canonically eligible in exact control scope"
    );
    if case_id == "rc06" {
        ensure!(
            before.revision.reviewer_id == Some(b.actor)
                && before.revision.review_decision_id.is_some()
                && !autonomous::maintained(&before.revision),
            "Manual control must carry actual human review attribution"
        );
    } else {
        ensure!(
            before.revision.origin == "model_extracted"
                && before.revision.reviewer_id.is_none()
                && before.revision.review_decision_id.is_none()
                && before.revision.review == "accepted"
                && autonomous::maintained(&before.revision),
            "Machine prior must come from policy publication without human review"
        );
    }
    let current_source = if case_id == "rc04" {
        let event = capture(
            binding_id.context("Capture binding missing")?,
            "current-control",
            current_text,
        );
        let s = device(
            &prior_state,
            &token,
            "POST",
            &format!("{base}/capture/events"),
            event,
        )
        .await?;
        json!({"id":s["source_id"],"version":{"id":s["source_version_id"]}})
    } else {
        device(&prior_state,&token,"POST",&format!("{base}/sources/{old_source_id}/versions"),json!({"operation_id":operation_id,"base_version":old_version,"title":"Current reconciliation source","media_type":"text/plain","content":current_text,"retain_content":true})).await?
    };
    let current_source_id = id(&current_source["id"])?;
    let current_version = id(&current_source["version"]["id"])?;
    ensure!(
        (old_source_id == current_source_id) == (case_id != "rc04"),
        "Wrong actual same-source/capture-session lineage"
    );
    drain_capture(&prior_state).await?;
    let mut run = schedule(&prior_state, admin, brain_id, current_version).await?;
    eprintln!("Canonical control setup phase: current_source_queued");
    let mut tx = product(db::preparation_tx(&state.pool, run.actor_id, run.device_id).await)?;
    let offered = product(autonomous::targets(state, &mut tx, &run, 16384).await)?;
    let expected = if case_id == "rc06" {
        vec![]
    } else {
        vec![before.revision.id]
    };
    ensure!(
        offered == expected,
        "Actual autonomous target selector differs from frozen control symbols"
    );
    let source = product(gateway::source(state, &mut tx, brain_id, current_version, 16384).await)?;
    ensure!(
        source.text == current_text && source.source_id == current_source_id,
        "Canonical input0 changed exact frozen bytes or identity"
    );
    ensure!(
        source.provenance["role"] == case["input0_provenance"]["role"],
        "Canonical provenance differs from corrected frozen control"
    );
    if case_id == "rc04" {
        ensure!(
            source.provenance["capture"]["binding_id"] == json!(binding_id)
                && source.provenance["capture"]["host_session_id"] == "synthetic-reconciliation-4"
                && source.provenance["capture"]["agent_id"].is_null()
                && source.provenance["capture"]["selection"] == selection,
            "Current capture does not preserve exact authenticated binding/session/agent/scope"
        );
    } else {
        ensure!(
            source.provenance["capture"].is_null(),
            "Ordinary source version must not impersonate captured prompt"
        );
    }
    tx.commit().await?;
    let mut canonical_inputs =
        vec![json!({"input":0,"data":source.text,"provenance":source.provenance})];
    if !offered.is_empty() {
        let view = claim_view(state, &b, &base, prior_claim).await?;
        ensure!(
            view.revision.id == before.revision.id
                && view.evidence.iter().all(|e| e.availability == "retained"),
            "Prior canonical view changed or lost retained evidence"
        );
        canonical_inputs.push(json!({"input":1,"data":serde_json::to_string(&view)?,"provenance":{"kind":"claim_revision","id":view.revision.id}}));
    }
    run.reconciliation_inputs = offered.clone();
    let invocation = autonomous::invocation(&run);
    eprintln!("Canonical control setup phase: invocation_ready");
    ensure!(
        invocation.inputs.len() == canonical_inputs.len()
            && invocation.prompt_label == autonomous::PROMPT,
        "Actual invocation input shape changed"
    );
    // This uses the actual private gateway resolver through its read-only
    // preparation API. The fake key only satisfies local fixture policy; it
    // never leaves this short-lived loopback-config AppState.
    let preview = product(
        gateway::preview_for_policy(
            &prior_state,
            gateway::Context {
                brain: brain_id,
                actor: run.actor_id,
                device: run.device_id,
            },
            &invocation,
            policy_id,
        )
        .await,
    )?;
    let resolved: Vec<Value> = preview.body["input"]
        .as_str()
        .context("Canonical gateway input missing")?
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;
    ensure!(
        resolved == canonical_inputs,
        "API typed ClaimView and actual gateway resolver disagree"
    );
    let requests: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id,prompt_label,state FROM model_requests WHERE brain_id=$1 ORDER BY created_at,id",
    )
    .bind(brain_id)
    .fetch_all(admin)
    .await?;
    let bodies = prior_provider.bodies.lock().unwrap().clone();
    let expected_requests = if case_id == "rc06" { 0 } else { 2 };
    ensure!(
        requests.len() == expected_requests
            && bodies.len() == expected_requests
            && requests.iter().all(|(_, _, s)| s == "succeeded"),
        "Prior workflow must contain only one loopback extraction and one independent support call"
    );
    let current_requests: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM model_requests WHERE brain_id=$1 AND operation_id=$2",
    )
    .bind(brain_id)
    .bind(run.id)
    .fetch_one(admin)
    .await?;
    ensure!(
        current_requests == 0,
        "Setup must never consume current evaluation request identity"
    );
    let canonical_id_map = json!({"brain":{"frozen":"61000000-0000-4000-8000-000000000001","actual":brain_id},
        "actor":{"frozen":"62000000-0000-4000-8000-000000000001","actual":b.actor},
        "device":{"frozen":"63000000-0000-4000-8000-000000000001","actual":device_id},
        "environment":{"frozen":"6c000000-0000-4000-8000-000000000001","actual":environment_id},
        "operation":{"frozen":case["operation_id"],"actual":operation_id},"run":{"frozen":case["run_id"],"actual":run.id},
        "source":{"frozen":case["source"]["source_id"],"actual":current_source_id},"version":{"frozen":case["source"]["version_id"],"actual":current_version},
        "prior_claim":{"frozen":memory["claim_id"],"actual":prior_claim},"prior_revision":{"symbol":memory["symbol"],"frozen":memory["revision_id"],"actual":before.revision.id},
        "prior_source":{"frozen":memory["support"]["source_id"],"actual":old_source_id},"prior_version":{"frozen":memory["support"]["version_id"],"actual":old_version}});
    let prior_workflow_receipt = json!({"kind":"actual_canonical_pipeline_with_synthetic_loopback_prior_outputs","paid_calls":0,"prior_run":prior_run,
        "prior_request_ids":requests.iter().map(|r|r.0).collect::<Vec<_>>(),"loopback_calls":bodies.len(),"prior_provider_inputs":bodies.iter().map(|b|b["input"].clone()).collect::<Vec<_>>(),
        "prior_claim":prior_claim,"prior_revision":before.revision.id,"reviewer_id":before.revision.reviewer_id,"review_decision_id":before.revision.review_decision_id,
        "input1_metadata_only_entity":"Cassandra","capture_binding_id":binding_id,"prior_capture_event_id":old_event_id,
        "current_run_queued":run.id,"current_provider_calls":0,"targets_persisted_by_setup":false,"normal_data_copied":false,"quality_evidence":false});
    Ok(PreparedControl {
        case_id: case_id.into(),
        brain_id,
        actor_id: b.actor,
        device_id,
        environment_id,
        policy_id,
        operation_id,
        run,
        prior_claim_id: prior_claim,
        prior_revision_id: before.revision.id,
        offered,
        source,
        canonical_inputs,
        canonical_id_map,
        prior_workflow_receipt,
        preview,
    })
}

fn capture(binding: Uuid, turn: &str, text: &str) -> Value {
    json!({"id":Uuid::new_v4(),"binding_id":binding,"event":{"host_event":"UserPromptSubmit","host_session_id":"synthetic-reconciliation-4",
        "turn_id":turn,"agent_id":null,"tool_use_id":null,"tool_name":null,"kind":"prompt","outcome":"reported","content":text,
        "coverage":["partial_host_coverage"],"captured_at":chrono::Utc::now()}})
}

/// Archive only after final outcome capture. This prevents later case setup
/// from scheduling unrelated prior-case audits; it never edits model history.
pub async fn archive_case(
    state: &AppState,
    admin: &PgPool,
    prepared: &PreparedControl,
) -> Result<()> {
    let b = browser(state).await?;
    let belongs: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM brains WHERE id=$1 AND owner_id=$2 AND name=$3)",
    )
    .bind(prepared.brain_id)
    .bind(b.actor)
    .bind(format!("Owned native {}", prepared.case_id))
    .fetch_one(admin)
    .await?;
    ensure!(belongs, "Refuse to archive an unrelated Brain");
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE id=$1 AND brain_id=$2 AND target_id=$3 AND kind='source.learn' AND state IN ('queued','running'))")
        .bind(prepared.run.job_id).bind(prepared.brain_id).bind(prepared.run.id).fetch_one(admin).await?;
    if pending {
        http(
            state,
            &b,
            "POST",
            &format!(
                "/api/brains/{}/jobs/{}/cancel",
                prepared.brain_id, prepared.run.job_id
            ),
            Value::Null,
        )
        .await?;
    }
    http(
        state,
        &b,
        "PATCH",
        &format!("/api/brains/{}", prepared.brain_id),
        json!({"archived":true}),
    )
    .await?;
    Ok(())
}

/// Native plumbing proof only. Execute the untouched queued current run through
/// the real worker with a fixed empty loopback response, then compare the wire
/// body with the read-only preview. This does not score semantic outcomes.
pub async fn prove_current_input_boundary(
    state: &AppState,
    admin: &PgPool,
    prepared: &PreparedControl,
) -> Result<Value> {
    let p = PriorProvider {
        candidate: json!({"claims":[],"retirements":[]}),
        bodies: Arc::new(Mutex::new(vec![])),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let provider = Router::new()
        .route("/v1/responses", post(prior_wire))
        .with_state(p.clone());
    let _local = LocalProvider(tokio::spawn(async move {
        let _ = axum::serve(listener, provider).await;
    }));
    let mut config = (*state.config).clone();
    config.models.endpoint = format!("http://{address}/v1");
    config.models.key = Some("synthetic-loopback-fixture-only".into());
    config.models.openrouter_key = None;
    let local = AppState::new(state.pool.clone(), config)?;
    let queued: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id,target_id FROM jobs WHERE lane='model' AND state IN ('queued','running')",
    )
    .fetch_all(admin)
    .await?;
    ensure!(
        queued == vec![(prepared.run.job_id, prepared.run.id)],
        "Refuse another model job during boundary proof"
    );
    ensure!(
        run_worker(&local, "model").await?,
        "Current boundary job missing"
    );
    let result = read_run(admin, prepared.brain_id, prepared.run.source_version_id).await?;
    ensure!(
        result.state == "succeeded"
            && result.reconciliation_inputs == prepared.offered
            && result.claim_ids.is_empty(),
        "Current empty probe failed or selected different actual memories"
    );
    let bodies = p.bodies.lock().unwrap().clone();
    ensure!(
        bodies.len() == 1 && bodies[0] == prepared.preview.body,
        "Actual worker gateway body differs from canonical read-only preview"
    );
    let original:(Uuid,Value)=sqlx::query_as("SELECT c.current_revision,r.revision FROM claims c JOIN claim_revisions r ON r.id=c.current_revision WHERE c.brain_id=$1 AND c.id=$2")
        .bind(prepared.brain_id).bind(prepared.prior_claim_id).fetch_one(admin).await?;
    ensure!(
        original.0 == prepared.prior_revision_id,
        "Empty provider probe changed the prior canonical identity"
    );
    Ok(
        json!({"run_id":result.id,"request_id":result.request_id,"offered":result.reconciliation_inputs,"actual_provider_body":bodies[0],
        "loopback_current_calls":1,"paid_calls":0,"canonical_prior_unchanged":true,"semantic_quality_measured":false}),
    )
}

/// Construct but do not poll product futures; no provider or SQL work occurs.
pub fn product_future_sizes(state: &AppState, p: &PreparedControl) -> Value {
    let job = worker::ClaimedJob {
        id: p.run.job_id,
        brain_id: p.brain_id,
        actor_id: p.run.actor_id,
        target_id: p.run.id,
        kind: "source.learn".into(),
        lease_token: Uuid::nil(),
        device_id: p.run.device_id,
    };
    let learning = std::mem::size_of_val(&recollect_server::learning::execute(state, &job));
    let gateway = std::mem::size_of_val(&gateway::invoke_for_policy(
        state,
        gateway::Context {
            brain: p.brain_id,
            actor: p.run.actor_id,
            device: p.run.device_id,
        },
        autonomous::invocation(&p.run),
        p.policy_id,
    ));
    let worker = std::mem::size_of_val(&worker::run_once(state, "model"));
    json!({"learning_execute_bytes":learning,"gateway_invoke_for_policy_bytes":gateway,"worker_run_once_bytes":worker,
        "diagnostic":"constructed_not_polled","runtime":"tokio current-thread test runtime with independent spawned worker tasks; standard Rust test thread, RUST_MIN_STACK unset"})
}
