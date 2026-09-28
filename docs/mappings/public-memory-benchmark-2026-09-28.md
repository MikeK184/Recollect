# Public retrieval benchmark and Atlas lifecycle proof

Observed: 2026-09-28
Confidence: observed-once

## Sources and Method

- [HotpotQA](https://hotpotqa.github.io/) and the official
  [dataset](https://huggingface.co/datasets/hotpotqa/hotpot_qa), CC BY-SA 4.0.
  First 50 validation/distractor rows, frozen before evaluation, input SHA-256
  `32dd92947d6c50194bc2e76588bc78ad6ad08805a4b05728336ac70cd7e19b96`.
- `scripts/fetch-public-benchmark.py` and `web/tests/public-benchmark.spec.ts`.
  Reproduction and metric definitions: [runbook](../runbooks/public-memory-benchmark.md).
- Local Cognee reference `c0d18c80e24b7b78918e7642c03f6f128fdd2aee`, including
  `cognee/eval_framework/beam/REPORT.md` and dataset adapters. Its published
  [BEAM report](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/eval_framework/beam/REPORT.md)
  uses conversational memory, benchmark-specific prompts and different inputs.
- Local Atlas reference `7eca7f7abd934c2e44bc096fc1dd0cbf94275b99`,
  `content/overview.md` and `content/patterns/hybrid-retrieval-fusion.md`.
  Atlas provides implementation-audit patterns and blind spots, not a runnable
  cross-system quality leaderboard.

All 491 distinct title/content documents were ingested and processed through the
source API and native worker in an isolated Brain. Every query searched the same
pooled corpus, including the distractors from other questions. Answers, questions
and supporting-fact labels were not ingested. Both runs used ten results,
16,384 context bytes and source diversity. The semantic run used the installed
`text-embedding-3-large`, 3,072 dimensions, with automatic indexing only. No
extraction, learning, answer generation or model judge was enabled.

## Observations

| Measure | Exact + lexical | Exact + lexical + semantic |
| --- | ---: | ---: |
| Questions | 50 | 50 |
| Supporting documents found | 5 / 100 | 100 / 100 |
| Mean supporting-document recall at 10 | 5% | 100% |
| Questions with all supporting documents | 1 / 50 | 50 / 50 |
| Mean precision among returned titles | 4.79% | 20% |
| Empty results | 46 | 0 |
| Median query latency | 239 ms | 1,091 ms |
| 95th percentile query latency | 260 ms | 1,268 ms |
| Maximum measured context bytes | 10,547 | 16,326 |
| Model requests, including indexing | 0 | 97 |
| Charged tokens, including indexing | 0 | 78,323 |

The semantic index had 493 ready entries for 491 documents, with zero failed,
blocked, pending, queued, running or truncated entries. Forty-seven indexing
batches and fifty query embeddings completed. The database request ledger
independently showed 97 succeeded requests, no in-flight request and all 542
jobs succeeded. All fifty recorded query identities completed. An independent
read of the frozen labels and saved retrieved titles confirmed the hit counts.
The request ledger was exported locally; privacy/graph cleanup then completed
and the owned benchmark database and private artifacts were removed. The empty
database from an earlier pre-test shell failure was also verified unused and
removed. Public reports remain available; the normal product database was not
used for benchmark ingestion.

Raw local reports, retained under ignored `.cache/`:

- `.cache/public-benchmark-5aee398f-d2a8-4c25-a58b-5640d291ce9b/report.json`
- `.cache/public-benchmark-231bcaa3-10df-4792-9247-0c7c19022426/report.json`
- `.cache/direct-mcp-final-proof-2.log`

The combined harness also contained unrelated browser setup and Context7 quota
assertion failures. The benchmark itself completed both lanes and wrote complete
reports; it was not rerun to hide those failures or incur duplicate model costs.
The connection tests are tracked separately in the
[MCP proof](mcp-direct-connections-2026-09-28.md).

Fresh production-handler lifecycle tests also passed:

- `review_authority_durable_rules_revalidation_and_replay`
- `review_scope_boundaries_and_failed_audit_are_atomic`
- `autonomous_inflight_erasure_expiry_and_older_restore_preserve_independent_evidence`

They cover correction/replay authority, access isolation, and in-flight
erasure/expiry/restore exclusion with independent evidence preserved. These use
controlled provider responses where needed; they are behavioral proofs, not
live-model learning-quality scores. Log: `.cache/direct-mcp-lifecycle.log`.

## Translation and Limits

This is a small pooled evidence-retrieval benchmark. A returned supporting title
does not prove the exact supporting sentence survived the context budget or that
an answer would be correct. This is neither official HotpotQA answer/supporting-
sentence EM/F1 nor BEAM/LongMemEval. Fifty public questions and 491 short documents
cannot establish scaling, resistance to contamination, or long-term learning
quality. Latency is client-observed on the local proof stack with the semantic
index ready; it excludes ingestion and indexing.

Cognee's BEAM report includes a 100K result from twenty questions on one held-out
conversation and explicitly exploratory 10M results using overlapping prompt
selection/evaluation questions. Those scores cannot be ranked against this run.
Neither reference checkout was modified or run as a product dependency.

The measured lexical weakness is retained as a result. Semantic indexing is
material for natural-language recall; the benchmark did not change retrieval
code or tune settings after observing gold labels. Full learning, correction,
forgetting and answer quality still require larger frozen conversational and
longitudinal evaluations. Existing desktop acceptance packs remain open.

## Follow-up

Use the same frozen input and budgets for future comparisons; retain misses and
provider failures. A future BEAM or LongMemEval run needs a separate frozen
ingestion/learning protocol, cost allowance and answer evaluator before dispatch.
