//! Exact evidence aliases preserve the assessed meaning and original authority.
use crate::{AppState, artifacts, error::Result, memory_evidence::Tx, model_gateway, model_policy};
use recollect_protocol::{ClaimContent, ClaimSupport, LearningRun};
use serde_json::{Value, json};
use sqlx::types::Json;
use uuid::Uuid;

pub(crate) async fn retain(
    state: &AppState,
    tx: &mut Tx<'_>,
    run: &LearningRun,
    content: &ClaimContent,
) -> Result<ClaimContent> {
    let mut final_content = content.clone();
    if !run.automatic {
        return Ok(final_content);
    }
    let capture: bool = sqlx::query_scalar("SELECT allow_document_content FROM brains WHERE id=$1")
        .bind(run.brain_id)
        .fetch_one(&mut **tx)
        .await?;
    let retention = crate::retention::settings(tx, run.brain_id).await?;
    if !capture || !retention.policy.allow_support_excerpts {
        return Ok(final_content);
    }
    let policy = model_policy::current(state, tx, run.brain_id).await?;
    if policy.change_id != run.policy_id
        || !policy
            .policy
            .content_classes
            .contains(&"support_excerpt".into())
    {
        return Ok(final_content);
    }
    for support in &mut final_content.supports {
        if support.kind != "source_version" {
            continue;
        }
        let automatic: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM automatic_support_excerpts WHERE brain_id=$1 AND version_id=$2)")
            .bind(run.brain_id).bind(support.id).fetch_one(&mut **tx).await?;
        if automatic {
            continue;
        }
        let source =
            model_gateway::source(state, tx, run.brain_id, support.id, 1024 * 1024).await?;
        if source.class == "support_excerpt" {
            continue;
        }
        if model_policy::permits(
            state,
            &policy.policy,
            "extraction",
            &[source.class.clone(), "support_excerpt".into()],
        )
        .is_err()
        {
            continue;
        }
        let (Some(from), Some(to)) = (support.line_from, support.line_to) else {
            continue;
        };
        let lines: Vec<_> = source.text.split_inclusive('\n').collect();
        if from < 1 || to < from || to as usize > lines.len() {
            return Err(model_policy::denied());
        }
        let text = lines[from as usize - 1..to as usize].concat();
        if text.is_empty() || text.len() > 8192 {
            continue;
        }
        let found: Option<Uuid> = sqlx::query_scalar("SELECT a.version_id FROM automatic_support_excerpts a JOIN source_versions v ON v.id=a.version_id AND v.brain_id=a.brain_id WHERE a.brain_id=$1 AND a.identity_key=md5(jsonb_build_array($1::uuid,$2::uuid,$3::integer,$4::integer,$5::jsonb,$6::uuid)::text) ")
            .bind(run.brain_id).bind(support.id).bind(from).bind(to).bind(Json(&run.selection)).bind(run.manifest_revision_id).fetch_optional(&mut **tx).await?;
        let id = if let Some(id) = found {
            let active: bool = sqlx::query_scalar("SELECT privacy_state='active' AND recollect_content_state(brain_id,retention_class,privacy_state,created_at)='active' FROM source_versions WHERE brain_id=$1 AND id=$2")
                .bind(run.brain_id).bind(id).fetch_one(&mut **tx).await?;
            if !active {
                continue;
            }
            id
        } else {
            let source_id = Uuid::new_v4();
            let id = Uuid::new_v4();
            let artifact = Uuid::new_v4();
            let provenance = projection(&source.provenance);
            crate::publication::safe_payload(state, &json!({"text":text,"provenance":provenance}))?;
            // Commit the content-free intent before any file write. A failed
            // canonical transaction cannot hide these bytes from erasure/replay.
            let mut intent = crate::db::device_tx(&state.pool, run.actor_id, run.device_id).await?;
            sqlx::query("INSERT INTO excerpt_artifact_intents(artifact_id,brain_id,run_id,parent_version_id) VALUES($1,$2,$3,$4)")
                .bind(artifact).bind(run.brain_id).bind(run.id).bind(support.id).execute(&mut *intent).await?;
            intent.commit().await?;
            // Retaining a copy is optional. Storage failure preserves the already
            // assessed raw support without another provider request.
            if artifacts::write(&state.config.artifact_dir, run.brain_id, artifact, &text)
                .await
                .is_err()
            {
                let _ =
                    crate::privacy_journal::remove_artifact(&state.config, run.brain_id, artifact)
                        .await;
                // Preserve the opaque identity even after successful removal:
                // a later original erasure must scrub pre-cleanup backups too.
                continue;
            }
            sqlx::query("INSERT INTO sources(id,brain_id,created_by,retention_class) VALUES($1,$2,$3,'support_excerpt')")
                .bind(source_id).bind(run.brain_id).bind(run.actor_id).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO source_versions(id,source_id,brain_id,title,media_type,artifact_id,byte_length,created_by,processing,retention_class) VALUES($1,$2,$3,'Supporting evidence','text/plain',$4,$5,$6,'ready','support_excerpt')")
                .bind(id).bind(source_id).bind(run.brain_id).bind(artifact).bind(text.len() as i32).bind(run.actor_id).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO source_excerpts(brain_id,version_id,parent_source_id,parent_version_id,first_line,last_line) VALUES($1,$2,$3,$4,$5,$6)")
                .bind(run.brain_id).bind(id).bind(source.source_id).bind(support.id).bind(from).bind(to).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO automatic_support_excerpts(version_id,brain_id,parent_version_id,first_line,last_line,selection,manifest_revision_id,original_class,provenance,identity_key) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,md5(jsonb_build_array($2::uuid,$3::uuid,$4::integer,$5::integer,$6::jsonb,$7::uuid)::text))")
                .bind(id).bind(run.brain_id).bind(support.id).bind(from).bind(to).bind(Json(&run.selection)).bind(run.manifest_revision_id).bind(&source.class).bind(Json(provenance)).execute(&mut **tx).await?;
            sqlx::query("UPDATE sources SET current_version=$2 WHERE id=$1 AND brain_id=$3")
                .bind(source_id)
                .bind(id)
                .bind(run.brain_id)
                .execute(&mut **tx)
                .await?;
            sqlx::query(
                "DELETE FROM excerpt_artifact_intents WHERE artifact_id=$1 AND brain_id=$2",
            )
            .bind(artifact)
            .bind(run.brain_id)
            .execute(&mut **tx)
            .await?;
            id
        };
        *support = ClaimSupport {
            kind: "source_version".into(),
            id,
            line_from: Some(1),
            line_to: Some(to - from + 1),
        };
    }
    equivalent(state, tx, run, content, &final_content).await?;
    Ok(final_content)
}

