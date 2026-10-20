//! Restart-safe bounded deterministic field preparation and leased publication.
use super::*;
use crate::db::admission::{self, BrainAccess, Role};
use crate::{
    jobs,
    worker::{self, ClaimedJob, Failure},
};

const BATCH: usize = 25;

#[derive(FromRow)]
struct Candidate {
    logical_id: Uuid,
    revision_id: Uuid,
    evidence_kind: String,
    evidence_id: Uuid,
    evidence_revision: Uuid,
}

fn phase_kind(phase: i32) -> Result<&'static str> {
    Ok(match phase {
        0 => "source_context",
        1 => "claim_revision",
        2 => "repository_snapshot",
        3 => "manifest_revision",
        _ => return Err(changed()),
    })
}
async fn candidates(
    tx: &mut Tx<'_>,
    brain: Uuid,
    phase: i32,
    after: Option<Uuid>,
) -> Result<Vec<Candidate>> {
    let branch = match phase {
        0 => {
            "SELECT s.id AS logical_id,v.id AS revision_id,'source_version'::text AS evidence_kind,s.id AS evidence_id,v.id AS evidence_revision FROM sources s JOIN source_versions v ON v.brain_id=s.brain_id AND v.id=s.current_version WHERE s.brain_id=$1 AND ($2::uuid IS NULL OR s.id>$2)"
        }
        1 => {
            "SELECT c.id AS logical_id,r.id AS revision_id,'claim'::text AS evidence_kind,c.id AS evidence_id,r.id AS evidence_revision FROM claims c JOIN claim_revisions r ON r.brain_id=c.brain_id AND r.id=c.current_revision WHERE c.brain_id=$1 AND ($2::uuid IS NULL OR c.id>$2)"
        }
        2 => {
            "SELECT DISTINCT ON(s.repository_id) s.repository_id AS logical_id,s.id AS revision_id,'repository_fact'::text AS evidence_kind,f.id AS evidence_id,f.id AS evidence_revision FROM repository_snapshots s JOIN LATERAL(SELECT id FROM repository_facts WHERE brain_id=s.brain_id AND snapshot_id=s.id ORDER BY id LIMIT 1) f ON true WHERE s.brain_id=$1 AND ($2::uuid IS NULL OR s.repository_id>$2) ORDER BY s.repository_id,s.created_at DESC,s.id DESC"
        }
        3 => {
            "SELECT m.id AS logical_id,r.id AS revision_id,'manifest_revision'::text AS evidence_kind,r.id AS evidence_id,r.id AS evidence_revision FROM revision_manifests m JOIN manifest_revisions r ON r.brain_id=m.brain_id AND r.id=m.current_revision WHERE m.brain_id=$1 AND ($2::uuid IS NULL OR m.id>$2)"
        }
        _ => return Ok(vec![]),
    };
    Ok(sqlx::query_as(&format!(
        "SELECT * FROM ({branch}) x ORDER BY logical_id LIMIT {}",
        BATCH + 1
    ))
    .bind(brain)
    .bind(after)
    .fetch_all(&mut **tx)
    .await?)
}
async fn lease(tx: &mut Tx<'_>, job: &ClaimedJob) -> Result<()> {
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE brain_id=$1 AND id=$2 AND target_id=$1 AND kind='knowledge.fields' AND actor_id=$3 AND device_id IS NOT DISTINCT FROM $4 AND state='running' AND lease_token=$5 AND lease_until>clock_timestamp())")
        .bind(job.brain_id).bind(job.id).bind(job.actor_id).bind(job.device_id).bind(job.lease_token).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(Error(
            StatusCode::CONFLICT,
            "field_lease_lost",
            "Canonical field worker lost its lease.",
        ));
    }
    Ok(())
}
async fn publish(state: &AppState, job: &ClaimedJob) -> Result<()> {
    let mut tx = admission::brain_preparation_tx(
        &state.pool,
        job.actor_id,
        job.device_id,
        job.brain_id,
        BrainAccess::READ.with_role(Role::Writer),
    )
    .await?
    .tx;
    lease(&mut tx, job).await?;
    let (revision,phase,after):(i64,i32,Option<Uuid>)=sqlx::query_as("SELECT revision,phase,after_id FROM knowledge_field_outbox WHERE brain_id=$1 AND job_id=$2").bind(job.brain_id).bind(job.id).fetch_one(&mut *tx).await?;
    let phase = if phase == 4 { 0 } else { phase };
    let epoch = graph::preparation_epoch(&mut tx, job.brain_id).await?;
    let mut batch = candidates(&mut tx, job.brain_id, phase, after).await?;
    let more = batch.len() > BATCH;
    batch.truncate(BATCH);
    let last = batch.last().map(|c| c.logical_id);
    let user: SqlJson<User> =
        sqlx::query_scalar("SELECT to_jsonb(a) FROM accounts a WHERE id=$1 AND enabled")
            .bind(job.actor_id)
            .fetch_one(&mut *tx)
            .await?;
    let auth = Auth {
        user: user.0,
        device_id: job.device_id,
        session: Uuid::nil(),
        csrf: Uuid::nil(),
    };
    let mut scope: GraphSelection =
        serde_json::from_value(json!({"kind":"knowledge"})).map_err(|_| changed())?;
    let entities: Vec<_> = batch
        .iter()
        .map(|c| graph::Entity {
            kind: c.evidence_kind.clone(),
            id: c.evidence_id,
            revision_id: c.evidence_revision,
        })
        .collect();
    let qualified = retrieval::graph::qualify(
        state,
        &mut tx,
        &auth,
        job.brain_id,
        &mut scope,
        &entities,
        (BATCH, None),
    )
    .await?;
    let kind = phase_kind(phase)?;
    let mut prepared = vec![];
    for c in &batch {
        let context: SqlJson<Value> =
            sqlx::query_scalar("SELECT recollect_field_context($1,$2,$3)")
                .bind(job.brain_id)
                .bind(kind)
                .bind(c.revision_id)
                .fetch_one(&mut *tx)
                .await?;
        let key = graph::key(&c.evidence_kind, c.evidence_revision);
        let fenced = if phase == 0 {
            sqlx::query_scalar::<_, bool>("SELECT recollect_source_model_fenced($1,$2)")
                .bind(job.brain_id)
                .bind(c.revision_id)
                .fetch_one(&mut *tx)
                .await?
        } else {
            false
        };
        let refs = if qualified.nodes.contains_key(&key) && !fenced {
            references(kind, &context)?
        } else {
            vec![]
        };
        prepared.push((
            c.logical_id,
            c.revision_id,
            context.0,
            c.evidence_kind.clone(),
            c.evidence_id,
            c.evidence_revision,
            refs,
            qualified.nodes.contains_key(&key) && !fenced,
        ));
    }
    tx.commit().await?;
    let mut tx = admission::brain_tx_as(
        &state.pool,
        job.actor_id,
        job.device_id,
        job.brain_id,
        BrainAccess::WRITE
            .with_statement_timeout(Duration::from_secs(2))
            .with_lock_timeout(Duration::from_millis(250)),
    )
    .await?
    .tx;
    sqlx::query("SELECT id FROM jobs WHERE id=$1 AND lease_token=$2 FOR UPDATE")
        .bind(job.id)
        .bind(job.lease_token)
        .fetch_optional(&mut *tx)
        .await?;
    lease(&mut tx, job).await?;
    graph::preparation_unchanged(&mut tx, job.brain_id, epoch).await?;
    let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM knowledge_field_outbox WHERE brain_id=$1 AND revision=$2 AND job_id=$3)").bind(job.brain_id).bind(revision).bind(job.id).fetch_one(&mut *tx).await?;
    if !current || qualified.deadline.is_some_and(|at| at <= Utc::now()) {
        return Err(changed());
    }
    let mut modified = false;
    for (
        logical,
        version,
        context,
        evidence_kind,
        evidence_id,
        evidence_revision,
        refs,
        eligible,
    ) in prepared
    {
        let previous:Option<(Uuid,Uuid,SqlJson<Value>)>=sqlx::query_as("SELECT id,revision_id,context FROM knowledge_field_inputs WHERE brain_id=$1 AND origin_kind=$2 AND logical_id=$3").bind(job.brain_id).bind(kind).bind(logical).fetch_optional(&mut *tx).await?;
        if let Some((id, old_version, SqlJson(old_context))) = &previous {
            if *old_version == version && old_context == &context && eligible {
                let still_current: bool =
                    sqlx::query_scalar("SELECT recollect_field_input_current($1,$2)")
                        .bind(job.brain_id)
                        .bind(id)
                        .fetch_one(&mut *tx)
                        .await?;
                if still_current {
                    continue;
                }
            }
        }
        if previous.is_some() {
            sqlx::query("DELETE FROM knowledge_field_inputs WHERE brain_id=$1 AND origin_kind=$2 AND logical_id=$3").bind(job.brain_id).bind(kind).bind(logical).execute(&mut *tx).await?;
            modified = true;
        }
        if !eligible {
            continue;
        }
        let input = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_field_inputs(id,brain_id,origin_kind,logical_id,revision_id,context,evidence_kind,evidence_id,evidence_revision,created_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
            .bind(input).bind(job.brain_id).bind(kind).bind(logical).bind(version).bind(SqlJson(&context)).bind(&evidence_kind).bind(evidence_id).bind(evidence_revision).bind(job.actor_id).execute(&mut *tx).await?;
        let current: bool = sqlx::query_scalar("SELECT recollect_field_input_current($1,$2)")
            .bind(job.brain_id)
            .bind(input)
            .fetch_one(&mut *tx)
            .await?;
        if !current {
            return Err(changed());
        }
        for r in refs {
            let entity = Uuid::new_v4();
            if r.kind == "repository" {
                sqlx::query("INSERT INTO knowledge_field_entities(id,brain_id,kind,repository_id) SELECT $1,$2,'repository',id FROM repositories WHERE brain_id=$2 AND id=$3 ON CONFLICT(brain_id,repository_id) WHERE repository_id IS NOT NULL DO NOTHING").bind(entity).bind(job.brain_id).bind(r.value).execute(&mut *tx).await?;
            } else {
                sqlx::query("INSERT INTO knowledge_field_entities(id,brain_id,kind,environment_id) SELECT $1,$2,'environment',id FROM evidence_groups WHERE brain_id=$2 AND id=$3 AND kind='environment' ON CONFLICT(brain_id,environment_id) WHERE environment_id IS NOT NULL DO NOTHING").bind(entity).bind(job.brain_id).bind(r.value).execute(&mut *tx).await?;
            }
            sqlx::query("INSERT INTO knowledge_field_references(id,brain_id,input_id,entity_id,field_pointer,value) SELECT $1,$2,$3,id,$4,$5 FROM knowledge_field_entities WHERE brain_id=$2 AND kind=$6 AND coalesce(repository_id,environment_id)=$5")
                .bind(Uuid::new_v4()).bind(job.brain_id).bind(input).bind(&r.pointer).bind(r.value).bind(r.kind).execute(&mut *tx).await?;
        }
        modified = true;
    }
    sqlx::query("DELETE FROM knowledge_field_entities e WHERE e.brain_id=$1 AND NOT EXISTS(SELECT 1 FROM knowledge_field_references r WHERE r.brain_id=e.brain_id AND r.entity_id=e.id)").bind(job.brain_id).execute(&mut *tx).await?;
    let next_phase = if more { phase } else { phase + 1 };
    sqlx::query("UPDATE knowledge_field_outbox SET phase=$2,after_id=$3,handled_revision=CASE WHEN $2=4 THEN revision ELSE handled_revision END,state=CASE WHEN $2=4 THEN 'ready' ELSE 'pending' END,deadline=CASE WHEN deadline IS NULL OR deadline<=clock_timestamp() THEN $4 ELSE least(deadline,$4) END,job_id=NULL,updated_at=clock_timestamp() WHERE brain_id=$1 AND revision=$5 AND job_id=$6")
        .bind(job.brain_id).bind(next_phase).bind(if more{last}else{None}).bind(qualified.deadline).bind(revision).bind(job.id).execute(&mut *tx).await?;
    if modified || next_phase == 4 {
        db::audit(
            &mut tx,
            job.actor_id,
            job.brain_id,
            "knowledge.fields.publish",
            job.brain_id,
            "canonical_typed_references",
        )
        .await?;
    }
    // Only changed typed references are analytical input. A refresh that a
    // no-op statement on a watched table queued completes without changes and
    // must leave current reports valid (graph-analytics: job progress does not
    // advance the analytics epoch).
    if modified {
        sqlx::query("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1")
            .bind(job.brain_id)
            .execute(&mut *tx)
            .await?;
    }
    let done=sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE id=$1 AND state='running' AND lease_token=$2 AND lease_until>clock_timestamp()")
        .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
    if done.rows_affected() != 1 {
        return Err(changed());
    }
    tx.commit().await?;
    Ok(())
}
pub async fn execute(state: &AppState, job: &ClaimedJob) -> std::result::Result<(), Failure> {
    let outcome = worker::with_lease(&state.pool, job, publish(state, job)).await?;
    match outcome {
        Ok(()) => Ok(()),
        Err(e) => {
            tracing::debug!(job_id=%job.id,code=e.1,"Canonical field publication deferred");
            Err(if e.1 == "field_lease_lost" {
                Failure::LostLease
            } else if e.0 == StatusCode::FORBIDDEN
                || e.0 == StatusCode::UNAUTHORIZED
                || e.0 == StatusCode::NOT_FOUND
            {
                Failure::Revoked
            } else {
                Failure::Database
            })
        }
    }
}
pub async fn run_once(state: &AppState) -> Result<usize> {
    let routing: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT brain_id,actor_id FROM recollect_field_brains()")
            .fetch_all(&state.pool)
            .await?;
    let mut count = 0;
    for (brain, actor) in routing {
        let mut tx = admission::brain_tx_as(&state.pool, actor, None, brain, BrainAccess::WRITE)
            .await?
            .tx;
        let available:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM knowledge_field_outbox o WHERE brain_id=$1 AND (revision>handled_revision OR deadline<=clock_timestamp()) AND NOT EXISTS(SELECT 1 FROM jobs WHERE brain_id=$1 AND kind='knowledge.fields' AND state IN ('queued','running')) FOR UPDATE)").bind(brain).fetch_one(&mut *tx).await?;
        if available {
            let audit = db::audit(
                &mut tx,
                actor,
                brain,
                "knowledge.fields.queue",
                brain,
                "canonical_metadata_only",
            )
            .await?;
            let job = jobs::enqueue_work(
                &mut tx,
                actor,
                brain,
                audit,
                brain,
                "knowledge.fields",
                "heavy",
            )
            .await?;
            sqlx::query("UPDATE knowledge_field_outbox SET job_id=$2,state='queued',phase=CASE WHEN phase=4 THEN 0 ELSE phase END,after_id=CASE WHEN phase=4 THEN NULL ELSE after_id END WHERE brain_id=$1").bind(brain).bind(job).execute(&mut *tx).await?;
            count += 1;
        }
        tx.commit().await?;
    }
    Ok(count)
}
