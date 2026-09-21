# Exact and lexical context retrieval

Status: shipped
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Useful bounded recall over canonical memory/evidence with the same scope, correction and erasure authority.
- Non-goals: Semantic/graph retrieval, generated answers, host recall injection and the final combined investigation UI.
- Delivery shape: Native PostgreSQL indexes, one Rust recall handler, browser/native consumers and local proof; uncommitted.

## Governing Sources

[Exact/lexical contract](../../../contracts/retrieval-exact-and-lexical.md),
[claims/time](../../../contracts/memory-claims-and-time.md),
[review/correction](../../../contracts/memory-review-and-corrections.md),
[retention](../../../contracts/memory-retention-and-erasure.md),
[workspace scope](../../../contracts/evidence-workspace-scope.md) and
[owning epic](../../epics/hybrid-retrieval.md).

## Scope

- In scope: Exact/literal and PostgreSQL full-text candidates; immutable request scope; collection/manifest/time filters; canonical eligibility; raw-evidence correction filtering; bounded attributed context; missing/partial states; UI and native recall.
- Out of scope: Model transmission/embedding, ANN, graph candidates, external reranking, changing customer files and external deployment.
- Blockers: None; the contract resolves routine decisions and predecessors are delivered.

## Surface and Interface Changes

- Interfaces: POST Brain recall with explicit channels/mode/selection and context budget; native scope recall; basic browser recall panel.
- Storage: Migration 015 adds generated lexical vectors/GIN indexes to canonical rows; no retained search-copy cache or query payload history.
- Ownership: Existing claims/evidence/privacy remain canonical; indexes and ranking cannot admit ineligible content.

## Data and Authority

- Inputs: Current authenticated Brain, validated immutable operation or browser selection, optional exact identity/manifest/time and bounded query text.
- Authority: Current Brain grants and canonical evidence/state/rules. Strict recall requires accepted/current supported claims; raw material remains qualified evidence.
- Blind spots: Bounded textual rejection matching is not semantic equivalence; candidate caps and unprocessed/missing source text are visible coverage limits.

## States and Edge Cases

- Loading: Request in flight; canonical source processing may still be pending.
- Empty: No match or no sufficient eligible evidence under the selected mode.
- Error: Invalid identity/filter/channel, bounded timeout, database/artifact failure.
- Blocked: Inapplicable manifest, blocked raw assertion, unavailable source and strict eligibility requirements.
- No-access: Foreign Brain/resource and wrong actor/device/operation denied.
- Duplicate or replay: Read-only repeat rechecks authority; exact/lexical hits deduplicate canonical identities.
- Stale data: Freeze knowledge time and operation selection; current privacy and rules override old index/knowledge state.
- Reconciliation divergence: Report unprocessed sources, withheld/capped candidates and truncated/budget-limited results explicitly.

## Integrations and Runtime Inputs

- Providers: Existing PostgreSQL 17; no new model/external search service.
- Environment: Repository-owned normal runtime plus disposable real database/browser/native fixtures.
- Secrets: Queries/results never include configured credentials; no query text in logs or audit.
- Failure handling: Four concurrent recalls, ten-second operation bound, two-second SQL bound, safe overload/error states and fresh retry.

## Tests and Acceptance

- Automated: Real API/PostgreSQL recall corpus, positive/negative authority, correction/raw-copy/rebuild, time/manifest, missing/erased evidence, bounds and browser/native consumers.
- Manual: Normal-runtime recall in the synthetic and existing SWEG Brains, inspected desktop/mobile output and preserved source/model state.
- Acceptance: Every contract criterion passes; record measured corpus hit/latency evidence and qualified limitations before archival.

## Closeout

- Planned: Complete exact/lexical baseline and its usable clients.
- Shipped: Migration 015 and exact/lexical canonical recall, immutable native
  operation scope, time/manifest/collection filters, correction-aware raw evidence,
  canonical eligibility and legacy defaults, bounded source-diverse context,
  expiry/erasure guards and browser/native consumers.
- Not shipped: Semantic/graph retrieval, combined investigation UI and MCP host
  injection belong to their named successors. No exhaustive bounded-candidate,
  semantic superiority or external-deployment claim.
- New blockers: None.
- Docs updated: Contract, recall runbook, README, owning epic, roadmap/execution
  indexes and dated retrieval/Atlas proof mappings.
- Validation: PostgreSQL 17 Context7/official interface guidance verified. Final
  platform suite passed 44 tests in 99.84s, including native recall, copied-value
  rejection/rebuild, legacy defaults, historical erasure and timed contribution
  expiry. Eight library tests, Clippy, API generation/build and 32 governance tests
  passed. Recall/capture/claims browser tests passed. Fresh normal-runtime SWEG
  exact/lexical recall took 68/73ms with 1,584-byte context; six Brains and their
  model usage were preserved, with no new model calls or customer-file reads.
  Desktop/mobile screenshots were inspected. Corpus controls and explicit bounds
  are recorded in the [retrieval mapping](../../../mappings/retrieval-baseline-2026-09-15.md).
- Version: N/A; no release requested.
- Commit: Uncommitted.
