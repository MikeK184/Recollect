# Brain model catalogue and inline AI selection

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: AI permissions edit inline; admins select account-available compatible
  text/embedding models with explanatory dated prices and safe explicit rebuilds.
- Non-goals: Arbitrary adapters/models, billing calculations, live paid probes.
- Delivery shape: Local Rust/API/browser implementation, migration and focused proof.

## Governing Sources

[Model catalogue contract](../../../contracts/brain-model-catalogue-and-selection.md),
[provider policy](../../../contracts/memory-provider-policy-and-learning.md),
[semantic contract](../../../contracts/retrieval-semantic.md),
[managed defaults](../../../contracts/memory-managed-experience.md),
[desktop contract](../../../contracts/desktop-experience.md), and
[owning epic](../../epics/memory-lifecycle.md).

## Scope

- In scope: Account catalogue, supported selection, inline purpose/content controls,
  model-specific gateway request shaping, dimension-safe generations/rebuilds,
  removal of recommended-default controls while preserving creation defaults.
- Out of scope: Optional provider integrations, ANN, invoices and unverified IDs.
- Blockers: None; user's 2026-10-05 authorization resolves proposed point 10.

## Surface and Interface Changes

- Interfaces: Brain read `GET /api/brains/{brain}/models/catalogue`; explicit
  admin POST on that path uses account model-list request with existing CSRF. Model policy PUT adds
  optional default-false `rebuild_embeddings` and retains revision/idempotency.
- Storage: Migration 034 changes embedding to unconstrained full-precision vector,
  bounds profiles and guards each vector against its profile's dimensions.
- Ownership: Model policy/catalogue and gateway own provider selection; semantic
  queue/work/query own active projection generation. Browser never sees keys.

## Data and Authority

- Inputs: Account model IDs joined to dated official capability/price metadata;
  immutable Brain policy, canonical eligible inputs and semantic profile IDs.
- Authority: Admin plus current expected policy revision; existing standing
  permissions, budgets, exact identities and erasure fences govern work.
- Blind spots: Listing does not prove execution; metadata may become stale. Account-list availability is not a successful paid model call; final CUA
  verification performs metadata refresh only.

## States and Edge Cases

- Loading: Current selection remains visible while explicit refresh runs.
- Empty: Show no verified available options; retain saved selection.
- Error: Safe rate-limit/timeout/body/shape states retain stale metadata.
- Blocked: Rebuild generation can exist with disabled standing permissions,
  missing credentials or blocked classes; no provider request bypasses policy.
- No-access: Readers inspect only; archived Brains cannot change policy.
- Duplicate or replay: One idempotent policy receipt produces one generation.
- Stale data: Revision mismatch preserves local edits with reload guidance;
  catalogue older than five minutes cannot admit a new model selection.
- Reconciliation divergence: Wrong model/dimensions and superseded profiles
  cannot publish or rank; existing exact/lexical channels stay accessible.

## Integrations and Runtime Inputs

- Providers: Existing OpenAI HTTPS adapter; isolated HTTP fixture for proof.
- Environment: Existing `OPENAI_API_KEY`, model-name/dimension/endpoint inputs.
- Secrets: Environment only; no provider bodies or credentials in errors/history/UI.
- Failure handling: Five-second list timeout, one-MiB bound, redirects denied,
  serialized five-minute cache and one-minute failure backoff. No implicit paid retry.

## Tests and Acceptance

- Automated: Catalogue filtering/cache/errors and role checks; model selection,
  dimension/profile DB guard, atomic rebuild/idempotency, old-output suppression,
  policy/erasure and lexical controls in isolated PostgreSQL/provider fixtures;
  inline Save/Cancel/purpose dependency and selector browser proof; focused Rust
  and frontend checks plus `./scripts/validate.sh`.
- Manual: Parent coordinates current-stack screenshots and second-agent review.
- Acceptance: All point-10 controls work in-place; prices/provenance honest; changed
  embeddings require Save and rebuild and never mix vectors or broaden permissions.

## Closeout

- Planned: Complete point 10 under the accepted catalogue/semantic amendment.
- Shipped: Local, uncommitted implementation now includes inline policy controls,
  account catalogue and dated price provenance, supported text/embedding selection,
  model-specific request shaping, migration 034 and atomic dimension-safe rebuilds.
  Creation defaults remain automatic; recommended-default controls are removed.
- Not shipped: No known deferrals inside accepted scope.
- New blockers: None.
- Docs updated: Model catalogue, provider policy, semantic contract, interface
  evidence mapping and this pack. The coordinated closeout reconciles epic/index lifecycle state.
- Validation: Catalogue unit tests (2) and isolated PostgreSQL/provider catalogue
  cases (3) pass; cases cover authorization/CSRF, bounded safe errors, cache
  expiry, availability, pricing, selection, immutable profiles, dimension guards,
  idempotent rebuild, stale revisions, in-flight suppression, erasure and lexical
  access. The final catalogue rerun also proves that an unreviewed installation
  ID cannot bypass the supported-model admission rule. The existing model suite
  passed 53 of 54 cases on its initial run;
  the remaining large-scope semantic admission case exposed a two-second query
  timeout and passes after an identity-only preflight repair without relaxing
  the deadline. All 19 cases pass in the semantic follow-up. Clippy for
  the server library, frontend typecheck, governance lint/32 tests and diff
  whitespace checks pass. Browser regression tests are authored for normal CI;
  they have not been executed in this task. Final local stack/UI and independent actual-pixel review pass. Synthetic inline
  checkbox Save persists after reload; installation model and policy drafts
  were cancelled. The account model list returned five supported text IDs and
  two embedding IDs.
- Evidence limit: The final CUA catalogue refresh used installation-account
  metadata authentication only. No paid model inference or customer-corpus
  provider request was performed. Model execution/rebuild proof uses isolated
  synthetic credentials and content.
- Focused commands: With repository `.env` inputs loaded and `OPENAI_API_KEY`
  unset, `cargo test -p recollect-server --test platform models::catalogue:: --
  --ignored --test-threads=1` passes 3 cases; `models::semantic::` with the same
  flags passes 19 cases. `cargo test -p recollect-server --lib
  model_policy::catalogue::tests` passes 2 units.
- Evidence: [Coordinated delivery and limits](../../../mappings/desktop-browser-management-2026-10-05.md).
- Version: N/A; no release requested.
- Commit: uncommitted.
