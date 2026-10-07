//! Standing Brain policy drives bounded scheduling; model output never acquires
//! human-review or tool-execution authority.
use crate::{
    AppState, db, error::Result, learning, memory, memory_evidence::Tx, model_gateway as gateway,
    model_policy,
};
use chrono::Utc;
use recollect_protocol::*;
use sqlx::types::Json;
use uuid::Uuid;

pub const POLICY: &str = "autonomous-evidence";
pub const PROMPT: &str = "source-reconciliation-1";

pub fn maintained(r: &ClaimRevision) -> bool {
    matches!(
        r.origin.as_str(),
        "model_extracted" | "model_reconciled" | "model_synthesized"
    ) && r.reviewer_id.is_none()
        && r.review_decision_id.is_none()
        && r.lifecycle == "active"
        && r.review != "rejected"
        && r.content.freshness != "superseded"
}

pub async fn targets(
    state: &AppState,
    tx: &mut Tx<'_>,
    run: &LearningRun,
    limit: usize,
) -> Result<Vec<Uuid>> {
    let source = gateway::source(state, tx, run.brain_id, run.source_version_id, limit).await?;
    let mut remaining =
        limit.saturating_sub(source.text.len() + source.provenance.to_string().len());
    let candidates: Vec<Json<ClaimRevision>> = sqlx::query_scalar(
        "WITH lineage AS MATERIALIZED (
           SELECT old.id FROM source_versions new JOIN source_versions old
             ON old.source_id=new.source_id AND old.brain_id=new.brain_id
             WHERE new.brain_id=$1 AND new.id=$2
           UNION
           SELECT old.source_version_id FROM capture_events new JOIN capture_events old
             ON old.brain_id=new.brain_id AND old.binding_id=new.binding_id
               AND old.metadata->>'host_session_id'=new.metadata->>'host_session_id'
               AND old.metadata->>'agent_id' IS NOT DISTINCT FROM new.metadata->>'agent_id'
             WHERE new.brain_id=$1 AND new.source_version_id=$2
               AND old.state='accepted' AND new.state='accepted'
               AND old.source_version_id IS NOT NULL
         ), supported AS MATERIALIZED (
           SELECT DISTINCT cs.revision_id FROM claim_supports cs JOIN lineage l ON l.id=cs.source_version_id
           WHERE cs.brain_id=$1
         )
         SELECT r.revision FROM supported s JOIN claim_revisions r ON r.id=s.revision_id
         JOIN claims c ON c.current_revision=r.id AND c.brain_id=r.brain_id
         WHERE c.brain_id=$1 AND r.revision->'content'->'selection'=$3
         AND r.revision->>'origin' IN ('model_extracted','model_reconciled','model_synthesized')
         AND r.revision->>'reviewer_id' IS NULL AND r.revision->>'review_decision_id' IS NULL
         AND r.revision->>'lifecycle'='active' AND r.revision->>'review'<>'rejected'
         AND r.revision#>>'{content,freshness}'<>'superseded'
         AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
         AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=r.brain_id AND f.revision_id=r.id)
         ORDER BY r.recorded_at DESC,r.id LIMIT 500")
        .bind(run.brain_id).bind(run.source_version_id).bind(Json(&run.selection))
        .fetch_all(&mut **tx).await?;
    let mut ids = Vec::new();
    for Json(r) in candidates {
        if !maintained(&r)
            || !matches!(r.content.kind.as_str(), "claim" | "decision" | "procedure")
            || r.content.manifest_revision_id != run.manifest_revision_id
        {
            continue;
        }
        let view = memory::view(state, tx, r, Utc::now(), None).await?;
        if view.eligibility.investigation
            && view.eligibility.rule_ids.is_empty()
            && view.evidence.iter().all(|e| e.availability == "retained")
        {
            // The gateway transmits the whole canonical view plus its input
            // provenance. Omit an optional revision rather than truncate its
            // evidence or let reconciliation overflow the source's allowance.
            let bytes = serde_json::to_string(&view)
                .map_err(|_| crate::error::Error::invalid("Invalid reconciliation input."))?
                .len()
                + serde_json::json!({"kind":"claim_revision","id":view.revision.id})
                    .to_string()
                    .len();
            if bytes <= remaining {
                remaining -= bytes;
                ids.push(view.revision.id);
            }
        }
        if ids.len() == 12 {
            break;
        }
    }
    Ok(ids)
}

