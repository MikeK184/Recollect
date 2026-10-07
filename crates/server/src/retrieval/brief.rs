//! Scoped supplement to ordinary recall. It grants no authority and pays no
//! extra model call; all records pass the ordinary attributed item gate.
use super::*;

pub(super) async fn supplement(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    read: &ReadContext<'_>,
    ranked: &mut Vec<semantic::Ranked>,
    coverage: &mut RecallCoverage,
) -> Result<()> {
    let input = read.input;
    if (!input.project_brief && input.continuation.is_none())
        || input.mode == "history"
        || input.knowledge_at.is_some()
    {
        return Ok(());
    }
    let mut strict = input.clone();
    strict.mode = if input.mode == "strict_operational" {
        "strict_operational"
    } else {
        "strict_accepted"
    }
    .into();
    let strict_read = ReadContext {
        brain: read.brain,
        input: &strict,
        at: read.at,
        manifest: read.manifest,
    };
    let continuation = continuation(state, tx, auth, &strict_read, coverage).await?;
    let sql = format!(
        "{} SELECT matched.* FROM matched JOIN selected_claims family ON family.id=matched.revision_id WHERE kind='claim' AND status_eligible AND (revision_id=$11 OR ($12 AND data#>>'{{content,kind}}'<>'handover' AND (data#>>'{{content,kind}}'='decision' OR data#>>'{{content,context_role}}' IN ('decision','convention','constraint')))) AND ($13::timestamptz IS NULL OR recollect_validity_range(data#>'{{content,validity}}') @> $13) ORDER BY (revision_id=$11) DESC NULLS LAST, ((selection->>'environment_id' IS NOT NULL)::integer+jsonb_array_length(coalesce(selection->'repository_ids','[]'))+jsonb_array_length(coalesce(selection->'area_ids','[]'))) DESC,coalesce(data#>>'{{content,context_role}}','decision'),family.subject_key,family.predicate_key,matched.id LIMIT 500",
        include_str!("../retrieval_candidates.sql")
    );
    let rows: Vec<Candidate> = sqlx::query_as(&sql)
        .bind(read.brain)
        .bind(read.at)
        .bind(&input.query)
        .bind(SqlJson(&input.selection))
        .bind(input.collection_id)
        .bind(read.manifest.cloned().map(SqlJson))
        .bind(&strict.mode)
        .bind(input.exact.as_ref().map(|e| &e.kind))
        .bind(input.exact.as_ref().map(|e| e.id))
        .bind(&input.channels)
        .bind(continuation)
        .bind(input.project_brief)
        .bind(input.fact_at)
        .fetch_all(&mut **tx)
        .await?;
    if rows.len() == 500 {
        note(coverage, "project_brief_discovery_bounded");
    }
    let mut eligible = 0;
    let mut count = 0;
    let mut brief_bytes = 2usize;
    for row in rows {
        let Some((mut item, deadline)) = item(state, tx, &strict_read, &row, coverage).await?
        else {
            continue;
        };
        if item
            .claim
            .as_ref()
            .is_none_or(|c| !c.conflicting_claim_ids.is_empty() || !c.rule_ids.is_empty())
        {
            continue;
        }
        eligible += 1;
        if eligible > 64 {
            note(coverage, "project_brief_discovery_bounded");
            break;
        }
        item.delivery_section = if Some(row.revision_id) == continuation {
            "continuation"
        } else {
            "project_brief"
        }
        .into();
        item.channels = vec![item.delivery_section.clone()];
        item.score = 0.;
        item.semantic_similarity = None;
        item.graph_match = None;
        if item.delivery_section == "project_brief" {
            let n = serde_json::to_vec(&item).expect("typed brief item").len() + 1;
            if count == 3 || brief_bytes + n > 1536 {
                note(coverage, "project_brief_budget");
                continue;
            }
            count += 1;
            brief_bytes += n;
        } else {
            note(coverage, "continuation_one_page");
        }
        ranked.push(semantic::Ranked {
            item,
            deadline,
            exact: false,
            alternatives: vec![],
            lexical_rank: None,
            semantic_rank: None,
            graph_rank: None,
        });
    }
    Ok(())
}

