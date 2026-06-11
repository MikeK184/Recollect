use super::*;
use crate::{model_gateway as gateway, model_policy, semantic as index};
use sqlx::{
    Postgres,
    postgres::{PgArguments, PgRow},
    query::QueryAs,
};

const SCAN_LIMIT: usize = 5000;
// The unique (profile_id,kind,input_id) key permits one indexed vector lookup
// per canonical candidate. Keep that bound even immediately after a large
// import, before PostgreSQL has refreshed its cardinality statistics.
const SCOPED: &str = ", semantic_scoped AS MATERIALIZED (
    SELECT m.*,e.embedding,e.truncated FROM matched m JOIN LATERAL (
      SELECT e.* FROM semantic_entries e WHERE e.brain_id=$1 AND e.profile_id=$11
      AND e.state='ready' AND e.embedding IS NOT NULL
      AND e.kind=CASE m.kind WHEN 'source_version' THEN 'source_chunk' WHEN 'claim' THEN 'claim_revision' ELSE m.kind END
      AND e.input_id=CASE WHEN m.kind='source_version' THEN m.chunk_id ELSE m.revision_id END
      AND (m.kind<>'source_version' OR e.source_version_id=m.revision_id)
      LIMIT 1
    ) e ON true
    WHERE m.status_eligible
      AND NOT EXISTS(SELECT 1 FROM model_input_fences f WHERE f.brain_id=$1 AND f.source_version_id=e.source_version_id)
      AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=$1 AND f.revision_id=e.claim_revision_id)
  )";

fn scoped<'q, O>(
    sql: &'q str,
    context: &ReadContext<'_>,
    profile: Uuid,
) -> QueryAs<'q, Postgres, O, PgArguments>
where
    O: for<'r> sqlx::FromRow<'r, PgRow>,
{
    let input = context.input;
    sqlx::query_as(sql)
        .bind(context.brain)
        .bind(context.at)
        .bind(input.query.clone())
        .bind(SqlJson(input.selection.clone()))
        .bind(input.collection_id)
        .bind(context.manifest.cloned().map(SqlJson))
        .bind(input.mode.clone())
        .bind(input.exact.as_ref().map(|e| e.kind.clone()))
        .bind(input.exact.as_ref().map(|e| e.id))
        .bind(input.channels.clone())
        .bind(profile)
}

pub(super) struct Admission {
    pub status: RecallSemantic,
    pub policy_id: Uuid,
}

pub(super) async fn prepare(
    state: &AppState,
    tx: &mut Tx<'_>,
    context: &ReadContext<'_>,
    coverage: &mut RecallCoverage,
) -> Result<Admission> {
    let p = model_policy::current(state, tx, context.brain).await?;
    model_policy::permits(state, &p.policy, "embedding", &["query".into()])?;
    let recorded:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM model_requests WHERE brain_id=$1 AND actor_id=recollect_actor() AND operation_id=$2 AND purpose='embedding')")
        .bind(context.brain).bind(context.input.semantic_request_id).fetch_one(&mut **tx).await?;
    if recorded {
        return Err(model_policy::failure(
            "model_attempt_recorded",
            "This query attempt is already recorded. Start an explicit new query; vectors are not retained for replay.",
        ));
    }
    let profile = index::profile(tx, context.brain).await?;
    if profile
        .as_ref()
        .is_some_and(|p| !index::compatible(state, p))
    {
        return Err(model_policy::failure(
            "semantic_profile_mismatch",
            "Semantic recall requires a compatible profile. Reindex with the approved model.",
        ));
    }
    let count = if let Some(profile) = &profile {
        let sql = format!(
            "{}{SCOPED} SELECT count(*)::bigint FROM (SELECT 1 FROM semantic_scoped LIMIT 5001) bounded",
            include_str!("../retrieval_candidates.sql")
        );
        let (count,): (i64,) = scoped(&sql, context, profile.id)
            .fetch_one(&mut **tx)
            .await?;
        if count > SCAN_LIMIT as i64 {
            return Err(model_policy::failure(
                "semantic_scope_too_large",
                "Exact semantic recall supports up to 5,000 scoped representations. Select a smaller collection or workspace scope.",
            ));
        }
        count as usize
    } else {
        0
    };
    if count == 0 {
        note(coverage, "semantic_scoped_coverage_missing");
    }
    // This is projection coverage, not truth qualification. Eligible canonical
    // rows without a vector remain visible as missing/partial index coverage.
    if let Some(profile) = &profile {
        let sql = format!(
            "{}{SCOPED} SELECT EXISTS(SELECT 1 FROM matched m WHERE m.status_eligible AND NOT EXISTS(SELECT 1 FROM semantic_scoped s WHERE s.kind=m.kind AND s.revision_id=m.revision_id AND coalesce(s.chunk_id,'00000000-0000-0000-0000-000000000000'::uuid)=coalesce(m.chunk_id,'00000000-0000-0000-0000-000000000000'::uuid)))",
            include_str!("../retrieval_candidates.sql")
        );
        let (missing,): (bool,) = scoped(&sql, context, profile.id)
            .fetch_one(&mut **tx)
            .await?;
        if missing {
            note(coverage, "semantic_scoped_coverage_partial");
        }
    }
    Ok(Admission {
        status: RecallSemantic {
            profile,
            model_request_id: None,
            scoped_entries: count,
            state: if count == 0 { "missing" } else { "available" }.into(),
        },
        policy_id: p.change_id,
    })
}

