# Desktop final polish and loading

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Finish the authorized Graph, Connections, setup and Privacy improvements and reduce initial JavaScript loading.
- Non-goals: File-size refactoring, new dependencies, authorization changes, automatic probes, commit/push/release, benchmark work.
- Delivery shape: Existing components and routes, measured build output, focused browser acceptance and independent UI review.

## Governing Sources

- [Desktop experience](../../../contracts/desktop-experience.md)
- [Desktop knowledge surface](../../../contracts/desktop-knowledge-surface.md)
- [Graph exploration](../../../contracts/graph-exploration.md)
- [Managed MCP runtime](../../../contracts/mcp-runtime-and-credentials.md)
- [Memory retention](../../../contracts/memory-retention-and-erasure.md)
- [Owning epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Graph secondary controls in one menu; searchable compact authorized connection cards; explicit setup completion link to Brain activity without a success claim; Privacy section links to existing editors; lazy presentation imports in existing files.
- Out of scope: New graph engine, new feeds, automatic tool tests, policy defaults or permissions, file splitting.
- Blockers: None; user authorized this exact follow-up on October 4.

## Surface and Interface Changes

- Interfaces: Existing Graph, connection catalogue, setup, Privacy and inspection routes; no new API for this slice.
- Storage: None.
- Ownership: Existing components retain their responsibilities; only presentation loading is deferred. Brain/auth refresh remains eager.

## Data and Authority

- Inputs: Existing authorized catalogue, observed sessions, privacy policies and graph reads.
- Authority: All grants, revisions, URL state, deadlines and explicit mutations remain canonical.
- Blind spots: Setup has no exact resulting plugin device identity; Brain activity is not confirmation of this setup. Connection search is within the authorized loaded catalogue.

## States and Edge Cases

- Loading: Lazy inspection has a visible Suspense fallback; authority checks continue.
- Empty: No matches differs from no configured connections; counts describe loaded scope.
- Error: Existing failures and diagnostic affordances remain visible.
- Blocked: Existing disabled/archived and grant gates remain.
- No-access: Failed authorized reads clear protected content.
- Duplicate or replay: Menu/navigation does not execute tools or mutate policy.
- Stale data: Existing polling, deadline and revision handling remain.
- Reconciliation divergence: N/A; no new derived store.

## Integrations and Runtime Inputs

- Providers: None added.
- Environment: Existing local stack and disposable UI harness.
- Secrets: Existing environment/OS storage only; no new handling.
- Failure handling: Existing retry and uncertainty rules preserved.

## Tests and Acceptance

- Automated: Existing focused Graph, connection/setup, Privacy and Knowledge inspector journeys; frontend build/typecheck/design; final validation.
- Manual: Independent review of current source and rendered laptop/desktop captures; measure the entry and actual feature requests, without inferring an equivalent route speedup.
- Acceptance: Controls remain reachable, no implicit mutation/probe, correct direct Privacy section, usable compact cards and reduced initial payload without hiding feature costs.

## Closeout

- Planned: Authorized Graph, Connections, setup and Privacy polish with measured deferred loading.
- Shipped: Secondary Graph tools menu; loaded-name/environment connection filtering and compact cards; truthful Brain-wide activity action at setup completion; direct permission-aware Privacy section actions; lazy Knowledge boundary and named heavy presentation imports in the same files. Entry 842,411 → 607,350 bytes; actual Brain chooser 755,589 bytes across 14 chunks. [Dated evidence](../../../mappings/desktop-visual-closeout-2026-10-04.md).
- Not shipped: Equivalent total network/speedup claims, file-size partitioning, new dependencies, new policies and new runtime controls are outside scope.
- New blockers: None.
- Docs updated: Desktop contract, this archived pack, epic and indexes, dated evidence and handoff.
- Validation: Existing Graph chrome/keyboard, Connections search/Pause, direct HTTP MCP setup, Privacy editing/erasure, and canonical lazy Knowledge/evidence focus/accessibility journeys; build/typecheck/design, clippy, independent current UI review, final governance/whitespace and CodeGraph; local readiness and new served entry verified.
- Version: N/A; no release policy or version bump.
- Commit: Uncommitted; no commit, push or external release.
