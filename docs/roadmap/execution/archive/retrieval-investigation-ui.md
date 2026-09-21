# Desktop investigation across memory and graphs

Status: shipped
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Inspect matched evidence, disagreements, time history and current graph
  relationships without losing the query's selection or canonical state.
- Non-goals: Generated answers, execution, saved searches and mobile views.
- Delivery shape: Desktop React workflow with additive API deadline metadata.

## Governing Sources

- [Investigation contract](../../../contracts/retrieval-investigation-ui.md)
- [Graph fusion](../../../contracts/retrieval-graph-fusion.md)
- [Canonical claims](../../../contracts/memory-claims-and-time.md)
- [Review and corrections](../../../contracts/memory-review-and-corrections.md)
- [Graph exploration](../../../contracts/graph-exploration.md)
- [Owning epic](../../epics/hybrid-retrieval.md)

## Scope

- In scope: Three result views, visible scope/time/trust/provenance, frozen claim
  inspection, exact evidence, canonical review links, current graph drill-through,
  expiry/error/epoch clearing and meaningful integrated desktop proof.
- Out of scope: New ranking, graph algorithms, provider operations, persistence,
  customer content transmission and host execution.
- Blockers: None; routine decisions are resolved and predecessors shipped.

## Surface and Interface Changes

- Interfaces: Existing recall panel and canonical inspection/graph handlers;
  nullable `expires_at` on RecallResponse and ClaimEvidenceDetail; generated API.
- Storage: No migration or new durable data.
- Ownership: Retrieval assembles context; memory owns history/review/evidence;
  graph owns qualified native exploration. Browser composes those interfaces.

## Data and Authority

- Inputs: One submitted query/response, exact evidence IDs, canonical conflict IDs,
  frozen time, returned graph/snapshot selection and existing catalogues.
- Authority: Existing server grants, scope, eligibility, review and retention.
- Blind spots: Bounded results are neither global consistency nor answer proof;
  source grouping and graph connectivity do not confer truth.

## States and Edge Cases

- Loading: Explicit search; read-only inspection has bounded loading and close.
- Empty: Distinct no match, insufficient support and no returned disagreement.
- Error: Safe errors hide old payload; explicit refresh where appropriate.
- Blocked: Graph history/unavailable center and missing support explain limits.
- No-access: Owner handlers deny access; observed failure clears the workspace.
- Duplicate or replay: Deduplicated source evidence and existing IDs; no paid replay.
- Stale data: Shared invalidation, all-channel epoch polling, deadlines and frozen
  historical inspection; late responses cannot restore superseded state.
- Reconciliation divergence: Existing graph/review handlers retain their checks;
  changed graph epoch requires a new explicit recall.

## Integrations and Runtime Inputs

- Providers: Existing React/Mantine/TanStack Query, PostgreSQL and Neo4j adapters.
- Environment: Existing local runtime and synthetic fixtures; no new dependency.
- Secrets: Existing ignored environment; no new model or secret transmission.
- Failure handling: Existing server bounds, cancellation signals, no automatic
  query retry; metadata polling does not launch graph computation.

## Tests and Acceptance

- Automated: Real API expiry fixtures, desktop investigation plus recall/review/
  graph regressions, clipboard/error/scope/epoch clearing and no extra model calls.
  Run appropriate Rust, generated API, browser build and governance checks.
- Manual: Inspect desktop screenshots and a retained normal SWEG investigation;
  compare existing Brain/model/job inventory before and after.
- Acceptance: Complete the contract cases with evidence in the
  [dated mapping](../../../mappings/retrieval-investigation-2026-09-22.md).

## Closeout

- Planned: Complete desktop inspection workflow and context freshness boundaries.
- Shipped: Three desktop result views, canonical disagreement comparison, grouped
  exact source inspection, frozen claim history, optional review, same-scope native
  graph/path navigation and complete deadline/error/epoch invalidation. Nullable
  API deadlines expose existing retention. Visual review also fixed sparse graph
  label spacing and automatic enlargement through the existing renderer.
- Not shipped: Mobile views, generated answers, saved searches and execution are
  explicit non-goals. Agent-facing tools follow in MCP coordination.
- New blockers: None.
- Docs updated: Contract, dated mapping, investigation runbook, epic/indexes,
  README and continuation record, with local delivery boundaries preserved.
- Validation: Seven integrated browser scenarios passed (53.2s), then four affected
  graph/investigation cases passed after visual adjustment (35.8s). Eleven platform
  retention scenarios passed (46.43s), 13 workspace tests, Clippy, generated 125
  operation API, web build/typecheck, formatting and governance/32 checker tests
  passed. Actual normal SWEG source and 74-node/151-edge graph inspection passed;
  seven Brains, 33 model requests, zero active jobs and migration 020 preserved.
- Version: N/A: no release policy.
- Commit: Uncommitted.
