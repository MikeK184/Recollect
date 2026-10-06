# Compact 2D graph interactions

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Readable graph labels, truthful quick filters and focused evidence inspection.
- Non-goals: 3D, Graphify extraction, new graph authority, invented relationships.
- Delivery shape: Existing frontend and local stack; no release or commit.

## Governing Sources

- [Graph exploration](../../../contracts/graph-exploration.md#compact-graph-interactions--2026-10-03).
- [Desktop knowledge](../../../contracts/desktop-knowledge-surface.md).
- [ADR 0017](../../../adr/0017-desktop-knowledge-and-ask-experience.md).

## Scope

- In scope: Shared 2D labels/finder/type filters/focus; compact Evidence and Contributions summaries with full evidence actions.
- Out of scope: Backend/API changes, new provider calls, TV acceptance or whole-epic closeout.
- Blockers: None; user approved the presented interaction plan.

## Surface and Interface Changes

- Interfaces: Internal GraphCanvas/GraphRenderer props only; public API unchanged.
- Storage: N/A: no persistence.
- Ownership: Cytoscape handles rendering/adjacency; existing parents own qualification, selection and canonical reads.

## Data and Authority

- Inputs: Already qualified bounded nodes/edges and exact contribution observations.
- Authority: Existing Brain grants, data leases, scope and native paths; local view never changes authority.
- Blind spots: Direct loaded connections are not an exhaustive Brain neighbourhood, live presence or inferred authorship.

## States and Edge Cases

- Loading: Existing bounded reads and lazy renderer; no illustrative data.
- Empty: Distinguish no local matches from no eligible evidence.
- Error: Clear affected graph and summary; preserve existing recovery controls.
- Blocked: Existing policy/scope notices remain; local controls introduce no new prerequisite.
- No-access: Existing fail-closed clearing removes summaries and canvas.
- Duplicate or replay: Stable canonical IDs; metadata polls retain viewport and layout.
- Stale data: Existing failure/expiry/epoch gates remain; removed selections clear.
- Reconciliation divergence: Exact kinds only; complete qualified paths override local filters/focus.

## Integrations and Runtime Inputs

- Providers: N/A: local controls make no model or provider request.
- Environment: Existing local stack only.
- Secrets: No credentials copied or exposed.
- Failure handling: Existing request cancellation/leases; render cleanup and safe plain labels.

## Tests and Acceptance

- Automated: Existing web build/design/typecheck and repository validator; no unnecessary new tests.
- Manual: Real filters/counts, readable labels, focus/restore, inspector actions, complete path, failed refresh clearing, poll stability and desktop bounds; independent UI reviewer.
- Acceptance: Both perspectives share compact interactions, exact data/qualifications remain reachable, no 3D or new engine/dependency.

## Closeout

- Planned: Above approved scope.
- Shipped: Shared lazy 2D finder, Auto/All/None labels, readable hover titles,
  exact-kind filters, direct-connection focus with overview restoration and
  compact qualified evidence summaries. Existing canonical evidence and path
  actions remain available. Existing local stack rebuilt with these controls.
- Not shipped: TV acceptance and epic closeout remain separate.
- New blockers: None.
- Docs updated: Graph contract and owning epic/index/pack lifecycle; handoff at delivery.
- Validation: Production web build/design/typecheck and independent source/live
  visual review pass. Focused browser evidence below, repository validator,
  whitespace and synchronized CodeGraph checks pass.
- Version: N/A: no release policy.
- Commit: Uncommitted.

## Dated evidence — 2026-10-03

- Dependency guidance: Context7 checked the official Cytoscape.js source/API for
  existing adjacency, display masks, viewport and styles. The lookup did not
  return `min-zoomed-font-size` specifically; the installed Cytoscape 3.34.3
  declarations were checked for that property. No new dependency or engine.
- Review: Independent source review caught outside-filter connection navigation,
  disappearing filter matches and tiny hover labels. All were corrected;
  subsequent source and actual screenshot review has no material blocker.
- Live data: SWEG Evidence loads 111 entities/86 directed relationships;
  Contributions loads 33/54 from the actual 30-input window. Agents plus direct
  connections shows 26/33 and 24/54, matching exact Cytoscape adjacency and
  endpoints. Agent focus shows 2/33 and 1/54. Following an outside-focus link
  reveals its target and retains selection; Show all restores exact zoom/pan.
- Read stability: Across 46 successful pipeline refreshes, the same Core,
  exact camera and node positions within floating-point tolerance remained.
  Labels, filters and focus introduced no POST or provider calls. Empty-filter
  retention and removed-selection handling were independently source-reviewed.
- Evidence and paths: A real memory's Open evidence reads its exact recorded
  revision through handover-inputs. A real directed one-hop path displays all
  two nodes/one edge, with local filters and both focus actions disabled.
  Material conflict/source/operational qualifications remain visible.
- Failure and bounds: Blocking the real pipeline refresh removes both canvas
  and compact summary; restored networking recovers the graph. Desktop
  1440×900 and 1920×1080 have no horizontal overflow; all canvas controls fit.
  The final 1280×720 build permits outer scrolling and has no internally clipped
  controls; independent review confirms the final screenshot. Browser errors: 0.
  Temporary network/viewport overrides were reset. No live fixtures or model
  calls, backend changes, new dependencies or new tests.
- Proof: Ignored `.cache/graph-interactions-contribution-focus.jpg`,
  `graph-interactions-path.jpg`, `graph-interactions-evidence-1440.jpg`,
  `graph-interactions-evidence-1920.jpg` and
  `graph-interactions-failed-refresh.jpg`, `graph-interactions-final.jpg` and
  `graph-interactions-browser-proof.json`; build/deploy/check logs share the
  `.cache/graph-interactions-` prefix. Existing initial shell chunk warning
  remains; this change adds no performance guarantee or unbounded loading.
