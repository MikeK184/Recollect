use super::*;
use crate::{
    auth::Auth,
    commands, db,
    db::admission::{self, Archived, BrainAccess},
    jobs,
};
use axum::{
    Json as ResponseJson,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use recollect_protocol::{
    ModelPolicyVersion, SemanticBatch, SemanticCounts, SemanticReindex, SemanticStatus,
};
use std::collections::{HashMap, HashSet};

pub(super) const ENTRY_LIMIT: i64 = 50000;

pub(super) async fn batch(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<SemanticBatch> {
    let row: Option<Json<SemanticBatch>> = sqlx::query_scalar(
        "SELECT to_jsonb(b) FROM semantic_batches b WHERE brain_id=$1 AND id=$2",
    )
    .bind(brain)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|r| r.0).ok_or_else(Error::missing)
}

pub(super) async fn create_profile(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    policy: &ModelPolicyVersion,
) -> Result<SemanticProfile> {
    if !model_policy::matches_installation(state, &policy.policy) {
        return Err(model_policy::failure(
            "semantic_profile_mismatch",
            "Choose supported embedding dimensions before rebuilding.",
        ));
    }
    let id = Uuid::new_v4();
    let Json(profile) = sqlx::query_scalar(
        "INSERT INTO semantic_profiles(id,brain_id,provider,model,dimensions,representation,created_by,policy_id)
         VALUES($1,$2,$8,$3,$4,$5,$6,$7) RETURNING to_jsonb(semantic_profiles)",
    )
    .bind(id).bind(brain).bind(&policy.policy.embedding_model).bind(policy.policy.embedding_dimensions)
    .bind(REPRESENTATION).bind(actor).bind(policy.change_id).bind(&policy.policy.provider).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO semantic_heads(brain_id,profile_id) VALUES($1,$2) ON CONFLICT(brain_id) DO UPDATE SET profile_id=excluded.profile_id")
        .bind(brain).bind(id).execute(&mut **tx).await?;
    db::audit(tx, actor, brain, "semantic.reindex", id, "queued").await?;
    Ok(profile)
}

pub(crate) async fn rebuild(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    policy: &ModelPolicyVersion,
) -> Result<SemanticProfile> {
    // Clear old vectors and stop old work in the same transaction that moves the
    // policy/profile head. In-flight network results still face publication fences.
    sqlx::query("UPDATE semantic_batches SET state='removed',error_code='semantic_profile_changed',finished_at=clock_timestamp() WHERE brain_id=$1 AND state IN ('queued','running','blocked')")
        .bind(brain).execute(&mut **tx).await?;
    sqlx::query("UPDATE jobs SET state='cancelled',error_code='semantic_profile_changed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE brain_id=$1 AND kind='semantic.generate' AND state IN ('queued','running')")
        .bind(brain).execute(&mut **tx).await?;
    sqlx::query("UPDATE semantic_entries SET state='removed',embedding=NULL,error_code='semantic_profile_changed',updated_at=clock_timestamp() WHERE brain_id=$1 AND state<>'removed'")
        .bind(brain).execute(&mut **tx).await?;
    create_profile(state, tx, brain, actor, policy).await
}

#[derive(serde::Deserialize)]
pub struct StatusPage {
    offset: Option<i64>,
    #[serde(default)]
    summary: bool,
}

#[utoipa::path(get,path="/api/brains/{brain}/semantic",operation_id="semanticStatus",params(("brain"=Uuid,Path),("offset"=Option<i64>,Query),("summary"=Option<bool>,Query)),responses((status=200,body=SemanticStatus)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(page): Query<StatusPage>,
) -> Result<ResponseJson<SemanticStatus>> {
    let offset = publication::offset(&publication::Page {
        offset: page.offset,
    })?;
    let mut tx = admission::brain_tx(&state.pool, &auth, brain, BrainAccess::READ)
        .await?
        .tx;
    let p = model_policy::current(&state, &mut tx, brain).await?;
    let profile = profile(&mut tx, brain).await?;
    let profile_id = profile.as_ref().map(|p| p.id);
    let Json(counts): Json<SemanticCounts> = sqlx::query_scalar(
        "SELECT jsonb_build_object('pending',count(*) FILTER(WHERE state='pending'),
          'queued',count(*) FILTER(WHERE state='queued'),'running',count(*) FILTER(WHERE state='running'),
          'ready',count(*) FILTER(WHERE state='ready'),'blocked',count(*) FILTER(WHERE state='blocked'),
          'failed',count(*) FILTER(WHERE state='failed'),'removed',count(*) FILTER(WHERE state='removed'),
          'truncated',count(*) FILTER(WHERE truncated AND state='ready'))
         FROM semantic_entries WHERE brain_id=$1 AND profile_id=$2",
    ).bind(brain).bind(profile_id).fetch_one(&mut *tx).await?;
    let allowed = model_policy::permits(&state, &p.policy, "embedding", &[]).is_ok();
    let blocked_rebuild = profile
        .as_ref()
        .is_some_and(|profile| profile.policy_id == p.change_id)
        && (!allowed || !p.policy.automatic_embedding);
    let mut coverage = Vec::new();
    if blocked_rebuild {
        coverage.push("semantic_rebuild_permission_blocked".into());
    }
    if counts.pending + counts.queued + counts.running > 0 {
        coverage.push("semantic_work_pending".into());
    }
    if counts.blocked + counts.failed > 0 {
        coverage.push("semantic_inputs_unavailable".into());
    }
    if counts.truncated > 0 {
        coverage.push("semantic_representation_truncated".into());
    }
    let candidate_sql = include_str!("../semantic_candidates.sql");
    let missing: bool =
        sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM ({candidate_sql}) c)"))
            .bind(brain)
            .bind(&p.policy.content_classes)
            .bind(profile_id)
            .fetch_one(&mut *tx)
            .await?;
    if missing {
        coverage.push("semantic_discovery_pending".into());
    }
    if counts.pending
        + counts.queued
        + counts.running
        + counts.ready
        + counts.blocked
        + counts.failed
        + counts.removed
        >= ENTRY_LIMIT
        && missing
    {
        coverage.push("semantic_capacity_reached".into());
    }
    let status = if blocked_rebuild {
        "blocked"
    } else if !allowed {
        "disabled"
    } else if profile
        .as_ref()
        .is_some_and(|profile| !compatible(&p.policy, profile))
    {
        "profile_mismatch"
    } else if profile.is_none() {
        "missing"
    } else if counts.ready > 0 && coverage.is_empty() {
        "ready"
    } else if counts.ready > 0 {
        "partial"
    } else if counts.pending + counts.queued + counts.running > 0 || missing {
        "building"
    } else if counts.blocked + counts.failed > 0 {
        "blocked"
    } else {
        "empty"
    };
    let (total_batches, batches) = if page.summary {
        (0, vec![])
    } else {
        let total_batches =
            sqlx::query_scalar("SELECT count(*) FROM semantic_batches WHERE brain_id=$1")
                .bind(brain)
                .fetch_one(&mut *tx)
                .await?;
        let batches: Vec<Json<SemanticBatch>> = sqlx::query_scalar("SELECT to_jsonb(b)||jsonb_build_object('can_retry',
        b.state IN ('blocked','failed','removed') AND b.profile_id=(SELECT profile_id FROM semantic_heads WHERE brain_id=$1)
        AND NOT EXISTS(SELECT 1 FROM semantic_batches child WHERE child.retry_of=b.id)
        AND EXISTS(SELECT 1 FROM semantic_batch_inputs i JOIN semantic_entries e ON e.brain_id=i.brain_id AND e.id=i.entry_id
          WHERE i.brain_id=$1 AND i.batch_id=b.id AND e.batch_id=b.id AND e.state IN ('blocked','failed')))
        FROM semantic_batches b WHERE brain_id=$1 ORDER BY created_at DESC,id DESC LIMIT 20 OFFSET $2")
        .bind(brain).bind(offset).fetch_all(&mut *tx).await?;
        (total_batches, batches)
    };
    tx.commit().await?;
    Ok(ResponseJson(SemanticStatus {
        enabled: p.policy.automatic_embedding,
        state: status.into(),
        profile,
        counts,
        coverage,
        batches: batches.into_iter().map(|b| b.0).collect(),
        total_batches,
        offset,
    }))
}

