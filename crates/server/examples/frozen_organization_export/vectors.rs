//! Explicit owned toy-only semantic worker calls. Standing permission is disabled
//! again before extracting/admitting experiments; no background worker runs.
use super::{
    io,
    owned::{Owned, api, http},
    scopes::Imported,
};
use anyhow::{Context, Result, ensure};
use recollect_server::{semantic, worker};
use serde_json::{Value, json};
use std::path::Path;
use uuid::Uuid;
pub async fn prepare(o: &Owned, sources: &[Imported], out: &Path) -> Result<Value> {
    let mut brains = std::collections::BTreeSet::new();
    for s in sources {
        brains.insert(s.brain);
    }
    let mut policies = vec![];
    let mut receipts = vec![];
    for brain in &brains {
        let path = format!("/api/brains/{brain}/models/policy");
        let (before, _) = http(&o.state, Some(&o.login), "GET", &path, Value::Null).await?;
        let mut p = before["current"]["policy"].clone();
        ensure!(
            p["automatic_embedding"] == false
                && p["automatic_learning"] == false
                && p["autonomous_memory"] == false,
            "Automatic work unexpectedly enabled"
        );
        p["automatic_embedding"] = json!(true);
        let (during, _) = http(
            &o.state,
            Some(&o.login),
            "PUT",
            &path,
            json!({"base_change":before["current"]["change_id"],"policy":p}),
        )
        .await?;
        let head: Option<Uuid> =
            sqlx::query_scalar("SELECT profile_id FROM semantic_heads WHERE brain_id=$1")
                .bind(brain)
                .fetch_optional(&o.admin)
                .await?;
        if head.is_none() {
            let (spare, _) = http(
                &o.state,
                Some(&o.login),
                "POST",
                &format!("/api/brains/{brain}/semantic/reindex"),
                json!({"base_profile":null}),
            )
            .await?;
            let spare_id: Uuid = spare["id"]
                .as_str()
                .context("Spare canonical profile")?
                .parse()?;
            http(
                &o.state,
                Some(&o.login),
                "POST",
                &format!("/api/brains/{brain}/semantic/reindex"),
                json!({"base_profile":spare_id}),
            )
            .await?;
        }
        api(semantic::maintain_brain(&o.state, *brain, o.actor).await)?;
        loop {
            let active:Vec<(Uuid,Uuid,String)>=sqlx::query_as("SELECT id,brain_id,kind FROM jobs WHERE lane='model' AND state IN('queued','running') ORDER BY created_at,id").fetch_all(&o.admin).await?;
            ensure!(
                active
                    .iter()
                    .all(|(_, b, k)| b == brain && k == "semantic.generate"),
                "Foreign model work; no claim"
            );
            if active.is_empty() {
                break;
            }
            let job = worker::claim(&o.state.pool, "model")
                .await?
                .context("Exact explicit semantic job missing")?;
            ensure!(
                active
                    .iter()
                    .any(|(id, b, k)| *id == job.id && *b == job.brain_id && k == &job.kind)
                    && job.actor_id == o.actor,
                "Semantic claim changed"
            );
            let result = Box::pin(semantic::execute(&o.state, &job)).await;
            if let Err(failure) = result {
                worker::fail(&o.state.pool, &job, failure).await?;
            }
            // A controlled incomplete batch can be requeued by generic worker handling.
            // Cancel that exact batch once after its own one-shot request; never retry it.
            let state: String = sqlx::query_scalar("SELECT state FROM jobs WHERE id=$1")
                .bind(job.id)
                .fetch_one(&o.admin)
                .await?;
            if matches!(state.as_str(), "queued" | "running") {
                let auth = o.auth().await?;
                let _ = api(recollect_server::jobs::cancel(
                    axum::extract::State(o.state.clone()),
                    auth,
                    axum::extract::Path((*brain, job.id)),
                )
                .await)?;
            }
            let summary:Value=sqlx::query_scalar("SELECT jsonb_build_object('job_id',j.id,'brain_id',j.brain_id,'target_id',j.target_id,'state',j.state,'kind',j.kind,'request_id',b.request_id,'batch_state',b.state,'entry_ids',(SELECT jsonb_agg(entry_id ORDER BY ordinal) FROM semantic_batch_inputs WHERE brain_id=b.brain_id AND batch_id=b.id)) FROM jobs j JOIN semantic_batches b ON b.job_id=j.id WHERE j.id=$1").bind(job.id).fetch_one(&o.admin).await?;
            receipts.push(summary);
        }
        p["automatic_embedding"] = json!(false);
        let (after, _) = http(
            &o.state,
            Some(&o.login),
            "PUT",
            &path,
            json!({"base_change":during["change_id"],"policy":p}),
        )
        .await?;
        policies.push(json!({"brain_id":brain,"initial_policy_id":before["current"]["change_id"],"explicit_embedding_policy_id":during["change_id"],"final_disabled_policy_id":after["change_id"],"standing_automatic_embedding_before_and_after":false}));
    }
    let unclaimed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM jobs WHERE lane='model' AND state IN('queued','running')",
    )
    .fetch_one(&o.admin)
    .await?;
    ensure!(unclaimed == 0, "Owned model job remains claimable");
    let receipt = json!({"policy_timing":policies,"jobs":receipts,"unclaimed_model_jobs":0,"background_worker_started":false,"only_exact_explicit_owned_jobs_executed":true});
    io::json(&out.join("explicit-vector-work.json"), &receipt)?;
    Ok(receipt)
}
pub async fn provenance(o: &Owned, s: &Imported) -> Result<Value> {
    let entries:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('entry_id',e.id,'profile_id',e.profile_id,'source_chunk_id',c.id,'source_version_id',c.version_id,'request_id',e.request_id,'ordinal',c.ordinal,'byte_start',c.byte_start,'byte_end',c.byte_end,'complete_chunk_sha256',encode(sha256(convert_to(c.content,'UTF8')),'hex'),'state',e.state,'truncated',e.truncated,'embedding_available',e.embedding IS NOT NULL,'embedding_sha256',CASE WHEN e.embedding IS NULL THEN NULL ELSE encode(sha256(convert_to(e.embedding::text,'UTF8')),'hex') END) FROM source_chunks c LEFT JOIN semantic_entries e ON e.brain_id=c.brain_id AND e.input_id=c.id AND e.source_version_id=c.version_id AND e.kind='source_chunk' WHERE c.brain_id=$1 AND c.version_id=$2 ORDER BY c.ordinal").bind(s.brain).bind(s.version).fetch_all(&o.admin).await?;
    let profile:Option<Value>=sqlx::query_scalar("SELECT to_jsonb(p) FROM semantic_profiles p JOIN semantic_heads h ON h.profile_id=p.id AND h.brain_id=p.brain_id WHERE p.brain_id=$1").bind(s.brain).fetch_optional(&o.admin).await?;
    let requests:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('request_id',q.id,'operation_id',q.operation_id,'purpose',q.purpose,'state',q.state,'provider',q.provider,'requested_model',q.model,'returned_model',q.returned_model,'dimensions',q.dimensions,'prompt_label',q.prompt_label,'schema_label',q.schema_label,'policy_id',q.policy_id,'offered_reference_digest',(SELECT encode(sha256(convert_to(coalesce(jsonb_agg(jsonb_build_object('kind',i.kind,'id',i.input_id) ORDER BY i.kind,i.input_id),'[]'::jsonb)::text,'UTF8')),'hex') FROM model_request_inputs i WHERE i.brain_id=q.brain_id AND i.request_id=q.id)) FROM model_requests q WHERE q.brain_id=$1 AND q.purpose='embedding' AND EXISTS(SELECT 1 FROM model_request_inputs i JOIN semantic_entries e ON e.brain_id=i.brain_id AND e.id=i.input_id WHERE i.request_id=q.id AND e.source_version_id=$2) ORDER BY q.created_at,q.id").bind(s.brain).bind(s.version).fetch_all(&o.admin).await?;
    Ok(
        json!({"profile":profile,"entries":entries,"requests":requests,"values_and_centroids_exported":false}),
    )
}
