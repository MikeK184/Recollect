# Read-only Ask and retained evidence search

Status: shipped
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
- Shipped: Governed temporary answers and explicit evidence search are deployed. The remaining actual-provider question, exact citation, authorized correction and fresh recall journey passed on an owned disposable Brain, followed by confirmed deletion.
- Not shipped: Saved conversations, effectful Ask and universal answer-quality claims remain excluded.
- New blockers: None for this slice.
- Docs updated: This pack, owning epic, active/archive and epic indexes, handoff, relevant runbooks and [final acceptance evidence](../../../mappings/desktop-final-acceptance-2026-10-01.md).
- Validation: Five deterministic Ask browser cases, production handler/provider fixtures and graph-backed lifecycle proof pass. Six earlier actual-provider quality cases remain dated evidence; the October 1 deployed journey is a fresh integration proof. Disabled fallback and no model calls on navigation pass. Final repository checks are recorded in the linked evidence.
- Version: N/A: no release requested.
- Commit: Uncommitted.
