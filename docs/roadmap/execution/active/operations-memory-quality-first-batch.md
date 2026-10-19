# Memory quality and local update first batch

Status: in-progress
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product
Implementation: first-batch checkpoint retained; October 18 graph follow-up is locally validated, with normal update and comparable quality rerun open

## Summary

- Goal: preserve the user's data across a dependable current-build update, deliver complete attributed source fragments, and open a bounded useful graph for a large Brain.
- Non-goals: all research phases, ANN, automatic topic/entity inference, unmeasured learning superiority or automatic paid retries.
- Delivery shape: local code and focused proof; normal update only after checkpoint and acceptance. No commit/push.

## Governing Sources

- [Installation](../../../contracts/operations-local-and-shared.md)
- [Recovery](../../../contracts/operations-recovery-drills.md)
- [Recall](../../../contracts/retrieval-exact-and-lexical.md)
- [Answers](../../../contracts/retrieval-answers.md)
- [Exploration](../../../contracts/graph-exploration.md)
- [Support verification and bounded audit discovery](../../../contracts/memory-source-support-verification.md)

## Scope

- In scope: root update/recovery bridge, build/schema diagnostics; whole canonical source chunks and bounded alternative-window packing; historical/preference answer behavior; separately frozen provider diagnostics with retained failures; opt-in partial graph entry and one-hop qualification/verification; focused local and budgeted evaluation.
- Out of scope: changing existing Brain provider policies, remote deployment, recursive summaries, schema reset, deletion of migration history and full-corpus learning.
- Blockers: No unresolved authority decision. The historical normal-installation graph failure remains unresolved there; later local mixed-workload and normal-data clone repairs pass. A reviewed installed update, final normal-runtime acceptance and comparable memory-quality rerun remain open. Later research targets are separate scope.

## Decisions

- Interfaces: `stack.sh update`; root recovery registration backed by existing encrypted recovery; secret-free build information; optional `windowed` graph requests with compatible false defaults.
- Storage: migrations 042–043 optimize existing audit-target and support predicates without changing eligibility or stored memory; retain all existing migrations. Later graph/candidate/deadline migrations 044–046 have their own governing slices. Migration 047 adds only bounded audit scan progress and deletion inventory below. Private ignored recovery descriptors, recipients/key references and encrypted checkpoints use current recovery format.
- Ownership: server canonical read gates, native Neo4j one-hop discovery, existing recovery modules and React consumers.
- Runtime follow-up: materialize only requested graph entities before support checks and source-chunk expansion, while keeping latest-at-time selection over the full authorized input; retain all canonical and final exact-revision fences. Detect overlapping settled node/label bounds and use the renderer's built-in grid fallback, without custom force algorithms.
- Cutover follow-up: compute the existing legacy-audit target closure once per scheduling query instead of rescanning current roots for every candidate. Preserve the exact/current contributor closure and existing four-audit admission bound, Brain policy, budgets and retry rules; no additional paid benchmark arm or policy change.
- Audit execution: the same canonical function first checks an exact current root, then computes the full shared root closure for historical contributors. Migration 042 replaces only that predicate; scoped historical/cycle negatives remain required.
- Supported-memory follow-up: use an explicit false-only early exit when neither authentic owner review nor current-policy positive assessment exists; positive revisions retain every existing evidence/rule/scope/retention/dependency/digest guard. Materialize this necessary direct-authority filter for graph rebuilds, qualify roots once under the existing Brain lock, and reuse their exact IDs across support/count/contribution reads in that same transaction. Keep epoch invalidation and all publication locks intact.
- Dependency query: expand only edges attached to the recursive frontier, retaining both immutable exact contributors and their current logical heads, Brain scoping and recursive deduplication. Differential fixtures must compare the complete returned sets against migration 037, including historical/current identities and cycles; no dependency may be omitted for speed.
- Remaining scheduler contention: the initial current-first query rewrite is superseded by migration 047's fair bounded missing-revision walk below. Historical contributor audits retain the same gates and every missing identity is revisited; scheduling remains best-effort with unchanged local/provider retry caps and no policy/budget increase.

## Data and Authority

Audit query-plan repair materializes current/historical target identities lacking
an assessment for the exact policy/verifier before canonical metadata lookup.
Resolve each revision/current-head metadata by parameterized primary-key lookup;
preserve the full exact/current target closure, complete exact prerequisites,
cycle checks, reviewed authority, invoker RLS and all existing retry states.
Apply capacity only after qualification. The initial pure SQL rewrite preserves
eligibility but still times out on the normal 914-row missing cohort. Its native
differential proof is a control, not a sufficient performance repair.

The follow-up uses migration 047 for Brain/policy/verifier UUID keyset progress,
with RLS, monotonic compare-and-set version and canonical Brain deletion inventory.
Rust prepares at most 16 identities outside the writer lock (two-second pass,
one-second per statement); it publishes at most four freshly qualified identities
under a new writer lock (one-second pass, 750 ms per statement). Timeouts roll
back to a savepoint, advance scheduling progress and remain unresolved for the
next walk. Empty tail wraps. Each candidate retains its whole target/dependency
closure. Fresh writer/standing-policy and cursor checks fence publication; full
requalification handles correction, erasure, expiry and other concurrent changes.
Audit progress commits separately from unrelated learning/digest maintenance.
No model policy, paid allowance, retry authority or supported-memory rule changes.

