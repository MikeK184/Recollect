# Actionable review and durable corrections

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Review supported claims and keep rejection/correction/withdrawal effective through new IDs, evidence and restart.
- Non-goals: Privacy erasure, automatic learning and unimplemented general retrieval/graph adapters.
- Delivery shape: Rust command/eligibility extensions, PostgreSQL migration, browser review/conflict workflows and focused local proof; uncommitted.

## Governing Sources

[ADR 0005](../../../adr/0005-canonical-claims-and-time.md),
[review contract](../../../contracts/memory-review-and-corrections.md),
[claims/time](../../../contracts/memory-claims-and-time.md),
[durable work](../../../contracts/platform-durable-work.md) and
[owning epic](../../epics/memory-lifecycle.md).

## Scope

- In scope: Actual reviewer authority, accepted/rejected/withdrawn revisions, explicit revalidation, typed conflict resolution, replay-resistant assertion rules and eligibility invalidation.
- Out of scope: Erase/retention, provider policies and downstream raw/vector/graph consumers in their named slices.
- Blockers: None; accepted contract resolves bounded textual applicability and actions.

## Surface and Interface Changes

- Interfaces: Per-claim review/history and conflict resolution endpoints; existing claim eligibility gains rule/conflict/lifecycle qualifiers; browser controls inspect evidence and act.
- Storage: Migration 009 adds immutable decisions, rules, exact-revision exceptions and a Brain eligibility counter under RLS; existing revision JSON gains default-compatible fields.
- Ownership: Shared DTOs, one canonical Rust review/admission/eligibility boundary and generated React client; no new provider runtime.

## Data and Authority

- Inputs: Current base revisions, normalized assertions, exact support, applicability and explicit reviewer reason/disposition.
- Authority: Authenticated browser writer/admin, current Brain grant, durable decisions/rules and canonical current revisions.
- Blind spots: Text matching is bounded; a model's confidence or a new evidence ID is not semantic equivalence or revalidation proof.

## States and Edge Cases

- Loading: Bounded review history/conflict/evidence loading remains visible.
- Empty: No decisions, conflicts or matching rules is explicit.
- Error: Invalid disposition/content, failed save and unreadable evidence expose retry or correction.
- Blocked: Applicable rejection, unresolved contradiction, archived Brain and capacity deny dependent acceptance.
- No-access: Reader/device/foreign Brain review fails with positive controls; current access covers history/replay.
- Duplicate or replay: Identical key replays; changed input or stale base conflicts; immutable rules survive alternate IDs and restarted services.
- Stale data: Historical review stays inspectable; current rules and invalidation prevent stale accepted use.
- Reconciliation divergence: Canonical eligibility applies before refresh/projection completion; downstream consumers cannot infer readiness.

## Integrations and Runtime Inputs

- Providers: Existing PostgreSQL/artifact service only; no model or external source call.
- Environment: Existing local runtime configuration; no new secret input.
- Secrets: Shared pre-persistence detector covers reasons, corrected content and command receipts.
- Failure handling: Transaction rollback, idempotency, existing database limits and durable refresh behavior.

## Tests and Acceptance

- Automated: Compound API/RLS/replay/revalidation/conflict atomicity fixtures plus actionable browser review/correction and desktop/mobile proof; build/format/Clippy/governance.
- Manual: Inspect local review decisions and qualified history with synthetic evidence; preserve user-authored/customer decisions.
- Acceptance: All review-contract scenarios pass before archiving; no customer assertion is automatically human-accepted as a test side effect.

## Closeout

- Planned: Actionable review, durable rejection/withdrawal/correction, typed conflicts, explicit revalidation and canonical invalidation.
- Shipped: All single-claim review actions, four conflict dispositions, durable assertion rules/exceptions, current eligibility and scope invalidation, browser evidence/review/history forms and generated API.
- Not shipped: General retrieval/graph consumers, automatic learning and erasure belong to their named successor slices.
- New blockers: None.
- Docs updated: Review contract, claims runbook, review runbook/proof, README, owning epic and execution indexes.
- Validation: Thirteen core API scenarios passed; three focused review scenarios and the final scope-removal/failure scenario passed. Both claim/review browser workflows passed in 14 seconds with worker restart. Actual local migration/readiness/SWEG review inspection, generated 84-operation API, build, rustfmt, Clippy and governance/32 checker tests passed; see [proof](../../../mappings/review-and-corrections-proof-2026-09-14.md).
- Version: N/A; no release requested.
- Commit: Uncommitted.
