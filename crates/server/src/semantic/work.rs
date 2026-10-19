use super::*;
use crate::db::admission::{self, BrainAccess};
use crate::{
    db,
    worker::{self, ClaimedJob, Failure},
};
use axum::http::StatusCode;
use recollect_protocol::{ModelPolicyVersion, SemanticBatch};

async fn lease(tx: &mut Tx<'_>, job: &ClaimedJob) -> Result<()> {
    let valid: Option<bool> = sqlx::query_scalar("SELECT lease_until>clock_timestamp() FROM jobs WHERE id=$1 AND brain_id=$2 AND state='running' AND lease_token=$3 FOR UPDATE")
        .bind(job.id).bind(job.brain_id).bind(job.lease_token).fetch_optional(&mut **tx).await?;
    if valid != Some(true) {
        return Err(model_policy::failure(
            "semantic_lease_lost",
            "This worker no longer owns its semantic batch.",
        ));
    }
    Ok(())
}

/// Everything about a batch that is cheap to check and must be current at the
/// moment of the check: the lease, batch, policy, profile and its input rows.
async fn inspect(
    state: &AppState,
    job: &ClaimedJob,
    tx: &mut Tx<'_>,
) -> Result<(SemanticBatch, ModelPolicyVersion, Vec<Uuid>)> {
    let batch = queue::batch(tx, job.brain_id, job.target_id).await?;
    if batch.job_id != job.id
        || batch.actor_id != job.actor_id
        || !matches!(batch.state.as_str(), "queued" | "running")
    {
        return Err(model_policy::failure(
            "semantic_batch_changed",
            "This batch no longer accepts model output.",
        ));
    }
    let policy = model_policy::current(state, tx, job.brain_id).await?;
    if policy.change_id != batch.policy_id {
        return Err(model_policy::failure(
            "model_policy_changed",
            "The model policy changed after this batch was queued.",
        ));
    }
    if !policy.policy.automatic_embedding {
        return Err(model_policy::denied());
    }
    model_policy::permits(state, &policy.policy, "embedding", &[])?;
    let profile = profile(tx, job.brain_id)
        .await?
        .ok_or_else(Error::missing)?;
    if profile.id != batch.profile_id || !compatible(&policy.policy, &profile) {
        return Err(model_policy::failure(
            "semantic_profile_mismatch",
            "The active semantic profile changed.",
        ));
    }
    let entries: Vec<Uuid> = sqlx::query_scalar("SELECT e.id FROM semantic_batch_inputs i JOIN semantic_entries e ON e.id=i.entry_id AND e.brain_id=i.brain_id WHERE i.brain_id=$1 AND i.batch_id=$2 AND e.batch_id=i.batch_id AND e.profile_id=$3 AND e.state IN ('queued','running') ORDER BY i.ordinal")
        .bind(job.brain_id).bind(batch.id).bind(profile.id).fetch_all(&mut **tx).await?;
    if entries.len() != batch.input_count as usize {
        return Err(model_policy::failure(
            "semantic_input_changed",
            "A batch input was removed or assigned to another attempt.",
        ));
    }
    Ok((batch, policy, entries))
}

/// The expensive part: every representation reads the retained artifact and
/// several rows. Read-only, so it can run in a lock-free snapshot.
async fn inspect_inputs(
    state: &AppState,
    job: &ClaimedJob,
    tx: &mut Tx<'_>,
    policy: &ModelPolicyVersion,
    entries: &[Uuid],
) -> Result<()> {
    let mut bytes = 0usize;
    for id in entries {
        let input = representation(state, tx, job.brain_id, *id).await?;
        model_policy::permits(
            state,
            &policy.policy,
            "embedding",
            std::slice::from_ref(&input.class),
        )?;
        bytes += input.text.len();
    }
    if bytes > policy.policy.max_input_bytes.min(8000) as usize {
        return Err(model_policy::failure(
            "semantic_input_too_large",
            "This batch exceeds the current model input allowance.",
        ));
    }
    Ok(())
}

