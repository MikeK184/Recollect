# Automatic entities, reversible aliases and overlapping topics

Status: in-progress
Owning epic: `docs/roadmap/epics/graph-intelligence.md`
Work type: product

## Summary

- Goal: Automatically organize eligible Brain evidence into typed entities,
  reversible aliases and overlapping topics with useful bounded navigation.
- Non-goals: Replace the Rust core, change truth/permission meaning, overwrite
  manual organization or add a compulsory foreign-language service.
- Delivery shape: Rust candidate/persistence/worker/protocol/API implementation,
  incremental privacy-safe reconciliation, UI and measured end-to-end proof.

## Governing Sources

[ADR 0020](../../../adr/0020-automatic-knowledge-organization.md),
[organization contract](../../../contracts/automatic-knowledge-organization.md),
[canonical graphs](../../../adr/0007-canonical-graph-projections.md),
[concurrent preparation](../../../contracts/graph-concurrent-preparation.md),
[epic](../../epics/graph-intelligence.md).

## Scope

- In scope: All five product improvements in the dedicated plan, with extractor
  fixtures, entity/alias storage, topics/overrides, automatic updates, current
  qualified navigation, privacy/restore closure and acceptance reporting.
- Out of scope: Unproved mandatory Graphify/Graphiti adapters, paid cloud
  summaries, new benchmark campaigns, commits and pushes.
- Blockers: Populated and normal-runtime graph acceptance gates expansion
  delivery; pure candidate/fixture and canonical implementation work is independent.

## Surface and Interface Changes

- Interfaces: Bounded optional interpretation candidates, typed semantic
  organization APIs and version-checked topic/alias overrides; no fake claims.
- Storage: Canonical persisted entities, mentions, supported alias/relation
  records, topics, automatic memberships, overrides and durable mapping inputs.
- Ownership: Graph intelligence owns organization; evidence owns source/manual
  groups; memory lifecycle owns corrections, erasure and restore dependency replay.

## Data and Authority

- Inputs: Exact eligible immutable source/repository/claim fields, existing
  interpretation/vectors and canonical scope; schema/adapter revisions retained.
- Authority: Rust candidate validation and final authenticated mutation with
  exact current inputs, epoch, fences, deadline and lease. Models propose only.
- Blind spots: GLiNER raw development quality failed; a successful parser/fake
  fixture does not prove actual extractor recall, grouping usefulness or UI benefit.

## States and Edge Cases

- Loading: Queued/running mapping and partial coverage are visible.
- Empty: Honest unassigned/unsupported/no-eligible-input state.
- Error: Malformed/foreign/duplicate/unbounded candidate batch rejects atomically.
- Blocked: Missing compatible extractor/profile or unavailable backend is explicit.
- No-access: Current Brain/task scope and revocation apply to every hydrated name.
- Duplicate or replay: Idempotent mapping input publication and stable identities.
- Stale data: Correction, scope/model change, expiry and erasure invalidate all
  affected derivatives; stale jobs cannot restore withdrawn content.
- Reconciliation divergence: Rebuild reproduces the current permitted view and
  human overrides; native graph presence alone does not qualify a semantic node.

## Integrations and Runtime Inputs

- Providers: Existing standing Brain model gateway and optional compatible local
  candidate adapter; PostgreSQL and native GDS. Fixtures make zero paid calls.
- Environment: Existing owned test/runtime services; no global Python/MCP changes.
- Secrets: No key/input payload printed in performance receipts or checked-in data.
- Failure handling: Bounded input/output, short checked publication, leases and
  visible blocked/failed coverage; no confidence-only scope or truth acceptance.

## Tests and Acceptance

- Automated: Native Unicode/exact span/input/endpoint gates; actual persistence,
  scope/alias and override/idempotency controls; correction/erasure/restore;
  frozen extraction/grouping fixtures; populated concurrent reads and UI flows.
- Manual: Blind source-backed label review and timed cross-session navigation;
  normal-installation graph reads and independent candidate-model quality proof.
- Acceptance: The full organization contract passes, manual groups stay intact,
  all requested read outcomes are counted and no derivative content resurrects.

## Closeout

- Planned: Full Rust automatic source-backed organization and useful graph entry.
- Shipped: N/A — full product slice remains in progress. Rust candidate parsing,
  source-bound persisted staging and direct-source erasure/restore controls are
  locally implemented and tested; no automatic semantic publication is selected.
- Not shipped: Automatic pipeline publication, qualified aliases/topics, automatic invalidation,
  qualified UI and full model/performance/privacy acceptance remain required.
- New blockers: None preventing independent implementation; graph/model quality
  failures remain open acceptance rather than a reason to narrow the objective.
- Docs updated: Accepted ADR/contract and owning slice/pack/index before code.
- Validation: Three Rust candidate test groups, two native storage/restore tests,
  two populated deletion/catalog tests and 40 native graph tests pass; the 10k
  capture workload passes 40/40 reads at p95 0.513 seconds with zero model calls.
  These do not establish extraction quality, full topic/privacy acceptance or
  normal-installation performance. See the dated progress mapping.
- Version: N/A — uncommitted implementation.
- Commit: uncommitted.
