# Hybrid Retrieval and Investigation

Status: complete

## Purpose

Return useful, bounded, evidence-backed context from exact, lexical, semantic
and graph retrieval under one Brain/scope/revision/lifecycle contract. Give
engineers an investigation UI with explicit disagreement and strict modes.

## Governing Sources

- [Desktop ADR](../../adr/0014-desktop-experience-and-answers.md)
- [Knowledge surface ADR](../../adr/0017-desktop-knowledge-and-ask-experience.md)
- [Tiered surface contract](../../contracts/desktop-knowledge-surface.md)
- [Temporary answers](../../contracts/retrieval-answers.md)
- [Desktop contract](../../contracts/desktop-experience.md)
- [Canonical exact and lexical recall](../../contracts/retrieval-exact-and-lexical.md)
- [Governed semantic representations and recall](../../contracts/retrieval-semantic.md)
- [Scoped graph fusion and source-aware context](../../contracts/retrieval-graph-fusion.md)
- [Desktop memory investigation](../../contracts/retrieval-investigation-ui.md)
- [Vision: memory forms and methods](../../foundation/vision.md#one-memory-model-complementary-methods)
- [Vision: correction and recall](../../foundation/vision.md#correction-and-learning-loop)
- [Stack: storage and hybrid retrieval](../../foundation/techstack.md#storage-ownership-and-hybrid-retrieval)
- [Engineering principles: retrieval](../../foundation/engineering-principles.md#combine-retrieval-methods-under-one-contract)

## Dependencies and Boundaries

Consume canonical evidence, claims, manifests and eligibility from their owners.
Start exact/lexical retrieval once correction/erasure rules exist; do not wait
for all graph analytics or MCP connections. Semantic work consumes the shared
model gateway. Graph fusion consumes graph intelligence's supported path queries.

Own search/recall interfaces, candidate fusion, source attribution/deduplication,
context budgets, scoped caches, and investigation versus strict retrieval.
PostgreSQL remains the text/vector store. Exact eligible-vector search supplies
the baseline; introduce ANN only with measured need and Brain index isolation.
Do not widen grants, revision applicability or provider permissions to improve recall.

User-selected provider input, 2026-09-14: use `text-embedding-3-large`, paired with
`gpt-5.6-luna` for text operations through the memory epic's gateway. A real
[provider probe](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/mappings/openai-provider-preflight-2026-09-14.md) returned
the embedding model's default 3,072 dimensions. Preserve that selection when
specifying representation and indexing; do not silently substitute the small
model or reduce dimensions. Exact eligible-vector search remains the baseline.

Own search results, evidence/claim drill-through, historical filters, missing
coverage, abstention and links into memory review/graph exploration. Those
feature owners retain mutation and graph-computation semantics. MCP exposes
these same application operations rather than a second retrieval implementation.
Scope-change events must support immediate newly applicable recall when the
host bridge is integrated.

## Slice Map

The [automated memory improvement plan](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/research/automated-memory-improvement-plan-2026-10-07.md)
authorizes a canonical scoped project brief alongside query recall within existing
plugin limits. The [accepted contract](../../contracts/retrieval-scoped-project-brief.md)
and archived pack govern deterministic selection, authenticated continuation and
shared eligibility/invalidation. [Native proof and independent review](../../mappings/memory-source-support-staging-2026-10-07.md)
complete local acceptance; the installed host binary is not upgraded by this work.

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `retrieval-attributed-feedback` | shipped | contract-backed | archived | Explicit scoped utility signals, privacy/recovery and frozen zero-call comparison locally delivered; two bad promotions retained, production RRF stays fixed |
| `retrieval-scoped-project-brief` | shipped | contract-backed | archived | Current supported compact context and authenticated continuation |
| `retrieval-semantic-discovery-plans` | shipped | contract-backed | archived | Missing-input cohorts preserve eligibility/replay, pass native and normal-data equivalence, and are deployed locally; remaining graph failures stay with the graph/operations owners |
| `retrieval-exact-and-lexical` | shipped | contract-backed | archived | Delivered canonical exact/text candidates, correction-aware bounded context, immutable scope, temporal/manifest filters and browser/native consumers with database/runtime proof |
| `retrieval-semantic` | shipped | contract-backed | archived | Approved automatic batches, full-dimension exact semantic recall/RRF, scope/erasure/recovery, browser/native proof and measured actual-model corpus |
| `retrieval-graph-fusion` | shipped | contract-backed | archived | Native qualified graph candidates, shared fusion, bounded source coverage/depth, desktop/native proof and measured actual-model ablations |
| `retrieval-investigation-ui` | shipped | contract-backed | archived | Desktop result/disagreement/source views, frozen history, exact evidence and scoped native graph links; expiry/access/epoch clearing and actual retained test-Brain proof |
| `retrieval-ask-experience` | shipped | adr-backed, contract-backed | archived | Read-only temporary answers, exact eligible retrieval bundles, default-off answering policy and retained evidence search |
| `desktop-ask-primary` | shipped | adr-backed, contract-backed | archived | Question composer as the Ask default with search retained as an explicit mode and disabled-path fallback |
| `ask-chat-conversation` | shipped | contract-backed | archived | Ask page reworked into a chat-style conversation thread — user questions and model-grounded answers as turns with expandable evidence attachments and inline follow-ups — over the unchanged answer backend |
| `retrieval-set-based-read-path` | shipped | contract-backed | archived | One candidate pass per recall with index-eligible lexical windows, set-based support/rule/conflict/deadline gating and bounded chunk-span verification; ranking and every gate unchanged |

## Completion Criteria

- Actual recall excludes unauthorized and inapplicable material while returning
  positive-control evidence; raw chunks/caches cannot bypass corrections or Erase.
- Investigations expose useful disagreement with status; strict mode withholds
  unqualified claims. Historical mention is not mistaken for a current assertion.
- Answers carry source/revision provenance and expose unavailable evidence;
  retrieval never treats a runbook as permission to execute it.
- Semantic and graph additions are compared against the exact/lexical baseline
  under the same corpus, model and context budget; misses and false assertions
  are reported alongside latency and cost, without a superiority claim by default.
- Host-facing retrieval can react to scope changes and preserve concurrent task
  isolation. End-to-end host proof is completed with the MCP epic.

## Current status

The original exact/lexical/semantic/graph consumers and cited Ask are delivered. Source-window/history corrections and semantic discovery query repair are committed in f2673d7. The measured discovery query improved from about 18–20 s to 225–334 ms on the retained fixture; this is not an end-to-end quality score. The October 18 attributed-feedback slice is locally delivered: current native qualification/privacy cases and the frozen zero-call comparison pass. The toy comparison retains two bad promotions; it does not demonstrate real helpfulness or qualify the failed organization model. Production ranking activation, typed-time and ANN changes remain separate decisions.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Shipped packs](https://github.com/MikeK184/Recollect/tree/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive) were removed from the tree on 2026-10-19 and remain in Git history.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/hybrid-retrieval.md).
Dated test/deployment observations are not a current runtime health claim.
