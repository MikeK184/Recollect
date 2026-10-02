# Plugin-managed automatic memory and optional runner

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Install/connect a plugin once, then automatically capture and recall during
  ordinary Codex, Claude Code and OpenCode sessions; preserve existing functionality.
- Non-goals: Public marketplace submission, OAuth redesign, unrelated engine work.
- Delivery shape: Bundled native runtime, host adapters, desktop setup, migration
  documentation and actual installed-host proof; deployment where server/UI changes require it.

## Governing Sources

[ADR 0018](../../../adr/0018-plugin-managed-agent-memory.md),
[plugin contract](../../../contracts/mcp-plugin-session-memory.md),
[capture](../../../contracts/evidence-session-capture.md),
[scope](../../../contracts/evidence-workspace-scope.md),
[runner](../../../contracts/mcp-vault-and-private-runners.md),
[owning epic](../../epics/mcp-coordination.md).

## Scope

- In scope: Plugin packaging/auth, lifecycle, capture/recall, compaction, queue,
  workspace functionality, scope/privacy guarantees, optional runner flag and migration.
- Out of scope: Paid benchmark, public publication and unrelated UI polish.
- Blockers: None. Native external interface compatibility is verified before adapter implementation.

## Surface and Interface Changes

- Interfaces: Bundled connect/status/hook/MCP/drain runtime; `--with-runner` and
  optional `--runner-id`; native host manifests/hooks; Agents setup.
- Storage: Private portable plugin data, session identities and existing durable
  sanitized capture inbox. Migration 030 admits OpenCode as a host producer under the same device/operation/RLS constraints. No new memory database or raw-transcript store.
- Ownership: Agent crate/plugin adapters own local behavior; canonical server
  handlers own knowledge, permissions, processing and execution.

## Data and Authority

- Inputs: Documented host events, selected Brain, canonical task scope and recalled evidence.
- Authority: Existing user credential/current grants and standing Brain policies.
- Blind spots: Host event coverage differs; record gaps and verify exact installed versions.

## States and Edge Cases

- Loading: Bounded startup and prompt recall; coding proceeds on timeout.
- Empty: Connected Brain with no relevant memory produces no invented context.
- Error: Safe diagnostic/status, retained queue and bounded retry.
- Blocked: Missing hook trust or account authorization is explicit.
- No-access: Stop recall/publication, apply deletion/revocation rules; never change destinations.
- Duplicate or replay: Stable event IDs and existing canonical idempotency.
- Stale data: Refresh grants/policy/privacy before upload and current scope before new turns.
- Reconciliation divergence: Old turns retain old scope; later changes never relabel evidence.

## Integrations and Runtime Inputs

- Providers: Existing Recollect HTTP/MCP; native Codex/Claude/OpenCode adapters.
- Environment: `RECOLLECT_PLUGIN_DATA` for explicit data placement; existing trusted CA settings.
- Secrets: OS credential store or transient standard input; no secret-bearing plugin configuration.
- Failure handling: Eight-second recall budget, short capture commit, independent bounded delivery.

## Tests and Acceptance

- Automated: Packaged runtime/adapter tests, positive and negative authority/recovery
  cases, actual host launch fixtures and affected browser checks; full required validation.
- Manual: Normal owner connect flow and laptop/desktop visual pass where changed.
- Acceptance: Every item in the plugin contract, including fresh-session automatic
  recall, OpenCode capture, optional runner execution and preserved original inventory.

## Closeout

- Planned: Complete plugin integration replacing separately operated companion memory setup while preserving existing capabilities and optional independent execution.
- Shipped: Bundled native runtime and Enola extractor; installed Codex/Claude/OpenCode adapters; connect-once OS credentials; automatic bounded cited recall, compaction retrieval and permitted capture; canonical learning after session end; durable original-authority delivery, privacy fences and session/child/scope attribution; explicit `--with-runner` execution and disablement; offline read-only checkout discovery and publication; legacy conflict detection and migration instructions; Agents setup. The final server/UI are deployed locally on migration 030 with original inventory preserved.
- Not shipped: Public marketplace publication, OAuth redesign and paid quality/capacity benchmarks are explicit non-goals. Windows and native host-initiated compaction are not claimed tested. Personal host configuration was preserved; fresh installs were proved in owned host homes. No contract implementation remains open.
- New blockers: None for this slice. The separate LongMemEval protocol/cost decision remains outside scope.
- Docs updated: ADR 0018, plugin/capture/direct-auth contracts, foundation amendments, README, package and setup/capture/pairing/workspace/publication/development runbooks, [final evidence](../../../mappings/plugin-session-memory-2026-10-01.md), owning epic, epic/execution/mapping indexes and CONTINUE_HERE.md.
- Validation: Final Linux native run passed four compatibility and eight plugin cases; macOS Codex 0.159.3 and OpenCode 2.0.21 installed-package proofs passed, including bundled Enola publication. Workspace tests passed 46 cases, final agent tests 25, strict workspace/all-target clippy passed. Broad platform checks passed 138 initially, with all six failures passing focused reruns after supplying SFTP and removing competing workloads; failure evidence is retained. Both Brain deletion API cases passed. Affected browser checks passed six cases with one optional live Context7 case skipped; frontend typecheck/design/build passed. Owner visuals passed at 1280/1440/1920 on fixtures and deployed setup. Local Compose upgrade/readiness and inventory comparison passed. Governance validation and diff hygiene passed; exact commands, logs and limitations are in the evidence mapping.
- Version: N/A; no release policy or publication requested. Native artifact: `.cache/plugin-dist/darwin-arm64-final-20261002-c`.
- Commit: Uncommitted; no commit or push performed.
