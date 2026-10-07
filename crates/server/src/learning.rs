use crate::{
    AppState,
    auth::Auth,
    autonomous, commands, db,
    error::{Error, Result},
    jobs, memory,
    memory_evidence::Tx,
    memory_policy, memory_rules, memory_support, model_gateway as gateway, model_policy,
    publication,
    worker::{self, ClaimedJob, Failure},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    #[serde(default = "claim_kind")]
    kind: String,
    subject: String,
    predicate: String,
    value: String,
    rationale: String,
    #[serde(default)]
    context_role: Option<String>,
    line_from: i32,
    line_to: i32,
    #[serde(default)]
    replaces_revision: Option<Uuid>,
    #[serde(default)]
    procedure: Option<LearnedProcedure>,
}
fn claim_kind() -> String {
    "claim".into()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LearnedProcedure {
    conditions: String,
    steps: Vec<String>,
    expected_outcome: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Retirement {
    revision_id: Uuid,
    reason: String,
    line_from: i32,
    line_to: i32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidates {
    claims: Vec<Candidate>,
    #[serde(default)]
    retirements: Vec<Retirement>,
}
#[derive(Default, Deserialize)]
pub struct LearningQuery {
    pub offset: Option<i64>,
    pub source_version_id: Option<Uuid>,
}
pub(crate) async fn row(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<LearningRun> {
    let r: SqlJson<LearningRun> =
        sqlx::query_scalar("SELECT to_jsonb(r) FROM learning_runs r WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(Error::missing)?;
    Ok(r.0)
}
fn content(input: &LearningInput, c: Candidate) -> ClaimContent {
    ClaimContent {
        kind: c.kind,
        subject: c.subject,
        predicate: c.predicate,
        value: c.value,
        rationale: c.rationale,
        context_role: c.context_role,
        selection: input.selection.clone(),
        manifest_revision_id: input.manifest_revision_id,
        validity: FactValidity {
            kind: "unknown".into(),
            from: None,
            to: None,
            precision: "unknown".into(),
        },
        freshness: "current".into(),
        operational: "declared".into(),
        observed_at: None,
        observation: String::new(),
        supports: vec![ClaimSupport {
            kind: "source_version".into(),
            id: input.source_version_id,
            line_from: Some(c.line_from),
            line_to: Some(c.line_to),
        }],
        procedure: c.procedure.map(|p| ProcedureContent {
            conditions: p.conditions,
            steps: p.steps,
            expected_outcome: p.expected_outcome,
            observations: vec![],
        }),
        handover: None,
    }
}
async fn validate_scope(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
    input: &mut LearningInput,
    automatic: bool,
) -> Result<()> {
    let mut draft = content(
        input,
        Candidate {
            kind: "claim".into(),
            subject: "Source".into(),
            predicate: "declaration".into(),
            value: "Learning request".into(),
            rationale: String::new(),
            context_role: None,
            line_from: 1,
            line_to: 1,
            replaces_revision: None,
            procedure: None,
        },
    );
    memory_policy::validate(&mut draft)?;
    input.selection = draft.selection.clone();
    memory::validate_evidence(state, tx, brain, &draft).await?;
    if !automatic && device.is_some() && input.operation_id.is_none() {
        return Err(Error::forbidden());
    }
    if let Some(id) = input.operation_id {
        let operation =
            publication::operation(tx, brain, id, actor, device, &input.selection, true).await?;
        if operation.kind != "write" || operation.scope.selection != input.selection {
            return Err(Error::forbidden());
        }
    }
    Ok(())
}
pub(crate) enum QueueMode {
    Requested,
    Automatic,
    Retry { parent: Uuid, attempt: i32 },
}
pub(crate) async fn enqueue(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
    input: &LearningInput,
    policy: Uuid,
    mode: QueueMode,
) -> Result<LearningRun> {
    let (automatic, parent, attempt, disposition) = match mode {
        QueueMode::Requested => (false, None, 0, "requested"),
        QueueMode::Automatic => (true, None, 0, "automatic"),
        QueueMode::Retry { parent, attempt } => (true, Some(parent), attempt, "automatic_retry"),
    };
    let id = Uuid::new_v4();
    let audit = db::audit(tx, actor, brain, "learning.queue", id, disposition).await?;
    let job = jobs::enqueue_work(tx, actor, brain, audit, id, "source.learn", "model").await?;
    sqlx::query("UPDATE jobs SET device_id=$2 WHERE id=$1")
        .bind(job)
        .bind(device)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO learning_runs(id,brain_id,source_version_id,policy_id,actor_id,device_id,operation_id,selection,manifest_revision_id,job_id,automatic,state,retry_of,automatic_attempt) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'queued',$12,$13)")
        .bind(id).bind(brain).bind(input.source_version_id).bind(policy).bind(actor).bind(device).bind(input.operation_id).bind(SqlJson(&input.selection)).bind(input.manifest_revision_id).bind(job).bind(automatic).bind(parent).bind(attempt).execute(&mut **tx).await?;
    row(tx, brain, id).await
}
async fn save(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    headers: &HeaderMap,
    mut input: LearningInput,
    retry: Option<Uuid>,
) -> Result<LearningRun> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    let key = commands::key(headers)?;
    if let Some(saved) = commands::reserve::<LearningRun>(
        &mut tx,
        key.as_deref(),
        "learning.queue",
        json!({"brain":brain,"input":input,"retry":retry}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(saved);
    }
    if let Some(id) = retry {
        let old = row(&mut tx, brain, id).await?;
        if !matches!(old.state.as_str(), "failed" | "cancelled") {
            return Err(model_policy::failure(
                "learning_not_retryable",
                "Only failed or cancelled learning can start a replacement attempt.",
            ));
        }
    }
    validate_scope(
        state,
        &mut tx,
        brain,
        auth.user.id,
        auth.device_id,
        &mut input,
        false,
    )
    .await?;
    let policy = model_policy::current(state, &mut tx, brain).await?;
    let source = gateway::source(
        state,
        &mut tx,
        brain,
        input.source_version_id,
        policy.policy.max_input_bytes as usize,
    )
    .await?;
    model_policy::permits(state, &policy.policy, "extraction", &[source.class])?;
    let result = enqueue(
        &mut tx,
        brain,
        auth.user.id,
        auth.device_id,
        &input,
        policy.change_id,
        QueueMode::Requested,
    )
    .await?;
    commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    tx.commit().await?;
    Ok(result)
}
#[utoipa::path(post,path="/api/brains/{brain}/learning",operation_id="startLearning",params(("brain"=Uuid,Path)),request_body=LearningInput,responses((status=200,body=LearningRun)))]
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<LearningInput>,
) -> Result<Json<LearningRun>> {
    save(&state, &auth, brain, &headers, input, None)
        .await
        .map(Json)
}
#[utoipa::path(post,path="/api/brains/{brain}/learning/{run}/retry",operation_id="retryLearning",params(("brain"=Uuid,Path),("run"=Uuid,Path)),request_body=LearningRetryInput,responses((status=200,body=LearningRun)))]
pub async fn retry(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<LearningRetryInput>,
) -> Result<Json<LearningRun>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    let old = row(&mut tx, brain, id).await?;
    tx.commit().await?;
    let input = LearningInput {
        source_version_id: old.source_version_id,
        selection: old.selection,
        manifest_revision_id: old.manifest_revision_id,
        operation_id: input.operation_id,
    };
    save(&state, &auth, brain, &headers, input, Some(id))
        .await
        .map(Json)
}
#[utoipa::path(get,path="/api/brains/{brain}/learning",operation_id="listLearning",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query),("source_version_id"=Option<Uuid>,Query)),responses((status=200,body=LearningPage)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<LearningQuery>,
) -> Result<Json<LearningPage>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, false).await?;
    let offset = model_policy::offset(&model_policy::PageQuery {
        offset: query.offset,
    })?;
    let total=sqlx::query_scalar("SELECT count(*) FROM learning_runs WHERE brain_id=$1 AND ($2::uuid IS NULL OR source_version_id=$2)")
        .bind(brain).bind(query.source_version_id).fetch_one(&mut *tx).await?;
    let rows:Vec<SqlJson<LearningRun>>=sqlx::query_scalar("SELECT to_jsonb(r) FROM learning_runs r WHERE brain_id=$1 AND ($2::uuid IS NULL OR source_version_id=$2) ORDER BY created_at DESC,id DESC LIMIT 20 OFFSET $3")
        .bind(brain).bind(query.source_version_id).bind(offset).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(LearningPage {
        items: rows.into_iter().map(|r| r.0).collect(),
        total,
        offset,
    }))
}
pub(crate) async fn automatic(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    source: Uuid,
) -> Result<()> {
    let automatic_copy: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM automatic_support_excerpts WHERE brain_id=$1 AND version_id=$2)")
        .bind(brain).bind(source).fetch_one(&mut **tx).await?;
    if automatic_copy {
        return Ok(());
    }
    let policy = model_policy::current(state, tx, brain).await?;
    let p = &policy.policy;
    // Autonomous work is scheduled under the standing policy grant by the
    // independent maintenance lane; capture must not wait for model capacity.
    if p.autonomous_memory {
        return Ok(());
    }
    if !p.enabled
        || !p.automatic_learning
        || !p.purposes.iter().any(|v| v == "extraction")
        || !model_policy::matches_installation(state, p)
    {
        return Ok(());
    }
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM learning_runs WHERE brain_id=$1 AND source_version_id=$2 AND automatic)")
        .bind(brain).bind(source).fetch_one(&mut **tx).await?;
    if exists {
        return Ok(());
    }
    let (actor,device,class,retained):(Uuid,Option<Uuid>,String,bool)=sqlx::query_as("SELECT created_by,device_id,retention_class,artifact_id IS NOT NULL FROM source_versions WHERE brain_id=$1 AND id=$2")
        .bind(brain).bind(source).fetch_one(&mut **tx).await?;
    if !retained || !p.content_classes.contains(&class) {
        return Ok(());
    }
    let selection = crate::capture::source_selection(tx, brain, source).await?;
    enqueue(
        tx,
        brain,
        actor,
        device,
        &LearningInput {
            source_version_id: source,
            selection,
            manifest_revision_id: None,
            operation_id: None,
        },
        policy.change_id,
        QueueMode::Automatic,
    )
    .await?;
    Ok(())
}
fn input(run: &LearningRun) -> LearningInput {
    LearningInput {
        source_version_id: run.source_version_id,
        selection: run.selection.clone(),
        manifest_revision_id: run.manifest_revision_id,
        operation_id: run.operation_id,
    }
}
async fn lease(tx: &mut Tx<'_>, job: &ClaimedJob) -> Result<()> {
    let valid:Option<bool>=sqlx::query_scalar("SELECT lease_until>clock_timestamp() FROM jobs WHERE id=$1 AND brain_id=$2 AND state='running' AND lease_token=$3 FOR UPDATE")
        .bind(job.id).bind(job.brain_id).bind(job.lease_token).fetch_optional(&mut **tx).await?;
    if valid != Some(true) {
        return Err(model_policy::failure(
            "learning_lease_lost",
            "This learning worker no longer owns its job.",
        ));
    }
    Ok(())
}
pub(crate) async fn ready<'a>(
    state: &'a AppState,
    job: &ClaimedJob,
) -> Result<(Tx<'a>, LearningRun, ModelPolicyVersion)> {
    let mut tx = db::device_tx(&state.pool, job.actor_id, job.device_id).await?;
    db::require_writer(&mut tx, job.brain_id).await?;
    lease(&mut tx, job).await?;
    let run = row(&mut tx, job.brain_id, job.target_id).await?;
    if !matches!(run.state.as_str(), "queued" | "running") {
        return Err(model_policy::failure(
            "learning_not_current",
            "This learning run no longer accepts output.",
        ));
    }
    let current = model_policy::current(state, &mut tx, job.brain_id).await?;
    if current.change_id != run.policy_id {
        return Err(model_policy::failure(
            "model_policy_changed",
            "Model policy changed after this learning request. Start a new attempt.",
        ));
    }
    if current.policy.autonomous_memory {
        let latest: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sources WHERE brain_id=$1 AND current_version=$2)",
        )
        .bind(job.brain_id)
        .bind(run.source_version_id)
        .fetch_one(&mut *tx)
        .await?;
        if !latest {
            return Err(model_policy::failure(
                "learning_input_changed",
                "This source was superseded before autonomous publication.",
            ));
        }
        for id in &run.reconciliation_inputs {
            autonomous::recheck_target(state, &mut tx, &run, *id).await?;
        }
    }
    validate_scope(
        state,
        &mut tx,
        job.brain_id,
        job.actor_id,
        job.device_id,
        &mut input(&run),
        run.automatic,
    )
    .await?;
    Ok((tx, run, current))
}
async fn literal(
    tx: &mut Tx<'_>,
    brain: Uuid,
    p: &ModelPolicy,
    source: &gateway::SourceText,
    c: &ClaimContent,
    line: &str,
) -> Result<bool> {
    let Some(rule) = &p.acceptance else {
        return Ok(false);
    };
    if !rule.source_classes.contains(&source.class)
        || !rule.properties.contains(&c.predicate)
        || !model_policy::identifier(&c.subject, 128)
        || !model_policy::identifier(&c.predicate, 80)
        || line.trim() != format!("{}.{} = {}", c.subject, c.predicate, c.value)
    {
        return Ok(false);
    }
    if rule.collection_ids.is_empty() {
        return Ok(true);
    }
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_memberships WHERE brain_id=$1 AND source_id=$2 AND group_id=ANY($3))")
        .bind(brain).bind(source.source_id).bind(&rule.collection_ids).fetch_one(&mut **tx).await?)
}
async fn existing(
    tx: &mut Tx<'_>,
    brain: Uuid,
    content: &ClaimContent,
) -> Result<Option<ClaimRevision>> {
    let rows:Vec<SqlJson<ClaimRevision>>=sqlx::query_scalar("SELECT r.revision FROM claims c JOIN claim_revisions r ON r.id=c.current_revision WHERE c.brain_id=$1 AND r.subject_key=$2 AND r.predicate_key=$3 AND r.value_key=$4 AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active' AND recollect_memory_supported(r.brain_id,r.id) ORDER BY r.recorded_at LIMIT 500")
        .bind(brain).bind(memory_policy::assertion_key(&content.subject,true)).bind(memory_policy::assertion_key(&content.predicate,true)).bind(memory_policy::assertion_key(&content.value,false)).fetch_all(&mut **tx).await?;
    Ok(rows.into_iter().map(|r| r.0).find(|r| {
        r.content.selection == content.selection
            && r.content.kind == content.kind
            && r.content.manifest_revision_id == content.manifest_revision_id
            && r.content.validity == content.validity
            && r.content.supports == content.supports
    }))
}
async fn prepare_stage(state: &AppState, job: &ClaimedJob) -> Result<memory_support::Stage> {
    let (mut tx, mut run, policy) = ready(state, job).await?;
    if let Some(stage) = memory_support::load(&mut tx, run.brain_id, run.id).await? {
        tx.commit().await?;
        return Ok(stage);
    }
    if policy.policy.autonomous_memory && run.reconciliation_inputs.is_empty() {
        run.reconciliation_inputs =
            autonomous::targets(state, &mut tx, &run, policy.policy.max_input_bytes as usize)
                .await?;
        sqlx::query("UPDATE learning_runs SET reconciliation_inputs=$2 WHERE id=$1")
            .bind(run.id)
            .bind(&run.reconciliation_inputs)
            .execute(&mut *tx)
            .await?;
        for id in &run.reconciliation_inputs {
            sqlx::query("INSERT INTO learning_run_inputs(run_id,brain_id,revision_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
                .bind(run.id).bind(run.brain_id).bind(id).execute(&mut *tx).await?;
        }
    }
    sqlx::query("UPDATE learning_runs SET state='running' WHERE id=$1")
        .bind(run.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let response = gateway::invoke_for_policy(
        state,
        gateway::Context {
            brain: job.brain_id,
            actor: job.actor_id,
            device: job.device_id,
        },
        if policy.policy.autonomous_memory {
            autonomous::invocation(&run)
        } else {
            gateway::extraction(run.id, run.source_version_id)
        }
        .with_work_lease(job),
        run.policy_id,
    )
    .await?;
    let Some(gateway::Output::Json(value)) = response.output else {
        return Err(model_policy::failure(
            "model_result_not_retained",
            "This attempt has no retained model result. Start an explicit new attempt.",
        ));
    };
    let candidates: Candidates = serde_json::from_value(value).map_err(|_| {
        Error(
            StatusCode::BAD_GATEWAY,
            "provider_shape",
            "The extracted facts do not match the required fields.",
        )
    })?;
    if candidates.claims.len() > 8 || candidates.retirements.len() > 12 {
        return Err(Error(
            StatusCode::BAD_GATEWAY,
            "provider_shape",
            "The provider returned too many claims.",
        ));
    }
    let (mut tx, mut run, policy) = ready(state, job).await?;
    let source = gateway::source(
        state,
        &mut tx,
        job.brain_id,
        run.source_version_id,
        policy.policy.max_input_bytes as usize,
    )
    .await?;
    model_policy::permits(
        state,
        &policy.policy,
        "extraction",
        std::slice::from_ref(&source.class),
    )?;
    let lines: Vec<&str> = source.text.lines().collect();
    let mut contents = Vec::new();
    let mut mutations = std::collections::HashSet::new();
    for candidate in candidates.claims {
        if !policy.policy.autonomous_memory
            && (candidate.kind != "claim" || candidate.procedure.is_some())
        {
            return Err(Error(
                StatusCode::BAD_GATEWAY,
                "provider_shape",
                "Explicit extraction supports claim candidates only.",
            ));
        }
        if candidate.line_from < 1
            || candidate.line_to < candidate.line_from
            || candidate.line_to as usize > lines.len()
        {
            return Err(Error(
                StatusCode::BAD_GATEWAY,
                "provider_shape",
                "The provider cited source lines that do not exist.",
            ));
        }
        let replaces = candidate.replaces_revision;
        if let Some(id) = replaces {
            if !policy.policy.autonomous_memory || !mutations.insert(id) {
                return Err(Error(
                    StatusCode::BAD_GATEWAY,
                    "provider_shape",
                    "The model repeated an invalid revision target.",
                ));
            }
            let old = autonomous::target(&mut tx, &run, id).await?;
            if old.content.kind != candidate.kind {
                return Err(Error(
                    StatusCode::BAD_GATEWAY,
                    "provider_shape",
                    "The model changed an existing memory kind.",
                ));
            }
        }
        let mut c = content(&input(&run), candidate);
        memory_policy::validate(&mut c).map_err(|_| {
            Error(
                StatusCode::BAD_GATEWAY,
                "provider_shape",
                "The generated memory candidate violates the required fields or size limits.",
            )
        })?;
        publication::safe_payload(state, &json!(&c))?;
        memory::validate_evidence(state, &mut tx, job.brain_id, &c).await?;
        if !contents.iter().any(|(other, _)| other == &c) {
            contents.push((c, replaces));
        }
    }
    let mut retirements = Vec::new();
    for retire in candidates.retirements {
        // A replacement already places the preceding value in history. Some
        // providers redundantly retire that same revision; apply one mutation.
        if contents
            .iter()
            .any(|(_, id)| *id == Some(retire.revision_id))
        {
            continue;
        }
        if !policy.policy.autonomous_memory
            || !mutations.insert(retire.revision_id)
            || retire.line_from < 1
            || retire.line_to < retire.line_from
            || retire.line_to as usize > lines.len()
        {
            return Err(model_policy::failure(
                "provider_shape",
                "The generated retirement has an invalid target or evidence span.",
            ));
        }
        memory_policy::text(&retire.reason, 2000, true).map_err(|_| {
            model_policy::failure(
                "provider_shape",
                "The generated retirement reason is invalid.",
            )
        })?;
        let old = autonomous::target(&mut tx, &run, retire.revision_id).await?;
        retirements.push((old, retire));
    }
    // Store normalized candidates only after full canonical shape, span and target
    // validation. A restart after this commit never repeats extraction.
    let mut payload = memory_support::Payload {
        claims: contents
            .into_iter()
            .map(|(content, replaces_revision)| memory_support::StagedClaim {
                content,
                replaces_revision,
                reuses_revision: None,
            })
            .collect(),
        retirements: retirements
            .into_iter()
            .map(|(_, r)| memory_support::StagedRetirement {
                revision_id: r.revision_id,
                reason: r.reason,
                line_from: r.line_from,
                line_to: r.line_to,
            })
            .collect(),
        discovery: vec![],
    };
    if policy.policy.autonomous_memory {
        autonomous::discover_families(
            state,
            &mut tx,
            &mut run,
            &mut payload,
            policy.policy.max_input_bytes as usize,
        )
        .await?;
    }
    lease(&mut tx, job).await?;
    let stage = memory_support::store(state, &mut tx, &run, response.request.id, payload).await?;
    tx.commit().await?;
    Ok(stage)
}
async fn run_job(state: &AppState, job: &ClaimedJob) -> Result<()> {
    let stage = prepare_stage(state, job).await?;
    let (tx, run, _) = ready(state, job).await?;
    tx.commit().await?;
    let verdicts = memory_support::assess(state, &run, &stage, job).await?;
    // Never publish from the pre-call transaction or trust stale base revisions.
    let (mut tx, run, policy) = ready(state, job).await?;
    memory_support::load(&mut tx, run.brain_id, run.id)
        .await?
        .ok_or_else(crate::retention::unavailable)?;
    let input_deadline = memory_support::deadline(&mut tx, &run).await?;
    let source = gateway::source(
        state,
        &mut tx,
        job.brain_id,
        run.source_version_id,
        policy.policy.max_input_bytes as usize,
    )
    .await?;
    model_policy::permits(
        state,
        &policy.policy,
        "extraction",
        std::slice::from_ref(&source.class),
    )?;
    let lines: Vec<&str> = source.text.lines().collect();
    let request = model_policy::request(&mut tx, run.brain_id, stage.extraction_request).await?;
    if request.suppressed {
        return Err(crate::retention::unavailable());
    }
    let mut contents = Vec::new();
    let mut retirements = Vec::new();
    let mut withheld = 0;
    for (index, candidate) in stage.payload.claims.iter().enumerate() {
        if verdicts[index].disposition == memory_support::Disposition::Supported {
            if contents
                .iter()
                .any(|(other, _): &(ClaimContent, Option<Uuid>)| {
                    memory_rules::family(other, &candidate.content)
                        && memory_rules::same_value(other, &candidate.content)
                        && other.kind == candidate.content.kind
                        && other.selection == candidate.content.selection
                        && other.manifest_revision_id == candidate.content.manifest_revision_id
                        && other.validity == candidate.content.validity
                        && other.context_role == candidate.content.context_role
                        && other.procedure == candidate.content.procedure
                })
            {
                continue;
            }
            if let Some(id) = candidate.reuses_revision {
                autonomous::recheck_target(state, &mut tx, &run, id).await?;
                let r = autonomous::target(&mut tx, &run, id).await?;
                // Reuse has no mutation or retention effect. Count it below
                // only after the new assertion's independent assessment.
                contents.push((candidate.content.clone(), Some(r.id)));
                continue;
            }
            contents.push((candidate.content.clone(), candidate.replaces_revision));
        } else {
            withheld += 1;
        }
    }
    for (index, retirement) in stage.payload.retirements.iter().enumerate() {
        if verdicts[stage.payload.claims.len() + index].disposition
            == memory_support::Disposition::Supported
        {
            retirements.push((
                autonomous::target(&mut tx, &run, retirement.revision_id).await?,
                retirement.clone(),
            ));
        } else {
            withheld += 1;
        }
    }
    let mut count: i64 = sqlx::query_scalar("SELECT count(*) FROM claims WHERE brain_id=$1")
        .bind(job.brain_id)
        .fetch_one(&mut *tx)
        .await?;
    jobs::capacity(&mut tx, job.brain_id).await?;
    let actor_name: String = sqlx::query_scalar("SELECT username FROM accounts WHERE id=$1")
        .bind(job.actor_id)
        .fetch_one(&mut *tx)
        .await?;
    let mut proposed = 0;
    let mut accepted = 0;
    let mut blocked = withheld;
    let mut conflicting = 0;
    let mut reused = 0;
    let mut revised = 0;
    let mut retired = 0;
    let mut ids = Vec::new();
    for (old, retirement) in retirements {
        let mut r = old.clone();
        r.id = Uuid::new_v4();
        r.actor_id = job.actor_id;
        r.actor_name = actor_name.clone();
        r.device_id = job.device_id;
        r.operation_id = run.operation_id;
        r.recorded_at = sqlx::query_scalar(
            "SELECT greatest(clock_timestamp(),$1::timestamptz+interval '1 microsecond')",
        )
        .bind(old.recorded_at)
        .fetch_one(&mut *tx)
        .await?;
        r.origin = "model_reconciled".into();
        r.content.freshness = "superseded".into();
        r.content.rationale = retirement.reason;
        r.content.supports.retain(|s| s.id != run.source_version_id);
        r.content.supports.push(ClaimSupport {
            kind: "source_version".into(),
            id: run.source_version_id,
            line_from: Some(retirement.line_from),
            line_to: Some(retirement.line_to),
        });
        r.admission = "retired_by_policy".into();
        r.acceptance_policy = Some(format!("{}@{}", autonomous::POLICY, policy.change_id));
        r.derivation = Some(derivation(&run, &request, true));
        memory_policy::validate(&mut r.content)?;
        publication::safe_payload(state, &json!(&r.content))?;
        memory::validate_evidence(state, &mut tx, job.brain_id, &r.content).await?;
        memory::append(&mut tx, &r).await?;
        autonomous::basis(&mut tx, &r, &run.reconciliation_inputs).await?;
        db::audit(
            &mut tx,
            job.actor_id,
            job.brain_id,
            "claim.retire",
            r.claim_id,
            &r.admission,
        )
        .await?;
        ids.push(r.claim_id);
        retired += 1;
    }
    for (c, replaces) in &contents {
        if let Some(candidate) = stage
            .payload
            .claims
            .iter()
            .find(|a| a.content == *c && a.reuses_revision.is_some())
        {
            let r = autonomous::target(&mut tx, &run, candidate.reuses_revision.unwrap()).await?;
            ids.push(r.claim_id);
            reused += 1;
            continue;
        }
        let mut previous = match replaces {
            Some(id) => Some(autonomous::target(&mut tx, &run, *id).await?),
            None => None,
        };
        if let Some(old) = existing(&mut tx, job.brain_id, c).await? {
            if !policy.policy.autonomous_memory
                || old.review != "proposed"
                || !autonomous::maintained(&old)
            {
                ids.push(old.claim_id);
                reused += 1;
                continue;
            }
            previous = Some(old);
        }
        memory_rules::check_family(
            &mut tx,
            job.brain_id,
            c,
            previous.as_ref().map(|r| r.claim_id),
        )
        .await?;
        let recorded_at: DateTime<Utc> = sqlx::query_scalar("SELECT greatest(clock_timestamp(),coalesce($1::timestamptz+interval '1 microsecond',clock_timestamp()))")
            .bind(previous.as_ref().map(|r|r.recorded_at))
            .fetch_one(&mut *tx)
            .await?;
        let mut r = ClaimRevision {
            id: Uuid::new_v4(),
            claim_id: previous
                .as_ref()
                .map(|r| r.claim_id)
                .unwrap_or_else(Uuid::new_v4),
            brain_id: job.brain_id,
            actor_id: job.actor_id,
            actor_name: actor_name.clone(),
            device_id: job.device_id,
            operation_id: run.operation_id,
            origin: if previous.is_some() {
                "model_reconciled"
            } else {
                "model_extracted"
            }
            .into(),
            recorded_at,
            content: c.clone(),
            review: "proposed".into(),
            reviewer_id: None,
            acceptance_policy: None,
            lifecycle: "active".into(),
            review_decision_id: None,
            admission: "proposed".into(),
            derivation: Some(derivation(&run, &request, policy.policy.autonomous_memory)),
        };
        let rules = memory_rules::matching(&mut tx, &r, false).await?;
        let conflicts = memory_rules::conflicts(&mut tx, &r).await?;
        let batch_conflict = contents.iter().any(|(other, _)| {
            memory_rules::family(c, other)
                && !memory_rules::same_value(c, other)
                && memory_rules::overlaps(c, other)
        });
        let support = &c.supports[0];
        let is_literal = support.line_from == support.line_to
            && literal(
                &mut tx,
                job.brain_id,
                &policy.policy,
                &source,
                c,
                lines[support.line_from.unwrap() as usize - 1],
            )
            .await?;
        if !rules.is_empty() {
            r.admission = "blocked_by_rule".into();
            blocked += 1;
            proposed += 1;
        } else if batch_conflict || !conflicts.is_empty() {
            r.admission = if policy.policy.autonomous_memory {
                "uncertain_evidence"
            } else {
                "needs_review"
            }
            .into();
            conflicting += 1;
            proposed += 1;
        } else if is_literal || policy.policy.autonomous_memory {
            r.review = "accepted".into();
            r.admission = "accepted_by_policy".into();
            r.acceptance_policy = Some(format!(
                "{}@{}",
                if policy.policy.autonomous_memory {
                    autonomous::POLICY
                } else {
                    &policy.policy.acceptance.as_ref().unwrap().name
                },
                policy.change_id
            ));
            accepted += 1;
        } else {
            proposed += 1;
        }
        // A blocked or unresolved replacement cannot silently displace the
        // previously supported alternative. Keep both identities inspectable.
        if previous.is_some()
            && matches!(
                r.admission.as_str(),
                "blocked_by_rule" | "uncertain_evidence"
            )
        {
            r.claim_id = Uuid::new_v4();
            previous = None;
            memory_rules::check_family(&mut tx, job.brain_id, c, None).await?;
        }
        if previous.is_none() {
            // Reuse, revision and retirement allocate no identity. Charge only
            // after disposition: an unresolved replacement may become a new claim.
            if count >= 5000 {
                return Err(memory_rules::capacity());
            }
            sqlx::query("INSERT INTO claims(id,brain_id,created_by) VALUES($1,$2,$3)")
                .bind(r.claim_id)
                .bind(job.brain_id)
                .bind(job.actor_id)
                .execute(&mut *tx)
                .await?;
            count += 1;
        } else {
            revised += 1;
        }
        r.content = crate::support_excerpts::retain(state, &mut tx, &run, c).await?;
        memory::validate_evidence(state, &mut tx, job.brain_id, &r.content).await?;
        memory::append(&mut tx, &r).await?;
        autonomous::basis(&mut tx, &r, &run.reconciliation_inputs).await?;
        sqlx::query("INSERT INTO claim_model_derivations(revision_id,brain_id,run_id,request_id) VALUES($1,$2,$3,$4)")
            .bind(r.id).bind(job.brain_id).bind(run.id).bind(request.id).execute(&mut *tx).await?;
        crate::support_excerpts::equivalent(state, &mut tx, &run, c, &r.content).await?;
        crate::memory_support_audit::record_learning(state, &mut tx, &r, &run, c).await?;
        db::audit(
            &mut tx,
            job.actor_id,
            job.brain_id,
            "claim.learn",
            r.claim_id,
            &r.admission,
        )
        .await?;
        ids.push(r.claim_id);
    }
    // The external call and canonical publication have separate durable outcomes.
    lease(&mut tx, job).await?;
    sqlx::query("UPDATE learning_runs SET state='succeeded',request_id=$2,claim_ids=$3,proposed=$4,accepted=$5,blocked=$6,conflicting=$7,reused=$8,revised=$9,retired=$10,finished_at=clock_timestamp() WHERE id=$1")
        .bind(run.id).bind(request.id).bind(&ids).bind(proposed).bind(accepted).bind(blocked).bind(conflicting).bind(reused).bind(revised).bind(retired).execute(&mut *tx).await?;
    let audit = db::audit(
        &mut tx,
        job.actor_id,
        job.brain_id,
        "learning.publish",
        run.id,
        "succeeded",
    )
    .await?;
    jobs::enqueue(&mut tx, job.actor_id, job.brain_id, audit).await?;
    sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,updated_at=now() WHERE id=$1 AND lease_token=$2")
        .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
    let within_retention: bool =
        sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
            .bind(input_deadline)
            .fetch_one(&mut *tx)
            .await?;
    if !within_retention {
        return Err(model_policy::failure(
            "learning_input_expired",
            "Required learning evidence expired during publication; its output was discarded.",
        ));
    }
    tx.commit().await?;
    Ok(())
}
fn derivation(run: &LearningRun, request: &ModelRequest, automatic: bool) -> ModelDerivation {
    ModelDerivation {
        run_id: run.id,
        request_id: request.id,
        policy_id: run.policy_id,
        provider: "openai".into(),
        requested_model: request.model.clone(),
        returned_model: request.returned_model.clone().unwrap_or_default(),
        prompt_label: if automatic {
            autonomous::PROMPT
        } else {
            gateway::EXTRACT_PROMPT
        }
        .into(),
        schema_label: if automatic {
            autonomous::PROMPT
        } else {
            gateway::EXTRACT_SCHEMA
        }
        .into(),
    }
}
pub async fn execute(state: &AppState, job: &ClaimedJob) -> std::result::Result<(), Failure> {
    let result = worker::with_lease(&state.pool, job, run_job(state, job)).await?;
    if let Err(error) = result {
        tracing::warn!(job_id=%job.id,code=error.1,reason=error.2,"Learning did not publish");
        if error.1 == "database_unavailable" {
            // Native bounded worker recovery preserves the same generation.
            // Saved stages/verdicts resume locally; a charged response lost
            // before its commit remains unavailable through gateway identity.
            return Err(Failure::Database);
        }
        let reason = if matches!(
            error.0,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        ) {
            "permission_or_policy_denied"
        } else {
            error.1
        };
        if matches!(reason, "model_budget_exhausted" | "model_concurrency_full") {
            sqlx::query("SELECT recollect_defer_learning_job($1,$2,$3)")
                .bind(job.id)
                .bind(job.lease_token)
                .bind(reason)
                .execute(&state.pool)
                .await?;
            return Ok(());
        }
        sqlx::query("SELECT recollect_fail_learning_job($1,$2,$3)")
            .bind(job.id)
            .bind(job.lease_token)
            .bind(reason)
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}
