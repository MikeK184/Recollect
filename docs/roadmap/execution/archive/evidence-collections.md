# Collections, source history and retained artifacts

Status: shipped
Owning epic: `docs/roadmap/epics/evidence-and-workspaces.md`
Work type: product

## Summary

- Goal: Import attributable evidence, organize one source across views and preserve inspectable history.
- Non-goals: Repository/session adapters, model extraction and memory/retrieval semantics.
- Delivery shape: Rust API/worker/artifact store, PostgreSQL migration, typed browser workflows and focused local proof.

## Governing Sources

- [Collections contract](../../../contracts/evidence-collections.md)
- [Runtime ADR](../../../adr/0003-product-runtime.md)
- [Durable work](../../../contracts/platform-durable-work.md)
- [Device authority](../../../contracts/platform-device-pairing.md)
- [Owning epic](../../epics/evidence-and-workspaces.md)

## Scope

- In scope: Collections/areas/environments, immutable text/reference source versions, controlled artifact persistence, source spans, reprocessing, provenance and UI.
- Out of scope: External connectors, repository extraction, knowledge claims and erasure.
- Blockers: None; detailed behavior and required platform primitives are available.

## Surface and Interface Changes

- Interfaces: Brain evidence catalogue/policy/groups, source import/history/version/content/associations/reprocess and generated shared DTOs.
- Storage: Brain-scoped groups/memberships, sources/current pointers, versions and rebuildable chunks; UUID files under the configured artifact directory.
- Ownership: Evidence handlers own canonical mutations; artifact adapter owns file access; worker owns fenced chunk projection; UI reuses current Brain grants.

## Data and Authority

- Inputs: Explicit retained text or source reference, title/type/time, selected associations and current actor/device.
- Authority: Canonical PostgreSQL history and grants; immutable artifact bytes supply rebuildable evidence.
- Blind spots: A reference alone is not verified availability; chunks are not accepted claims or operational observations.

## States and Edge Cases

- Loading: Paginated catalogue/history, import progress and worker status polling.
- Empty: No groups/sources, reference-only content and unknown observation time are explicit.
- Error: Validation 400, denied write/policy 403, foreign IDs 404, stale editor/archived Brain 409, oversized upload 413 and queue/group capacity 429.
- Blocked: Storage failure rolls back canonical metadata/audit/jobs; missing input remains visible and can be restored/reprocessed.
- No-access: Current Brain/device authorization and RLS on every data path, including replay and worker publication.
- Duplicate or replay: Existing optional receipt keys compare normalized input; version processing is idempotent and fenced.
- Stale data: A new immutable version changes only the current pointer; stale editor tokens fail and old jobs cannot change it.
- Reconciliation divergence: Files precede canonical commit; unreferenced crash leftovers are never published or silently deleted. Missing files remain declared missing.

## Integrations and Runtime Inputs

- Providers: Existing SQLx/PostgreSQL, Tokio filesystem APIs and Mantine/browser file input; no external service/model calls.
- Environment: RECOLLECT_ARTIFACT_DIR plus existing server variables.
- Secrets: Never log content or source input; no new credentials. Explicit import consent and Brain capture policy gate retained payloads and receipts.
- Failure handling: 1 MiB text, 2 MiB request, 50-source pages, 20-version pages, 100 associations, bounded capture lane and existing lease/retry/backpressure.

## Tests and Acceptance

- Automated: Real API/database/file lifecycle and authority scenario, missing/failed artifact recovery, old-version/revoked worker controls, and browser import/history/view/removal workflows.
- Manual: Inspect desktop/mobile source screens and verify actual local retained-content reads/processing.
- Acceptance: Collections contract passes through real handlers, worker, artifact store and browser before archive.

## Closeout

- Planned: Attributable source history, retained artifacts, overlapping views and visible processing.
- Shipped: Brain-scoped text/reference imports, immutable versions and provenance, retained artifacts, shared collections/areas/environments, policy controls, fenced support-span processing and complete browser flows.
- Not shipped: N/A within this slice. Repository/workspace/session adapters, semantic memory, erasure and operational restore remain named successors.
- New blockers: None.
- Docs updated: Collections contract, [proof mapping](../../../mappings/evidence-collections-proof-2026-09-14.md), [runbook](../../../runbooks/evidence-collections.md), local development, README, environment example and reconciled epic/execution indexes.
- Validation: Seven core Rust scenarios and one real OIDC scenario passed; five isolated browser workflows, native capture/restarted workers, inspected desktop/mobile screens, 51 unique API operations, frontend build, Clippy with warnings denied, rustfmt, governance lint and all 32 checker tests passed. Actual local startup, owner login and readiness succeeded. No external deployment is claimed.
- Version: N/A: no release or strict versioning.
- Commit: uncommitted.
