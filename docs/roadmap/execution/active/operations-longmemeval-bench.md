# LongMemEval answer-level benchmark run

Status: blocked
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: execute the official LongMemEval-S answer-level benchmark through the
  governed Ask path and the pinned upstream judge, once the
  [proposed protocol contract](../../../contracts/operations-longmemeval-protocol.md)
  is accepted with an explicit cost approval.
- Non-goals: BEAM, LoCoMo, the `_m` haystack, vendor head-to-head tables.
- Delivery shape: frozen fetcher, opt-in harness, committed report with all
  three accuracies and the per-type breakdown.

## Governing Sources

[Integrated evaluation contract](../../../contracts/operations-integrated-evaluations.md),
[answer contract](../../../contracts/retrieval-answers.md),
[model policy contract](../../../contracts/memory-provider-policy-and-learning.md),
[epic](../../epics/operational-readiness.md), and the
[2026-09-29 lifecycle evidence](../../../mappings/atlas-lifecycle-proof-2026-09-29.md)
that motivates the answer-level complement. The LongMemEval protocol contract
linked in the Summary is `proposed` and is not a governing source; it becomes
one only at acceptance with the user's explicit cost approval, which is this
pack's named blocker.

## Scope

- In scope: dataset/evaluator freezing with SHA-256 pins, canonical haystack
  ingestion into a dedicated bench Brain, frozen Ask answering budget, pinned
  `gpt-4o-2024-08-06` judging with a bounded disagreement audit, the
  three-accuracy report.
- Out of scope: everything the protocol defers; any product behavior change
  discovered mid-run (recorded, not tuned away).
- Blockers: **explicit user cost approval.** The protocol's drafting was
  authorized on 2026-09-29 with "no spend"; dispatch requires the written
  token/cost estimate approved in explicit terms. The protocol contract is
  `proposed` and becomes governing authority only when accepted at that
  approval.

## Surface and Interface Changes

- Interfaces: new opt-in fetcher script and Playwright spec mirroring the
  public-benchmark pattern; no product API change.
- Storage: owned ephemeral proof database; frozen inputs and report under
  ignored `.cache/`; vendored evaluator content-addressed in the repo.
- Ownership: evaluation harness; answer-path behavior stays governed by the
  existing answer/model contracts.

## Data and Authority

- Inputs: `longmemeval_s_cleaned.json` (hash-pinned at fetch) from the pinned
  upstream commit; questions/gold/labels never enter memory.
- Authority: the upstream evaluator defines scoring; this repo adds no local
  abstention or grading rule.
- Blind spots: pipeline-level score (six stages, memory layer is one);
  single-run variance unless the user approves repeated runs.

## States and Edge Cases

- Loading: full haystack ingest drained before any question runs.
- Empty: unanswered questions count as misses; no discarding.
- Error: uncertain paid attempts stop the run; database retained for
  reconciliation per the semantic-benchmark precedent.
- Duplicate or replay: question and request identities persist before
  dispatch, so a resumed run cannot double-answer or double-judge a question;
  re-ingesting an already-imported haystack session is refused by the run's
  owned-database boundary rather than silently deduplicated.
- Blocked: this pack — cost approval outstanding.
- No-access: questions, gold answers and evidence-session labels never enter
  memory, so no recall path can surface them; the bench Brain is owned by the
  proof installation and no normal credential reaches it.
- Stale data: dataset drift refused by the fetcher; report names file, hash
  and commit or is invalid.
- Reconciliation divergence: usage ledger vs report tokens reconciled before
  cleanup.

## Integrations and Runtime Inputs

- Providers: OpenAI embeddings + answer generation + judge, isolated daily
  token ceiling and concurrency set in the protocol approval; gateway remains
  the sole dispatcher.
- Environment: opt-in flags on the existing test-ui lifecycle.
- Secrets: `.env` provider key only; no key material in reports.
- Failure handling: no blind retry; failed runs preserve owned state.

## Tests and Acceptance

- Automated: fetcher hash gate; ingest drain; every question answered and
  judged exactly once; report regenerates from the raw answer file with the
  vendored evaluator.
- Manual: review the three accuracies, per-type table, cost sheet, and
  declared N/A fields before publishing the mapping.
- Acceptance: a dated mapping committing the full report with dataset hash,
  commit, prompt variant, judge model, usage and per-type breakdown, with
  normal Brains untouched and the cost within the approved estimate.

## Closeout

- Planned: — (awaiting the blocker's resolution).
- Shipped: none.
- Not shipped: the entire run.
- New blockers: explicit user cost approval; protocol contract acceptance.
- Docs updated: this pack, epic slice, indexes.
- Validation: N/A until dispatched.
- Version: N/A.
- Commit: uncommitted.
