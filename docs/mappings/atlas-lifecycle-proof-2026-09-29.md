# Atlas lifecycle proof and public benchmark reproduction

Observed: 2026-09-29
Confidence: observed-once

## Sources and Method

- [agent-memory-atlas benchmarks page](https://neoneye.github.io/agent-memory-atlas/benchmarks/)
  §6 (thirteen-step deletion sequence) and §7 (contradiction matrix), read on
  2026-09-29; local reference checkout `agent-memory-atlas/` is unmodified.
- Harness: `web/tests/atlas-lifecycle.spec.ts`, run through
  `RECOLLECT_ATLAS_LIFECYCLE=1 RECOLLECT_TEST_MODEL_WORKER=1 ./scripts/test-ui.sh tests/atlas-lifecycle.spec.ts`
  against an owned ephemeral PostgreSQL proof database with the real worker;
  the database and graph fixture were removed after the green run. Zero model
  calls: the Brain's model policy stayed disabled and the harness asserts
  `models/usage.total == 0`. Every verdict is machine-scored from recall
  qualifications, claim state, erasure status, graph view, evidence catalogue
  and audit events — no LLM judge.
- Reproduction: `python3 scripts/fetch-public-benchmark.py` (frozen HotpotQA
  input SHA-256 verified) then
  `RECOLLECT_PUBLIC_BENCHMARK=1 RECOLLECT_PUBLIC_SEMANTIC=1 ./scripts/test-ui.sh tests/public-benchmark.spec.ts`
  on the same day. The first two attempts failed on infrastructure, not
  measurement: a Brain-admission-lock 503 during ingest (live Docker stack
  competing for the same PostgreSQL) and a 60-second 504 on one recall under
  the same contention. The retained failed-run database was inspected
  (zero `model_requests`, so no uncertain paid attempt), dropped, its graph
  fixture cleaned, and the pair reran with the stack's api/worker containers
  stopped.

## Observations

### HotpotQA reproduction — digit-for-digit

Baseline (exact+lexical): supporting-document recall@10 5.0%, complete support
2% (1/50), precision 4.79%, 46 empty queries, p50 236 ms / p95 255 ms, 0 model
requests. Semantic (exact+lexical+semantic): recall@10 100%, complete support
100%, precision 20%, 0 empty, p50 1,078 ms / p95 1,252 ms, 97 model requests.
Every accuracy count and the model-request count match the
[2026-09-28 mapping](public-memory-benchmark-2026-09-28.md) exactly; latency
differs within run-to-run noise.

### Atlas §6/§7 matrix — 16 rows, 40 cells, 0 FAIL

Deletion sequence (§6, steps 1–10): baseline retrievable PASS; after source
erasure — erasure complete PASS, all four recall modes token-clear PASS, claim
answers as a value-free tombstone (`selection_state=erased`, `selected=null`)
PASS; re-ingesting the identical source text resurrects no claim revision and
asserts nothing current PASS (raw re-feed evidence: 0 hits); after a
knowledge-graph rebuild and privacy maintenance the verdicts are unchanged
PASS. Derived stores: evidence catalogue retains only a tombstone row without
plaintext or active availability PASS, excerpt of the erased version refused
PASS, graph view shows no stale current assertion PASS; semantic index and
handovers/manifests are declared `N/A` (embedding disabled / no model). Audit:
`memory.erase` event recorded and no canary value appears in any audit event
PASS. Steps 11–13 (propagated copies to second scopes/devices) declared `N/A`:
no publication or paired device exists in the disposable proof Brain.

Contradiction matrix (§7): replacement, polarity flip, retraction, partial
supersession and bounded validity all PASS on current-answer delivery (A),
retrieval hygiene (B — stale values absent or machine-qualified), history
knowability (D — revision history and qualified history-mode recall) and
durability after the background sweep (C). Equal-weight contradiction PASS:
accepting the second same-family claim is refused with HTTP 409 (no silent
pick), and the `keep_both` resolution with disjoint validity windows makes
each value current only inside its own window at `fact_at` query time
(R_disjoint_resolution PASS). Adversarial re-entry PASS: re-proposing a
corrected-away value returns admission `blocked_by_rule` (durable
rejected-value tombstone). Measured extras: write-to-readable lag 1,058 ms
(source ingest → accepted claim unqualified in investigation recall), recall
latency p50 35 ms / p95 51 ms / max 72 ms over 24 probes, erasure closure
covered 1 source version + 2 claim revisions + 1 artifact, graph view 13 nodes.

Harness iteration record (disclosed for auditability): the first green run was
preceded by two runs whose six red cells were all harness-semantics mismatches,
not product defects — expecting 404 instead of the designed tombstone read,
one conjunctive lexical query for two claims, overlapping validity windows in
the equal-weight case, the wrong graph-completion state vocabulary, and
treating the catalogue's addressable tombstone row as a leak. No product code
changed; the metric definitions come from the Atlas page and were fixed before
scoring. The intermediate red matrix is preserved in
`.cache/atlas-lifecycle-*` report files.

## Translation and Limits

- This is delivery-level evidence through the real product API: Recollect's
  assembled recall context is the prompt prefix an answerer would receive, and
  the Atlas page's own "cheap alternative" (machine-readable qualifiers) is
  what B/E scoring used. It is not an answer-level score — no model answered,
  so §7's A column measures current-value delivery, not generated text.
- The §6 mapping to Recollect distinguishes erasure (record-scoped removal,
  answerable only as a tombstone) from the rejected-value rule (blocks
  re-assertion). Both were probed separately; the Atlas harness conflates them
  because most systems have neither.
- Single run, single fictional corpus, one disposable Brain, local hardware,
  debug binaries; not a concurrency or capacity measurement. Steps 11–13 and
  the semantic/handover leak probes are untested paths here, declared rather
  than scored.
- The HotpotQA reproduction validates reproducibility of our own harness on a
  frozen corpus; it remains pooled evidence retrieval, not an official
  HotpotQA, LoCoMo, LongMemEval or BEAM score, and no published vendor number
  is comparable on these harnesses.

## Follow-up

- A LongMemEval answer-level run needs the [proposed protocol contract](../contracts/operations-longmemeval-protocol.md)
  accepted with an explicit user cost approval before any dispatch; the
  protocol freezes dataset hashes, the pinned judge, the three-accuracy report
  shape and stopping rules.
- Rerun the lifecycle harness after any change to erasure, review, conflict or
  recall qualification semantics; the six harness fixes above are the
  calibration record for what the product actually promises.
