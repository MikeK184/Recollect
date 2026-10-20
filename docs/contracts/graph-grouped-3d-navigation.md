# Grouped 3D graph navigation

Status: accepted

## Source

The user's 2026-10-18 full implementation request and
[ADR 0021](../adr/0021-grouped-spatial-graph-navigation.md).
[Exploration](graph-exploration.md) and
[automatic organization](automatic-knowledge-organization.md) retain data authority.

## Contract

Lazy-load a React Three.js graph adapter on entering 3D. Prefer 3D when supported;
retain an explicit 2D/3D choice for the mounted Brain view. WebGL creation failure,
context loss or renderer failure clears affected 3D state and offers functional
2D fallback. Do not retry forever or leave a blank canvas. Do not persist source
labels, selections or coordinates across authorization/scope invalidation.

Both dimensions share exact opaque node/edge identities, directions, parallel
edges, self-loops, isolates, current selection, highlighted qualified paths and
canonical evidence inspection. Clone engine inputs because force engines may
mutate them. Use plain-text labels with safe DOM handling, never untrusted HTML.
Keep the existing 500-node/2,000-edge envelope and window coverage semantics.

Semantic overview uses qualified typed topic/entity records and visible coverage;
topics start as named collapsed cards/groups, with explicit expansion into bounded
neighborhoods. Overlapping memberships remain associations with one entity ID,
not duplicated canonical entities. Display-only type grouping must be labeled as
such, never presented as discovered semantic topics. Contribution edges remain
separate from canonical knowledge paths. Names/counts cannot come from a wider
scope or unchecked cached report.

Use established finite force-layout facilities. Preserve positions/camera for
unchanged topology, selection, hover, labels, filtering and resize. Reheat only
for topology changes or explicit rearrange. Bound settling and keep reduced-motion
transitions instantaneous. No automatic orbit or continuous link particles.
Pause hidden rendering and release GPU resources/listeners/workers on unmount.
Move expensive static layout to a worker before increasing display bounds.

Keep keyboard-operable finder/lists, selected-node/edge inspector and textual
paths. Scope/Brain changes, failed refresh, expiry, correction, role loss and
generation invalidation clear graph, selection and groups together. A late result
cannot replace the newer view. Colours come from the current Light and Ink design
tokens ([ADR 0025](../adr/0025-two-theme-desktop-redesign.md)), which replaced the
cream/ink/sage palette on 2026-10-19; both renderers must read correctly in both themes.

## Acceptance

- Actual browser 3D selection, directed/parallel edges, loops, isolated nodes,
  path highlights and exact evidence inspection work on qualified fixture data.
- Topic overview, expansion, overlapping membership and scope changes work
  without whole-Brain loading; inaccessible labels/counts never remain visible.
- Keyboard/list and 2D fallback support the same tasks; reduced motion, resize,
  hidden views, WebGL failure and cleanup do not break or continuously relayout.
- Record first usable render, frame times, interaction delay and memory on a
  named desktop/browser with 500 nodes/2,000 edges; qualify software-WebGL results.
  Target p95 interaction below 100 ms after settling and no unbounded memory
  growth across repeated mount/switch/expand cycles. Record failures as outcomes.
- Independent rendered visual review at laptop and wide desktop widths, focused
  browser/design/type checks and `./scripts/validate.sh` pass.

## Explicit Deferrals

VR/AR, all-history full-Brain rendering and a custom graph/layout engine are not
selected. These do not defer grouped 3D navigation, 2D fallback or its proof.
