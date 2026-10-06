# Ubuntu private runner and filesystem proof

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Prove a coding-agent MCP caller can read an isolated Ubuntu folder through Recollect's real private runner.
- Non-goals: Customer filesystem access, arbitrary shell execution, existing grant changes, paid model acceptance or release.
- Delivery shape: Owned local Docker fixture, canonical demo resources and dated runtime evidence.

## Governing Sources

- [Private runner contract](../../../contracts/mcp-vault-and-private-runners.md).
- [Runtime contract](../../../contracts/mcp-runtime-and-credentials.md).
- [Agent MCP contract](../../../contracts/mcp-memory-and-workspace-tools.md).
- [Catalogue contract](../../../contracts/mcp-catalogue-and-profiles.md).
- [Owning epic](../../epics/mcp-coordination.md).

## Scope

- In scope: Ubuntu 24.04 native runner, official filesystem stdio MCP, demo directory, explicit new connection/profile/Use grants on SWEG test, agent-side requests and denial/recovery checks.
- Out of scope: Host folders, Docker socket access, unrelated containers/resources, wider permissions and existing native-host acceptance.
- Blockers: None; accepted runtime and authority contracts cover the test.

## Surface and Interface Changes

- Interfaces: Existing canonical pairing, registration, catalogue, connection/profile and agent MCP APIs; fixture lifecycle scripts only.
- Storage: N/A: existing schema and revisions; new bounded demo records only.
- Ownership: Repo-owned fixture; runtime OS store and receipt directory; no external checkout changes.

## Data and Authority

- Inputs: Official filesystem descriptors, synthetic marker and current SWEG resources.
- Authority: Installation owner approves definition; Brain admin registers own paired device; independent Use permits caller; fixed approved executable/argv and directory.
- Blind spots: Protocol proof does not establish an OpenCode LLM's tool selection unless an actual native host run is separately recorded.

## States and Edge Cases

- Loading: Wait for an actual runner lease and terminal call result with bounded deadlines.
- Empty: Provision uniquely named demo resources; refuse collisions rather than replace unknown resources.
- Error: Retain bounded error codes; keep secrets out of output.
- Blocked: Failed pairing/build/runtime prevents connectivity claims.
- No-access: Denied caller and outside-directory path must fail; only approved read tools are exposed.
- Duplicate or replay: Stable call request IDs; never retry a dispatched tool effect.
- Stale data: Inspect current eligibility/availability and original call IDs after changes.
- Reconciliation divergence: Offline calls remain queued for this runner; no central fallback; resume only undispatched read calls.

## Integrations and Runtime Inputs

- Providers: Official `@modelcontextprotocol/server-filesystem` in owned Ubuntu; no incoming runner port, customer mounts or generic shell tool.
- Environment: RECOLLECT_URL, RECOLLECT_DEVICE_PROFILE and private-runner ID; separate caller credential.
- Secrets: Owner credential read from ignored environment; runner credential in Linux Secret Service; disposable unlock secret/private caller files ignored and mode 0600.
- Failure handling: Existing leases, heartbeat/reconnect and bounded polling; stop/restart only the named demo container.

## Tests and Acceptance

- Automated: Real agent MCP task/operation/call/status, marker returned, distinct device IDs, outside-root and no-Use denial, offline queue and recovery; scripts/validate.sh and whitespace.
- Manual: Inspect live Private Runners and Connections state in a task-owned browser tab.
- Acceptance: A real Ubuntu process returns its private synthetic file through Recollect to an independent caller, denials hold and offline work returns without central execution. Retained demo resources and exact stop command documented.

## Closeout

- Planned: Real isolated Ubuntu fixture and end-to-end proof with denials/recovery.
- Shipped: Real Ubuntu native runner and official filesystem MCP; separate reader caller, three approved read tools, successful private file return, outside-root/write/Use/Manage/Share denial, offline queue/recovery and fresh-container OS-store reuse. Retained named demo is Connected on the ready local installation; see [evidence](../../../mappings/private-runner-ubuntu-proof-2026-10-06.md).
- Not shipped: Customer network/filesystem access, arbitrary commands, write tools, native OpenCode LLM selection or production deployment; separate native-token acceptance remains active.
- New blockers: None.
- Docs updated: Fixture procedure, private-runner runbook, dated evidence/index, archived pack, owning epic/index, active/archive indexes and handoff.
- Validation: Native Linux build, real MCP success/denial/offline/fresh-container checks, live CUA Connected/connection/test-tools inspection, preparation rerun, Python compilation, shell syntax, governance32, CodeGraph and whitespace passed. Existing backend/frontend/schema untouched; no paid model acceptance.
- Version: N/A: no release requested.
- Commit: Uncommitted.