pub async fn target(tx: &mut Tx<'_>, run: &LearningRun, id: Uuid) -> Result<ClaimRevision> {
    if !run.reconciliation_inputs.contains(&id) {
        return Err(crate::error::Error::invalid(
            "The model selected an unoffered revision.",
        ));
    }
    let claim: Uuid =
        sqlx::query_scalar("SELECT claim_id FROM claim_revisions WHERE brain_id=$1 AND id=$2")
            .bind(run.brain_id)
            .bind(id)
            .fetch_one(&mut **tx)
            .await?;
    let r = memory::current(tx, run.brain_id, claim).await?;
    if r.id != id
        || !maintained(&r)
        || r.content.selection != run.selection
        || r.content.manifest_revision_id != run.manifest_revision_id
    {
        return Err(model_policy::failure(
            "reconciliation_input_changed",
            "Reconciliation input changed; its replacement was discarded.",
        ));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM claim_revisions WHERE claim_id=$1")
        .bind(claim)
        .fetch_one(&mut **tx)
        .await?;
    if count >= 1000 {
        return Err(crate::memory_rules::capacity());
    }
    Ok(r)
}

/// Exact families become known only after typed extraction. A sole match is a
/// hypothesis for the independent support check, never a newest-wins decision.
pub(crate) async fn discover_families(
    state: &AppState,
    tx: &mut Tx<'_>,
    run: &mut LearningRun,
    payload: &mut crate::memory_support::Payload,
    limit: usize,
) -> Result<()> {
    let source = gateway::source(state, tx, run.brain_id, run.source_version_id, limit).await?;
    let mut used = source.text.len()
        + source.provenance.to_string().len()
        + serde_json::to_vec(payload)
            .map_err(|_| crate::error::Error::invalid("Invalid stage."))?
            .len();
    for id in &run.reconciliation_inputs {
        let r = target(tx, run, *id).await?;
        used += serde_json::to_vec(&memory::view(state, tx, r, Utc::now(), None).await?)
            .map_err(|_| crate::error::Error::invalid("Invalid family input."))?
            .len();
    }
    let mut claimed = payload
        .claims
        .iter()
        .filter_map(|c| c.replaces_revision)
        .chain(payload.retirements.iter().map(|r| r.revision_id))
        .collect::<std::collections::HashSet<_>>();
    for (index, c) in payload.claims.iter_mut().enumerate() {
        if c.replaces_revision.is_some() {
            continue;
        }
        let rows: Vec<Json<ClaimRevision>> = sqlx::query_scalar(
            "SELECT r.revision FROM claims c JOIN claim_revisions r ON r.id=c.current_revision AND r.brain_id=c.brain_id
             WHERE c.brain_id=$1 AND r.subject_key=$2 AND r.predicate_key=$3
             AND recollect_memory_supported(r.brain_id,r.id)
             AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
             AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=r.brain_id AND f.revision_id=r.id)
             ORDER BY r.id LIMIT 500")
            .bind(run.brain_id).bind(crate::memory_policy::assertion_key(&c.content.subject,true))
            .bind(crate::memory_policy::assertion_key(&c.content.predicate,true)).fetch_all(&mut **tx).await?;
        let mut matches = Vec::new();
        for Json(r) in rows {
            if !maintained(&r)
                || r.content.kind != c.content.kind
                || r.content.selection != c.content.selection
                || r.content.manifest_revision_id != c.content.manifest_revision_id
                || r.content.validity != c.content.validity
            {
                continue;
            }
            let view = memory::view(state, tx, r.clone(), Utc::now(), None).await?;
            if view.eligibility.investigation
                && view.eligibility.rule_ids.is_empty()
                && view.evidence.iter().all(|e| e.availability == "retained")
            {
                matches.push((
                    r,
                    serde_json::to_vec(&view)
                        .map_err(|_| crate::error::Error::invalid("Invalid family input."))?
                        .len(),
                ));
            }
        }
        let count = matches.len();
        let mut disposition = if count == 0 { "none" } else { "ambiguous" };
        if count == 1 {
            let (r, bytes) = matches.pop().unwrap();
            let already = run.reconciliation_inputs.contains(&r.id);
            if claimed.contains(&r.id) {
                disposition = "already_mutated";
            } else if !already
                && (run.reconciliation_inputs.len() >= 12 || used + bytes + 128 > limit)
            {
                disposition = "input_budget";
            } else {
                if !already {
                    run.reconciliation_inputs.push(r.id);
                    used += bytes + 128;
                }
                if crate::memory_rules::same_value(&c.content, &r.content)
                    && c.content.procedure == r.content.procedure
                {
                    c.reuses_revision = Some(r.id);
                    disposition = "reuse_hypothesis";
                } else {
                    c.replaces_revision = Some(r.id);
                    claimed.insert(r.id);
                    disposition = "replacement_hypothesis";
                }
            }
        }
        payload
            .discovery
            .push(crate::memory_support::FamilyDiscovery {
                index,
                disposition: disposition.into(),
                eligible_matches: count,
            });
    }
    sqlx::query("UPDATE learning_runs SET reconciliation_inputs=$2 WHERE id=$1 AND brain_id=$3")
        .bind(run.id)
        .bind(&run.reconciliation_inputs)
        .bind(run.brain_id)
        .execute(&mut **tx)
        .await?;
    for id in &run.reconciliation_inputs {
        sqlx::query("INSERT INTO learning_run_inputs(run_id,brain_id,revision_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(run.id).bind(run.brain_id).bind(id).execute(&mut **tx).await?;
    }
    Ok(())
}

pub async fn recheck_target(
    state: &AppState,
    tx: &mut Tx<'_>,
    run: &LearningRun,
    id: Uuid,
) -> Result<()> {
    let revision = target(tx, run, id).await?;
    let fenced: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM model_claim_fences WHERE brain_id=$1 AND revision_id=$2)",
    )
    .bind(run.brain_id)
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    let view = memory::view(state, tx, revision, Utc::now(), None).await?;
    if fenced
        || !view.eligibility.investigation
        || !view.eligibility.rule_ids.is_empty()
        || view.evidence.iter().any(|e| e.availability != "retained")
    {
        return Err(model_policy::failure(
            "reconciliation_input_changed",
            "Reconciliation support or policy changed; its replacement was discarded.",
        ));
    }
    Ok(())
}

