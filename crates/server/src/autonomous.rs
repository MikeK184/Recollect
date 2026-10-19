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

pub mod maintenance;
pub mod support_discovery;

pub const POLICY: &str = "autonomous-evidence";
pub const PROMPT: &str = "source-reconciliation-4";

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
           WHERE cs.brain_id=$1 AND NOT recollect_source_model_fenced(cs.brain_id,cs.source_version_id)
           UNION
           SELECT cs.revision_id FROM claim_supports cs
           JOIN automatic_support_excerpts a ON a.brain_id=cs.brain_id AND a.version_id=cs.source_version_id
           JOIN source_excerpts e ON e.brain_id=a.brain_id AND e.version_id=a.version_id
             AND e.parent_version_id=a.parent_version_id AND e.first_line=a.first_line AND e.last_line=a.last_line
           JOIN source_versions parent ON parent.brain_id=e.brain_id AND parent.id=e.parent_version_id AND parent.source_id=e.parent_source_id
           JOIN source_versions child ON child.brain_id=a.brain_id AND child.id=a.version_id
           JOIN lineage l ON l.id=a.parent_version_id
           WHERE cs.brain_id=$1 AND a.privacy_state='active' AND a.selection=$3
             AND a.manifest_revision_id IS NOT DISTINCT FROM $4 AND child.retention_class='support_excerpt'
             AND recollect_content_state(child.brain_id,child.retention_class,child.privacy_state,child.created_at)='active'
             AND NOT recollect_source_model_fenced(a.brain_id,a.version_id)
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
        .bind(run.brain_id).bind(run.source_version_id).bind(Json(&run.selection)).bind(run.manifest_revision_id)
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

/// Experimental evaluation must not re-admit an old reconciliation output after
/// its offered memories leave the exact original selector. The persisted run
/// supplies canonical scope/manifest/lineage, not a caller-built memory view.
/// Direct extraction-only controls may legitimately leave this run cancelled
/// or queued; they do not claim that learning/support publication succeeded.
pub(crate) async fn shadow_offered_current(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
    operation: Uuid,
    input0_version: Uuid,
    offered: &[Uuid],
) -> Result<bool> {
    if offered.is_empty()
        || offered.len() > 12
        || offered
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != offered.len()
    {
        return Ok(false);
    }
    let run: Option<Json<LearningRun>> = sqlx::query_scalar(
        "SELECT to_jsonb(r) FROM learning_runs r WHERE r.brain_id=$1 AND r.id=$2 AND r.actor_id=$3 AND r.source_version_id=$4",
    )
    .bind(brain)
    .bind(operation)
    .bind(actor)
    .bind(input0_version)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(Json(run)) = run else {
        return Ok(false);
    };
    if run.device_id != device
        && !(run.automatic
            && run.device_id.is_none()
            && device.is_some()
            && shadow_source_device(state, &run, device.unwrap()).await?)
    {
        return Ok(false);
    }
    let policy = model_policy::current(state, tx, brain).await?;
    if policy.change_id != run.policy_id {
        return Ok(false);
    }
    let current = targets(state, tx, &run, policy.policy.max_input_bytes as usize).await?;
    Ok(offered.iter().all(|id| current.contains(id)))
}

