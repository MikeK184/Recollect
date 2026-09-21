# Exact combined repository views and links

Status: shipped
Owning epic: `docs/roadmap/epics/graph-intelligence.md`
Work type: product

## Summary

- Goal: Traverse validated dependencies across exact manifest-selected repositories.
- Non-goals: Generic name matching, external ref resolution, analytics or a canvas.
- Delivery shape: Local Rust/React/schema, actual integrations and documented proof.

## Governing Sources

[ADR 0007](../../../adr/0007-canonical-graph-projections.md),
[combined graph contract](../../../contracts/graph-cross-repository-views.md),
[projection contract](../../../contracts/graph-projection-and-traversal.md),
[publication](../../../contracts/evidence-repository-publication.md) and
[owning epic](../../epics/graph-intelligence.md).

## Scope

- In scope: Parsed committed module-source evidence, exact source/target linking,
  shared generation union, automatic recovery, canonical read/removal gates and UI.
- Shared gate repair: Apply exact manifest configuration paths to raw repository
  facts before recall ranking and graph admission; prove positive and excluded
  paths through both consumers.
- Out of scope: The contract's explicitly unsupported protocols and successor slices.
- Blockers: None; routine decisions are resolved in the governing contract.

## Surface and Interface Changes

- Interfaces: Combined graph selection/rebuild, exact input generations, link
  issues and generated browser types; publication preserves normalized witnesses.
- Storage: Migration 019 extends graph generation inputs and origin-binding epoch.
- Ownership: Companion reuses Tree-sitter/HCL; graph adapter joins exact evidence
  and reuses existing durable work, canonical recall and Neo4j path operations.

## Data and Authority

- Inputs: Permitted committed text locally, retained immutable facts/manifests
  centrally, registered origin identities and existing snapshot projections.
- Authority: Brain grants, fixed operation scope, exact manifests and canonical
  rejection/retention. A link is declared structure, never deployment proof.
- Blind spots: Legacy regex hints, dynamic/registry/ref resolution and unsupported
  extractors remain explicitly unresolved with source evidence when eligible.

## States and Edge Cases

- Loading: Queued/building exact combined inputs and visible durable progress.
- Empty: Qualified empty view/no bounded path distinct from incomplete inputs.
- Error: Parse/link limits, backend/mismatch, ambiguous sources and unsupported refs.
- Blocked: Missing exact snapshot/base projection; no latest-input fallback.
- No-access: Canonical Brain/operation checks before selection and publication.
- Duplicate or replay: Exact input reuse, frozen descriptors and existing receipts.
- Stale data: Origin epoch invalidates linking; manifests preserve exact old inputs.
- Reconciliation divergence: Existing graph generation/entity fences and privacy replay.

## Integrations and Runtime Inputs

- Providers: Repo-owned PostgreSQL and Neo4j; native Tree-sitter/HCL dependencies.
- Environment: Existing database/Neo4j variables; no new credentials or model use.
- Secrets: Existing pre-materialization exclusion and artifact checks apply.
- Failure handling: Existing graph lease/batches/timeout/retry limits plus bounded
  native parser cancellation. Reference checkouts and customer worktrees stay intact.

## Tests and Acceptance

- Automated: Native dirty-tree/parser proof; real combined paths, exact manifests,
  identity ambiguity, limits, removal/recovery/authorization and focused regressions.
- Manual: Desktop/mobile exact combined selection, processing, issues/path/evidence.
- Acceptance: All combined contract criteria and `./scripts/validate.sh` pass.

## Closeout

- Planned: Complete the governing combined graph contract.
- Shipped: Native committed module evidence, exact manifest/alias/ref linking,
  shared combined generations, current canonical qualification, durable recovery,
  correction/erasure, fair discovery, bounded native paths and browser workflows.
- Not shipped: Explicitly deferred protocols/ref resolution and the required
  analytics, graph exploration and retrieval fusion successor slices.
- New blockers: None.
- Docs updated: Combined contract, shared recall path-selection contract, interface
  and validation mapping, graph runbooks, owning epic and execution/indexes.
- Validation: 79 platform integrations passed in 221.27 seconds (one unrelated
  live OIDC test filtered); 13 ordinary workspace tests passed, including native
  committed publication in 2.52 seconds; Clippy passed with warnings denied;
  native combined and ordinary graph browser flows passed in 17.5 seconds;
  32 governance tests passed. Six combined integration scenarios cover exact
  identity/reuse, corrections/erasure, current retention, queue fairness, native
  authorization/retry and union limits, alongside the existing graph recovery
  regressions. Normal runtime migration 019 and restart preserved all seven
  Brains, SWEG's exact 99-node/183-edge graph, capture/revision history and model
  request counts. No new model calls or customer source reads. See the
  [dated evidence](../../../mappings/cross-repository-interfaces-2026-09-15.md).
- Version: N/A; no release policy.
- Commit: uncommitted.
