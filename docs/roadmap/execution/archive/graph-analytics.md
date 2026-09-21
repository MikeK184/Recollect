# Scoped queued graph analytics

Status: shipped
Owning epic: `docs/roadmap/epics/graph-intelligence.md`
Work type: product

## Summary

- Goal: Queue usable centrality, communities and connected-component reports on exact eligible graph inputs.
- Non-goals: Custom algorithms, Enterprise features, speculative automatic analysis and interactive canvas.
- Delivery shape: Local Rust/React implementation, migration 020, actual GDS/PG/browser proof and documentation.

## Governing Sources

[Analytics contract](../../../contracts/graph-analytics.md),
[ADR 0007](../../../adr/0007-canonical-graph-projections.md),
[graph epic](../../epics/graph-intelligence.md),
[canonical traversal](../../../contracts/graph-projection-and-traversal.md),
[combined inputs](../../../contracts/graph-cross-repository-views.md),
[retention](../../../contracts/memory-retention-and-erasure.md).

## Scope

- In scope: Native fixed GDS recipes, full input qualification, queue/resource limits, invalidation, owned catalog cleanup, reports/API/UI and recovery integration.
- Out of scope: Source ingestion changes, model calls, external deployment, custom graph algorithms and slice 19 rendering.
- Blockers: None. Local implementation, recovery, browser and normal runtime acceptance completed on 2026-09-22.

## Surface and Interface Changes

- Interfaces: Analytics queue/list/view API and protocol types, graph panel report controls, shared durable cancel and fresh-report retry behavior.
- Storage: Add migration 020 for reports/attempt ownership and analytics epochs; preserve already applied migrations. Extend private journal ownership using the existing durable writer and Query API adapter.
- Ownership: Graph analytics owns derived reports/scratch; canonical graph qualification and privacy/workers remain shared authorities.

## Data and Authority

- Inputs: Exact canonical graph selection/generations, full qualified entity/edge sets, epochs, coverage, retention and current reader/worker authority.
- Authority: PostgreSQL canonical state and existing scoped reads; GDS calculates only admitted topology. No score creates belief.
- Blind spots: Partial extraction and unresolved links stay visible. Fixed PageRank iterations do not prove convergence; communities are report-local.

## States and Edge Cases

- Loading: Explicit queued/running stage and job progress, bounded read pages.
- Empty: Empty eligible graph refuses analysis with an actionable reason; an isolate is a valid input.
- Error: Bounded static backend/admission errors and cleanup pending; no successful prefix or raw backend payload.
- Blocked: Oversized selection or uncertain scratch cleanup prevents allocation; recall/capture remain independent.
- No-access: Browser Brain role and current paired retrieval-operation scope are rechecked; revoked publication is withheld.
- Duplicate or replay: Shared command receipts; retries have distinct report/attempt identities and exact owned scratch names.
- Stale data: Canonical mutation/clock expiry invalidates the entire score set, even when only a hidden contributor changed or a withheld conflict expired; record a conservative Brain deadline and complete-set requalification.
- Reconciliation divergence: Journal replays cleanup after older PG restore; guarded creation/attempt deadlines prevent delayed scratch resurrection. Never delete unowned graphs.

## Integrations and Runtime Inputs

- Providers: Existing local Neo4j/GDS Community and PostgreSQL; no paid model request or customer mutation.
- Environment: Existing `NEO4J_URL`, `NEO4J_USERNAME`, `NEO4J_PASSWORD`, `DATABASE_URL`, `RECOLLECT_ERASURE_JOURNAL`; no new credential.
- Secrets: Environment-only credentials, bounded sanitized errors; journal/report descriptors contain canonical identities, not source bodies.
- Failure handling: Existing heavy lane/lease/retry, 120-second attempt, 90-second compute query, short control calls, exact transaction termination and verified catalog eviction.

## Tests and Acceptance

- Automated: Actual GDS numeric/group fixtures, scope/hidden-contributor invalidation, same-epoch materialization/clock expiry, paired authority, size/estimate refusal, cancel/lease/crash/late creation and privacy restore cleanup; browser report/stale/error/mobile proof, relevant regressions and `./scripts/validate.sh`.
- Manual: Inspect browser screenshots and normal local UI; preserve seven existing Brains, exact SWEG graph/recall and model counters through migration/restart.
- Acceptance: Every criterion in the contract must have executed evidence. Record focused checks and limitations; archive only after full slice acceptance passes.

## Closeout

- Planned: Complete bounded native graph analytics and lifecycle/UI proof above.
- Shipped: Fixed native PageRank/Leiden/WCC reports, complete input qualification, whole-result invalidation, scoped API/UI, serialized final reads, durable exact scratch ownership, delayed-creation fencing, cancellation and restore cleanup. Migration 020 is applied to the normal Mac instance; its SWEG WCC report is ready with 99 vertices, 183 relationships, GDS 2026.08.1 and confirmed cleanup.
- Not shipped: Contract deferrals only: custom/weighted algorithms, historical aggregate access, speculative automatic analysis, Enterprise features, external deployment, slice 19 rendering and slice 20 fusion. Abrupt worker-task interruption was tested; an operating-system process-kill drill is not claimed.
- New blockers: None.
- Docs updated: Analytics contract, interface mapping, operating runbook, epic, indexes, continuation record and this archived pack.
- Validation: Eleven analytics scenarios are included in 90 passing PostgreSQL/GDS platform tests (283.55 seconds; unrelated live OIDC test excluded). Workspace tests: 13 passed; Clippy with warnings denied passed; 124 API operations generated; documentation governance: 32 passed. Analytics browser proof passed in 23.9 seconds, including a long label and mobile layout; two existing graph browser regressions passed in 17.7 seconds. Real native GDS work was observed, terminated through product cleanup and confirmed absent after eventual retirement. Actual older-PostgreSQL replay, immutable journal mismatch, both reader interleavings, clock boundaries, interrupted tasks and replacement leases passed. Normal runtime and inspected screenshots preserve seven Brains, the exact SWEG graph/fact/recall, autonomous replacement/capture recall and all 33 model requests, with zero new model calls or customer file reads. Logs and private pre-020 backups are in `.cache/analytics-proof/resume-20260922/`; earlier failures remain diagnostic history.
- Version: N/A; no release policy or release requested.
- Commit: uncommitted.
