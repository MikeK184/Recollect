//! Request-local canonical gates for recall candidates, loaded per set. Each
//! `load` answers scope validity, claim eligibility (supports, review rules,
//! conflicts, support audit), retention deadlines and raw-evidence rule blocks
//! for a whole page of rows in a fixed number of statements; `item` then reads
//! the cache. Nothing outlives the caller's transaction.
use super::*;
use std::collections::HashMap;

pub(super) enum ClaimGate {
    Ready {
        view: Box<ClaimView>,
        dependency_deadline: Option<DateTime<Utc>>,
    },
    Failed(Error),
}

#[derive(Default)]
pub(super) struct Gates {
    scopes: HashMap<String, bool>,
    claims: HashMap<Uuid, ClaimGate>,
    raw: HashMap<(String, Uuid, Option<Uuid>), bool>,
    manifests: HashMap<Uuid, ManifestRevision>,
    pub(super) evidence: memory_evidence::EvidenceCache,
}

fn scope_key(row: &Candidate) -> String {
    serde_json::to_string(&row.selection.0).unwrap_or_default()
}
fn raw_key(row: &Candidate) -> (String, Uuid, Option<Uuid>) {
    (row.kind.clone(), row.id, row.chunk_id)
}

impl Gates {
    /// Forget per-row results (claims, evidence, raw blocks) between pages while
    /// keeping scope and manifest answers, which are shared by every page.
    pub fn release(&mut self) {
        self.claims.clear();
        self.raw.clear();
        self.evidence = Default::default();
    }
    pub fn scope(&self, row: &Candidate) -> bool {
        self.scopes.get(&scope_key(row)).copied().unwrap_or(false)
    }
    pub fn claim(&self, revision: Uuid) -> Option<&ClaimGate> {
        self.claims.get(&revision)
    }
    pub fn blocked(&self, row: &Candidate) -> bool {
        self.raw.get(&raw_key(row)).copied().unwrap_or(false)
    }
    pub async fn manifest(
        &mut self,
        tx: &mut Tx<'_>,
        brain: Uuid,
        id: Uuid,
    ) -> Result<&ManifestRevision> {
        match self.manifests.entry(id) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                Ok(entry.insert(memory::manifest(tx, brain, id).await?))
            }
        }
    }

    /// Load every gate the rows need and the cache lacks. Rows already cached
    /// cost nothing, so a page load followed by per-row `item` calls is exact.
    pub async fn load(
        &mut self,
        state: &AppState,
        tx: &mut Tx<'_>,
        context: &ReadContext<'_>,
        rows: &[&Candidate],
    ) -> Result<()> {
        let ReadContext {
            brain, input, at, ..
        } = *context;
        let mut need = BTreeMap::new();
        for row in rows {
            let key = scope_key(row);
            if !self.scopes.contains_key(&key) {
                need.entry(key).or_insert(&row.selection.0);
            }
        }
        if !need.is_empty() {
            let valid =
                workspace::selections_valid(tx, brain, &need.values().copied().collect::<Vec<_>>())
                    .await?;
            self.scopes.extend(need.into_keys().zip(valid));
        }
        // Rows that already fail scope or retention never reach the claim/raw gates.
        let now = Utc::now();
        let live: Vec<&Candidate> = rows
            .iter()
            .copied()
            .filter(|r| self.scope(r) && r.expires_at.is_none_or(|t| t > now))
            .collect();
        let mut revisions = Vec::new();
        let mut seen = BTreeSet::new();
        for row in live.iter().filter(|r| r.kind == "claim") {
            if !self.claims.contains_key(&row.revision_id) && seen.insert(row.revision_id) {
                revisions.push(decode::<ClaimRevision>(row.data.0.clone())?);
            }
        }
        if !revisions.is_empty() {
            let ids: Vec<Uuid> = revisions.iter().map(|r| r.id).collect();
            let views = memory::views(
                state,
                tx,
                &mut self.evidence,
                revisions,
                at,
                input.fact_at,
                true,
            )
            .await?;
            let deadlines: HashMap<Uuid, Option<DateTime<Utc>>> = sqlx::query_as::<
                _,
                (Uuid, Option<DateTime<Utc>>),
            >(
                "SELECT t.id,recollect_memory_deadline($1,t.id) FROM unnest($2::uuid[]) AS t(id)",
            )
            .bind(brain)
            .bind(&ids)
            .fetch_all(&mut **tx)
            .await?
            .into_iter()
            .collect();
            for (id, view) in ids.into_iter().zip(views) {
                let gate = match view {
                    Ok(view) => ClaimGate::Ready {
                        view: Box::new(view),
                        dependency_deadline: deadlines.get(&id).copied().flatten(),
                    },
                    Err(error) => ClaimGate::Failed(error),
                };
                self.claims.insert(id, gate);
            }
        }
        let raw: Vec<&Candidate> = live
            .into_iter()
            .filter(|r| r.kind != "claim" && !self.raw.contains_key(&raw_key(r)))
            .collect();
        if !raw.is_empty() {
            let blocked = raw_blocked(tx, brain, input, &raw).await?;
            for (row, blocked) in raw.iter().zip(blocked) {
                self.raw.insert(raw_key(row), blocked);
            }
        }
        Ok(())
    }
}