/// Current model inputs and their required evidence can expire while a writer
/// holds the Brain lock. Historical reasoning edges are erasure dependencies,
/// not a permanent TTL dependency on every superseded source.
pub async fn input_deadline(
    tx: &mut Tx<'_>,
    run: &LearningRun,
) -> Result<Option<chrono::DateTime<Utc>>> {
    Ok(sqlx::query_scalar("WITH deadlines AS (
      SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) AS deadline
        FROM source_versions v WHERE v.brain_id=$1 AND v.id=$2
      UNION ALL SELECT recollect_retention_deadline(r.brain_id,'claim',r.recorded_at)
        FROM claim_revisions r WHERE r.brain_id=$1 AND r.id=ANY($3)
      UNION ALL SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at)
        FROM claim_supports s JOIN source_versions v ON v.id=s.source_version_id AND v.brain_id=s.brain_id
        WHERE s.brain_id=$1 AND s.revision_id=ANY($3)
      UNION ALL SELECT recollect_retention_deadline(p.brain_id,'repository',p.created_at)
        FROM claim_supports s JOIN repository_facts f ON f.id=s.fact_id AND f.brain_id=s.brain_id
        JOIN repository_snapshots p ON p.id=f.snapshot_id AND p.brain_id=f.brain_id
        WHERE s.brain_id=$1 AND s.revision_id=ANY($3)
      ) SELECT min(deadline) FROM deadlines")
        .bind(run.brain_id).bind(run.source_version_id).bind(&run.reconciliation_inputs)
        .fetch_one(&mut **tx).await?)
}

