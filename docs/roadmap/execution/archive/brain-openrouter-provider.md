# Select OpenRouter models per Brain

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: select and use OpenRouter text/embedding models per Brain through the shared gateway.
- Non-goals: local serving, arbitrary URLs/models, asynchronous Batch adapter.
- Delivery shape: backend, existing inline settings, isolated live-provider proof.

## Governing Sources

[Provider selection contract](../../../contracts/brain-model-catalogue-and-selection.md),
[provider policy](../../../contracts/memory-provider-policy-and-learning.md),
[semantic contract](../../../contracts/retrieval-semantic.md) and
[epic](../../epics/memory-lifecycle.md).

## Scope

- In scope: installed provider identities, scoped discovery, reviewed Muse/GLM/Qwen catalogue, fixed-endpoint adapter, usage/provenance, rebuild fences, selector and meaningful tests.
- Out of scope: changing existing Brain selections or customer data transmission.
- Blockers: None; explicit October 8 user authorization resolves the provider gap.

## Surface and Interface Changes

- Interfaces: provider selection on existing policy; provider query on catalogue; installed provider list; additive request provider/cost fields.
- Storage: additive provider/cost ledger migration, Qwen dimension bound; immutable semantic generations retained.
- Ownership: installation owns credentials/endpoints; Brain owns selected provider/model policy.

## Data and Authority

- Inputs: canonical product records; public or synthetic live proof inputs only.
- Authority: shared gateway and existing permission/replay/retention fences.
- Blind spots: catalogue listing is not a successful call; model quality measured separately.

## States and Edge Cases

- Loading: explicit admin catalogue refresh with provider-separated cache.
- Empty: missing credential prevents calls and gives a safe error.
- Error: incomplete/refused/invalid response rejected; charged attempt retained.
- Blocked: token/concurrency/policy constraints preserve bounded recovery.
- No-access: admin/browser/CSRF and Brain grant checks retained.
- Duplicate or replay: immutable policy receipt and paid-attempt identities retained.
- Stale data: new provider/model selection requires fresh discovery.
- Reconciliation divergence: provider changes invalidate old semantic profiles; in-flight results cannot publish.

## Integrations and Runtime Inputs

- Providers: fixed OpenAI and OpenRouter endpoints, no fallback to another model/provider.
- Environment: `OPENROUTER_API_KEY`, existing OpenAI names and defaults.
- Secrets: installation environment only; both keys excluded by publication guard.
- Failure handling: existing bounded timeout/body limits, no blind retry.

## Tests and Acceptance

- Automated: actual HTTP fixture adapter, provider-separated discovery, missing credentials, wrong aliases/dimensions/finish reasons, rebuild/replay and existing OpenAI regression; selector browser proof and repository validation.
- Manual: actual synthetic provider calls and product-path public benchmark; inspect selector screenshot.
- Acceptance: an admin can select OpenRouter and its compatible models, save/rebuild and run text/semantic operations; existing Brains/defaults remain unchanged; live usage recorded and below campaign cap.

## Closeout

- Planned: above.
- Shipped: local per-Brain OpenRouter selection, provider-separated discovery,
  strict text and Qwen embedding adapters, billed-cost receipts, durable replay
  and profile fences, and the existing inline settings selector.
- Not shipped: normal-stack deployment, asynchronous Batch/local serving, and
  the independent complete LongMemEval answer/judge campaign.
- New blockers: None.
- Docs updated: governing selection/protocol amendments, provider/semantic
  operator runbooks and lifecycle indexes.
- Validation: provider HTTP fixture (including actual worker vector publication),
  inline provider/Cancel/rebuild browser proof, frontend type checks and
  `./scripts/validate.sh` pass. Live Qwen HotpotQA: 99% supporting-document
  recall and 49/50 complete support sets for USD 0.00082907. Atlas matrix has
  no failures. Wider model suite: 84/96 pass; the exact same 12 failures
  reproduce on unchanged HEAD (83/95), and remain unresolved. Non-ignored
  workspace suite: 75 pass, 229 explicitly ignored. Final OpenRouter billing
  and OpenAI concurrency/failure/replay fixtures pass. Actual native support
  audits cover 50 GLM 4.7 and 26 GLM 5.3 cases: model safety/availability
  limitations are retained, rather than classified as adapter success.
  Legacy receipt deserialization and actual managed-memory API replay also
  pass without reapplying adoption or requiring the new provider fields.
  The separate LongMemEval pack is now archived with complete recovery/scoring; see
  [current evidence](../../../mappings/openrouter-benchmark-proof-2026-10-08.md).
- Version: N/A; no release requested.
- Commit: uncommitted.
