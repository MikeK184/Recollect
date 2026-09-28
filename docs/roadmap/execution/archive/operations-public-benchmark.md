# Public retrieval benchmark

Status: shipped
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: reproducible measured evidence retrieval against public questions and fresh lifecycle proof.
- Non-goals: universal memory ranking, full BEAM or official HotpotQA answer scores.
- Delivery shape: opt-in harness and dated results.

## Governing Sources

[Evaluation contract](../../../contracts/operations-integrated-evaluations.md),
[vision](../../../foundation/vision.md) and [epic](../../epics/operational-readiness.md).

## Scope

- In scope: first 50 HotpotQA validation distractor rows, pooled paragraphs, lexical baseline and opt-in semantic comparison, scope/correction/erasure production tests, current Cognee/Atlas methodology research.
- Out of scope: customer documents, model learning or judge calls, tuning after observing gold labels.
- Blockers: None.

## Surface and Interface Changes

- Interfaces: opt-in Playwright API harness through existing source and recall APIs.
- Storage: disposable test-ui PostgreSQL database and ignored cache report only.
- Ownership: operations harness, no product retrieval changes.

## Data and Authority

- Inputs: official HotpotQA dataset viewer rows; frozen input hash and labels before dispatch.
- Authority: product ingestion, processing and recall; benchmark labels never enter memory.
- Blind spots: no generation quality, no conversational long-term memory score; small public subset is not customer capacity.

## States and Edge Cases

- Loading: wait for all imported source versions to finish processing.
- Empty: zero recalled evidence counts as a miss.
- Error: fail and retain diagnostics; do not silently drop questions.
- Blocked: unready sources or unavailable owned test listener stop measurement.
- No-access: source IDs and Brain provenance checked in every result; separate lifecycle negative tests.
- Duplicate or replay: deduplicate identical title/content paragraphs, preserve question labels and original row order.
- Stale data: freeze corpus file hash; never reuse normal Brain state.
- Reconciliation divergence: report exact index counts and model usage.

## Integrations and Runtime Inputs

- Providers: public dataset download; zero model calls in initial lane. Optional semantic lane uses installed embeddings with 500,000-token daily ceiling and two concurrent calls.
- Environment: opt-in `RECOLLECT_PUBLIC_BENCHMARK=1`; existing test database/server inputs.
- Secrets: no credentials in reports, copied data or output.
- Failure handling: bounded test timeout and no blind retries; retain failed output.

## Tests and Acceptance

- Automated: import and process all corpus documents, run all 50 queries, measure gold supporting-document retrieval under fixed budgets; focused Atlas lifecycle tests and `./scripts/validate.sh`.
- Manual: review metric definitions, corpus provenance and report limitations.
- Acceptance: reproducible report with every query including misses, no quality threshold selected after the run, no effect on seven normal Brains.

## Closeout

- Planned: public retrieval baseline and lifecycle evidence.
- Shipped: Frozen 50-question/491-document pooled HotpotQA harness, lexical and semantic reports, measured usage/latency and focused lifecycle proof. [Dated evidence](../../../mappings/public-memory-benchmark-2026-09-28.md).
- Not shipped: full BEAM, LongMemEval, learning/generation leaderboard comparison.
- New blockers: None.
- Docs updated: Evaluation contract, benchmark runbook, dated mapping, owning epic and indexes.
- Validation: Both 50-question reports complete; lexical supporting-document recall 5%, semantic 100%; 97 succeeded embedding requests and 78,323 charged tokens; three correction/access/erasure production-handler tests passed. Owned test data reconciled and removed. No universal quality claim or Cognee ranking.
- Version: N/A: no release policy.
- Commit: uncommitted.
