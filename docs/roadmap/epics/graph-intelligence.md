# Repository Graphs, Knowledge Graphs and Analytics

Status: active

## Purpose

Make exact repository structure and evidence-linked knowledge relationships
queryable and explorable, including supported cross-repository paths, centrality,
clustering and bounded large-graph analysis in the first usable product.

## Governing Sources

- [Desktop contract](../../contracts/desktop-experience.md)
- [Desktop ADR](../../adr/0014-desktop-experience-and-answers.md)
- [ADR 0007](../../adr/0007-canonical-graph-projections.md)
- [Projection/traversal contract](../../contracts/graph-projection-and-traversal.md)
- [Exact combined graph contract](../../contracts/graph-cross-repository-views.md)
- [Bounded analytics contract](../../contracts/graph-analytics.md)
- [Exploration contract](../../contracts/graph-exploration.md)
- [Vision: revisions and evidence](../../foundation/vision.md#revisions-and-evidence)
- [Stack: graph computation and recovery](../../foundation/techstack.md#graph-computation-and-recovery)
- [Engineering principles: graph reproducibility](../../foundation/engineering-principles.md#preserve-repository-and-graph-reproducibility)
- [Engineering principles: correction](../../foundation/engineering-principles.md#make-correction-survive-every-regeneration-path)

## Dependencies and Boundaries

Use the selected Neo4j/GDS Community stack and Rust Query API adapter. Consume
immutable extraction artifacts, manifests, claims and current correction/erasure
eligibility. PostgreSQL/retained artifacts remain authoritative; this epic owns
derived graph generations, supported query paths and analytical results.

Own both graph forms while retaining the distinction between extracted structure,
model-inferred relationships and observed operational evidence. Cross-repository
links identify endpoint snapshots, linker version and evidence. A merged search
result set is not a validated path. Reuse repository snapshots across manifests
instead of copying whole graphs for each environment.

Own graph exploration, path/impact views and queued analysis/report UI with input
provenance and coverage. Retrieval consumes graph queries but owns ranking/fusion.
Memory lifecycle emits eligibility changes; this epic invalidates and recomputes
affected aggregates because filtering removed nodes from results cannot undo
their influence. Operations verifies complete graph reconstruction after restore.

No extra vector/text indexes in Neo4j, paid analytics license, new graph engine
or promise that all histories fit memory is introduced. Analytics is first-usable
scope, with queued work and explicit resource limits.

## Decisions Before Implementation

- Specify projection generation/publication, typed relationships, manifest
  selection, linking evidence and supported traversal/query interfaces.
- Specify the analytical operations/parameters, meaningful edge families,
  memory admission, cancellation and result provenance before their slice.
- Specify correction epochs, aggregate invalidation and historical-result
  eligibility; test the pinned Query API/server/GDS compatibility.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `graph-projection-and-traversal` | shipped | contract-backed | pack | Delivered scoped structural/knowledge generations, autonomous recovery and qualified native paths with actual Neo4j/PG and browser proof |
| `graph-cross-repository-views` | shipped | contract-backed | pack | Delivered exact combined inputs and parsed pinned Terraform links with qualified native paths, correction/recovery and browser proof |
| `graph-analytics` | shipped | contract-backed | pack | Delivered native GDS reports, complete qualification/invalidation, owned scratch recovery and reader concurrency with normal runtime/browser proof |
| `graph-exploration` | shipped | contract-backed | pack | Delivered bounded native reachability and accessible desktop Cytoscape exploration with actual scope, limits, retention, browser and SWEG runtime proof |
| `graph-desktop-workspace` | in-progress | adr-backed, contract-backed | pack | Canvas-first bounded graph workspace with accessible inspector, paths, explicit insights and exact scope |

## Slice Dependencies

| Slice ID | Predecessors |
| --- | --- |
| `graph-projection-and-traversal` | `evidence-repository-publication`, `memory-retention-and-erasure` |
| `graph-cross-repository-views` | `graph-projection-and-traversal` |
| `graph-analytics` | `graph-cross-repository-views` |
| `graph-exploration` | `graph-analytics` |
| `graph-desktop-workspace` | `platform-desktop-shell` |

## Completion Criteria

- Structural and knowledge graphs retain their evidence class and exact inputs;
  restart/rebuild preserves scope, corrections and selected manifests.
- Paths across cycles, hubs and repository boundaries are correct on fixtures;
  unresolved links and mixed/incomplete generations cannot appear as proven paths.
- Centrality/clustering results are checked against known fixtures and record
  algorithm/version, parameters, input eligibility and relevant seed/weights.
- Removing a contributing edge invalidates affected current aggregates, including
  changes during queued computation. Historical reports follow retention rules.
- Resource admission and concurrency limits protect capture/recall. UI progress,
  stale/error states and large-graph limitations match actual job state.

## Desktop implementation evidence, 2026-09-26

The [dated implementation mapping](../../mappings/desktop-experience-implementation-2026-09-26.md)
records implemented routes/assets and the verified final local image separately
from remaining current-code domain and integrated acceptance. The
[desktop guide](../../runbooks/desktop-experience.md) documents the current function
locations. Product/proof slices stay in progress until their required checks pass;
prior shipped domain records remain historical evidence rather than redesign proof.
