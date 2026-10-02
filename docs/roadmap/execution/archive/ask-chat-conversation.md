# Ask as a chat-style conversation thread

Status: shipped
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Present the existing model-grounded answer flow as a chat-style conversation — user questions and Brain answers as turns in a thread, citations as expandable evidence attachments, follow-ups inline — so Ask reads like talking to the Brain rather than filling a form.
- Non-goals: No backend change to retrieval, answering policy, budgets, citation validation or the temporary (non-persisted) conversation lifecycle; no streaming protocol change; no new destinations or routes.
- Delivery shape: A React rework of the Ask page's conversation view over the unchanged `answer_requests` API, plus one contract amendment and live browser verification.

## Governing Sources

- [Contextual desktop experience](../../../contracts/desktop-experience.md) (amended: chat-style Ask default view)
- [Retrieval answers](../../../contracts/retrieval-answers.md)
- [Owning epic](../../epics/hybrid-retrieval.md)

## Scope

- In scope: Rework `AskConversation` into a conversation thread: an empty-state prompt, user question turns and assistant answer turns (answer text with citation chips that expand the exact evidence in the existing inspector), inline follow-up questions appended to the same thread, the existing filter drawer and "New conversation" control retained, applied restrictions visible, and the disabled/unavailable model path degrading to the search mode exactly as today. The Search evidence tab is unchanged.
- Out of scope: Any change to `RecallRequest`/`AnswerResponse`, answer lifecycle states, polling semantics, model policy evaluation, budgets, or citation validation; persisting conversations; multi-user shared threads.
- Blockers: None. The 2026-10-02 user direction (Ask should feel like a chat that uses the configured AI over retrieved evidence) is the governing decision record; contract amendments are part of this change.

## Surface and Interface Changes

- Interfaces: None. The page consumes the existing `POST /api/brains/{brain}/answer-requests`, status polling, cancellation and search endpoints unchanged.
- Storage: None.
- Ownership: Hybrid retrieval owns the Ask presentation; the answer backend and its contract are untouched.

## Data and Authority

- Inputs: Existing answer-request responses (state, answer text, citations, failure codes) and the Brain model policy read that gates the composer.
- Authority: Unchanged — answering requires the Brain's model policy to permit it with credentials present; otherwise the composer degrades to search-only exactly as before.
- Blind spots: The thread is temporary (component memory); a reload clears it, which the UI states. Long answers keep the existing truncation/inspector behavior.

## States and Edge Cases

- Loading: A pending turn shows an in-thread "answering" state; earlier turns stay readable.
- Empty: No turns → the conversation prompt with the existing suggested questions.
- Error: Per-turn failure codes render inside that turn (insufficient support, policy denied, credentials missing, budget, timeout, stale); the thread remains usable for a new question.
- Blocked: A disabled model blocks only answering, not the page; search evidence stays available.
- No-access: Standard Brain 403/404; no answer data from foreign Brains can render.
- Duplicate or replay: Each submit creates one answer request; in-flight submits are suppressed for that turn.
- Stale data: Polling and the existing stale/interrupted handling apply per turn; a Brain switch clears the thread.
- Reconciliation divergence: None — presentation only; no second source of truth is introduced.

## Integrations and Runtime Inputs

- Providers: No new provider or model call path.
- Environment: No new environment variables.
- Secrets: None.
- Failure handling: Existing failure-code mapping retained, rendered per turn.

## Tests and Acceptance

- Automated: Web typecheck and design checks; no backend tests required (no API change).
- Manual: Owner login at the live stack on a Brain with answering enabled — ask a question, see the user/assistant thread, expand a citation to its evidence, follow up in the same thread, clear with "New conversation"; on a Brain without answering the composer degrades to search.
- Acceptance: Browser verification passes on real data; typecheck and design checks clean; `./scripts/validate.sh` passes; live stack rebuilt and healthy.

## Closeout

- Planned: Chat-thread Ask view over the unchanged answer API, contract amendment, browser proof.
- Shipped: The Ask landing now reads like a chat: the prompt is centered in the conversation area with the composer beneath it (single-line start, growing with input) instead of above it; the existing thread — user question bubbles, model-grounded answer turns with citation markers and the evidence inspector, inline follow-ups, per-turn failure states, "New conversation" — is unchanged. Live proof 2026-10-03: on the SWEG Brain a real question ("What do we know about the SWEG project?") produced a completed model-grounded answer turn with citation markers in the thread; no console errors. See [evidence](../../../mappings/agents-ask-chrome-refinements-2026-10-03.md).
- Not shipped: Nothing from this scope. Streaming and persisted conversations remain out of scope by design (temporary threads, unchanged backend).
- New blockers: None.
- Docs updated: Contract `desktop-experience.md` (chat-style Ask default view); epic slice and indexes reconciled; evidence mapping added.
- Validation: Web typecheck and design checks clean; no backend tests required (no API change); `./scripts/validate.sh` passes; live stack rebuilt via `./scripts/stack.sh up --build` with api/worker healthy.
- Version: N/A: no release policy or version bump in this scope.
- Commit: Uncommitted; no commit, push or release performed.
