# Bounded canonical graph analytics

Status: accepted

## Source

[ADR 0007](../adr/0007-canonical-graph-projections.md), the
[graph foundation](../foundation/techstack.md#graph-computation-and-recovery),
[graph epic](../roadmap/epics/graph-intelligence.md),
[canonical traversal](graph-projection-and-traversal.md),
[combined inputs](graph-cross-repository-views.md),
[durable work](platform-durable-work.md) and
[retention/erasure](memory-retention-and-erasure.md). The active full-product
goal authorizes these routine interface and implementation decisions. The
[installed interface evidence](../mappings/graph-analytics-interfaces-2026-09-15.md)
records Cognee/Atlas reuse, successful probes and their limits.

## Contract

### Algorithms and meaning

Reuse native Neo4j GDS Community algorithms through the existing Rust Query API
adapter. Do not implement graph algorithms in Rust or introduce Python, a new
graph engine or paid GDS features. The first operations are PageRank centrality,
Leiden communities and weakly connected components (WCC).

Requests name an exact graph selection and a non-empty supported relation list.
Repository/combined analysis accepts extracted structural relations and the
validated `terraform_module` link; knowledge accepts only `supported_by` and
`contributed_to`. These calculate topology or recorded provenance, respectively;
scores and communities do not establish truth, causality or operational success.
Mixed structural/provenance families are invalid. No inferred/similarity edges
are added. PageRank accepts outgoing, incoming or both directions; Leiden and
WCC require both. Incoming reverses every selected edge before projection. Both
uses GDS's undirected projection, which doubles recorded relationships. Preserve
parallel relationships and self loops; each recorded relationship has unit weight.
Preserve selected isolated vertices with a null target.

Use concurrency one. Record the actual GDS version and the complete effective
configuration in each report: PageRank damping 0.85, maxIterations 50, tolerance
0.000001 and scaler None; Leiden randomSeed 42, maxLevels 10, gamma 1, theta 0.01,
tolerance 0.0001; WCC's native defaults apart from explicit concurrency/jobId.
These are named fixed first-version recipes, not a user-supplied Cypher/config
escape hatch. Stream output does not prove PageRank convergence. Community and
component IDs are report-local groups, never canonical entity identity.

### Exact inputs and result validity

Share canonical graph selection/qualification with traversal, including all
intermediate contributors, current trust/corrections, source fragment integrity,
collection/area, configured path and secret gates, exact manifests and retention.
Increase only the analytics admission limits to 10,000 canonical candidates and
50,000 selected relationships; retain the 64 MiB aggregate descriptor bound.
Overflow refuses the analysis without publishing a truncated prefix. Freeze the
resolved selection, exact ready generation IDs, complete canonical entity keys,
edge IDs, memory epoch, analytics epoch, coverage and earliest input deadline.
Combined reports retain every required manifest input, including hidden inputs.
Partial extraction/knowledge coverage is explicit and cannot be called complete.

A dedicated monotonically increasing Brain analytics epoch tracks canonical
input, correction, scope membership, retention policy and ready generation
changes, including materialization that leaves the memory epoch unchanged.
Use statement-level invalidation for bulk canonical inserts. The first version
conservatively invalidates all reports in that Brain on these changes; unrelated
changes may therefore require recomputation. It does not alter learning policy
or the memory epoch. Job progress and analytics' own writes do not advance it.
Every worker publication and report read also checks the complete qualified
input set, selected generation identities, current authority and clock deadlines.
The final report read is serialized against observed invalidation through commit,
including invalidation that leaves the Brain epoch unchanged. Buffered scores
cannot outlive a concurrent stale-state transition.
Freeze an explicitly supplied fact time; absent fact time keeps the established
unfiltered applicability semantics. Include the earliest active Brain source,
claim or snapshot retention deadline after qualification starts, even for a
withheld contributor: expiry of a conflicting peer can admit another node.
Already-expired unswept rows do not permanently block new reports.

Changing one contributor invalidates the whole aggregate, including scores for
surviving vertices and inputs absent from the displayed page. Invalidation
discards numeric rows and cancels old work; never filter rows from an old score
set and present the remainder as current. A fresh analysis receives a new report
identity. Stale/removed reports show status and bounded provenance, with no old
scores. Historical analytics over removed or superseded inputs is not supported.
Reports never promote claims or trigger a model call.

### Queue, resources and recovery

Writers queue analyses under the shared mutation/audit/idempotency boundary.
Readers can view retained eligible reports. Browser requests use current Brain
authority. Paired requests additionally bind a current retrieval operation with
the same immutable scope, owned by that account/device. A saved report's original
operation is provenance, not authorization for another reader. Recheck actor,
device, scope, lease and input eligibility when running and publishing.

Use the existing single heavy lane, 20-second fenced leases, shared heartbeat
and bounded retries. Persist report ownership and frozen inputs before execution.
Each attempt has a fresh UUID and a 120-second deadline, including qualification,
estimation, projection and computation. Bound individual computation queries to
90 seconds; ordinary catalog/control calls retain short Query API bounds. Require
both projection and algorithm native estimates; reject a combined estimate over
64 MiB. Account for undirected relationship duplication. Native GDS memory
admission remains enabled; an estimate cannot guarantee absence of OOM.

Persist an attempt's content-free ownership in a private durable installation
journal before any GDS creation, while holding the canonical Brain lock. Reuse
the existing durable journal writer. Ownership records contain installation,
Brain, report/job/lease/attempt UUIDs, current privacy sequence and timestamps,
never source text or secrets.
The journal is separate from privacy entry files and must accompany recovery.
Each attempt uses its own GDS graph name and transaction metadata/jobId.
Before allocating another projection, reconcile orphaned attempts and refuse
admission while cleanup is uncertain. Do not evict an unrelated catalog graph.

Projection creation holds the existing Neo4j per-Brain guard, checks an exact
attempt fence and server clock deadline, and uses native conditional Cypher to
skip the GDS function entirely when closed. Cleanup takes that same guard to
fence creation, terminates only transactions carrying the attempt's exact owned
metadata, confirms they stopped, drops the exact catalog projection and verifies
absence. It remains retryable through outages and crashes. Cancellation is
requested immediately but physical completion is eventual. Stream algorithms
into validated staged PostgreSQL numeric rows; never use GDS write-back modes.
Do not publish success until cleanup and final canonical/lease gates pass.

Run cleanup automatically during maintenance and startup/reconciliation. Reuse
the retained ownership journal after an older PostgreSQL restore, including
attempts absent from that database. Keep closed-attempt ownership until its
creation deadline has passed and absence is confirmed, so delayed requests or
an independent restore cannot recreate untracked scratch state. Canonical
erasure/expiry scrubs affected report payloads transactionally and waits for
scratch cleanup before privacy completion. Arming records the latest privacy
sequence under the Brain lock; an entry cleans only attempts recording an older
sequence, preserving later valid work without relying on timestamp ordering.

Retain at most 100 reports and 20 pending analyses per Brain, within the existing
500 pending jobs limit. Automatically evict oldest terminal reports at capacity
and terminal reports older than seven days; never evict active work. Report
pages contain at most 100 numeric results hydrated from canonical evidence;
metadata lists contain at most 20 reports. All result shapes, IDs, numeric values
and node counts are validated before storage; backend bodies never enter errors.

### API and UI

`POST /api/brains/{brain}/graph/analytics` queues a recipe, direction and selection.
`GET /graph/analytics` lists browser-visible metadata/jobs, with pagination.
`POST /graph/analytics/{report}/view` supplies the current reader operation and
page offset, returning current hydrated results or explicit stale/removed state.
Existing cancel controls apply. Retry of a terminal analysis queues a new report
from current explicitly selected inputs, not a mutation of the original result.

The graph UI exposes recipe, meaningful relation selection, direction, limits,
queue/progress, failure/cancellation/cleanup, input coverage, algorithm version
and parameters. Show exact inputs and evidence for ranked/grouped results.
Loading/empty/no-access/oversized/stale/unavailable states are distinct. Refresh
never queues work. Existing recall/capture remains usable while analysis runs.
Interactive graph canvas/layout and richer impact exploration belong to slice 19.

## Acceptance

Use actual local PostgreSQL and installed Neo4j/GDS to prove analytical fixtures
(independent finite-iteration PageRank check, disconnected groups and isolates),
direction and duplicate-edge meaning, an excluded hub with a positive control,
whole-result invalidation/recomputation after correction and hidden-contributor
erasure, materialization/generation changes, same-epoch clock expiry, paired
scope/revocation, bounded large input/admission, queued cancellation/lost lease,
real GDS termination, crash/retry and delayed fenced creation/restore cleanup.
Prove report staging and no successful prefix on error. Verify browser queue,
results/provenance, stale/error and mobile states. Run focused proof and
`./scripts/validate.sh`, preserve normal Brains/recall and model counters through
the local migration/restart, then reconcile all lifecycle records.

## Explicit Deferrals

Custom algorithm configuration, weighted/inferred graphs, automatic speculative
analysis of every Brain, historical aggregate access, Enterprise GDS, external
deployment, graph canvas/impact exploration (19) and graph retrieval fusion (20).
Automatic memory maintenance and optional review overrides remain unchanged.
