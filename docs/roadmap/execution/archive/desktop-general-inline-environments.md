# General settings and inline environments

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Implement the user-selected General and environment-add concepts.
- Non-goals: New environment semantics, permissions, schema or deletion API.
- Delivery shape: Bounded light General layout and existing inline commands.

## Governing Sources

- [Latest selected desktop amendment](../../../contracts/desktop-experience.md)
- [Team access](../../../contracts/platform-team-access.md)
- [Workspace scope](../../../contracts/evidence-workspace-scope.md)
- [Brain deletion](../../../contracts/platform-brain-deletion.md)
- [Visual epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Main Brain details/environment cards plus side status/archive/delete;
  stable inline environment addition/rename. Brain details reuse existing
  name/description/icon editor semantics, with inline form instead of overlay
  where safe reuse permits it. Match selected light concept hierarchy.
- Out of scope: Changes to defaults, grants, archive/delete results or real edits
  to existing Brain/environment data for visual proof.
- Blockers: None; user selected concepts. Demo/MCP runtime exercise is separate
  evidence over canonical APIs, not new product behavior.

## Surface and Interface Changes

- Interfaces: Existing Brain PATCH/icon handlers and evidence-group POST/PATCH;
  archive/reopen and deletion preview/exact-name flow unchanged.
- Storage: N/A; no migration.
- Ownership: General presentation; preserve all previous fields/actions.

## Data and Authority

- Inputs: Current Brain and workspace/evidence group reads; mock data is illustrative.
- Authority: Retain the existing command boundaries, idempotency and reload checks.
  Admins may edit Brain metadata, including on archived Brains, as allowed by the
  existing canonical PATCH. Environment writes require admin and a non-archived
  Brain. Reader sees actual data without mutation actions.
- Blind spots: No server CAS for environment rename; preserve current fresh-read
  existence and description checks, without claiming atomicity.

## States and Edge Cases

- Loading: Add remains disabled without a current workspace read.
- Empty: Concise empty list with inline Add; Brain-wide work needs no environment.
- Error: Existing errors and retry; no silent clearing or automatic mutation retry.
- Blocked: Archive, role or workspace-read loss closes environment drafts and
  disables their writes. Brain metadata editing retains its historical archived
  behavior; loss of admin authority closes that editor.
- No-access: No edit/add/archive/delete for reader. Existing server enforcement.
- Duplicate or replay: Existing name validation and creation idempotency.
- Stale data: Rename re-reads exact environment and preserves its description;
  retain stale/error response, cancel draft on authority or read loss.
- Reconciliation divergence: Brain identity/icon commands remain separate where
  existing behavior requires it; no atomic combined-save claim. Delete still
  requires its existing canonical preview/confirmation, never a plain click.

## Integrations and Runtime Inputs

- Providers: Existing React/Mantine/client; no new dependency.
- Environment: Existing local stack. Full build disk guard may require a bounded
  frontend-only image layer over the exact existing backend image; record which
  path was actually used, never report it as a fresh backend build.
- Secrets: No credentials in UI artifacts/source.
- Failure handling: Preserve partial/current error behavior and pending controls.

## Tests and Acceptance

- Automated: Production type/design/build, pure preserved-policy checks,
  governance, whitespace and CodeGraph; update existing regression selectors.
- Manual: CUA real owner/reader view, inline Add/Rename cancel, existing confirmation
  entry visibility, width/overflow and independent source/pixel acceptance. No
  archive/delete or real policy edits for proof.
- Acceptance: Selected main/side hierarchy, visible compact environment rows,
  inline focused Add/Rename with stable list and Save/Cancel; actual permissions
  govern controls. All existing identity/archive/deletion features retained.

## Closeout

- Planned: Selected bounded General and inline environments.
- Shipped: Bounded main/side General hierarchy, safe reused inline Brain identity
  editor and stable inline environment Add/Rename, over existing APIs and gates.
- Not shipped: New APIs, schema or revised scope/grants.
- New blockers: None.
- Docs updated: Contract, epic, archived pack/index, dated settings/demo mapping
  and handoff. [Evidence](../../../mappings/settings-demo-scenario-2026-10-06.md).
- Validation: Type/design/production build, four pure Privacy checks, affected
  regression selector discovery, 32 governance tests, whitespace, CodeGraph;
  local frontend-only deployment over the existing backend and readiness.
  CUA owner/reader controls, inline cancelled drafts, 2504 px width proof and
  independent final source/pixel acceptance. No real settings writes or deletion.
- Version: N/A; no release policy.
- Commit: Uncommitted.
