# Consistent management scale and persistent diagnostics

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Connectors and both agent views use the Brains shell scale; diagnostics is always reachable at top right and automatic context navigation disappears.
- Non-goals: New diagnostic/backend authority or changes to automatic scope discovery.
- Delivery shape: Local frontend implementation, focused browser proof and documentation.

## Governing Sources

- [Desktop experience](../../../contracts/desktop-experience.md), browser feedback amendment.
- [Managed experience](../../../contracts/memory-managed-experience.md).
- [Owning epic](../../epics/desktop-visual-experience.md).

## Scope

- In scope: Shared 248px shell, 14px nav, existing compact headings/gutters, always-visible diagnostic icon, account Team, normalization of automatic-context links and removal of recommended-default settings controls with creation defaults preserved.
- Out of scope: Model catalogue/backend selection, separately owned by the memory successor.
- Blockers: None; user approved the concrete review and card correction.

## Surface and Interface Changes

- Interfaces: Existing routes and diagnostics dialog; no new server endpoint.
- Storage: N/A; presentation and route normalization only.
- Ownership: Desktop App chrome/shared management CSS; agents worker removes context helpers.

## Data and Authority

- Inputs: Existing health, session, Brain and roster reads.
- Authority: Canonical route authorization and existing operator diagnostic controls.
- Blind spots: No online claim from an enabled credential or current health inference from a historic successful call.

## States and Edge Cases

- Loading: Persistent icon remains available while detail loads.
- Empty: No invented activity or status; existing empty-state actions remain.
- Error: Diagnostics exposes recorded failures; route failures retain explicit states.
- Blocked: No hidden automatic-context UI is reintroduced when discovery is unavailable.
- No-access: Existing denied operators do not gain protected detail from icon visibility.
- Duplicate or replay: Route normalization is idempotent.
- Stale data: Existing refresh/role-loss clearing stays intact.
- Reconciliation divergence: No backend mutation or automatic policy change from chrome.

## Integrations and Runtime Inputs

- Providers: N/A; existing local service reads only.
- Environment: Existing frontend service origin.
- Secrets: No new secret handling; never persist credential values in chrome.
- Failure handling: Existing read errors and retry; no implicit tool/provider execution.

## Tests and Acceptance

- Automated: Frontend build/typecheck/design checks, focused route/chrome checks, governance and whitespace checks.
- Manual: Same-viewport Brains/Connectors/global and Brain Agents shell measurements; healthy diagnostic icon; legacy context links; second-agent screenshots.
- Acceptance: Sidebar/navigation/heading scale stays consistent through route changes; diagnostics works including healthy state; Team remains in account controls; ordinary automatic-context and recommended-default entry points are gone; creation defaults remain.

## Closeout

- Planned: Shared compact scale, persistent diagnostics and automatic-context navigation removal.
- Shipped: Shared 248px/14px/40px shell scale, compact management controls,
  persistent healthy-state operator diagnostics, Team in account controls,
  normalized legacy context routes and removal of recommended-default controls.
- Not shipped: N/A within this presentation slice; native-host acceptance belongs
  to the active direct-token pack.
- New blockers: None.
- Docs updated: Desktop contract, owning epic/index, archived pack, evidence and handoff.
- Validation: Same-viewport CUA measurements across all requested routes,
  healthy diagnostics/Team and legacy context navigation pass. Independent
  final actual-pixel review, frontend production build/type/design checks and
  local stack readiness pass. Creation defaults are covered by model fixtures.
- Evidence: [Coordinated delivery and limits](../../../mappings/desktop-browser-management-2026-10-05.md).
- Version: N/A; no release requested.
- Commit: Uncommitted.
