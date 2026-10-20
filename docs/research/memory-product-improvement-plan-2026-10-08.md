# Remaining memory-product improvement proposal

Status: first-batch acceptance partial; larger follow-ups tracked by the current roadmap
Original audit: 2026-10-08; reconciled against current code on 2026-10-18
Owner: operational readiness, with retrieval, memory, evidence and graph owners

The [current roadmap](../roadmap/epics/index.md) owns status and priorities.
This proposal retains the unsolved design/evaluation choices from the benchmark
audit. It is not a second implementation plan or authority for new behavior.
The [dedicated organization plan](automatic-knowledge-mapping-plan-2026-10-09.md)
and its accepted contract/pack own automatic entities, aliases and topics.

## Evidence and current state

- [Completed benchmark](../mappings/openrouter-benchmark-proof-2026-10-08.md):
  all 50 LongMemEval answers are available after recovery, with official overall
  accuracy 20/50 (40%). Failures and uncertain billing remain in the report.
- [First batch](../mappings/memory-quality-first-batch-proof-2026-10-08.md):
  whole source chunks and multiple attributed windows, historical/personalized
  answer handling, bounded graph entry and an updater with checkpoints are
  implemented. This does not establish a new answer-quality score.
- [Latest checkpoint](../mappings/automatic-knowledge-mapping-progress-2026-10-09.md):
  graph preparation and bounded support discovery are committed. The last normal
  graph series passed 35/40, p95 4.027 s; the capture-only 10k-node fixture passed
  40/40, p95 0.513 s. Digest/handover writer tenure remains a concrete repair seam.
- Automatic capture, learning, support checks, settled-session digests, project
  briefs and exact-family cross-session reconciliation already exist. The
  benchmark's source-only lane does not measure the whole automatic pipeline.
- Candidate entity parsing/storage exists. Production organization, aliases,
  topics and their qualified lifecycle/navigation remain unfinished.

The old runtime/code migration mismatch was repaired through migration 047 in
the October 9 installation proof. Do not keep listing migration 035 as the
current installation. Recheck actual build/schema before new runtime work;
committing source does not deploy it.

## Product behavior we should aim for

A useful answer should retrieve the right source spans, preserve who/when/scope,
explain contradictions and cite exact support. Ordinary capture and organization
should run automatically within the configured Brain permissions and budgets.
The graph should open promptly with useful small neighborhoods and groups.
Erasure, correction and scope changes must invalidate every derived copy.

## Remaining work and decisions

| Area | Existing behavior | Remaining work and proof |
| --- | --- | --- |
| Background work and graph latency | Bounded graph entry, off-lock preparation, semantic-discovery repair and bounded audit scanning | Shorten cumulative digest/handover writer work; prove normal and 10k-node capture/audit/rebuild workloads without losing authority, deadlines or supported contributors |
| Answer evidence and usefulness | Whole canonical chunks, bounded alternative windows, history and preference instructions | Rerun comparable frozen questions; measure delivered answer-bearing spans, support/citation accuracy, abstention and failure rates before changing ranking or prompts again |
| Full automatic memory | Capture, support verification, digests, scoped context and exact-family corrections | Compare source-only, learning-enabled and full captured-session arms with explicit stage settlement; measure delayed/corrected/contradictory memories, noise and budget starvation |
| Typed time and broader reconciliation | Recorded/valid time and exact normalized assertion families | Specify event-time extraction, relative dates, changed preferences and ambiguous identity resolution; preserve attribution and historical questions; new schemas/rules need an amendment |
| Semantic organization | Manual groups, provenance graph, analytics and typed mapping candidates | Deliver the existing organization contract: production worker, aliases, overlapping topics, overrides, API/UI, incremental correction/erasure and ancestor-fence closure |
| Supported relationship views | Qualified source/claim/repository provenance edges | Evaluate additional typed semantic relations separately from facts; require independently eligible support for assertions and separate navigation-only candidates |
| Search and ingestion scale | Exact eligible-vector baseline and durable jobs | Measure index size, scan cost, write-to-readable lag and queue fairness; consider scoped ANN only against the exact oracle with Brain/scope isolation and deletion tests |
| Reliability and cost visibility | Durable provider outcomes, budget controls, explicit incomplete/unknown states | Measure stage/provider/DB/UI latency separately, account for failures and unresolved bills, and prove crash/retry/offline recovery; any new provider fallback needs a contract |
| Installed host experience | Plugin source/HTTP/configuration and prior native proofs | Finish the current native-host acceptance gaps without confusing configuration recognition with working capture/recall |

