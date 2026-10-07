//! Incremental session continuation. Scheduling is deterministic and model-free;
//! synthesis, independent support and recovery reuse the native handover worker.
use crate::{AppState, db, error::Result, jobs, memory, memory_evidence::Tx, model_policy};
use recollect_protocol::{
    ClaimRevision, HandoverInput, HandoverRun, ModelPolicyVersion, ScopeSelection,
};
use serde::Deserialize;
use sqlx::types::Json;
use uuid::Uuid;

enum Preflight {
    Fits((usize, usize)),
    Omitted(&'static str),
}

#[derive(Deserialize)]
struct Partition {
    id: Uuid,
    selection: ScopeSelection,
    manifest_revision_id: Option<Uuid>,
    observed_receipt: i64,
}

pub(crate) async fn schedule(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    policy: &ModelPolicyVersion,
    limit: usize,
) -> Result<usize> {
    if limit == 0
        || model_policy::permits(
            state,
            &policy.policy,
            "synthesis",
            &["claim".into(), "query".into()],
        )
        .is_err()
        || model_policy::permits(state, &policy.policy, "extraction", &["claim".into()]).is_err()
    {
        return Ok(0);
    }
    // Only server-authenticated capture bindings supply a partition. No latest
    // session guess, filesystem inspection, or summary of another summary.
    sqlx::query("SELECT recollect_digest_discover($1)")
        .bind(brain)
        .execute(&mut **tx)
        .await?;
    let partitions: Vec<Json<Partition>> = sqlx::query_scalar("SELECT to_jsonb(p) FROM session_digest_partitions p
      WHERE p.brain_id=$1 AND p.privacy_state='active' AND (recollect_digest_task_closed(p.brain_id,p.id) OR NOT EXISTS(SELECT 1 FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id WHERE d.partition_id=p.id AND e.received_at>clock_timestamp()-interval '5 minutes'))
      AND NOT EXISTS(SELECT 1 FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id JOIN source_versions v ON v.id=e.source_version_id AND v.brain_id=e.brain_id
       WHERE d.partition_id=p.id AND (v.processing IN ('pending','processing','queued') OR EXISTS(SELECT 1 FROM learning_runs l WHERE l.brain_id=$1 AND l.source_version_id=v.id AND l.policy_id=$2 AND l.state IN ('queued','running'))))
      ORDER BY (p.observed_receipt>p.covered_receipt) DESC,p.last_checked_at,p.id LIMIT 100")
        .bind(brain).bind(policy.change_id).fetch_all(&mut **tx).await?;
    let mut queued = 0;
    for Json(p) in partitions {
        sqlx::query(
            "UPDATE session_digest_partitions SET last_checked_at=clock_timestamp() WHERE id=$1",
        )
        .bind(p.id)
        .execute(&mut **tx)
        .await?;
        let rows: Vec<Json<ClaimRevision>> = sqlx::query_scalar("SELECT DISTINCT r.revision FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id
          JOIN learning_runs l ON l.brain_id=e.brain_id AND l.source_version_id=e.source_version_id AND l.state IN ('succeeded','removed')
          JOIN claims c ON c.brain_id=l.brain_id AND c.id=ANY(l.claim_ids) JOIN claim_revisions r ON r.id=c.current_revision AND r.brain_id=c.brain_id
          WHERE d.partition_id=$1 AND r.revision#>>'{content,kind}'<>'handover' AND r.revision#>'{content,selection}'=$2
           AND r.revision#>>'{content,manifest_revision_id}' IS NOT DISTINCT FROM $3::text
           AND r.revision->>'review'='accepted' AND recollect_memory_supported(r.brain_id,r.id)
          ORDER BY r.revision LIMIT 500")
            .bind(p.id).bind(Json(&p.selection)).bind(p.manifest_revision_id.map(|m|m.to_string())).fetch_all(&mut **tx).await?;
        let scan_bounded = rows.len() == 500;
        let mut eligible = Vec::new();
        for Json(r) in rows {
            let v = memory::view(state, tx, r, chrono::Utc::now(), None).await?;
            if v.eligibility.investigation
                && v.eligibility.rule_ids.is_empty()
                && v.revision.content.freshness == "current"
            {
                eligible.push(v);
            }
        }
        eligible.sort_by_key(|v| v.revision.id);
        // A page is a whole-input budget, never a clipped assertion. Empty pages
        // record coverage but cannot preserve a formerly complete old digest.
        let eligible_count = eligible.len();
        let mut omitted = 0usize;
        let mut omissions = std::collections::BTreeMap::<&str, usize>::new();
        let mut pages: Vec<(Vec<Uuid>, (usize, usize))> = vec![];
        let mut page = Vec::new();
        let mut allowance = (0, 0);
        for v in eligible {
            let mut trial = page.clone();
            trial.push(v.revision.id);
            if let Preflight::Fits(n) = preflight(state, tx, brain, &p, &trial, policy).await? {
                page = trial;
                allowance = n;
                continue;
            }
            if !page.is_empty() {
                pages.push((std::mem::take(&mut page), allowance));
            }
            let single = vec![v.revision.id];
            match preflight(state, tx, brain, &p, &single, policy).await? {
                Preflight::Fits(n) => {
                    page = single;
                    allowance = n;
                }
                Preflight::Omitted(reason) => {
                    omitted += 1;
                    *omissions.entry(reason).or_default() += 1;
                }
            }
        }
        if !page.is_empty() {
            pages.push((page, allowance));
        }
        sqlx::query("UPDATE session_digest_partitions SET coverage=$2 WHERE id=$1")
            .bind(p.id).bind(Json(serde_json::json!({"eligible":eligible_count,"omitted":omitted,"bounded_scan":scan_bounded,"pages":pages.len(),"omissions":omissions}))).execute(&mut **tx).await?;
        for (page, (ids, allowance)) in pages.iter().enumerate() {
            if ids.is_empty() {
                continue;
            }
            let previous: Option<(Uuid,Uuid)> = sqlx::query_as("SELECT d.claim_id,c.current_revision FROM session_digest_claims d JOIN claims c ON c.id=d.claim_id AND c.brain_id=d.brain_id WHERE d.partition_id=$1 AND d.page=$2")
                .bind(p.id).bind(page as i32).fetch_optional(&mut **tx).await?;
            if let Some((claim, base)) = previous {
                let old = memory::current(tx, brain, claim).await?;
                if old.id != base
                    || !crate::autonomous::maintained(&old)
                    || !crate::memory_rules::matching(tx, &old, false)
                        .await?
                        .is_empty()
                {
                    sqlx::query("UPDATE session_digest_pages SET active=false WHERE partition_id=$1 AND page=$2").bind(p.id).bind(page as i32).execute(&mut **tx).await?;
                    continue;
                }
            }
            let existing: Option<(Uuid,bool)> = sqlx::query_as("SELECT g.id,g.privacy_state='active' AND (EXISTS(
                  WITH RECURSIVE recovery AS (
                    SELECT r.id,r.state,r.error_code,r.automatic_attempt,r.local_recoveries,r.job_id,0 depth
                    UNION ALL SELECT child.id,child.state,child.error_code,child.automatic_attempt,child.local_recoveries,child.job_id,parent.depth+1
                    FROM handover_runs child JOIN recovery parent ON child.retry_of=parent.id
                    WHERE child.brain_id=g.brain_id AND child.policy_id=g.policy_id AND parent.depth<2
                  ) SELECT 1 FROM recovery candidate JOIN jobs j ON j.id=candidate.job_id
                  LEFT JOIN handover_support_stages stage ON stage.run_id=candidate.id AND stage.brain_id=g.brain_id
                  WHERE candidate.state IN ('queued','running') OR (candidate.state='failed'
                    AND NOT EXISTS(SELECT 1 FROM handover_runs child WHERE child.brain_id=g.brain_id AND child.retry_of=candidate.id)
                    AND (stage.run_id IS NULL OR (stage.privacy_state='active' AND
                      (recollect_handover_support_deadline(g.brain_id,candidate.id) IS NULL OR recollect_handover_support_deadline(g.brain_id,candidate.id)>clock_timestamp())))
                    AND ((j.error_code IN ('database_error','lease_expired') AND candidate.local_recoveries<2
                      AND (stage.verdict IS NOT NULL OR NOT EXISTS(SELECT 1 FROM model_requests m WHERE m.brain_id=g.brain_id AND m.operation_id=coalesce(stage.assessment_operation_id,candidate.id))))
                      OR (candidate.error_code IN ('provider_rate_limited','provider_unavailable','provider_incomplete','provider_shape','model_response_invalid')
                        AND candidate.automatic_attempt<2 AND stage.verdict IS NULL
                        AND EXISTS(SELECT 1 FROM model_requests m WHERE m.brain_id=g.brain_id AND m.operation_id=coalesce(stage.assessment_operation_id,candidate.id) AND m.state IN ('succeeded','failed') AND NOT m.suppressed))))
                ) OR EXISTS(SELECT 1 FROM session_digest_claims d JOIN claims c ON c.id=d.claim_id AND c.brain_id=d.brain_id WHERE d.generation_id=g.id AND d.revision_id=c.current_revision)) FROM session_digest_generations g JOIN handover_runs r ON r.id=g.run_id AND r.brain_id=g.brain_id WHERE g.partition_id=$1 AND g.page=$2 AND g.contributors=$3 AND g.policy_id=$4 AND g.version='session-digest-1' AND (g.base_revision_id IS NOT DISTINCT FROM $5 OR EXISTS(SELECT 1 FROM session_digest_claims d JOIN claims c ON c.id=d.claim_id AND c.brain_id=d.brain_id WHERE d.generation_id=g.id AND d.revision_id=c.current_revision)) ORDER BY g.created_at DESC LIMIT 1")
                .bind(p.id).bind(page as i32).bind(ids).bind(policy.change_id).bind(previous.map(|x|x.1)).fetch_optional(&mut **tx).await?;
            if let Some((generation, reusable)) = existing {
                if !reusable {
                    sqlx::query("UPDATE session_digest_pages SET active=false WHERE partition_id=$1 AND page=$2").bind(p.id).bind(page as i32).execute(&mut **tx).await?;
                    sqlx::query("UPDATE session_digest_partitions SET coverage=coverage||jsonb_build_object('generation_outcome','terminal_unchanged_inputs') WHERE id=$1").bind(p.id).execute(&mut **tx).await?;
                    continue;
                }
                // A newer receipt does not change immutable model inputs. Once
                // settlement proves exact contributors unchanged, advance only
                // their coverage authorization; no provider replay.
                sqlx::query("UPDATE session_digest_generations SET receipt=$2 WHERE id=$1 AND privacy_state='active'")
                    .bind(generation).bind(p.observed_receipt).execute(&mut **tx).await?;
                sqlx::query("INSERT INTO session_digest_pages(brain_id,partition_id,page,generation_id) VALUES($1,$2,$3,$4) ON CONFLICT(partition_id,page) DO UPDATE SET generation_id=EXCLUDED.generation_id,active=true")
                    .bind(brain).bind(p.id).bind(page as i32).bind(generation).execute(&mut **tx).await?;
                sqlx::query("UPDATE session_digest_claims SET covered_receipt=$3 WHERE partition_id=$1 AND generation_id=$2")
                    .bind(p.id).bind(generation).bind(p.observed_receipt).execute(&mut **tx).await?;
                continue;
            }
            if queued == limit {
                return Ok(queued);
            }
            let input = HandoverInput {
                title: "Session continuation".into(),
                contributions: ids.clone(),
                operation_id: None,
            };
            let (selection, _) =
                crate::handovers::scope(state, tx, brain, actor, None, &input).await?;
            if selection != p.selection {
                continue;
            }
            jobs::capacity(tx, brain).await?;
            let run = Uuid::new_v4();
            let audit = db::audit(
                tx,
                actor,
                brain,
                "handover.queue",
                run,
                "automatic_session_digest",
            )
            .await?;
            let job =
                jobs::enqueue_work(tx, actor, brain, audit, run, "handover.generate", "model")
                    .await?;
            sqlx::query("INSERT INTO handover_runs(id,brain_id,title,contributions,selection,policy_id,actor_id,job_id,state,automatic,claim_id,base_revision_id) VALUES($1,$2,'Session continuation',$3,$4,$5,$6,$7,'queued',true,$8,$9)")
                .bind(run).bind(brain).bind(ids).bind(Json(&p.selection)).bind(policy.change_id).bind(actor).bind(job).bind(previous.map(|x|x.0)).bind(previous.map(|x|x.1)).execute(&mut **tx).await?;
            for id in ids {
                sqlx::query(
                    "INSERT INTO handover_run_inputs(run_id,brain_id,revision_id) VALUES($1,$2,$3)",
                )
                .bind(run)
                .bind(brain)
                .bind(id)
                .execute(&mut **tx)
                .await?;
            }
            let generation = Uuid::new_v4();
            sqlx::query("INSERT INTO session_digest_generations(id,brain_id,partition_id,page,contributors,policy_id,receipt,run_id,candidate_bytes,candidate_overhead,base_revision_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
                .bind(generation).bind(brain).bind(p.id).bind(page as i32).bind(ids).bind(policy.change_id).bind(p.observed_receipt).bind(run).bind(allowance.0 as i32).bind(allowance.1 as i32).bind(previous.map(|x|x.1)).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO session_digest_pages(brain_id,partition_id,page,generation_id) VALUES($1,$2,$3,$4) ON CONFLICT(partition_id,page) DO UPDATE SET generation_id=EXCLUDED.generation_id,active=true")
                .bind(brain).bind(p.id).bind(page as i32).bind(generation).execute(&mut **tx).await?;
            queued += 1;
        }
        // Pages which disappeared cannot claim fresh coverage.
        sqlx::query(
            "UPDATE session_digest_pages SET active=false WHERE partition_id=$1 AND page>=$2",
        )
        .bind(p.id)
        .bind(pages.len() as i32)
        .execute(&mut **tx)
        .await?;
        sqlx::query("UPDATE session_digest_partitions SET covered_receipt=$2 WHERE id=$1")
            .bind(p.id)
            .bind(p.observed_receipt)
            .execute(&mut **tx)
            .await?;
    }
    Ok(queued)
}

async fn preflight(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    p: &Partition,
    ids: &[Uuid],
    policy: &ModelPolicyVersion,
) -> Result<Preflight> {
    if ids.len() > 12 {
        return Ok(Preflight::Omitted("whole_input_budget"));
    }
    let contributors = crate::procedures::inputs(state, tx, brain, ids).await?;
    let Ok((selection, supports)) = crate::procedures::combined(&contributors) else {
        return Ok(Preflight::Omitted("incompatible_inputs"));
    };
    if selection != p.selection {
        return Ok(Preflight::Omitted("scope_mismatch"));
    }
    if supports.len() > 20 {
        return Ok(Preflight::Omitted("support_count_budget"));
    }
    let mut synthesis =
        "Session continuation".len() + serde_json::json!({"kind":"query"}).to_string().len();
    for c in &contributors {
        let v = memory::view(state, tx, c.clone(), chrono::Utc::now(), None).await?;
        synthesis += serde_json::to_vec(&v).expect("typed input").len()
            + serde_json::json!({"kind":"claim_revision","id":c.id})
                .to_string()
                .len();
    }
    let limit = policy.policy.max_input_bytes as usize;
    if synthesis > limit {
        return Ok(Preflight::Omitted("whole_input_budget"));
    }
    let (mut payload, classes, _) =
        crate::handover_support::evidence(state, tx, brain, &contributors, &supports).await?;
    if model_policy::permits(state, &policy.policy, "extraction", &classes).is_err() {
        return Ok(Preflight::Omitted("source_permission_denied"));
    }
    payload["candidate"] = serde_json::Value::Null;
    let base = payload.to_string().len() - 4
        + serde_json::json!({"kind":"handover_support_stage","id":Uuid::nil()})
            .to_string()
            .len();
    let allowance = limit.saturating_sub(base);
    let empty = crate::handovers::content(
        selection,
        "Session continuation".into(),
        ids.to_vec(),
        supports,
        p.manifest_revision_id,
        crate::handovers::Draft::default(),
    );
    let overhead = serde_json::to_vec(&empty).expect("typed candidate").len() + 32;
    if allowance < overhead + 256 {
        return Ok(Preflight::Omitted("whole_input_budget"));
    }
    Ok(Preflight::Fits((allowance, overhead)))
}

async fn generation(tx: &mut Tx<'_>, brain: Uuid, run: Uuid) -> Result<Option<serde_json::Value>> {
    let row: Option<Json<serde_json::Value>> = sqlx::query_scalar("WITH RECURSIVE lineage(id,retry_of) AS (SELECT id,retry_of FROM handover_runs WHERE brain_id=$1 AND id=$2 UNION ALL SELECT r.id,r.retry_of FROM handover_runs r JOIN lineage l ON r.id=l.retry_of WHERE r.brain_id=$1)
      SELECT to_jsonb(g) FROM session_digest_generations g JOIN lineage l ON l.id=g.run_id WHERE g.brain_id=$1 LIMIT 1")
        .bind(brain).bind(run).fetch_optional(&mut **tx).await?;
    Ok(row.map(|x| x.0))
}

pub(crate) async fn manifest(tx: &mut Tx<'_>, brain: Uuid, run: Uuid) -> Result<Option<Uuid>> {
    let Some(g) = generation(tx, brain, run).await? else {
        return Ok(None);
    };
    sqlx::query_scalar(
        "SELECT manifest_revision_id FROM session_digest_partitions WHERE id=$1 AND brain_id=$2",
    )
    .bind(g["partition_id"].as_str().unwrap().parse::<Uuid>().unwrap())
    .bind(brain)
    .fetch_one(&mut **tx)
    .await
    .map_err(Into::into)
}

pub(crate) async fn candidate_limit(
    tx: &mut Tx<'_>,
    brain: Uuid,
    run: Uuid,
) -> Result<Option<(usize, usize)>> {
    Ok(generation(tx, brain, run).await?.map(|g| {
        (
            g["candidate_bytes"].as_u64().unwrap() as usize,
            g["candidate_overhead"].as_u64().unwrap() as usize,
        )
    }))
}
pub(crate) async fn check(
    tx: &mut Tx<'_>,
    run: &HandoverRun,
    completed_response: bool,
) -> Result<()> {
    let Some(g) = generation(tx, run.brain_id, run.id).await? else {
        return Ok(());
    };
    let id = g["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    let (active,desired,coverage,heads):(bool,bool,bool,bool)=sqlx::query_as("SELECT g.privacy_state='active' AND p.privacy_state='active', EXISTS(SELECT 1 FROM session_digest_pages desired WHERE desired.partition_id=p.id AND desired.page=g.page AND desired.generation_id=g.id AND desired.active), g.receipt=p.observed_receipt, g.contributors=(SELECT contributions FROM handover_runs WHERE id=g.run_id AND brain_id=g.brain_id) AND NOT EXISTS(SELECT 1 FROM unnest(g.contributors) i LEFT JOIN claim_revisions r ON r.id=i AND r.brain_id=g.brain_id LEFT JOIN claims c ON c.id=r.claim_id AND c.brain_id=g.brain_id WHERE c.current_revision IS DISTINCT FROM i OR NOT recollect_memory_supported(g.brain_id,i)) FROM session_digest_generations g JOIN session_digest_partitions p ON p.id=g.partition_id WHERE g.id=$1")
        .bind(id).fetch_one(&mut **tx).await?;
    if !active {
        return Err(crate::retention::unavailable());
    }
    if !desired || !heads {
        return Err(model_policy::failure(
            "handover_input_changed",
            "This continuation generation was superseded.",
        ));
    }
    if !coverage && !completed_response {
        return Err(model_policy::failure(
            "session_coverage_pending",
            "Session continuation is waiting for settled capture coverage.",
        ));
    }
    Ok(())
}

pub(crate) async fn published(
    tx: &mut Tx<'_>,
    run: &HandoverRun,
    claim: Uuid,
    revision: Uuid,
) -> Result<()> {
    let Some(g) = generation(tx, run.brain_id, run.id).await? else {
        return Ok(());
    };
    check(tx, run, false).await?;
    let updated = sqlx::query("INSERT INTO session_digest_claims(brain_id,partition_id,page,claim_id,generation_id,revision_id,covered_receipt) VALUES($1,$2,$3,$4,$5,$7,$6) ON CONFLICT(partition_id,page) DO UPDATE SET generation_id=EXCLUDED.generation_id,revision_id=EXCLUDED.revision_id,covered_receipt=EXCLUDED.covered_receipt WHERE session_digest_claims.claim_id=EXCLUDED.claim_id")
        .bind(run.brain_id).bind(g["partition_id"].as_str().unwrap().parse::<Uuid>().unwrap()).bind(g["page"].as_i64().unwrap() as i32).bind(claim).bind(g["id"].as_str().unwrap().parse::<Uuid>().unwrap()).bind(g["receipt"].as_i64().unwrap()).bind(revision).execute(&mut **tx).await?;
    if updated.rows_affected() != 1 {
        return Err(model_policy::failure(
            "handover_input_changed",
            "Session continuation identity changed before publication.",
        ));
    }
    Ok(())
}
