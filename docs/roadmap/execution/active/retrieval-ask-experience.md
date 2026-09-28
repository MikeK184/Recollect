# Read-only Ask and retained evidence search

Status: in-progress
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Ask a bounded question, inspect its qualified exact citations and use working evidence search when answering is unavailable.
- Non-goals: Saved chat, implicit tools/memory writes, unvalidated token streaming, model/provider substitution or a question-rewriting model.
- Delivery shape: Rust protocol/server/gateway metadata migration plus route-owned browser Ask/search, focused real-handler proof and local runtime acceptance.

## Governing Sources

- [ADR 0014](../../../adr/0014-desktop-experience-and-answers.md)
- [Answer contract](../../../contracts/retrieval-answers.md)
- [Desktop contract](../../../contracts/desktop-experience.md)
- [Model policy](../../../contracts/memory-provider-policy-and-learning.md)
- [Investigation](../../../contracts/retrieval-investigation-ui.md)
- [Owning epic](../../epics/hybrid-retrieval.md)

## Scope

- In scope: Typed POST/status/cancel lifecycle, durable metadata idempotency, answering purpose default-off, server-owned exact-fragment bundle, pre/post canonical gates, structured citation validation, temporary four-turn browser display and search fallback.
- Out of scope: Persisted prompt/output, assistant prose as evidence, new agent permissions, automatic provider retry/fallback, saved conversations and action dispatch.
- Blockers: None for backend; UI integrates the shared shell/inspectors from predecessor slices. Their acceptance is required before final default-Ask cutover.

## Surface and Interface Changes

- Interfaces: POST /api/brains/{brain}/answer-requests with UUID request_id, question up to 512 UTF-8 bytes and RecallRequest; own metadata GET and cancel POST. Exact response fields and lexical-normalization rules are fixed in the answer contract. Existing recall/search behavior remains compatible.
- Storage: Ordered migration adds Brain/account-scoped request metadata and minimal indefinite replay tombstones; never question, answer, retrieved body or custom content hash. Existing model usage accounting and retention stay authoritative. Existing policy documents without answering remain disabled.
- Ownership: Protocol owns wire types; retrieval owns exact bundle/validity/citation keys; gateway owns provider/admission/accounting; browser owns temporary display and route state.

## Data and Authority

- Inputs: Browser question/explicit selection; canonical final RecallItems and exact provenance; current session/Brain/policy/epoch/retention; untrusted provider structured response.
- Authority: Canonical frozen-time candidate/item checks and current grants govern transmission and publication. Client evidence/actor/model labels confer no authority.
- Blind spots: Valid citations do not prove entailment, independent corroboration or deployment; missing bytes and partial coverage stay explicit.

## States and Edge Cases

- Loading: Bounded admitted/retrieving/answering states without token streaming; superseded requests cancel display.
- Empty: No eligible evidence yields insufficient support without provider transmission; derived empty lexical query requests clarification.
- Error: Safe distinct refusal/malformed/citation/provider/timeout/budget states; no raw provider body or silent paid retry.
- Blocked: Disabled purpose/transmission/content or missing credentials leads to exact/text search and a role-appropriate settings link.
- No-access: Reject foreign Brain/request/account and archived new work; observed access loss clears browser payload.
- Duplicate or replay: Request ID admits at most one answering attempt and, only when selected/permitted, one separately keyed semantic embedding attempt, including after payload loss/restart. Explicit retry uses a new ID; detached work completes accounting after disconnect.
- Stale data: Current epoch/policy/expiry/access gates suppress changed work; status GET rechecks them; browser clears invalid evidence and conversation.
- Reconciliation divergence: Cancel before dispatch causes zero calls; in-flight cancel suppresses output but preserves actual/uncertain cost. Frozen bundle is checked without reranking or re-embedding.

## Integrations and Runtime Inputs

- Providers: Existing installed provider/model via one gateway, Responses store:false and no tools; local HTTP provider fixture for deterministic proof.
- Environment: Existing OPENAI_API_KEY and configured text model inputs only; no arbitrary provider URL from the UI.
- Secrets: Environment-backed credentials; no prompt/response/provider body in logs or request metadata.
- Failure handling: Existing 45-second model timeout, bounded response/token/concurrency, no automatic retry/fallback and metadata-only uncertain disposition.

## Tests and Acceptance

- Automated: Production handlers with positive support; scope/time/rejection/erasure/expiry/whole-source exclusion; concurrent policy/grant/epoch change; citation and injection/no-side-effect tests; duplicate/cancel/restart/cost proof; browser expiry/switch/fallback. Rust checks, frontend generation/type/build and ./scripts/validate.sh.
- Manual: Ask with an explicitly permitted test Brain/provider, inspect exact citations/qualifications, exercise disabled fallback and temporary repeated questions; preserve existing user policies.
- Acceptance: Useful supported answers and honest abstention pass the production path; no output survives invalidation or causes tools/writes; metadata contains no transcripts. Record fixture, actual provider and deployed browser evidence separately.

## Closeout

- Planned: Governed read-only Ask, metadata idempotency/cancel and migrated evidence search.
- Shipped: Not yet. Typed read-only lifecycle, canonical bundle, guarded gateway, metadata migration and temporary Ask/search are implemented in the final local image. Five deterministic database/provider scenarios, graph-backed lifecycle proof, five Ask UI cases and six distinct installed-provider quality cases passed across the recorded runs.
- Not shipped: The declared source/memory integration acceptance and complete question-to-evidence-to-correction-to-recall journey remain open with those owner packs. The dated mapping preserves the initial paid quality-rubric failure and targeted rerun; positive browser answer transport fixtures are distinguished from actual provider proof. Explicit product non-goals remain excluded.
- New blockers: No unresolved product decision is known. Implementation refinements and verification are tracked in the [dated mapping](../../../mappings/desktop-experience-implementation-2026-09-26.md).
- Docs updated: [Desktop guide](../../../runbooks/desktop-experience.md), [implementation/assets/dependency evidence](../../../mappings/desktop-experience-implementation-2026-09-26.md), [current handoff](../../../../CONTINUE_HERE.md), affected domain runbooks, owning epic and indexes.
- Validation: Frontend typecheck/design and final image build passed; workspace clippy and 21 unit tests passed with 3 live-Vault cases explicitly ignored; governance lint and 32 tests passed. Desktop seven cases passed across a six-pass run and the repaired font-fallback targeted rerun; real Team/OIDC two, Ask five and MCP setup/runtime six passed. The dated mapping separates each owner result, initial failures, fixture/provider boundaries and remaining checks.
- Version: N/A: no release requested.
- Commit: uncommitted.
