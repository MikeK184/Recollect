# Automatic session digests and durable excerpts

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Settled captured sessions automatically produce supported scoped continuation with permitted durable evidence.
- Non-goals: Universal semantic merging, personal inference, automatic command execution.
- Delivery shape: Local Rust scheduling/excerpts, migration 039, canonical handover reuse and controlled native proofs.

## Governing Sources

- [Session digest contract](../../../contracts/memory-automatic-session-digests.md)
- [Source support](../../../contracts/memory-source-support-verification.md)
- [Autonomous maintenance](../../../contracts/memory-autonomous-maintenance.md)
- [Memory owner](../../epics/memory-lifecycle.md)

## Scope

- In scope: Exact authenticated partitions, five-minute/closed settlement, unique current generations, bounded pages, minimal exact excerpts, class/role preservation and privacy/recovery.
- Out of scope: Semantic similarity, unconstrained family retirement parsing and paid quality claims from fixtures.
- Blockers: None for local acceptance; deployment remains separate.

## Surface and Interface Changes

- Interfaces: Native session partition, generation and current-coverage metadata; no mandatory UI or agent command.
- Storage: Migration 039 registers server receipts, exact partitions/pages, unique generations and retained excerpt equivalence metadata under Brain RLS/deletion closure.
- Ownership: Native coordinator schedules; existing handover/support workers synthesize and assess; learning remaps exact permitted support atomically.

## Data and Authority

- Inputs: Authenticated capture binding/task/session/child, exact selection, nullable manifest, current supported contributors and server receipt order.
- Authority: Standing Brain writer, content/provider/budget policy, shared canonical support predicate and protected human decisions.
- Blind spots: Missing capture remains missing. A digest supplies continuation, not proof that reported production changes happened.

## States and Edge Cases

- Loading: Five quiet server-receipt minutes or closed task; relevant processing and learning must settle.
- Empty: No eligible contributions supplies no digest and invalidates obsolete coverage.
- Error: Staged native handover recovery remains bounded; unknown provider completion never resends blindly.
- Blocked: Unsupported or conflicting synthesis remains inspectable; no required review task.
- No-access: Writer, original/excerpt content classes, scope, retention and erasure rechecked at publication.
- Duplicate or replay: Exact generation identity deduplicates provider work; receipt-only changes update coverage without a model call.
- Stale data: Late arrivals and changed contributor heads remove current usability across consumers immediately.
- Reconciliation divergence: At most twelve contributors and twenty supports per whole-input page; at most two generations per coordinator pass.
- Privacy: Original erasure removes aliases; raw expiry preserves independently permitted retained support. Partition content is scrubbed on affected canonical removal.

## Integrations and Runtime Inputs

- Providers: Existing synthesis and independent support assessment; no scheduler model call.
- Environment: Existing Brain scope/manifest and installed model configuration.
- Secrets: Existing payload exclusion; synthetic fixtures only.
- Failure handling: Inherit bounded native/stage-only recovery and non-replay of unknown outcomes.

## Tests and Acceptance

- Automated: Quiet/closed/late/abrupt sessions, partition/page/replay and no-charge coverage; exact excerpt role/scope, original-class denial, no loop, expiry/erasure/old-backup and protected controls.
- Manual: Independent source review and actual-model baseline receipts.
- Acceptance: Automatic continuation and durable support controls pass without duplicate identity, scope leakage or human override loss.

## Closeout

- Planned: Deliver the authorized slice and its governed acceptance boundaries.
- Shipped: Automatic quiet/closed-session scheduling, exact partitions and bounded fair scans, current page coverage, supported canonical handovers, change-key deduplication and no-charge metadata refresh. Minimal exact permitted excerpts preserve original provenance/class/scope, avoid loops and participate in intent-ledger rollback recovery, final-support expiry, original erasure and old-backup fences.
- Not shipped: Running-stack deployment and fresh installed-host upgrade are separate. Missing capture is not reconstructed; digests have explicit bounded coverage. No release, commit, push or deployment is included.
- New blockers: None for this local slice.
- Docs updated: Governing contracts, operating runbooks, research/evaluation evidence, owner epics and active/archive indexes.
- Validation: Six native automatic-session/digest scenarios pass, including staged response loss boundaries, recovery/budget omissions, settled/late/no-charge behavior, pending leased rollback artifact cleanup/erasure/restore, final excerpt expiry with preserved control and scanner rotation beyond 100 sessions. Independent storage review found no remaining source blocker; controlled verdict fixtures prove lifecycle rather than model reasoning. [Dated evidence and limitations](../../../mappings/memory-source-support-staging-2026-10-07.md).
- Version: N/A; no release requested.
- Commit: uncommitted; local working-tree implementation.
