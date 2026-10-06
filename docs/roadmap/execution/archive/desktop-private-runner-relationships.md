# Private Runner relationship cards

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Implement approved `view-v2.png` with device, assigned MCPs/tools and group links.
- Non-goals: New execution, grants, credentials, OS/location telemetry or runner rebinding.
- Delivery shape: Typed Rust read projection, React cards, local runtime and independent UI review.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md#private-runner-relationship-cards--2026-10-06)
- [Catalogue authority](../../../contracts/mcp-catalogue-and-profiles.md#brain-connections-and-profiles)
- [Private runner contract](../../../contracts/mcp-vault-and-private-runners.md)
- [Owning epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Compact read/Edit layout, canonical private runner UUID in existing
  visible summaries, approved metadata reads, specific connection/group links.
- Out of scope: Broader metadata exposure or changes to Use/Manage/Share/admission.
- Blockers: None; user approved the relationship design and independent review.

## Surface and Interface Changes

- Interfaces: Nullable backward-compatible `private_runner_id` in summaries;
  validated UUID-only `connection`/`profile` UI search keys select linked resources.
- Storage: N/A; derive from existing immutable binding, no migration.
- Ownership: Protocol/server own binding projection; desktop owns its read presentation.

## Data and Authority

- Inputs: Current runner/device inventory, filtered catalogue/workspace environments,
  admin definition or Use-gated cached discovery, visible group memberships.
- Authority: Existing Brain/catalogue filtering and definition/discovery checks unchanged.
- Blind spots: Device inventory is current-account only; use UUID fallback for others.
  Approved metadata is not proof of installed/reachable tools. Missing old-server binding
  means unavailable relationship information, never an empty assignment.

## States and Edge Cases

- Loading: Label catalogue, environment and tool reads; counts only after success.
- Empty: Admin No assigned connections; reader No connections visible to you.
- Error: Hide failed-refresh stale data; explicit retry over canonical reads.
- Blocked: Disabled connection/group and archived Brain retain truthful metadata state.
- No-access: No admin detail reads for readers; no Use means no discovery/tool count.
- Duplicate or replay: Existing idempotent runner saves unchanged; reads start no tools.
- Stale data: Scope keys include Brain/resource revisions and rights; poll existing cadence.
- Reconciliation divergence: Preserve original draft revision conflict and failed-save retry.

## Integrations and Runtime Inputs

- Providers: Existing API only; no external connection/provider call for card display.
- Environment: Existing local stack variables; no new configuration.
- Secrets: No credentials/targets/configuration in projection, URLs, tests or screenshots.
- Failure handling: Bounded transient metadata retries; denied/invalid reads never auto-retry.

## Tests and Acceptance

- Automated: Projection unit coverage and focused browser/data-state checks,
  web type/design/build, required governance32, whitespace and CodeGraph.
- Manual: Actual SWEG Ubuntu metadata and reader access, links, read/Edit/Add/Cancel,
  laptop/wide captures and independent review against approved concept.
- Acceptance: Exact runner associations; no incorrect zero/stale/permission counts;
  compact readable card and stable pending/revision/editor behavior; ready local runtime.

## Closeout

- Planned: Approved relationship cards and independent rendered acceptance.
- Shipped: Approved cards, canonical UUID summaries, shared authorized metadata reads,
  exact connection/group links and truthful data states, live on the local stack.
- Not shipped: New execution/grants/telemetry remain outside scope.
- New blockers: None.
- Docs updated: Contracts, epic/index, archived pack/index, mapping/index and handoff.
- Validation: Two projection/compatibility Rust tests, production web type/design/build,
  13 CUA fixture checks, actual owner/reader API and browser reads, exact links,
  Add/Edit/Cancel, laptop/wide geometry and independent source/rendered review,
  readiness, governance32, CodeGraph and whitespace passed.
- Evidence: [Dated acceptance](../../../mappings/private-runner-relationships-2026-10-06.md).
- Version: N/A; no release policy.
- Commit: Uncommitted.
