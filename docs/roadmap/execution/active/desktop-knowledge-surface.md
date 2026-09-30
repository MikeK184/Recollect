# Tiered knowledge surface and shared lineage inspector

Status: planned
Owning epic: `docs/roadmap/epics/product-platform.md`
Work type: product

## Summary

- Goal: Collapse the four peer knowledge pages into one surface with a single canonical lineage inspector, move graph controls into the canvas chrome, apply the shared density treatment to Memory, Sources and Repositories lists, and add the standing assurance band, while every contracted route stays deep-linkable.
- Non-goals: No new endpoints, aggregation, persistence, role or authorization layer. No change to mutation, grant, retention, policy, ranking or graph-computation semantics. No dark mode, mobile work or framework change.
- Delivery shape: React and TypeScript presentation change in the platform shell plus the shared component set, coordinated with the memory, evidence and graph feature owners; focused browser and route proof; reconciled indexes.

## Governing Sources

- [ADR 0017: One knowledge surface](../../../adr/0017-desktop-knowledge-and-ask-experience.md)
- [Tiered desktop surface contract](../../../contracts/desktop-knowledge-surface.md)
- [Contextual desktop experience](../../../contracts/desktop-experience.md)
- [Claims and temporal history](../../../contracts/memory-claims-and-time.md)
- [Human review and durable corrections](../../../contracts/memory-review-and-corrections.md)
- [Collections and retained evidence](../../../contracts/evidence-collections.md)
- [Bounded graph exploration](../../../contracts/graph-exploration.md)
- [Repository snapshots and revision manifests](../../../contracts/evidence-repository-publication.md)
- [Vision: one memory model, complementary methods](../../../foundation/vision.md#one-memory-model-complementary-methods)
- [Owning epic](../../epics/product-platform.md)

## Scope

- In scope: Four-tier sidebar with visible group labels; the Knowledge surface hosting Memory, Sources, Graph and Repositories as validated views; one shared lineage inspector used by all four views; graph kind, entity, scope, layout, zoom, fit, focus, filter, insights and status controls relocated into the canvas chrome; collapsed-but-visible coverage and eligibility indicator; density treatment for the three lists including composite status mark with text alternative, normalized origin display and UUID relocation; devices and Brains active-first ordering with historical filter; the standing assurance band composing only existing authorized feeds.
- Out of scope: Any Ask behavior change, which is `desktop-ask-primary`. Any agent or connection placement change, which is `desktop-connection-authority`. Activity feed restructuring beyond the band drill-down, which is `desktop-assurance-pulse`. New server reads, saved conversations, unified feed endpoints, terminology renames beyond the where-it-runs and tool-group explanations, and any permission or policy change.
- Blockers: None. [ADR 0017](../../../adr/0017-desktop-knowledge-and-ask-experience.md) and its contract are accepted with the user's 2026-09-29 decision recorded.

## Surface and Interface Changes

- Interfaces: No API change. Route and query-state change only: `/memory`, `/sources`, `/graph` and `/repositories` remain valid and select the corresponding Knowledge view, with the view value validated and unknown values falling back to the documented default.
- Storage: N/A: presentation only, with no schema, migration or persisted state change beyond existing validated URL state.
- Ownership: Platform shell owns tiers, route resolution, the shared inspector region and the density primitives. Memory lifecycle owns claim and status semantics, Evidence owns source, version, collection and snapshot semantics, Graph intelligence owns traversal, bounds and analytics semantics. No feature module may fork a private inspector or status-mark implementation.

## Data and Authority

- Inputs: Existing authorized reads for claims, sources, versions, collections, areas, environments, snapshots, manifests, graph view and explore responses, capture and processing feeds, and per-Brain automation status. Selection identity from the current view.
- Authority: Canonical domain reads remain the only source of truth. Derived representations, summaries, embeddings and graph edges are displayed as derivations and never as independent support. Mutations continue through each owner's canonical handlers.
- Blind spots: Aggregate counts the band displays exist only where a verified response field supplies them; anything unsupported is omitted rather than estimated client-side. Offline companion coverage and backup copy lifetime remain outside what the band can assert. Whether the current graph default is bounded for a large Brain is measured, not assumed.

## States and Edge Cases

- Loading: Each view shows its own skeleton; the inspector shows a pending region rather than the previous selection's content; the graph canvas draws progressively instead of a blocking spinner.
- Empty: Distinct handling for a genuinely empty Brain, a filtered-empty result, and a search with no matches.
- Error: Per-view failed read keeps the surface usable and offers refetch; a failed inspector read clears protected payload while preserving safe user-entered form values.
- Blocked: Archived Brain keeps its existing read-mostly behavior and shows the archive state in the surface rather than hiding it.
- No-access: A reader sees permitted content only; admin-only audit and profile outputs stay absent; losing access mid-session clears affected cached payloads.
- Duplicate or replay: The same record reached from different views resolves to one identity and one inspector instance; returning to a prior view restores the selection without duplicating requests.
- Stale data: Epoch change, advertised expiry and Brain switching cancel superseded requests; late responses cannot populate the new Brain or selection.
- Reconciliation divergence: A projection that lags canonical state is labeled as lagging rather than presented as current; a source whose derived records were erased shows the erasure, not an empty success.

## Integrations and Runtime Inputs

- Providers: No model, embedding or external provider call is introduced. Cytoscape remains the graph renderer, restyled and reorganized rather than replaced.
- Environment: No new variable. Existing same-origin API base and static asset serving apply.
- Secrets: No secret is displayed in any list, inspector, band or fixture. Credential references stay references.
- Failure handling: Refetch and retry follow existing TanStack Query behavior per route consumer. No navigation, view switch or band refresh may trigger a provider request, job, rebuild or tool call.

## Tests and Acceptance

- Automated: Route, deep-link, Back and forward, unknown-view fallback and Brain-switch browser cases; selection-preservation and inspector-single-instance assertions; graph request-count capture proving one bounded default read; canvas-versus-chrome geometry check at 1440 by 900; keyboard, focus-return and non-color status checks; existing memory, evidence, graph, retention and access regressions from their new locations; web typecheck; `./scripts/validate.sh`; `git diff --check`.
- Manual: Visual review at 1280, 1440 and 1920 of each Knowledge view, the lineage thread in both directions, the collapsed coverage indicator and the band in ordinary and escalated states.
- Acceptance: All four tiers render and every pre-existing route deep-links into the correct view and selection; selecting a memory reveals its supporting evidence and bounded neighbourhood, and selecting a source reveals what it supports, through one inspector implementation verified by source inspection; the canvas exceeds its combined chrome height at 1440 by 900 with the coverage limitation visible while collapsed; the three status dimensions remain separately readable as text after compression; the band matches authorized feed counts and states no-action-required in the ordinary case; request volume is measured against the current comparable fixture with the result recorded rather than claimed lower by layout.

## Closeout

- Planned: Tiered sidebar, Knowledge surface, shared lineage inspector, graph chrome reorganization, list density treatment, historical filters and the assurance band.
- Shipped: Not yet implemented. This pack is the specification; delivery evidence is recorded here only after the checks above run.
- Not shipped: Ask default, wiring split and Activity restructuring are named successor and sibling slices rather than absorbed here.
- New blockers: None recorded at authoring time.
- Docs updated: Owning epic slice map and dependencies, active execution index, desktop contract navigation paragraph, and the desktop runbook function map.
- Validation: Not yet executed for this slice.
- Version: N/A: no release policy exists.
- Commit: Uncommitted.