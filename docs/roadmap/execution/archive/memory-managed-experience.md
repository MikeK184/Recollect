# Managed autonomous memory setup

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Connect a Brain once and let agents use its automatic memory lifecycle without manual resource tuning.
- Non-goals: New retrieval algorithms, unrestricted telemetry or automatic MCP execution.
- Delivery shape: Shared Rust handlers, browser simplification, focused proof and local stack update.

## Governing Sources

- [Managed experience](../../../contracts/memory-managed-experience.md)
- [Autonomous maintenance](../../../contracts/memory-autonomous-maintenance.md)
- [Catalogue](../../../contracts/mcp-catalogue-and-profiles.md)
- [Vision](../../../foundation/vision.md)
- [Owning epic](../../epics/memory-lifecycle.md)

## Scope

- In scope: Managed model/capture setup, new-Brain opt-in request, simple Ask/search/notes, compact navigation and agent setup, status-first settings and owner connector import.
- Out of scope: Existing customer Brain policy conversion, paid live-source testing, external deployment and unrelated desktop regressions.
- Blockers: None; user supplied the operating-model decision.

## Surface and Interface Changes

- Interfaces: Automation GET/PUT, optional managed setup on Brain creation and owner-only connector manifest POST, as specified by the contract.
- Storage: Reuse policy/audit/outbox tables; a migration grants narrowly owner-scoped catalogue insertion.
- Ownership: Memory owns the preset; existing domain persistence and workers remain authoritative. MCP owns connector validation and execution rights.

## Data and Authority

- Inputs: Installed models, current Brain policies and validated connector manifests.
- Authority: Authenticated admin adoption; owner-browser connector approval; existing canonical evidence policy.
- Blind spots: Configured provider/capture is not successful live processing or complete host coverage.

## States and Edge Cases

- Loading: Disable dependent actions until current policy is available.
- Empty: New Brain and empty connector catalogue have actionable setup.
- Error: Preserve form input and explain rejected configuration.
- Blocked: Missing provider/host explains the remaining installation requirement.
- No-access: Server rejects non-admin automation and non-owner connector import; devices cannot broaden policy.
- Duplicate or replay: Existing command reservation and stable connector key prevent duplicate effects.
- Stale data: Compare both policy heads and reject stale adoption atomically.
- Reconciliation divergence: Keep custom exclusions, retention, corrections and separate execution grants intact.

## Integrations and Runtime Inputs

- Providers: Existing installed model gateway and deterministic fixtures.
- Environment: Existing RECOLLECT model and database inputs only.
- Secrets: No values in browser manifests, docs or test output.
- Failure handling: Existing bounded background retries and provider accounting remain.

## Tests and Acceptance

- Automated: Focused API authorization/atomicity/preservation and browser setup tests, existing closed-loop regression, Rust checks, generated API, frontend build, ./scripts/validate.sh and git diff --check.
- Manual: Inspect the running local UI and readiness after a data-preserving rebuild.
- Acceptance: All acceptance criteria in the managed experience contract; no claim of completing other desktop packs.

## Closeout

- Planned: Managed preset and simple agent-first setup across the annotated screens.
- Shipped: Managed model/capture preset through shared Rust persistence, compatible old create receipts, one existing-Brain adoption action, compact navigation, simple Ask/search/notes, status-first settings, coding-agent setup and owner-only inert connector import. Deployed locally with exact recorded inventory preservation.
- Not shipped: Explicit deferrals remain: arbitrary telemetry, connector marketplace, adaptive budgets, new ranking algorithms, existing-Brain bulk conversion and unrelated desktop acceptance.
- New blockers: None.
- Docs updated: Vision, managed experience, affected contracts, runbooks, [dated proof](../../../mappings/managed-memory-experience-2026-09-28.md), [handover](../../../../CONTINUE_HERE.md), epic and indexes.
- Validation: Two new API cases, three existing controlled native lifecycle/provider/MCP cases, 26 focused browser cases across documented runs, 21 unit cases with three live-Vault cases ignored, workspace clippy, generated schema, frontend type/design/build, governance and whitespace checks. Local migration/readiness and before/after data/grant/policy inventory equality passed. Initial failures, selector fixes, worker prerequisite and targeted reruns are recorded in the dated proof; broader desktop acceptance remains open.
- Version: N/A; no release requested.
- Commit: uncommitted.