pub(crate) async fn equivalent(
    state: &AppState,
    tx: &mut Tx<'_>,
    run: &LearningRun,
    original: &ClaimContent,
    copy: &ClaimContent,
) -> Result<()> {
    let mut restored = copy.clone();
    for (index, s) in restored.supports.iter_mut().enumerate() {
        let Some(expected) = original.supports.get(index) else {
            return Err(model_policy::denied());
        };
        if s == expected {
            continue;
        }
        type Alias = (Uuid, i32, i32, Json<Value>, String, Json<Value>);
        let row: Option<Alias> = sqlx::query_as("SELECT parent_version_id,first_line,last_line,selection,original_class,provenance FROM automatic_support_excerpts WHERE brain_id=$1 AND version_id=$2 AND privacy_state='active' AND manifest_revision_id IS NOT DISTINCT FROM $3")
            .bind(run.brain_id).bind(s.id).bind(run.manifest_revision_id).fetch_optional(&mut **tx).await?;
        let (parent, from, to, selection, class, provenance) =
            row.ok_or_else(model_policy::denied)?;
        if parent != expected.id
            || expected.line_from != Some(from)
            || expected.line_to != Some(to)
            || s.line_from != Some(1)
            || s.line_to != Some(to - from + 1)
            || selection.0 != json!(run.selection)
        {
            return Err(model_policy::denied());
        }
        let source = model_gateway::source(state, tx, run.brain_id, parent, 1024 * 1024).await?;
        let alias = model_gateway::source(state, tx, run.brain_id, s.id, 8192).await?;
        let lines: Vec<_> = source.text.split_inclusive('\n').collect();
        if source.class != class
            || to as usize > lines.len()
            || alias.text != lines[from as usize - 1..to as usize].concat()
            || provenance.0 != projection(&source.provenance)
        {
            return Err(model_policy::denied());
        }
        *s = expected.clone();
    }
    if &restored != original {
        return Err(model_policy::denied());
    }
    Ok(())
}

fn projection(source: &Value) -> Value {
    let mut value = source.clone();
    if let Some(fields) = value.as_object_mut() {
        fields.retain(|key, _| {
            matches!(
                key.as_str(),
                "kind"
                    | "source_id"
                    | "version_id"
                    | "content_class"
                    | "contributor_id"
                    | "observed_at"
                    | "recorded_at"
                    | "role"
                    | "capture"
            )
        });
    }
    // Capture metadata is canonical, bounded and sanitized at admission. Keep
    // its full attribution, outcome and coverage, including unknown qualifiers.
    value
}

/// Remove only ledger-owned, unreferenced files after their worker lease ends.
/// Failure preserves the opaque intent for the next maintenance or erasure pass.
pub(crate) async fn reconcile_artifacts(
    state: &AppState,
    run: Option<Uuid>,
) -> anyhow::Result<usize> {
    let pending: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT brain_id,artifact_id FROM recollect_pending_excerpt_artifacts($1)")
            .bind(run)
            .fetch_all(&state.pool)
            .await?;
    let mut count = 0;
    for (brain, artifact) in pending {
        if let Err(error) =
            crate::privacy_journal::remove_artifact(&state.config, brain, artifact).await
        {
            tracing::warn!(%brain, %artifact, %error, "Excerpt cleanup remains pending");
            continue;
        }
        sqlx::query("SELECT recollect_forget_excerpt_artifact($1,$2)")
            .bind(brain)
            .bind(artifact)
            .execute(&state.pool)
            .await?;
        count += 1;
    }
    Ok(count)
}