/// A lock-free, repeatable-read check of the batch inputs. Only its success is
/// used, and only if the canonical-input epoch is still the same under the lock.
async fn prepare(
    state: &AppState,
    job: &ClaimedJob,
) -> Result<(i64, ModelPolicyVersion, Vec<Uuid>)> {
    let mut tx = admission::brain_preparation_tx(
        &state.pool,
        job.actor_id,
        job.device_id,
        job.brain_id,
        BrainAccess::READ,
    )
    .await?
    .tx;
    let epoch = crate::graph::preparation_epoch(&mut tx, job.brain_id).await?;
    let (_, policy, entries) = inspect(state, job, &mut tx).await?;
    inspect_inputs(state, job, &mut tx, &policy, &entries).await?;
    Ok((epoch, policy, entries))
}

/// Take the exclusive Brain lock only after the representations were checked
/// without it. If a canonical input changed in between (or the snapshot check
/// failed), check them again under the lock exactly as before, so a failure or
/// a race can never publish what the locked check would have refused.
async fn ready<'a>(
    state: &'a AppState,
    job: &ClaimedJob,
) -> Result<(Tx<'a>, SemanticBatch, Vec<Uuid>)> {
    let prepared = prepare(state, job).await.ok();
    let mut tx = admission::brain_tx_as(
        &state.pool,
        job.actor_id,
        job.device_id,
        job.brain_id,
        BrainAccess::WRITE,
    )
    .await?
    .tx;
    lease(&mut tx, job).await?;
    let (batch, policy, entries) = inspect(state, job, &mut tx).await?;
    let proven = match prepared {
        Some((epoch, prepared_policy, prepared_entries)) => {
            prepared_policy.change_id == policy.change_id
                && prepared_entries == entries
                && crate::graph::preparation_epoch(&mut tx, job.brain_id).await? == epoch
        }
        None => false,
    };
    if !proven {
        inspect_inputs(state, job, &mut tx, &policy, &entries).await?;
    }
    Ok((tx, batch, entries))
}

