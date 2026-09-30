# Public memory benchmark

Run from the Recollect checkout with the local PostgreSQL/Neo4j stack available:

```sh
python3 scripts/fetch-public-benchmark.py
RECOLLECT_PUBLIC_BENCHMARK=1 ./scripts/test-ui.sh tests/public-benchmark.spec.ts
```

The wrapper builds the current application, creates a separate owned PostgreSQL
database and listener, processes the corpus using the real worker, and removes
that proof database after success. Existing Brains and policies are untouched.
Results remain under `.cache/public-benchmark-<run-id>/report.json`; failed runs
are preserved. Zero model calls are required by this baseline.

To compare the same corpus and budgets with the installed semantic retriever:

```sh
RECOLLECT_PUBLIC_BENCHMARK=1 RECOLLECT_PUBLIC_SEMANTIC=1 ./scripts/test-ui.sh tests/public-benchmark.spec.ts
```

This separately runs the baseline, indexes public paragraphs using the installed
embedding provider, then runs exact+lexical+semantic retrieval. It enables only
embedding, with a 500,000-token ceiling and two concurrent calls in the isolated
Brain. There is no extraction, learning, answer generation or LLM judge. Reports
include usage and index coverage. Query identities are saved before dispatch.
A failed semantic run preserves its owned database and private artifacts so
uncertain attempts can be inspected before any intentional new paid run.

## Corpus and interpretation

Source: [HotpotQA](https://hotpotqa.github.io/) and its official
[Hugging Face dataset](https://huggingface.co/datasets/hotpotqa/hotpot_qa),
CC BY-SA 4.0. The frozen input is the first 50 rows of validation/distractor,
SHA-256 `32dd92947d6c50194bc2e76588bc78ad6ad08805a4b05728336ac70cd7e19b96`.
The fetcher refuses changed input. Keep the original file for repeat runs.

All supplied paragraphs are pooled in one Brain, deduplicating identical
title/content pairs. Every paragraph, including distractors, goes through the
source API and canonical worker. Gold labels, answers and questions are never
ingested. Each query receives ten items and at most 16 KiB using exact+lexical
retrieval, with source diversity enabled. No query-specific tuning is applied.

Supporting-document recall is the fraction of unique gold titles retrieved.
Complete support requires every supporting title for that question. Precision
is gold titles divided by unique returned titles, or zero for an empty result.
These metrics are document-level: they do not prove the specific supporting
sentence survived the context budget or that an answer is correct. Latency is
client-observed on the local proof stack; report the corpus size and index-ready
condition alongside it.

This pooled evidence-retrieval variant is not the official HotpotQA answer or
supporting-sentence EM/F1 leaderboard, nor a BEAM/LongMemEval conversational-memory
score. Comparing systems requires the same corpus, budgets, model, ingestion,
queries and evaluator. Cognee's published BEAM scores use different inputs and
benchmark-specific prompts and cannot be compared to this score.

Correction, re-ingestion, erasure and access isolation are separately verified by
the production-handler tests and the [evaluation contract](../contracts/operations-integrated-evaluations.md).

## Atlas lifecycle proof

The benchmarks page's own two specified tests run as a deterministic, judge-free
pass/fail matrix through the product API in a separate owned proof database:

```sh
RECOLLECT_ATLAS_LIFECYCLE=1 RECOLLECT_TEST_MODEL_WORKER=1 \
  ./scripts/test-ui.sh tests/atlas-lifecycle.spec.ts
```

`web/tests/atlas-lifecycle.spec.ts` covers §6 deletion steps 1–10 (write →
retrieve → erase → probe all four recall modes → re-ingest the original source
material → background sweep → derived-store leak probes → audit) and the §7
contradiction matrix (replacement, polarity flip, retraction, partial
supersession, bounded validity, equal-weight contradiction scored on
current-delivery, hygiene, durability, history and derived reach, plus
adversarial re-entry). It requires zero model calls and asserts the Brain's
usage stays at 0. Erased records answer as value-free tombstones by design, so
the probes accept tombstone reads rather than 404s; untested paths (propagated
copies, semantic/handover stores without a model) are declared `N/A` with a
reason. Reports land in `.cache/atlas-lifecycle-<run-id>/report.json`.

The [2026-09-29 result](../mappings/atlas-lifecycle-proof-2026-09-29.md)
records the 40-cell green matrix, the measured write-to-readable lag and recall
latencies, the digit-for-digit HotpotQA reproduction of the 2026-09-28 numbers,
and the harness calibration record. An answer-level LongMemEval run is governed
by the separate [proposed protocol](../contracts/operations-longmemeval-protocol.md)
and requires explicit user cost approval before dispatch.

The [2026-09-28 result](../mappings/public-memory-benchmark-2026-09-28.md)
records the paired lexical/semantic measurement, provider usage, lifecycle proof
and comparison limits.