/// Standing-policy scheduling legitimately has no device. A tooling-only
/// extraction may retain the actual input0 device, but only its immutable
/// source origin can establish that attribution. Read workspace rows as that
/// already validated producer; never alter the evaluator transaction's actor.
async fn shadow_source_device(state: &AppState, run: &LearningRun, device: Uuid) -> Result<bool> {
    // Capture/import origins do not record a selected manifest. They cannot
    // prove a non-null manifest selected independently by a learning request.
    if run.manifest_revision_id.is_some() {
        return Ok(false);
    }
    let mut tx = db::preparation_tx(&state.pool, run.actor_id, Some(device)).await?;
    if !matches!(
        db::require_role(&mut tx, run.brain_id, false)
            .await?
            .as_str(),
        "writer" | "admin"
    ) {
        return Ok(false);
    }
    let origins: Vec<(Uuid, Json<ScopeSelection>, String)> = sqlx::query_as(
        "SELECT b.operation_id,b.selection,'capture'::text FROM capture_events e
         JOIN capture_bindings b ON b.brain_id=e.brain_id AND b.id=e.binding_id
         JOIN source_versions v ON v.brain_id=e.brain_id AND v.id=e.source_version_id AND v.source_id=e.source_id
         WHERE e.brain_id=$1 AND e.source_version_id=$2 AND e.state='accepted' AND e.expires_at>clock_timestamp()
           AND b.actor_id=$3 AND b.device_id=$4 AND v.created_by=$3
         UNION ALL
         SELECT i.operation_id,i.selection,'write'::text FROM source_import_scopes i
         JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.version_id
         WHERE i.brain_id=$1 AND i.version_id=$2 AND i.operation_id IS NOT NULL AND v.created_by=$3 AND v.device_id=$4",
    )
    .bind(run.brain_id)
    .bind(run.source_version_id)
    .bind(run.actor_id)
    .bind(device)
    .fetch_all(&mut *tx)
    .await?;
    if origins.len() != 1 {
        return Ok(false);
    }
    let (operation, Json(selection), kind) = &origins[0];
    if *selection != run.selection {
        return Ok(false);
    }
    let bound = crate::workspace::bound_operation(&mut tx, run.brain_id, *operation).await?;
    let valid = bound.actor_id == run.actor_id
        && bound.device_id == Some(device)
        && bound.kind == *kind
        && bound.scope_valid
        && bound.scope.selection == run.selection;
    tx.commit().await?;
    Ok(valid)
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
            "WITH base AS MATERIALIZED (SELECT r.id,r.revision FROM claims c JOIN claim_revisions r ON r.id=c.current_revision AND r.brain_id=c.brain_id
             WHERE c.brain_id=$1 AND r.subject_key=$2 AND r.predicate_key=$3
             AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
             AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=r.brain_id AND f.revision_id=r.id)),
             gate AS MATERIALIZED (SELECT s.revision_id FROM recollect_memory_supported_many($1,ARRAY(SELECT b.id FROM base b)) s WHERE s.supported)
             SELECT b.revision FROM base b WHERE b.id IN (SELECT revision_id FROM gate) ORDER BY b.id LIMIT 500")
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
/// holds the Brain lock. The basis edges to every offered input are provenance
/// and erasure dependencies (see `basis`), not a permanent TTL dependency on
/// every offered or superseded source.
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
    item["properties"]["kind"] = json!({"type":"string","enum":["claim","decision","procedure"],"description":"The source-supported assertion kind; preserve the existing kind for an exact offered revision replacement."});
    item["properties"]["procedure"] = json!({"type":["object","null"],"properties":{
        "conditions":{"type":"string","maxLength":2000},"steps":{"type":"array","minItems":1,"maxItems":20,"items":{"type":"string","minLength":1,"maxLength":1000}},
        "expected_outcome":{"type":"string","minLength":1,"maxLength":2000}},"required":["conditions","steps","expected_outcome"],"additionalProperties":false});
    item["required"]
        .as_array_mut()
        .unwrap()
        .extend([json!("kind"), json!("procedure")]);
    item["properties"]["replaces_revision"] = json!({"type":["string","null"],"description":"Exact offered revision.id only for an explicitly supported replacement of that assertion; null for a new assertion or an unchanged assertion. Never guess an ID."});
    let mut allowed = vec![serde_json::Value::Null];
    allowed.extend(run.reconciliation_inputs.iter().map(|id| json!(id)));
    item["properties"]["replaces_revision"]["enum"] = json!(allowed);
    item["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("replaces_revision"));
    schema["properties"]["retirements"] = json!({"type":"array","maxItems":12,"items":{
        "type":"object","description":"Explicit evidence of obsolescence or unsupportedness for an exact offered assertion; omission is never retirement.","properties":{"revision_id":{"type":"string","description":"Exact offered revision.id to retire, never a claim ID or an unoffered revision."},"reason":{"type":"string","description":"Concise explanation of the source evidence explicitly making this assertion obsolete or unsupported."},
        "line_from":{"type":"integer","minimum":1,"description":"First one-based raw-source line explicitly supporting retirement."},"line_to":{"type":"integer","minimum":1,"description":"Last inclusive one-based raw-source line explicitly supporting retirement."}},
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
    inv.instructions.push_str(" Input 0 is the source to learn now; subsequent inputs are existing machine-maintained memories from the same source lineage or authenticated captured session and exact scope. Being offered is only permission to consider a candidate; it is not equivalence, corroboration or evidence that the newest statement wins. Reconcile meaning against the cited source and canonical provenance, using the exact revision.id as replaces_revision only when new evidence explicitly supports an updated assertion. Preserve subject and predicate when their meaning is unchanged. Use null for a new assertion. Do not manufacture a replacement or retirement solely because an unchanged assertion is offered. When input0 independently asserts the same value, you may return that assertion once with replaces_revision=null for native reuse and independent support assessment; retain its genuine new evidence in the run/stage without adding corroboration weight or extending the old claim retention. Omit an unchanged offered assertion when input0 does not assert it. If evidence explicitly updates its actual value, return one replacement assertion with the exact offered revision.id; put the change explanation only in rationale. If evidence explicitly establishes obsolescence without a replacement, return only a retirement of that exact offered revision. These choices are independent of mapping and no missing action may be inferred from omission. Do not revise unrelated facts, assume a newer date makes something true, or treat historical quotations, later mentions, assistant proposals or echoes as current changes. Retire an existing assertion only when exact lines explicitly establish it is obsolete or unsupported; return revision_id, a concise evidence-based reason, and line_from/line_to in retirements. Omission, lack of mention, model confidence and age alone are not grounds for retirement. Leave unresolved alternatives qualified by their wording instead of inventing a winner. Return an empty retirements array when nothing can be retired. One existing revision may be replaced or retired at most once.");
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

/// Link a published replacement, retirement or new claim to every offered
/// reconciliation input. These edges are provenance and erasure dependencies
/// only (migration 071): erasing an offered input's source erases the published
/// claim, but the claim's support is verified through its own supports and
/// verdicts, so it never inherits an offered input's expiry or unsupported
/// state. Only the contributions `procedures::append` writes for a handover are
/// support and deadline dependencies.
pub async fn basis(tx: &mut Tx<'_>, r: &ClaimRevision, inputs: &[Uuid]) -> Result<()> {
    for id in inputs {
        sqlx::query("INSERT INTO claim_contributions(revision_id,brain_id,input_revision_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(r.id).bind(r.brain_id).bind(id).execute(&mut **tx).await?;
    }
    Ok(())
}

async fn brain(state: &AppState, brain: Uuid, actor: Uuid) -> Result<usize> {
    let audits = support_discovery::schedule(state, brain, actor).await?;
    match maintenance(state, brain, actor).await {
        Ok(queued) => Ok(audits + queued),
        Err(error) => {
            tracing::warn!(brain_id=%brain,code=error.1,"Other autonomous maintenance is waiting");
            Ok(audits)
        }
    }
}

async fn maintenance(state: &AppState, brain: Uuid, actor: Uuid) -> Result<usize> {
    // Each phase owns its preparation and commit boundaries. A slow or invalid
    // candidate cannot roll back unrelated progress or retain one Brain lock.
    let mut queued = 0;
    match maintenance::sources(state, brain, actor).await {
        Ok(count) => queued += count,
        Err(error) => tracing::debug!(brain_id=%brain,code=error.1,"Source discovery is waiting"),
    }
    if let Err(error) = maintenance::discover_digests(state, brain, actor).await {
        tracing::debug!(brain_id=%brain,code=error.1,"Digest identity discovery is waiting");
    }
    match maintenance::prepare_digest(state, brain, actor).await {
        Ok(Some(prepared)) => match maintenance::publish_digest(state, prepared, 2).await {
            Ok(count) => queued += count,
            Err(error) => {
                tracing::debug!(brain_id=%brain,code=error.1,"Digest publication is waiting")
            }
        },
        Ok(None) => {}
        Err(error) => tracing::debug!(brain_id=%brain,code=error.1,"Digest preparation is waiting"),
    }
    match maintenance::prepare_handovers(state, brain, actor).await {
        Ok(Some(prepared)) => match maintenance::publish_handovers(state, prepared, 5).await {
            Ok(count) => queued += count,
            Err(error) => {
                tracing::debug!(brain_id=%brain,code=error.1,"Handover publication is waiting")
            }
        },
        Ok(None) => {}
        Err(error) => {
            tracing::debug!(brain_id=%brain,code=error.1,"Handover preparation is waiting")
        }
    }
    match retry_maintenance(state, brain, actor, 20usize.saturating_sub(queued)).await {
        Ok(count) => queued += count,
        Err(error) => tracing::debug!(brain_id=%brain,code=error.1,"Retry maintenance is waiting"),
    }
    Ok(queued)
}

// The same full retry predicates qualify discovery and each exact fresh
// publication. A saved run id is never permission to repeat a provider request.
const LOCAL_RECOVERY_INPUTS: &str = "SELECT r.id,j.id FROM handover_runs r JOIN jobs j ON j.id=r.job_id LEFT JOIN handover_support_stages s ON s.run_id=r.id AND s.brain_id=r.brain_id WHERE r.brain_id=$1 AND r.policy_id=$2 AND r.state='failed' AND j.state='failed' AND j.error_code IN ('database_error','lease_expired') AND r.local_recoveries<2 AND j.updated_at+make_interval(mins=>CASE r.local_recoveries WHEN 0 THEN 5 ELSE 30 END)<=clock_timestamp() AND (s.run_id IS NULL OR (s.privacy_state='active' AND (recollect_handover_support_deadline(s.brain_id,s.run_id) IS NULL OR recollect_handover_support_deadline(s.brain_id,s.run_id)>clock_timestamp()))) AND (s.verdict IS NOT NULL OR NOT EXISTS(SELECT 1 FROM model_requests m WHERE m.brain_id=r.brain_id AND m.operation_id=coalesce(s.assessment_operation_id,r.id))) AND ($4::uuid IS NULL OR r.id=$4) ORDER BY j.updated_at,r.id LIMIT $3";
/// Failures that say nothing permanent about the source. Each is retried at most
/// twice (`automatic_attempt<2`) with the 5 and 30 minute backoff; budget
/// exhaustion waits for the next UTC day. Content removal is never retried.
pub(crate) const LEARNING_RETRY_CODES: &[&str] = &[
    "provider_rate_limited",
    "provider_unavailable",
    "provider_incomplete",
    "provider_shape",
    "model_budget_exhausted",
    "reconciliation_input_changed",
    "learning_input_expired",
    "claim_capacity",
    "review_capacity",
    "permission_or_policy_denied",
];
const LEARNING_RETRY_INPUTS: &str = "SELECT to_jsonb(r) FROM learning_runs r JOIN sources s ON s.current_version=r.source_version_id
         WHERE r.brain_id=$1 AND r.policy_id=$2 AND r.automatic AND r.state='failed'
         AND r.error_code=ANY($5) AND r.automatic_attempt<2
         AND CASE WHEN r.error_code='model_budget_exhausted'
           THEN ((date_trunc('day',r.finished_at AT TIME ZONE 'UTC')+interval '1 day') AT TIME ZONE 'UTC')<=clock_timestamp()
           ELSE r.finished_at+make_interval(mins=>CASE r.automatic_attempt WHEN 0 THEN 5 ELSE 30 END)<=clock_timestamp() END
         AND NOT EXISTS(SELECT 1 FROM learning_runs child WHERE child.retry_of=r.id)
         AND NOT EXISTS(SELECT 1 FROM learning_runs newer WHERE newer.brain_id=r.brain_id AND newer.source_version_id=r.source_version_id AND newer.policy_id=r.policy_id AND newer.automatic AND newer.retry_of IS NULL AND newer.created_at>r.created_at)
         AND NOT EXISTS(SELECT 1 FROM learning_support_stages stage WHERE stage.run_id=r.id AND stage.brain_id=r.brain_id AND (stage.verdicts IS NOT NULL OR stage.privacy_state<>'active' OR recollect_learning_support_deadline(stage.brain_id,stage.run_id)<=clock_timestamp()))
         AND ($4::uuid IS NULL OR r.id=$4) ORDER BY r.finished_at,r.id LIMIT $3";
const HANDOVER_RETRY_INPUTS: &str = "SELECT to_jsonb(r) FROM handover_runs r WHERE r.brain_id=$1 AND r.policy_id=$2 AND r.state='failed'
         AND r.error_code IN ('provider_rate_limited','provider_unavailable','provider_incomplete','provider_shape','model_response_invalid') AND r.automatic_attempt<2
         AND r.finished_at+make_interval(mins=>CASE r.automatic_attempt WHEN 0 THEN 5 ELSE 30 END)<=clock_timestamp()
         AND NOT EXISTS(SELECT 1 FROM handover_runs child WHERE child.retry_of=r.id)
         AND NOT EXISTS(SELECT 1 FROM handover_support_stages stage WHERE stage.run_id=r.id AND stage.brain_id=r.brain_id AND (stage.verdict IS NOT NULL OR stage.privacy_state<>'active' OR recollect_handover_support_deadline(stage.brain_id,stage.run_id)<=clock_timestamp()))
         AND EXISTS(SELECT 1 FROM model_requests m LEFT JOIN handover_support_stages stage ON stage.run_id=r.id AND stage.brain_id=r.brain_id WHERE m.brain_id=r.brain_id AND m.operation_id=coalesce(stage.assessment_operation_id,r.id) AND m.state IN ('succeeded','failed') AND NOT m.suppressed)
         AND ($4::uuid IS NULL OR r.id=$4) ORDER BY r.finished_at,r.id LIMIT $3";

async fn retry_maintenance(
    state: &AppState,
    brain: Uuid,
    actor: Uuid,
    limit: usize,
) -> Result<usize> {
    if limit == 0 {
        return Ok(0);
    }
    let Some((mut tx, p, cursor)) =
        maintenance::Cursor::snapshot(state, brain, actor, "source").await?
    else {
        return Ok(0);
    };
    let local: Vec<(Uuid, Uuid)> = sqlx::query_as(LOCAL_RECOVERY_INPUTS)
        .bind(brain)
        .bind(p.change_id)
        .bind(limit as i64)
        .bind(None::<Uuid>)
        .fetch_all(&mut *tx)
        .await?;
    let learning: Vec<Json<LearningRun>> = sqlx::query_scalar(LEARNING_RETRY_INPUTS)
        .bind(brain)
        .bind(p.change_id)
        .bind(limit.saturating_sub(local.len()) as i64)
        .bind(None::<Uuid>)
        .bind(LEARNING_RETRY_CODES)
        .fetch_all(&mut *tx)
        .await?;
    let handovers: Vec<Json<HandoverRun>> = sqlx::query_scalar(HANDOVER_RETRY_INPUTS)
        .bind(brain)
        .bind(p.change_id)
        .bind(limit.saturating_sub(local.len() + learning.len()) as i64)
        .bind(None::<Uuid>)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    let mut queued = 0;
    for (id, _) in local {
        let outcome = maintenance::bounded(async {
            let Some((mut tx, p)) = cursor.writer(state, true).await? else { return Ok(0); };
            let fresh: Option<(Uuid, Uuid)> = sqlx::query_as(LOCAL_RECOVERY_INPUTS)
                .bind(brain).bind(p.change_id).bind(1_i64).bind(Some(id)).fetch_optional(&mut *tx).await?;
            let Some((run, job)) = fresh else { return Ok(0); };
            sqlx::query("UPDATE handover_runs SET state='queued',error_code=NULL,finished_at=NULL,local_recoveries=local_recoveries+1 WHERE brain_id=$1 AND id=$2")
                .bind(brain).bind(run).execute(&mut *tx).await?;
            sqlx::query("UPDATE jobs SET state='queued',error_code=NULL,max_attempts=max_attempts+3,not_before=clock_timestamp(),updated_at=clock_timestamp() WHERE brain_id=$1 AND id=$2")
                .bind(brain).bind(job).execute(&mut *tx).await?;
            tx.commit().await?;
            Ok(1)
        }).await;
        match outcome {
            Ok(n) => queued += n,
            Err(e) => tracing::debug!(brain_id=%brain,code=e.1,"Local handover recovery deferred"),
        }
    }
    for Json(prior) in learning {
        let outcome = maintenance::bounded(async {
            let Some((mut tx, p)) = cursor.writer(state, true).await? else {
                return Ok(0);
            };
            let fresh: Option<Json<LearningRun>> = sqlx::query_scalar(LEARNING_RETRY_INPUTS)
                .bind(brain)
                .bind(p.change_id)
                .bind(1_i64)
                .bind(Some(prior.id))
                .bind(LEARNING_RETRY_CODES)
                .fetch_optional(&mut *tx)
                .await?;
            let Some(Json(old)) = fresh else {
                return Ok(0);
            };
            let run = learning::enqueue(
                &mut tx,
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
            crate::memory_support::inherit_assessment_retry(&mut tx, &old, &run).await?;
            tx.commit().await?;
            Ok(1)
        })
        .await;
        match outcome {
            Ok(n) => queued += n,
            Err(e) => tracing::debug!(brain_id=%brain,code=e.1,"Learning retry deferred"),
        }
    }
    for Json(prior) in handovers {
        let outcome = maintenance::bounded(async {
            let Some((mut tx, p)) = cursor.writer(state, true).await? else { return Ok(0); };
            let fresh: Option<Json<HandoverRun>> = sqlx::query_scalar(HANDOVER_RETRY_INPUTS)
                .bind(brain).bind(p.change_id).bind(1_i64).bind(Some(prior.id)).fetch_optional(&mut *tx).await?;
            let Some(Json(old)) = fresh else { return Ok(0); };
            crate::handovers::scope(state, &mut tx, brain, actor, None, &crate::handovers::input(&old)).await?;
            let id = Uuid::new_v4();
            let audit = db::audit(&mut tx, actor, brain, "handover.queue", id, "automatic_retry").await?;
            let job = crate::jobs::enqueue_work(&mut tx, actor, brain, audit, id, "handover.generate", "model").await?;
            sqlx::query("INSERT INTO handover_runs(id,brain_id,title,contributions,selection,policy_id,actor_id,job_id,state,automatic,claim_id,base_revision_id,retry_of,automatic_attempt) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'queued',true,$9,$10,$11,$12)")
                .bind(id).bind(brain).bind(&old.title).bind(&old.contributions).bind(Json(&old.selection))
                .bind(p.change_id).bind(actor).bind(job).bind(old.claim_id).bind(old.base_revision_id)
                .bind(old.id).bind(old.automatic_attempt+1).execute(&mut *tx).await?;
            for input in &old.contributions {
                sqlx::query("INSERT INTO handover_run_inputs(run_id,brain_id,revision_id) VALUES($1,$2,$3)")
                    .bind(id).bind(brain).bind(input).execute(&mut *tx).await?;
            }
            crate::handover_support::inherit_retry(&mut tx, &old, id).await?;
            tx.commit().await?;
            Ok(1)
        }).await;
        match outcome {
            Ok(n) => queued += n,
            Err(e) => tracing::debug!(brain_id=%brain,code=e.1,"Handover retry deferred"),
        }
    }
    Ok(queued)
}

pub async fn run_once(state: &AppState) -> Result<usize> {
    let brains: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT brain_id,actor_id FROM recollect_autonomous_brains()")
            .fetch_all(&state.pool)
            .await?;
    Ok(crate::worker::each_brain("autonomous", brains, |id, actor| brain(state, id, actor)).await)
}
