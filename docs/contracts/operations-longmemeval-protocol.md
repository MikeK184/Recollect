# LongMemEval answer-level benchmark protocol

Status: accepted

## Budgeted public evaluation amendment — 2026-10-08

The user explicitly authorizes actual benchmarks using the USD 10 OpenRouter
key allowance and delegates model/dataset selection. This supersedes the earlier
no-spend gate. Freeze a stratified 50-question LongMemEval-S cleaned subset,
including all six types and abstention, before dispatch; preserve full-file hash,
subset IDs and upstream commit. Run the existing HotpotQA and Atlas lifecycle
harnesses first. Reserve at most USD 8 for the complete campaign, leaving USD 2
headroom; record per-call reservations, actual usage and failures without silent
retry. Pilot before expanding. OpenRouter Muse/GLM/Luna answering and Qwen embeddings
are permitted; record the exact selected pair. Use the pinned upstream judge
model/prompt where available and explicitly label any unavailable or diagnostic
replacement. No replacement may be described as the original official score.
A matched corpus/settings comparison with Cognee or a reproducible retrieval
baseline is authorized within the same ceiling; vendor scores remain context
only. Separate retrieval-only ingestion from autonomous-learning scenarios so
answer accuracy cannot hide correction/deletion errors. Existing normal Brains
remain untouched. This amendment authorizes the bounded implementation and
execution; it does not require another approval step.

## Explicit failure recovery amendment — 2026-10-08

After the original 50-case measurement completed, the user explicitly requested
rerunning failed questions to obtain answers for all 50 and a full Markdown
report. This authorizes bounded fresh attempts for known terminal failures;
uncertain original attempts remain unrepeated. Retain the original run, score,
request IDs and costs. A separately named recovery report reuses the 22 original
completed answers and retries the 28 failed cases with fresh identities. Label
its combined score as recovery, not single-pass performance. Record every
additional failed attempt and charge; known rate limits may be attempted again
only as an explicit bounded recovery wave, with no silent gateway retry.

The rejected history may resume canonical import after the confirmed TypeScript
credential-classifier false positive is fixed and independently tested; do not
edit or sanitize benchmark histories. Existing compatible indexes are reused.
GLM endpoint selection may be revised within the delegated model choice and
existing strict schema/price/timeout fences. Both original and recovered answers
use the pinned upstream grading rule; the original evaluator accepts bounded
judge content even with `finish_reason=length`, so the transport must not impose
an incompatible stop-only rule. Keep incomplete original judge attempts and
billing, and distinguish newly authorized judge recovery from duplicated success.
The final Markdown report includes original availability, recovery attempts,
quality, costs, timings, remaining failures, dataset pins and comparison limits.
The same USD 8 campaign ceiling and USD 10 key limit apply.

## Source

The user's 2026-09-29 benchmark request selected this protocol as a draft
with no spend. The October 8 authorization above accepts the protocol and
resolves that historical gate. The
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
  extraction and learning are disabled for this retrieval/answer lane and
  named in the report. Native support auditing is measured separately.
- The Brain model policy enables exactly two purposes — embedding for the
  semantic channel and answer generation — under an isolated daily token
  ceiling and concurrency limit recorded in the report before dispatch.
  Request identities persist before dispatch; uncertain attempts stop the run
  rather than resend.
  A known terminal question failure remains in the full denominator with an
  empty hypothesis and its exact pipeline failure code; independent subsequent
  questions continue without retrying the failed one. Report answer availability
  separately from the harness's completion. Retain reservations for requests
  whose billed cost is unavailable, even when their failure outcome is known.

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

- The October 8 user authorization permits dispatch within the USD 10 key
  limit and USD 8 campaign reservation ceiling. Retain dated rates, bounded
  token estimates, per-call billed usage and unresolved reservations.
- If a stage fails mid-run, the owned database and request ledger are retained
  for reconciliation per the public-benchmark precedent; no blind retry.
- A diagnosed model/configuration change starts a separately named experiment
  with fresh answer identities and a frozen configuration before dispatch.
  Existing compatible indexes may be reused; keep original results and costs,
  exclude their request IDs from incremental new-run accounting, and retain
  unresolved reservations. Do not replace failed results with the new score.

## Acceptance

A retained report under ignored `.cache/` plus a tracked dated mapping that: names dataset
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
