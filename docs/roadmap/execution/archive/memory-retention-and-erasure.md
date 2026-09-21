# Retention and controlled erasure

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Apply per-class expiry and erase controlled memory without queued/rebuilt resurrection.
- Non-goals: External-copy deletion, account deletion, automatic host capture and unimplemented retrieval/graph adapters.
- Delivery shape: Rust/PostgreSQL privacy lifecycle, artifact/journal maintenance, companion cleanup, API/browser controls and local proof; uncommitted.

## Governing Sources

[Retention contract](../../../contracts/memory-retention-and-erasure.md),
[canonical memory](../../../adr/0005-canonical-claims-and-time.md),
[review](../../../contracts/memory-review-and-corrections.md),
[evidence](../../../contracts/evidence-collections.md) and
[owning epic](../../epics/memory-lifecycle.md).

## Scope

- In scope: Class policy, 30-day raw defaults, retained excerpts, previewed erasure, tombstones, receipt invalidation, physical cleanup, restore journal and current companion copies.
- Out of scope: External checkouts/backups, host adapters, model/graph/vector representations that do not yet exist.
- Blockers: None; routine policy and dependent-copy choices are resolved in the accepted contract.

## Surface and Interface Changes

- Interfaces: Retention/excerpt/erasure API and UI; companion sync/cleanup and operator maintenance/reconcile commands.
- Storage: Migration 010 adds policy, classification, privacy tombstones/requests and cleanup/journal tracking while preserving existing evidence identities and time boundaries.
- Ownership: Canonical Rust privacy module, shared DTOs, generated UI client, current evidence/claim/publication/worker consumers and native owned storage.

## Data and Authority

- Inputs: Explicit target identities, current preview counter, class policy, exact support/derivation links and owned artifact paths.
- Authority: Browser Brain admin for erasure/policy; committed request for system cleanup; latest retained minimal journal before restored content service.
- Blind spots: Offline/uncontrolled copies and arbitrary content reuploaded with unrelated identities cannot be called erased by a server result.

## States and Edge Cases

- Loading: Policy, bounded preview and cleanup state remain visible.
- Empty: No due records, excerpts, requests or local copies is explicit.
- Error: Unavailable journal/artifact storage is retryable pending cleanup, never false completion.
- Blocked: Missing restore journal, stale preview or active reconciliation blocks dependent service.
- No-access: Reader/device/foreign admin attempts fail; ordinary retained records have positive controls.
- Duplicate or replay: Privacy requests are idempotent; invalidated old receipt keys cannot repeat an erased write.
- Stale data: Due/erased inputs are denied before cleanup; old leases and historical intervals cannot revive content.
- Reconciliation divergence: Central file completion and offline-device acknowledgement are separately reported.

## Integrations and Runtime Inputs

- Providers: Existing PostgreSQL and repository-owned artifact/companion storage; no model or customer runtime call.
- Environment: Erasure journal directory is an explicit repository-owned local input alongside existing artifact configuration.
- Secrets: Ledger/audit retain opaque identities and times without reasons, paths or payloads; pre-persistence sanitation remains required.
- Failure handling: Atomic database invalidation, journal/export state, resumable bounded cleanup and serving barrier during restore reconciliation.

## Tests and Acceptance

- Automated: Class expiry/excerpt controls, erasure dependency and receipt proof, stale/queued work, storage retry, older database/journal replay, companion isolation and real UI workflows.
- Manual: Synthetic local fixtures only; preserve SWEG and user decisions and inspect resulting erased/expired/pending states.
- Acceptance: Every retention-contract requirement for existing consumers passes before archive; future consumers retain explicit obligations.

## Closeout

- Planned: Per-class policy, excerpts, erasure, queue fences, journal/restore barrier, controlled local cleanup and UI.
- Shipped: Class policy/deadlines, exact excerpts, dependency erasure, minimal history markers, transactional fences, durable journal/restore barrier, resumable central/native cleanup, acknowledgement and API/browser controls.
- Not shipped: Future host/model/vector/graph/backup consumers remain owned by their named slices; arbitrary external copies are outside controlled erasure.
- New blockers: None.
- Docs updated: Retention contract, owning epic, execution pack, runbook, dated proof and indexes.
- Validation: Nineteen core scenarios, native extraction, Clippy, three browser workflows and final erased-history checks passed. Normal runtime migration 010, authenticated UI/readiness and SWEG preservation passed; see [dated proof](../../../mappings/retention-and-erasure-proof-2026-09-14.md). Governance and all 32 checker tests passed.
- Version: N/A; no release requested.
- Commit: Uncommitted.
