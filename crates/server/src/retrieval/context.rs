//! Request-local source lineage and complete attributed context admission.
//! These groups are coverage hints, never independent votes or truth evidence.
use super::*;
use semantic::Ranked;

type Identity = (String, Uuid);
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Root {
    Source(Uuid),
    Capture(Uuid, String, Option<String>),
    Repository(Uuid, Uuid, String),
    Fact(Uuid),
    Manifest(Uuid),
}
#[derive(sqlx::FromRow)]
struct Version {
    id: Uuid,
    source_id: Uuid,
    parent_version_id: Option<Uuid>,
    parent_valid: bool,
    binding_id: Option<Uuid>,
    capture_active: bool,
    metadata: Option<SqlJson<Value>>,
}
const VERSION_ROWS: usize = 8192;
const PARENT_STEPS: usize = 8;

fn source_root(versions: &BTreeMap<Uuid, Version>, mut id: Uuid) -> Option<Root> {
    let mut visited = BTreeSet::new();
    for depth in 0..=PARENT_STEPS {
        if !visited.insert(id) {
            return None;
        }
        let row = versions.get(&id)?;
        if let Some(parent) = row.parent_version_id {
            if !row.parent_valid || depth == PARENT_STEPS {
                return None;
            }
            id = parent;
        } else if row.capture_active {
            let metadata = &row.metadata.as_ref()?.0;
            let session = metadata.get("host_session_id")?.as_str()?;
            if !capture_identity(session) {
                return None;
            }
            let agent = match metadata.get("agent_id") {
                Some(Value::String(s)) if capture_identity(s) => Some(s.clone()),
                None | Some(Value::Null) => None,
                _ => return None,
            };
            return Some(Root::Capture(row.binding_id?, session.into(), agent));
        } else {
            // Removed parent bytes and expired capture metadata are never read.
            // An independently retained excerpt can use this opaque identity.
            return Some(Root::Source(row.source_id));
        }
    }
    None
}

