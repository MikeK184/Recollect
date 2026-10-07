# Bounded correction across sessions

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Automatically reconcile exact assertion families across sessions without newest-wins behavior.
- Non-goals: Universal semantic merging, personal inference, automatic command execution.
- Delivery shape: Local Rust discovery, typed stage metadata and controlled native proofs.

## Governing Sources

- [Cross-session contract](../../../contracts/memory-cross-session-reconciliation.md)
- [Source support](../../../contracts/memory-source-support-verification.md)
- [Autonomous maintenance](../../../contracts/memory-autonomous-maintenance.md)
- [Memory owner](../../epics/memory-lifecycle.md)

## Scope

- In scope: Post-extraction exact-family discovery, sole replacement/reuse hypotheses, frozen targets, bounded ambiguity/input coverage, current guards and recovery.
- Out of scope: Semantic similarity, unconstrained family retirement parsing and paid quality claims from fixtures.
- Blockers: None for local acceptance; deployment remains separate.

## Surface and Interface Changes

- Interfaces: Internal discovery metadata in existing typed learning stages; existing run reused/revised counters.
- Storage: Existing immutable JSON stage and run/input relations; compatible optional typed fields, no separate authority or migration.
- Ownership: Autonomous discovery selects hypotheses; support checks their relationships; learning commits canonical changes.

## Data and Authority

- Inputs: Normalized typed candidates and current exact families under standing Brain policy.
- Authority: Exact authenticated/current revision and shared support predicate, preserving human decisions/rules.
- Blind spots: Exact normalization does not infer semantic equivalents or missing historical qualifiers.

## States and Edge Cases

- Loading: Discover after extraction and freeze atomically with typed stage.
- Empty: No match means ordinary addition.
- Error: Invalid extractor-selected target fails existing shape validation.
- Blocked: Unsupported relationships preserve prior head; no mandatory review task.
- No-access: Current permission, retention and erasure prohibit selection/publication.
- Duplicate or replay: Sole same-value reuse allocates nothing; saved stage skips discovery.
- Stale data: Recheck exact target before every assessment/publication.
- Reconciliation divergence: Multiple matches/insufficient budget omit targets and record coverage.

## Integrations and Runtime Inputs

- Providers: Existing support assessor only; no discovery call/provider.
- Environment: Existing Brain scope/manifest and installed model configuration.
- Secrets: Existing payload exclusion; synthetic fixtures only.
- Failure handling: Inherit bounded native/stage-only recovery and non-replay of unknown outcomes.

## Tests and Acceptance

- Automated: New-session correction/reuse, unsupported and protected controls, scope/ambiguity/budget and frozen recovery.
- Manual: Independent source review and actual-model baseline receipts.
- Acceptance: Exact relationship support and all controls pass without duplicate identity or human override loss.

## Closeout

- Planned: Deliver the authorized slice and its governed acceptance boundaries.
- Shipped: Bounded exact-family discovery after extraction, sole frozen replacement/reuse hypotheses, ambiguity/budget coverage, whole-assertion/relationship verification and current publication rechecks. Corrections preserve identity/history, faithful equivalent reuse avoids a revision/TTL reset, unsupported updates preserve their old head and human rules remain protected.
- Not shipped: Universal semantic discovery and full typed producer-action statistical performance are not claimed. Actual installed runtime has not been upgraded. No release, commit, push or deployment is included.
- New blockers: None for this local slice.
- Docs updated: Governing contracts, operating runbooks, research/evaluation evidence, owner epics and active/archive indexes.
- Validation: Native exact-family fixture proves new-session correction, same-identity reuse and unsupported control with eight accounted calls; all 29 support scenarios pass surrounding human/rule/scope/manifest/deadline/frozen-target and recovery controls. Independent consumer/source review found no remaining material blocker. The actual-model assertion corpus is reported separately from controlled producer-action enforcement. [Dated evidence and limitations](../../../mappings/memory-source-support-staging-2026-10-07.md).
- Version: N/A; no release requested.
- Commit: uncommitted; local working-tree implementation.