- Inputs: current local checkout, explicit owned root Compose installation, original canonical chunks and projection identities.
- Authority: original source/scope/epoch/deadlines and source-support/correction gates; partial topics/graphs never prove truth or absence.
- Blind spots: fifty-case scores are small-corpus proof; installed external hosts and new learning models remain separate.

## States and Edge Cases

- Loading: bounded graph window and current update phase.
- Empty: no eligible evidence or selected window, with explicit coverage.
- Error: migration, backup, projection or provider errors remain visible and separately accounted.
- Blocked: failed checkpoint prevents update; failed migration leaves app stopped.
- No-access: preserve canonical actor/Brain/scope gates before and after reads.
- Duplicate or replay: no repeated paid attempt; stable source identities with distinct exact chunk citations.
- Stale data: clear graph/evidence on epoch/revocation/expiry; build stamp is informational, not a cryptographic release gate.
- Reconciliation divergence: no automatic rollback against a migrated live database; restore only into a fresh compatible named installation with current erasure journal.

## Integrations and Runtime Inputs

- Providers: retain cheap GLM/Qwen for isolated diagnostic answers; normal Brain policies unchanged.
- Environment: build revision/time variables only; existing provider and installation variables retained.
- Secrets: reuse private recovery configuration and encryption; no copying secrets to code/logs/output.
- Failure handling: preserve deadlines, unknown bills and no automatic paid retry/fallback.

## Acceptance

The audit repair compares old/new current and historical identity sets through
the actual app role, including bottom-up lineage, cycle/rejection/current-head
and recorded-policy/verifier controls. Normal-data probes remain read-only;
normal graph measurements count every outcome under ordinary worker activity.
The bounded follow-up additionally proves an unavailable prefix larger than the
scan budget, wrap/restart and competing prepared cursors, same-Brain writes during
preparation, correction/review/policy/access/expiry changes before publication,
bottom-up historical prerequisites, cycles, all recorded attempt states, and
canonical Brain deletion. Normal candidate profiling remains read-only before
any checkpointed update; graph success is measured with every outcome counted.

- Automated: meaningful canonical span/multi-window publication controls; >500-node partial graph and damaged/excluded inputs; root recovery ownership/encryption and failed-update checks; focused browser, native tests and governance validation.
- Manual: verify build/schema and preserved normal inventory after an explicitly owned update, using retained checkpoint.
- Acceptance: complete >2048-byte supported evidence survives to answer publication; additional source fragments remain bounded and revalidated; partial graph entry avoids full-source qualification/verification and opens a nonempty default overview plus centered read on the normal Brain during ordinary background verification; original complete API retains refusal semantics; update checkpoint is compatible with existing fresh-target recovery; score changes reported rather than assumed.
- Historical installation outcome (October 9): initial migration-043 graph checks failed in 2–15 seconds,
  as retained in the [first-batch report](../../../mappings/memory-quality-first-batch-proof-2026-10-08.md).
  The recorded normal image was on migration 047, built at 13:57:36Z, with exact IDs
  preserved through checkpointed updates. The migration-046 series passed
  37/40 with three database failures and p95 3.520 seconds. The new 047 series
  passes only 35/40, with five database failures and p95 4.027 seconds. Reliable
  graph acceptance remains open; the [current mapping](../../../mappings/automatic-knowledge-mapping-progress-2026-10-09.md)
  retains every outcome and identifies cumulative digest/handover maintenance
  under the writer lock as the next measured seam. The native 10k capture-only
  workload passes 40/40 at p95 0.513 seconds; it is not normal-runtime acceptance.

- Current local follow-up (October 18): the owning
  [concurrency pack](graph-concurrent-preparation.md) records 602/602 mixed 10k
  reads, zero errors and p95 0.460 seconds during capture/audit/rebuild, plus both
  normal-data clone series passing 40/40 below two seconds. These are later
  passing repair measurements, with the old failures preserved. They did not
  update the normal installation. A comparable post-repair memory-quality run
  has not been completed; the latest LongMemEval baseline remains 20/50 (40%).
  [Automatic organization](graph-automatic-knowledge-mapping.md) has its own
  failed current4 extractor/group/canonical quality gates.

## Closeout

- Planned: the bounded update, evidence and graph batch above.
- Shipped: Partial first-batch implementation and the dated October 9 local update of evidence-window, answer and graph/audit repairs. The full slice remains in-progress. October 18 graph repairs have passing local proof, with no new normal-installation deployment or comparable benchmark score.
- Not shipped: Verified repaired behavior in the normal installation, final integrated closeout and a comparable post-repair memory-quality run. Later learning/time/topic/ANN work remains separately scoped.
- New blockers: The remaining readiness gates are an owned normal-installation update and all-outcome graph proof under ordinary background work, followed by the comparable quality run. Local writer-tenure repair and forced capture/audit/rebuild proof now pass in the concurrency pack; those completed controls are not reopened as unimplemented work. Separate automatic-mapping quality failures remain with its owning pack.
- Docs updated: accepted first-batch amendments, installation runbook, dated proof, research follow-up and owning epic/index/pack.
- Validation: Focused native answer/evidence/support/graph/MCP controls, browser geometry, root update and isolated recovery passed in the original batch. Its model-free normal graph failures and incomplete paid diagnostics near the USD 8 commitment ceiling remain retained. The later October 18 local graph measurements above pass; they are separate from installed-runtime and answer-quality proof. This documentation update makes no new provider calls or runtime measurements.
