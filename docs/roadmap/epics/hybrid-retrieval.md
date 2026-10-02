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
[provider probe](../../mappings/openai-provider-preflight-2026-09-14.md) returned
the embedding model's default 3,072 dimensions. Preserve that selection when
specifying representation and indexing; do not silently substitute the small
model or reduce dimensions. Exact eligible-vector search remains the baseline.

Own search results, evidence/claim drill-through, historical filters, missing
coverage, abstention and links into memory review/graph exploration. Those
feature owners retain mutation and graph-computation semantics. MCP exposes
these same application operations rather than a second retrieval implementation.
Scope-change events must support immediate newly applicable recall when the
host bridge is integrated.

## Decisions Before Implementation

- Specify public retrieval inputs/results, eligibility modes, temporal/manifest
  selection, cache invalidation and context-budget behavior.
- Specify embedding identity/dimensions and reindexing; select/test ranking and
  fusion on representative questions before introducing approximate indexes.
- Define quality baselines and acceptable failure/latency behavior for each slice;
  exact API names and numerical tuning belong in its execution pack.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `retrieval-exact-and-lexical` | shipped | contract-backed | pack | Delivered canonical exact/text candidates, correction-aware bounded context, immutable scope, temporal/manifest filters and browser/native consumers with database/runtime proof |
| `retrieval-semantic` | shipped | contract-backed | pack | Approved automatic batches, full-dimension exact semantic recall/RRF, scope/erasure/recovery, browser/native proof and measured actual-model corpus |
| `retrieval-graph-fusion` | shipped | contract-backed | pack | Native qualified graph candidates, shared fusion, bounded source coverage/depth, desktop/native proof and measured actual-model ablations |
| `retrieval-investigation-ui` | shipped | contract-backed | pack | Desktop result/disagreement/source views, frozen history, exact evidence and scoped native graph links; expiry/access/epoch clearing and actual retained SWEG proof |
| `retrieval-ask-experience` | shipped | adr-backed, contract-backed | pack | Read-only temporary answers, exact eligible retrieval bundles, default-off answering policy and retained evidence search |
| `desktop-ask-primary` | shipped | adr-backed, contract-backed | pack | Question composer as the Ask default with search retained as an explicit mode and disabled-path fallback |
| `ask-chat-conversation` | shipped | contract-backed | pack | Ask page reworked into a chat-style conversation thread — user questions and model-grounded answers as turns with expandable evidence attachments and inline follow-ups — over the unchanged answer backend |

## Slice Dependencies

| Slice ID | Predecessors |
| --- | --- |
| `retrieval-exact-and-lexical` | `memory-retention-and-erasure` |
| `retrieval-semantic` | `retrieval-exact-and-lexical`, `memory-provider-policy-and-learning` |
| `retrieval-graph-fusion` | `retrieval-semantic`, `graph-cross-repository-views` |
| `retrieval-investigation-ui` | `retrieval-graph-fusion`, `memory-procedures-and-handovers` |
| `retrieval-ask-experience` | `desktop-experience-contracts`, `platform-desktop-shell`, `evidence-desktop-workflows`, `memory-desktop-workflows` |
| `desktop-ask-primary` | `retrieval-ask-experience` |
| `ask-chat-conversation` | `desktop-ask-primary` |

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

## Desktop implementation evidence, 2026-09-26

The [dated implementation mapping](../../mappings/desktop-experience-implementation-2026-09-26.md)
records implemented routes/assets and the verified final local image separately
from remaining current-code domain and integrated acceptance. The
[desktop guide](../../runbooks/desktop-experience.md) documents the current function
locations. Product/proof slices stay in progress until their required checks pass;
prior shipped domain records remain historical evidence rather than redesign proof.

## 2026-09-29 Ask default

[ADR 0017](../../adr/0017-desktop-knowledge-and-ask-experience.md) makes the
question composer the default Ask view, amending the search-first landing recorded
in the shipped [successor cleanup pack](../execution/archive/mcp-successor-cleanup.md).
The `desktop-ask-primary` slice owns the change and follows `retrieval-ask-experience`,
because the accepted plan keeps the default-Ask cutover behind real answer
acceptance. Answer semantics, citation enforcement and conversation temporariness
are unchanged.

The user resumed Ask-primary and its remaining answer acceptance on 2026-10-01,
together with the final local deployment and desktop acceptance closeout.

## Final desktop and tooling acceptance — 2026-10-01

The owning desktop/tooling slices are shipped with [current deployed and host evidence](../../mappings/desktop-final-acceptance-2026-10-01.md). Earlier pending checks above describe the September 26 snapshot; their remaining acceptance is now complete. No release, commit or push was performed.

## Ask as a chat conversation — 2026-10-03

The user's 2026-10-03 direction asked for Ask to read like a chat with the
Brain rather than a form. The model-grounded answer backend already synthesizes
from retrieved evidence; the gap was presentation. Shipped 2026-10-03: the Ask
landing centers the prompt in the conversation area with the composer beneath
it (single-line start, growing with input), and the existing thread — question
bubbles, cited answer turns, inline follow-ups, per-turn failure states — is
retained. The [archived pack](../execution/archive/ask-chat-conversation.md)
and [evidence](../../mappings/agents-ask-chrome-refinements-2026-10-03.md)
record the live SWEG Brain question-and-answer check; the contract amendment
lands in the [desktop experience](../../contracts/desktop-experience.md).
