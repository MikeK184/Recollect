//! Bounded retained-vector preparation. Only exact scalar affinities leave
//! PostgreSQL; vectors and centroids never become a second persisted store.
use super::{affinity, storage};
use crate::db::admission::{self, BrainAccess};
use crate::{
    AppState,
    auth::Auth,
    error::{Error, Result},
    memory_evidence::Tx,
    model_gateway, model_policy, semantic,
};
use chrono::{DateTime, Utc};
use recollect_protocol::{GraphSelection, SemanticProfile};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Json;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const RECIPE: &str = "source-idf-retained-vector-leiden-1";
pub const SOURCES: usize = 250;
/// Frozen candidate parameters. The only experimental alternatives are the
/// predeclared development grid; no held-out-driven configuration is accepted.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HybridParameters {
    pub neighbors: usize,
    pub minimum_cosine: f64,
    pub vector_scale: f64,
}
impl Default for HybridParameters {
    fn default() -> Self {
        Self {
            neighbors: 5,
            minimum_cosine: 0.75,
            vector_scale: 1.0,
        }
    }
}
impl HybridParameters {
    pub fn valid(self) -> bool {
        [3, 5, 8].contains(&self.neighbors)
            && [0.65, 0.75, 0.85].contains(&self.minimum_cosine)
            && [0.25, 0.5, 1.0].contains(&self.vector_scale)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SourceInput {
    pub id: Uuid,
    pub source: Uuid,
    pub version: Uuid,
    pub context: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorDependency {
    pub entry: Uuid,
    pub profile: Uuid,
    pub request: Uuid,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Features {
    pub sources: Vec<SourceInput>,
    pub vectors: Vec<VectorDependency>,
    pub coverage: String,
    pub deadline: Option<DateTime<Utc>>,
}
fn corrupt() -> Error {
    Error(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        "organization_vector_corrupt",
        "The retained vector provenance or dimensions are inconsistent.",
    )
}

/// Credential availability governs new provider calls, not use of retained
/// compatible projections. This predicate deliberately never calls permits().
pub fn retained_policy(
    policy: &recollect_protocol::ModelPolicy,
    profile: &SemanticProfile,
    classes: &[String],
) -> bool {
    policy.enabled
        && semantic::compatible(policy, profile)
        && policy.purposes.iter().any(|p| p == "embedding")
        && classes.iter().all(|c| policy.content_classes.contains(c))
}

/// Returns None for an honest whole-cohort feature fallback. The caller keeps
/// its complete entity projection; it must not allocate a first-250 prefix.
pub async fn prepare(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    mapped: &[(Uuid, Uuid)],
    expected: (&str, &str, i32, &str),
    parameters: HybridParameters,
) -> Result<(Option<(Features, affinity::Projection)>, String)> {
    if !parameters.valid() {
        return Err(corrupt());
    }
    if mapped.len() > SOURCES {
        return Ok((None, "budget_sources".into()));
    }
    let Some(profile) = semantic::profile(tx, brain).await? else {
        return Ok((None, "missing_profile".into()));
    };
    let policy = model_policy::current(state, tx, brain).await?;
    if (
        profile.provider.as_str(),
        profile.model.as_str(),
        profile.dimensions,
        profile.representation.as_str(),
    ) != expected
        || !retained_policy(&policy.policy, &profile, &[])
    {
        return Ok((None, "incompatible_policy".into()));
    }
    // A bound before canonical checks causes only whole-feature fallback, never
    // omitted eligible contributors or a fabricated partial vector cohort.
    let candidates:Vec<(Uuid,Uuid)>=sqlx::query_as("SELECT s.id,s.current_version FROM sources s WHERE s.brain_id=$1 AND EXISTS(SELECT 1 FROM semantic_entries e WHERE e.brain_id=s.brain_id AND e.source_version_id=s.current_version AND e.profile_id=$2 AND e.kind='source_chunk') ORDER BY s.id LIMIT 10001")
        .bind(brain).bind(profile.id).fetch_all(&mut **tx).await?;
    if candidates.len() > 10000 {
        return Ok((None, "budget_candidates".into()));
    }
    let mut scope: GraphSelection =
        serde_json::from_value(serde_json::json!({"kind":"knowledge"})).map_err(|_| corrupt())?;
    crate::retrieval::graph::authorize(tx, auth, brain, &scope).await?;
    let at: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    let mut versions: BTreeMap<Uuid, Uuid> = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT id,source_id FROM source_versions WHERE brain_id=$1 AND id=ANY($2)",
    )
    .bind(brain)
    .bind(mapped.iter().map(|(_, v)| *v).collect::<Vec<_>>())
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .collect();
    let mut deadline = None;
    for page in candidates.chunks(250) {
        let entities: Vec<_> = page
            .iter()
            .map(|(s, v)| crate::graph::Entity {
                kind: "source_version".into(),
                id: *s,
                revision_id: *v,
            })
            .collect();
        let qualified = crate::retrieval::graph::qualify(
            state,
            tx,
            auth,
            brain,
            &mut scope,
            &entities,
            (250, Some(at)),
        )
        .await?;
        deadline = [deadline, qualified.deadline].into_iter().flatten().min();
        for (s, v) in page {
            if qualified
                .nodes
                .contains_key(&crate::graph::key("source_version", *v))
            {
                versions.insert(*v, *s);
            }
        }
        if versions.len() > SOURCES {
            return Ok((None, "budget_sources".into()));
        }
    }
    let mut sources = vec![];
    for (version, source) in &versions {
        let context = storage::context(tx, brain, *version, *source).await?;
        let existing:Option<Uuid>=sqlx::query_scalar("SELECT id FROM knowledge_source_inputs WHERE brain_id=$1 AND source_version_id=$2 AND context=$3 ORDER BY id LIMIT 1").bind(brain).bind(version).bind(Json(&context)).fetch_optional(&mut **tx).await?;
        sources.push(SourceInput {
            id: existing.unwrap_or_else(Uuid::new_v4),
            source: *source,
            version: *version,
            context,
        });
    }
    let retained = retained_for_versions(
        state,
        tx,
        brain,
        &versions.keys().copied().collect::<Vec<_>>(),
    )
    .await?;
    if retained.budget {
        return Ok((None, retained.coverage));
    }
    let vectors = retained.entries;
    let missing = sources.len().saturating_sub(retained.available.len());
    // SQL owns normalized centroids and cosine. Only edge scalars are returned.
    let entry_ids: Vec<_> = vectors.iter().map(|v| v.entry).collect();
    let (zero, edges) = cosine_edges(tx, brain, &entry_ids, parameters).await?;
    let remap: BTreeMap<_, _> = sources.iter().map(|s| (s.version, s.id)).collect();
    let mapped_ids: Vec<_> = mapped.iter().map(|(i, _)| *i).collect();
    let associations:Vec<(Uuid,Uuid,Uuid,String)>=sqlx::query_as("SELECT DISTINCT i.id,i.source_version_id,m.entity_id,e.kind FROM knowledge_mapping_inputs i JOIN knowledge_mentions m ON m.input_id=i.id AND m.brain_id=i.brain_id JOIN knowledge_entities e ON e.id=m.entity_id AND e.brain_id=m.brain_id WHERE i.brain_id=$1 AND i.id=ANY($2) AND e.kind IN('technology','concept') ORDER BY i.id,m.entity_id")
        .bind(brain).bind(mapped_ids).fetch_all(&mut **tx).await?;
    let inputs: Vec<_> = sources.iter().map(|s| (s.id, s.version)).collect();
    let a: Vec<_> = associations
        .into_iter()
        .map(|(_, v, e, k)| affinity::Association {
            input_id: remap[&v],
            source_version_id: v,
            entity_id: e,
            kind: if k == "technology" {
                super::EntityKind::Technology
            } else {
                super::EntityKind::Concept
            },
            boilerplate: false,
        })
        .collect();
    let projection = project(&inputs, &a, &edges, parameters)?;
    let coverage = if zero {
        "zero_centroid".into()
    } else if missing > 0 {
        format!("partial_features:{missing}")
    } else {
        "complete".into()
    };
    let features = Features {
        sources,
        vectors,
        coverage,
        deadline,
    };
    Ok((Some((features, projection)), "prepared".into()))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RetainedFeatures {
    pub profile: Option<Uuid>,
    pub entries: Vec<VectorDependency>,
    pub available: BTreeSet<Uuid>,
    pub coverage: String,
    pub budget: bool,
}

/// Shared exact retained-input qualification for production and unpublished
/// experiments. Canonical source qualification precedes this helper in both.
pub async fn retained_for_versions(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    versions: &[Uuid],
) -> Result<RetainedFeatures> {
    if versions.len() > SOURCES || versions.iter().collect::<BTreeSet<_>>().len() != versions.len()
    {
        return Err(corrupt());
    }
    let unavailable = |reason: &str, budget: bool| RetainedFeatures {
        profile: None,
        entries: vec![],
        available: BTreeSet::new(),
        coverage: reason.into(),
        budget,
    };
    let Some(profile) = semantic::profile(tx, brain).await? else {
        return Ok(unavailable("missing_profile", false));
    };
    let policy = model_policy::current(state, tx, brain).await?;
    if !retained_policy(&policy.policy, &profile, &[]) {
        return Ok(unavailable("incompatible_policy", false));
    }
    let chunk_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM source_chunks WHERE brain_id=$1 AND version_id=ANY($2)",
    )
    .bind(brain)
    .bind(versions)
    .fetch_one(&mut **tx)
    .await?;
    if chunk_count > 1000 {
        return Ok(unavailable("budget_chunks", true));
    }
    let mut vectors = vec![];
    let mut available = BTreeSet::new();
    let mut missing = 0usize;
    for &version in versions {
        type Chunk = (
            Uuid,
            i32,
            i32,
            String,
            Option<Uuid>,
            Option<Uuid>,
            Option<String>,
            Option<bool>,
            Option<i32>,
            Option<bool>,
            Option<bool>,
            Option<String>,
            Option<bool>,
            Option<bool>,
            Option<String>,
        );
        let chunks:Vec<Chunk>=sqlx::query_as("SELECT c.id,c.byte_start,c.byte_end,c.content,e.id,e.request_id,e.state,e.truncated,vector_dims(e.embedding),vector_norm(e.embedding)>0,(e.input_id=c.id AND e.source_version_id=c.version_id AND q.purpose='embedding' AND (q.detail_expired OR(q.provider=$4 AND q.model=$5 AND q.dimensions=$6 AND q.prompt_label='engineering-text-1' AND q.schema_label='embedding-float-1')) AND EXISTS(SELECT 1 FROM model_request_inputs qi WHERE qi.brain_id=q.brain_id AND qi.request_id=q.id AND qi.kind='source_version' AND qi.input_id=$3) AND EXISTS(SELECT 1 FROM model_request_inputs qi WHERE qi.brain_id=q.brain_id AND qi.request_id=q.id AND qi.kind='semantic_entry' AND qi.input_id=e.id)),q.returned_model,q.detail_expired,q.suppressed,q.state FROM source_chunks c LEFT JOIN semantic_entries e ON e.brain_id=c.brain_id AND e.chunk_id=c.id AND e.profile_id=$2 AND e.kind='source_chunk' LEFT JOIN model_requests q ON q.brain_id=e.brain_id AND q.id=e.request_id WHERE c.brain_id=$1 AND c.version_id=$3 ORDER BY c.id LIMIT 17")
            .bind(brain).bind(profile.id).bind(version).bind(&profile.provider).bind(&profile.model).bind(profile.dimensions).fetch_all(&mut **tx).await?;
        if chunks.len() > 16 {
            return Ok(RetainedFeatures {
                profile: Some(profile.id),
                entries: vec![],
                available: BTreeSet::new(),
                coverage: "budget_chunks_per_source".into(),
                budget: true,
            });
        }
        if chunks.is_empty() {
            missing += 1;
            continue;
        }
        if chunks
            .iter()
            .any(|c| c.6.as_deref() != Some("ready") || c.7 != Some(false))
        {
            missing += 1;
            continue;
        }
        if chunks
            .iter()
            .any(|c| c.13 == Some(true) || c.14.as_deref().is_some_and(|s| s != "succeeded"))
        {
            missing += 1;
            continue;
        }
        if chunks.iter().any(|c| {
            c.8 != Some(profile.dimensions)
                || c.9 != Some(true)
                || c.10 != Some(true)
                || c.4.is_none()
                || c.5.is_none()
                || (c.12 != Some(true)
                    && !model_policy::catalogue::returned_model_matches(
                        &profile.provider,
                        &profile.model,
                        c.11.as_deref().unwrap_or_default(),
                        true,
                    ))
        }) {
            return Err(corrupt());
        }
        let source =
            model_gateway::source(state, tx, brain, version, crate::artifacts::MAX_TEXT).await?;
        let mut classes = vec![source.class.clone()];
        if let Some(original) = source.provenance["original_content_class"].as_str() {
            classes.push(original.into());
        }
        if !retained_policy(&policy.policy, &profile, &classes) {
            missing += 1;
            continue;
        }
        for c in &chunks {
            let exact = usize::try_from(c.1)
                .ok()
                .zip(usize::try_from(c.2).ok())
                .and_then(|(s, e)| source.text.get(s..e));
            if exact != Some(c.3.as_str()) || c.3.is_empty() || c.3.len() > 4096 {
                return Err(corrupt());
            }
        }
        available.insert(version);
        vectors.extend(chunks.iter().map(|c| VectorDependency {
            entry: c.4.unwrap(),
            profile: profile.id,
            request: c.5.unwrap(),
        }));
    }
    Ok(RetainedFeatures {
        profile: Some(profile.id),
        entries: vectors,
        available,
        coverage: if missing == 0 {
            "complete".into()
        } else {
            format!("partial_features:{missing}")
        },
        budget: false,
    })
}

/// Computes only scalar affinities from an already exactly qualified retained
/// cohort. The caller owns canonical source/chunk/profile/provenance checks.
/// Vectors and centroids stay in PostgreSQL, including experimental evaluation.
pub async fn cosine_edges(
    tx: &mut Tx<'_>,
    brain: Uuid,
    entry_ids: &[Uuid],
    parameters: HybridParameters,
) -> Result<(bool, Vec<(Uuid, Uuid, f64)>)> {
    if !parameters.valid()
        || entry_ids.len() > 1000
        || entry_ids.iter().collect::<BTreeSet<_>>().len() != entry_ids.len()
    {
        return Err(corrupt());
    }
    admission::admit_in(tx, brain, &BrainAccess::READ).await?;
    let zero:bool=sqlx::query_scalar("WITH c AS(SELECT source_version_id,l2_normalize(avg(l2_normalize(embedding) ORDER BY chunk_id)) v FROM semantic_entries WHERE brain_id=$1 AND id=ANY($2) GROUP BY source_version_id) SELECT EXISTS(SELECT 1 FROM c WHERE vector_norm(v)=0)")
        .bind(brain).bind(entry_ids).fetch_one(&mut **tx).await?;
    if zero {
        return Ok((true, vec![]));
    }
    let edges:Vec<(Uuid,Uuid,f64)>=sqlx::query_as("WITH c AS MATERIALIZED(SELECT source_version_id id,l2_normalize(avg(l2_normalize(embedding) ORDER BY chunk_id)) v FROM semantic_entries WHERE brain_id=$1 AND id=ANY($2) GROUP BY source_version_id), pairs AS MATERIALIZED(SELECT a.id a,b.id b,1-(a.v<=>b.v) similarity FROM c a CROSS JOIN c b WHERE a.id<>b.id), ranked AS(SELECT *,row_number() OVER(PARTITION BY a ORDER BY similarity DESC,b) rank FROM pairs WHERE similarity>=$3) SELECT a.a,a.b,a.similarity FROM ranked a JOIN ranked b ON a.a=b.b AND a.b=b.a WHERE a.rank<=$4 AND b.rank<=$4 AND a.a<a.b ORDER BY a.a,a.b")
        .bind(brain).bind(entry_ids).bind(parameters.minimum_cosine).bind(parameters.neighbors as i64).fetch_all(&mut **tx).await?;
    Ok((false, edges))
}

/// Shared deterministic feature construction, not a clustering algorithm. Both
/// production and unpublished quality experiments feed this exact function to
/// native Leiden. Labels and raw vectors are absent from its interface.
pub fn project(
    inputs: &[(Uuid, Uuid)],
    associations: &[affinity::Association],
    edges: &[(Uuid, Uuid, f64)],
    parameters: HybridParameters,
) -> Result<affinity::Projection> {
    if !parameters.valid() || inputs.len() > SOURCES {
        return Err(corrupt());
    }
    let remap: BTreeMap<_, _> = inputs.iter().map(|(i, v)| (*v, *i)).collect();
    let mut projection = affinity::prepare(inputs, associations).map_err(|_| corrupt())?;
    let mut terms = BTreeMap::new();
    for edge in &projection.edges {
        if let affinity::Vertex::Term { id } = projection.nodes[edge.to] {
            terms.insert(id, edge.weight);
        }
    }
    let mut idfs: Vec<_> = terms.into_values().collect();
    idfs.sort_by(f64::total_cmp);
    let median = if idfs.is_empty() {
        1.0
    } else if idfs.len() % 2 == 0 {
        (idfs[idfs.len() / 2 - 1] + idfs[idfs.len() / 2]) / 2.0
    } else {
        idfs[idfs.len() / 2]
    };
    let scale = parameters.vector_scale * median;
    let mut pairs = BTreeSet::new();
    for &(a, b, weight) in edges {
        if a >= b
            || !remap.contains_key(&a)
            || !remap.contains_key(&b)
            || !weight.is_finite()
            || weight < parameters.minimum_cosine
            || weight > 1.000001
            || !pairs.insert((a, b))
        {
            return Err(corrupt());
        }
    }
    let connected: BTreeSet<_> = edges.iter().flat_map(|(a, b, _)| [*a, *b]).collect();
    let mut ordered = inputs.to_vec();
    ordered.sort();
    for (id, version) in ordered {
        if connected.contains(&version)
            && !projection
                .nodes
                .iter()
                .any(|v| matches!(v,affinity::Vertex::Input{id:i,..} if *i==id))
        {
            projection.nodes.push(affinity::Vertex::Input {
                id,
                source_version_id: version,
            });
        }
    }
    let positions: BTreeMap<_, _> = projection
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            affinity::Vertex::Input { id, .. } => Some((*id, i)),
            _ => None,
        })
        .collect();
    for &(a, b, weight) in edges {
        projection.edges.push(affinity::WeightedEdge {
            from: positions[&remap[&a]],
            to: positions[&remap[&b]],
            weight: weight * scale,
        });
    }
    if projection.nodes.len() > 10000 || projection.edges.len() > 50000 {
        return Err(corrupt());
    }
    projection.recipe = RECIPE.into();
    projection.unassigned_versions = inputs
        .iter()
        .filter(|(i, _)| !positions.contains_key(i))
        .map(|(_, v)| *v)
        .collect();
    Ok(projection)
}

/// Caller holds the freshly authenticated Brain writer. Provisional IDs are
/// never published before exact context/version/retention gates are repeated.
pub async fn persist(tx: &mut Tx<'_>, brain: Uuid, actor: Uuid, features: &Features) -> Result<()> {
    admission::admit_in(tx, brain, &BrainAccess::WRITE).await?;
    let duplicate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jsonb_to_recordset($2) x(id uuid,source uuid,version uuid,context jsonb) JOIN knowledge_source_inputs i ON i.brain_id=$1 AND i.source_version_id=x.version AND i.context=x.context AND i.id<>x.id)").bind(brain).bind(Json(&features.sources)).fetch_one(&mut **tx).await?;
    if duplicate {
        return Err(corrupt());
    }
    sqlx::query("INSERT INTO knowledge_source_inputs(id,brain_id,source_id,source_version_id,context,created_by) SELECT id,$1,source,version,context,$2 FROM jsonb_to_recordset($3) x(id uuid,source uuid,version uuid,context jsonb) ON CONFLICT(id) DO NOTHING")
        .bind(brain).bind(actor).bind(Json(&features.sources)).execute(&mut **tx).await?;
    let valid: i64 =
        sqlx::query_scalar("SELECT count(*) FROM recollect_organization_current_source_ids($1,$2)")
            .bind(brain)
            .bind(features.sources.iter().map(|s| s.id).collect::<Vec<_>>())
            .fetch_one(&mut **tx)
            .await?;
    if valid as usize != features.sources.len() {
        return Err(corrupt());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hybrid_projection_uses_distinct_term_median_and_never_invents_entities() {
        let inputs: Vec<_> = (1..=4)
            .map(|n| (Uuid::from_u128(n), Uuid::from_u128(100 + n)))
            .collect();
        let mut associations = vec![];
        for (term, indices) in [(201, vec![0]), (202, vec![0, 1]), (203, vec![0, 1, 2, 3])] {
            for i in indices {
                associations.push(affinity::Association {
                    input_id: inputs[i].0,
                    source_version_id: inputs[i].1,
                    entity_id: Uuid::from_u128(term),
                    kind: super::super::EntityKind::Technology,
                    boilerplate: false,
                });
            }
        }
        let parameters = HybridParameters {
            vector_scale: 0.5,
            ..Default::default()
        };
        let p = project(
            &inputs,
            &associations,
            &[(inputs[0].1, inputs[1].1, 0.8)],
            parameters,
        )
        .unwrap_or_else(|e| panic!("{}", e.1));
        assert!((p.edges.last().unwrap().weight - 0.8 * 0.5 * 3_f64.ln()).abs() < 1e-12);
        let empty = project(&inputs, &[], &[], parameters).unwrap_or_else(|e| panic!("{}", e.1));
        assert!(empty.nodes.is_empty());
        assert_eq!(empty.unassigned_versions.len(), 4);
        let vectors = project(&inputs, &[], &[(inputs[0].1, inputs[1].1, 0.8)], parameters)
            .unwrap_or_else(|e| panic!("{}", e.1));
        assert_eq!(vectors.nodes.len(), 2);
        assert!(
            vectors
                .nodes
                .iter()
                .all(|n| matches!(n, affinity::Vertex::Input { .. }))
        );
        assert_eq!(vectors.unassigned_versions.len(), 2);
    }
    #[test]
    fn hybrid_projection_rejects_foreign_duplicate_nonfinite_and_unfrozen_parameters() {
        let inputs = vec![
            (Uuid::from_u128(1), Uuid::from_u128(101)),
            (Uuid::from_u128(2), Uuid::from_u128(102)),
        ];
        for edges in [
            vec![(inputs[0].1, Uuid::from_u128(999), 0.9)],
            vec![(inputs[0].1, inputs[1].1, f64::NAN)],
            vec![(inputs[0].1, inputs[1].1, 0.9); 2],
        ] {
            assert!(project(&inputs, &[], &edges, Default::default()).is_err());
        }
        assert!(
            project(
                &inputs,
                &[],
                &[],
                HybridParameters {
                    neighbors: 4,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            project(
                &inputs,
                &[],
                &[],
                HybridParameters {
                    minimum_cosine: f64::INFINITY,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}

pub(crate) async fn canonical_context(
    tx: &mut Tx<'_>,
    brain: Uuid,
    version: Uuid,
    source: Uuid,
) -> Result<Value> {
    storage::context(tx, brain, version, source).await
}
pub(crate) async fn canonical_gate(
    tx: &mut Tx<'_>,
    brain: Uuid,
    version: Uuid,
) -> Result<Option<DateTime<Utc>>> {
    storage::input_gate(tx, brain, version).await
}
