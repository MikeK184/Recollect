use super::*;
use crate::{model_gateway as gateway, model_policy, semantic as index};
use sqlx::{
    Postgres,
    postgres::{PgArguments, PgRow},
    query::QueryAs,
};

/// Exact cosine ranks every scoped vector up to this many representations.
pub const EXACT_LIMIT: usize = 5000;
/// Profiles of at most this many dimensions store their vectors inline (under
/// the 2 KB row threshold) and are exactly ranked over any scope instead:
/// 50,000 vectors measured 0.6 s at 8 and 384 dimensions.
pub const EXACT_DIMENSIONS: i32 = 384;
/// A profile holds at most 50,000 entries (the discovery cap), so no recall
/// scope is larger. Above the exact bound the scope is ranked by a bit-signature
/// shortlist, then exactly re-ranked.
pub const SCOPE_LIMIT: usize = 50_000;
/// Rows of one exact page handed to qualification (the statement still scores
/// every scoped vector).
const EXACT_PAGE: usize = 5000;
/// Hamming-nearest rows per page that receive an exact cosine score.
pub const SHORTLIST: usize = 4000;
/// Shortlist pages fetched before the candidate list is reported exhausted.
pub const SHORTLIST_PAGES: usize = 2;
const EXACT_COSINE: &str = "exact_cosine";
pub(super) const SHORTLIST_RERANK: &str = "binary_shortlist_rerank";

/// The largest scope this profile ranks with exact cosine.
fn exact_limit(dimensions: i32) -> usize {
    if dimensions <= EXACT_DIMENSIONS {
        SCOPE_LIMIT
    } else {
        EXACT_LIMIT
    }
}

// The unique (profile_id,kind,input_id) key permits one indexed vector lookup
// per canonical candidate. Keep that bound even immediately after a large
// import, before PostgreSQL has refreshed its cardinality statistics.
// Apply readiness outside the unique lookup: with fresh statistics PostgreSQL
// can otherwise scan the pending-work index once for every candidate instead.
// The selected columns say how much of the entry a consumer needs: admission
// reads identities only, the exact tier the vector, the shortlist tier the
// small signature (never the out-of-line vector).
fn scoped_cte(columns: &str) -> String {
    format!(
        "semantic_scoped AS MATERIALIZED (
    SELECT {columns} FROM recall_pool m JOIN LATERAL (
      SELECT e.* FROM semantic_entries e WHERE e.profile_id=$2
      AND e.kind=CASE m.kind WHEN 'source_version' THEN 'source_chunk' WHEN 'claim' THEN 'claim_revision' ELSE m.kind END
      AND e.input_id=CASE WHEN m.kind='source_version' THEN m.chunk_id ELSE m.revision_id END
      LIMIT 1
    ) e ON true
    WHERE m.status_eligible AND e.brain_id=$1 AND e.state='ready' AND e.embedding IS NOT NULL
      AND (m.kind<>'source_version' OR e.source_version_id=m.revision_id)
      AND NOT EXISTS(SELECT 1 FROM model_input_fences f WHERE f.brain_id=$1 AND f.source_version_id=e.source_version_id)
      AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=$1 AND f.revision_id=e.claim_revision_id)
  )"
    )
}

