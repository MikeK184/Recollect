# Authorized MCP execution and credential delivery

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Execute approved central/local MCP calls with isolated credentials,
  compatible instance reuse, durable outcomes and honest interruption recovery.
- Non-goals: Vault/private transport, memory-tool server, observation enrichment,
  arbitrary shell registration, automatic effectful retry and mobile views.
- Delivery shape: Shared Rust SDK runtime, database call/runner authority, paired
  local executor, native CLI and desktop execution/inspection workflow.

## Governing Sources

- [Managed execution ADR](../../../adr/0008-managed-mcp-runtime.md)
- [Runtime contract](../../../contracts/mcp-runtime-and-credentials.md)
- [Catalogue contract](../../../contracts/mcp-catalogue-and-profiles.md)
- [Device authority](../../../contracts/platform-device-pairing.md)
- [Immutable scope](../../../contracts/evidence-workspace-scope.md)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: Actual rmcp transports, environment/OS-store credential references,
  two-stage authorization, durable calls, startup/operation leases, idle/draining,
  cancellation/unknown completion and receipt/evidence reconciliation, desktop/CLI.
- Out of scope: Customer external writes, deployment, releases and named successors.
- Blockers: None; accepted ADR/contract resolve routine decisions before code.

## Surface and Interface Changes

- Interfaces: Brain call/runtime routes, authenticated runner protocol, operator
  credential-reference file, definition receipt policies and native execution CLI.
- Storage: New additive migration for call dispositions, minimal attempts/audit,
  runner leases and instance metadata; bounded expiring operational payloads.
- Ownership: Server owns authority and durable state; shared runtime owns SDK/I/O;
  companion owns local processes and private receipt outbox, never database access.

## Data and Authority

- Inputs: Approved definition/connection/profile, independent Use, immutable scope,
  exact runner and server-issued attempt; validated bounded JSON tool arguments.
- Authority: Existing current account/Brain/device/profile checks at admission and
  pre-send; exact attempt fencing on completion. Caller session is isolation only.
- Blind spots: A tool response is a reported observation, not universal effect
  verification; timeout/cancellation can leave unknown external completion.

## States and Edge Cases

- Loading: Queued, starting and running, with active leases and selected runner.
- Empty: No calls/instances, absent approved tools and expired output explained.
- Error: Stable non-secret schema/provider/transport/credential codes.
- Blocked: Missing Use/runner/credentials, incompatible config or unknown completion.
- No-access: Current authority gates dispatch/results; observed loss clears UI.
- Duplicate or replay: Same request identity yields existing disposition, changed
  input conflicts, no automatic tool replay or stale receipt overwrite.
- Stale data: Frozen call scope/configuration; incompatible instances drain.
- Reconciliation divergence: Preserve unknown attempt plus separately attributed
  receipt/evidence resolution; never infer rollback from cancellation.

## Integrations and Runtime Inputs

- Providers: Official rmcp, Tokio/process wrappers, Reqwest, PostgreSQL, existing
  paired OS store and shared redaction; synthetic owned integration fixtures.
- Environment: Existing local stack plus optional credential-reference file.
- Secrets: Resolve solely on selected runner; cleared child environment and exact
  value redaction; no values in approved metadata, logs or model arguments.
- Failure handling: Bounded startup/calls/I/O/capacity, fenced heartbeat/recovery,
  receipt-only upload retries, owned cleanup and no hosted-process termination.

## Tests and Acceptance

- Automated: Actual central/local SDK calls, direct/RLS authorization, concurrent
  startup/isolation, long-call idle behavior, revocation/startup ordering, process
  death, lease fencing, uncertain side effects and receipt reconciliation; desktop
  execution/error/revocation and native runner flow. Appropriate workspace checks.
- Manual: Inspect desktop runtime views and normal-data preservation; no customer
  provider writes or paid model calls are needed by the synthetic fixtures.
- Acceptance: Full [contract](../../../contracts/mcp-runtime-and-credentials.md)
  evidence in the [mapping](../../../mappings/mcp-runtime-2026-09-22.md) before archive.

## Closeout

- Planned: Complete central/local execution, credentials, runtime lifecycle and recovery.
- Shipped: Central/shared/native execution, isolated credentials, compatible
  lifecycle and private receipt outbox, evidence/receipt reconciliation, desktop
  workflow and paired CLI. Local schema 022 upgrade preserves existing data.
- Not shipped: Named Vault/private, memory/workspace and observation successors.
- New blockers: None.
- Docs updated: ADR, contract, epic, archived pack, mapping, runbook, README,
  continuation handoff and lifecycle indexes.
- Validation: Nine actual SDK cases; six combined central/paired execution cases
  and ten catalogue/coordinator/recovery cases; four affected desktop flows and
  visual inspection; 17 workspace tests, Clippy and all 32 governance tests pass.
  Normal schema 022 is ready with seven Brains, the existing profile/grants and
  33 model requests preserved; exact SWEG recall and desktop checks pass without
  new paid calls or customer reads. The linked mapping records failures and fixes.
- Version: N/A: no release policy.
- Commit: Uncommitted.
