# Compact inline Private Runners

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Approved compact cards with inline Add/Edit and truthful runner status.
- Non-goals: New runner execution, authority, credentials or device reassignment.
- Delivery shape: Frontend, documentation and existing local stack UI.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md#private-runner-cards-and-inline-editing--2026-10-06).
- [Private runners](../../../contracts/mcp-vault-and-private-runners.md).
- [Plugin session memory](../../../contracts/mcp-plugin-session-memory.md).
- [Owning epic](../../epics/desktop-visual-experience.md).

## Scope

- In scope: Replace the modal with compact inline cards, canonical save flow and copy setup command; preserve the existing query hook used by connection placement.
- Out of scope: Backend changes, runner startup, real private registration, new credentials or external publication.
- Blockers: None; user approved the generated view and inline-v2 concepts.

## Surface and Interface Changes

- Interfaces: Existing GET/POST/PUT private-runner routes and own GET devices only; no public type changes.
- Storage: N/A; existing canonical registrations and revisions.
- Ownership: McpPrivateRunners and scoped presentation CSS; existing UI acceptance selectors.

## Data and Authority

- Inputs: Canonical runner metadata, availability, eligibility and own active device inventory.
- Authority: Current Brain admin/open state, immutable device, original base_revision and server idempotency remain authoritative.
- Blind spots: Other accounts' device names are not exposed; UUID fallback. Registration does not prove a live worker.

## States and Edge Cases

- Loading: Label runner/device loading, disable unavailable creation inputs.
- Empty: Clear empty state and inline Add runner for permitted admins.
- Error: Failed list read hides stale cards/editing; device errors block creation only; failed save retains draft and retry.
- Blocked: Missing/expired/revoked/already-registered device or empty name blocks creation; 32-registration cap remains.
- No-access: Readers inspect metadata only; archived Brains cannot edit.
- Duplicate or replay: Existing command idempotency and server uniqueness remain; prevent double submit while pending.
- Stale data: Original revision retained; changed/missing registration disables Save and asks Cancel/reopen.
- Reconciliation divergence: Polling never rebases unsaved draft; actual availability remains separate from draft Enabled.

## Integrations and Runtime Inputs

- Providers: None invoked by UI; runner setup command is copied only.
- Environment: Existing browser origin and Brain/runner IDs in setup command.
- Secrets: No secrets displayed or written; current account device metadata only.
- Failure handling: Existing five-second read polling, explicit refresh and command retry; no tool invocation or startup fallback.

## Tests and Acceptance

- Automated: Web type/design/build, existing private-runner scenario selectors, required scripts/validate.sh, CodeGraph and whitespace.
- Manual: CUA live empty/Add/Cancel plus tab-scoped controlled fixtures for card/edit/add/save/error/retry/stale/read-only/archive/device guards and laptop/wide geometry.
- Acceptance: Compact read/edit cards match approved layout; fixed device cannot move; status is canonical; Save/Cancel and failure fences work. Fixture connectivity is labelled as such; actual registration inventory unchanged.

## Closeout

- Planned: Compact runner cards, inline add/edit and explicit setup copy.
- Shipped: Compact cards, inline Add/Edit, locked device/status badges, explicit setup copy and guarded canonical saves on the ready local stack; see [evidence](../../../mappings/desktop-inline-private-runners-2026-10-06.md).
- Not shipped: Backend/runtime startup changes and real runner connectivity.
- New blockers: None.
- Docs updated: Contract, archived pack, owning epic/index, active/archive indexes, evidence/index and handoff.
- Validation: Web type/design/build, required governance32, whitespace, CodeGraph, CUA live Add/Cancel and isolated save/error/retry/stale/device/reader/archive tests plus laptop/wide layout proof passed. Backend fixture suite not launched; no real registration created or worker started.
- Version: N/A; no version bump requested.
- Commit: Uncommitted.