fn scoped<'q, O>(
    sql: &'q str,
    context: &ReadContext<'_>,
    profile: Uuid,
) -> QueryAs<'q, Postgres, O, PgArguments>
where
    O: for<'r> sqlx::FromRow<'r, PgRow>,
{
    sqlx::query_as(sql).bind(context.brain).bind(profile)
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
        .is_some_and(|profile| !index::compatible(&p.policy, profile))
    {
        return Err(model_policy::failure(
            "semantic_profile_mismatch",
            "Semantic recall requires a compatible profile. Reindex with the approved model.",
        ));
    }
    // Admission needs identities only. Keep canonical text and full vectors out
    // of this materialized count so a large fresh projection can be sized
    // within the unchanged read deadline before any provider request.
    let identities = scoped_cte("m.kind,m.revision_id,m.chunk_id");
    let (count, unindexed, method) = if let Some(profile) = &profile {
        tune(tx).await?;
        // Semantic scoring is independent of the query text: the whole eligible
        // population, from one pass of the canonical candidate chain.
        pool::build(
            tx,
            context,
            pool::Breadth {
                claims: true,
                rest: true,
            },
            true,
        )
        .await?;
        plan(tx).await?;
        // One pass sizes the scope and the eligible population, so projection
        // coverage is a count and not just a flag. Every scoped row is one
        // eligible pool row, so the difference is the eligible representations
        // that have no usable vector.
        let sql = format!(
            "WITH {identities} SELECT (SELECT count(*) FROM (SELECT 1 FROM semantic_scoped LIMIT {}) bounded)::bigint,
            (SELECT count(*) FROM recall_pool WHERE status_eligible)::bigint",
            SCOPE_LIMIT + 1
        );
        let (scoped_rows, eligible): (i64, i64) = scoped(&sql, context, profile.id)
            .fetch_one(&mut **tx)
            .await?;
        // The identity table is admission-only; the request's candidate pass is
        // built (in this or the post-embedding transaction) with full content.
        // Drop it before any refusal: automatic recall continues in this
        // transaction without the semantic channel.
        sqlx::query("DROP TABLE recall_pool")
            .execute(&mut **tx)
            .await?;
        let scoped_rows = scoped_rows as usize;
        let bound = exact_limit(profile.dimensions);
        if scoped_rows > SCOPE_LIMIT || (scoped_rows > bound && context.input.semantic_exact) {
            return Err(model_policy::failure(
                "semantic_scope_too_large",
                if scoped_rows > SCOPE_LIMIT {
                    "Semantic recall supports up to 50,000 scoped representations. Select a smaller collection or workspace scope."
                } else {
                    "Exact semantic recall supports up to 5,000 scoped representations. Select a smaller collection or workspace scope."
                },
            ));
        }
        let method = if scoped_rows > bound {
            SHORTLIST_RERANK
        } else {
            EXACT_COSINE
        };
        (
            scoped_rows,
            usize::try_from((eligible - scoped_rows as i64).max(0)).unwrap_or(0),
            method,
        )
    } else {
        (0, 0, EXACT_COSINE)
    };
    if count == 0 {
        note(coverage, "semantic_scoped_coverage_missing");
    }
    // This is projection coverage, not truth qualification. Eligible canonical
    // rows without a vector remain visible as missing/partial index coverage,
    // with their count in the semantic status.
    if unindexed > 0 {
        note(coverage, "semantic_scoped_coverage_partial");
    }
    Ok(Admission {
        status: RecallSemantic {
            profile,
            model_request_id: None,
            scoped_entries: count,
            state: if count == 0 { "missing" } else { "available" }.into(),
            method: method.into(),
            reranked: 0,
            unindexed_entries: unindexed,
        },
        policy_id: p.change_id,
    })
}

pub(super) async fn embed(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    input: &RecallRequest,
    query: &str,
    admission: &mut Admission,
    answer: Option<&crate::answers::Guard<'_>>,
) -> Result<Vec<f32>> {
    let context = gateway::Context {
        brain,
        actor: auth.user.id,
        device: auth.device_id,
    };
    let invocation = gateway::Invocation {
        operation: input.semantic_request_id.ok_or_else(Error::missing)?,
        purpose: "embedding".into(),
        inputs: vec![],
        query: Some(query.into()),
        instructions: String::new(),
        prompt_label: "semantic-query-1".into(),
        schema_label: "embedding-float-1".into(),
        format: gateway::Format::Embedding,
        work_lease: None,
        metadata_replay: false,
        expected_json: None,
    };
    let response = if let Some(answer) = answer {
        gateway::invoke_answer_embedding(state, context, invocation, admission.policy_id, answer)
            .await?
    } else {
        gateway::invoke_for_policy(state, context, invocation, admission.policy_id).await?
    };
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
            .is_some_and(|profile| !index::compatible(&p.policy, profile))
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
    /// Rows the exact statement scored (the scoped population); the shortlist
    /// statement does not report it.
    #[sqlx(default)]
    scope_rows: i64,
}

