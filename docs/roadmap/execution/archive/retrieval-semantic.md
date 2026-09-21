# Governed semantic representations and recall

Status: shipped
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Find paraphrased engineering evidence through permitted embeddings with
  the same scope, correction, retention and attribution as exact/lexical recall.
- Non-goals: ANN, graph candidates, answer generation and host prompt injection.
- Delivery shape: Additive PostgreSQL projection, Rust gateway/worker/recall,
  browser/native controls, controlled and real-provider proof; uncommitted.

## Governing Sources

- [Semantic contract](../../../contracts/retrieval-semantic.md)
- [Exact/lexical contract](../../../contracts/retrieval-exact-and-lexical.md)
- [Provider policy](../../../contracts/memory-provider-policy-and-learning.md)
- [Autonomous maintenance](../../../contracts/memory-autonomous-maintenance.md)
- [Retention](../../../contracts/memory-retention-and-erasure.md)
- [Capture reconciliation](../../../contracts/memory-capture-reconciliation.md)

## Scope

- In scope: Defined canonical representations, standing automatic embedding,
  durable bounded batches, status/retry/reindex, semantic-only/combined recall,
  exact eligible-vector comparison, deletion/recovery and usable clients.
- Out of scope: New providers, dimension reduction, embeddings in Neo4j, customer
  transmission, unapproved fallback, commits and external deployment.
- Blockers: None; both original predecessors and the Atlas repair are delivered.

## Surface and Interface Changes

- Interfaces: Default-false `automatic_embedding` in existing model policy;
  GET semantic status, POST admin reindex and POST writer batch retry; recall
  `semantic` channel, `semantic_request_id` and optional `semantic_min_similarity`.
  Native recall channel options and browser policy/readiness/search controls.
- Storage: Migration 017 introduces Brain-RLS semantic profiles/active heads,
  canonical entry references, durable batches and per-entry full `vector(3072)`
  payloads. Entries identify exact canonical kind/input and parent dependencies;
  no retained duplicate source text, query cache or vector response history.
- Ownership: Gateway resolves canonical representations and records all input
  dependencies; semantic worker owns projections. Shared recall helpers retain
  scope, rules, availability, canonical views and final context/deadline authority.

## Data and Authority

- Inputs: Exact source chunks and retained artifacts, eligible current claim
  revisions, facts and manifests; bounded explicit query text and immutable scope.
- Authority: Current Brain/device grants, installed/approved model identity,
  current profile, canonical retention/correction and durable privacy journal.
- Blind spots: Similarity is not entailment; historical representations can be
  missing, deterministic text can be truncated and large scopes can exceed limits.

## States and Edge Cases

- Loading: Distinguish canonical processing, pending discovery, batch/model work
  and usable compatible embeddings; expose counts and partial coverage.
- Empty: No compatible scoped entries skips query transmission; no match passes
  through canonical insufficient-support/no-match semantics.
- Error: Wrong model/shape/dimensions, zero/nonfinite vectors, stale input/profile,
  unavailable artifacts, query timeouts and bounded-scan refusal have safe codes.
- Blocked: Policy/classes, budget, queue capacity and oversized inputs remain
  visible. Changed pre-admission constraints may resume without per-record review.
- No-access: Current Brain/actor/device/operation denial before transmission and
  after provider response; no cross-Brain distance/index influence.
- Duplicate or replay: One gateway attempt per batch/query UUID; preserve charged
  outcome after restart. Confirmed transient replacement attempts use the accepted
  two-retry schedule; uncertain outcomes require an explicit separately recorded retry.
- Stale data: Active profile switch excludes old vectors immediately. Canonical
  mutation/removal invalidates entries and in-flight publication. Source expiry
  qualifies separately retained claims rather than deleting their memory.
- Reconciliation divergence: Coverage names unsupported classes, pending/missing,
  truncated or incompatible representations. Rebuild never changes canonical truth.

## Integrations and Runtime Inputs

- Providers: Existing OpenAI embedding gateway, `text-embedding-3-large`/3,072;
  existing PostgreSQL 17.10 with pgvector 0.8.6, exact cosine and no ANN index.
- Environment: Existing ignored `.env`, owned Compose API/worker/services and
  disposable database/browser/native fixtures. Preserve all six normal Brains.
- Secrets: Existing environment key and canonical sanitizer; no vector/text in
  error/audit logs. Real-provider acceptance uses synthetic repository-owned data.
- Failure handling: Maximum 20 inputs/8,000 bytes per batch, 100 discovered inputs
  per Brain pass, 50,000 entries per active profile, existing queue/model limits.
  Exact semantic scan accepts at most 5,000 scoped entries; otherwise asks for a
  narrower selection. Four recalls/process, 90s semantic operation, 45s provider
  and 2s SQL bounds. Defaults for exact/lexical remain unchanged.

## Tests and Acceptance

- Automated: Controlled HTTP and real PostgreSQL for batch/cost/shape/policy,
  canonical transmission, exact cosine math, scoped/ineligible distractors,
  duplicates, correction/raw-copy/reindex, expiry/erasure/restart/older restore.
  Reuse positive canonical controls and test through actual consumers. Browser
  desktop/mobile and native recall call the normal endpoint.
- Manual: One bounded real synthetic embedding corpus in the normal runtime;
  compare exact/lexical, semantic and combined hit/distractor/no-support behavior
  with equal context limits and reported latency/tokens. Verify all existing Brain
  and model usage preservation, owned restart/readiness and inspected screenshots.
- Acceptance: Every semantic contract item has observed proof, with no ANN,
  semantic correctness, operational verification or external deployment overclaim.
  Run focused checks, Clippy, API/build, `./scripts/validate.sh` and reconcile docs.

## Closeout

- Planned: Full governed semantic projection, recall and consumer proof above.
- Shipped: Canonical representations, durable bounded generation/retry/reindex,
  full 3,072-dimensional exact semantic recall and RRF, canonical scope/privacy/
  retention authority, generated interfaces and browser/native controls. Migration
  017 and matching UI are running locally with actual synthetic provider proof.
- Not shipped: N/A for this slice. Graph fusion, host injection, learned relevance
  gates, ANN and generated answers remain the explicitly named later/deferred work.
- New blockers: None.
- Docs updated: Semantic contract, interface and
  [implementation mapping](../../../mappings/semantic-retrieval-2026-09-15.md),
  semantic/canonical recall runbooks, Atlas audit, exact synthetic corpus, README,
  owning epic and roadmap/contract/mapping/runbook/execution indexes.
- Validation: Context7 and official pgvector/OpenAI/PostgreSQL/Query interfaces
  verified. Final full platform pass: 61 tests in 141.11s, including 17 semantic
  fixtures. The strengthened partial-erasure retry control then passed in 1.36s;
  all-target Clippy passed again. Eight native/protocol library tests, 117 generated
  API operations, web type/build, browser recall in 4.4s, CodeGraph sync and 32
  governance checks passed. The Atlas reviewer confirmed its four targeted fixes.
  Owned migration/restart at 11:35:52 UTC passed readiness and normal-runtime
  preservation. The fixed seven-document corpus used 20 actual embedding requests
  and 1,036 recorded tokens, with scope/bytes/latency/cost and unsupported-return
  measurements. Desktop/mobile source/readiness/reindex and post-rebuild search
  were inspected. All six prior Brains and their model totals stayed unchanged;
  only the new synthetic Brain enabled embeddings, with zero customer-file reads.
- Version: N/A; no release policy.
- Commit: Uncommitted.
