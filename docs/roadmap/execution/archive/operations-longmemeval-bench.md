# LongMemEval answer-level benchmark run

Status: shipped
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: execute the official LongMemEval-S answer-level benchmark through the
  governed Ask path and the pinned upstream judge under the
  [accepted protocol contract](../../../contracts/operations-longmemeval-protocol.md)
  and October 8 budget authorization.
- Non-goals: BEAM, LoCoMo, the `_m` haystack, vendor head-to-head tables.
- Delivery shape: frozen fetcher, opt-in harness, retained report with all
  three accuracies and the per-type breakdown.
  `scripts/summarize-longmemeval.py` reproduces the official aggregates from
  completed raw judgments and adds latency, delivered input tokens, known
  billing and explicit unavailable measurements without provider calls.
  A separate campaign summary reconciles all original/discarded experiments,
  identifier-free preflights and audited proxy receipts against a saved key
  account observation. Duplicate paid identities are refused; unknown charges
  retain conservative reservations. Preserved database copies add no spend.

## Governing Sources

[Integrated evaluation contract](../../../contracts/operations-integrated-evaluations.md),
[answer contract](../../../contracts/retrieval-answers.md),
[model policy contract](../../../contracts/memory-provider-policy-and-learning.md),
[epic](../../epics/operational-readiness.md), and the
[2026-09-29 lifecycle evidence](../../../mappings/atlas-lifecycle-proof-2026-09-29.md)
that motivates the answer-level complement. The accepted LongMemEval protocol
linked in the Summary governs the budgeted run.

## October 8 authorized scope

The accepted [protocol amendment](../../../contracts/operations-longmemeval-protocol.md)
authorizes the USD 10 campaign: frozen 50-question stratified LongMemEval-S,
HotpotQA lexical/Qwen retrieval, Atlas lifecycle, actual-model learning scenarios
and a matched baseline/comparator where feasible. Spend reservation ceiling USD 8;
no normal Brain writes or blind retries. OpenRouter integration is the first
dependency; local answering, judging, explicit failure recovery and campaign accounting are complete.

## Scope

- In scope: dataset/evaluator freezing with SHA-256 pins, canonical haystack
  ingestion into a dedicated bench Brain, frozen Ask answering budget, pinned
  `gpt-4o-2024-08-06` judging with a bounded disagreement audit, the
  three-accuracy report.
- Out of scope: everything the protocol defers. The explicit failure-rerun
  amendment permits the independently tested admission correction and fixed
  GLM recovery route; retain original results and label the combined score.
- Blockers: none. Original uncertain experiments A/B remain retained. C measured
  all 50 questions and its 50 primary/25 audit judgments completed. Explicit
  recovery retained 22 C answers and obtained the remaining 28 through two
  fixed-endpoint waves. All 50 final answers, primary/audit judgments and direct
  history/receipt proof are complete. The data, recall budget, prompt and native
  timeout were unchanged; endpoint revision and admission correction are declared.

## Surface and Interface Changes

- Interfaces: new opt-in fetcher script and Playwright spec mirroring the
  public-benchmark pattern; no product API change.
- Storage: owned ephemeral proof database; frozen inputs and report under
  ignored `.cache/`; evaluator content-addressed under the owned cache and pinned by the fetcher.
  Success does not clean up a fixture with unresolved paid receipts. An owned,
  restore-verified snapshot preserves experiment A's uncertainty across the
  already-running older wrapper's cleanup; no server/worker runs on that copy.
- Ownership: evaluation harness; answer-path behavior stays governed by the
  existing answer/model contracts.

## Data and Authority

- Inputs: `longmemeval_s_cleaned.json` (hash-pinned at fetch) from the pinned
  upstream commit; questions/gold/labels never enter memory.
- Authority: the upstream evaluator defines scoring; this repo adds no local
  abstention or grading rule.
- Blind spots: pipeline-level score (six stages, memory layer is one);
  single-run variance; the user authorized failure recovery and a bounded judge audit, not repeated fresh full-suite averages.

## States and Edge Cases