/// Every scoped vector, scored once, best first, one page at a time. `$4` is
/// the page size and `$5` its offset.
fn exact_sql() -> String {
    // The query vector is parsed once, and each distance is computed once and
    // reused by the ordering and the similarity. The window count tells the
    // caller how many vectors the statement scored, whatever the page.
    format!(
        "WITH {}, q AS MATERIALIZED (SELECT $3::text::vector v)
        SELECT {},(1-d)::real similarity,truncated,scope_rows
        FROM (SELECT s.*,s.embedding<=>q.v d,count(*) OVER () scope_rows FROM semantic_scoped s,q) x
        ORDER BY d,kind,id,chunk_id NULLS FIRST LIMIT $4 OFFSET $5",
        scoped_cte("m.*,e.embedding,e.truncated"),
        pool::CANDIDATE,
    )
}

/// One page of the shortlist, exactly re-ranked. Eligibility and scope are
/// applied before any distance, and the first pass reads only the small
/// signature of each scoped row; the stored vectors of the page alone are read
/// to compute the exact cosine. `$4` is the page size and `$5` its offset.
fn shortlist_sql() -> String {
    let candidate = pool::CANDIDATE
        .split(',')
        .map(|column| format!("p.{column}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "WITH {}, q AS MATERIALIZED (SELECT $3::text::vector v,binary_quantize($3::text::vector) b),
        shortlist AS MATERIALIZED (
          SELECT s.pool_ctid,s.entry_id,s.truncated FROM semantic_scoped s,q
          ORDER BY s.signature<~>q.b,s.kind,s.id,s.chunk_id NULLS FIRST LIMIT $4 OFFSET $5)
        SELECT {},(1-d)::real similarity,truncated
        FROM (SELECT {candidate},l.truncated,e.embedding<=>q.v d
          FROM shortlist l JOIN recall_pool p ON p.ctid=l.pool_ctid
          JOIN semantic_entries e ON e.id=l.entry_id AND e.brain_id=$1,q) x
        ORDER BY d,kind,id,chunk_id NULLS FIRST",
        scoped_cte(
            "m.ctid AS pool_ctid,m.kind,m.id,m.chunk_id,e.id AS entry_id,e.signature,e.truncated"
        ),
        pool::CANDIDATE,
    )
}

async fn fetch_exact(
    tx: &mut Tx<'_>,
    context: &ReadContext<'_>,
    profile: Uuid,
    encoded: &str,
    size: usize,
    offset: usize,
) -> Result<Vec<Scored>> {
    Ok(scoped(&exact_sql(), context, profile)
        .bind(encoded)
        .bind(size as i64)
        .bind(offset as i64)
        .fetch_all(&mut **tx)
        .await?)
}

async fn fetch_shortlist(
    tx: &mut Tx<'_>,
    context: &ReadContext<'_>,
    profile: Uuid,
    encoded: &str,
    size: usize,
    offset: usize,
) -> Result<Vec<Scored>> {
    Ok(scoped(&shortlist_sql(), context, profile)
        .bind(encoded)
        .bind(size as i64)
        .bind(offset as i64)
        .fetch_all(&mut **tx)
        .await?)
}

/// Give the planner statistics for the temporary candidate table, which has
/// none until it is analyzed. Only the two columns the scoped filter reads: a
/// full analyze samples the wide text columns and costs about as much as the
/// scoring statement itself, without changing its plan.
pub(super) async fn plan(tx: &mut Tx<'_>) -> Result<()> {
    sqlx::query("ANALYZE recall_pool(kind,status_eligible)")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Disable JIT for the semantic statements of this transaction. Their planner
/// cost estimates cross the compilation threshold although they run in tens
/// to hundreds of milliseconds, so compiling would only add latency.
pub(super) async fn tune(tx: &mut Tx<'_>) -> Result<()> {
    sqlx::query("SET LOCAL jit=off").execute(&mut **tx).await?;
    Ok(())
}

/// Which scoring statement a proof run executes.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub enum ProofTier {
    Exact,
    Shortlist,
}

/// Result of one proof run: the ranked identities with their cosine, the plan
/// text when it was requested, and the cost of preparing the candidate table.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct SemanticProof {
    pub ranked: Vec<(Uuid, f32)>,
    pub plan: Vec<String>,
    pub pool_plan: Vec<String>,
    pub sql: String,
    pub scoped: usize,
    pub pool_ms: u128,
    pub analyze_ms: u128,
}

