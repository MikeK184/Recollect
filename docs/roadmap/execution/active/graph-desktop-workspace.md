# Canvas-first contextual graph workspace

Status: in-progress
Owning epic: `docs/roadmap/epics/graph-intelligence.md`
Work type: product

## Summary

- Goal: Open a useful bounded graph immediately when eligible, then inspect evidence, paths and explicit analytics through compact contextual controls.
- Non-goals: Graph editing, arbitrary Cypher, unbounded auto-traversal, model calls/rebuild/analytics on entry, guessed manifests and historical current-graph substitution.
- Delivery shape: Local capability-owned frontend/API changes and focused proof, no external deployment.

## Governing Sources

- [Desktop ADR](../../../adr/0014-desktop-experience-and-answers.md) and [desktop contract](../../../contracts/desktop-experience.md)
- [graph-projection-and-traversal](../../../contracts/graph-projection-and-traversal.md)
- [graph-cross-repository-views](../../../contracts/graph-cross-repository-views.md)
- [graph-exploration](../../../contracts/graph-exploration.md)
- [graph-analytics](../../../contracts/graph-analytics.md)
- [Owning epic](../../epics/graph-intelligence.md)

## Scope

- In scope: Canvas-first layout, Knowledge/Repository/Combined toolbar, scope summary/filter drawer, right inspector, keyboard/list alternative, contextual path endpoints, explicit Insights and status/repair popover.
- Out of scope: Graph editing, arbitrary Cypher, unbounded auto-traversal, model calls/rebuild/analytics on entry, guessed manifests and historical current-graph substitution.
- Blockers: None; accepted desktop/domain contracts resolve behavior. Shell integrates through its predecessor slice; final browser acceptance requires that integration.

## Surface and Interface Changes

- Interfaces: Existing graph/explore, view/path/status/analytics APIs. Entry with no explicit selection performs one existing bounded knowledge overview under current Brain-only investigation selection; explicit exact scope is preserved.
- Storage: N/A: presentation/read default introduces no new graph persistence or canonical schema.
- Ownership: graph-intelligence owns these feature views and canonical domain handlers; platform owns shared tokens/shell/components.

## Data and Authority

- Inputs: Canonical qualified view, exact manifests/snapshots, node/edge evidence, generations, epoch/deadline and actual job reports.
- Authority: Existing canonical graph selector/native Neo4j/GDS bounds; layout and clicked node never confer new permission or deployment proof.
- Blind spots: Reachability is recorded structure rather than runtime impact; missing links/coverage/projections remain explicit.

## States and Edge Cases

- Loading: Canvas async state and bounded read indicator; renderer is lazy-loaded and disposed on replacement/unmount.
- Empty: Useful no eligible graph/no selected input/no path states; never fabricate an edge or broaden scope.
- Error: Projection/capacity/oversize/read errors clear prior results and provide a narrower selection or retry action.
- Blocked: Missing exact repository/manifest selection requests it; no substitute latest snapshot.
- No-access: Current grants and eligible evidence rechecked by canonical handlers; revoked reads clear renderer/inspector.
- Duplicate or replay: Navigation/read retry creates no paid/background mutation; rebuild and analytics require explicit authorized actions.
- Stale data: Cancel superseded reads; Brain/scope/epoch/expiry/read failure clears graph, path highlights and selected evidence.
- Reconciliation divergence: Preserve readiness/generation/coverage and stale analytics semantics; do not render a prefix as complete.

## Integrations and Runtime Inputs

- Providers: Existing Neo4j/GDS service through Rust plus Cytoscape renderer; no replacement graph library.
- Environment: Existing NEO4J_URL and installed service inputs only; no new environment variables.
- Secrets: Preserve existing credential transport/redaction; no secret values in assets, URLs, fixtures, docs or output.
- Failure handling: Retain two-read capacity, 15-second query boundary and existing graph display limits; no hidden heavy retry.

## Tests and Acceptance

- Automated: Bounded entry call count/no mutation or model, scope preservation, keyboard/list/node/path behavior, stale/cancel/oversize fixtures and existing graph regression; type/build and governance checks.
- Manual: Real retained graph at desktop sizes; inspect node/edge source, keyboard list, select path, explicit analytics and missing exact-input states.
- Acceptance: Graph dominates the page, controls remain reachable and exact, useful graph proof passes with positive/negative eligibility, no stale graph or guessed scope survives.

## Closeout

- Planned: Canvas-first layout, Knowledge/Repository/Combined toolbar, scope summary/filter drawer, right inspector, keyboard/list alternative, contextual path endpoints, explicit Insights and status/repair popover.
- Shipped: Not yet. Canvas/list and filter/path/insights/status views, exact URL scope/center and bounded entry are implemented. Combined graph, current recall-graph and investigation journeys passed. The final live retained two-node graph shows separated captions and the human-readable uncertainty label; current route accessibility checks passed.
- Not shipped: Final Graph and Graph analytics browser regressions and the remaining graph/path interaction acceptance are open. The small live screenshot and heading-visible timing samples do not establish canvas interaction p95 or large-graph capacity. Explicit product non-goals remain excluded.
- New blockers: No unresolved product decision is known. Implementation refinements and verification are tracked in the [dated mapping](../../../mappings/desktop-experience-implementation-2026-09-26.md).
- Docs updated: [Desktop guide](../../../runbooks/desktop-experience.md), [implementation/assets/dependency evidence](../../../mappings/desktop-experience-implementation-2026-09-26.md), [current handoff](../../../../CONTINUE_HERE.md), affected domain runbooks, owning epic and indexes.
- Validation: Frontend typecheck/design and final image build passed; workspace clippy and 21 unit tests passed with 3 live-Vault cases explicitly ignored; governance lint and 32 tests passed. Desktop seven cases passed across a six-pass run and the repaired font-fallback targeted rerun; real Team/OIDC two, Ask five and MCP setup/runtime six passed. The dated mapping separates each owner result, initial failures, fixture/provider boundaries and remaining checks.
- Version: N/A: no release requested.
- Commit: uncommitted.
