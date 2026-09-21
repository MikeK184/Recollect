# Procedures, handovers and autonomous maintenance

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Deliver evidence-backed procedures and multi-repository handovers with autonomous learning, revision and maintenance; human review is optional.
- Non-goals: Executing procedures, recursive/cross-environment composition and retrieval/graph ranking.
- Delivery shape: Rust canonical memory extension, model worker, API/browser and local proof; uncommitted.

## Governing Sources

[Procedures/handovers contract](../../../contracts/memory-procedures-and-handovers.md),
[canonical memory ADR](../../../adr/0005-canonical-claims-and-time.md),
[provider contract](../../../contracts/memory-provider-policy-and-learning.md),
[retention contract](../../../contracts/memory-retention-and-erasure.md) and
[owning epic](../../epics/memory-lifecycle.md).

[ADR 0006](../../../adr/0006-autonomous-memory.md) and the
[autonomous maintenance contract](../../../contracts/memory-autonomous-maintenance.md)
incorporate the user's later clarification before dependent implementation.

## Scope

- In scope: Typed procedures/outcomes, handover contributions, canonical eligibility, synthesis, automatic source catch-up/reconciliation and handover refresh, erasure/restore and UI. Preserve history-linked job identities during terminal cleanup and keep model-job cancellation available.
- Out of scope: Execution permissions, automatic host capture and later retrieval/graphs.
- Blockers: None; routine bounded form, dependency and provider decisions are resolved.

## Surface and Interface Changes

- Interfaces: Additive memory forms/kind filter/contribution views; handover queue/list/explicit retry.
- Storage: Migrations 012/013 for contribution links, durable synthesis/reconciliation attempts, standing-policy scheduling, bounded retries and erasure/model-input fences.
- Ownership: Canonical memory/review/retention own authority; shared gateway/model lane own external calls.

## Data and Authority

- Inputs: Exact same-Brain contributors/evidence, combined scope, authored outcomes and current policy.
- Authority: Current authenticated actor/device or the standing policy grant, canonical eligibility and optional human override; generated content cannot confer human authority.
- Blind spots: Attributed test observations are not independent deployment proof; selected evidence bounds synthesis.

## States and Edge Cases

- Loading: Form/candidate fetch, queued/running generation and evidence inspection.
- Empty: Untested procedures, no contributions/results and disabled model policy are explicit.
- Error: Invalid outcomes, conflicting environments/spans, stale input and safe provider failure.
- Blocked: Budget/concurrency, unavailable source, lease loss or changed policy.
- No-access: Read/write separation, foreign IDs and exact paired operation scope enforced.
- Duplicate or replay: Canonical receipts and fixed attempt request IDs prevent silent charged replay.
- Stale data: Contributor changes qualify/exclude dependent handovers before use.
- Reconciliation divergence: Provider accounting is separate from canonical publication and erasure completion.

## Integrations and Runtime Inputs

- Providers: Existing Luna Responses gateway with fixed strict synthesis schema.
- Environment: Existing OPENAI_API_KEY/installed models and repository-owned databases.
- Secrets: Environment only; no raw request/response cache or payload logging.
- Failure handling: Durable model lane, lease heartbeat, current authority checks and explicit replacement attempts.

## Tests and Acceptance

- Automated: Compound procedure/contribution/review/retention and model-worker scenarios with positive controls, API/browser and restore replay.
- Manual: One small synthetic real-Luna handover plus normal-runtime UI/preservation.
- Acceptance: Every contract behavior and current consumer path verified before archival.

## Closeout

- Planned: Structured procedures/outcomes, multi-repository handovers, governed synthesis, autonomous learning/revision/maintenance, canonical lifecycle and UI.
- Shipped: Typed forms and observations, exact contributions, governed synthesis, standing-policy catch-up, policy acceptance, same-identity reconciliation, retirement, handover refresh, bounded retries, erasure/older-restore fences and browser controls. [Dated proof](../../../mappings/procedures-and-handovers-proof-2026-09-14.md) records the actual local runtime and synthetic demo with zero individual review decisions.
- Not shipped: No unmet slice acceptance. Session capture, retrieval/graphs and session-derived handovers retain their named successor slices. Procedure execution and universal semantic equivalence are explicit non-goals; uncertain provider outcomes remain visible and can require an explicit replacement attempt.
- New blockers: None.
- Docs updated: ADR 0006, autonomous/procedure/provider contracts and indexes, foundations, proof mapping, model/procedure/claims/review/local-development runbooks, README, historical Atlas amendment, epic and execution indexes.
- Validation: 33 API/database scenarios passed in 29.81s, clean Clippy, four browser regressions in 22.5s and one real-Luna autonomous browser scenario in 34.2s. The final normal-runtime demo verified automatic typed procedure learning, port revision and handover refresh with five successful calls and one preserved timeout; SWEG and the earlier demo passed read-only preservation. API generation verified 105 unique operations; TypeScript/Vite and CodeGraph checks passed. Governance lint and all 32 checker tests passed; see the dated evidence for commands, failures and limits.
- Version: N/A; no release requested.
- Commit: Uncommitted.
