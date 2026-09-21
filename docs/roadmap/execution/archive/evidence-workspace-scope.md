# Workspace catalogue and task operation scope

Status: shipped
Owning epic: `docs/roadmap/epics/evidence-and-workspaces.md`
Work type: product

## Summary

- Goal: Discover a workspace's local checkouts and keep concurrent task/subagent operations bound to immutable scope.
- Non-goals: Repository extraction, event capture, retrieval or MCP execution.
- Delivery shape: Shared DTOs/normalization, native discovery/CLI, PostgreSQL/API and browser catalogue/task workflows.

## Governing Sources

- [Workspace scope contract](../../../contracts/evidence-workspace-scope.md)
- [Runtime ADR](../../../adr/0003-product-runtime.md)
- [Device authority](../../../contracts/platform-device-pairing.md)
- [Collections](../../../contracts/evidence-collections.md)
- [Owning epic](../../epics/evidence-and-workspaces.md)

## Scope

- In scope: Nearest selector, bounded read-only Git discovery, canonical repository/alias identities, private checkout observations, immutable task/child/operation snapshots and fresh-context handoff.
- Out of scope: Raw checkout upload, extraction/publication, host hooks, source retrieval and profile execution.
- Blockers: None; predecessors shipped and routine details resolved in the contract.

## Surface and Interface Changes

- Interfaces: Workspace catalogue/refresh, alias registration, task creation/history/scope/close and operation binding/inspection; native workspace/scope commands and typed UI.
- Storage: Brain repositories/origins, account/device workspace checkouts, tasks, scope snapshots and operation records under RLS.
- Ownership: Companion discovers local metadata; server validates identities/grants; task rows select future defaults; operation rows preserve immutable attribution.

## Data and Authority

- Inputs: A one-field TOML selector, read-only Git observations, paired principal/device and explicit resource UUID selections.
- Authority: Current Brain grants, per-account working context and canonical repository IDs. Local origin/path alone grants no access.
- Blind spots: Discovery may be incomplete; HEAD/dirty is a checkout observation, not publication/deployment. Handoff exposes pending retrieval without claiming it ran.

## States and Edge Cases

- Loading: Bounded local scan and paginated server inventories/history.
- Empty: No selector, repositories, registered workspaces or tasks are distinct states.
- Error: Malformed/ambiguous selector, unsupported origin, bounded subprocess failure, validation 400, unauthorized 401, role 403, foreign 404, stale/closed/archived/missing-scope 409 and capacity 429.
- Blocked: Missing selected view prevents a new operation until an explicit scope change; unavailable Git metadata is never clean/empty proof.
- No-access: Every API/RLS path rechecks current Brain/account/device; private checkout/task data is not exposed to other Brain members.
- Duplicate or replay: Refresh upserts device/root/path observations and normalized origins; aliases never silently merge identities.
- Stale data: CWD and defaults cannot relabel existing tasks/children/operations. Base-scope conflicts preserve both histories.
- Reconciliation divergence: Partial scans preserve unobserved registrations; complete scans mark absent paths without deleting history. Lost responses can be reconciled through catalogue/task history.

## Integrations and Runtime Inputs

- Providers: Installed Git, Serde/TOML, existing SQLx/Axum and native paired client.
- Environment: Existing RECOLLECT_URL and RECOLLECT_DEVICE_PROFILE; one-field workspace.toml contains no endpoint or secret.
- Secrets: Drop origin credentials before output/persistence; suppress config contents and Git stderr in errors. No source files or credentials are uploaded.
- Failure handling: Contract limits bound scans, subprocess output/time, registry sizes, request bodies and task histories.

## Tests and Acceptance

- Automated: Real Git/worktree discovery fixture, API/RLS catalogue/task concurrency and revocation scenario, native paired refresh and browser scope/history workflow.
- Manual: Inspect desktop/mobile workspace/task screens and exact runtime call results; preserve the normal user's data.
- Acceptance: Contract behavior and meaningful negative/positive cases pass with actual integrations, frontend/Rust checks and ./scripts/validate.sh.

## Closeout

- Planned: Local workspace catalogue and immutable per-operation task/subagent scope with visible handoff.
- Shipped: Bounded read-only Git/worktree discovery, nearest/nested selectors, normalized repository/alias identities, private checkout registration, task/child scope snapshots, immutable operation bindings, fresh-context handoff and browser/CLI flows.
- Not shipped: N/A within this slice. Exact extraction/publication, host capture, retrieval and MCP execution remain named successors and are not claimed by bindings.
- New blockers: None.
- Docs updated: Workspace contract, [proof mapping](../../../mappings/workspace-scope-proof-2026-09-14.md), [operator runbook](../../../runbooks/workspace-scope.md), local development, README and reconciled epic/execution indexes.
- Validation: Two protocol/discovery tests, eight core API/database scenarios, dedicated real OIDC proof, five existing browser workflows plus the final expanded workspace/native workflow passed. Inspected desktop/mobile screens; 60 unique API operations, frontend build, Clippy with warnings denied, rustfmt, governance lint and all 32 checker tests passed. Native workers drained persisted queues; normal startup/migration, readiness and owner sign-in were verified. External deployment remains out of scope.
- Version: N/A: no release or strict versioning.
- Commit: uncommitted.