- Loading: full haystack ingest drained before any question runs.
- Empty: unanswered questions count as misses; no discarding.
- Known terminal failure: empty hypothesis and original failure/paid ledger retained
  in that experiment; remaining independent questions continue. Explicit authorized
  recovery uses fresh identities for known failed cases and retains successful
  answers and original costs separately. Report
  pipeline states separately from measurement completion.
  This includes a reconciled terminal indexing request failure: retain its
  request/code, cancel remaining work through the Brain's policy fence, and
  record no Ask response, latency or recall for that case. Unknown paid states
  still stop the experiment; unavailable cost retains a reservation.
  A known HTTP 400 `invalid_input` history-import rejection likewise stays
  failed with the original unchanged history, no Ask or semantic dispatch, and
  unavailable latency/recall. The original experiment does not re-import or sanitize the rejected history.
  Authorized recovery after the independently validated admission correction
  imports the unchanged previously uncommitted session; other committed sessions
  and successful answers are retained.
- Error: uncertain paid attempts stop the run; database retained for
  reconciliation per the semantic-benchmark precedent.
- Duplicate or replay: question and request identities persist before
  dispatch, so a resumed run cannot double-answer or double-judge a question;
  re-ingesting an already-imported haystack session is refused by the run's
  owned-database boundary rather than silently deduplicated.
- Blocked: uncertain requests stop their experiment and retain state.
- No-access: questions, gold answers and evidence-session labels never enter
  memory, so no recall path can surface them; the bench Brain is owned by the
  proof installation and no normal credential reaches it.
- Stale data: dataset drift refused by the fetcher; report names file, hash
  and commit or is invalid.
- Reconciliation divergence: usage ledger vs report tokens reconciled before
  cleanup.

## Integrations and Runtime Inputs

- Providers: OpenRouter Qwen embeddings, GLM answers and the dated GPT-4o judge;
  isolated daily token ceiling and concurrency recorded before dispatch.
  The product gateway remains the sole ingest/answer dispatcher; the unmodified
  official evaluator uses a separately audited loopback transport.
- Environment: opt-in flags on the existing test-ui lifecycle.
- Secrets: `.env` provider key only; no key material in reports.
- Failure handling: no blind retry; failed runs preserve owned state.

## Tests and Acceptance

- Automated: fetcher hash gate; ingest drain; every question measured once per experiment,
  with original failures retained, fresh authorized recovery identities and no
  replay of successful answers; each scored result has 50 primary judgments and
  25 audit judgments; report regenerates from the raw answer file with the
  vendored evaluator.
- Manual: review the three accuracies, per-type table, cost sheet, and
  declared N/A fields before closing the mapping.
- Acceptance: a dated tracked mapping containing the full report with dataset hash,
  commit, prompt variant, judge model, usage and per-type breakdown, with
  normal Brains untouched and the cost within the approved estimate.

## Closeout

- Planned: frozen 50-question answering, official judging and 25-question audit.
- Shipped: reproducible opt-in harness and offline summaries; complete original
  C measurement and combined recovery with 50/50 available answers; all three
  official accuracies/per-type outcomes, matched Cognee vector comparison,
  support/lifecycle diagnostics and reconciled unique campaign receipts.
- Result: combined recovery overall 20/50 (40%), task-averaged 36.71%,
  abstention 8/8 (100%); audit disagreements 0/25. Original C retains 22/50
  available answers and official 24% overall accuracy, including four failed
  blank hypotheses credited as abstentions. Quality gaps remain explicit.
- Cost: known reported charges USD 0.21783050; 40 unknown bills retain USD
  4.07071 reservation. Known costs plus reserves USD 4.28854 remain below the
  USD 8 ceiling. Timestamped key observation differs from reported receipt sums
  and is retained as an unresolved accounting discrepancy, not a final invoice.
- Not shipped: full official suite, large-scale memory proof, automatic-learning
  benchmark, full Cognee graph score, batch/local serving or normal-stack rollout.
- New blockers: none for this evaluation deliverable. Earlier uncertainties,
  weak answer quality, support-safety failures and broader baseline failures are
  findings in the report, not erased by the recovery.
- Docs updated: governing selection/protocol amendments, owning slices/epics,
  execution indexes, runbooks and the
  [complete dated report](../../../mappings/openrouter-benchmark-proof-2026-10-08.md).
- Validation: 50 full histories/2,402 canonical sources match frozen bytes and
  timestamps; 50 complete compatible indexes, 50 distinct successful gateway
  answer receipts, zero active native requests. Required repository validation,
  focused admission/route fixtures, 76 non-ignored workspace tests (229 ignored),
  browser/type proof and offline official metric/accounting parity pass. The
  unchanged broader suite's same 12 failures remain disclosed.
- Version: N/A.
- Commit: uncommitted; no push or deployment.
