//! Graph consumers share the canonical read gates, without invoking ranking or
//! a model. Every selected source fragment participates in vertex eligibility.
use super::*;
use crate::graph::{self, Entity};

pub(crate) struct Qualified {
    pub nodes: BTreeMap<String, GraphNode>,
    pub scope_id: Option<Uuid>,
    pub coverage: RecallCoverage,
    pub deadline: Option<DateTime<Utc>>,
}

// Narrow graph representations before support checks and source-chunk fan-out.
// Latest-at-time CTEs retain entire histories for the selected logical IDs:
// an old descriptor must never make an older revision the latest eligible one.
// MATERIALIZED prevents expensive canonical gates being evaluated outside the
// requested entity window. These comments are no-ops for ordinary recall SQL.
fn candidate_sql() -> String {
    const REQUESTED: &str = r#"
graph_requested_claims AS MATERIALIZED (
  SELECT r.claim_id FROM unnest($11::text[]) key CROSS JOIN LATERAL (
    SELECT claim_id FROM claim_revisions WHERE brain_id=$1
      AND id=CASE WHEN starts_with(key,'claim:') THEN split_part(key,':',2)::uuid END LIMIT 1
  ) r WHERE starts_with(key,'claim:')
), graph_requested_sources AS MATERIALIZED (
  SELECT v.source_id FROM unnest($11::text[]) key CROSS JOIN LATERAL (
    SELECT source_id FROM source_versions WHERE brain_id=$1
      AND id=CASE WHEN starts_with(key,'source_version:') THEN split_part(key,':',2)::uuid END LIMIT 1
  ) v WHERE starts_with(key,'source_version:')
),
"#;
    const WINDOWS: &str = r#"
graph_claims AS MATERIALIZED (
  SELECT r.* FROM known_claims r WHERE r.id IN (
    SELECT split_part(key,':',2)::uuid FROM unnest($11::text[]) key WHERE starts_with(key,'claim:'))
), graph_sources AS MATERIALIZED (
  SELECT v.* FROM source_knowledge v WHERE v.id IN (
    SELECT split_part(key,':',2)::uuid FROM unnest($11::text[]) key WHERE starts_with(key,'source_version:'))
), graph_facts AS MATERIALIZED (
  SELECT f.* FROM repository_facts f WHERE f.brain_id=$1 AND f.id IN (
    SELECT split_part(key,':',2)::uuid FROM unnest($11::text[]) key WHERE starts_with(key,'repository_fact:'))
), graph_manifests AS MATERIALIZED (
  SELECT r.* FROM manifest_revisions r WHERE r.brain_id=$1 AND r.id IN (
    SELECT split_part(key,':',2)::uuid FROM unnest($11::text[]) key WHERE starts_with(key,'manifest_revision:'))
),
"#;
    // This CTE must precede selected_claims (which consumes graph_claims).
    include_str!("../retrieval_candidates.sql")
        .replace("/* graph_requested_ctes */", REQUESTED)
        // Narrow logical identities, retaining their ENTIRE version histories.
        // Filtering the exact revision before latest selection would resurrect
        // stale evidence. Capture associations keep the same source boundary.
        .replace(
            "/* graph_window:capture_sources */",
            "AND e.source_id IN (SELECT source_id FROM graph_requested_sources)",
        )
        .replace(
            "/* graph_window:source_knowledge */",
            "AND v.source_id IN (SELECT source_id FROM graph_requested_sources)",
        )
        .replace(
            "/* graph_window:claim_histories */",
            "AND claim_id IN (SELECT claim_id FROM graph_requested_claims)",
        )
        .replace("/* graph_window_ctes */", WINDOWS)
        .replace("/* graph_window:claims */ known_claims", "graph_claims")
        .replace(
            "/* graph_window:sources */ source_knowledge",
            "graph_sources",
        )
        .replace("/* graph_window:facts */ repository_facts", "graph_facts")
        .replace(
            "/* graph_window:manifests */ manifest_revisions",
            "graph_manifests",
        )
}

