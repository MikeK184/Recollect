# Desktop chrome declutter: conditional assurance band and knowledge tab removal

Status: shipped
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: Stop rendering the standing assurance band on a healthy Brain (it appears only while something needs attention), and remove the horizontal Memory/Sources/Graph/Repositories tab strip from the four Knowledge pages because the contextual sidebar already owns that navigation.
- Non-goals: No change to what the band measures, its data feeds, the Activity drill-down, or any page's content; intra-page tabs on Agents, Activity, Connections and Settings stay.
- Delivery shape: Two bounded React changes plus one contract amendment and live browser verification.

## Governing Sources

- [Contextual desktop experience](../../../contracts/desktop-experience.md) (amended: conditional assurance band, sidebar as sole Knowledge navigation)
- [Tiered knowledge surface](../../../contracts/desktop-knowledge-surface.md)
- [Owning epic](../../epics/operational-readiness.md)

## Scope

- In scope: `AssuranceBand` renders only when attention is required — a blocker, pending read, failed feed, archived Brain, disabled autonomous memory, or an empty first session — and returns nothing on the healthy "Nothing needs you" state. The four Knowledge pages (Memory, Sources, Graph, Repositories) drop their `FeatureTabs` strip and render their own feature directly; routes, URL state, inspector behavior and cross-links are unchanged.
- Out of scope: The band's feeds, queries or thresholds; Activity page tabs; Agents/Connections/Settings intra-page tabs; any backend change.
- Blockers: None. The 2026-10-02 user direction (no band when nothing is required; no duplicate navigation) is the governing decision record; contract amendments are part of this change.

## Surface and Interface Changes

- Interfaces: None — presentation only over existing reads.
- Storage: None.
- Ownership: Operational readiness owns desktop chrome behavior; each Knowledge page keeps owning its content.

## Data and Authority

- Inputs: The existing assurance feeds (blockers, pending, failed, archived, enabled, latest run, captured total).
- Authority: Unchanged; hiding the band grants nothing and changes no permission.
- Blind spots: A healthy Brain shows no band at all, so its latest learning-run figures are visible only in Activity; that is the intended trade-off recorded in the contract.

## States and Edge Cases

- Loading: While feeds are pending the band still renders ("Checking recent activity"); it disappears once everything resolves healthy.
- Empty: An empty first session keeps its "Ready for your first session" band; a populated healthy Brain shows none.
- Error: Failed feeds keep their "Some activity is unavailable" band.
- Blocked: Any blocker keeps the attention band with its drill-down.
- No-access: Standard Brain access rules apply before any band logic.
- Duplicate or replay: Pure presentation; no writes.
- Stale data: Band state derives from the same polling feeds as before.
- Reconciliation divergence: None — no new data path.

## Integrations and Runtime Inputs

- Providers: None.
- Environment: None.
- Secrets: None.
- Failure handling: Unchanged feed error behavior.

## Tests and Acceptance

- Automated: Web typecheck and design checks; no backend tests required (no API change).
- Manual: Owner login at the live stack — a healthy Brain (SWEG) shows no assurance band on any page; Memory/Sources/Graph/Repositories pages render without the horizontal tab strip while the sidebar navigates between them; Agents, Activity, Connections and Settings keep their intra-page tabs.
- Acceptance: Browser verification passes on real data; typecheck and design checks clean; `./scripts/validate.sh` passes; live stack rebuilt and healthy.

## Closeout

- Planned: Conditional assurance band, knowledge tab-strip removal, contract amendment, browser proof.
- Shipped: `AssuranceBand` returns nothing on a healthy Brain — it renders only for a blocker, pending read, failed feed, archived Brain, disabled autonomous memory, or an empty first session; the four Knowledge pages no longer render the horizontal Memory/Sources/Graph/Repositories switcher (the contextual sidebar is the sole navigation), while their intra-page sub-tabs and the inspector's "Show in…" links are untouched. Live proof 2026-10-03: the healthy SWEG Brain shows no band on any page; Memory renders without the switcher and its All memory/Claims/Decisions/Procedures/Handovers tabs remain; sidebar navigation to Sources/Graph works; no console errors. See [evidence](../../../mappings/agents-ask-chrome-refinements-2026-10-03.md).
- Not shipped: Nothing from this scope.
- New blockers: None.
- Docs updated: Contract `desktop-experience.md` (conditional assurance band, sidebar as sole Knowledge navigation); epic slice and indexes reconciled; evidence mapping added.
- Validation: Web typecheck and design checks clean; no backend tests required (no API change); `./scripts/validate.sh` passes; live stack rebuilt via `./scripts/stack.sh up --build` with api/worker healthy.
- Version: N/A: no release policy or version bump in this scope.
- Commit: Uncommitted; no commit, push or release performed.
