# Graph exploration interfaces and reuse evidence

Observed: 2026-09-22
Confidence: verified

## Sources and verification

- Context7 `/cytoscape/cytoscape.js` supplied initialization, style, layout and
  lifecycle guidance; its branch documentation was checked against the official
  [Cytoscape API](https://js.cytoscape.org/) and registry release 3.34.3 (MIT,
  bundled `index.d.ts`, no React peer dependency). The installed types and browser
  proof remain required before claiming integration works.
- [Neo4j shortest paths](https://neo4j.com/docs/cypher-manual/current/patterns/shortest-paths/)
  documents `SHORTEST 1` per endpoint pair and pre-filter placement. A local
  Query API probe on the installed 2026.08 server used exact allowed node,
  relationship and generation predicates inside the parenthesized selector.
- Read-only Cognee checkout: current graph route uses `BusinessPage` and
  `BusinessCanvas` with D3 `forceSimulation`; the older `GraphVisualization`
  uses `react-force-graph-2d`. `business/useShortestPath.ts` is client undirected
  BFS over a capped fetched graph, which cannot replace Recollect's canonical
  scoped native traversal. Reuse inspect/select interaction ideas, not that BFS.
- Read-only Atlas: `evidence-before-belief` requires inspectable provenance;
  `scope-as-a-first-class-key` covers hidden contributors; the tensions document
  distinguishes connectivity/prominence from truth. These support displaying
  evidence and complete eligibility, rather than inferring authority from layout.

## Native probe

An isolated six-node fixture contained a cycle, parallel relationships, self
loop, isolate and an excluded shorter hub. From `a`, outgoing/both reached
`c` at one hop, `e` at two and `d` at three; incoming reached only `c` at one.
The excluded two-hop route never won. Queries took 87–120 ms on this run.
The finally block deleted all six probe-owned nodes. Ignored evidence:
`.cache/exploration-proof/probe.py`, `probe.json`, `probe.log`.
This proves the interface/filter placement once, not the product endpoint.

## Decision and limits

Use Cytoscape core with a small React lifecycle adapter: native directed
multiedges/self loops, built-in bounded layouts, style/events and lifecycle
without a wrapper dependency. Sigma/Graphology adds WebGL and more components;
react-force-graph is viable but provides less direct graph styling/inspection.
Use canonical Neo4j shortest witnesses for reachability and existing path API;
do not implement BFS, centrality or force simulation. Native ties can differ
between equally short paths. Rendering and bounded dense-graph responsiveness
were subsequently exercised as recorded below.

## Delivered implementation and proof

The accepted [contract](../contracts/graph-exploration.md) is implemented in
`graph/exploration.rs`, the native adapter and lazy `GraphCanvas.tsx` /
`GraphExplorer.tsx`. The installed 3.34.3 types, initialization, layouts,
selection, resize and destruction were exercised in the real browser. Native
`SHORTEST 1` supplies witnesses; Rust validates identities/adjacency and selects
the induced canonical edges. No new graph algorithm, index, storage migration or
model operation was introduced.

Executed 2026-09-22:

- Full platform integration: **93 passed** in 282.83 s; unrelated live OIDC
  excluded. New scenarios cover native direction/hops/cycles/parallel edges/
  self loops/isolates, excluded-shorter-hub positive controls, ordinary reader
  and paired scope, projection damage and malformed native witnesses. Complete
  501-node and 2,070-edge displays refuse; an actual 500-node/2,000-edge overview
  succeeds. The shared combined-input fixture verifies hidden-input retention
  across a deliberately paused exploration request.
- Workspace: **13 passed**; separately run platform tests remain ignored there.
  Clippy workspace/all-targets with denied warnings passed. OpenAPI has **125
  unique operations**; generated types and the desktop web build passed.
  Governance validation and all **32 checker tests** passed after closeout.
- Real graph browser flow: **passed in 12.8 s**, including pointer selection,
  keyboard entity/edge inspection, canonical evidence dialog, native path
  display, failed refresh, scope clearing, response-deadline expiry and actual
  renderer destruction. Existing combined and analytical browser scenarios
  passed in the preceding regression run. Initial pointer-test timing/selected
  option assumptions were corrected; the final test exercises both routes.
- Separate renderer-only fixture: 500 nodes/2,000 edges plus keyboard inspection
  completed in **1,442 ms** on this Mac. This is a synthetic browser response,
  not proof of native qualification; the actual maximum API case is above.
- Normal SWEG runtime: the retained exact commit returned **99 nodes / 183
  edges** in **148 ms** on this run. A native two-hop `both` view around the
  retained `var.ssh_public_key` fact returned **74 nodes / 151 induced edges**.
  Desktop canvas, exact commit and relationship inspection succeeded. Coverage
  remains partial (`fragment_truncated`, `unresolved_extraction_targets`). All
  **7 Brain IDs, 33 model requests, migration 020 and zero active jobs** matched
  before/after; no customer file read or paid model call was added.
- Desktop overview, reachability and dense-renderer screenshots were inspected.
  Mobile was removed from the active acceptance scope following the user's
  explicit instruction. Earlier mobile-sizing repair is retained, but no further
  mobile product work is claimed or required by this slice.

Ignored logs/artifacts: `.cache/exploration-proof/platform.log`, `clippy.log`,
`workspace.log`, `ui.log` (prior regression run), `ui-focused.log` (final graph
success), `runtime.json`, `runtime.log`, `preservation-{before,after}.json` and
desktop screenshots. The normal API/worker is running at `127.0.0.1:8787`.
The renderer is a separate 447 kB (143 kB gzip) chunk; the pre-existing main
bundle size warning remains, without preventing the build. These are local
observations, not a universal latency claim or deployment evidence.
