# Grouped spatial graph exploration

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Readable stable grouped 3D navigation with exact qualified evidence.
- Non-goals: Change graph authority, replace Neo4j/GDS or load a whole Brain.
- Delivery shape: Lazy React renderer, typed organization integration and browser proof.

## Governing Sources

[ADR 0021](../../../adr/0021-grouped-spatial-graph-navigation.md),
[3D contract](../../../contracts/graph-grouped-3d-navigation.md),
[organization](../../../contracts/automatic-knowledge-organization.md),
[exploration](../../../contracts/graph-exploration.md).

## Scope

- In scope: Named topic overview/expansion, shared 2D/3D identities/inspection,
  finite stable layout, fallback, accessibility and measured browser performance.
- Out of scope: VR/AR, graph authority changes, new extraction engine.
- Blockers: No architectural blocker; organization API integration follows its owner.

## Surface and Interface Changes

- Interfaces: Extend the renderer adapter with optional explicit typed display
  groups; dimension choice remains local presentation. Existing graph APIs retain
  qualification. Organization APIs own topic membership and evidence reads.
- Storage: No new browser evidence persistence; backend organization owns topics.
- Ownership: Desktop renderer/interaction; graph intelligence owns data and lifecycle.

## Data and Authority

- Inputs: Current bounded qualified graph/organization selection.
- Authority: Canonical backend reads; layout is presentation only.
- Blind spots: Dependency and software-WebGL proof do not establish hardware FPS.

## States and Edge Cases

- Loading: Lazy module/scene progress and bounded data expansion.
- Empty: Honest no eligible inputs or unassigned coverage.
- Error: Clear failed view and offer usable 2D fallback.
- Blocked: Retain explicit backend unavailable/capacity/quality state.
- No-access: Remove graph, groups, inspector and selection.
- Duplicate or replay: Stable canonical IDs; cloned renderer inputs.
- Stale data: Existing epoch/generation/scope/deadline clearing applies.
- Reconciliation divergence: Semantic membership is never inferred from layout.

## Integrations and Runtime Inputs

- Providers: React force-graph, Three.js; existing canonical API.
- Environment: No new secrets or model provider.
- Secrets: Plain labels, no external asset fetch or unsafe HTML.
- Failure handling: WebGL/context errors fall back; stop hidden simulation and dispose.

Final acceptance review found a small visibility-ordering defect within the
existing hidden-rendering contract: an initial nonintersecting observer event
can precede the measured renderer mount, losing its pause state. Preserve that
observed state across mount and apply it during initialization. Add a controlled
browser regression for false-before-mount followed by visible resumption, plus
the existing visibility/camera regression. Strengthen the existing adapter proof
with actual scene-material path highlights and keyboard ArrowDown/Enter selection;
path counters and mouse-driven finder selection alone do not establish those
acceptance tasks. This is an isolated correction under
the documented small-fix exception; no interface, dependency or backend change
is required. The new regression reproduced offscreen frames advancing 11 to 19
before the fix; the complete four-case adapter suite now passes in 38.0 s,
including actual mesh path highlights and keyboard selection. Design/type
checks pass, and the current laptop/wide renders have been reviewed.

Final review also reproduced a generation-invalidation gap: a loaded generation
became superseded at unchanged memory/link epochs, but its old canvas and source
inspector remained through multiple heartbeats. The corrected browser clears
the scene, paths and inspector on exact invalidation, with explicit reload
recovery. Missing IDs only in the inventory page, unrelated generations and
pending rebuilds alone do not clear the scene. A poll is fenced to its accepted
parent/centered read revision, and the status drawer uses that accepted scene's
generation, inputs, counts and coverage. It never substitutes an old initial
overview or unscoped inventory epoch for the current rendered view.

The pagination counterexample is confirmed: more than twenty unrelated newer
generations can hide both a loaded generation and its replacement indefinitely.
The accepted exploration contract therefore adds bounded exact generation
tracking to the existing status GET: at most 102 IDs, additive default-empty
tracked/missing fields, unchanged inventory pagination and Brain read gates.
The actual accepted parent/centered scene is wired to that heartbeat. Current
proof passes eleven focused browser cases in 1.0 min: terminal/missing tracked
rows, combined inputs, paginated absence, unrelated and pending rows, old-poll/
new-read ordering for parent/child/same-ID reads, current status provenance,
explicit recovery and incomplete/foreign/malformed tracking failures. Two native
metadata API cases pass in 2.15 s, alongside legacy JSON compatibility and
regenerated optional OpenAPI/TypeScript fields. Production web build, type/design
and independent source review pass. Named receipts are in the implementation
ledger. The replacement backend qualification freeze is v4 with unchanged paid
scope; its external grant authorized the now-completed campaign, whose quality gates failed; activation remains off.

## Tests and Acceptance

- Automated: Type/design/build, graph fixture browser tasks, stale clearing,
  keyboard/fallback/resize/reduced-motion and repeated lifecycle checks.
- Manual: Independent laptop/wide rendered review and performance receipt.
- Acceptance: Complete 3D contract including source-linked topic navigation;
  no claim that renderer performance repairs backend contention.

## Closeout

- Planned: Complete grouped 3D navigation and qualified integration.
- Shipped: Local lazy 3D renderer, shared qualified 2D/3D identities, topic/source
  entry, bounded layout, inspection, fallback and lifecycle disposal are implemented;
  exact generation tracking, native topic/source navigation and manual-overlay
  integration pass the complete local renderer contract. Independent review
  found no remaining renderer acceptance gap.
- Not shipped: Deployment/normal-installation update. Real automatic organization
  qualification belongs to its owning pack and has failed; closing the renderer
  does not select or qualify a producer or community recipe.
- New blockers: None.
- Docs updated: ADR and contract before implementation; implementation ledger,
  owning epic, pack and active/archive/epic indexes reconciled at closeout.
- Validation: Focused 3D/browser and native legacy/organization checks pass. The
  latest unmocked worker-to-API-to-browser fixture passes eight checks in 12.89 s,
  with three controlled extraction requests, no embedding or paid calls, and
  exact rename/split/exclude audits. Independent laptop/wide visual review passes.
  The bounded 500-node/2,000-edge M4 Pro/Chrome hardware receipt measures p95
  selection-to-paint 48.3 ms, actual pointer-to-inspector 60.0 ms and twelve-cycle
  retained heap growth 3.72 MiB. Cold first usable render is 2.606 s. Recorded RAF
  cadence is not GPU rendering time; these finite local observations do not prove
  universal FPS or absence of leaks. The reproduced initial offscreen race is
  fixed; four renderer cases pass in 38.0 s. Eleven exact-generation browser
  cases pass in 1.0 min, and two native metadata cases/legacy JSON compatibility
  pass. Type/design/production build and full repository validation pass. Actual unmatched-manual-overlay
  publication through API and browser passes four checks in 7.04 s with zero
  provider calls, exact source evidence and laptop/wide visual review. Duplicate
  source titles receive a short version suffix in the editor. See the
  [implementation ledger](../../../mappings/graph-organization-implementation-2026-10-18.md).