#[utoipa::path(post,path="/api/brains/{brain}/semantic/reindex",operation_id="reindexSemantic",params(("brain"=Uuid,Path)),request_body=SemanticReindex,responses((status=200,body=SemanticProfile)))]
pub async fn reindex(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    ResponseJson(input): ResponseJson<SemanticReindex>,
) -> Result<ResponseJson<SemanticProfile>> {
    auth.require_browser()?;
    let key = commands::key(&headers)?;
    let mut tx = admission::brain_tx(
        &state.pool,
        &auth,
        brain,
        BrainAccess::admin(Archived::Allow),
    )
    .await?
    .tx;
    if let Some(saved) = commands::reserve::<SemanticProfile>(
        &mut tx,
        key.as_deref(),
        "semantic.reindex",
        json!({"brain":brain,"input":input}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(ResponseJson(saved));
    }
    let p = model_policy::current(&state, &mut tx, brain).await?;
    model_policy::permits(&state, &p.policy, "embedding", &[])?;
    if !p.policy.automatic_embedding {
        return Err(Error::invalid(
            "Enable standing automatic embedding before rebuilding its index.",
        ));
    }
    if profile(&mut tx, brain).await?.map(|p| p.id) != input.base_profile {
        return Err(model_policy::failure(
            "semantic_profile_changed",
            "The index profile changed. Refresh before rebuilding.",
        ));
    }
    let result = rebuild(&state, &mut tx, brain, auth.user.id, &p).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    tx.commit().await?;
    Ok(ResponseJson(result))
}

#[derive(sqlx::FromRow)]
struct Candidate {
    kind: String,
    input_id: Uuid,
    source_version_id: Option<Uuid>,
    claim_revision_id: Option<Uuid>,
    fact_id: Option<Uuid>,
    manifest_revision_id: Option<Uuid>,
}

pub(super) async fn enqueue(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    profile: Uuid,
    p: &ModelPolicyVersion,
    entries: &[Uuid],
    retry: Option<(Uuid, i32)>,
) -> Result<SemanticBatch> {
    if entries.is_empty() || entries.len() > 20 {
        return Err(Error::invalid("Select 1–20 permitted semantic inputs."));
    }
    let id = Uuid::new_v4();
    let audit = db::audit(
        tx,
        actor,
        brain,
        "semantic.queue",
        id,
        if retry.is_some() {
            "retry"
        } else {
            "automatic"
        },
    )
    .await?;
    let job = jobs::enqueue_work(tx, actor, brain, audit, id, "semantic.generate", "model").await?;
    let Json(batch) = sqlx::query_scalar(
        "INSERT INTO semantic_batches(id,brain_id,profile_id,actor_id,policy_id,job_id,state,input_count,retry_of,automatic_attempt)
         VALUES($1,$2,$3,$4,$5,$6,'queued',$7,$8,$9) RETURNING to_jsonb(semantic_batches)")
        .bind(id).bind(brain).bind(profile).bind(actor).bind(p.change_id).bind(job).bind(entries.len() as i32)
        .bind(retry.map(|r|r.0)).bind(retry.map_or(0,|r|r.1)).fetch_one(&mut **tx).await?;
    for (ordinal, entry) in entries.iter().enumerate() {
        sqlx::query("INSERT INTO semantic_batch_inputs(brain_id,batch_id,entry_id,ordinal) VALUES($1,$2,$3,$4)")
            .bind(brain).bind(id).bind(entry).bind(ordinal as i32).execute(&mut **tx).await?;
    }
    sqlx::query("UPDATE semantic_entries SET state='queued',embedding=NULL,batch_id=$3,request_id=NULL,error_code=NULL,updated_at=clock_timestamp() WHERE brain_id=$1 AND id=ANY($2)")
        .bind(brain).bind(entries).bind(id).execute(&mut **tx).await?;
    Ok(batch)
}

/// What a maintenance pass decided under its first lock: the policy and profile
/// it planned against, the automatic retries and the pending entries to judge.
struct Survey {
    policy: ModelPolicyVersion,
    profile: SemanticProfile,
    retries: Vec<SemanticBatch>,
    /// Entry id with the state and timestamp it was selected in.
    pending: Vec<(Uuid, String, chrono::DateTime<chrono::Utc>)>,
    batch_limit: usize,
}

/// Judgement of one pending entry's representation. Computing it reads the
/// retained artifact and several rows, so it never runs under the Brain lock
/// unless a canonical input changed after the lock-free preparation.
enum Verdict {
    Keep {
        len: usize,
        truncated: bool,
    },
    Reject {
        code: &'static str,
        disposition: &'static str,
    },
}

struct Judged {
    /// `None` skips a batch whose inputs are no longer retryable.
    retries: Vec<Option<Vec<Uuid>>>,
    verdicts: HashMap<Uuid, Verdict>,
}

struct Prepared {
    epoch: i64,
    judged: Judged,
}

async fn judge(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    p: &ModelPolicyVersion,
    id: Uuid,
) -> Verdict {
    let limit = p.policy.max_input_bytes.min(8000) as usize;
    match representation(state, tx, brain, id).await {
        Ok(input)
            if input.text.len() <= limit
                && model_policy::permits(
                    state,
                    &p.policy,
                    "embedding",
                    std::slice::from_ref(&input.class),
                )
                .is_ok() =>
        {
            Verdict::Keep {
                len: input.text.len(),
                truncated: input.truncated,
            }
        }
        result => {
            let code = match result {
                Err(e) => e.1,
                Ok(input) if input.text.len() > limit => "semantic_input_too_large",
                _ => "model_policy_denied",
            };
            let disposition = if matches!(code, "content_removed" | "model_input_fenced") {
                "removed"
            } else {
                "blocked"
            };
            Verdict::Reject { code, disposition }
        }
    }
}

async fn judge_all(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    survey: &Survey,
    pending: &[Uuid],
) -> Result<Judged> {
    let mut retries = Vec::new();
    for old in &survey.retries {
        retries.push(
            match retry_inputs(state, tx, brain, old, &survey.policy).await {
                Ok(entries) => Some(entries),
                Err(error) if error.1 != "database_unavailable" => None,
                Err(error) => return Err(error),
            },
        );
    }
    let mut verdicts = HashMap::new();
    for id in pending {
        verdicts.insert(*id, judge(state, tx, brain, &survey.policy, *id).await);
    }
    Ok(Judged { retries, verdicts })
}

pub async fn maintain_brain(state: &AppState, brain: Uuid, actor: Uuid) -> Result<usize> {
    let Some(survey) = survey(state, brain, actor).await? else {
        return Ok(0);
    };
    if survey.retries.is_empty() && survey.pending.is_empty() {
        return Ok(0);
    }
    // A failed preparation only costs the fast path: publication judges the
    // inputs again under the lock and reports the same error.
    let prepared = prepare(state, brain, actor, &survey).await.ok();
    publish(state, brain, actor, survey, prepared).await
}

/// Short exclusive-lock pass: housekeeping, discovery and the choice of work.
async fn survey(state: &AppState, brain: Uuid, actor: Uuid) -> Result<Option<Survey>> {
    let mut tx = db::actor_tx(&state.pool, actor).await?;
    // Give up on a Brain whose admission lock is held elsewhere instead of
    // pinning this connection for the rest of the Brain's budget.
    sqlx::query("SET LOCAL lock_timeout='2s'")
        .execute(&mut *tx)
        .await?;
    db::lock_brain(&mut tx, brain, true).await?;
    db::require_writer(&mut tx, brain).await?;
    cleanup(&mut tx, brain).await?;
    let p = model_policy::current(state, &mut tx, brain).await?;
    if p.created_by != Some(actor)
        || !p.policy.automatic_embedding
        || model_policy::permits(state, &p.policy, "embedding", &[]).is_err()
    {
        tx.commit().await?;
        return Ok(None);
    }
    let profile = match profile(&mut tx, brain).await? {
        Some(profile) if compatible(&p.policy, &profile) => profile,
        Some(_) => {
            tx.commit().await?;
            return Ok(None);
        }
        None => create_profile(state, &mut tx, brain, actor, &p).await?,
    };
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM semantic_entries WHERE brain_id=$1 AND profile_id=$2",
    )
    .bind(brain)
    .bind(profile.id)
    .fetch_one(&mut *tx)
    .await?;
    let candidates = include_str!("../semantic_candidates.sql");
    let inputs: Vec<Candidate> = sqlx::query_as(&format!("SELECT kind,input_id,source_version_id,claim_revision_id,fact_id,manifest_revision_id FROM ({candidates}) c
        ORDER BY c.created_at,c.kind,c.input_id LIMIT $4"))
        .bind(brain).bind(&p.policy.content_classes).bind(profile.id).bind((ENTRY_LIMIT-count).clamp(0,100)).fetch_all(&mut *tx).await?;
    for input in &inputs {
        sqlx::query("INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id,claim_revision_id,fact_id,manifest_revision_id)
            VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT(profile_id,kind,input_id) DO NOTHING")
            .bind(Uuid::new_v4()).bind(brain).bind(profile.id).bind(&input.kind).bind(input.input_id)
            .bind((input.kind=="source_chunk").then_some(input.input_id)).bind(input.source_version_id)
            .bind(input.claim_revision_id).bind(input.fact_id).bind(input.manifest_revision_id).execute(&mut *tx).await?;
    }
    if !inputs.is_empty() {
        db::audit(
            &mut tx,
            actor,
            brain,
            "semantic.discover",
            profile.id,
            "pending",
        )
        .await?;
    }
    let pending_jobs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM jobs WHERE brain_id=$1 AND state IN ('queued','running')",
    )
    .bind(brain)
    .fetch_one(&mut *tx)
    .await?;
    let batch_limit = (500 - pending_jobs).clamp(0, 10) as usize;
    let retries = retry_batches(&mut tx, brain, &profile, &p, batch_limit).await?;
    let pending = if batch_limit == 0 {
        vec![]
    } else {
        sqlx::query_as("SELECT e.id,e.state,e.updated_at FROM semantic_entries e WHERE brain_id=$1 AND profile_id=$2 AND (state='pending' OR (state='blocked' AND request_id IS NULL AND error_code<>'batch_input_removed' AND updated_at<clock_timestamp()-interval '1 minute' AND (batch_id IS NULL OR EXISTS(SELECT 1 FROM semantic_batches b WHERE b.id=e.batch_id AND b.state='blocked' AND b.request_id IS NULL AND (e.error_code<>'model_budget_exhausted' OR b.policy_id<>$3 OR e.updated_at<(date_trunc('day',clock_timestamp() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC')))))) ORDER BY updated_at,id LIMIT 100")
            .bind(brain).bind(profile.id).bind(p.change_id).fetch_all(&mut *tx).await?
    };
    tx.commit().await?;
    Ok(Some(Survey {
        policy: p,
        profile,
        retries,
        pending,
        batch_limit,
    }))
}

/// Judge the surveyed work in a repeatable-read, read-only transaction that
/// takes no Brain lock. The epoch read in the same snapshot lets `publish` prove
/// no canonical input changed before it trusts the result.
async fn prepare(state: &AppState, brain: Uuid, actor: Uuid, survey: &Survey) -> Result<Prepared> {
    let mut tx =
        admission::brain_preparation_tx(&state.pool, actor, None, brain, BrainAccess::READ)
            .await?
            .tx;
    let epoch = crate::graph::preparation_epoch(&mut tx, brain).await?;
    let ids: Vec<Uuid> = survey.pending.iter().map(|e| e.0).collect();
    let judged = judge_all(state, &mut tx, brain, survey, &ids).await?;
    Ok(Prepared { epoch, judged })
}

/// Short exclusive-lock pass: re-check what the survey and preparation assumed,
/// then enqueue batches and record the rejected entries.
async fn publish(
    state: &AppState,
    brain: Uuid,
    actor: Uuid,
    survey: Survey,
    prepared: Option<Prepared>,
) -> Result<usize> {
    let mut tx = db::actor_tx(&state.pool, actor).await?;
    sqlx::query("SET LOCAL lock_timeout='2s'")
        .execute(&mut *tx)
        .await?;
    db::lock_brain(&mut tx, brain, true).await?;
    db::require_writer(&mut tx, brain).await?;
    let (p, profile) = (&survey.policy, &survey.profile);
    // A changed policy or profile head replans on the next pass.
    if model_policy::current(state, &mut tx, brain)
        .await?
        .change_id
        != p.change_id
        || self::profile(&mut tx, brain).await?.map(|h| h.id) != Some(profile.id)
    {
        tx.commit().await?;
        return Ok(0);
    }
    // Entries another worker or a rebuild touched since the survey are skipped.
    let ids: Vec<Uuid> = survey.pending.iter().map(|e| e.0).collect();
    let current: Vec<(Uuid, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT id,state,updated_at FROM semantic_entries WHERE brain_id=$1 AND id=ANY($2)",
    )
    .bind(brain)
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await?;
    let unchanged: HashSet<Uuid> = survey
        .pending
        .iter()
        .filter(|surveyed| current.iter().any(|c| c == *surveyed))
        .map(|e| e.0)
        .collect();
    let pending: Vec<Uuid> = ids
        .into_iter()
        .filter(|id| unchanged.contains(id))
        .collect();
    let judged = match prepared {
        Some(prepared)
            if crate::graph::preparation_epoch(&mut tx, brain).await? == prepared.epoch =>
        {
            prepared.judged
        }
        // Something canonical changed since the snapshot (or preparation
        // failed). Judging again under the lock keeps a busy Brain from starving.
        _ => judge_all(state, &mut tx, brain, &survey, &pending).await?,
    };
    let mut retried = 0;
    for (old, entries) in survey.retries.iter().zip(judged.retries) {
        let Some(entries) = entries else { continue };
        let open: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM semantic_batches b WHERE b.brain_id=$1 AND b.id=$2 AND b.state='failed' AND NOT EXISTS(SELECT 1 FROM semantic_batches c WHERE c.retry_of=b.id))")
            .bind(brain).bind(old.id).fetch_one(&mut *tx).await?;
        if !open || retry_candidates(&mut tx, brain, old.id).await? != entries {
            continue;
        }
        enqueue(
            &mut tx,
            brain,
            actor,
            profile.id,
            p,
            &entries,
            Some((old.id, old.automatic_attempt + 1)),
        )
        .await?;
        retried += 1;
    }
    let batch_limit = survey.batch_limit - retried;
    let mut batch = Vec::new();
    let mut bytes = 0;
    let mut queued = 0;
    let limit = p.policy.max_input_bytes.min(8000) as usize;
    for id in pending {
        if batch_limit == 0 {
            break;
        }
        let Some(verdict) = judged.verdicts.get(&id) else {
            continue;
        };
        let (len, truncated) = match verdict {
            Verdict::Keep { len, truncated } => (*len, *truncated),
            Verdict::Reject { code, disposition } => {
                sqlx::query("UPDATE semantic_entries SET state=$3,embedding=NULL,error_code=$2,updated_at=clock_timestamp() WHERE id=$1")
                    .bind(id).bind(*code).bind(*disposition).execute(&mut *tx).await?;
                continue;
            }
        };
        if !batch.is_empty() && (batch.len() == 20 || bytes + len > limit) {
            enqueue(&mut tx, brain, actor, profile.id, p, &batch, None).await?;
            queued += 1;
            batch.clear();
            bytes = 0;
            if queued >= batch_limit {
                break;
            }
        }
        sqlx::query("UPDATE semantic_entries SET truncated=$2 WHERE id=$1")
            .bind(id)
            .bind(truncated)
            .execute(&mut *tx)
            .await?;
        bytes += len;
        batch.push(id);
    }
    if !batch.is_empty() && queued < batch_limit {
        enqueue(&mut tx, brain, actor, profile.id, p, &batch, None).await?;
        queued += 1;
    }
    tx.commit().await?;
    Ok(queued + retried)
}

async fn cleanup(tx: &mut Tx<'_>, brain: Uuid) -> Result<()> {
    sqlx::query("UPDATE semantic_entries SET state='removed',embedding=NULL,error_code='semantic_profile_obsolete',updated_at=clock_timestamp() WHERE id IN (SELECT e.id FROM semantic_entries e JOIN semantic_heads h ON h.brain_id=e.brain_id WHERE e.brain_id=$1 AND e.profile_id<>h.profile_id AND e.state<>'removed' ORDER BY e.updated_at,e.id LIMIT 100)")
        .bind(brain).execute(&mut **tx).await?;
    sqlx::query("UPDATE semantic_batches SET state='removed',error_code='semantic_profile_obsolete',finished_at=clock_timestamp() WHERE id IN (SELECT b.id FROM semantic_batches b JOIN semantic_heads h ON h.brain_id=b.brain_id WHERE b.brain_id=$1 AND b.profile_id<>h.profile_id AND b.state IN ('queued','running','blocked') ORDER BY b.created_at,b.id LIMIT 100)")
        .bind(brain).execute(&mut **tx).await?;
    sqlx::query("UPDATE jobs SET state='cancelled',error_code='semantic_profile_obsolete',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE id IN (SELECT j.id FROM jobs j JOIN semantic_batches b ON b.job_id=j.id WHERE b.brain_id=$1 AND b.state='removed' AND j.state IN ('queued','running') LIMIT 100)")
        .bind(brain).execute(&mut **tx).await?;
    Ok(())
}

async fn retry_candidates(tx: &mut Tx<'_>, brain: Uuid, batch: Uuid) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar("SELECT e.id FROM semantic_batch_inputs i JOIN semantic_entries e ON e.id=i.entry_id AND e.brain_id=i.brain_id WHERE i.brain_id=$1 AND i.batch_id=$2 AND e.batch_id=i.batch_id AND e.state IN ('blocked','failed') ORDER BY i.ordinal")
        .bind(brain).bind(batch).fetch_all(&mut **tx).await?)
}

async fn retry_inputs(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    old: &SemanticBatch,
    p: &ModelPolicyVersion,
) -> Result<Vec<Uuid>> {
    let ids = retry_candidates(tx, brain, old.id).await?;
    let mut valid = Vec::new();
    let mut bytes = 0;
    for id in ids {
        let input = representation(state, tx, brain, id).await?;
        model_policy::permits(
            state,
            &p.policy,
            "embedding",
            std::slice::from_ref(&input.class),
        )?;
        bytes += input.text.len();
        valid.push(id);
    }
    if valid.is_empty() || bytes > p.policy.max_input_bytes.min(8000) as usize {
        return Err(model_policy::failure(
            "semantic_retry_unavailable",
            "This batch has no remaining permitted inputs or exceeds the current input allowance.",
        ));
    }
    Ok(valid)
}

#[utoipa::path(post,path="/api/brains/{brain}/semantic/batches/{batch}/retry",operation_id="retrySemanticBatch",params(("brain"=Uuid,Path),("batch"=Uuid,Path)),responses((status=200,body=SemanticBatch)))]
pub async fn retry(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<ResponseJson<SemanticBatch>> {
    let key = commands::key(&headers)?;
    let mut tx = admission::brain_tx(&state.pool, &auth, brain, BrainAccess::WRITE)
        .await?
        .tx;
    if let Some(saved) = commands::reserve::<SemanticBatch>(
        &mut tx,
        key.as_deref(),
        "semantic.retry",
        json!({"brain":brain,"batch":id}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(ResponseJson(saved));
    }
    let old = batch(&mut tx, brain, id).await?;
    let p = model_policy::current(&state, &mut tx, brain).await?;
    model_policy::permits(&state, &p.policy, "embedding", &[])?;
    if !p.policy.automatic_embedding {
        return Err(model_policy::denied());
    }
    let head = profile(&mut tx, brain).await?.ok_or_else(Error::missing)?;
    let has_child: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM semantic_batches WHERE brain_id=$1 AND retry_of=$2)",
    )
    .bind(brain)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if !matches!(old.state.as_str(), "blocked" | "failed" | "removed")
        || has_child
        || head.id != old.profile_id
        || !compatible(&p.policy, &head)
    {
        return Err(model_policy::failure(
            "semantic_not_retryable",
            "Refresh and select an unsuccessful batch in the active profile that has no replacement attempt.",
        ));
    }
    let entries = retry_inputs(&state, &mut tx, brain, &old, &p).await?;
    let result = enqueue(
        &mut tx,
        brain,
        auth.user.id,
        head.id,
        &p,
        &entries,
        Some((id, old.automatic_attempt)),
    )
    .await?;
    commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    tx.commit().await?;
    Ok(ResponseJson(result))
}

async fn retry_batches(
    tx: &mut Tx<'_>,
    brain: Uuid,
    profile: &SemanticProfile,
    p: &ModelPolicyVersion,
    limit: usize,
) -> Result<Vec<SemanticBatch>> {
    let batches: Vec<Json<SemanticBatch>> = sqlx::query_scalar("SELECT to_jsonb(b) FROM semantic_batches b JOIN model_requests r ON r.brain_id=b.brain_id AND r.id=b.request_id
        WHERE b.brain_id=$1 AND b.profile_id=$2 AND b.policy_id=$3 AND b.state='failed' AND r.state='failed' AND NOT r.suppressed
        AND b.error_code IN ('provider_rate_limited','provider_unavailable') AND b.automatic_attempt<2
        AND b.finished_at+make_interval(mins=>CASE b.automatic_attempt WHEN 0 THEN 5 ELSE 30 END)<=clock_timestamp()
        AND NOT EXISTS(SELECT 1 FROM semantic_batches c WHERE c.retry_of=b.id)
        ORDER BY b.finished_at,b.id LIMIT $4")
        .bind(brain).bind(profile.id).bind(p.change_id).bind(limit as i64).fetch_all(&mut **tx).await?;
    Ok(batches.into_iter().map(|Json(b)| b).collect())
}

pub async fn run_once(state: &AppState) -> Result<usize> {
    let brains: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT brain_id,actor_id FROM recollect_semantic_brains()")
            .fetch_all(&state.pool)
            .await?;
    Ok(
        crate::worker::each_brain("semantic", brains, |brain, actor| {
            maintain_brain(state, brain, actor)
        })
        .await,
    )
}
