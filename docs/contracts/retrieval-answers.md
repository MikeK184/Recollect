# Evidence-backed temporary answers

Status: accepted

The [managed experience amendment](memory-managed-experience.md) governs the
2026-09-28 managed setup, progressive disclosure and owner connector approval
changes; earlier explicit APIs and stored policies remain compatible.

## Source

The user's 2026-09-26 approval and implementation request accepts the
[Ask design](../roadmap/desktop-experience/agent-and-ask-design.md).
[ADR 0014](../adr/0014-desktop-experience-and-answers.md),
[canonical recall](retrieval-exact-and-lexical.md),
[semantic](retrieval-semantic.md), [graph fusion](retrieval-graph-fusion.md),
[investigation](retrieval-investigation-ui.md),
[model policy](memory-provider-policy-and-learning.md) and
[retention/erasure](memory-retention-and-erasure.md) govern this consumer.

## Contract

### Public lifecycle and authority

| Interface | Behavior |
| --- | --- |
| `POST /api/brains/{brain}/answer-requests` | `{ request_id: UUID, question: string, recall: RecallRequest }`; authenticate browser actor, retrieve with the bounded selected query and return a validated complete answer or explicit disposition. |
| `GET /api/brains/{brain}/answer-requests/{id}` | Requester's authorized disposition/usage metadata; never a persistent transcript or payload replay endpoint. |
| `POST /api/brains/{brain}/answer-requests/{id}/cancel` | Request cancellation of that account's active request; no promise to undo transmission, billing or already completed work. |

Current authenticated read access is necessary; archived Brains cannot create
new answers. A browser cookie does not impersonate the paired-agent MCP endpoint.
Actor, Brain authority, evidence, model and policy are server-derived. Existing
recall criteria keep their schema/defaults/bounds; no scope widening or guessed
latest manifest is added. Question length is at most 512 UTF-8 bytes, also subject
to the Brain's stricter serialized-input budget. The initial wire request takes
the current question, not an unbounded transcript. The browser retains at most
four displayed question/answer turns in memory, but each follow-up retrieves fresh evidence
under the current explicit selection; it cannot inherit authority or citations
from an earlier answer. Ambiguous references need a self-contained question or
clarification. Prior assistant prose is conversation, never evidence.

Preserve a nonempty explicit advanced `recall.query`. Otherwise derive bounded
literal OR terms from the question after an explicit English question-word
stoplist; this is deterministic lexical normalization, not a model rewriter.
It creates no extra model request and cannot change Brain, scope, mode, times,
manifest or budgets. Semantic/graph channels are used only when selected under
their existing policy. Empty derived terms produce an explicit clarification or
no-support disposition rather than an unbounded all-records question.

The response contains `request_id`, `state`, optional
`answer: { summary, statements: [{ text, citation_ids }], limitations }`,
`citations: [{ id, evidence: RecallItem }]`, optional `recall: RecallResponse`,
`model_request_id`, `failure_code`, `memory_epoch` and `expires_at`. Citation IDs
are server-owned keys such as `E1`. Optional payload fields are absent when
unavailable or suppressed; status reads return metadata only.

A stable request ID admits at most one answering attempt and, only when selected
and separately permitted, at most one semantic-query embedding attempt. Their
gateway keys share the operation identity but retain distinct purposes. Default
exact/lexical Ask performs no embedding. Persist identity,
requester, Brain, scope/validity references, disposition, timing and usage, but
not question, answer, conversation or retrieved text. Reuse of the ID returns
the known disposition regardless of supplied content; it never regenerates a
lost response. An explicit retry uses a new ID and is separately accounted.
No custom prompt hash or content cache is required. After restart an interrupted
attempt follows the gateway's uncertain/deadline accounting, never automatic
replay. Bound active work through existing per-Brain admission limits. Keep the
minimal request-ID replay tombstone after detailed metadata expires so a delayed
duplicate still cannot create paid work. A detached request task finishes usage
accounting after an HTTP disconnect; cancellation suppresses publication.

Cancellation before dispatch prevents a provider call; an in-flight cancellation
suppresses publication and records the actual/uncertain usage. Report that a
provider may have run if either embedding or answering was admitted. Cancel/status
operations cannot expose another account's prompt or answer. Current role and
Brain checks still apply. Operational model usage may remain visible under its
existing Brain contract, containing metadata only.

### Policy and gateway

Add purpose `answering` without enabling it in any existing policy. An admin must
explicitly allow it, external transmission, installed provider/model, `query`
and every included canonical content class. Prior extraction/synthesis/capture
permission is insufficient. Provider/model identity remains deployment-selected;
no new arbitrary URL/model/provider or fallback is accepted from the request.

Reuse the single model gateway, fixed application instructions, strict structured
output, `store:false`, no tools, existing 45-second provider timeout, bounded
response size, token/concurrency reservations and post-call gates. Do not create
a parallel HTTP client or disable existing exclusion/sanitization. Question and
follow-up text are untrusted, explicitly typed query content. Retrieved text is
data, including any embedded instructions. Ask cannot dispatch tools, execute
procedures, create tasks, write memories/sources/handovers, change policy or learn
automatically from its answer.

