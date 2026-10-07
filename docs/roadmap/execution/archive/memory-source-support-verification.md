# Source support verification

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Unsupported generated or unreviewed memory cannot become usable knowledge or replace supported knowledge.
- Non-goals: Truth guarantees, automatic command execution, digest/brief/cross-session discovery phases.
- Delivery shape: Local Rust, migration, handler/database/model proofs and reconciled docs.

## Governing Sources

- [Support contract](../../../contracts/memory-source-support-verification.md)
- [Autonomous ADR](../../../adr/0006-autonomous-memory.md)
- [Provider gateway](../../../contracts/memory-provider-policy-and-learning.md)
- [Retention](../../../contracts/memory-retention-and-erasure.md)
- [Claims](../../../contracts/memory-claims-and-time.md)
- [Memory owner](../../epics/memory-lifecycle.md)

## Scope

- In scope: Typed staging and versioned assessment; publication guard; legacy/direct audit; every consumer; protected human authority; privacy/recovery; structured and real-model proofs.
- Out of scope: Later automatic digest, context brief and family discovery implementation.
- Blockers: None; full-plan implementation authorizes this settled contract.

## Surface and Interface Changes

- Interfaces: Internal canonical learning-stage and verification-revision model inputs; inspectable support reasons; no arbitrary verifier API or approval queue.
- Storage: Migrations 036–038; immutable bounded stages, indexed typed verdicts, exact input relations and per-revision support assessments under Brain RLS; preserve existing histories.
- Canonical assessment extension: Migration 037 stores revision/policy/verifier generations. Initial learning publication records its existing supported verdict atomically; direct/legacy and generated handover revisions use verification-only model work without changing author-owned text. Reserve four of each twenty coordinator admissions for audits so ongoing capture cannot starve them. Audit the oldest unchecked current revisions first, at most four per pass; completed generations are not resubmitted unchanged.
- Handover publication: Migration 038 stores prepublication normalized handover drafts and whole-content verdicts with separate synthesis/assessment receipts and frozen run/policy/contributor/base identities. Only supported drafts append with their canonical assessment in one transaction. Unsupported refreshes preserve the old head; saved stages/verdicts and assessment-only replacements recover without repeat synthesis. Include stages in privacy closure, replay, logical deadlines and Brain deletion.
- Consumer boundary: A shared SQL predicate traverses required contribution identities, validating current assessment or authenticated human decision on every revision. Rust base views call that same predicate; SQL retrieval, semantic selection, graphs, conflict detection and reuse apply it before limits. History remains an inspection surface and cannot bypass model-input eligibility. Assessment changes advance the canonical memory epoch in the same transaction.
- Audit recovery: Separate immutable operation keys per attempt; known completed transient/invalid requests permit two linked replacements at five/thirty minutes. No-call budget/slot denial resumes the same generation. Database recovery reuses committed verdicts; a completed/unknown request with no retained verdict is terminal result-unavailable. Exact policy/current revision/dependencies/lease are rechecked before commit. Derived reasons participate in source/contributor privacy closure and older-journal replay.
- Ownership: Memory support owns semantic disposition; learning/handovers own staging and canonical publication; gateway owns all admission/accounting; retrieval/graph consume shared eligibility.

## Data and Authority

- Inputs: Exact retained sources with canonical role/time/scope, normalized ClaimContent and offered target revisions; supported/contradicted/insufficient indexed verdicts.
- Authority: Standing Brain policy and current authenticated author/lease; canonical human decision alone grants the protected exception.
- Blind spots: Support assessment is interpretation of available evidence, not production proof or independent corroboration; no source means no usable automatic claim.

## States and Edge Cases

- Loading: Persist typed extraction before assessment; use bounded batches and no network inside transactions.
- Empty: Complete empty extraction with no verifier call.
- Error: Malformed or incomplete verdict cannot publish; retained request metadata distinguishes failed/uncertain/lost response.
- Blocked: Unsupported assertions remain inspectable, create no user task, preserve old supported target.
- No-access: Current policy/access/retention denial suppresses output; cannot use verifier input as a normal context bypass.
- Duplicate or replay: Distinct extraction/assessment keys; committed stages/verdicts resume without repeating charged calls.
- Stale data: Exact policy/base/support/manifest/lease rechecked before atomic publication; stale generation suppressed.
- Reconciliation divergence: Whole correction/retirement must be supported; positive members may publish without a withheld member displacing a target.

## Integrations and Runtime Inputs

- Providers: Existing installed OpenAI adapter behind the canonical gateway; mocked HTTP for handler proofs and isolated synthetic actual model measurement.
- Environment: Existing provider environment configuration only; no new credentials.
- Secrets: Deterministic exclusion before transmission; no provider bodies or customer evidence in fixtures/results.
- Failure handling: Same-work no-call deferral for budgets/slots; separately accounted bounded completed-failure replacements; unknown/lost outcomes never blind replay.
- Stage recovery: A completed assessment failure inherits saved extraction, frozen targets and the original lifetime into its linked replacement. Publication/database faults use bounded same-generation native worker recovery. Tests must drive the normal scheduler and worker without manually resetting work states.

## Tests and Acceptance

- Automated: Supported and unsupported full assertions/procedures/targets; exact verdict index sets; direct/legacy/human/child controls; all recall/model/semantic/graph consumers; crash boundaries, budget deferral, concurrency, expiry and privacy replay. Run focused Rust/database tests and `scripts/validate.sh`.
- Manual: Inspect actual synthetic gateway request receipts and frozen scorer outcomes; local stack and UI independent review if UI changes.
- Acceptance: Every contract gate and frozen semantic threshold proven; no fixture-only semantic claim, no loss of supported unaffected memory, no blind replay charges.

## Closeout

- Planned: Deliver the authorized slice and its governed acceptance boundaries.
- Shipped: Typed learning/handover stages, source-support-3 whole-assertion checks with exact admitted evidence quotation, canonical pre-ranking consumer guard, bounded direct/legacy audits, human/contributor authority, saved-result recovery and transitive erasure/restore. Qualified expired direct evidence remains investigable only for authenticated reviewed retained claims; strict modes, erasure and selected manifests retain their gates.
- Not shipped: External semantic truth is not guaranteed; fixed-corpus assertion quality does not measure every typed producer action. No deployment/cutover is claimed. No release, commit, push or deployment is included.
- New blockers: None for this local slice.
- Docs updated: Governing contracts, operating runbooks, research/evaluation evidence, owner epics and active/archive indexes.
- Validation: All 29 controlled native support tests and 20 server library tests passed, including supported/unsupported mixed batches, direct/legacy audits, cache/semantic/graph consumers, near-limit windows, exact manifests, human/contributor controls, leased recovery and old-backup replay. The inherited assessment replacement additionally retains its permitted excerpt with only three calls. Frozen v3 actual-model support comparison and independent receipt audit pass; final retention/deadline regression and governance checks pass. [Dated evidence and limitations](../../../mappings/memory-source-support-staging-2026-10-07.md).
- Version: N/A; no release requested.
- Commit: uncommitted; local working-tree implementation.