/// Run one scoring statement exactly as recall builds it, with no bound check,
/// over the candidate pool of `selection`; `explain` returns its
/// `EXPLAIN (ANALYZE, BUFFERS)` plan instead of rows. A proof aid for the
/// recall-versus-exact and timing tests, not a product surface.
pub(super) async fn proof(
    tx: &mut Tx<'_>,
    brain: Uuid,
    selection: ScopeSelection,
    vector: &[f32],
    tier: ProofTier,
    limit: usize,
    explain: bool,
) -> Result<SemanticProof> {
    let input = RecallRequest {
        channels: vec!["semantic".into()],
        selection,
        ..RecallRequest::default()
    };
    let at: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    let context = ReadContext {
        brain,
        input: &input,
        at,
        manifest: None,
    };
    let profile = index::profile(tx, brain)
        .await?
        .ok_or_else(Error::missing)?;
    tune(tx).await?;
    let started = Instant::now();
    let breadth = pool::Breadth {
        claims: true,
        rest: true,
    };
    let mut pool_plan = vec![];
    if explain {
        pool_plan = pool::build_explained(tx, &context, breadth).await?;
    } else {
        pool::build(tx, &context, breadth, false).await?;
    }
    let pool_ms = started.elapsed().as_millis();
    let started = Instant::now();
    plan(tx).await?;
    let analyze_ms = started.elapsed().as_millis();
    let encoded =
        serde_json::to_string(vector).map_err(|_| Error::invalid("Invalid query vector."))?;
    let sql = match tier {
        ProofTier::Exact => exact_sql(),
        ProofTier::Shortlist => shortlist_sql(),
    };
    let mut out = SemanticProof {
        sql: sql.clone(),
        pool_plan,
        pool_ms,
        analyze_ms,
        ..SemanticProof::default()
    };
    if explain {
        let sql = format!("EXPLAIN (ANALYZE, BUFFERS) {sql}");
        out.plan = scoped::<(String,)>(&sql, &context, profile.id)
            .bind(&encoded)
            .bind(limit as i64)
            .bind(0i64)
            .fetch_all(&mut **tx)
            .await?
            .into_iter()
            .map(|(line,)| line)
            .collect();
        return Ok(out);
    }
    let rows: Vec<Scored> = match tier {
        ProofTier::Exact => fetch_exact(tx, &context, profile.id, &encoded, limit, 0).await?,
        ProofTier::Shortlist => {
            fetch_shortlist(tx, &context, profile.id, &encoded, limit, 0).await?
        }
    };
    out.scoped = rows.len();
    out.ranked = rows
        .into_iter()
        .map(|r| (r.candidate.id, r.similarity))
        .collect();
    Ok(out)
}