async fn continuation(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    read: &ReadContext<'_>,
    coverage: &mut RecallCoverage,
) -> Result<Option<Uuid>> {
    let Some(operation) = read.input.operation_id else {
        if read.input.continuation.is_some() {
            return Err(Error::forbidden());
        }
        return Ok(None);
    };
    let op = workspace::bound_operation(tx, read.brain, operation).await?;
    let hint = read.input.continuation.as_ref();
    let task = hint.map_or(op.task_id, |h| h.task_id);
    let lineage:Vec<Uuid>=sqlx::query_scalar("WITH RECURSIVE predecessors AS (
      SELECT t.id,t.continuation_of_task_id,0 depth FROM workspace_tasks t
       WHERE t.brain_id=$1 AND t.id=$2 AND t.account_id=$3 AND t.device_id IS NOT DISTINCT FROM $4
      UNION ALL SELECT t.id,t.continuation_of_task_id,p.depth+1 FROM workspace_tasks t JOIN predecessors p ON t.id=p.continuation_of_task_id JOIN scope_snapshots s ON s.id=t.current_scope
       WHERE p.depth<31 AND t.brain_id=$1 AND t.account_id=$3 AND t.device_id IS NOT DISTINCT FROM $4 AND t.closed AND s.snapshot->'selection'=$5)
      SELECT id FROM predecessors")
        .bind(read.brain).bind(op.task_id).bind(auth.user.id).bind(auth.device_id).bind(SqlJson(&read.input.selection)).fetch_all(&mut **tx).await?;
    if !lineage.contains(&task) {
        return Err(Error::forbidden());
    }
    if let Some(binding) = hint.and_then(|h| h.binding_id) {
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM capture_bindings b JOIN operation_bindings o ON o.id=b.operation_id WHERE b.brain_id=$1 AND b.id=$2 AND b.actor_id=$3 AND b.device_id IS NOT DISTINCT FROM $4 AND o.task_id=$5 AND b.selection=$6)")
            .bind(read.brain).bind(binding).bind(auth.user.id).bind(auth.device_id).bind(task).bind(SqlJson(&read.input.selection)).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(Error::forbidden());
        }
    }
    // Same-task native resume may follow its closed predecessor. Distinct
    // episodes/pages remain visible coverage, never selected by newest time.
    let tasks = if task == op.task_id {
        lineage
    } else {
        vec![task]
    };
    #[derive(sqlx::FromRow)]
    struct Episode {
        #[sqlx(flatten)]
        candidate: Candidate,
        digest_coverage: SqlJson<Value>,
    }
    // Resolve ambiguity only among applicable, deliverable episodes. The same
    // collection/time/status/scope gate used by recall precedes this bounded scan.
    let sql = format!(
        "{} SELECT m.*,p.coverage AS digest_coverage FROM matched m JOIN session_digest_claims d ON d.revision_id=m.revision_id AND d.brain_id=$1 JOIN session_digest_partitions p ON p.id=d.partition_id JOIN claims c ON c.id=d.claim_id AND c.brain_id=d.brain_id
        WHERE m.kind='claim' AND m.status_eligible AND d.page=0 AND p.task_id=ANY($11)
          AND p.actor_id=$12 AND p.device_id IS NOT DISTINCT FROM $13 AND p.selection=$4
          AND p.manifest_revision_id IS NOT DISTINCT FROM $14 AND ($15::uuid IS NULL OR p.binding_id=$15)
          AND c.current_revision=d.revision_id
          AND ($16::timestamptz IS NULL OR recollect_validity_range(m.data#>'{{content,validity}}') @> $16)
        ORDER BY p.id LIMIT 500", include_str!("../retrieval_candidates.sql"));
    let rows: Vec<Episode> = sqlx::query_as(&sql)
        .bind(read.brain)
        .bind(read.at)
        .bind(&read.input.query)
        .bind(SqlJson(&read.input.selection))
        .bind(read.input.collection_id)
        .bind(read.manifest.cloned().map(SqlJson))
        .bind(&read.input.mode)
        .bind(read.input.exact.as_ref().map(|e| &e.kind))
        .bind(read.input.exact.as_ref().map(|e| e.id))
        .bind(&read.input.channels)
        .bind(&tasks)
        .bind(auth.user.id)
        .bind(auth.device_id)
        .bind(read.input.manifest_revision_id)
        .bind(hint.and_then(|h| h.binding_id))
        .bind(read.input.fact_at)
        .fetch_all(&mut **tx)
        .await?;
    if rows.len() == 500 {
        note(coverage, "continuation_discovery_bounded");
        return Ok(None);
    }
    let mut selected = None;
    for row in rows {
        let Some((item, _)) = item(state, tx, read, &row.candidate, coverage).await? else {
            continue;
        };
        if item
            .claim
            .as_ref()
            .is_none_or(|c| !c.conflicting_claim_ids.is_empty() || !c.rule_ids.is_empty())
        {
            continue;
        }
        if selected.is_some() {
            note(coverage, "continuation_ambiguous");
            return Ok(None);
        }
        selected = Some((row.candidate.revision_id, row.digest_coverage.0));
    }
    if let Some((id, data)) = selected {
        if data["omitted"].as_u64().is_some_and(|n| n > 0) || data["bounded_scan"] == true {
            note(coverage, "continuation_partial_inputs");
        }
        return Ok(Some(id));
    }
    Ok(None)
}
