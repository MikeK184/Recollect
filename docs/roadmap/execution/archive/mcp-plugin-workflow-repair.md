# Plugin workflow repair

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Normal DLAG memory use needs supported tools rather than guessed internal paths; rejected learning results recover within bounds and management controls are visible.
- Non-goals: Customer infrastructure changes, unlimited model retries, releases.
- Delivery shape: Rust core/native plugin, desktop UI, compatible local migration and installed-host validation.

## Governing Sources

- [MCP tools](../../../contracts/mcp-memory-and-workspace-tools.md)
- [Direct authentication](../../../contracts/mcp-plugin-direct-auth.md)
- [Autonomous maintenance](../../../contracts/memory-autonomous-maintenance.md)
- [Desktop](../../../contracts/desktop-experience.md)
- [Session memory](../../../contracts/mcp-plugin-session-memory.md)

## Scope

- In scope: Source import/list/inspect tools and skills, source scope persistence, usage/host tracking, result validation/recovery, visible processing budgets and compact access tokens.
- Out of scope: Customer document edits, authority widening, benchmark work, unrelated MCP configuration.
- Blockers: None.

## Surface and Interface Changes

- Interfaces: Three canonical MCP source tools; optional bound operation on source input; observed host metadata; existing policy UI controls.
- Storage: Add version-specific import scope and metadata-only device Brain usage; retain old usage evidence and unscoped source compatibility.
- Ownership: MCP coordinates existing evidence/memory handlers; learning owns publication; devices owns usage; desktop renders canonical reads/commands.

## Data and Authority

- Inputs: Authorized UTF-8 evidence, bound operations, canonical policy, real host metadata.
- Authority: Brain membership/write grants, device ownership, exact operation actor/device scope.
- Blind spots: No reconstruction of absent historical host telemetry or guarantee of model quality.

## States and Edge Cases

- Loading: Existing query/loading states and bounded native errors.
- Empty: Explicit no-results and no-credentials states.
- Error: Invalid generated candidates never publish; bounded separately accounted recovery.
- Input/budget: Whole reconciliation inputs fit the remaining byte allowance; daily budget rejection resumes after the next UTC reset without overriding the cap.
- Blocked: Unknown provider completion is not replayed.
- No-access: Scope and role denials; own tokens only.
- Duplicate or replay: Source mutations retain canonical command idempotency.
- Stale data: Current source/policy/lease fences suppress obsolete learning.
- Reconciliation divergence: Actual observed host takes precedence over credential setup hints.

## Integrations and Runtime Inputs

- Providers: Existing installed OpenAI provider and owned deterministic proof fixtures.
- Environment: Existing Compose and OS credential store; no new secret variable.
- Secrets: No tokens or customer text in proof output.
- Failure handling: Existing 5/30-minute retry backoff and two-replacement maximum; no uncertain mutation replay.

## Tests and Acceptance

- Automated: Focused Rust/source scope and recovery tests, desktop build, governance validation and diff checks.
- Manual: Local readiness, visible UI controls, installed native catalogue/workflow and DLAG recovery observations.
- Acceptance: Scoped tools reject widening; memory/workspace calls appear in roster; actual hosts are readable; limits save canonically; compact credentials preserve revoke/history; no internal-path guessing in bundled guidance.

## Closeout

- Planned: All scoped deliverables above.
- Shipped: Canonical scoped source tools and bundled memory guidance; version-specific
  applicability; payload-free successful Brain usage and actual observed hosts;
  bounded invalid-result/daily-budget recovery, fitting whole reconciliation inputs;
  visible processing limits and compact searchable/history-aware access tokens.
  Fresh-import semantic admission uses the unique identity lookup within the
  existing two-second deadline. Local Compose and installed compatible stdio
  workflow are verified; successful native Codex automatic capture is retained.
- Not shipped: Paid completion of budget-blocked DLAG learning and automatic
  OpenCode capture retest. The new macOS binary is built/tested but its local
  Keychain trust was not established; the previous trusted installed runtime
  was restored and works with updated skills and the server catalogue.
- New blockers: None.
- Docs updated: Governing amendments, runbooks, plugin skill/README, owner,
  active/archive/epic/mapping indexes and
  [dated evidence](../../../mappings/plugin-workflow-repair-2026-10-07.md).
- Validation: Server/agent libraries, real SDK scoped source/idempotency/host
  tests, invalid-result and input/budget recovery fixtures, capture reconciliation,
  existing roster/cold-start/retry proofs; 55/56 broad model checks initially
  passed, and the fresh-import query fix then passed all 19 semantic regressions.
  Web type/design/build, API generation, formatting, workspace Clippy with
  warnings denied, governance and whitespace
  checks passed. Browser canonical Save/Cancel and token search/history/revoke
  cancel passed. Installed 26-tool source list/inspect and normal Compose
  readiness passed. Live DLAG failures remain budget-blocked, not falsely recovered.
- Version: N/A; no release requested.
- Commit: Uncommitted.