async fn run_job(
    state: &AppState,
    job: &ClaimedJob,
    preview: Option<&gateway::Preview>,
) -> Result<()> {
    let (mut tx, batch, entries) = ready(state, job).await?;
    sqlx::query("UPDATE semantic_batches SET state='running' WHERE id=$1")
        .bind(batch.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE semantic_entries SET state='running',updated_at=clock_timestamp() WHERE brain_id=$1 AND batch_id=$2")
        .bind(job.brain_id).bind(batch.id).execute(&mut *tx).await?;
    tx.commit().await?;
    // The gateway acquires its own short authorization transaction. No job or
    // Brain lock is held across outbound I/O, and one batch is one paid attempt.
    let context = gateway::Context {
        brain: job.brain_id,
        actor: job.actor_id,
        device: job.device_id,
    };
    let invocation = gateway::Invocation {
        operation: batch.id,
        purpose: "embedding".into(),
        inputs: entries
            .iter()
            .map(|id| InputRef {
                kind: "semantic_entry".into(),
                id: *id,
            })
            .collect(),
        query: None,
        instructions: String::new(),
        prompt_label: REPRESENTATION.into(),
        schema_label: "embedding-float-1".into(),
        format: gateway::Format::Embedding,
        work_lease: None,
        metadata_replay: false,
        expected_json: None,
    }
    .with_work_lease(job);
    let response = if let Some(preview) = preview {
        if preview.policy_id != batch.policy_id {
            return Err(model_policy::failure(
                "model_policy_changed",
                "The preview belongs to another semantic policy.",
            ));
        }
        gateway::invoke_previewed_for_policy(state, context, invocation, preview).await?
    } else {
        gateway::invoke_for_policy(state, context, invocation, batch.policy_id).await?
    };
    let Some(gateway::Output::Embeddings(vectors)) = response.output else {
        return Err(model_policy::failure(
            "model_result_not_retained",
            "This attempt has no retained vector output. Start an explicit new attempt.",
        ));
    };
    if vectors.len() != entries.len() {
        return Err(model_policy::failure(
            "provider_shape",
            "The provider returned an incomplete embedding batch.",
        ));
    }
    let (mut tx, current, current_entries) = ready(state, job).await?;
    let profile = profile(&mut tx, job.brain_id)
        .await?
        .ok_or_else(Error::missing)?;
    if current_entries != entries {
        return Err(model_policy::failure(
            "semantic_input_changed",
            "The batch inputs changed during model execution.",
        ));
    }
    let returned: Option<Option<String>> = sqlx::query_scalar("SELECT returned_model FROM model_requests WHERE brain_id=$1 AND id=$2 AND state='succeeded' AND NOT suppressed AND policy_id=$3 AND model=$4 AND dimensions=$5 AND provider=$6")
        .bind(job.brain_id).bind(response.request.id).bind(current.policy_id)
        .bind(&profile.model).bind(profile.dimensions).bind(&profile.provider).fetch_optional(&mut *tx).await?;
    let usable = returned.flatten().is_some_and(|returned| {
        model_policy::catalogue::returned_model_matches(
            &profile.provider,
            &profile.model,
            &returned,
            true,
        )
    });
    if !usable {
        return Err(model_policy::failure(
            "model_result_not_retained",
            "The model request is no longer eligible for publication.",
        ));
    }
    let deadline: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar(include_str!("../semantic_deadline.sql"))
            .bind(job.brain_id)
            .bind(&entries)
            .fetch_one(&mut *tx)
            .await?;
    for (id, vector) in entries.iter().zip(vectors) {
        // Floats and dimensions were validated by the gateway. Passing a bound
        // vector literal avoids a second client representation or lossy storage.
        let vector = serde_json::to_string(&vector)
            .map_err(|_| Error::invalid("Invalid embedding vector."))?;
        sqlx::query("UPDATE semantic_entries SET embedding=$4::text::vector,state='ready',request_id=$3,error_code=NULL,updated_at=clock_timestamp() WHERE brain_id=$1 AND id=$2")
            .bind(job.brain_id).bind(id).bind(response.request.id).bind(vector).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE semantic_batches SET state='succeeded',request_id=$2,error_code=NULL,finished_at=clock_timestamp() WHERE id=$1")
        .bind(batch.id).bind(response.request.id).execute(&mut *tx).await?;
    db::audit(
        &mut tx,
        job.actor_id,
        job.brain_id,
        "semantic.publish",
        batch.id,
        "succeeded",
    )
    .await?;
    lease(&mut tx, job).await?;
    sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE id=$1 AND lease_token=$2")
        .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
    let retained: bool =
        sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
            .bind(deadline)
            .fetch_one(&mut *tx)
            .await?;
    if !retained {
        return Err(model_policy::failure(
            "semantic_input_expired",
            "A required input expired during publication; the whole batch was discarded.",
        ));
    }
    tx.commit().await?;
    Ok(())
}

pub async fn execute(state: &AppState, job: &ClaimedJob) -> std::result::Result<(), Failure> {
    execute_inner(state, job, None).await
}
/// The normal canonical worker with a frozen internal request preview. All
/// lease, profile, input, deadline and publication checks remain identical.
pub async fn execute_previewed(
    state: &AppState,
    job: &ClaimedJob,
    preview: &gateway::Preview,
) -> std::result::Result<(), Failure> {
    execute_inner(state, job, Some(preview)).await
}
async fn execute_inner(
    state: &AppState,
    job: &ClaimedJob,
    preview: Option<&gateway::Preview>,
) -> std::result::Result<(), Failure> {
    let result = worker::with_lease(&state.pool, job, run_job(state, job, preview)).await?;
    if let Err(error) = result {
        let reason = if matches!(
            error.0,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        ) {
            "permission_or_policy_denied"
        } else {
            error.1
        };
        tracing::warn!(job_id=%job.id,code=reason,"Semantic batch did not publish");
        sqlx::query("SELECT recollect_fail_semantic_job($1,$2,$3)")
            .bind(job.id)
            .bind(job.lease_token)
            .bind(reason)
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}
