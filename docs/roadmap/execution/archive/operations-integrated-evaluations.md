# Integrated first-product evaluation

Status: shipped
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: Demonstrate the complete scoped product and publish measured useful
  retrieval/generation and concurrent operating limits.
- Non-goals: External deployment, mobile, general chat, universal scale or ranking claims.
- Delivery shape: Opt-in owned runtime harness, meaningful regression/desktop proof,
  evidence matrix and final operator handoff.

## Governing Sources

- [Integrated acceptance](../../../contracts/operations-integrated-evaluations.md)
- [Installation](../../../contracts/operations-local-and-shared.md)
- [Recovery](../../../contracts/operations-recovery-drills.md)
- [Fusion](../../../contracts/retrieval-graph-fusion.md)
- [Handovers](../../../contracts/memory-procedures-and-handovers.md)
- [Owning epic](../../epics/operational-readiness.md)

## Scope

- In scope: Full contracted workflow/quality/concurrency proof and required fixes.
- Out of scope: New engines, production changes, release/commit and mobile work.
- Blockers: None; predecessors are shipped and model credentials are authorized.

## Surface and Interface Changes

- Interfaces: Operator evaluation command and content-free report; existing product endpoints.
- Storage: Private proof state and synthetic fixtures; migration 026 adds an internal
  Brain admission identity and transaction guard after the workload exposed writer starvation.
- Ownership: Existing Rust mutation/provider/graph/runtime services retain all decisions.
- Query repair: Reuse authorized Brain capture associations/source knowledge within
  each canonical recall statement, preserving original capture scope and receipt
  time. Compare actual query plans/results and re-run affected eligibility checks.
  Bind source-lineage joins to their unique identities so a fresh import cannot
  turn each candidate's parent/capture lookup into a whole-Brain scan.

## Data and Authority

- Inputs: Declared synthetic corpus, generated proof accounts, approved model configuration.
- Authority: Existing canonical policy/scope/retention/review records and product gateways.
- Blind spots: Single local workload and annotated model sample; no generalization to production.

## States and Edge Cases

- Loading: Source processing, bounded synthesis and heavy analytics queues.
- Empty: Unsupported questions and no eligible synthesis contributors remain explicit.
- Error: Failed provider, load or proof is recorded; no automatic paid replay.
- Blocked: Missing current inputs, resources or authority cannot count as a successful capability.
- No-access: Cross-Brain/private task/profile and forged-review controls with positive counterparts.
- Duplicate or replay: Preserved operation/request identities and prior accepted recovery semantics.
- Stale data: Exact revisions, late evidence, correction and graph invalidation remain visible.
- Reconciliation divergence: Check accepted events against final canonical evidence and queue state.

## Integrations and Runtime Inputs

- Providers: Existing PostgreSQL/pgvector, Neo4j/GDS, OpenAI gateway and owned MCP/Vault fixtures.
- Environment: Selected saved installation; explicit ignored OpenAI input, no ambient Vault access.
- Secrets: Never printed or committed; provider use only for the declared synthetic quality Brain.
- Failure handling: Existing timeouts/admission; durable proof state and no silent retries.

## Tests and Acceptance

- Automated: Contracted 50 repositories/200 documents/eight callers, eight-question
  four-lane comparison, production-handler regression matrix and desktop workflows.
- Manual: Inspect synthetic generated outputs against fixed facts and desktop screenshots.
- Native fixture timing: Completion polling covers the existing 30-second startup
  and 30-second fixture call deadlines plus 15 seconds of queue/poll overhead;
  the previous 15-second test wait could abandon a valid starting call. Product
  and mixed-workload limits are unchanged.
- Acceptance: Contract gates pass, limits and carried evidence remain explicit, governance reconciled.

## Closeout

- Planned: Complete integrated acceptance and measured operating/quality report.
- Shipped: Opt-in quality/workload harness, seven-capability proof, actual-model
  comparison, unchanged mixed workload, migration 026 writer admission and bounded
  recall queries, final-image encrypted restore/desktop proof and preserved normal
  installation. All six full-fusion targets and five supported answers passed;
  recall/capture p95 were 863/441 ms with all captures readable within 8.1 seconds.
- Not shipped: No required acceptance remains. Mobile, external deployment,
  production capacity, high availability and broader model comparisons are outside
  scope. The recorded answer miss, unsupported semantic context, backpressure and
  4.9-second recall tail remain explicit limits.
- New blockers: None.
- Docs updated: Contract, retrieval admission contract, this pack, operational
  epic and indexes, README/handoff, [evaluation runbook](../../../runbooks/integrated-evaluation.md)
  and [dated evidence](../../../mappings/integrated-evaluations-2026-09-26.md).
- Validation: 133 distinct noninteractive platform cases with documented focused
  reruns; ten affected cases after the final lineage query change; four actual
  native/coding-host cases; 26 workspace passes; all-target Clippy/formatting;
  standard image/OpenAPI/TypeScript/Vite build; actual 120-capture/120-reader
  workload; final encrypted restore; both desktop workflows together (18.0 s);
  normal migration/inventory/SWEG proof; governance and diff/format checks recorded
  in the dated evidence. Ignored and carried integration checks are not fresh proof.
- Version: N/A: no release policy.
- Commit: Uncommitted.
