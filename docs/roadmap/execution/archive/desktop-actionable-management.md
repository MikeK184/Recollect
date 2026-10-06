# Actionable Brain management

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Address the user's nine annotated management issues with clear actions and sensible defaults.
- Non-goals: New runners/providers, generic arbitrary tool tests, permission migration, file-length splitting.
- Delivery shape: Existing web components, focused existing tests, local stack and documentation.

## Governing Sources

[Desktop](../../../contracts/desktop-experience.md), [managed memory](../../../contracts/memory-managed-experience.md), [autonomy](../../../contracts/memory-autonomous-maintenance.md), [MCP catalogue](../../../contracts/mcp-catalogue-and-profiles.md), [runtime](../../../contracts/mcp-runtime-and-credentials.md), [private execution](../../../contracts/mcp-vault-and-private-runners.md), [owning epic](../../epics/desktop-visual-experience.md).

## Scope

- In scope: Generic server setup/catalogue provenance; bounded explicit anonymous HTTP Test; compact tools; member list/popups; inline privacy sections; conditional legacy model overrides; optional environment management; clearer private-network execution.
- Out of scope: Changing grants, existing live privacy/provider settings, new endpoint/storage/dependency, auto-running a tool during test.
- Blockers: None; user decisions are reflected in the desktop amendment.

## Surface and Interface Changes

- Interfaces: Existing owner-only HTTP inspection, connection/definition/profile reads, grants, workspace/groups and policy APIs. No endpoint added.
- Storage: N/A; canonical schemas unchanged.
- Ownership: Existing management/feature components; shared existing policy editors gain inline rendering.

## Data and Authority

- Inputs: Actual catalogue, members, policies, environment groups and explicit inspection response.
- Authority: Existing server permission gates, policy change IDs, idempotency and independent tool rights.
- Blind spots: Anonymous inspection does not prove configured authentication/private execution or actual tool-call success. All such cases direct to explicit authorized tool use.

## States and Edge Cases

- Loading: Existing skeletons/loaders; unavailable configuration cannot be edited/tested.
- Empty: Empty approved catalogue, members, tools/environments have useful bounded actions.
- Error: Failed refresh hides protected reads; failed Test clears success and reports actual failure.
- Blocked: Unsupported test placement/authentication explains the actual tool-call path; missing provider remains explicit.
- No-access: Readers/archived Brains cannot edit; owner-only inspection remains owner-only; changed access closes forms.
- Duplicate or replay: Reuse policy/group idempotency and base revisions; no silent test retry.
- Stale data: Clear inspection when connection revision/authority changes; no stale policy editor after refresh failure.
- Reconciliation divergence: Display effective memberships and retained policy values; no implicit bulk preset conversion.

## Integrations and Runtime Inputs

- Providers: User-explicit anonymous MCP inspection only; no dependency added. Atlas local trust/editing/scope patterns inform presentation only.
- Environment: Existing installation inputs only; no secrets or paths copied to docs.
- Secrets: Existing credential reference and redaction boundaries; tests use disposable installation credentials.
- Failure handling: Existing 25-second bounded metadata handshake, no retries; existing policy stale/replay gates.

## Tests and Acceptance

- Automated: Build/typecheck/design; focused existing Access, privacy, MCP and managed UI journeys; governance/whitespace and CodeGraph sync.
- Manual: Browser inspection of real local pages plus independent UI source/screenshot review.
- Acceptance: No provider hardcode; actual test result distinguished from execution; compact metadata; members/popups; privacy changes inline without tabs/drawer; managed autonomy hides literal-only overrides; optional environments actionable; local readiness.

## Closeout

- Planned: All nine user comments plus optional environments/default clarity.
- Shipped: Generic server entry and installation catalogue with actual provenance; explicit qualified metadata Test and compact tools; member list and focused popups; inline one-section privacy editors; conditional legacy learning controls; optional environment creation/rename; clearer private-network execution. Existing policies, grants and live inventory are preserved. Privacy editors load on demand within existing files. Accepted and deployed locally.
- Not shipped: Automatic tool execution during Test, authenticated/private generic probes, atomic storage-policy CAS, new dependencies or API/schema changes. Deferred research and LongMemEval remain separate.
- New blockers: None.
- Docs updated: Desktop amendment, dated evidence, this archived pack, epic and execution/evidence indexes, and `CONTINUE_HERE.md`.
- Validation: Six existing focused browser journeys pass across targeted disposable runs: name/URL and real metadata Test; catalogue/environment creation/rename; access invitation/revocation; inline retention/storage safeguards; managed defaults; empty catalogue registration. Build/typecheck/design pass. Independent source and actual deployed UI review found no blocker. Final governance/whitespace and CodeGraph sync pass; rebuilt local stack reports ready. See [dated evidence](../../../mappings/desktop-actionable-management-2026-10-04.md) for limits and the corrected required-label test locator.
- Version: N/A; no release policy.
- Commit: Uncommitted.