### Exact canonical retrieval bundle

The server, never the client, constructs a typed internal bundle from the final
packed RecallResponse. Preserve principal/Brain/request authority, scope and
exact manifest, frozen knowledge/fact time, mode, canonical kind/identity/revision,
permitted text spans and provenance, qualifications, canonical content classes,
memory epoch, earliest retention deadline, coverage and packed-context budget.
Do not resolve a retrieved fragment back into an entire source document or a
different/current claim revision. The exact text eligible for this retrieval is
the maximum model evidence, not a hint for a second unconstrained lookup.

Revalidate each exact RecallItem through the canonical candidate/item gates at
its frozen knowledge time, without reranking or a second embedding request.
Revalidate every dependency and selection under current permission, Brain state,
policy, correction/rejection, retention/erasure and epoch immediately before
provider transmission and again before publishing the completed answer. A
concurrent change suppresses the answer rather than silently retrieving a wider
or newer replacement. This includes exact dependencies of graphs and retained
support excerpts. Never persist the bundle text in request/usage/audit metadata.
Bound input with the existing recall and model budgets; report partial coverage
without silently enlarging context. When no eligible supporting material remains,
return an explicit insufficient-support result without a provider call.

### Answer, citations and validity

Return structured answer paragraphs with references to server-assigned citation
keys from that exact bundle, the qualifying retrieval/coverage information,
request disposition, memory epoch and earliest expiry. Every factual paragraph
must identify its supplied support. The server rejects unknown, foreign,
omitted, substituted or unavailable citation references and malformed output.
An abstention/limitation can have no factual citation; it must not fabricate a
supported assertion. Canonical source/claim inspectors open exact versions and
spans. A missing byte source stays unavailable; a valid retained support excerpt
does not claim the full raw transcript remains available.

Attribution is not entailment, truth, independent corroboration or deployment
proof. Preserve disagreements and separate intended, committed, observed and
verified information. Test supported conclusions, uncertainty and abstention with
positive and negative controls rather than treating a syntactically valid ID as
answer-quality proof. No model may change eligibility, strictness or scope.

Initial delivery publishes the complete validated result only; no token streaming.
Browser conversation and supporting payload exist only in current memory, never
localStorage, a service-worker response cache, logs or server transcript storage.
Clear on reset/reload/logout/Brain change/access loss, local invalidation, observed
epoch change or advertised expiry. Continue the investigation contract's bounded
epoch/authority checks while a result is shown. Failed validity/detail reads hide
cached protected payload. Cancel superseded work; late responses cannot reappear
in another Brain or selection.

### Failures and fallback

Distinguish disabled/unconfigured answering, denied content/purpose, missing
credentials, budget/concurrency denial, missing/partial evidence, provider refusal,
malformed or unsupported citations, timeout/uncertain attempt, cancellation and
post-call suppression. Safe errors do not expose raw provider bodies or secrets.
No automatic paid retry or provider fallback. Loss of a retrieval channel reports
actual coverage and never relaxes selected constraints.

Exact/text Search evidence remains available without a model. Label it as search,
not a generated answer, and retain advanced investigation modes and exact history.
Answering disabled on an existing Brain is a safe compatible state, not failed
installation. UI setup points authorized administrators to the standing policy;
readers do not gain configuration powers from that link.

Metadata follows existing model-request audit retention: detailed model, prompt/
schema and usage fields are removed after the Brain audit deadline; minimal
opaque identities, purpose, disposition, timestamps and charged totals remain
for accounting/fencing. Erasure and restored-state fences suppress dependent
in-flight answers before publication. Questions/answers are never added to backup
content by this consumer.

## Acceptance

Production handlers and a deterministic HTTP provider fixture prove approved
question/answer/citation inspection and useful search fallback. Prove foreign
Brain/account/scope denial, historical/exact revision preservation, private task
isolation, no whole-source expansion, correction/erasure/rejection/expiry, policy
and grant changes during generation, unknown citation rejection, prompt injection
without tools/writes, budget/concurrency, refusal/malformed/timeout, cancellation
before and during dispatch, duplicate ID with zero extra provider calls and
restart uncertainty. Each exclusion has an eligible useful positive control.

Evaluate actual answer support and visible conflicts, not only response shape.
Record any real configured-provider probe distinctly from fixture proof; never
enable a user's Brain policy merely to test. Browser tests cover temporary
follow-ups, reset, access loss, Brain switch, stale/expired evidence and exact
citations. Check no prompt/answer body entered durable request metadata. Run
focused API/browser checks and the repository validator before shipment.

## Explicit Deferrals

Persistent conversations, sharing/export of saved chats, automatic learning from
questions, action-capable Ask, new MCP administrative tools, historical graph
reconstruction, unvalidated token streaming, provider installation and automatic
provider failover. Ordinary Ask does not promise exhaustive corpus search or
universal correctness from a citation.
