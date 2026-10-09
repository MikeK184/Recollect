# Semantic discovery before canonical qualification

Status: shipped
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Reduce semantic discovery's repeated full-population qualification under
  the Brain lock while preserving the exact set of new eligible inputs.
- Non-goals: Change models, permissions, retry rules, representations or graph truth.
- Delivery shape: Rust-owned SQL selection, native integration controls and
  sanitized read-only normal-data query measurements.

## Governing Sources

[Semantic contract](../../../contracts/retrieval-semantic.md),
[source support](../../../contracts/memory-source-support-verification.md),
[engineering principles](../../../foundation/engineering-principles.md),
[epic](../../epics/hybrid-retrieval.md).

## Scope

- In scope: Both semantic status and maintenance discovery consumers; current
  profile identity exclusion before canonical predicates; exact PK metadata lookup;
  reconcile older positive fixtures with the accepted source-support gate and
  preserve direct history inspection separately from model-facing recall.
- Out of scope: General model admission, automatic entity/topic publication and
  changes to canonical support functions.
- Blockers: None for the established selection contract. Normal graph performance
  remains a separate acceptance gate in concurrent graph preparation.

## Surface and Interface Changes

- Interfaces: Existing status/maintenance behavior and API types remain.
- Storage: N/A — use current semantic identity indexes; no persisted eligibility cache.
- Ownership: Rust semantic queue owns selection; PostgreSQL enforces RLS/current gates.

## Data and Authority

- Inputs: Current source chunks, claim revisions, repository facts/manifests,
  policy content classes and active-profile represented identities.
- Authority: Current authenticated transaction and existing Brain lock, role,
  policy, canonical eligibility and subsequent worker publication checks.
- Blind spots: This removes redundant eligibility work, not all background locks.

## States and Edge Cases

- Loading: Existing pending/queued/running status and discovery coverage.
- Empty: No missing eligible identities means no discovery or new model attempt.
- Error: Existing database errors; no fallback to unqualified inputs.
- Blocked: Capacity/policy/retry behavior remains unchanged.
- No-access: All cohorts and canonical lookups retain Brain RLS.
- Duplicate or replay: Existing identities in any state exclude rediscovery;
  historical profiles do not exclude a new profile's work.
- Stale data: Canonical gates still run on each missing input; worker rechecks
  before model transmission/publication. Current correction/fences remain binding.
- Reconciliation divergence: Compare old/new canonical identity sets in a single
  disposable transaction, including removed/failed entries and a new profile.

## Integrations and Runtime Inputs

- Providers: Repository-owned PostgreSQL; local fake provider only for native controls.
- Environment: Existing database variables; no new model configuration.
- Secrets: Profiles retain only sanitized plans, counts and timing; no source text,
  tokens, credentials or raw database errors in receipts.
- Failure handling: Existing statement/transaction limits and durable retry rules.

## Tests and Acceptance

- Automated: Missing-profile/new-profile/all-state exclusion, unavailable-input
  positive control, RLS isolation and existing semantic lifecycle/retry controls;
  formatting, `git diff --check`, `./scripts/validate.sh`.
- Manual: Old/new actual normal-data query plans within read-only app-role
  transactions, preserving data and making no provider calls.
- Acceptance: Identical qualified input identities across counterfactual controls;
  missing cohorts exclude represented rows before support evaluation; no capacity,
  current-input or replay regression. Report measured timings without claiming
  they alone solve normal graph concurrency.

## Closeout

- Planned: Query repair, canonical equivalence and real-data measurements.
- Shipped: Materialized missing-input cohorts and exact metadata qualification
  in Rust-owned semantic status/maintenance SQL; native counterfactual controls,
  reconciled support fixtures, real MCP navigation and checkpointed local deployment.
- Not shipped: Normal graph reliability and full automatic entity/topic organization
  remain owned by their active graph/operations packs, not acceptance for this
  bounded semantic discovery repair.
- New blockers: None.
- Docs updated: Semantic contract, owning epic, epic index and active index.
- Validation: All 22 semantic controls pass across focused runs, including the
  strengthened 101-unavailable-prefix/all-state/RLS counterfactual. Five older
  recovery failures reproduce with the previous query and consumers; fixtures
  now use authenticated review and preserve direct historical inspection while
  model-facing recall honors current correction. App-role normal-data queries
  improve from 17.7–19.6 seconds to 225–334 ms; a repeatable-read comparison
  returns the same eight canonical IDs. Actual Rust Analyzer/Graft MCP proof and
  governance validation pass. Runtime graph acceptance remains separate.
  The new normal image is ready at migration 046, built at 13:07:42Z; checkpoint
  04fc5bcb-362f-4a31-9e28-3ce02b188229 and all original data IDs are preserved.
  Longer normal graph proof passes 37/40, retaining three database failures and
  p95 3.520 seconds; support-audit discovery remains the next measured seam.
- Version: N/A — local development.
- Commit: uncommitted.
