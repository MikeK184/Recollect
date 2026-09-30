# Ask leads with the question composer

Status: planned
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Opening a Brain or `/ask` presents the question composer as the default view, with exact/text search retained as an explicit mode and as the working fallback whenever answering is disabled or unconfigured.
- Non-goals: No new answer behavior, endpoint, schema, persistence, streaming, tool execution or memory write. No change to retrieval eligibility, ranking, scope, budget, citation rules or model policy.
- Delivery shape: React presentation and route-state change in the Ask feature plus explicit empty-state copy; focused browser proof; reconciled epic and execution indexes.

## Governing Sources

- [ADR 0017: Primary Ask](../../../adr/0017-desktop-knowledge-and-ask-experience.md)
- [Tiered desktop surface contract](../../../contracts/desktop-knowledge-surface.md)
- [Evidence-backed temporary answers](../../../contracts/retrieval-answers.md)
- [Contextual desktop experience](../../../contracts/desktop-experience.md)
- [Canonical exact and lexical recall](../../../contracts/retrieval-exact-and-lexical.md)
- [Brain model policy and evidence-backed learning](../../../contracts/memory-provider-policy-and-learning.md)
- [Vision: correction and learning loop](../../../foundation/vision.md#correction-and-learning-loop)
- [Owning epic](../../epics/hybrid-retrieval.md)

## Scope

- In scope: Default view selection for `/ask` and `/brains/{brain}`; validated `?tab=` state that keeps Search reachable by direct link; the disabled-and-unconfigured explanation that leads into search; applied-constraint visibility above results in both modes; empty, no-evidence, partial-support and conflict presentations carried over unchanged from the successor cleanup slice.
- Out of scope: Any change to the answer request lifecycle, evidence bundle, citation enforcement, conversation temporariness, model permission or provider policy. Saved conversations and action-capable Ask stay deferred by [ADR 0014](../../../adr/0014-desktop-experience-and-answers.md). Removing or narrowing the search mode.
- Blockers: None. This slice explicitly amends the landing-tab decision recorded in the shipped [successor cleanup pack](../archive/mcp-successor-cleanup.md) under the user's 2026-09-29 instruction and [ADR 0017](../../../adr/0017-desktop-knowledge-and-ask-experience.md); the earlier pack keeps its historical closeout and is not rewritten. Sequencing: the accepted desktop plan keeps the final default-Ask cutover behind actual answer acceptance, so this slice follows `retrieval-ask-experience` rather than preceding it.

## Surface and Interface Changes

- Interfaces: No API change. Route default changes from the search tab to the question tab for `/ask` and for `/brains/{brain}` resolution.
- Storage: N/A: no schema or migration. Conversation state remains temporary browser memory exactly as the answers contract requires.
- Ownership: Hybrid retrieval owns the Ask consumer, question lifecycle and citation presentation. Memory lifecycle owns the answering model purpose and policy check. Platform owns the route shell that resolves the Brain landing path. This slice changes only which view the feature renders first.

## Data and Authority

- Inputs: Current Brain identity, caller role, installed-provider status, answering permission from the existing automation and model-policy reads, question text, applied scope and time criteria, recall and answer responses.
- Authority: The server decides answer eligibility from canonical policy; the browser supplies only question and selection. Applied permissions are read, never inferred from which view is showing.
- Blind spots: Whether answering is currently usable on a specific Brain is known only from the policy and installed-provider reads, not from a rendered composer. Actual end-to-end answer quality remains the responsibility of the ask-experience slice and its evaluation, and is not proven by a default-view change.

## States and Edge Cases

- Loading: Composer is immediately interactive; submitting shows the existing bounded stages rather than unvalidated token streaming.
- Empty: A Brain with no evidence shows the explicit no-evidence next steps already specified, in the question mode as well as search.
- Error: A failed answer keeps the question recoverable, shows the actual failure cause, and offers search of the same question without implying the answer succeeded.
- Blocked: Answering disabled or provider unconfigured explains the missing configuration and lands the person in working exact/text search.
- No-access: A reader without answering permission sees search and the reason; no partial composer state suggests an available model call.
- Duplicate or replay: The existing stable request ID admits at most one answering attempt; a repeated submit does not create a second provider request, unchanged by this slice.
- Stale data: Brain switching cancels the in-flight request and clears the conversation, evidence and inspector state; late responses cannot populate the new Brain.
- Reconciliation divergence: Evidence withdrawn or expired after retrieval is reflected in the result qualification rather than shown as settled support.

## Integrations and Runtime Inputs

- Providers: OpenAI through the existing shared gateway only when the Brain's policy permits answering. No new provider, model or purpose is introduced.
- Environment: No new variable. Existing installed-provider configuration applies.
- Secrets: No key value is rendered, logged or placed in URLs. Question text and answers never enter URLs.
- Failure handling: Existing gateway timeout, quota, concurrency and refusal behavior is unchanged. A refused or failed answer never silently falls back to a different provider and never converts into a search result without saying so.

## Tests and Acceptance

- Automated: Browser cases asserting `/ask` and `/brains/{brain}` open on the question composer, that `?tab=search` still resolves to search, that search works with answering disabled, that no navigation or tab switch issues a provider request, and that the disabled path explains configuration. Existing answer, citation, expiry and Brain-switch cases keep passing from the new default. Web typecheck, `./scripts/validate.sh` and `git diff --check`.
- Manual: One authorized real-provider question on a disposable Brain to confirm the default path produces a cited answer rather than only a correct-looking composer. Recorded as connectivity proof separately from the view change.
- Acceptance: Opening a Brain shows the question composer by default; exact/text search remains available by explicit mode and direct link and functions with answering disabled; applied constraints are visible in both modes; no view change triggers a model call, tool execution or memory write; the disabled path explains what an administrator can configure instead of failing silently.

## Closeout

- Planned: Default-view change, validated tab state, disabled and unconfigured explanation, applied-constraint visibility and retained empty states.
- Shipped: Not yet implemented. This pack is the specification; delivery evidence is recorded here only after the checks above run.
- Not shipped: Saved conversations, action-capable Ask and answer-quality evaluation remain deferred by their governing sources.
- New blockers: None recorded at authoring time.
- Docs updated: Owning epic slice map and dependencies, active execution index, desktop contract Ask row, and the successor-cleanup supersession note in ADR 0017.
- Validation: Not yet executed for this slice.
- Version: N/A: no release policy exists.
- Commit: Uncommitted.