pub fn invocation(run: &LearningRun) -> gateway::Invocation {
    use serde_json::json;
    let mut inv = gateway::extraction(run.id, run.source_version_id);
    let mut schema = gateway::extraction_schema();
    let item = &mut schema["properties"]["claims"]["items"];
    item["properties"]["kind"] = json!({"type":"string","enum":["claim","decision","procedure"]});
    item["properties"]["procedure"] = json!({"type":["object","null"],"properties":{
        "conditions":{"type":"string","maxLength":2000},"steps":{"type":"array","minItems":1,"maxItems":20,"items":{"type":"string","minLength":1,"maxLength":1000}},
        "expected_outcome":{"type":"string","minLength":1,"maxLength":2000}},"required":["conditions","steps","expected_outcome"],"additionalProperties":false});
    item["required"]
        .as_array_mut()
        .unwrap()
        .extend([json!("kind"), json!("procedure")]);
    item["properties"]["replaces_revision"] = json!({"type":["string","null"]});
    let mut allowed = vec![serde_json::Value::Null];
    allowed.extend(run.reconciliation_inputs.iter().map(|id| json!(id)));
    item["properties"]["replaces_revision"]["enum"] = json!(allowed);
    item["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("replaces_revision"));
    schema["properties"]["retirements"] = json!({"type":"array","maxItems":12,"items":{
        "type":"object","properties":{"revision_id":{"type":"string"},"reason":{"type":"string"},
        "line_from":{"type":"integer"},"line_to":{"type":"integer"}},
        "required":["revision_id","reason","line_from","line_to"],"additionalProperties":false}});
    schema["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("retirements"));
    if run.reconciliation_inputs.is_empty() {
        schema["properties"]["retirements"]["maxItems"] = json!(0);
    } else {
        schema["properties"]["retirements"]["items"]["properties"]["revision_id"]["enum"] =
            json!(run.reconciliation_inputs);
    }
    inv.inputs.extend(
        run.reconciliation_inputs
            .iter()
            .map(|id| gateway::InputRef {
                kind: "claim_revision".into(),
                id: *id,
            }),
    );
    inv.instructions.push_str(" Input 0 is the source to learn now; subsequent inputs are existing machine-maintained memories from the same source lineage or authenticated captured session and exact scope. Being offered is only permission to consider a candidate; it is not equivalence, corroboration or evidence that the newest statement wins. Reconcile meaning against the cited source and canonical provenance, using the exact revision.id as replaces_revision only when new evidence explicitly supports an updated assertion. Preserve subject and predicate when their meaning is unchanged. Use null for a new assertion. Do not revise unrelated facts, assume a newer date makes something true, or treat historical quotations, later mentions, assistant proposals or echoes as current changes. Retire an existing assertion only when exact lines explicitly establish it is obsolete or unsupported; return revision_id, a concise evidence-based reason, and line_from/line_to in retirements. Omission, lack of mention, model confidence and age alone are not grounds for retirement. Leave unresolved alternatives qualified by their wording instead of inventing a winner. Return an empty retirements array when nothing can be retired. One existing revision may be replaced or retired at most once.");
    if run.automatic_attempt > 0 {
        inv.instructions.push_str(" The preceding completed response was rejected. Return at most three concise fully valid candidates; keep all string bounds, use procedure=null for other kinds and exact positive source line numbers. Return an empty claims array rather than fabricate support.");
    }
    inv.prompt_label = PROMPT.into();
    inv.instructions.push_str(" Select kind claim for ordinary assertions, decision only for a decision explicitly recorded in the source, and procedure for a stated reusable sequence of steps. A procedure must include conditions, ordered steps and expected_outcome; other kinds use procedure=null. Never supply test observations or execution authority. Keep an existing target's kind when revising it. Do not split an assertion and its change description into separate facts. Replacing a revision already preserves its preceding value as history; do not also retire it.");
    inv.schema_label = PROMPT.into();
    inv.format = gateway::Format::Json {
        name: "source_reconciliation".into(),
        schema,
    };
    inv
}

pub async fn basis(tx: &mut Tx<'_>, r: &ClaimRevision, inputs: &[Uuid]) -> Result<()> {
    for id in inputs {
        sqlx::query("INSERT INTO claim_contributions(revision_id,brain_id,input_revision_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(r.id).bind(r.brain_id).bind(id).execute(&mut **tx).await?;
    }
    Ok(())
}

async fn brain(state: &AppState, brain: Uuid, actor: Uuid) -> Result<usize> {
    let mut tx = db::actor_tx(&state.pool, actor).await?;
    db::require_writer(&mut tx, brain).await?;
    let p = model_policy::current(state, &mut tx, brain).await?;
    if p.created_by != Some(actor)
        || !p.policy.enabled
        || !p.policy.autonomous_memory
        || !model_policy::matches_installation(state, &p.policy)
    {
        return Ok(0);
    }
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT v.id FROM sources s JOIN source_versions v ON v.id=s.current_version
         WHERE s.brain_id=$1 AND v.processing='ready' AND v.artifact_id IS NOT NULL
         AND v.retention_class=ANY($2) AND v.byte_length<=$3
         AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
         AND NOT EXISTS(SELECT 1 FROM automatic_support_excerpts a WHERE a.brain_id=v.brain_id AND a.version_id=v.id)
         AND NOT EXISTS(SELECT 1 FROM model_input_fences f WHERE f.brain_id=v.brain_id AND f.source_version_id=v.id)
         AND NOT EXISTS(SELECT 1 FROM learning_runs l WHERE l.brain_id=v.brain_id AND l.source_version_id=v.id AND l.policy_id=$4 AND l.automatic)
         ORDER BY v.created_at,v.id LIMIT 10")
        .bind(brain).bind(&p.policy.content_classes).bind(p.policy.max_input_bytes).bind(p.change_id)
        .fetch_all(&mut *tx).await?;
    let mut queued = crate::memory_support_audit::enqueue(&mut tx, brain, actor, &p, 4).await?;
    for id in ids {
        crate::jobs::capacity(&mut tx, brain).await?;
        let selection = crate::capture::source_selection(&mut tx, brain, id).await?;
        learning::enqueue(
            &mut tx,
            brain,
            actor,
            None,
            &LearningInput {
                source_version_id: id,
                selection,
                manifest_revision_id: None,
                operation_id: None,
            },
            p.change_id,
            learning::QueueMode::Automatic,
        )
        .await?;
        queued += 1;
    }
    queued +=
        crate::session_digests::schedule(state, &mut tx, brain, actor, &p, (20 - queued).min(2))
            .await?;
    queued +=
        crate::handovers::refresh(state, &mut tx, brain, actor, &p, (20 - queued).min(5)).await?;
    queued += retries(&mut tx, brain, actor, &p, 20 - queued).await?;
    tx.commit().await?;
    Ok(queued)
}

async fn retries(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    p: &ModelPolicyVersion,
    limit: usize,
) -> Result<usize> {
    if limit == 0 {
        return Ok(0);
    }
    let mut queued = crate::handover_support::recover(tx, brain, p.change_id, limit).await?;
    if queued == limit {
        return Ok(queued);
    }
    let runs:Vec<Json<LearningRun>>=sqlx::query_scalar(
        "SELECT to_jsonb(r) FROM learning_runs r JOIN sources s ON s.current_version=r.source_version_id
         WHERE r.brain_id=$1 AND r.policy_id=$2 AND r.automatic AND r.state='failed'
         AND r.error_code IN ('provider_rate_limited','provider_unavailable','provider_incomplete','provider_shape','model_budget_exhausted') AND r.automatic_attempt<2
         AND CASE WHEN r.error_code='model_budget_exhausted'
           THEN ((date_trunc('day',r.finished_at AT TIME ZONE 'UTC')+interval '1 day') AT TIME ZONE 'UTC')<=clock_timestamp()
           ELSE r.finished_at+make_interval(mins=>CASE r.automatic_attempt WHEN 0 THEN 5 ELSE 30 END)<=clock_timestamp() END
         AND NOT EXISTS(SELECT 1 FROM learning_runs child WHERE child.retry_of=r.id)
         AND NOT EXISTS(SELECT 1 FROM learning_support_stages stage WHERE stage.run_id=r.id AND stage.brain_id=r.brain_id AND (stage.verdicts IS NOT NULL OR stage.privacy_state<>'active' OR recollect_learning_support_deadline(stage.brain_id,stage.run_id)<=clock_timestamp()))
         ORDER BY r.finished_at,r.id LIMIT $3")
        .bind(brain).bind(p.change_id).bind((limit-queued) as i64).fetch_all(&mut **tx).await?;
    for Json(old) in runs {
        let run = learning::enqueue(
            tx,
            brain,
            actor,
            None,
            &LearningInput {
                source_version_id: old.source_version_id,
                selection: old.selection.clone(),
                manifest_revision_id: old.manifest_revision_id,
                operation_id: None,
            },
            p.change_id,
            learning::QueueMode::Retry {
                parent: old.id,
                attempt: old.automatic_attempt + 1,
            },
        )
        .await?;
        crate::memory_support::inherit_assessment_retry(tx, &old, &run).await?;
        queued += 1;
    }
    if queued == limit {
        return Ok(queued);
    }
    let runs:Vec<Json<HandoverRun>>=sqlx::query_scalar(
        "SELECT to_jsonb(r) FROM handover_runs r WHERE r.brain_id=$1 AND r.policy_id=$2 AND r.state='failed'
         AND r.error_code IN ('provider_rate_limited','provider_unavailable','provider_incomplete','provider_shape','model_response_invalid') AND r.automatic_attempt<2
         AND r.finished_at+make_interval(mins=>CASE r.automatic_attempt WHEN 0 THEN 5 ELSE 30 END)<=clock_timestamp()
         AND NOT EXISTS(SELECT 1 FROM handover_runs child WHERE child.retry_of=r.id)
         AND NOT EXISTS(SELECT 1 FROM handover_support_stages stage WHERE stage.run_id=r.id AND stage.brain_id=r.brain_id AND (stage.verdict IS NOT NULL OR stage.privacy_state<>'active' OR recollect_handover_support_deadline(stage.brain_id,stage.run_id)<=clock_timestamp()))
         AND EXISTS(SELECT 1 FROM model_requests m LEFT JOIN handover_support_stages stage ON stage.run_id=r.id AND stage.brain_id=r.brain_id WHERE m.brain_id=r.brain_id AND m.operation_id=coalesce(stage.assessment_operation_id,r.id) AND m.state IN ('succeeded','failed') AND NOT m.suppressed)
         ORDER BY r.finished_at,r.id LIMIT $3")
        .bind(brain).bind(p.change_id).bind((limit-queued) as i64).fetch_all(&mut **tx).await?;
    for Json(old) in runs {
        let id = Uuid::new_v4();
        let audit = db::audit(tx, actor, brain, "handover.queue", id, "automatic_retry").await?;
        let job =
            crate::jobs::enqueue_work(tx, actor, brain, audit, id, "handover.generate", "model")
                .await?;
        sqlx::query("INSERT INTO handover_runs(id,brain_id,title,contributions,selection,policy_id,actor_id,job_id,state,automatic,claim_id,base_revision_id,retry_of,automatic_attempt) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'queued',true,$9,$10,$11,$12)")
            .bind(id).bind(brain).bind(&old.title).bind(&old.contributions).bind(Json(&old.selection))
            .bind(p.change_id).bind(actor).bind(job).bind(old.claim_id).bind(old.base_revision_id)
            .bind(old.id).bind(old.automatic_attempt+1).execute(&mut **tx).await?;
        for input in &old.contributions {
            sqlx::query(
                "INSERT INTO handover_run_inputs(run_id,brain_id,revision_id) VALUES($1,$2,$3)",
            )
            .bind(id)
            .bind(brain)
            .bind(input)
            .execute(&mut **tx)
            .await?;
        }
        crate::handover_support::inherit_retry(tx, &old, id).await?;
        queued += 1;
    }
    Ok(queued)
}

pub async fn run_once(state: &AppState) -> Result<usize> {
    let brains: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT brain_id,actor_id FROM recollect_autonomous_brains()")
            .fetch_all(&state.pool)
            .await?;
    let mut count = 0;
    for (id, actor) in brains {
        match brain(state, id, actor).await {
            Ok(n) => count += n,
            Err(e) => tracing::warn!(brain_id=%id,code=e.1,"Autonomous maintenance is waiting"),
        }
    }
    Ok(count)
}
