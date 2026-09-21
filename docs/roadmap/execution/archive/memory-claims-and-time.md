# Evidence-linked claims and two time dimensions

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Author attributable claims/decisions and inspect what applied then separately from what Recollect knew then.
- Non-goals: Review decisions, correction/erasure, automatic learning and general retrieval.
- Delivery shape: Rust protocol/service, PostgreSQL migration, React UI, focused tests and local runbook; uncommitted.

## Governing Sources

[ADR 0005](../../../adr/0005-canonical-claims-and-time.md),
[claims contract](../../../contracts/memory-claims-and-time.md),
[durable commands](../../../contracts/platform-durable-work.md),
[workspace scope](../../../contracts/evidence-workspace-scope.md) and
[owning epic](../../epics/memory-lifecycle.md).

## Scope

- In scope: Manual proposals/decisions, exact evidence support, immutable knowledge history, independent states, canonical eligibility and affected-evidence reassessment.
- Out of scope: Named successor review, correction, retention, model and retrieval work.
- Blockers: None; accepted authority resolves the bounded first claims interface.

## Surface and Interface Changes

- Interfaces: Brain-scoped claims and evidence catalogue/detail API plus browser create/edit/history and separate fact/knowledge-time filters.
- Storage: Migration 008 for claim identities, immutable revisions and typed support links under RLS; UUID conditional edits, no hashes.
- Ownership: Shared DTOs; server memory command/eligibility module owns policy; React consumes generated OpenAPI.

## Data and Authority

- Inputs: Exact source versions, repository facts, manifest revisions and explicit author applicability/observations.
- Authority: Current actor/device/Brain grants, bound operation where present and canonical revision history.
- Blind spots: Author assessment is not review or verified runtime truth; unavailable source bytes limit strict use.

## States and Edge Cases

- Loading: Visible loading and bounded paging for claims, evidence and history.
- Empty: No claims/evidence and no matching historical knowledge are explicit.
- Error: Invalid time/support, source absence and failed saves remain visible with retry.
- Blocked: Archived Brain, missing scope and unsupported state promotion deny dependent writes/use.
- No-access: Current authorization and RLS cover every ID, evidence and history path.
- Duplicate or replay: Same normalized command replays; mismatched identity/base returns conflict.
- Stale data: Immutable prior knowledge, separate fact time and selective document/manifest reassessment.
- Reconciliation divergence: Canonical reads enforce eligibility while refresh work is queued; no derived search readiness claim.

## Integrations and Runtime Inputs

- Providers: Existing local PostgreSQL/artifacts only; Context7 verified PostgreSQL 17 timestamp and RLS behavior.
- Environment: Existing database/artifact/runtime variables; no new provider credential needed.
- Secrets: Existing configured-secret detector before persistence; no copied excerpts or external transmission.
- Failure handling: Existing database timeout, transaction rollback, idempotency and bounded durable jobs.

## Tests and Acceptance

- Automated: Compound API/RLS/concurrency/time/evidence fixture, eligibility boundary controls and browser create/edit/history/mobile proof; generated API/build, rustfmt/Clippy and governance checks.
- Manual: Inspect local desktop/mobile claim/evidence history, including the existing SWEG snapshot when appropriate.
- Acceptance: All claims-contract scenarios pass; archive only on recorded evidence.

## Closeout

- Planned: Claims/decisions, exact evidence, independent state and temporal history with canonical eligibility/UI.
- Shipped: Evidence-linked claims/decisions, immutable knowledge revisions, fact/knowledge-time selection, independent trust dimensions, selective document/manifest reassessment, canonical eligibility and browser authoring/history. A real SWEG test proposal links the retained structural fact and exact manifest without inventing runtime proof.
- Not shipped: N/A within this slice. Human review/correction, retention/erasure, model learning, procedures and general retrieval remain in their named successor slices.
- New blockers: None.
- Docs updated: ADR 0005, claims contract, [runbook](../../../runbooks/claims-and-time.md), [proof mapping](../../../mappings/claims-and-time-proof-2026-09-14.md), local development guidance, README, owning epic and indexes. The platform epic records the discovered pairing URL fix under its small-fix exception.
- Validation: Eleven core API/database scenarios; focused temporal/environment/readability/authority proof; seven ordinary browser workflows plus deterministic opaque-code regression; 81 unique OpenAPI operations, frontend build, rustfmt/Clippy, governance lint and 32 checker tests. Actual SWEG API/browser evidence and normal-runtime readiness passed. Optional OIDC was skipped in this regression.
- Version: N/A; no release requested.
- Commit: Uncommitted.
