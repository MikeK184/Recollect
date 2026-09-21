# Automatic managed-tool observation capture

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Managed outcomes automatically become attributable permitted evidence.
- Non-goals: Automatic effect execution, universal transcripts, external deployment.
- Delivery shape: Runtime/capture bridge, durable publication, desktop/native state.

## Governing Sources

- [ADR 0012](../../../adr/0012-managed-tool-observations.md)
- [Observation contract](../../../contracts/mcp-observation-capture.md)
- [Session capture](../../../contracts/evidence-session-capture.md)
- [Managed runtime](../../../contracts/mcp-runtime-and-credentials.md)
- [Vault/private execution](../../../contracts/mcp-vault-and-private-runners.md)
- [Retention](../../../contracts/memory-retention-and-erasure.md)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: Full contracted producer, publication/retention/replay and user surfaces.
- Out of scope: Recovery/evaluation successor slices and unrelated services.
- Blockers: None; both predecessor slices shipped and routine decisions resolved.

## Surface and Interface Changes

- Interfaces: Standing `managed_tools` policy, bounded observations GET, call status,
  runner receipt-removal synchronization and desktop capture/source navigation.
- Storage: Migration 024; managed bindings, admitted policy/target snapshots and
  bounded publication outbox, with existing source/event/privacy identities reused.
- Ownership: Runtime authenticates receipts; capture publishes evidence; canonical
  memory, retrieval, model policy and privacy retain their existing authority.

## Data and Authority

- Inputs: Canonical call/receipt/resolution plus admitted/current policy and scope.
- Authority: Original actor and target records, current grants and privacy fences.
- Blind spots: Unreceived expired receipts, disconnected runner deletion and
  encoded/unknown secrets retain explicit coverage limits.

## States and Edge Cases

- Loading: Awaiting outcome, queued publication and source processing.
- Empty: No calls or managed capture disabled; no implicit retroactive import.
- Error: Safe publication error/backoff or outcome missing without invented output.
- Blocked: Current policy, access, expiry, deletion, excluded content or capacity.
- No-access: Current call/profile and Brain boundaries; no forged internal producer.
- Duplicate or replay: Stable call/resolution IDs; no duplicate source or effect.
- Stale data: Frozen original scope/target, current admission and content deadlines.
- Reconciliation divergence: Original unknown persists beside later evidence;
  erasure fences suppress pending and late copies through older-state replay.

## Integrations and Runtime Inputs

- Providers: Existing actual rmcp fixtures and PostgreSQL/Neo4j; no new engine.
- Environment: Existing explicit fixture inputs; no inherited root Vault token.
- Secrets: Shared sanitizer before added capture persistence; no content in logs.
- Failure handling: Bounded durable publication retries, existing receipt bound,
  no automatic provider/effect replay and no Vault revocation.

## Tests and Acceptance

- Automated: Contract's actual runtime, scope, learning, failure and privacy cases;
  existing host capture remains a separate compatible producer.
- Manual: Desktop inspection and normal-data preservation.
- Acceptance: Full contracted behavior, focused integration plus required checks;
  configuration alone cannot establish publication.

## Closeout

- Planned: Complete managed observation capture with privacy and user surfaces.
- Shipped: Standing managed capture, canonical terminal/reconciliation provenance,
  bounded durable publication, scope/authority enforcement, autonomous learning,
  retention/erasure/native receipt synchronization and desktop/native state.
- Not shipped: The contract's explicit deferrals; full backup/recovery drills and
  integrated corpus/capacity evaluation remain the named operational successors.
- New blockers: None currently established.
- Docs updated: ADR, contract, pack, epic, indexes, dated mapping, runtime runbook
  and continuation record.
- Validation: Five actual central/local/private observation scenarios, two restart
  receipt cases, nine runtime regressions and five host-capture regressions pass.
  A sanitizer unit covers full-envelope exclusions, configured secrets and UTF-8
  limits. Four desktop cases pass after fixing nested Escape handling; screenshots
  inspected. Workspace 26 passed/149 fixture cases ignored, all-target Clippy,
  159-operation API generation, TypeScript/web build and 32 governance tests pass.
  Normal migration 024/readiness/desktop checks preserve seven Brains, 33 model
  requests and existing profiles/grants; actual SWEG exact recall returns one item,
  1,630 bytes in 50 ms without new model requests or customer repository reads.
  Evidence and limits are in the [dated mapping](../../../mappings/mcp-observations-2026-09-26.md).
- Version: N/A: no release policy.
- Commit: Uncommitted.
