# Concurrent graph preparation and checked publication

Status: in-progress
Owning epic: `docs/roadmap/epics/graph-intelligence.md`
Work type: product
Implementation: resumed 2026-10-18 by the user's complete graph implementation request

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
  Includes bounded off-lock digest/handover discovery and complete contributor
  preparation, independent short publication transactions, restart-safe fair
  cursors and model-free concurrency/recovery proof under the October 18 contract.
- Out of scope: Automatic entity/topic schema and optional extractors (next
  dedicated slice); generalized answer/model admission changes.
- Blockers: No unresolved implementation decision. Local mixed-workload and
  normal-data clone performance gates pass; normal-installation update and
  final integrated acceptance remain outstanding.

## Decisions

- Interfaces: Existing graph APIs retain wire formats; stale preparation has
  static `graph_preparation_changed` failure; session is checked at final commit.
- Storage: Migration 044 extends preparation admission and invalidation triggers;
  descriptor metadata records preparation epoch, accepting old descriptors for
  reads and rebuilding them before worker reuse. Migration 046 preserves exact
  canonical support/deadline meaning with parameterized dependency metadata
  lookups; graph-window identity qualification retains complete histories.
- Ownership: `db` owns transaction setup; graph read/worker own preparation/final
  validation; PostgreSQL remains canonical, Neo4j separately verified.
- Native verification (2026-10-19, [contract amendment](../../../contracts/graph-concurrent-preparation.md#native-verification-amendment--2026-10-19)):
  the static 20-second cache of verified native items is removed. Each read sends
  one seal request (stored token, closed import, stored counts) and falls back to
  the exact selective check on any difference; traversals also verify the
  relationships they return. Migration 069 adds `graph_generations.native_verified_at`
  (backfilled from `published_at`, set at readiness), a `recollect_graph_suspect`
  function that lets a failed read clear that stamp, and a trigger-function
  refinement so a stamp-only update does not advance the analytics epoch. Graph
  maintenance deep-verifies one ready generation per Brain per pass after 24 hours
  and marks damage `graph_import_mismatch`; existing discovery then queues one
  replacement under the 500-job cap. The adapter also creates a
  `RECOLLECT_GRAPH_EDGE(generation)` index and a `(generation, id)` index: PROFILE
  on the local Neo4j showed the counts, cleanup and exact verification otherwise
  scan every relationship of every Brain.

## Data and Authority

- Inputs: Current canonical source/revision/scope/retention gates, immutable
  descriptors, exact native generations and lease state.
- Authority: Final fresh authenticated transaction with fair Brain lock,
  epoch/scope/deadline and lease checks. Snapshots are never acceptance.
- Blind spots: Revised populated 10k capture/audit/rebuild controls and both
  normal-data clone gates pass. The original installation runtime has not been
  updated; local proof does not establish deployed health.

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

## Acceptance

- Automated: Actual snapshot/write concurrency, preparation write denial, final
  invalidation and access controls; native knowledge/exploration/path tests;
  `./scripts/validate.sh`, formatting and focused compile checks. Native
  verification: seal comparison unit tests; two actual Neo4j cases (one request
  per warm read, immediate damage failure, unselected erasure through the
  fallback; stamping, raced-token refusal, one replacement per damage) in
  `graph_seal.rs`.
- Manual: Populated model-free normal-installation graph requests while ordinary
  worker activity continues; checkpoint and exact build/schema proof if updated.
- Acceptance: All contract controls pass, no stale publication, errors included
  in declared-concurrency timing report, normal graph timeout check resolves.

## Closeout

- Planned: Two-phase graph preparation, checked publication and populated concurrency proof.
- Shipped: N/A for the complete slice. Checkpoint f2673d7 contains earlier partial preparation; the October 18 local implementation adds bounded off-lock maintenance, short checked publication and fair restart-safe discovery. Full slice acceptance remains open.
- Not shipped: A reviewed normal-runtime update and final integrated closeout. Local mixed 10k and both normal-data clone gates pass; these results do not establish updated installed behavior.
- New blockers: None for independent local implementation; normal-runtime state remains unchanged.
- Docs updated: Accepted graph contract/ADR amendment, owning epic/pack/index, [historical progress](../../../mappings/automatic-knowledge-mapping-progress-2026-10-09.md) and [current local evidence receipts](../../../mappings/graph-organization-implementation-2026-10-18.md).
- Validation: October 18 migration-063 mixed 10k proof passes 602/602 reads with zero errors and p95 0.459626 seconds during captures, audits, native import and two ready rebuilds. Both normal-data clone series pass 40/40 with the same hydrated payload, at p95 1.629 and 0.951 seconds. Four focused maintenance cases pass. The October 9 normal-installation failure (35/40, p95 4.027 seconds), earlier capture-only 40/40 proof and prior fixture failures remain retained; they are not the latest local repair result. Detailed workload versions and limits follow below.

October 18 local evidence: all four focused autonomous maintenance cases pass.
The final migration-061 sustained mixed 10k workload completed 600/600 requests
with zero errors at all-outcome p95 0.450670 seconds, 16 actual audit completions,
64 durable cursor steps and 63 actual native import windows. Thirty-four import
windows overlap 61 read-window events. Both requested rebuilds reached ready
(97.836 and 138.781 seconds), and reads continued through 139.209 seconds.
The workload used 34 controlled loopback calls and zero paid calls. Eight focused
generation/read/deferral controls passed in the full series; the remaining
delayed-I/O fixture passed after fixing its missing proxy response counter.
Earlier non-overlapping and one-error workload receipts are retained. These
are disposable-database/native proofs, separate from the October 9
normal-runtime observation. Current normal-data template and logical-restore
clones each pass 40/40 reads with identical 60-node hydrated payloads at p95
1.629 and 0.951 seconds respectively; only the destination of the logical restore
is analyzed. Original data/schema and normal API/worker state stay unchanged.
The compact [evidence receipts](../../../mappings/graph-organization-implementation-2026-10-18.md)
record the frozen inputs, cost approvals and failures; remaining gates are in
the [roadmap](../../epics/index.md#graph-delivery-status).

The later migration-063 regression passes 602/602 reads, zero errors and p95
0.459626 seconds during eight captures, sixteen forced audit completions and two
completed rebuilds. Of 63 native import windows, 32 overlap 55 read-window events.
Both generations reach ready at 100.420 and 141.747 seconds, with reads through
142.157 seconds. The 187.27-second test makes 34 controlled loopback requests and
zero paid calls. The exact fixture database, four native Brain/fence rows and
private paths are verified absent in its separate cleanup receipt. This fixture
has no edited human groups; the subsequent transitive-split bound and migration
extension have separate focused proof in the ledger. Neither run updates the
normal installation or replaces the retained October 9 failure observation.