pub(crate) async fn qualify(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    scope: &mut GraphSelection,
    entities: &[Entity],
    bounds: (usize, Option<DateTime<Utc>>),
) -> Result<Qualified> {
    let (node_limit, at) = bounds;
    let mut input = RecallRequest {
        query: "Graph selection".into(),
        selection: scope.selection.clone(),
        operation_id: scope.operation_id,
        collection_id: scope.collection_id,
        manifest_revision_id: scope.manifest_revision_id,
        fact_at: scope.fact_at,
        mode: scope.mode.clone(),
        ..Default::default()
    };
    validate(&mut input)?;
    scope.selection = input.selection.clone();
    let scope_id = authority(tx, auth, brain, &input).await?;
    let at: DateTime<Utc> = match at {
        Some(at) => at,
        None => {
            sqlx::query_scalar("SELECT clock_timestamp()")
                .fetch_one(&mut **tx)
                .await?
        }
    };
    let manifest = selected_manifest(tx, brain, &input, at).await?;
    // An exact snapshot outside an environment is an explicit immutable input,
    // not a fabricated public manifest. Only the structural candidate branch
    // consumes this internal selector; provenance still records the snapshot.
    let selected = if scope.kind == "repository" && manifest.is_none() {
        let snapshot = scope.snapshot_id.ok_or_else(Error::missing)?;
        let row = publication::snapshot(tx, brain, snapshot).await?;
        if input.selection.environment_id.is_some()
            || input.collection_id.is_some()
            || (!input.selection.repository_ids.is_empty()
                && !input.selection.repository_ids.contains(&row.repository_id))
        {
            return Err(Error::invalid(
                "This snapshot is outside the explicit repository/environment selection.",
            ));
        }
        Some(
            json!({"entries":[{"repository_id":row.repository_id,"snapshot_id":row.id,"revision":row.revision}]}),
        )
    } else {
        manifest.as_ref().map(|m| json!(m))
    };
    if let (Some(manifest), Some(snapshot)) = (&manifest, scope.snapshot_id)
        && !manifest
            .entries
            .iter()
            .any(|e| e.snapshot_id == Some(snapshot))
    {
        return Err(Error::invalid(
            "The snapshot is not part of the selected manifest revision.",
        ));
    }
    let keys: Vec<_> = entities.iter().map(Entity::key).collect();
    let sql = format!(
        "{} SELECT kind,id,revision_id,chunk_id,label,text,recorded_at,selection,source_id,repository_id,snapshot_id,revision,path,line_from,line_to,byte_from,byte_to,artifact_id,byte_length,processing,data,expires_at,exact_match,lexical_match,rank FROM matched WHERE status_eligible AND kind||':'||revision_id::text=ANY($11) ORDER BY kind,id,chunk_id NULLS FIRST LIMIT $12",
        candidate_sql()
    );
    let rows: Vec<Candidate> = sqlx::query_as(&sql)
        .bind(brain)
        .bind(at)
        .bind("")
        .bind(SqlJson(&input.selection))
        .bind(input.collection_id)
        .bind(selected)
        .bind(&input.mode)
        .bind(Option::<String>::None)
        .bind(Option::<Uuid>::None)
        .bind(Vec::<String>::new())
        .bind(keys)
        .bind(32_001_i64)
        .fetch_all(&mut **tx)
        .await?;
    if rows.len() > 32_000
        || rows
            .iter()
            .map(|r| (r.kind.as_str(), r.revision_id))
            .collect::<BTreeSet<_>>()
            .len()
            > node_limit
    {
        return Err(graph::selection_limit(node_limit));
    }
    let context = ReadContext {
        brain,
        input: &input,
        at,
        manifest: manifest.as_ref(),
    };
    let mut result = Qualified {
        nodes: BTreeMap::new(),
        scope_id,
        coverage: RecallCoverage::default(),
        deadline: None,
    };
    let mut withheld = BTreeSet::new();
    let source_ids: Vec<_> = entities
        .iter()
        .filter(|e| e.kind == "source_version")
        .map(|e| e.revision_id)
        .collect();
    let incomplete: Vec<Uuid> = sqlx::query_scalar("SELECT v.id FROM source_versions v WHERE v.brain_id=$1 AND v.id=ANY($2) AND (v.processing<>'ready' OR coalesce((SELECT sum(c.byte_end-c.byte_start) FROM source_chunks c WHERE c.brain_id=v.brain_id AND c.version_id=v.id),0)<>v.byte_length)")
        .bind(brain).bind(source_ids).fetch_all(&mut **tx).await?;
    withheld.extend(
        incomplete
            .into_iter()
            .map(|id| graph::key("source_version", id)),
    );
    let blocked = raw_blocked(tx, brain, &input, &rows).await?;
    // This cache cannot outlive the current canonical Brain lock/qualification.
    let mut scopes = BTreeMap::new();
    for (ordinal, row) in rows.into_iter().enumerate() {
        result.coverage.examined += 1;
        let key = graph::key(&row.kind, row.revision_id);
        let scope_key = serde_json::to_string(&row.selection.0)
            .map_err(|_| Error::invalid("Stored graph scope is invalid."))?;
        let scope_valid = if let Some(valid) = scopes.get(&scope_key) {
            *valid
        } else {
            let valid = workspace::selection_valid(tx, brain, &row.selection).await?;
            scopes.insert(scope_key, valid);
            valid
        };
        let entry = item_with_gates(
            state,
            tx,
            &context,
            &row,
            &mut result.coverage,
            Some((scope_valid, blocked.contains(&ordinal))),
        )
        .await?;
        match entry {
            Some((mut evidence, deadline))
                if evidence.kind != "source_version"
                    || evidence
                        .provenance
                        .iter()
                        .all(|p| p.availability == "retained") =>
            {
                result.deadline = [result.deadline, deadline].into_iter().flatten().min();
                evidence.channels = vec!["graph".into()];
                evidence.score = 0.0;
                result
                    .nodes
                    .entry(key.clone())
                    .or_insert(GraphNode { key, evidence });
            }
            _ => {
                withheld.insert(key);
            }
        }
    }
    for key in withheld {
        result.nodes.remove(&key);
    }
    Ok(result)
}

pub(crate) async fn authorize(
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    scope: &GraphSelection,
) -> Result<()> {
    let mut input = RecallRequest {
        query: "Graph selection".into(),
        selection: scope.selection.clone(),
        operation_id: scope.operation_id,
        collection_id: scope.collection_id,
        manifest_revision_id: scope.manifest_revision_id,
        fact_at: scope.fact_at,
        mode: scope.mode.clone(),
        ..Default::default()
    };
    validate(&mut input)?;
    authority(tx, auth, brain, &input).await?;
    selected_manifest(tx, brain, &input, Utc::now()).await?;
    Ok(())
}
