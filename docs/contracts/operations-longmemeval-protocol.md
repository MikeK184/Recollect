# LongMemEval answer-level benchmark protocol

Status: proposed

## Source

The user's 2026-09-29 benchmark request selected this protocol as a draft
with no spend: it may not dispatch any paid model call until the user
separately approves its cost budget. The
[integrated evaluation contract](operations-integrated-evaluations.md)
defers BEAM/LongMemEval runs to "a separate frozen ingest/learning protocol,
cost budget, and answer evaluator before dispatch"; this document is that
protocol. The [agent-memory-atlas benchmarks page](https://neoneye.github.io/agent-memory-atlas/benchmarks/)
supplies the failure modes this protocol must repair: LongMemEval publishes
three different accuracies from one run, pins none of its data, and its
published scores are a six-stage pipeline measurement of which the memory
layer is one stage. The [2026-09-29 lifecycle evidence](../mappings/atlas-lifecycle-proof-2026-09-29.md)
shows the deterministic half is already measured; this contract governs only
the answer-level half.

## Contract

### Frozen inputs (before any dispatch)

- Dataset: the official LongMemEval Hugging Face release files
  (`xiaowu0162/longmemeval`, `longmemeval_s_cleaned.json` for the small
  haystack; `_m_cleaned` only by separate explicit approval). A fetcher
  modeled on `scripts/fetch-public-benchmark.py` downloads each file once and
  refuses any SHA-256 drift. The report must name the data file, its hash, and
  the upstream repo commit; a result without those fields is invalid on this
  harness, because the September 2025 cleaning change made pre/post files
  different benchmarks.
- Evaluator: the upstream `evaluate_qa.py` and `print_qa_metrics.py` from the
  pinned commit are vendored content-addressed (SHA-256 recorded) and run
  locally unmodified; the judge prompt and `gpt-4o-2024-08-06` target are
  taken from the same commit, not chosen after seeing answers.
- Questions, gold answers, and evidence-session labels never enter memory.
  Only haystack sessions are ingested.

### Installation and ingestion

- A dedicated bench Brain on an owned ephemeral proof database through the
  existing `scripts/test-ui.sh` lifecycle; normal Brains, credentials, and the
  live stack are untouched.
- Ingestion uses only canonical product paths: each timestamped haystack
  session becomes a retained source through the source API and the real
  worker, preserving session timestamps as observed time. No synthetic claims,
  no hand-authored memory, no benchmark-specific prompt or index tuning;
  extraction/learning settings are the installed defaults and are named in
  the report.
- The Brain model policy enables exactly two purposes — embedding for the
  semantic channel and answer generation — under an isolated daily token
  ceiling and concurrency limit recorded in the report before dispatch.
  Request identities persist before dispatch; uncertain attempts stop the run
  rather than resend.

### Answering and judging

- Every question runs the governed Ask path with a frozen prompt and a
  declared recall budget (channels, item count, context bytes) identical
  across all six question types, recorded in the report.
- The pinned judge grades answers; judge calls count against the same
  pre-approved budget. A bounded disagreement audit (second sample on a
  declared subset, minimum 25 questions) is reported alongside the primary
  judge scores.
- Abstention items are scored by the official evaluator's rule, not a local
  heuristic; the report separates abstention accuracy from the other two.

### Reporting

- Publish all three accuracies the official harness produces — task-averaged
  (macro over six types), overall (micro), and abstention — plus the
  per-type breakdown including any type scored badly, delivered-token and
  latency columns, actual provider usage and cost, dataset hash, commit,
  prompt variant, and every declared `N/A` with its reason. Silence is never
  scored as success.
- The report labels the score as a six-stage pipeline measurement (extraction
  defaults, storage, retrieval, prompt assembly, answering model, judge) and
  makes no head-to-head claim against vendor-published numbers, which name
  none of these fields.

### Gate

- The run is dispatched only after the user approves the written cost estimate
  (tokens × dated rates for ingest, answer, and judge passes) in explicit
  terms. Approval of this protocol's drafting is not approval to spend.
- If a stage fails mid-run, the owned database and request ledger are retained
  for reconciliation per the public-benchmark precedent; no blind retry.

## Acceptance

A committed report under `.cache/` plus a dated mapping that: names dataset
file/hash/commit, reproduces the vendored evaluator's numbers from the raw
answer file, reports all three accuracies and the per-type table, states
usage/cost/latency, and declares every untested path. The harness reruns
green from the frozen inputs without touching normal Brains.

## Explicit Deferrals

BEAM, LoCoMo, MemoryAgentBench, GoodAI LTM, PersistBench, and the large
`_m` haystack remain deferred; multi-run averaged headlines are deferred
unless the user approves the multiplied budget; no comparison table with
vendor-published scores is produced; the deterministic §6/§7 lifecycle matrix
stays the separate judge-free harness it already is.
