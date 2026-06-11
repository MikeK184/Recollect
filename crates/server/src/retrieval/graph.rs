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
        include_str!("../retrieval_candidates.sql")
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
        .bind((node_limit + 1) as i64)
        .fetch_all(&mut **tx)
        .await?;
    if rows.len() > node_limit {
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
