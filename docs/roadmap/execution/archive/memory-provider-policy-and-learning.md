# Brain provider policy and governed learning

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Use the selected real LLM/embedding provider under Brain policy and publish evidence-backed learning through canonical memory.
- Non-goals: Semantic retrieval indexes, model tools, customer-corpus bulk ingestion and unapproved fallback.
- Delivery shape: Rust gateway/model worker, policy/usage/learning API and UI, local provider proof; uncommitted.

## Governing Sources

[Provider/learning contract](../../../contracts/memory-provider-policy-and-learning.md),
[claims ADR](../../../adr/0005-canonical-claims-and-time.md),
[retention contract](../../../contracts/memory-retention-and-erasure.md) and
[owning epic](../../epics/memory-lifecycle.md).

## Scope

- In scope: Installed OpenAI adapter, per-Brain immutable policy, bounded gateway/usage, source learning, literal acceptance, review/erasure integration and browser controls.
- Out of scope: Later retrieval, graphs, host adapters and additional provider backends.
- Blockers: None; the selected Luna/embedding-large pair has passed preflight and detailed routine decisions are resolved.

## Surface and Interface Changes

- Interfaces: Policy/history/usage, fixed synthetic connection check and source learning/list/retry APIs; internal typed gateway for future consumers.
- Storage: Migration 011 adds policy revisions, model request/input metadata and source learning/derivation records with privacy fences.
- Ownership: Server model gateway and learning worker share existing command, access, memory, review and retention modules.

## Data and Authority

- Inputs: Exact retained source/canonical references, installed model choices, named policy, explicit scope and authenticated actor/device.
- Authority: Brain policy before transmission; current canonical access/review/retention before returning or publishing model output.
- Blind spots: Provider transmission cannot be recalled; a synthetic success does not establish extraction quality on all customer content.

## States and Edge Cases

- Loading: Policy, usage and queued/running learning are visible.
- Empty: Disabled policy, no attempts and zero extracted claims are explicit.
- Error: Safe provider/refusal/shape/timeout codes and explicit new attempts; no fallback.
- Blocked: Budget, concurrency, policy mismatch, unavailable inputs and stale updates deny work.
- No-access: Browser admin policy controls, writer learning and paired operation scope; foreign readers denied.
- Duplicate or replay: Commands are idempotent; outbound uncertainty cannot silently repeat a charged request.
- Stale data: Worker admission/publication checks current source, policy, permissions, scope and lease.
- Reconciliation divergence: Metadata records completed provider work separately from canonical publication and central erasure.

## Integrations and Runtime Inputs

- Providers: OpenAI HTTPS Responses and Embeddings via existing Reqwest; local controlled HTTP fixture only in test configuration.
- Environment: OPENAI_API_KEY and installed text/embedding model/dimension settings; preserve ignored credentials.
- Secrets: Existing deterministic exclusions before external calls; no body/key/vector logging or raw-response cache.
- Failure handling: Durable model lane, bounded admission/body/time, conservative usage reservations and explicit uncertain-outcome retry.

## Tests and Acceptance

- Automated: Policy/role/budget/concurrency and real HTTP failures; governed learning, rejection/conflict, restart/replay and in-flight erasure; real browser workflows.
- Manual: Fixed-synthetic actual OpenAI pair through the gateway and normal-runtime UI/preservation.
- Acceptance: All contract requirements for current consumers pass before archival; downstream consumers remain explicit.

## Closeout

- Planned: Brain model policy, gateway, usage, source learning, literal acceptance, retention/review and UI.
- Shipped: Migration 011, Brain model policy/history/usage, bounded selected OpenAI gateway, canonical source learning/literal acceptance, review/erasure/restore integration and browser controls. Normal runtime and persistent synthetic demo verified.
- Not shipped: Downstream semantic retrieval, procedures, graph and host consumers belong to their named slices; no external deployment.
- New blockers: None.
- Docs updated: Accepted contract, pack, owning epic, runbook, dated proof and indexes.
- Validation: Full 24-core run plus added device/revocation proof, final Clippy, real OpenAI browser and three browser regressions passed. Normal-runtime migration, selected pair, learned claim, mobile/desktop UI and SWEG preservation passed; governance/32 checker tests and diff checks passed. See [dated proof](../../../mappings/provider-learning-proof-2026-09-14.md).
- Version: N/A; no release requested.
- Commit: Uncommitted.