pub(super) async fn embed(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    input: &RecallRequest,
    admission: &mut Admission,
) -> Result<Vec<f32>> {
    let response = gateway::invoke_for_policy(
        state,
        gateway::Context {
            brain,
            actor: auth.user.id,
            device: auth.device_id,
        },
        gateway::Invocation {
            operation: input.semantic_request_id.ok_or_else(Error::missing)?,
            purpose: "embedding".into(),
            inputs: vec![],
            query: Some(input.query.clone()),
            instructions: String::new(),
            prompt_label: "semantic-query-1".into(),
            schema_label: "embedding-3072-1".into(),
            format: gateway::Format::Embedding,
            metadata_replay: false,
            expected_json: None,
        },
        admission.policy_id,
    )
    .await?;
    let Some(gateway::Output::Embeddings(mut vectors)) = response.output else {
        return Err(model_policy::failure(
            "model_result_not_retained",
            "This query has no retained vector result. Start an explicit new query attempt.",
        ));
    };
    if vectors.len() != 1 {
        return Err(model_policy::failure(
            "provider_shape",
            "The provider returned an invalid query embedding.",
        ));
    }
    admission.status.model_request_id = Some(response.request.id);
    Ok(vectors.remove(0))
}

pub(super) async fn recheck(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    admission: &Admission,
) -> Result<()> {
    let p = model_policy::current(state, tx, brain).await?;
    model_policy::permits(state, &p.policy, "embedding", &["query".into()])?;
    if p.change_id != admission.policy_id {
        return Err(model_policy::failure(
            "model_policy_changed",
            "The query policy changed during semantic recall.",
        ));
    }
    let profile = index::profile(tx, brain).await?;
    if profile.as_ref().map(|p| p.id) != admission.status.profile.as_ref().map(|p| p.id)
        || profile
            .as_ref()
            .is_some_and(|p| !index::compatible(state, p))
    {
        return Err(model_policy::failure(
            "semantic_profile_changed",
            "The semantic index was rebuilt during this query. Start a new explicit query.",
        ));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct Scored {
    #[sqlx(flatten)]
    candidate: Candidate,
    similarity: f32,
    truncated: bool,
}

pub(super) struct Ranked {
    pub item: RecallItem,
    pub deadline: Option<DateTime<Utc>>,
    pub exact: bool,
    pub alternatives: Vec<(RecallItem, Option<DateTime<Utc>>)>,
    lexical_rank: Option<usize>,
    semantic_rank: Option<usize>,
    graph_rank: Option<usize>,
}

pub(super) async fn ranked(
    state: &AppState,
    tx: &mut Tx<'_>,
    context: &ReadContext<'_>,
    baseline: &[Candidate],
    admission: Option<&Admission>,
    vector: Option<&[f32]>,
    coverage: &mut RecallCoverage,
) -> Result<Vec<Ranked>> {
    let mut identities: BTreeMap<(String, Uuid), Ranked> = BTreeMap::new();
    let mut lexical_scores = BTreeMap::<(String, Uuid), f32>::new();
    // Ranking operates on fully qualified canonical items before applying the
    // caller's context budget. Channel scores never establish eligibility.
    for row in baseline.iter().take(CANDIDATES) {
        let key = (row.kind.clone(), row.id);
        coverage.examined += 1;
        if let Some((item, deadline)) = item(state, tx, context, row, coverage).await? {
            if row.lexical_match {
                lexical_scores
                    .entry(key.clone())
                    .and_modify(|rank| *rank = rank.max(row.rank))
                    .or_insert(row.rank);
            }
            let existing = identities.entry(key).or_insert_with(|| Ranked {
                item: item.clone(),
                deadline,
                exact: row.exact_match,
                alternatives: vec![],
                lexical_rank: None,
                semantic_rank: None,
                graph_rank: None,
            });
            if existing
                .item
                .provenance
                .first()
                .map(|p| (p.id, p.byte_from, p.byte_to))
                != item
                    .provenance
                    .first()
                    .map(|p| (p.id, p.byte_from, p.byte_to))
            {
                existing.alternatives.push((item, deadline));
            }
            existing.exact |= row.exact_match;
            for (matched, channel) in [(row.exact_match, "exact"), (row.lexical_match, "lexical")] {
                if matched && !existing.item.channels.iter().any(|c| c == channel) {
                    existing.item.channels.push(channel.into());
                }
            }
        }
    }
    let mut lexical_scores: Vec<_> = lexical_scores.into_iter().collect();
    lexical_scores.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (rank, (key, _)) in lexical_scores.into_iter().enumerate() {
        identities
            .get_mut(&key)
            .expect("qualified lexical identity")
            .lexical_rank = Some(rank + 1);
    }
    if let (Some(vector), Some(profile)) =
        (vector, admission.and_then(|a| a.status.profile.as_ref()))
    {
        let sql=format!("{}{SCOPED} SELECT kind,id,revision_id,chunk_id,label,text,recorded_at,selection,source_id,repository_id,
          snapshot_id,revision,path,line_from,line_to,byte_from,byte_to,artifact_id,byte_length,processing,data,expires_at,exact_match,lexical_match,rank,
          (1-(embedding<=>$12::text::vector))::real similarity,truncated FROM semantic_scoped
          ORDER BY embedding<=>$12::text::vector,kind,id,chunk_id NULLS FIRST LIMIT 5001",include_str!("../retrieval_candidates.sql"));
        let encoded =
            serde_json::to_string(vector).map_err(|_| Error::invalid("Invalid query vector."))?;
        let rows: Vec<Scored> = scoped(&sql, context, profile.id)
            .bind(encoded)
            .fetch_all(&mut **tx)
            .await?;
        if rows.len() > SCAN_LIMIT {
            return Err(model_policy::failure(
                "semantic_scope_too_large",
                "The semantic projection grew beyond this recall scope. Narrow the selection.",
            ));
        }
        let mut semantic_rank = 0;
        for scored in rows {
            if !scored.similarity.is_finite() {
                return Err(model_policy::failure(
                    "semantic_distance_unavailable",
                    "The stored vector did not produce a finite cosine distance.",
                ));
            }
            let similarity = scored.similarity.clamp(-1.0, 1.0);
            if similarity < context.input.semantic_min_similarity.unwrap_or(0.0) {
                continue;
            }
            let row = scored.candidate;
            let key = (row.kind.clone(), row.id);
            if identities
                .get(&key)
                .is_some_and(|r| r.semantic_rank.is_some())
            {
                continue;
            }
            coverage.examined += 1;
            let Some((mut item, deadline)) = item(state, tx, context, &row, coverage).await? else {
                continue;
            };
            if item.kind == "source_version"
                && item.provenance.iter().any(|p| p.availability != "retained")
            {
                coverage.withheld += 1;
                continue;
            }
            if semantic_rank == CANDIDATES {
                note(coverage, "semantic_candidate_limit");
                break;
            }
            semantic_rank += 1;
            item.channels = vec!["semantic".into()];
            let ranked = match identities.entry(key) {
                std::collections::btree_map::Entry::Vacant(entry) => entry.insert(Ranked {
                    item,
                    deadline,
                    exact: false,
                    alternatives: vec![],
                    lexical_rank: None,
                    semantic_rank: None,
                    graph_rank: None,
                }),
                std::collections::btree_map::Entry::Occupied(entry) => {
                    let ranked = entry.into_mut();
                    // Channel ranks belong to the source identity, but the
                    // shown cosine must travel with its exact scoring span.
                    // Preserve exact priority and lexical rank while selecting
                    // the semantic fragment for the attributed context.
                    if ranked.lexical_rank.is_some()
                        && ranked
                            .item
                            .provenance
                            .first()
                            .map(|p| (p.id, p.byte_from, p.byte_to))
                            != item
                                .provenance
                                .first()
                                .map(|p| (p.id, p.byte_from, p.byte_to))
                    {
                        item.qualifications
                            .push("lexical_match_in_another_fragment".into());
                    }
                    item.channels = ranked.item.channels.clone();
                    ranked.item = item;
                    ranked.alternatives.clear();
                    ranked.deadline = [ranked.deadline, deadline].into_iter().flatten().min();
                    ranked
                }
            };
            ranked.semantic_rank = Some(semantic_rank);
            ranked.item.semantic_similarity = Some(similarity);
            if !ranked.item.channels.iter().any(|c| c == "semantic") {
                ranked.item.channels.push("semantic".into());
            }
            ranked
                .item
                .qualifications
                .push("semantic_similarity_not_truth".into());
            if scored.truncated {
                ranked
                    .item
                    .qualifications
                    .push("semantic_representation_truncated".into());
                note(coverage, "semantic_representation_truncated");
            }
        }
    }
    let mut ranked: Vec<Ranked> = identities.into_values().collect();
    sort(&mut ranked, context.input);
    Ok(ranked)
}

pub(super) fn merge_graph(
    ranked: &mut Vec<Ranked>,
    graph: &crate::graph::recall::Output,
    input: &RecallRequest,
) -> Result<()> {
    for (rank, item) in graph.items.iter().enumerate() {
        if let Some(existing) = ranked
            .iter_mut()
            .find(|r| r.item.kind == item.kind && r.item.id == item.id)
        {
            if existing.item.revision_id != item.revision_id {
                return Err(Error::invalid(
                    "Graph and recall revisions disagree. Start a fresh query.",
                ));
            }
            // Keep the semantic scoring span (or qualified lexical fragment).
            // Discovery provenance is separate from the target's own support.
            existing.item.graph_match = item.graph_match.clone();
            existing.deadline = [existing.deadline, graph.status.expires_at]
                .into_iter()
                .flatten()
                .min();
            existing.item.channels.push("graph".into());
            existing
                .item
                .qualifications
                .push("graph_proximity_not_truth".into());
            existing.graph_rank = Some(rank + 1);
        } else {
            ranked.push(Ranked {
                item: item.clone(),
                deadline: graph.status.expires_at,
                exact: false,
                alternatives: vec![],
                lexical_rank: None,
                semantic_rank: None,
                graph_rank: Some(rank + 1),
            });
        }
    }
    sort(ranked, input);
    Ok(())
}

fn sort(ranked: &mut [Ranked], input: &RecallRequest) {
    let semantic = input.channels.iter().any(|c| c == "semantic");
    let graph = input.channels.iter().any(|c| c == "graph");
    for entry in ranked.iter_mut() {
        entry.item.score = if graph || (semantic && input.channels.len() > 1) {
            entry.lexical_rank.map_or(0.0, |r| 1.0 / (60 + r) as f32)
                + entry.semantic_rank.map_or(0.0, |r| 1.0 / (60 + r) as f32)
                + entry.graph_rank.map_or(0.0, |r| 1.0 / (60 + r) as f32)
        } else if semantic {
            entry.item.semantic_similarity.unwrap_or(0.0)
        } else {
            entry.item.score
        };
    }
    ranked.sort_by(|a, b| {
        b.exact
            .cmp(&a.exact)
            .then_with(|| b.item.score.total_cmp(&a.item.score))
            .then_with(|| (&a.item.kind, a.item.id).cmp(&(&b.item.kind, b.item.id)))
    });
}
