//! Bounded affinity preparation and persisted-topic matching, not a clustering
//! algorithm. Native GDS owns Leiden/WCC and canonical publication owns truth.
use super::EntityKind;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const RECIPE: &str = "entity-idf-bipartite-leiden-1";
const MAX_NODES: usize = 10_000;
const MAX_EDGES: usize = 50_000;

#[derive(Clone, Debug)]
pub struct Association {
    pub input_id: Uuid,
    pub source_version_id: Uuid,
    pub entity_id: Uuid,
    pub kind: EntityKind,
    pub boilerplate: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Vertex {
    Input { id: Uuid, source_version_id: Uuid },
    Term { id: Uuid },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeightedEdge {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projection {
    pub recipe: String,
    pub nodes: Vec<Vertex>,
    pub edges: Vec<WeightedEdge>,
    pub unassigned_versions: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparationFailure {
    DuplicateVersion,
    ForeignAssociation,
    Limit,
}

/// Call only with source versions already qualified under the exact producing
/// profile, context and current scope. Labels/text never enter the native graph.
pub fn prepare(
    inputs: &[(Uuid, Uuid)],
    associations: &[Association],
) -> Result<Projection, PreparationFailure> {
    if inputs.len() > MAX_NODES || associations.len() > MAX_EDGES * 2 {
        return Err(PreparationFailure::Limit);
    }
    let mut known = BTreeMap::new();
    let mut versions = BTreeSet::new();
    for &(input, version) in inputs {
        if known.insert(input, version).is_some() || !versions.insert(version) {
            return Err(PreparationFailure::DuplicateVersion);
        }
    }
    let mut terms = BTreeMap::<Uuid, BTreeSet<Uuid>>::new();
    for association in associations {
        if known.get(&association.input_id) != Some(&association.source_version_id) {
            return Err(PreparationFailure::ForeignAssociation);
        }
        if association.boilerplate
            || !matches!(
                association.kind,
                EntityKind::Technology | EntityKind::Concept
            )
        {
            continue;
        }
        terms
            .entry(association.entity_id)
            .or_default()
            .insert(association.input_id);
    }
    let assigned: BTreeSet<_> = terms.values().flat_map(|s| s.iter().copied()).collect();
    let edge_count: usize = terms.values().map(BTreeSet::len).sum();
    if assigned.len() + terms.len() > MAX_NODES || edge_count > MAX_EDGES {
        return Err(PreparationFailure::Limit);
    }
    let mut nodes = Vec::with_capacity(assigned.len() + terms.len());
    let mut input_indices = BTreeMap::new();
    for input in &assigned {
        input_indices.insert(*input, nodes.len());
        nodes.push(Vertex::Input {
            id: *input,
            source_version_id: known[input],
        });
    }
    let mut edges = Vec::with_capacity(edge_count);
    for (term, sources) in terms {
        let to = nodes.len();
        nodes.push(Vertex::Term { id: term });
        let weight = (1.0 + known.len() as f64 / sources.len() as f64).ln();
        for input in sources {
            edges.push(WeightedEdge {
                from: input_indices[&input],
                to,
                weight,
            });
        }
    }
    Ok(Projection {
        recipe: RECIPE.into(),
        nodes,
        edges,
        unassigned_versions: known
            .into_iter()
            .filter(|(id, _)| !assigned.contains(id))
            .map(|(_, version)| version)
            .collect(),
    })
}

/// Identity reconciliation only. A unique strict-majority Jaccard match may
/// retain a persisted ID; ties/weak overlap never equate unrelated communities.
/// A caller performs one-to-one assignment and records merge/split lineage.
pub fn match_topic(
    contributors: &BTreeSet<Uuid>,
    previous: &[(Uuid, BTreeSet<Uuid>)],
    already_used: &BTreeSet<Uuid>,
) -> Option<Uuid> {
    let mut best: Option<(Uuid, usize, usize)> = None;
    let mut ambiguous = false;
    for (id, old) in previous {
        if already_used.contains(id) {
            continue;
        }
        let intersection = contributors.intersection(old).count();
        let union = contributors.union(old).count();
        if union == 0 || intersection * 2 <= union {
            continue;
        }
        match best {
            None => {
                best = Some((*id, intersection, union));
                ambiguous = false;
            }
            Some((_, n, d)) => match (intersection * d).cmp(&(n * union)) {
                std::cmp::Ordering::Greater => {
                    best = Some((*id, intersection, union));
                    ambiguous = false;
                }
                std::cmp::Ordering::Equal => ambiguous = true,
                std::cmp::Ordering::Less => {}
            },
        }
    }
    if ambiguous {
        None
    } else {
        best.map(|(id, _, _)| id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }
    fn association(input: u128, term: u128) -> Association {
        Association {
            input_id: id(input),
            source_version_id: id(100 + input),
            entity_id: id(term),
            kind: EntityKind::Technology,
            boilerplate: false,
        }
    }
    #[test]
    fn affinity_is_linear_deduplicated_opaque_and_frequency_weighted() {
        let inputs = vec![(id(1), id(101)), (id(2), id(102)), (id(3), id(103))];
        let mut associations = vec![
            association(1, 10),
            association(1, 10),
            association(2, 10),
            association(1, 11),
        ];
        let mut environment = association(3, 12);
        environment.kind = EntityKind::Environment;
        associations.push(environment);
        let projection = prepare(&inputs, &associations).unwrap();
        assert_eq!(projection.nodes.len(), 4);
        assert_eq!(projection.edges.len(), 3);
        assert_eq!(projection.unassigned_versions, vec![id(103)]);
        assert!(projection.edges[2].weight > projection.edges[0].weight);
        let many: Vec<_> = (1..=5_000).map(|n| (id(n), id(100_000 + n))).collect();
        let shared: Vec<_> = many
            .iter()
            .map(|(input, version)| Association {
                input_id: *input,
                source_version_id: *version,
                entity_id: id(999_999),
                kind: EntityKind::Concept,
                boilerplate: false,
            })
            .collect();
        let linear = prepare(&many, &shared).unwrap();
        assert_eq!(linear.nodes.len(), 5_001);
        assert_eq!(linear.edges.len(), 5_000);
    }
    #[test]
    fn foreign_duplicates_and_budget_overflow_fail_instead_of_partial_success() {
        assert_eq!(
            prepare(&[(id(1), id(101)), (id(2), id(101))], &[]),
            Err(PreparationFailure::DuplicateVersion)
        );
        assert_eq!(
            prepare(&[(id(1), id(101))], &[association(2, 10)]),
            Err(PreparationFailure::ForeignAssociation)
        );
        let inputs: Vec<_> = (1..=6_000).map(|n| (id(n), id(100_000 + n))).collect();
        let terms: Vec<_> = inputs
            .iter()
            .map(|(input, version)| Association {
                input_id: *input,
                source_version_id: *version,
                entity_id: id(200_000 + input.as_u128()),
                kind: EntityKind::Concept,
                boilerplate: false,
            })
            .collect();
        assert_eq!(prepare(&inputs, &terms), Err(PreparationFailure::Limit));
    }
    #[test]
    fn topic_identity_survives_community_renumbering_but_not_ties_or_splits() {
        let members = BTreeSet::from([id(1), id(2), id(3)]);
        let old = vec![
            (id(20), members.clone()),
            (id(21), BTreeSet::from([id(3), id(4)])),
        ];
        assert_eq!(match_topic(&members, &old, &BTreeSet::new()), Some(id(20)));
        assert_eq!(
            match_topic(&BTreeSet::from([id(1)]), &old, &BTreeSet::new()),
            None
        );
        assert_eq!(
            match_topic(
                &members,
                &[(id(20), members.clone()), (id(22), members.clone())],
                &BTreeSet::new()
            ),
            None
        );
        assert_eq!(match_topic(&members, &old, &BTreeSet::from([id(20)])), None);
    }
}