async fn groups(
    tx: &mut Tx<'_>,
    brain: Uuid,
    ranked: &[Ranked],
    coverage: &mut RecallCoverage,
) -> Result<BTreeMap<Identity, Option<BTreeSet<Root>>>> {
    let mut pending: BTreeSet<Uuid> = ranked
        .iter()
        .flat_map(|r| &r.item.provenance)
        .filter(|p| p.kind == "source_version")
        .map(|p| p.id)
        .collect();
    if pending.len() > 6000 {
        pending = pending.into_iter().take(6000).collect();
        note(coverage, "source_lineage_bounded");
    }
    let mut versions = BTreeMap::new();
    for _ in 0..=PARENT_STEPS {
        pending.retain(|id| !versions.contains_key(id));
        if pending.is_empty() || versions.len() == VERSION_ROWS {
            break;
        }
        let requested: Vec<_> = pending
            .into_iter()
            .take(VERSION_ROWS - versions.len())
            .collect();
        // Every joined identity is unique. Keep each lookup parameterized even
        // before import statistics catch up; a flattened outer-join plan can
        // otherwise scan all parent versions once for every requested source.
        let rows: Vec<Version> = sqlx::query_as("SELECT v.id,v.source_id,e.parent_version_id,
            (e.version_id IS NULL OR p.id IS NOT NULL) AS parent_valid,b.id AS binding_id,
            coalesce(c.state='accepted' AND v.privacy_state='active' AND
              (recollect_retention_deadline(c.brain_id,c.retention_class,c.captured_at)>clock_timestamp()),false) AS capture_active,
            CASE WHEN c.state='accepted' AND v.privacy_state='active' AND
              recollect_retention_deadline(c.brain_id,c.retention_class,c.captured_at)>clock_timestamp()
              THEN c.metadata ELSE NULL END AS metadata
            FROM source_versions v
            LEFT JOIN LATERAL (SELECT e.* FROM source_excerpts e
              WHERE e.brain_id=$1 AND e.version_id=v.id LIMIT 1) e ON true
            LEFT JOIN LATERAL (SELECT p.id FROM source_versions p
              WHERE p.brain_id=$1 AND p.id=e.parent_version_id AND p.source_id=e.parent_source_id LIMIT 1) p ON true
            LEFT JOIN LATERAL (SELECT c.* FROM capture_events c
              WHERE c.brain_id=$1 AND c.source_version_id=v.id AND c.source_id=v.source_id LIMIT 1) c ON true
            LEFT JOIN LATERAL (SELECT b.id FROM capture_bindings b
              WHERE b.brain_id=$1 AND b.id=c.binding_id LIMIT 1) b ON true
            WHERE v.brain_id=$1 AND v.id=ANY($2) ORDER BY v.id")
            .bind(brain).bind(requested).fetch_all(&mut **tx).await?;
        pending = rows
            .iter()
            .filter(|r| r.parent_valid)
            .filter_map(|r| r.parent_version_id)
            .collect();
        versions.extend(rows.into_iter().map(|v| (v.id, v)));
    }
    let mut result = BTreeMap::new();
    for entry in ranked {
        let roots: Option<BTreeSet<_>> = entry
            .item
            .provenance
            .iter()
            .map(|p| match p.kind.as_str() {
                "source_version" => source_root(&versions, p.id),
                "repository_fact" => match (p.repository_id, p.snapshot_id, p.path.as_ref()) {
                    (Some(repo), Some(snapshot), Some(path)) if !path.is_empty() => {
                        Some(Root::Repository(repo, snapshot, path.clone()))
                    }
                    _ => Some(Root::Fact(p.id)),
                },
                "manifest_revision" => Some(Root::Manifest(p.id)),
                _ => None,
            })
            .collect();
        let roots = roots.filter(|r| !r.is_empty());
        if roots.is_none() {
            note(coverage, "source_lineage_unknown");
        }
        result.insert((entry.item.kind.clone(), entry.item.id), roots);
    }
    Ok(result)
}

pub(super) struct Packed {
    pub context: RecallContext,
    pub deadlines: BTreeMap<Identity, Option<DateTime<Utc>>>,
    pub selection: RecallContextSelection,
    groups: BTreeMap<Identity, Option<BTreeSet<Root>>>,
}
impl Packed {
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.context
            .items
            .iter()
            .filter_map(|item| {
                self.deadlines
                    .get(&(item.kind.clone(), item.id))
                    .copied()
                    .flatten()
            })
            .min()
    }

    pub fn expire(&mut self, now: DateTime<Utc>, coverage: &mut RecallCoverage) {
        let before = self.context.items.len();
        self.context.items.retain(|item| {
            self.deadlines
                .get(&(item.kind.clone(), item.id))
                .is_none_or(|deadline| deadline.is_none_or(|at| at > now))
        });
        if self.context.items.len() < before {
            coverage.withheld += before - self.context.items.len();
            note(coverage, "content_expired_during_recall");
        }
        let mut roots = BTreeSet::new();
        self.selection.unknown_lineage_items = 0;
        for item in &self.context.items {
            match self
                .groups
                .get(&(item.kind.clone(), item.id))
                .and_then(Option::as_ref)
            {
                Some(group) => roots.extend(group.iter().cloned()),
                None => self.selection.unknown_lineage_items += 1,
            }
        }
        self.selection.distinct_source_groups = roots.len();
    }
}

