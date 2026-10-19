# Product status and implementation roadmap

Reviewed: 2026-10-18 against repository checkpoint `f2673d7` and the current
source. On 2026-10-18 implementation of the complete graph, automatic
organization, grouped 3D and bounded feedback-ranking work resumed; the
documentation cleanup of the same date is preserved. Runtime and benchmark
numbers below are dated observations until replacement proof is recorded.

## Epic Overview

| Epic | Status | Current focus |
| --- | --- | --- |
| [repository-governance.md](repository-governance.md) | complete | Current documentation, governance and public website delivered |
| [developer-tooling.md](developer-tooling.md) | complete | Repository-local Rust Analyzer MCP, Graft and documentation skills |
| [product-platform.md](product-platform.md) | complete | Accounts, Brain access, durable work, installation identity and canonical Brain deletion |
| [evidence-and-workspaces.md](evidence-and-workspaces.md) | complete | Versioned sources, collections, workspace scope, capture and repository publication |
| [memory-lifecycle.md](memory-lifecycle.md) | active | Existing automatic learning/digests/correction delivered; derived mapping privacy remains an active dependency |
| [hybrid-retrieval.md](hybrid-retrieval.md) | complete | Delivered recall baseline, set-based read path (lexical index windows, one candidate pass per transaction, set gating) and locally validated attributed-feedback offline comparison; production ranking fixed |
| [graph-intelligence.md](graph-intelligence.md) | active | Locally passing graph reliability and lease repair; failed semantic qualification and final integrated acceptance remain open |
| [mcp-coordination.md](mcp-coordination.md) | active | Shared/native skills and cross-machine coordination locally delivered; direct-token and normal installed-host acceptance remain separate |
| [operational-readiness.md](operational-readiness.md) | active | Normal graph reliability and a comparable memory-quality rerun |
| [desktop-visual-experience.md](desktop-visual-experience.md) | complete | Cream/ink/sage desktop, locally validated grouped 3D, source navigation and exact generation invalidation |

## Implemented baseline

Recollect owns its Rust server and host plugin, PostgreSQL authority/pgvector,
Neo4j/GDS projections and React desktop. Delivered behavior includes versioned
source evidence, manual collections/areas/environments, automatic capture and
learning, support verification, settled-session digests, scoped project briefs,
exact-family cross-session reconciliation, cited Ask/recall, repository graphs,
optional review/correction, erasure/recovery and governed MCP/private execution.
Automatic learning already exists. Automatic semantic organization is the
unfinished addition below.

Checkpoint `f2673d7` contains the earlier provider, source-window/history, graph,
audit and backup-backed update repairs. The October 18 work remains local and
uncommitted. Its implementation and test results below do not establish an
updated normal installation. The latest retained installation proof is migration
047 from October 9; see the
[dated checkpoint](../../mappings/automatic-knowledge-mapping-progress-2026-10-09.md).

## Graph delivery status

These are evidence states, not new lifecycle statuses. The full graph request
remains unfinished; the two graph packs stay in progress.

| Capability | What is done | What remains |
| --- | --- | --- |
| Graph reliability | Off-lock preparation and short checked publication are implemented. Local mixed 10k proof passes 602/602 reads, zero errors, p95 0.460 s; both normal-data clone series pass 40/40 below two seconds. | Final integrated closeout and a verified normal-installation update; the newer tests did not update that installation. |
| Semantic worker lease admission | Fix is applied to the main checkout. Six native cancellation/reclaim/healthy regressions pass in 5.47 s; stale work creates zero requests or publications. | Rollout is not performed. Cancellation API, Linux runtime and cancellation after network admission were not established by these fixtures. |
| Mapping, entities and aliases | Durable producer-bound worker, exact source spans, scoped IDs, retained stages and reversible human alias decisions are implemented and locally tested. | Real producer qualification fails; no qualified production extractor is selected. |
| Topics, overrides and navigation | Native GDS topics, overlapping memberships, stable IDs, manual overlays for edited topics, qualified APIs and browser navigation are implemented and locally tested. | The automatic grouping recipe fails quality/coverage; current blind label usefulness and complete organization acceptance remain open. |
| Correction, privacy and recovery | Focused correction/erase/scope, older-backup replay and manual-overlay tests pass. | Final integrated acceptance over the complete delivered feature set; focused proof is not universal recovery assurance. |
| Grouped 3D | [Renderer slice](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/desktop-grouped-3d-graph.md) is locally delivered with 2D fallback, selection/evidence, generation clearing, keyboard, visual and bounded hardware proof. | Normal-installation rollout; renderer completion does not qualify automatic topic semantics or universal GPU performance. |
| Feedback for ranking | [Bounded feedback slice](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/retrieval-attributed-feedback.md) is locally delivered: attributed votes, privacy controls and a frozen zero-call experiment. | Real usefulness is unproved. Online reinforcement training and production ranking activation are not implemented by this slice; production RRF remains fixed. |

