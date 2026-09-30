# Atlas lifecycle proof

Status: shipped
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: run the agent-memory-atlas benchmarks page's own two specified tests —
  the §6 thirteen-step deletion sequence and the §7 contradiction matrix — as a
  deterministic, judge-free pass/fail matrix through the real product API, and
  reproduce the frozen HotpotQA public benchmark on the current build.
- Non-goals: answer-level LLM scoring, BEAM/LongMemEval/LoCoMo runs,
  propagated-copy scopes (steps 11–13), any universal memory ranking claim.
- Delivery shape: one opt-in Playwright API harness plus a dated evidence
  mapping; no product code change.

## Governing Sources

[Evaluation contract](../../../contracts/operations-integrated-evaluations.md)
(owned evaluation boundary and the Atlas lifecycle invariant clause),
[retention/erasure contract](../../../contracts/memory-retention-and-erasure.md),
[review contract](../../../contracts/memory-review-and-corrections.md),
[claims contract](../../../contracts/memory-claims-and-time.md),
[epic](../../epics/operational-readiness.md) and the
[benchmarks page](https://neoneye.github.io/agent-memory-atlas/benchmarks/).

## Scope

- In scope: §6 steps 1–10 with derived-store probes; §7 cases replacement,
  polarity flip, retraction, partial supersession, bounded validity and
  equal-weight contradiction scored on A/B/C/D/E plus adversarial re-entry;
  write-to-readable lag and recall latency observation; HotpotQA rerun both
  lanes.
- Out of scope: §6 steps 11–13 (no second-scope publication or paired device
  in the proof Brain), semantic/handover leak probes (model disabled), answer
  generation, dataset judge.
- Blockers: none.

## Surface and Interface Changes

- Interfaces: existing product APIs only — sources, claims, review
  (accept/correct/withdraw/reject), claim-conflicts resolve, recall (all four
  modes, `fact_at`/`knowledge_at`), erasures preview→erase→retry, graph
  rebuild/view, evidence catalogue, excerpts, brain audit, model usage.
- Storage: owned ephemeral test-ui PostgreSQL proof database and ignored
  per-run report directories under `.cache/atlas-lifecycle-` (one JSON report
  per run id); the HotpotQA rerun reused the existing public-benchmark report
  path.
- Ownership: evaluation harness under operational readiness; no retrieval,
  privacy or graph product changes; the harness's six initial red cells were
  harness-semantics fixes (tombstone reads, per-claim probes, disjoint
  validity, graph state vocabulary, catalogue tombstone) applied before any
  product claim, disclosed in the mapping.

## Data and Authority

- Inputs: fictional high-entropy canaries authored for this run
  ("Plumbus Vantablack-7", "Zircon-11", relay/port/beacon personas); frozen
  HotpotQA validation input unchanged (SHA-256 verified by the fetcher).
- Authority: the Atlas page §6/§7 specs define the steps and columns; metric
  vocabulary (absent-or-qualified = pass, unqualified stale assertion = fail,
  silent pick = fail) was fixed from the page before scoring and not adjusted
  after seeing results, except aligning probes with the product's documented
  tombstone semantics.
- Blind spots: delivery-level only (no answering model); single machine, debug
  binaries, one run; steps 11–13 untested paths declared N/A per the
  capability-declaration rule the page credits ForgetEval with.

## States and Edge Cases

- Loading: source versions polled to `ready`; graph generation polled to
  `ready`; erasure polled to `complete` with the retry endpoint driving
  privacy maintenance.
- Empty: a never-retrievable baseline is a harness setup failure, gated by
  step 2 before any deletion claim.
- Error: any FAIL cell fails the run and the matrix is still written to the
  report; the retained-database rule of the semantic benchmark applies.
- Blocked: model-policy-enabled brains abort the run (zero-call assertion).
- No-access: canary values live only in the disposable proof Brain; normal
  Brains and credentials are untouched, and the audit probe verifies the
  erasure event references target ids without retaining plaintext.
- Duplicate or replay: every mutation carries an Idempotency-Key; the erasure
  retry endpoint is the only replay path and converges on `complete`; a rerun
  uses a fresh disposable Brain rather than reusing proof state.
- Stale data: disposable Brain per run; no reuse of prior proof state.
- Reconciliation divergence: report includes erasure closure counts, graph
  node count, model usage total (asserted 0) and latency samples.

## Integrations and Runtime Inputs

- Providers: none. Zero model calls by construction; the harness asserts
  `models/usage.total == 0` before writing the report.
- Environment: opt-in `RECOLLECT_ATLAS_LIFECYCLE=1` with
  `RECOLLECT_TEST_MODEL_WORKER=1` through `scripts/test-ui.sh`; the HotpotQA
  rerun used the documented `RECOLLECT_PUBLIC_BENCHMARK=1
  RECOLLECT_PUBLIC_SEMANTIC=1` pair.
- Secrets: owner credentials come from the existing `.env` test inputs; no
  credential appears in reports or docs.
- Failure handling: infrastructure failures (lock-contention 503/504 with the
  live stack competing) are recorded in the mapping; retained databases are
  inspected for paid attempts (ledger count) before removal.

## Tests and Acceptance

- Automated: `web/tests/atlas-lifecycle.spec.ts` green with 0 FAIL cells
  across 16 rows/40 cells; `tests/public-benchmark.spec.ts` both lanes
  reproducing the committed 2026-09-28 accuracy digits; `./scripts/validate.sh`.
- Manual: review each cell against the Atlas page spec; confirm the mapping
  discloses harness iteration and untested paths.
- Acceptance: a committed pass/fail matrix (not a percentage), every N/A
  carrying a reason, zero model calls, no effect on normal Brains, and the
  reproduction delta reported honestly.

## Closeout

- Planned: deterministic §6/§7 matrix through the product API plus a fresh
  HotpotQA reproduction on the current build.
- Shipped: `web/tests/atlas-lifecycle.spec.ts` (16 rows, 40 cells, 0 FAIL,
  6 declared N/A) and the exact HotpotQA reproduction (5%/100% recall,
  46/0 empty, 97 requests). [Dated evidence](../../../mappings/atlas-lifecycle-proof-2026-09-29.md).
- Not shipped: §6 steps 11–13, semantic/handover leak probes, answer-level
  §7 A column, LongMemEval/BEAM/LoCoMo (governed by the
  [proposed protocol](../../../contracts/operations-longmemeval-protocol.md)).
- New blockers: none.
- Docs updated: evaluation contract Atlas-lifecycle clause, benchmark
  runbook section, dated mapping, epic slice, both execution indexes,
  contracts/mappings/epic indexes.
- Validation: lifecycle suite passed in 10.5 s with the worker drained and the
  owned database removed; both benchmark lanes passed; governance lint and
  `./scripts/validate.sh` run with this change.
- Version: N/A: no release policy.
- Commit: uncommitted.