pub(super) async fn pack(
    tx: &mut Tx<'_>,
    brain: Uuid,
    input: &RecallRequest,
    ranked: &[Ranked],
    coverage: &mut RecallCoverage,
) -> Result<Packed> {
    let groups = groups(tx, brain, ranked, coverage).await?;
    let mut packed = Packed {
        context: RecallContext {
            instruction: INSTRUCTION.into(),
            mode: input.mode.clone(),
            items: vec![],
        },
        deadlines: BTreeMap::new(),
        selection: RecallContextSelection {
            source_diversity: input.source_diversity,
            ..Default::default()
        },
        groups: BTreeMap::new(),
    };
    let mut selected = BTreeMap::new();
    let mut covered = BTreeSet::new();
    let mut refused = BTreeSet::new();
    let mut admitted = BTreeSet::new();
    // Three small bounded passes: exact priority, optional coverage reservation,
    // and global-rank depth fill. Complete serialized attribution must fit.
    let has_aux = ranked.iter().any(|r| r.item.delivery_section != "query");
    let mut aux_selected = 0usize;
    for pass in 0..4 {
        if pass == 1 && !input.source_diversity && !has_aux {
            continue;
        }
        for (index, entry) in ranked.iter().enumerate() {
            if selected.len() >= input.limit
                || (pass == 1 && selected.len() >= input.limit.div_ceil(2))
            {
                break;
            }
            let aux = entry.item.delivery_section != "query";
            if (pass == 2) != aux {
                continue;
            }
            if aux && aux_selected >= input.limit / 2 {
                continue;
            }
            if selected.contains_key(&index)
                || refused.contains(&index)
                || (pass == 0 && !entry.exact)
            {
                continue;
            }
            let key = (entry.item.kind.clone(), entry.item.id);
            if admitted.contains(&key) {
                continue;
            }
            let roots = groups.get(&key).and_then(Option::as_ref);
            if pass == 1
                && !has_aux
                && roots.is_none_or(|rs| rs.iter().all(|r| covered.contains(r)))
            {
                continue;
            }
            let candidates = std::iter::once((&entry.item, entry.deadline)).chain(
                entry
                    .alternatives
                    .iter()
                    .filter(|_| entry.semantic_rank.is_none())
                    .map(|(item, deadline)| (item, *deadline)),
            );
            for (position, (candidate, deadline)) in candidates.enumerate() {
                let mut candidate = candidate.clone();
                if position > 0 {
                    candidate
                        .qualifications
                        .push("alternate_fragment_for_context_budget".into());
                    candidate.graph_match = entry.item.graph_match.clone();
                    if candidate.graph_match.is_some() {
                        candidate
                            .qualifications
                            .push("graph_proximity_not_truth".into());
                        candidate.score = entry.item.score;
                    }
                }
                packed.context.items.push(candidate.clone());
                if serde_json::to_vec(&packed.context)
                    .expect("typed recall context")
                    .len()
                    > input.context_bytes
                {
                    packed.context.items.pop();
                    note(coverage, "context_budget");
                } else {
                    selected.insert(index, candidate);
                    admitted.insert(key.clone());
                    if aux {
                        aux_selected += 1;
                    }
                    packed.deadlines.insert(
                        key.clone(),
                        [entry.deadline, deadline].into_iter().flatten().min(),
                    );
                    if let Some(roots) = roots {
                        covered.extend(roots.iter().cloned());
                    }
                    break;
                }
            }
            if !selected.contains_key(&index) {
                refused.insert(index);
            }
        }
    }
    if selected.len() == input.limit && selected.len() < ranked.len() {
        note(coverage, "result_limit");
    }
    packed.context.items.clear();
    packed.context.items = selected.into_values().collect();
    // Reserve diversity and primary ranks first. Remaining capacity may retain
    // another exact, nonoverlapping chunk from an already admitted source.
    // Each chunk remains a separate citation and consumes the caller's limits.
    for entry in ranked
        .iter()
        .filter(|entry| entry.item.kind == "source_version")
    {
        let key = (entry.item.kind.clone(), entry.item.id);
        if !admitted.contains(&key) {
            continue;
        }
        for (candidate, deadline) in std::iter::once((&entry.item, entry.deadline)).chain(
            entry
                .alternatives
                .iter()
                .map(|(item, deadline)| (item, *deadline)),
        ) {
            let same_source: Vec<_> = packed
                .context
                .items
                .iter()
                .filter(|item| item.kind == candidate.kind && item.id == candidate.id)
                .collect();
            if packed.context.items.len() >= input.limit || same_source.len() >= 3 {
                break;
            }
            if same_source
                .iter()
                .any(|item| overlapping_windows(item, candidate))
            {
                continue;
            }
            let mut candidate = candidate.clone();
            candidate
                .qualifications
                .push("additional_source_window".into());
            packed.context.items.push(candidate);
            if serde_json::to_vec(&packed.context)
                .expect("typed recall context")
                .len()
                > input.context_bytes
            {
                packed.context.items.pop();
                note(coverage, "context_budget");
            } else {
                let current = packed.deadlines.get(&key).copied().flatten();
                packed
                    .deadlines
                    .insert(key.clone(), [current, deadline].into_iter().flatten().min());
            }
        }
    }
    packed.groups = groups;
    Ok(packed)
}

fn overlapping_windows(a: &RecallItem, b: &RecallItem) -> bool {
    let span = |item: &RecallItem| {
        item.provenance
            .iter()
            .find(|p| p.kind == "source_version" && p.id == item.revision_id)
            .and_then(|p| Some((p.byte_from?, p.byte_to?)))
    };
    match (span(a), span(b)) {
        (Some((af, at)), Some((bf, bt))) => a.revision_id == b.revision_id && af < bt && bf < at,
        _ => true,
    }
}
