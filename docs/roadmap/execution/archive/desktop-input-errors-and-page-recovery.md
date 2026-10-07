# Input errors and page recovery

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Understand actual capture/learning failures and recover stale lazy pages.
- Non-goals: Increasing paid allowances, modifying customer text or releasing.
- Delivery shape: Existing web UI, static hosting and model/semantic guards.

## Governing Sources

- [Desktop](../../../contracts/desktop-experience.md)
- [Provider policy](../../../contracts/memory-provider-policy-and-learning.md)
- [Semantic retrieval](../../../contracts/retrieval-semantic.md)
- [Autonomous maintenance](../../../contracts/memory-autonomous-maintenance.md)

## Scope

- In scope: Processing limit help, stage-specific reasons, typed sensitive-input denial, budget scheduling and missing-asset recovery.
- Out of scope: Authentication changes, provider calls for live customer proofs, unrelated configuration.
- Blockers: None.

## Surface and Interface Changes

- Interfaces: Existing failure code gains model_input_sensitive; existing policy commands unchanged.
- Storage: No schema changes; scheduling consumes existing timestamps and policy identities.
- Ownership: Gateway guards transmission; semantic queue schedules; web renders canonical failures; static host owns HTML/assets.

## Data and Authority

- Inputs: Canonical pipeline metadata, existing model policy and static paths.
- Authority: Existing Brain grants, publication guard and budgets.
- Blind spots: Historical generic invalid_input reasons cannot be reconstructed from the code alone.

## States and Edge Cases

- Loading: Existing bounded queries and Suspense.
- Empty: Existing no-source/capture-only handling.
- Error: Human-readable processing/learning labels and tooltip; friendly page error panel.
- Blocked: Sensitive inputs and budget denial make no provider call; budget waits for reset or policy change.
- No-access: Existing checks preserved; no privileged error payload.
- Duplicate or replay: No automatic reload loop or replay of mutations; explicit reload only.
- Stale data: HTML revalidated, missing asset 404, URL-preserving user reload.
- Reconciliation divergence: Historical failures remain intact; a new attempt records its own outcome.

## Integrations and Runtime Inputs

- Providers: Owned deterministic fixtures only for new learning proof.
- Environment: Normal local Compose stack, no new variables.
- Secrets: Metadata-only runtime inspection; private-key matches never emitted.
- Failure handling: Existing bounded learning retries; prevent minute-by-minute budget-blocked semantic requeue.

## Tests and Acceptance

- Automated: Missing-asset/SPA fallback tests and live HTML cache headers, state/reason tests, sensitive-input no-call and semantic reset/policy scheduling fixtures; build and required governance validation.
- Manual: Ready local stack, Explore repositories opens, processing controls and accessible help in main form, real DLAG failure reasons visible.
- Acceptance: No blanket capture failure for learning rejection; no raw lazy module error; no budget increase or relaxed secret detection.

## Closeout

- Planned: Scoped deliverables above.
- Shipped: Main-form limits with on-demand help; stage-specific failure labels/reasons; typed sensitive-input denial; budget-blocked semantic scheduling; missing-asset 404 and styled user-triggered page recovery. Normal local Compose deployment and browser recovery/Cancel are verified.
- Not shipped: Paid completion of every historical learning failure; protected or oversized evidence is not altered to force learning. No release or new macOS plugin trust setup is included.
- New blockers: None.
- Docs updated: Governing amendments, provider runbook, owner/indexes and [dated evidence](../../../mappings/input-errors-and-page-recovery-2026-10-07.md).
- Validation: Static-file/SPA fallback and sensitive-input no-call proofs; all 20 semantic regression tests; three pure UI tests; web type/design/build; formatting and Clippy with warnings denied; 32 governance fixtures. Browser info icon, main-form controls, Cancel and isolated missing-chunk/Reload recovery passed. Actual legacy URL returns 404 and HTML remains no-store. Live new learning succeeds while retained input/model failures remain visible.
- Version: N/A; no release requested.
- Commit: Uncommitted.
