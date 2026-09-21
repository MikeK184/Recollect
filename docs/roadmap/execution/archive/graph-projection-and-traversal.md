# Canonical graph projection and traversal

Status: shipped
Owning epic: `docs/roadmap/epics/graph-intelligence.md`
Work type: product

## Summary

- Goal: Rebuildable structural and knowledge graphs with qualified bounded paths.
- Non-goals: Cross-repository linking, analytics, full graph canvas and recall fusion.
- Delivery shape: Local Rust/React/schema/integration proof and documentation.

## Governing Sources

[ADR 0007](../../../adr/0007-canonical-graph-projections.md),
[graph contract](../../../contracts/graph-projection-and-traversal.md),
[owning epic](../../epics/graph-intelligence.md),
[runtime posture](../../../adr/0003-product-runtime.md).

## Scope

- In scope: Typed canonical descriptors, staged Neo4j publication, autonomous
  discovery, current eligibility before path selection, recovery/erasure and UI.
  Reuse the established durable heartbeat across graph/semantic/learning/handover
  workers and verify the existing delayed-publication proof after extraction.
  Reuse the visible-Brain membership policy for repository fact and source-chunk
  SELECTs so large graph input scans do not repeat the role lookup for every row; verify
  direct RLS isolation and the existing publication/recall regressions.
- Out of scope: The graph contract's explicit successor capabilities.
- Blockers: None; all dependent routine decisions are resolved in the contract.

## Surface and Interface Changes

- Interfaces: Graph status/rebuild/view/path endpoints and generated browser types.
- Storage: Migration 018 for generation descriptors/state, discovery and graph
  cleanup accounting; Neo4j owned identity/membership/relationship schema.
- Ownership: Rust graph adapter consumes publication, memory and shared read gates.

## Data and Authority

- Inputs: Retained snapshot fact records and explicit claim supports/contributions.
- Authority: PostgreSQL RLS, immutable task scope, canonical lifecycle and manifests.
- Blind spots: Extraction gaps, unresolved/duplicate targets and unsupported links.

## States and Edge Cases

- Loading: Queued/importing state and job progress.
- Empty: Empty eligible view or explicit bounded no-path response.
- Error: Typed timeout/backend/validation failures; no partial success publication.
- Blocked: Missing input/projection or unavailable current authority.
- No-access: Brain and paired operation gates on reads, mutations and worker output.
- Duplicate or replay: Idempotent import and mutation receipts; no mixed generations.
- Stale data: Previous generation requalified; current missing inputs shown partial.
- Reconciliation divergence: Canonical privacy replay and graph fences precede reads.

## Integrations and Runtime Inputs

- Providers: Repository-owned actual Neo4j Community/GDS and PostgreSQL services.
- Environment: NEO4J_URL, NEO4J_USERNAME, NEO4J_PASSWORD; existing database variables.
- Secrets: Environment only; no evidence text or credential fields in Neo4j.
- Failure handling: Bounded batches/HTTP/transactions, heavy-lane leases/retry and
  cleanup retained until actual completion; contract defines the precise limits.

## Tests and Acceptance

- Automated: Real Neo4j/PG stage/recovery/scope/removal/alternative-path proofs;
  input limits, malformed/error responses, existing regressions and governance.
- Manual: Browser desktop/mobile readiness, exact input, path and evidence states.
- Acceptance: Every graph contract criterion proved with retained test evidence;
  `./scripts/validate.sh` and affected Rust/web checks pass before closeout.

## Closeout

- Planned: Complete graph contract and all listed acceptance scenarios.
- Shipped: Canonical repository/knowledge descriptors, staged Neo4j import and
  physical validation, autonomous discovery, qualified bounded native traversal,
  replay/lease/erasure recovery and desktop/mobile graph evidence UI. Shared worker
  heartbeat and canonical candidate-query regressions were repaired and verified.
- Not shipped: Cross-repository links (17), analytics (18), full graph canvas (19)
  and retrieval fusion (20), as explicitly assigned to successor slices.
- New blockers: None.
- Docs updated: ADR, contract, owning epic, execution/indexes, interface/reuse and
  validation mappings, graph runbook and local-development navigation.
- Validation: [Executed proof](../../../mappings/graph-validation-2026-09-15.md):
  73 platform tests, ten ordinary workspace tests, Clippy, desktop/mobile graph UI
  and 32 governance checks passed. Normal runtime through migration 018 proved
  exact SWEG snapshot/manifest selection, 99 entities/183 relationships, path UI,
  all seven Brains and recall preservation with zero new model calls or customer
  file reads. Local delivery on 2026-09-15; no external deployment.
- Version: N/A; no release policy.
- Commit: uncommitted.