## Open-source reuse boundaries

The organization plan records pinned primary sources for Cognee's entity/chunk
mapping and optional context grouping, Graphify's structural extraction and
Graphiti's temporal/entity resolution. These are design candidates, not mandatory
runtimes. Recollect retains Rust/PostgreSQL authority and Neo4j/GDS projections.

The local GLiNER experiment failed its declared quality gate. Improving a schema
or selecting another local/cloud extractor requires development comparison and
untouched held-out proof. Graphify must beat or complement Enola on identical
repository inputs before adoption. A new Python service needs an explicit
adapter/runtime decision; no service is selected by this research.

The inspected claude-mem worker used SQLite WAL, bounded search/detail queries,
progressive hydration and asynchronous observation work. Those patterns motivate
bounded discovery and shorter writer transactions; they do not demonstrate that
replacing Recollect's graph database would fix the measured lock waits. The
[original pinned comparison](https://github.com/MikeK184/Recollect/blob/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs/research/memory-product-improvement-plan-2026-10-08.md#graph-performance-reuse-follow-up--2026-10-09)
retains method and source references.

## Evaluation that can prove improvement

Preserve the [frozen holdout manifest](memory-product-holdout-manifest-2026-10-08.json)
and [organization fixtures](automatic-knowledge-mapping-fixtures-2026-10-09.json).
Do not tune on held-out questions or silently change chunking, model, scorer or
failure denominators. Record each arm's configuration and full settlement state.

1. Run deterministic lifecycle/authority controls first: current versus historical
   revisions, aliases and DEV/PROD separation, rejection, erase/re-ingest/rebuild,
   ancestry fences, restore, no-access and scope change. No LLM judge is needed.
2. Measure populated retrieval/graph latency with every timeout/failure counted;
   report concurrency, data volume, provider calls and native resource use.
3. Compare source-only retrieval/answering with the existing baseline, then add
   learning and faithful session capture as separate controlled arms. Diagnose
   missing evidence separately from wrong reasoning and unsupported answers.
4. Score semantic answers with a declared evaluator; audit disagreements manually.
   Compare model-judged results only under the same frozen protocol. Report
   accuracy, citations, abstention, availability, latency and total known/unknown
   cost together.

The earlier paid campaign's allowance is not a new ongoing budget. Account for
retained unresolved charges before any further paid run, and apply the accepted
[benchmark protocol](../contracts/operations-longmemeval-protocol.md). Offline
fixtures do not require a new paid benchmark or model judge.

## Ownership and explicit deferrals

Continue the existing operations/concurrency/organization packs for authorized
work. Typed event-time rules, new semantic relation authority, ANN, new reranking
or fallback behavior require their own decision-complete scope before code.
Their proposed owners remain memory lifecycle, graph intelligence, hybrid
retrieval and operational readiness respectively.

Learned retrieval gates, adaptive decay, reinforcement ranking, advanced reranking,
automatic executable-skill learning and multi-device synchronization are optional
future choices. Progressive recall, raw-source retention, digests and exact-family
reconciliation already have implementations and should be evaluated before adding
another mechanism. Current desktop polish and plugin hook behavior are in their
contracts/runbooks; previous UI reviews are not an open product backlog.

Version: N/A. Source baseline: `f2673d7`; documentation reconciliation uncommitted.
No benchmark, deployment or product implementation was performed for this cleanup.
