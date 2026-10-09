# Memory quality and local update first batch

Status: in-progress
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

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
- Blockers: No unresolved authority decision; final normal-Brain graph performance acceptance remains open after authenticated endpoints timed out under background contention. Later research targets remain unimplemented.

## Surface and Interface Changes

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

## Tests and Acceptance

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
- Current outcome: initial migration-043 graph checks failed in 2–15 seconds,
  as retained in the [first-batch report](../../../mappings/memory-quality-first-batch-proof-2026-10-08.md).
  The normal image is now on migration 047, built at 13:57:36Z, with exact IDs
  preserved through checkpointed updates. The migration-046 series passed
  37/40 with three database failures and p95 3.520 seconds. The new 047 series
  passes only 35/40, with five database failures and p95 4.027 seconds. Reliable
  graph acceptance remains open; the [current mapping](../../../mappings/automatic-knowledge-mapping-progress-2026-10-09.md)
  retains every outcome and identifies cumulative digest/handover maintenance
  under the writer lock as the next measured seam. The native 10k capture-only
  workload passes 40/40 at p95 0.513 seconds; it is not normal-runtime acceptance.

## Closeout

- Planned: the bounded update, evidence and graph batch above.
- Shipped: Local implementation/deployment of the update, evidence-window, answer and graph/audit query repairs; the slice remains in-progress until live graph acceptance passes. No new comparable benchmark score is established.
- Not shipped: reliable populated normal-Brain graph reads; later learning/time/topic/ANN research phases; a complete fresh held-out benchmark.
- New blockers: Measured background writer contention and repeated canonical qualification still exceed graph read deadlines. PID/advisory-key sampling now ties long digest/handover maintenance to the tested Brain; next action is reducing cumulative writer tenure through bounded preparation/publication while preserving complete support/privacy, exact generation and deadline gates. Full automatic mapping and forced audit/rebuild acceptance remain open.
- Docs updated: accepted first-batch amendments, installation runbook, dated proof, research follow-up and owning epic/index/pack.
- Validation: Focused native answer/evidence/support/graph/MCP controls, browser geometry, root update and isolated recovery passed; final governance result is in the dated proof. Final model-free normal graph checks failed and are retained. Paid diagnostics were incomplete and stopped near the USD 8 commitment ceiling.
- Version: N/A — local development.
- Commit: uncommitted.
