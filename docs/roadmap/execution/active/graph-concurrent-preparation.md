# Concurrent graph preparation and checked publication

Status: in-progress
Owning epic: `docs/roadmap/epics/graph-intelligence.md`
Work type: product

## Summary

- Goal: Populated graph discovery/traversal and background generation preparation
  avoid long Brain lock tenure while preserving current canonical publication.
- Non-goals: Change evidence meaning, replace Neo4j/GDS, relax privacy or claim
  acceptance, or add paid model requests.
- Delivery shape: Rust/SQL implementation, native concurrency/graph tests and
  dated proof; normal runtime update after checkpoint and complete verification.

## Governing Sources

[ADR 0007](../../../adr/0007-canonical-graph-projections.md),
[concurrent preparation](../../../contracts/graph-concurrent-preparation.md),
[exploration](../../../contracts/graph-exploration.md),
[durable work](../../../contracts/platform-durable-work.md),
[epic](../../epics/graph-intelligence.md).

## Scope

- In scope: Read-only preparation transaction, fair final publication, epoch
  invalidation coverage, interactive graph and generation worker integration;
  exact-identity canonical deadline/window query plans with unchanged RLS.
- Out of scope: Automatic entity/topic schema and optional extractors (next
  dedicated slice); generalized answer/model admission changes.
- Blockers: None for this contract-backed implementation. Existing normal-runtime
  timeout remains an acceptance failure until measured repair passes.

## Surface and Interface Changes

- Interfaces: Existing graph APIs retain wire formats; stale preparation has
  static `graph_preparation_changed` failure; session is checked at final commit.
- Storage: Migration 044 extends preparation admission and invalidation triggers;
  descriptor metadata records preparation epoch, accepting old descriptors for
  reads and rebuilding them before worker reuse. Migration 046 preserves exact
  canonical support/deadline meaning with parameterized dependency metadata
  lookups; graph-window identity qualification retains complete histories.
- Ownership: `db` owns transaction setup; graph read/worker own preparation/final
  validation; PostgreSQL remains canonical, Neo4j separately verified.

## Data and Authority

- Inputs: Current canonical source/revision/scope/retention gates, immutable
  descriptors, exact native generations and lease state.
- Authority: Final fresh authenticated transaction with fair Brain lock,
  epoch/scope/deadline and lease checks. Snapshots are never acceptance.
- Blind spots: Native snapshot/I/O and populated 10k capture controls pass;
  audit/rebuild concurrency and normal-runtime graph proof remain open.
  No general performance claim follows.

## States and Edge Cases

- Loading: Existing bounded progress and queued/running statuses.
- Empty: Eligible empty views retain honest generation coverage.
- Error: Changed input drops buffered output; damaged native state fails closed.
- Blocked: Capacity/backend outage retains existing static errors/backoff.
- No-access: Revoked role/device/session cannot publish prepared content.
- Duplicate or replay: Generation staging/import stays lease-fenced/idempotent.
- Stale data: Epoch/deadline changes reject output and stale saved descriptors.
- Reconciliation divergence: Canonical selection and native verification must
  agree before response/readiness.

## Integrations and Runtime Inputs

- Providers: PostgreSQL 17 and existing Neo4j/GDS; no model provider calls.
- Environment: Existing database/Neo4j variables; no new credential configuration.
- Secrets: Never output tokens or source payloads in profiling.
- Failure handling: Existing request/statements, leases, bounded retry/backoff.

## Tests and Acceptance

- Automated: Actual snapshot/write concurrency, preparation write denial, final
  invalidation and access controls; native knowledge/exploration/path tests;
  `./scripts/validate.sh`, formatting and focused compile checks.
- Manual: Populated model-free normal-installation graph requests while ordinary
  worker activity continues; checkpoint and exact build/schema proof if updated.
- Acceptance: All contract controls pass, no stale publication, errors included
  in declared-concurrency timing report, normal graph timeout check resolves.

## Closeout

- Planned: Two-phase graph preparation, checked publication and concurrency proof.
- Shipped: N/A — implementation in progress.
- Not shipped: Populated audit/rebuild concurrency and normal-runtime timeout repair proof;
  complete stale-input/revocation acceptance and final closeout.
- New blockers: None beyond open acceptance; trace other background lock holders
  if this change does not resolve the populated-runtime timeout.
- Docs updated: Contract, ADR amendment, epic/pack/index, approved plan status and
  [dated progress](../../../mappings/automatic-knowledge-mapping-progress-2026-10-09.md).
- Validation: PostgreSQL preparation/write/bypass proof and paused actual-Neo4j
  scope/session checks passed with zero model requests. Native graph suite had
  35/38 pass initially; the current complete native suite passes 40/40, including
  the stronger collection-membership control. The 10k fixture passes 40/40 reads
  during 32 captures at p95 0.513 seconds with zero model requests. Formatting,
  focused server compile and documentation validation passed. This is local
  proof; audit/rebuild and normal-runtime acceptance remain open.
  The subsequent complete suite passes 41/41, including old/new app-role
  deadline/support equivalence and foreign-Brain isolation. Five focused support
  regression controls pass. The new capture run passes 40/40 at p95 0.519 seconds.
  Normal migration-045 reads still fail in 2–15 seconds. Rolled-back normal-data
  probes return the identical revised deadline in 4–4.5 ms. The normal update to
  migration 046 completed with checkpoint and exact data-ID preservation. Two
  runtime probe rounds include one failed view, three successful overview reads
  at 2.35–4.79 seconds and two centered reads at 0.27–0.74 seconds. Background
  admission waits/ten-second discovery work remain, so runtime acceptance is
  still open. The dated mapping records every outcome and the next repair seam.
  The later semantic-discovery query repair is deployed at the same migration
  with build time 13:07:42Z and exact data-ID preservation. Initial normal reads
  pass; the longer series passes 37/40 with three retained database failures and
  p95 3.520 seconds. Support-audit discovery still times out; this pack remains
  in progress, including the forced audit/rebuild workload gate.
- Version: N/A — local development.
- Commit: uncommitted.