pub(super) struct Ranked {
    pub item: RecallItem,
    pub deadline: Option<DateTime<Utc>>,
    pub exact: bool,
    pub alternatives: Vec<(RecallItem, Option<DateTime<Utc>>)>,
    pub(super) lexical_rank: Option<usize>,
    pub(super) semantic_rank: Option<usize>,
    pub(super) graph_rank: Option<usize>,
}

pub(super) async fn ranked(
    state: &AppState,
    tx: &mut Tx<'_>,
    context: &ReadContext<'_>,
    baseline: &[Candidate],
    admission: Option<&mut Admission>,
    vector: Option<&[f32]>,
    coverage: &mut RecallCoverage,
) -> Result<Vec<Ranked>> {
    let mut identities: BTreeMap<(String, Uuid), Ranked> = BTreeMap::new();
    let mut lexical_scores = BTreeMap::<(String, Uuid), f32>::new();
    // Ranking operates on fully qualified canonical items before applying the
    // caller's context budget. Channel scores never establish eligibility.
    let mut gates = Gates::default();
    let page: Vec<&Candidate> = baseline.iter().take(CANDIDATES).collect();
    gates.load(state, tx, context, &page).await?;
    for row in page {
        let key = (row.kind.clone(), row.id);
        coverage.examined += 1;
        if let Some((mut item, deadline)) =
            item_with(state, tx, context, row, coverage, &mut gates).await?
        {
            for (matched, channel) in [(row.exact_match, "exact"), (row.lexical_match, "lexical")] {
                if matched && !item.channels.iter().any(|c| c == channel) {
                    item.channels.push(channel.into());
                }
            }
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
    if let (Some(vector), Some(admission)) = (vector, admission)
        && let Some(profile) = admission.status.profile.clone()
    {
        let encoded =
            serde_json::to_string(vector).map_err(|_| Error::invalid("Invalid query vector."))?;
        // Admission sized the scope in an earlier transaction and chose the
        // method; the statement that ranks confirms it.
        let bound = exact_limit(profile.dimensions);
        let mut method = admission.status.method.clone();
        let mut semantic_rank = 0;
        let mut reranked = 0;
        let mut scope_rows = 0;
        let mut offset = 0;
        let mut pages = 0;
        let mut exhausted = false;
        // The exact statement scores every scoped vector and returns it a page
        // at a time. The shortlist statement scores only its page, and pages
        // through the Hamming order while eligible identities are still missing.
        'pages: loop {
            let (rows, size) = if method == EXACT_COSINE {
                let rows =
                    fetch_exact(tx, context, profile.id, &encoded, EXACT_PAGE, offset).await?;
                if offset == 0 {
                    scope_rows = rows.first().map_or(0, |r| r.scope_rows) as usize;
                    if scope_rows > bound {
                        if context.input.semantic_exact {
                            return Err(model_policy::failure(
                                "semantic_scope_too_large",
                                "The semantic projection grew beyond this recall scope. Narrow the selection.",
                            ));
                        }
                        // The projection grew past the exact bound after admission.
                        method = SHORTLIST_RERANK.into();
                        continue 'pages;
                    }
                    reranked = scope_rows;
                }
                (rows, EXACT_PAGE)
            } else {
                let rows =
                    fetch_shortlist(tx, context, profile.id, &encoded, SHORTLIST, offset).await?;
                reranked += rows.len();
                (rows, SHORTLIST)
            };
            let more = if method == EXACT_COSINE {
                offset + rows.len() < scope_rows
            } else {
                rows.len() >= size
            };
            let approximate = method == SHORTLIST_RERANK;
            let mut limited = false;
            // Gates load one page at a time: rows beyond the 100-rank limit are
            // never qualified, and a page is dropped once it has been read.
            'rows: for (index, scored) in rows.iter().enumerate() {
                if index % 128 == 0 {
                    gates.release();
                    let page: Vec<&Candidate> = rows[index..]
                        .iter()
                        .take(128)
                        .map(|r| &r.candidate)
                        .collect();
                    gates.load(state, tx, context, &page).await?;
                }
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
                let row = &scored.candidate;
                let key = (row.kind.clone(), row.id);
                if identities.get(&key).is_some_and(|r| {
                    r.semantic_rank.is_some()
                        && r.alternatives
                            .iter()
                            .filter(|(i, _)| i.semantic_similarity.is_some())
                            .count()
                            >= 3
                }) {
                    continue;
                }
                coverage.examined += 1;
                let Some((mut item, deadline)) =
                    item_with(state, tx, context, row, coverage, &mut gates).await?
                else {
                    continue;
                };
                if item.kind == "source_version"
                    && item.provenance.iter().any(|p| p.availability != "retained")
                {
                    coverage.withheld += 1;
                    continue;
                }
                if let Some(ranked) = identities.get_mut(&key)
                    && ranked.semantic_rank.is_some()
                {
                    if !same_span(&ranked.item, &item) {
                        item.channels = vec!["semantic".into()];
                        item.semantic_similarity = Some(similarity);
                        item.qualifications
                            .push("semantic_similarity_not_truth".into());
                        if approximate {
                            item.qualifications
                                .push("semantic_shortlist_approximate".into());
                        }
                        if scored.truncated {
                            item.qualifications
                                .push("semantic_representation_truncated".into());
                            note(coverage, "semantic_representation_truncated");
                        }
                        if let Some((other, until)) = ranked
                            .alternatives
                            .iter_mut()
                            .find(|(other, _)| same_span(other, &item))
                        {
                            for channel in &other.channels {
                                if !item.channels.contains(channel) {
                                    item.channels.push(channel.clone());
                                }
                            }
                            *other = item;
                            *until = [*until, deadline].into_iter().flatten().min();
                        } else {
                            ranked.alternatives.push((item, deadline));
                        }
                        // Semantic windows must not be crowded out by an earlier
                        // lexical-only page. Retain their own scores and channels.
                        ranked
                            .alternatives
                            .sort_by_key(|(i, _)| i.semantic_similarity.is_none());
                        ranked.alternatives.truncate(4);
                    }
                    continue;
                }
                if semantic_rank == CANDIDATES {
                    note(coverage, "semantic_candidate_limit");
                    limited = true;
                    break 'rows;
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
                        if !same_span(&ranked.item, &item) {
                            let previous = std::mem::replace(&mut ranked.item, item);
                            ranked
                                .alternatives
                                .retain(|(other, _)| !same_span(other, &ranked.item));
                            ranked.alternatives.insert(0, (previous, ranked.deadline));
                            ranked.alternatives.truncate(4);
                        }
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
                if approximate {
                    ranked
                        .item
                        .qualifications
                        .push("semantic_shortlist_approximate".into());
                }
                if scored.truncated {
                    ranked
                        .item
                        .qualifications
                        .push("semantic_representation_truncated".into());
                    note(coverage, "semantic_representation_truncated");
                }
            }
            if limited || !more {
                break;
            }
            if method == SHORTLIST_RERANK {
                pages += 1;
                if pages == SHORTLIST_PAGES {
                    exhausted = true;
                    break;
                }
            }
            offset += size;
        }
        if method == SHORTLIST_RERANK {
            note(coverage, "semantic_shortlist_approximate");
            if exhausted {
                note(coverage, "semantic_shortlist_exhausted");
            }
        }
        admission.status.method = method;
        admission.status.reranked = reranked;
    }
    let mut ranked: Vec<Ranked> = identities.into_values().collect();
    sort(&mut ranked, context.input);
    Ok(ranked)
}

fn same_span(a: &RecallItem, b: &RecallItem) -> bool {
    a.kind == b.kind
        && a.revision_id == b.revision_id
        && a.provenance
            .iter()
            .map(|p| (p.id, p.byte_from, p.byte_to))
            .eq(b.provenance.iter().map(|p| (p.id, p.byte_from, p.byte_to)))
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
