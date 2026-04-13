# Explore graph relationships on desktop

## Purpose and prerequisites

Start the local product with `./scripts/dev.sh`, then open
`http://127.0.0.1:8787` and a Brain. Use the owner credentials in the ignored
`.env`, or an account with Brain read access. PostgreSQL, the worker and Neo4j
must be healthy; exact repository inputs must already be materialized and
projected. This workflow makes no model request or customer checkout read.

## Procedure

1. Expand **Evidence graphs**. Select knowledge, repository structure or combined
   repositories, then the desired repositories, areas, environment, exact
   snapshot/manifest, evidence mode and relation families. **Load graph view**
   loads eligible entities and the interactive overview.
2. Inspect generation/readiness, exact inputs and coverage limitations. Blue
   nodes represent repository evidence, purple claims and green retained sources.
   Arrow direction is the recorded relationship direction. Position is a layout
   choice; it does not indicate truth, importance or deployed impact.
3. Select an entity/arrow on the canvas, or use **Inspect graph entity** and
   **Inspect graph relationship** with the keyboard. Details retain canonical
   identity, evidence class, source/revision/commit and relationship provenance.
   **Inspect selected evidence** or **Inspect relationship evidence** opens the
   existing canonical inspection dialog.
4. Select **Explore connections**, or choose a center from the current entity
   page and **Explore from center**. Choose outgoing, incoming or both directions
   and one to eight hops. The response includes all reached eligible entities,
   their induced relationships and one shortest witness per entity. Incoming
   traversal does not reverse recorded arrow directions.
5. Set path start/end and **Find shortest eligible path**. The ordered textual
   path remains available; **Show shortest path on canvas** displays its exact
   witness relationships. Green highlights mark this path, and orange marks the
   selected element. Equal-length native shortest paths can choose different ties.
6. Use **Graph layout**, **Fit graph**, zoom and **Focus selection** to inspect
   the display. These are renderer operations, not evidence mutations. Queued
   [analytics and reports](graph-analytics.md) remain in the same graph panel.

## Limits and recovery

- A canvas admits at most 500 entities and 2,000 relationships. Oversized results
  are refused as a whole. Select a narrower repository/area/relation scope, or
  choose an eligible center and a smaller hop bound. A smaller view still uses
  the complete canonical qualification budget of 5,000 candidates/20,000 edges.
- No other entity reached means none was found in that exact scope, direction
  and hop bound. Missing/damaged projection and service failures are errors;
  they never become an empty or disconnected graph.
- Scope changes, canonical invalidation/epoch changes, retention expiry and
  failed refreshes clear affected results. Load again after processing recovers.
  No earlier dataset is silently merged into a new selection.
- Combined views require every manifest input to remain eligible even when a
  narrower selection hides some repositories. Resolve unavailable inputs before
  retrying. Follow [combined graph recovery](graph-cross-repository-views.md).
- Mobile views and mobile acceptance are deferred by the user's 2026-09-22
  instruction. The current workflow and validation target desktop use.

## Verification

```sh
./scripts/test-ui.sh tests/graph.spec.ts
./scripts/test-platform.sh
./scripts/validate.sh
```

The [dated evidence](../mappings/graph-exploration-interfaces-2026-09-22.md)
distinguishes actual canonical/native API proof, real browser interactions, and
the separate synthetic 500-node/2,000-edge renderer stress case. Renderer
fixtures never establish canonical eligibility or graph completeness.
