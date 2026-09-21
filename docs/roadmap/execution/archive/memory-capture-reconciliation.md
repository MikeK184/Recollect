# Capture provenance and autonomous reconciliation

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Preserve the meaning, origin and time of captured evidence while allowing
  supported autonomous maintenance across bound session events and at capacity.
- Non-goals: Universal semantic merging, adaptive ranking and external deployment.
- Delivery shape: Local Rust protocol/server changes, focused proof and reconciled
  documentation; one audit repair slice in addition to the original 29.

## Governing Sources

- [Capture reconciliation contract](../../../contracts/memory-capture-reconciliation.md)
- [Autonomous maintenance](../../../contracts/memory-autonomous-maintenance.md)
- [Session capture](../../../contracts/evidence-session-capture.md)
- [Claims/time](../../../contracts/memory-claims-and-time.md)
- [Retention/erasure](../../../contracts/memory-retention-and-erasure.md)
- [Retrieval](../../../contracts/retrieval-exact-and-lexical.md)

## Scope

- In scope: Canonical model attribution, bound session targets, capacity accounting,
  first-party capture feedback prevention and late-capture temporal correctness.
- Out of scope: Arbitrary cross-source truth resolution or new model providers.
- Blockers: None; existing user authorization resolves the bounded details above.

## Surface and Interface Changes

- Interfaces: Preserve existing host/API commands and source-line spans. Text
  model input adds canonical provenance alongside `data`. UI continues to use
  canonical source, claim history, learning status and coverage surfaces.
- Storage: Keep independent capture events and current transactional memory
  append/dependency tables. Preserve server knowledge time separately from raw
  capture retention time, including existing captured versions. Any additive
  migration is registered and tested against populated storage.
- Ownership: Protocol owns normalized exclusions; server owns source attribution,
  candidate authority, canonical publication and temporal reads.

## Data and Authority

- Inputs: Authorized source artifacts, capture events/bindings and current claims.
- Authority: Accepted contract; server-derived actor/binding/selection and exact
  evidence identities. Atlas/Cognee remain read-only reference checkouts.
- Blind spots: Models can misinterpret text; timestamps and shared sessions do
  not establish truth, and arbitrary copied/paraphrased content is not detectable.

## States and Edge Cases

- Loading: Existing durable capture and model work report queue/processing state.
- Empty: Excluded events retain content-free coverage; no targets means new-claim
  extraction under current rules, not implicit unrestricted reconciliation.
- Error: Capacity failures atomically discard canonical publication; provider
  outcomes retain existing accounting and safe failure codes.
- Blocked: Disabled policy, uncertain provider calls and unavailable support stay
  visible while independent work proceeds.
- No-access: Recheck current Brain/device grants before sending or publishing.
- Duplicate or replay: Existing capture deduplication, immutable input lists and
  durable job/request identity prevent duplicate canonical publication after restart.
- Stale data: Recheck offered revisions, human decisions and erasure at publication;
  late source evidence remains unavailable to earlier knowledge-time queries.
- Reconciliation divergence: Preserve uncertain alternatives; no newest-wins rule.

## Integrations and Runtime Inputs

- Providers: Existing OpenAI gateway; controlled local HTTP for deterministic proof.
  Selected `gpt-5.6-luna` and `text-embedding-3-large` at 3,072 dimensions persist.
- Environment: Existing ignored `.env`; `OPENAI_API_KEY` stays outside output.
- Secrets: Use canonical redaction and safe model-payload validation for metadata
  as well as text. No new customer content is sent to providers.
- Failure handling: Existing bounded model concurrency, deadlines, charged-attempt
  rules, durable retries and deletion fences remain authoritative.

## Tests and Acceptance

- Automated: Controlled HTTP/real PostgreSQL tests for provenance and session
  reconciliation, 5,000-identity maintenance and mixed rollback, protocol and server
  recall exclusions, delayed capture/history/retention, erasure and stale work.
  Run affected libraries, platform suite, Clippy and repository validation.
- Manual: Inspect existing synthetic and SWEG local views read-only after an owned
  runtime restart. Preserve all existing Brains and model usage; no customer writes.
- Acceptance: Every acceptance item in the governing contract has passing evidence.
  Record actual tests and remaining limits before archival.

## Closeout

- Planned: The bounded capture/learning repairs and their current-consumer proof.
- Shipped: Canonical source/capture provenance at the model boundary, bounded
  same-binding/session reconciliation, exact identity-capacity accounting,
  native/server recall-feedback exclusions, independent capture/knowledge time,
  current publication/expiry guards and nonblocking lease renewal.
- Not shipped: Arbitrary cross-document merging and deferred Atlas optimizations.
- New blockers: None.
- Docs updated: Capture/provider/reconciliation contracts, capture/recall runbooks,
  Atlas/retrieval mappings, README, owning epic and roadmap/execution indexes.
- Validation: Final real PostgreSQL/API platform run passed 44 tests in 99.84s;
  eight native/protocol library tests, Clippy, generated API/build and 32 governance
  tests passed. Capture, claims and recall browser tests passed. The normal runtime
  applied migration 016 and preserved existing Brains. Four synthetic native capture
  events used real Luna responses (15,010 tokens): suggestion/history preserved the
  original fact, explicit tool evidence revised it under standing policy, and no
  operational verification was invented. Temporary credentials were removed; no
  customer files were read or sent. Detailed controls and limits are in the
  [Atlas audit](../../../mappings/atlas-implementation-audit-2026-09-15.md).
- Version: N/A; no release policy.
- Commit: Uncommitted.
