# Irreversible Brain deletion

Status: shipped
Owning epic: `docs/roadmap/epics/product-platform.md`
Work type: product

## Summary

- Goal: An authorized administrator can permanently delete a Brain and everything derived from it through one audited command, with accurate intermediate states and a restore barrier that keeps it deleted.
- Non-goals: Account deletion, bulk multi-Brain deletion, undo or recycle bin, restoring deleted content, Vault credential revocation, and any change to per-record Withdraw or Erase semantics.
- Delivery shape: Local Rust server command and read endpoints, browser Settings and Brains presentation, focused integration/RLS/browser proof, and reconciled epic and execution indexes.

## Governing Sources

- [ADR 0016: Brain deletion](../../../adr/0016-brain-deletion.md)
- [Brain deletion contract](../../../contracts/platform-brain-deletion.md)
- [Retention and controlled erasure](../../../contracts/memory-retention-and-erasure.md)
- [Durable commands and workers](../../../contracts/platform-durable-work.md)
- [Platform bootstrap](../../../contracts/platform-bootstrap.md)
- [Companion pairing and device authority](../../../contracts/platform-device-pairing.md)
- [Canonical graph projection and traversal](../../../contracts/graph-projection-and-traversal.md)
- [Managed MCP calls and credentials](../../../contracts/mcp-runtime-and-credentials.md)
- [Encrypted backup, upgrade and recovery](../../../contracts/operations-recovery-drills.md)
- [Vision: capture, retention and model policy](../../../foundation/vision.md#capture-retention-and-model-policy)
- [Engineering principles: route every mutation through one policy](../../../foundation/engineering-principles.md#route-every-mutation-through-one-policy)
- [Owning epic](../../epics/product-platform.md)

## Scope

- In scope: `DELETE /api/brains/{brain}`, `POST /api/brains/{brain}/deletions/preview` and `GET /api/brains/{brain}/deletions/{request}` and the paired-companion `GET /api/brains/{brain}/deletions/fence`; Brain-wide closure composed from existing per-class erasure rules; tombstone and journal entries; read and recall denial across every consumer; physical cleanup queue participation; companion fence synchronization on next check-in; restore-barrier proof; Settings General delete control separated from archive.
- Out of scope: Account deletion and bulk deletion; undo, recycle bin or restoration; changing retention classes, per-record closure, the journal format, Withdraw or Erase; Vault token or lease revocation; deleting global device or account rows; Brains and Devices list ordering and density, which `desktop-knowledge-surface` owns; any new mutation boundary.
- Blockers: None. [ADR 0016](../../../adr/0016-brain-deletion.md) and its contract are accepted with the user's 2026-09-29 decision recorded.

## Surface and Interface Changes

- Interfaces: Four deletion and companion-fence endpoints above, generated OpenAPI client and TypeScript types; closure counters serialize as lossless decimal strings; the existing `GET/PATCH /api/brains/{id}` pair keeps its current rename/archive/reopen meaning with no delete alias.
- Storage: New minimal tombstone and deletion-request rows carrying identity, actor, time, disposition, closure counter and cleanup state only; no controlled content. Existing deletion journal gains Brain-wide target entries using its current fields. Migration adds no name, description or content column.
- Ownership: Command authorization, closure orchestration and audit live in the server platform module beside Brain administration; per-class dependent removal stays with each domain owner's existing erasure helpers; physical artifact and graph-generation cleanup stays in the existing worker lanes; companion fence application stays in the agent crate; the browser delete control lives in the Settings feature with the shared confirmation flow.

## Data and Authority

- Inputs: Target Brain identity, preview closure counter, confirmation string equal to the Brain name, optional idempotency key, authenticated administrator identity, existing dependent identity sets, artifact and vector locations, graph generation records, companion check-in state.
- Authority: PostgreSQL canonical rows, retained artifacts and the deletion journal are authoritative. Neo4j generations, semantic representations and analytics reports are derived and removed as dependents. The shared mutation policy is the only write path, and readers enforce canonical state while projections lag.
- Blind spots: Offline companion copies are unknown until check-in. Product-managed backups can still contain the Brain until their own window expires. External or uncontrolled copies are outside reach. No content fingerprint exists, so a future manual re-upload under a new identity is not recognized as the same content.

## States and Edge Cases

- Loading: Preview shows dependent counts as they resolve and disables confirmation until the counter arrives.
- Empty: A Brain with no sources, claims, snapshots or connections still deletes and still produces a tombstone; the preview distinguishes an empty Brain from a no-match.
- Error: Mismatched confirmation, stale counter, unknown Brain, archived-state rule violation or journal export failure each produce a distinct message. Journal failure leaves the request pending and never reports complete.
- Blocked: Physical cleanup that cannot reach storage leaves the request pending and retryable with visible progress; canonical reads stay absent throughout.
- No-access: Readers, contributors, device tokens and MCP callers are denied with no mutation. A principal that previously had access receives unknown rather than forbidden, so existence is not leaked.
- Duplicate or replay: The same idempotency key returns the original result. A repeated delete of an already-absent Brain is a no-op against the tombstone and creates nothing.
- Stale data: Queued or running jobs for the Brain are fenced; stale leases cannot publish; cached recall, semantic and graph results are invalidated so a lagging projection cannot serve content.
- Reconciliation divergence: File absence is success. Inaccessible storage is pending. Companion copies unacknowledged are reported separately from central completion, and a restored older database replays the journal rather than resurrecting the Brain.

## Integrations and Runtime Inputs

- Providers: No model or embedding provider call is required or permitted by this operation. Neo4j and PostgreSQL are the only runtime dependencies; the optional remote recovery mirror participates through its existing adapter.
- Environment: No new variable is introduced. Existing database, artifact-root, erasure-journal and optional mirror configuration names apply.
- Secrets: No secret value is read, written or logged. Credential references are removed as records only. No Vault call occurs, honoring the standing prohibition on revoking any Vault token or lease.
- Failure handling: Database commit, journal export and physical cleanup each have independent retry through the existing durable queues with bounded attempts. Interrupted work resumes idempotently. No timeout converts to a success claim.

## Tests and Acceptance

- Automated: Focused Rust integration and RLS cases for authorization, preview counts, closure, tombstone content, replay, fencing and restore; companion fence unit and integration cases; generated client and web typecheck; affected browser specs; `./scripts/validate.sh`; `git diff --check`.
- Manual: Operator delete of a disposable fixture Brain through the real browser, confirming the preview counts, the confirmation string, the resulting absence from every listing and the accurate pending or complete status.
- Acceptance: Authorized delete removes the Brain and its dependents and leaves other Brains byte-identical in a parsed before/after inventory; deleted Brain reads as unknown everywhere; device and account rows survive with only its grants removed and no Vault request observed; interrupted cleanup resumes without resurrecting content; restore from an older database with the retained journal does not serve the Brain; tombstone, journal, receipt and log inspection show no controlled content.

## Closeout

- Planned: Deletion endpoints, Brain-wide closure, tombstone and journal behavior, read and recall denial, physical cleanup participation, companion fence, restore proof, and the Settings and Brains browser surface.
- Shipped: Locally delivered on 2026-10-01. Migration 029 and four deletion/fence endpoints; Brain closure, tombstone/journal, unknown reads, queued-work fencing and physical cleanup; companion synchronization; Settings preview, exact-name confirmation, stale-preview recovery and content-free cleanup receipt. The closure counter now crosses JSON as a lossless decimal string, and companion cleanup counts are local fields. See the [continuation evidence](../../../mappings/desktop-continuation-2026-10-01.md).
- Not shipped: Account deletion, bulk deletion, undo and Vault credential revocation remain excluded. Offline copies await companion check-in and backup copies retain their documented lifetime. Final backend changes are not deployed to the existing port-8787 installation.
- New blockers: None for this slice. Unrelated desktop acceptance packs and the held Ask slice remain open.
- Docs updated: Governing desktop priority amendment, dated evidence, desktop/agent/capture runbooks as applicable, this pack, owning epic and epic/execution indexes, and CONTINUE_HERE.md.
- Validation: Four focused Rust HTTP/companion deletion cases passed, including authorization, replay, closure and restore/fence behavior. Disposable browser deletion passed wrong-name rejection, stale preview, absence from listing and direct reads, cleanup completion and reader restrictions. Fresh/upgrade migration evidence is retained from the implementation handover; current disposable database migration also passed. Web design/typecheck/build, workspace Rust tests, platform integration with documented focused reruns, Clippy, formatting, all 32 governance checker tests and git diff --check passed. The evidence mapping distinguishes opt-in skips, real external results and current deployment.
- Version: N/A: no release policy or version bump in this scope.
- Commit: Uncommitted; no commit, push or release performed.
