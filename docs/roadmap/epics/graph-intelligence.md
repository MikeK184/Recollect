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
- [Concurrent preparation contract](../../contracts/graph-concurrent-preparation.md)
- [Automatic organization ADR](../../adr/0020-automatic-knowledge-organization.md)
- [Automatic organization contract](../../contracts/automatic-knowledge-organization.md)
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

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `graph-projection-and-traversal` | shipped | contract-backed | pack | Delivered scoped structural/knowledge generations, autonomous recovery and qualified native paths with actual Neo4j/PG and browser proof |
| `graph-cross-repository-views` | shipped | contract-backed | pack | Delivered exact combined inputs and parsed pinned Terraform links with qualified native paths, correction/recovery and browser proof |
| `graph-analytics` | shipped | contract-backed | pack | Delivered native GDS reports, complete qualification/invalidation, owned scratch recovery and reader concurrency with normal runtime/browser proof |
| `graph-exploration` | shipped | contract-backed | pack | Delivered bounded native reachability and accessible desktop Cytoscape exploration with actual scope, limits, retention, browser and test-Brain runtime proof |
| `graph-desktop-workspace` | shipped | adr-backed, contract-backed | pack | Canvas-first bounded graph workspace with accessible inspector, paths, explicit insights and exact scope |
| `graph-concurrent-preparation` | in-progress | contract-backed | pack | Off-lock preparation and checked publication implemented; mixed 10k and normal-data clone gates pass locally, with normal-installation update and final closeout outstanding |
| `graph-automatic-knowledge-mapping` | in-progress | adr-backed, contract-backed | pack | Worker, aliases, topics, overlays, navigation and privacy are locally implemented; actual extractor, grouping and canonical quality fail and no producer/recipe is selected |

The native qualification setup also corrects the existing selector's exact
automatic-support-excerpt lineage within `graph-automatic-knowledge-mapping`.
This isolated small fix follows accepted capture reconciliation and independent
support contracts: no new schema, interface, semantic match or arbitrary ancestor
authority. The active pack records the exact gates and focused proof boundary.

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

## Current status

The complete October 18 graph request is partially delivered locally. The
normal installation has not been updated and both product packs remain in
progress. Keep Neo4j/GDS for computation; the locally delivered grouped 3D
renderer belongs to the desktop epic.

- Implemented and locally validated: short graph/maintenance publication,
  producer-bound mapping, reversible human alias decisions, weighted topics,
  stable IDs and edited-topic overlays, qualified navigation, privacy/older-backup
  replay and browser integration. Mixed 10k reads pass 602/602 at p95 0.460 s;
  both normal-data clone series pass 40/40 below two seconds.
- Lease admission is fixed in the main checkout. Six actual-main native tests
  pass in 5.47 s; cancelled/reclaimed work creates no requests or publications,
  while healthy work publishes. This is local proof, not deployment or atomic
  cancellation of an already admitted network call.
- Actual semantic quality fails. The frozen 105-call current4 campaign yields
  mention precision/recall 51.67%/8.73% for facts and 40.63%/3.66% for
  reconciliation. Hybrid pair F1 is 26.36%. Canonical behavior passes 4/6;
  exact action/citation agreement is 3/6. Earlier failed recipes remain retained.
- Remaining: source-anchor/endpoint and historical/retirement repairs, a newly
  frozen extractor/grouping comparison, independent label usefulness, final
  integrated acceptance and normal-installation verification. No real producer
  or topic recipe is selected; passing plumbing does not qualify model output.

The [roadmap delivery summary and next actions](index.md#graph-delivery-status)
are the concise current entry point. The
[implementation evidence](../../mappings/graph-organization-implementation-2026-10-18.md)
retains exact receipts, source/build identities, all 92 observations/460 mode rows
and 21 unknown topic pairs per producer. Full acceptance stays with the
[active packs](../execution/active/README.md); version/commit are N/A/uncommitted.

## Delivery evidence

[Archived packs](../execution/archive/README.md) retain the delivered slices.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/graph-intelligence.md).
Dated test/deployment observations are not a current runtime health claim.