The latest frozen real qualification completed all 105 calls within its separate
USD 2.20 grant, with no retries. It preserves 92 source observations, 460 mode
rows, three underlying empty sources and 21 unknown topic pairs per producer.

| Latest quality gate | Result | State |
| --- | --- | --- |
| Source-facts-4 mentions | Precision 51.67%; recall 8.73% | Failed 95% / 80% target |
| Source-reconciliation-4 mentions | Precision 40.63%; recall 3.66% | Failed 95% / 80% target |
| Native hybrid topics | Pair F1 26.36%; assignment 27/46 or 26/46 by producer | Failed qualification; vector and metadata baselines retained |
| Canonical behavior | 4/6 controls pass; strict action/citation agreement 3/6 | Historical retirement, required retirement and one exact citation remain wrong |

No producer or recipe is activated. Independent scalar, budget and source
reviews, exact cohort denominators, prior failed recipes and cleanup limits are
in the [implementation ledger](../../mappings/graph-organization-implementation-2026-10-18.md#completed-frozen-46-source-qualification).

### Next graph work

1. Repair quote/occurrence selection and endpoint evidence, preserving exact
   span/scope gates. Freeze a new candidate before an untouched evaluation;
   tuning on these failed held-out cases requires a new held-out boundary.
2. Correct historical/negation reconciliation and the exact citation mismatch;
   pass canonical behavior and support/publication checks separately from NER.
3. Requalify grouping against all five frozen baselines, assignment coverage and
   the independent label rubric. Preserve empty, failed and unknown outcomes.
4. Complete the organization and concurrency packs' integrated acceptance, then
   verify any normal-installation update under its owned update/recovery rules.

These are remaining implementation and acceptance tasks, not authorization for
another paid campaign. Completed cost grants do not cover new calls. No commit,
push, release or deployment has been performed for this work.

## Other locally delivered work

[Attributed feedback](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/retrieval-attributed-feedback.md) is locally
delivered with exact scoped votes, current qualification/privacy and a frozen
zero-call 20-query comparison. Two deliberately mistaken votes cause bad
promotions. Recorded-eligibility toy gains do not establish real helpfulness;
production RRF remains fixed and organization quality remains open.

[Brain-managed skills and instructions](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-managed-skills.md)
are locally delivered with administrator publication, environment applicability,
advisory MCP profile links, progressive fetch, plugin metadata/local copies and
Agents management. Focused proof uses disposable databases, real MCP and mocked
browser routes; it does not deploy the feature or update installed host packages.
See the [dated evidence](../../mappings/managed-skills-host-consumption-2026-10-18.md).

[Cross-machine agent coordination](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-agent-coordination.md)
is locally delivered: authenticated session context, explainable goal/planned-file
overlap, durable peer discussion, SSE/pull, MCP, native hooks/CLI and a narrow A2A
1.0 message adapter. The [macOS/Ubuntu proof](../../mappings/agent-coordination-proof-2026-10-18.md)
passed 15 checks with zero model requests. This does not establish production
deployment or installed-host model consumption.

[Native managed skills](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-native-skill-reload.md) now have
owned workspace registration, automatic synchronization, shared cross-host scope
leases and tested temporary/retained cleanup. Actual isolated Codex 0.162.1,
Claude Code 2.1.302 and OpenCode 2.0.22 processes proved native v1/v2 body/reference
activation and removal behavior. The [proof](../../mappings/native-skill-reload-host-proof-2026-10-18.md)
uses synthetic providers and does not upgrade a user's current host or deployment.

## Remaining authorized work

| Priority | Deliverable | Current gap and completion gate | Owner |
| --- | --- | --- | --- |
| 1 | Reliable graph reads during background work | Local mixed 10k and normal-data clone performance gates pass. Final integrated closeout and a verified normal-installation update remain; see the delivery summary above. | [Concurrency pack](../execution/active/graph-concurrent-preparation.md), [operations pack](../execution/active/operations-memory-quality-first-batch.md) |
| 1 | Privacy closure for derived excerpts | Focused ancestry, suppression, manual-overlay and actual older-backup replay controls pass. Complete combined feature acceptance; do not treat isolated controls as full product closeout. | [Organization pack](../execution/active/graph-automatic-knowledge-mapping.md), memory lifecycle |
| 2 | Automatic source-linked entities and reversible aliases | Worker and alias commands are locally implemented. Current4 extraction and canonical semantics fail; repair anchors/endpoints and historical/retirement behavior before new qualification or production profile selection. | [Organization pack](../execution/active/graph-automatic-knowledge-mapping.md) |
| 2 | Automatic overlapping topics and useful graph entry | Grouping, overlays, APIs and 3D plumbing exist. The frozen 46-source hybrid fails quality/coverage; current blind label usefulness is unmeasured. Requalify the recipe and close integrated navigation acceptance. | [Organization pack](../execution/active/graph-automatic-knowledge-mapping.md) |
| 2 | Incremental organization and recovery | Native invalidation, source edits, erasure, alias/override changes and older-backup replay have focused proof. Retain those results and complete final combined idempotency/privacy/recovery acceptance. | [Organization contract](../../contracts/automatic-knowledge-organization.md), [organization pack](../execution/active/graph-automatic-knowledge-mapping.md) |
| 3 | Demonstrated memory-quality improvement | Reuse frozen questions and separate retrieval, answering and full-learning effects. The latest completed LongMemEval baseline is 20/50 (40%); subsequent repairs have no comparable scored rerun. | [Operations pack](../execution/active/operations-memory-quality-first-batch.md), [benchmark report](../../mappings/openrouter-benchmark-proof-2026-10-08.md) |
| 3 | Finish direct-token and installed-host acceptance | Browser config and HTTP proof exist; fresh OpenCode/Claude launch and the rebuilt plugin's native acceptance remain separate. The recorded OpenCode service timeout needs a fresh verification. | [Direct-token pack](../execution/active/mcp-copy-ready-agent-config.md), [plugin evidence](../../mappings/plugin-workflow-repair-2026-10-07.md) |

Priorities order the remaining work; they do not turn a partial implementation
into a shipped slice. Current local reliability receipts replace the older
capture-only and failing normal-read results without removing their history.
They do not establish deployment or real extractor/topic quality. Exact methods
and limits are in the [implementation ledger](../../mappings/graph-organization-implementation-2026-10-18.md).
Unfinished product packs remain in [active](../execution/active/README.md).
The [grouped 3D renderer](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/desktop-grouped-3d-graph.md) and
[bounded feedback experiment](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/retrieval-attributed-feedback.md)
are locally delivered; neither closeout qualifies the failed semantic producer
or establishes deployment.

## Proposed follow-ups requiring a scoped decision

The [remaining improvement proposal](../../research/memory-product-improvement-plan-2026-10-08.md)
retains typed event-time handling, supported semantic relationship views, scoped
ANN/ranking experiments, background fairness/observability and cost/reliability
improvements. They need specific contracts and evaluation before implementation.
Graphify must demonstrate additional coverage over Enola on identical inputs;
Graphiti and a local extractor service are optional adapter decisions. No graph
engine replacement has been selected.

Small-screen polish, general desktop SSE, learned retrieval gates/decay, advanced reranking,
automatic executable skills, OAuth/marketplace publishing and a release policy
remain deferred. Do not present those as missing implementations of accepted
current behavior. Use current contracts for their exact boundaries.

## Delivery records and evidence

[Archived packs](https://github.com/MikeK184/Recollect/tree/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive) retain concrete delivered scope
and acceptance. [Mappings](../../mappings/README.md) retain current baseline and
open-issue evidence. Superseded studies and concepts are available at the fixed
[pre-cleanup checkpoint](https://github.com/MikeK184/Recollect/tree/f2673d71c4b63b536d41b3143eb39c207955b201/docs).
Dates, model scores and successful host connections describe their recorded runs;
no current connectivity or benchmark claim follows from documentation cleanup.
