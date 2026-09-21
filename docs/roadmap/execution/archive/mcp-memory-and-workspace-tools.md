# Memory/workspace MCP tools and coding-host integration

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Expose existing scoped memory/workspace and managed tools through real MCP.
- Non-goals: Successor observation capture, mobile, deployment and another memory engine.
- Delivery shape: Stateless paired HTTP service, native stdio bridge, generated
  host settings, fresh context and existing desktop integration guidance.

## Governing Sources

- [ADR 0010](../../../adr/0010-agent-memory-mcp.md)
- [Tools contract](../../../contracts/mcp-memory-and-workspace-tools.md)
- [Workspace](../../../contracts/evidence-workspace-scope.md)
- [Capture](../../../contracts/evidence-session-capture.md)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: Complete contracted catalogue, current authority, immutable operation
  scope, canonical reads/writes, fresh context, host capture defaults and proof.
- Out of scope: Explicit contract deferrals and unrelated configuration/state.
- Blockers: None. Approved Docker recovery and fresh isolated native/host proof
  resolved the earlier runtime blockers; normal inventory remains preserved.

## Surface and Interface Changes

- Interfaces: Brain-addressed MCP HTTP, stdio helper, host settings and optional
  expected-scope check on the existing operation-start API.
- Storage: Reuse canonical/operation/call records; additive local inbox future-default
  mapping. No new central memory writer or authentication-session scope.
- Ownership: Brain holds data, account/device owns tasks, operations freeze scope,
  host launch owns its future capture default, existing runner owns execution.

## Data and Authority

- Inputs: Fixed launch destination and typed tool arguments/immutable operation IDs.
- Authority: Fresh paired and Brain checks; write, scoped read and profile Use
  enforced by shared handlers. Human-review mutations remain browser-only.
- Blind spots: Delivered host context cannot be retracted; unsupported host
  attribution remains an explicit gap. No claim of deployment or model truth.

## States and Edge Cases

- Loading: Connection, context refresh, queued managed call and handover.
- Empty: No workspace/repositories, memory matches, profiles or permitted tools.
- Error: Invalid schema, bounded payload, offline service and safe tool errors.
- Blocked: Missing pairing, changed/closed scope, model policy or current grants.
- No-access: Wrong Brain/device/kind and absent independent Use with positive controls.
- Duplicate or replay: Existing command IDs and receipts; never replay uncertain writes.
- Stale data: Expected-scope check, current canonical/retention and fresh discovery.
- Reconciliation divergence: Committed scope change reports failed refresh separately;
  original host turns and uncertain managed outcomes retain their original meaning.

## Integrations and Runtime Inputs

- Providers: rmcp 3.4.0, current Codex/Claude interfaces and existing owned fixtures.
- Environment: Fixed endpoint/profile/Brain/directory or original capture setup.
- Secrets: OS-store only for bridge auth; no values in schemas/config/protocol logs.
- Failure handling: Bounded I/O, safe status, no mutation retry or scope fallback.

## Tests and Acceptance

- Automated: Contracted real HTTP/stdio/API/host flows and meaningful scope,
  rejection/erasure/revocation, concurrent task, capture and failure controls.
- Manual: Desktop inspection and normal preservation without fixture import.
- Acceptance: Full contract and dated mapping proof before archival.

## Closeout

- Planned: Complete agent memory/workspace MCP and host integration.
- Shipped: Stateless paired HTTP MCP and native bridge, fixed secret-free host
  settings, complete scoped catalogue, fresh context with explicit failure,
  launch-local capture defaults and desktop guidance. Linux Codex retains its
  Secret Service session through explicit variable-name forwarding.
- Not shipped: Named managed-observation successor and contract deferrals remain
  separate. The ordinary running native service remains its slice-24 build;
  rebuild/restart is distinct from source and isolated runtime delivery.
- New blockers: None. The full product goal remains incomplete.
- Docs updated: ADR, contract, pack, epic, mapping, runbook and lifecycle indexes.
- Validation: Actual Codex/Claude and Linux native bridge pass with Secret Service;
  real managed calls, exactly-once observed effects/receipt lookup, cancellation,
  capture failure, old/child scopes, current authority and unavailable dependency
  controls pass. HTTP/policy, handover pagination/synthesis and native graph paths,
  scoped history, committed scope with unavailable/recovered context, both desktop
  flows and normal preservation pass. Workspace 25 passed/143 explicitly skipped;
  focused integrations run separately. Clippy, 156-operation API/web build,
  governance and diff checks pass. See the [dated mapping](../../../mappings/mcp-tools-2026-09-22.md)
  and [runbook](../../../runbooks/agent-memory-tools.md) for evidence and limits.
- Version: N/A: no release policy.
- Commit: Uncommitted.
