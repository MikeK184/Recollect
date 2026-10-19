# Cross-machine agent coordination and A2A

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Mike's and Paul's agents in one Brain discover overlapping goals/planned
  files, receive push plus pull notices, and discuss work with visible peer identity.
- Non-goals: File locks, automatic merges, external-agent federation, arbitrary
  webhooks, mandatory agentgateway and paid model matching.
- Delivery shape: Local Rust controller/protocol/plugin, additive persistence,
  official A2A adapter, docs and synthetic macOS/Ubuntu-container proof.

## Governing Sources

- [ADR 0023](../../../adr/0023-agent-to-agent-coordination.md) and
  [coordination contract](../../../contracts/mcp-agent-coordination.md).
- [Workspace scope](../../../contracts/evidence-workspace-scope.md),
  [plugin lifecycle](../../../contracts/mcp-plugin-session-memory.md),
  [team access](../../../contracts/platform-team-access.md) and
  [MCP authority](../../../contracts/mcp-memory-and-workspace-tools.md).
- [Owning epic](../../epics/mcp-coordination.md).

## Scope

- In scope: Session projection, scoped presence/intentions, explainable matcher,
  durable recipient events/messages, negotiation, authenticated SSE/pull, MCP and
  native hooks/CLI, bounded A2A adapter, isolated Ubuntu fixture and failure proof.
- Out of scope: New source capture, raw task sharing, managed execution changes,
  desktop redesign, external callbacks/providers and public deployment.
- Blockers: None. Official A2A 1.0 message-only wire is fixed in the contract;
  dated research records the current specification and stale example limits.

## Surface and Interface Changes

- Interfaces: Contract-owned coordination DTOs and `/coordination` HTTP routes;
  `coordination.*` MCP tools; `recollect-plugin coordination` commands/watch;
  native hook context; narrow A2A skill/Agent Card.
- Storage: Additive normal migration 051 for Brain-scoped sessions, overlaps,
  messages/inbox and audit/RLS, including canonical Brain deletion. Existing
  unrelated dirty schema/code was preserved.
- Ownership: Protocol owns typed inputs/validation; server owns authority,
  matching, persistence and transports; plugin owns binding/local delivery and
  presentation; test fixtures own synthetic accounts and containers.

## Data and Authority

- Inputs: Current authenticated principal/device, private task scope, sanitized
  declared goals/topics and canonical repository-relative file intentions.
- Authority: Server-derived identity/scope, lease time, committed messages/inbox;
  peer text is untrusted advice and grants no execution authority.
- Blind spots: Undeclared work, unsupported hooks, paraphrases, shell-derived
  filenames and host mid-turn delivery; expose gaps rather than infer proof.

## States and Edge Cases

- Loading: Bounded announce/list/pull and visible connecting watcher state.
- Empty: No peers/matches/messages is a successful explicit empty state.
- Error: Typed validation/conflict/capacity/network errors; normal coding proceeds.
- Blocked: Unsupported host injection is reported as pull-only/degraded coverage.
- No-access: Foreign task/session/inbox and revoked account/device/Brain deny reads,
  writes and subsequent stream events.
- Duplicate or replay: Stable request/event UUIDs, optimistic revision, durable
  cursors and idempotent acknowledge prevent duplicate delivery/effects.
- Stale data: Server TTL expires presence; stale events retain timestamps and
  current peer state; cursor expiry explicitly reports lost retained history.
- Reconciliation divergence: Server receipt, local receipt, hook injection and
  peer agreement remain distinct; agreement is advisory and inspectable.

## Integrations and Runtime Inputs

- Providers: Existing PostgreSQL/Axum/rmcp/native plugin plus official A2A JSON
  schema. Model providers and agentgateway runtime: N/A, not required.
- Environment: Existing server/database/plugin settings; fixture-only test
  configuration under repository-owned paths. No new production secret required.
- Secrets: OS-store paired credentials; synthetic fixture credentials never
  enter committed docs/output; no arbitrary callback URLs or shell interpolation.
- Failure handling: Bounded hook requests, lease heartbeat, stream reconnect and
  inbox pull fallback; replay only stable idempotent coordination commands.

## Tests and Acceptance

- Automated: Contract matrix for matcher, path/identity validation, replay,
  concurrency, TTL, privacy/RLS/revocation, inbox restart/gaps, proposal replies,
  real HTTP/MCP/A2A and native hook presentation; regenerate relevant API schema.
- Manual: Observe macOS CLI and Ubuntu-container peer identity, push notice and
  negotiation using made-up test-Brain repositories/files. Verify actual host boundary
  output where installed; record any untested host explicitly.
- Acceptance: Every coordination-contract criterion has retained proof. Synthetic
  zero-model-call tests cover matching and delivery separately from live model use.
  Run focused checks, `git diff --check` and `./scripts/validate.sh`.

## Closeout

- Planned: Decision-complete architecture, official-source research, Rust
  controller/native integrations, A2A adapter and two-environment failure proof.
- Shipped: Rust Brain-scoped controller with authenticated leases, automatic
  checkout/goal and planned-file context, explainable matching, durable peer
  messages, SSE/pull, MCP, native hook/CLI/watch and the narrow official A2A 1.0
  message binding. Local deletion fences and revocation preserve privacy.
  The [dated proof](../../../mappings/agent-coordination-proof-2026-10-18.md)
  records the final macOS/Ubuntu 24.04 run and binary/fixture hashes.
- Not shipped: Full public A2A federation, gateway deployment, file locking,
  automatic edits and model-based matching are explicit exclusions.
- New blockers: None for delivered scope. Installed-model consumption, a second
  physical Mac and production deployment were not part of this synthetic proof.
- Docs updated: ADR 0023, coordination contract, owning epic and indexes;
  existing task/privacy/plugin/MCP authority, official-source research mapping,
  [runbook](../../../runbooks/agent-coordination.md) and dated proof.
- Validation: Final fixture `proof-c00339f9482e` passed all 15 checks and exited 0;
  canonical model requests 0; owned resources cleaned up. Four server matcher
  tests, comprehensive real HTTP/MCP/A2A database integration, 54 native tests
  (11 coordination), API generation (200 unique operations), web typecheck/design,
  `git diff --check` and `./scripts/validate.sh` (35 governance + 10 baseline tests)
  passed. Prior failed fixture/review cases and fixes remain in the proof record.
