# Graph exploration and bounded impact views

Status: shipped
Owning epic: `docs/roadmap/epics/graph-intelligence.md`
Work type: product

## Summary

- Goal: Explore qualified graph relationships visually, inspect evidence, and
  follow native shortest paths or bounded recorded reachability.
- Non-goals: Unbounded visualization, graph editing, inferred runtime impact,
  new graph engines and client graph algorithms.
- Delivery shape: Local Rust API, React UI, integration proof and operator docs.

## Governing Sources

- [Exploration contract](../../../contracts/graph-exploration.md)
- [Projection contract](../../../contracts/graph-projection-and-traversal.md)
- [Combined input contract](../../../contracts/graph-cross-repository-views.md)
- [Analytics contract](../../../contracts/graph-analytics.md)
- [ADR 0007](../../../adr/0007-canonical-graph-projections.md)
- [Owning epic](../../epics/graph-intelligence.md)

## Scope

- In scope: Qualified overview/reachability endpoint; Cytoscape canvas and HTML
  inspection; direction, hops, path highlighting, current provenance and limits.
- Out of scope: New storage/migrations, model calls, graph editing and retrieval
  fusion. The existing analytics report UI remains available.
- Blockers: None. Slice 18 is shipped; the native query probe succeeded.

## Surface and Interface Changes

- Interfaces: Add `graphExplore` and protocol/OpenAPI types from the contract;
  preserve existing graph view/path/analytics interfaces.
- Storage: N/A: canonical PostgreSQL descriptors and existing Neo4j projections
  suffice; exploration results are transient reads.
- Ownership: Graph read selector owns qualification and final gates; native
  adapter owns traversal; a lazy React/Cytoscape adapter owns rendering only.

## Data and Authority

- Inputs: Existing exact qualified repository, combined or knowledge graph;
  optional eligible center, direction and hop bound.
- Authority: PostgreSQL and retained artifacts; Neo4j verifies and traverses the
  derived projection, and every returned witness is checked against canonical
  identities. Canvas connectivity or prominence never changes trust.
- Blind spots: Extraction coverage, unresolved links, stale generations and
  investigation-only evidence retain explicit reasons. Bounded reachability is
  not a claim of deployment behavior or an exhaustive transitive dependency.

## States and Edge Cases

- Loading: Show pending request; dispose of replaced graph and selection.
- Empty: Show eligible empty graph or center-only reachability explicitly.
- Error: Display bounded error; no old graph survives a failed refresh.
- Blocked: Missing projection, capacity, timeout and display-size refusal have
  specific messages with smaller-scope/retry guidance.
- No-access: Enforce current Brain and paired operation access; clear results
  after denied refresh and reuse canonical evidence dialogs.
- Duplicate or replay: Read-only requests have no persistence; request identity
  and cancellation prevent late results from replacing a current selection.
- Stale data: Clear on scope/epoch/invalidation and retention expiry; request
  current results again. Never silently merge datasets across scopes.
- Reconciliation divergence: Verify all selected native input and validate
  witness adjacency/bounds; refuse malformed or missing projection data.

## Integrations and Runtime Inputs

- Providers: Existing repository-owned Neo4j Query API; Cytoscape.js 3.34.3
  with built-in layouts and bundled types. No mandatory Cognee runtime.
- Environment: Existing `NEO4J_URL`, `NEO4J_USER`, `NEO4J_PASSWORD`,
  `NEO4J_DATABASE` and PostgreSQL settings; no new environment variables.
- Secrets: Environment/ignored local secret files only; sanitized diagnostics.
- Failure handling: Shared two-read capacity, 15-second request deadline,
  database statement timeout; no silent retry or partial graph on overflow.

## Tests and Acceptance

- Automated: Actual scoped native reachability and overflow fixtures; meaningful
  access/correction/projection failure checks; Playwright real canvas, keyboard,
  native path, selection, stale-state clearing and desktop proof. Run workspace
  checks, OpenAPI generation, web build and `./scripts/validate.sh`.
- Manual: Inspect screenshots and retained SWEG runtime graph; preserve normal
  Brain and model-request counts and avoid new model/customer repository reads.
- Acceptance: Contract behavior works through API and UI with complete selected
  provenance; no custom traversal/layout or hidden incomplete display.

## Closeout

- Planned: Qualified native exploration API and accessible interactive graph UI.
- Shipped: Read-only qualified native overview/reachability with complete
  input verification, witness validation and explicit display refusal; lazy
  desktop Cytoscape exploration, evidence/relationship inspection, native path
  display, keyboard controls and stale/expiry/disposal handling. Existing
  canonical graph, combined-input and analytical report behavior is preserved.
- Not shipped: Mobile views and mobile acceptance are deferred by the user's
  explicit 2026-09-22 direction. Desktop acceptance remains required.
- New blockers: None.
- Docs updated: Contract, dated interface/proof mapping, desktop runbook, graph
  epic, execution/epic indexes, README and local continuation record.
- Validation: Actual platform 93 passed (live OIDC excluded); workspace 13
  passed; Clippy all targets passed; API 125 operations and web build passed;
  graph desktop browser passed, existing combined/analytics scenarios passed;
  native maximum/overflow and hidden-retention proof passed. Renderer-only
  500/2000 case took 1,442 ms. Retained normal SWEG 99/183 overview and 74/151
  two-hop view passed, with seven Brains and 33 model requests preserved. See
  the [dated mapping](../../../mappings/graph-exploration-interfaces-2026-09-22.md)
  for evidence boundaries and retained coverage limits. Governance validation
  and all 32 checker tests passed after lifecycle reconciliation.
- Version: N/A: no release policy.
- Commit: Uncommitted.
