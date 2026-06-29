# Bounded graph exploration

Status: accepted

## Source

The user's authorized full-product implementation includes browser graph,
path and impact exploration. This contract refines [ADR 0007](../adr/0007-canonical-graph-projections.md),
[canonical traversal](graph-projection-and-traversal.md), [combined inputs](graph-cross-repository-views.md),
and [analytics](graph-analytics.md). The [interface evidence](../mappings/graph-exploration-interfaces-2026-09-22.md)
records the native query probe and renderer choice. These are routine choices
within the accepted independent Rust and Neo4j baseline.

## Contract

`POST /api/brains/{brain}/graph/explore` is a read-only operation accepting
`GraphExploreRequest { scope: GraphSelection, center: string | null,
direction: outgoing | incoming | both, max_hops: 1..8 }`. A null center returns
the complete eligible overview. A center returns that eligible entity and all
entities reachable within the specified bound and direction. Impact means
recorded reachability; neither a static dependency nor a citation proves runtime
impact, causation, acceptance or truth. The response includes canonical nodes,
directed edges, complete selected-input provenance in `view`, the request
bounds, the earliest input retention deadline, and a native shortest witness
path for each reached node (the center has a zero-edge witness). Witnesses are
absent for an overview. Equal-length shortest paths need not have stable ties.

Use the existing canonical qualification before querying Neo4j: Brain access,
paired-device operation scope, repositories, paths/areas, collections,
environment and exact manifest, trust mode, fact time, revisions, corrections,
conflicts, erasure and retention. All combined inputs remain required even
when a narrower view hides their entities. Complete canonical selection is
bounded by the existing 5,000 candidates, 20,000 edges and descriptor byte
budget. The display envelope is 500 nodes and 2,000 edges. Reject an oversized
overview or reached subgraph with `graph_display_too_large` (422), inviting a
smaller scope or hop bound; do not display a prefix as a complete graph. Isolated
nodes, parallel edge identities, self loops and recorded directions survive.
Centered views include all qualified edges between their reached nodes; a
visible edge is not necessarily a shortest-path witness.

Neo4j `SHORTEST 1` computes bounded reachability against exact allowed node,
edge and generation sets inside the path selector. No client or Rust BFS,
centrality, clustering, or force-layout implementation is introduced. Verify
the complete selected physical projection before interpreting no reachability
as absence, then validate returned witness bounds, unique endpoints, edge
identity, adjacency and direction against canonical records. Missing/damaged
projections and malformed native responses fail closed. Reads share the
existing two-read capacity, 15-second request limit and bounded database
statements; final authorization and retention checks precede commit. There is
no new persistent state, model operation or native scratch projection.

Use Cytoscape.js through a small React lifecycle adapter for rendering,
selection, pan/zoom and built-in layouts. Lazy-load the renderer; use plain
text labels, preserve canonical IDs, dispose of layout/listeners/renderer on
replacement or unmount, resize for the container and respect reduced motion.
Node and edge selection must also work through keyboard-accessible HTML lists
and controls. Show labels, evidence class, exact source/revision/commit,
relationship meaning, counts, generation/readiness and coverage limitations.
Offer evidence inspection, center/direction/hop controls, native shortest-path
highlighting, and ordered textual paths. Keep the existing analytical report
panel and its provenance/status semantics accessible alongside exploration.

Scope changes, Brain changes, invalidation, epoch changes, expired results and
failed refreshes clear affected displayed results and selections. Cancel
superseded requests; late responses cannot replace a newer selection. A failed
refresh cannot leave an old graph appearing current. Existing evidence dialogs
remain canonical reads. Desktop UI controls must handle long repository labels
and provide an understandable empty, no-path, loading,
unavailable, capacity or no-access state.

## Acceptance

Exercise actual PostgreSQL and Neo4j with cycles, parallel edges, self loops,
isolates, directional and hop-bounded reachability, an excluded shorter hub
and an eligible control. Verify overview and reachability display refusal,
canonical projection corruption, paired/reader access and combined-input
qualification. Reuse established correction, scope, authorization and retention
fixtures where they cover the common selector; add focused proof for the new
endpoint and its final gates. Browser proof covers the real renderer, node and
edge inspection, path highlighting, keyboard controls, clearing stale results,
errors and desktop layout. Verify a retained local SWEG graph without new model
calls or repository reads. Run the required repository validator.

## Explicit Deferrals

Unbounded graph visualization, graph editing, arbitrary Cypher, exhaustive
enumeration of equally short paths, custom graph algorithms and asserted
runtime impact are outside this contract. Graph-fused retrieval and its
investigation UI belong to their successor slices. Mobile views and mobile
acceptance are deferred by the user's explicit 2026-09-22 instruction; continue
with desktop product behavior without spending further work on mobile.
