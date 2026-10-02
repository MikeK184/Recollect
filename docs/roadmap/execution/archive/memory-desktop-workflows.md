# Readable memory and standing Brain settings

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Make learned knowledge readable and optional correction easy while keeping policy, capture and retention decisions in Settings.
- Non-goals: Mandatory human review queue, human to-do app, arbitrary provider installation, auto-saving Ask answers and changes to independent states or erasure semantics.
- Delivery shape: Local capability-owned frontend/API changes and focused proof, no external deployment.

## Governing Sources

- [Desktop ADR](../../../adr/0014-desktop-experience-and-answers.md) and [desktop contract](../../../contracts/desktop-experience.md)
- [memory-claims-and-time](../../../contracts/memory-claims-and-time.md)
- [memory-review-and-corrections](../../../contracts/memory-review-and-corrections.md)
- [memory-retention-and-erasure](../../../contracts/memory-retention-and-erasure.md)
- [memory-provider-policy-and-learning](../../../contracts/memory-provider-policy-and-learning.md)
- [memory-procedures-and-handovers](../../../contracts/memory-procedures-and-handovers.md)
- [memory-autonomous-maintenance](../../../contracts/memory-autonomous-maintenance.md)
- [Owning epic](../../epics/memory-lifecycle.md)

## Scope

- In scope: Memory All/Decisions/Procedures/Handovers and exact support/history; optional review/correction/withdraw/conflicts/erase; handover generation from exact claim revisions; General/Access/AI & automation/Capture/Retention settings and bounded literal assertion search.
- Out of scope: Mandatory human review queue, human to-do app, arbitrary provider installation, auto-saving Ask answers and changes to independent states or erasure semantics.
- Blockers: None; accepted desktop/domain contracts resolve behavior. Shell integrates through its predecessor slice; final browser acceptance requires that integration.

## Surface and Interface Changes

- Interfaces: Existing memory/policy/capture/retention APIs; claim-list q trimmed up to 200 UTF-8 bytes, literal case-insensitive containment over subject/predicate/value/rationale before pagination. Policy adds answering purpose off by default under answer contract.
- Storage: No memory-policy reset or new transcript store. Existing immutable policy revisions/lifecycle/audit remain; answer metadata migration belongs to retrieval slice.
- Ownership: memory-lifecycle owns these feature views and canonical domain handlers; platform owns shared tokens/shell/components.

## Data and Authority

- Inputs: Canonical claims/revisions/reviews/support, exact handover contributors, effective grants, installed model metadata and current policy revision.
- Authority: Canonical memory mutation and model/retention contracts; actual role, scope and optimistic version checks remain server-owned.
- Blind spots: Claim confidence or policy acceptance is not human review/deployment proof; installed provider identity is read-only and configuration is not observed connection.

## States and Edge Cases

- Loading: Readable list/inspector and actual generation/maintenance status; only active section queries mount.
- Empty: No memory versus no assertion matches and no eligible handover inputs are distinct.
- Error: Preserve safe unsaved policy fields with stale-version recovery; clear invalid protected bodies on failed reads.
- Blocked: Disabled model purposes, denied content, quota and unavailable supporting bytes show the actual prerequisite, not a generic disconnected badge.
- No-access: Readers inspect permitted summaries and exact evidence; policy/edit/review/erase controls preserve real role and archive constraints.
- Duplicate or replay: Reuse canonical command IDs and handover lifecycle; navigation never auto-learns, saves an answer or resubmits paid generation.
- Stale data: Refetch exact current policy before mutation; shared invalidation/epoch/expiry clears dependent context and inspectors.
- Reconciliation divergence: Independent review/freshness/operational state and fact/knowledge times remain visible; no newest-wins conflict resolution.

## Integrations and Runtime Inputs

- Providers: Current installed provider/model through the existing gateway; no new adapter or automatic fallback.
- Environment: Existing model configuration only; secrets stay environment-backed and are never shown in settings.
- Secrets: Preserve existing credential transport/redaction; no secret values in assets, URLs, fixtures, docs or output.
- Failure handling: Keep model timeout/quota/uncertain-attempt semantics; explicit new retries remain separately charged and attributed.

## Tests and Acceptance

- Automated: Memory q before pagination including literal special characters and cross-Brain controls; existing correction/rejection/erasure/handover/policy/capture regressions; default-off answering compatibility; frontend and governance checks.
- Manual: Read a decision/procedure/handover, inspect support/history, make an authorized correction, generate from exact revisions and verify standing settings/retention descriptions without enabling existing user policies.
- Acceptance: Autonomy requires no new daily clicks; all optional interventions remain reachable and enforced; policy/capture/transmission/retention remain separate; source/claim validity and useful positive records survive redesign.

## Closeout

- Planned: Memory All/Decisions/Procedures/Handovers and exact support/history; optional review/correction/withdraw/conflicts/erase; handover generation from exact claim revisions; General/Access/AI & automation/Capture/Retention settings and bounded literal assertion search.
- Shipped: Readable memory, exact support/history, optional interventions, handovers and standing settings are delivered and deployed. Fixed lineage correction's missing scope catalogue, which otherwise disabled Save.
- Not shipped: No remaining slice acceptance; automatic learning remains the default and human review remains optional.
- New blockers: None for this slice.
- Docs updated: This pack, owning epic, active/archive and epic indexes, handoff, relevant runbooks and [final acceptance evidence](../../../mappings/desktop-final-acceptance-2026-10-01.md).
- Validation: Claims, Retention, Review and Procedures pass in the 65-case matrix. Two separately opted-in real-provider browser tests prove model settings and autonomous learn/revise/handover without human review. The new correction regression and deployed evidence-to-correction-to-recall journey pass. Final repository checks are recorded in the linked evidence.
- Version: N/A: no release requested.
- Commit: Uncommitted.
