# Vault delivery and authenticated private runners

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Authorized private execution with Vault delivery, renewal, rotation,
  expiry and reliable recovery.
- Non-goals: Customer deployment, secret store, mobile, tool replay and successors.
- Delivery shape: Shared adapter, paired runner, additive registration schema,
  desktop/native configuration and actual service proof.

## Governing Sources

- [ADR 0009](../../../adr/0009-vault-and-private-execution.md)
- [Vault/private contract](../../../contracts/mcp-vault-and-private-runners.md)
- [Runtime contract](../../../contracts/mcp-runtime-and-credentials.md)
- [Device authority](../../../contracts/platform-device-pairing.md)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: Registered private execution, current authority/RLS, Proxy adapter,
  coherent static/dynamic fields, renewal/rotation/draining and UI/native proof.
- Out of scope: Named successors, contract deferrals and changes outside the
  user's explicitly authorized new Enterprise test namespace.
- Blockers: None; routine decisions resolved before dependent implementation.

## Surface and Interface Changes

- Interfaces: Registration APIs, optional private runner selector, credential
  authorization, private-runner command and Vault references.
- Storage: Additive migration 023; existing calls/instances/outbox reused.
- Ownership: Brain owns registration, paired device owns epoch, Vault owns
  authentication/leases, executor holds secret values in memory.

## Data and Authority

- Inputs: Approved profile/connection, immutable scope, fixed socket/path references.
- Authority: Current caller Use and exact host/Brain/device checks at all boundaries.
- Blind spots: Grant revocation does not undo external effects or issued secrets.

## States and Edge Cases

- Loading: Queued/offline, startup, credential acquisition and active work.
- Empty: No paired devices, private registrations or credential references.
- Error: Safe missing/invalid/unavailable/expired credential and protocol codes.
- Blocked: Current grant, device, registration, Vault or fixed target unavailable.
- No-access: Cross-Brain and wrong-device API/RLS denial with positive controls.
- Duplicate or replay: Existing idempotency and receipt-only upload; no tool replay.
- Stale data: Revision conflicts and credential change/expiry drain old instances.
- Reconciliation divergence: Unknown attempts retain distinct later observations.

## Integrations and Runtime Inputs

- Providers: Official Vault Proxy/API, the user-authorized Enterprise namespace,
  owned fixtures and existing Reqwest/rmcp/PostgreSQL adapters.
- Environment: Companion endpoint/profile and operator credential-reference file;
  optional fixture Vault executable path, no secret values in source.
- Secrets: Exact redaction; no raw Vault logs, tokens or lease IDs in app output.
- Failure handling: Bounded control/Vault I/O, expiry cancellation, stopped renewal
  and natural TTL expiry, receipt recovery without fallback execution. Never
  revoke Vault tokens or delete the authorized namespace/auth mount.

## Tests and Acceptance

- Automated: API/RLS controls, real Vault/Proxy issuance/renewal/rotation, private
  stdio/HTTP/native/desktop flows and affected runtime/workspace/governance checks.
- Manual: Inspect desktop and compare normal data around migration.
- Acceptance: Full contract proof in the
  [mapping](../../../mappings/mcp-vault-private-2026-09-22.md) before archive.

## Closeout

- Planned: Vault credential lifecycle and authenticated private execution.
- Shipped: Optional per-connection Vault Proxy delivery, coherent KV/dynamic
  credentials, current-authority renewal and expiry/draining; exact registered
  private execution through the shared coordinator, SDK, native CLI and desktop.
  Additive normal migration 023 preserves existing Brains, profile and grants.
- Not shipped: Memory/workspace MCP tools and managed observation capture belong
  to the named successor slices. Enterprise database issuance was unreachable
  from that cluster; actual dynamic proof uses owned local Vault/PostgreSQL.
- New blockers: None.
- Docs updated: ADR, contract, mapping, operating runbook, handoff, README, pack,
  epic and execution/roadmap indexes.
- Validation: 17 database/API/SDK cases; actual native private stdio and HTTP;
  three real Vault cases; six credential boundary and nine transport cases;
  existing catalogue desktop controls and new private registration flow;
  20 workspace tests, Clippy, generated API/web build and 32 governance tests.
  Normal readiness, renewed central lease, empty test runtime and exact data
  preservation pass; desktop SWEG recall returns one item/1,630 bytes/62 ms
  without new model calls or customer file reads. Root token is absent from
  API/worker environments and the restored Enterprise token helper still matches.
  See the mapping for logs, incident history and evidence boundaries.
- Version: N/A: no release policy.
- Commit: Uncommitted.
