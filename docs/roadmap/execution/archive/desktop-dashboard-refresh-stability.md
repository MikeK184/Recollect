# Stable dashboard refreshes

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Background activity refreshes do not unnecessarily unload the dashboard.
- Non-goals: Changing authorization, retention, polling cadence or server feeds.
- Delivery shape: Local UI correction and deployment to the existing local stack.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md), including its short snapshot deadline and protected payload clearing.
- [Owning epic](../../epics/desktop-visual-experience.md).

## Scope

- In scope: Pipeline visibility lifecycle and Brain provider identity.
- Out of scope: Customer writes, paid learning, schema or new dependencies.
- Blockers: None. Context7 is unavailable; React and TanStack official documentation verified 2026-10-07.

## Surface and Interface Changes

- Interfaces: Existing GETs only; no endpoint or payload change.
- Storage: N/A; existing query lifecycle only.
- Ownership: BrainLayout controls identity; BrainPipeline controls snapshot validity and motion.

## Data and Authority

- Inputs: Current authorized Brain metadata and bounded pipeline responses.
- Authority: Server validity deadline, current Brain identity and effective role.
- Blind spots: No claim that every historical flicker has been reproduced.

## States and Edge Cases

- Loading: Skeleton on initial data load only.
- Empty: Preserve existing empty state.
- Error: Failed reads clear protected data; explicit retry remains.
- Blocked: Existing recorded stage failures remain truthful.
- No-access: Changed role remounts the boundary; failed Brain/feed reads clear content.
- Duplicate or replay: Returning to visibility resets the animation baseline, not the fresh snapshot.
- Stale data: Existing request-duration-adjusted deadline remains, including while hidden.
- Reconciliation divergence: Removed feed rows close inspection; current metadata revisions invalidate assurance reads without unloading the route.

## Integrations and Runtime Inputs

- Providers: None; GETs do not enqueue work or call a model.
- Environment: Existing local stack at 127.0.0.1:8787.
- Secrets: No payload, credential or configuration logging.
- Failure handling: Existing two-second foreground polling; no new automatic retry or extended cache lifetime.

## Tests and Acceptance

- Automated: Regression for pending refresh after visibility changes and Brain revisions; existing stale/error/removed-row coverage; web build and governance.
- Manual: CUA browser proof of stable graph DOM during pending refresh, expired/error clearing and local readiness.
- Acceptance: Fresh graph remains mounted, selection stays; failures/expiry clear data; identity/role changes still reset. No browser exceptions.

## Closeout

- Planned: Lifecycle correction, focused proof and local stack deployment.
- Shipped: Fresh dashboard graph/inspection survives visibility and metadata refreshes; role changes, expiry and failed reads still discard protected state. Hidden motion pauses. Existing local stack rebuilt and ready.
- Not shipped: No server, schema, polling cadence or provider-policy changes. No external publication.
- New blockers: None.
- Docs updated: Desktop contract, owning epic, epic/execution indexes and dated mapping.
- Validation: CUA reproduced the old reset and verified pending refresh, metadata, role, hidden, expiry and failure scenarios on the corrected build. Six live polls kept the same graph DOM without loading/stale states; no unexpected console errors. Web build/design/typecheck, two pure pipeline regressions, 32 governance fixtures, CodeGraph sync and diff checks passed. Browser regression authored and collected; its CLI browser run was not used. [Dated evidence](../../../mappings/dashboard-refresh-stability-2026-10-07.md).
- Version: N/A; no release requested.
- Commit: uncommitted.
