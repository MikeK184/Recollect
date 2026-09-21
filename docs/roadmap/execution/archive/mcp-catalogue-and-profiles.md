# Approved MCP catalogue and profile permissions

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Configure Brain/environment MCP connections and profiles, separate use,
  manage and share, and discover authorized cached tools without starting providers.
- Non-goals: Runtime calls, credential values/resolution, Vault transport and mobile UI.
- Delivery shape: Rust/PostgreSQL catalogue, operator approval command and desktop UI.

## Governing Sources

- [Catalogue contract](../../../contracts/mcp-catalogue-and-profiles.md)
- [Runtime ADR](../../../adr/0003-product-runtime.md)
- [Team identity](../../../contracts/platform-team-access.md)
- [Workspace scope](../../../contracts/evidence-workspace-scope.md)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: Approved definitions, scoped connections, profiles/grants, cached schema
  discovery, configuration UI, transactional audit and actual database/browser proof.
- Out of scope: Starting/connecting providers, actual credential delivery and releases.
- Blockers: None; accepted contract resolves routine decisions and predecessors shipped.

## Surface and Interface Changes

- Interfaces: Operator import/disable; Brain catalogue/detail/create/update/grant/
  discover routes and generated protocol; desktop MCP panel.
- Storage: Migration 021 adds definitions, connections, profiles, memberships and
  independent direct/group grants with Brain RLS and effective permission functions.
- Ownership: Coordination owns metadata/permissions; workspace owns operation scope;
  platform retains identity/Brain authority. JSON Schema uses the maintained library.

## Data and Authority

- Inputs: Operator-approved manifests, non-secret browser configuration, explicit
  profile/environment and owned operation binding; fresh grants/membership snapshots.
- Authority: Brain ownership plus independent use/manage/share; no admin use bypass.
- Blind spots: Configured aliases/runners and cached schemas do not prove live calls.

## States and Edge Cases

- Loading: Bounded catalogue/detail reads and explicit discovery with visible progress.
- Empty: No approved definitions/connections/profiles or no authorized tools explained.
- Error: Safe schema/configuration errors; no remote payload/secret echo.
- Blocked: Disabled definitions, invalid config and missing use grant shown explicitly.
- No-access: Direct API/RLS enforcement and complete stale UI clearing after observation.
- Duplicate or replay: Unique names/keys, idempotent creation and stale-edit conflicts.
- Stale data: Five-second rights observation; optimistic edit IDs, no silent replacement.
- Reconciliation divergence: Changed approved schema revalidates configurations;
  runtime connection/schema reconciliation remains a named successor obligation.

## Integrations and Runtime Inputs

- Providers: PostgreSQL, JSON Schema validator; existing React/Mantine/TanStack stack.
- Environment: Existing DATABASE_ADMIN_URL/owner input for operator import only.
- Secrets: Only opaque approved credential aliases; no values in manifests or responses.
- Failure handling: Existing transaction/query bounds; disabled external ref resolvers;
  discovery never performs external I/O, provider retries, jobs or model requests.

## Tests and Acceptance

- Automated: Real RLS/API permission combinations, operation/environment isolation,
  schema/duplicate/stale/capacity/audit and dormant-provider sentinel checks; desktop
  configure/share/discover/revocation flow, API generation and appropriate checks.
- Manual: Inspect desktop screenshots, normal runtime metadata and inventory preservation.
- Acceptance: All [contract](../../../contracts/mcp-catalogue-and-profiles.md) cases
  with evidence recorded in the dated mapping before closeout.

## Closeout

- Planned: Approved catalogue, scoped configuration and independent profile discovery.
- Shipped: Operator-approved definitions, schema-validated scoped connections,
  independent direct/group profile grants, cold paginated discovery, transactional
  audit and complete desktop configuration/inspection with permission/error clearing.
- Not shipped: Named successor runtime/Vault/agent-tool work; no mobile work.
- New blockers: None.
- Docs updated: Contract, dated mapping, catalogue runbook, epic/indexes, README
  and continuation record.
- Validation: Five real PostgreSQL/API/RLS scenarios (4.82s), three affected
  workspace/team/evidence cases, 13 workspace tests, Clippy, generated 136-operation
  API and frontend build/typecheck passed. Two desktop flows passed (30.3s), with
  screenshots inspected. Sentinel/listener proof confirms no provider startup
  during discovery. Normal migration 021/browser proof preserves seven Brains,
  33 model requests and zero active jobs; existing SWEG recall remains usable.
  Governance lint/32 cases, formatting and whitespace checks passed. Detailed
  scope and limits are in the [mapping](../../../mappings/mcp-catalogue-2026-09-22.md).
- Version: N/A: no release policy.
- Commit: Uncommitted.
