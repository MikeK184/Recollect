# Explore graph relationships on desktop

## Purpose and prerequisites

Start the local product with `./scripts/dev.sh`, then open
`http://127.0.0.1:8787` and a Brain. Use the owner credentials in the ignored
`.env`, or an account with Brain read access. PostgreSQL, the worker and Neo4j
must be healthy; exact repository inputs must already be materialized and
projected. This workflow makes no model request or customer checkout read.

## Procedure

1. Open **Graph**. A bounded current knowledge overview loads when no exact
   selection is supplied. Use Knowledge/Repository/Combined and **Filters** for
   repositories, areas, environment, exact snapshot/manifest, mode and relations.
   **Apply graph filters** records that exact selection; missing inputs are not guessed.
2. Open **Graph status** for readiness/generation and coverage. Blue nodes represent
   repository evidence, amber claims and sage retained sources. Arrow direction is
   the recorded relationship. Layout does not imply truth or deployed impact.
3. Select an entity/arrow on the canvas, use **Inspect graph entity** / **Inspect
   graph relationship**, or select **List** for keyboard-readable entities. The
   inspector preserves canonical identity, revision and provenance. **Inspect
   selected evidence** or **Inspect relationship evidence** opens exact support.
4. Use **Explore from an entity**, choose a center and **Explore from center**.
   Direction and one-to-eight hops bound the read. The result contains reached
   eligible entities, their induced relationships and shortest witnesses. Incoming
   traversal does not reverse recorded arrow directions.
5. Use **Find path** with contextual **Use as start** / **Use as end**, then
   **Find shortest eligible path**. The ordered textual path remains available;
   **Show shortest path on canvas** displays its exact witness relationships.
   Dark sage highlights the path and ink highlights selection. Equal-length paths
   may choose different ties.
6. **Graph layout**, **Fit graph**, zoom and **Focus selection** only affect the
   renderer. **Insights** opens explicit [analytics](graph-analytics.md); **Graph
   status** contains authorized rebuild. Neither action runs just by opening Graph.

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
