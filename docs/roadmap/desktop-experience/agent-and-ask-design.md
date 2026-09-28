# Agent interfaces and Ask Brain design

Status: accepted
Observed: 2026-09-26
Accepted: 2026-09-26, by explicit user approval and implementation request.
Scope: reviewed existing interfaces and selected answer design; implementation is in progress.

The [answer contract](../../contracts/retrieval-answers.md) and
[ADR 0014](../../adr/0014-desktop-experience-and-answers.md) specify final wire,
bound, authority and persistence decisions. Existing-interface observations below
describe the review baseline, not a claim that new endpoints are already running.

## Existing autonomy and authority

[ADR 0006](../../adr/0006-autonomous-memory.md) requires normal learning,
reconciliation, revision and retirement without per-record human approval.
The [worker](../../../crates/server/src/worker.rs) (220) and
[autonomous scheduler](../../../crates/server/src/autonomous.rs) (225) implement
bounded automatic work under standing Brain permission. The browser should
configure that policy and expose outcomes/optional intervention.

Two MCP directions must remain distinct:

- Incoming memory/workspace MCP: an external coding agent uses a paired device,
  fixed Brain and explicit operation scope; current account/device/Brain
  permissions are checked. This belongs in Agents.
- Outgoing managed MCP: Recollect routes calls to an approved connection/runner
  under independent profile Use permission. This belongs in Connections.

Sources: [agent contract](../../contracts/mcp-memory-and-workspace-tools.md),
[transport ADR](../../adr/0010-agent-memory-mcp.md),
[service](../../../crates/server/src/mcp/agent.rs) (39),
[profiles](../../contracts/mcp-catalogue-and-profiles.md),
[runtime](../../contracts/mcp-runtime-and-credentials.md).
Browser cookies do not authorize the paired-device agent endpoint.

These are accepted/source-inspected capabilities, not a fresh connected-host
or provider proof in this review.

## Current MCP tool inventory

[Remote catalogue](../../../crates/server/src/mcp/agent/catalogue.rs) (94)
implements 22 tools; the [native bridge](../../../crates/agent/src/mcp_bridge.rs)
(41) adds workspace.refresh. Listing is role-aware and each call rechecks rights.

| Tool | Purpose | Boundary |
| --- | --- | --- |
| workspace.list | Catalogue metadata and own contexts | No extraction or tool start |
| workspace.start_task | Create own task/child with fresh context | Fixed Brain; independent scope |
| workspace.set_scope | Change future task scope and recall | Existing operations unchanged |
| workspace.inspect_task | Scope/operation history | Account-private |
| workspace.begin | Freeze an operation binding | Does not itself execute operation |
| workspace.close | Stop new operations | Children/in-flight work retain scope |
| memory.recall | Exact/lexical/semantic/graph context | Current scope, evidence, retention and budgets |
| memory.inspect | Exact claim and retained history | Historical != currently eligible |
| memory.review_history | Decisions/rejections/conflicts | Cannot impersonate a human review |
| memory.contribute | Supported claim/decision/procedure/revision | Writer/admin; canonical mutation and provenance |
| memory.handover | Generate from exact contributions | Standing model policy and combined scope |
| memory.handover_status | Generation disposition | Does not broaden applicability |
| memory.graph_explore | Bounded neighborhood | Exact selection and eligible evidence |
| memory.graph_path | Bounded path | Explicit qualified input and limits |
| mcp.profiles | Profile inventory and rights | Brain access != Use |
| mcp.discover | Cached tool schemas | Does not start tools or retrieve credentials |
| mcp.call | Queue authorized call | Approved target/runner, immutable scope, stable request ID |
| mcp.status | Call/output disposition | Original scope and current Use |
| mcp.cancel | Request cancellation | External effect may already have occurred |
| mcp.reconcile | Approved receipt lookup | No blind replay of effect |
| mcp.resolve | Add scoped evidence about uncertain call | Preserve original uncertainty |
| mcp.release | Release idle client-session runtime | Active leases protected |
| workspace.refresh — native only | Bounded workspace Git metadata discovery | No fetch/extraction/ordinary file reads or Brain change |

No browser clicking is necessary for ordinary agent task/recall/contribution/tool
work once the user has configured access and policy.

## Tasks, capture and visibility

A task is an immutable-scope working context, not a project-management issue.
Own tasks/children/operation history are account-private even from other Brain
admins. Shared published capture evidence has its own visibility and retention;
do not derive permission from the new page layout.

[Workspace scope](../../contracts/evidence-workspace-scope.md) (65–117),
[capture](../../contracts/evidence-session-capture.md), and
[WorkspacePanel](../../../web/src/WorkspacePanel.tsx) (110–135) establish this.
Browser Refresh catalogue only refetches metadata; native refresh performs the
actual bounded discovery. Put scope/manual binding tools in session details.

Onboarding: choose Brain → pair companion if needed → configure supported host
and workspace → set permitted capture/model behavior → optionally grant approved
tools → verify a real memory read. Setup cards describe supported hosts; only
actual observations justify active/connected status. Recollect does not already
host/create general-purpose LLM agents.

## Human authority versus useful agent gaps

Keep human/operator controls for device approval, Brain/profile grants, provider/
content/budget/capture permission, retention, explicit human correction and Erase,
approved executable definitions, credential references and runner placement.
Existing protections are server-side, not just hidden buttons. Admin knowledge
access does not grant profile Use, even to the Brain owner.

Current catalogue gaps are not all missing product behavior. Some operations
exist through HTTP/native interfaces but deliberately not ordinary MCP tools.
Evaluate useful extensions separately:

1. Bounded exact source-version/span inspection behind a citation.
2. Read-only learning/processing status and reasons for paused work.
3. Published snapshot/coverage/manifest inspection.
4. Governed evidence-source contribution for permitted writers.
5. Explicit bounded analytics scheduling.
6. Native wrappers for already governed committed publication.

Each needs explicit schemas, scope, content permissions, provenance and tests.
Do not add arbitrary filesystem access, credential/admin changes, permission
self-grants, executable installation or human-review impersonation as incidental
UI parity.

## Why Ask is new product work

[RecallResponse](../../../crates/protocol/src/retrieval.rs) (117) returns bounded
items, attribution, scope/time, coverage, qualifications and expiry. The
[investigation contract](../../contracts/retrieval-investigation-ui.md) explicitly
excludes generated answers and saved cross-query investigations.

[Model gateway](../../../crates/server/src/model_gateway.rs) (48) has internal
synthesis/Text infrastructure, but no inspected Ask/chat/conversation endpoint,
answer-citation consumer, tool-calling chat or streaming answer path. A new
textarea alone does not produce the requested experience.

## Selected first Ask design

The user accepted this design; the linked contract supplies its exact implementation
boundary, including bounded current-question requests and metadata-only status.

### Read-only questions with meaningful scope

Ask returns an evidence-supported answer, exact citations, and concise
qualifications/insufficient-support results. Current Brain is explicit.
Default investigation eligibility preserves disagreements and uncertainties;
advanced accepted-only, operational-only, historical, exact/time/manifest and
retrieval-budget criteria remain available and visibly summarized.

An ordinary question does not write a source/claim/handover, enable capture,
dispatch tools, execute procedures, change policy, or treat its answer as new
evidence. Operational request/usage metadata is still recorded as appropriate.

A production chip does not prove deployment. A model may select retrieval
mechanics within standing policy, but not expand Brain access, remove strictness,
substitute a latest revision, settle a conflict by recency alone or turn intent
into observation.

### Explicit answering permission

Add a separate `answering` model purpose. Existing policies migrate with it off.
Current purposes are extraction, synthesis, embedding and reranking:
[policy](../../../crates/server/src/model_policy.rs) (22).
Prior learning/handover consent does not silently authorize interactive answers.

Both question and context must be allowed for transmission; current provider,
model, purpose, content class, concurrency and budget limits apply. Capture is
independent from sending content to a model. Present understandable settings
such as Answer questions, Learn from sources and Search by meaning without
collapsing their enforcement.

The current installation's provider/model pair is deployment-selected and the
UI displays it read-only. The redesign can simplify permission for those installed
models; browser-based provider installation/selection is separate new work.

### Temporary conversation and payload lifetime

First version keeps bounded conversation only in current browser memory:

- No saved conversation sidebar, server transcript, automatic local-storage
  cache, or automatic learning from questions/generated answers.
- Clear on Brain switch/reset/reload/logout/access loss. Clear answers/context
  when their evidence expires or changes according to validity metadata.
- Follow-ups perform new authorized retrieval; prior assistant text is
  conversational input, never independent supporting evidence.
- Ambiguous follow-ups clarify rather than guessing a different scope.
- Model gateway/request logging must not accidentally persist prompt/answer
  bodies contrary to this contract. Metadata retention gets explicit limits.

A later saved-history feature needs ownership, sharing, retention, exports,
erasure, searchability and restore behavior. It is not hidden scope in this
redesign.

### Canonical retrieval-bundle input

Do not trust a client-supplied RecallResponse, treat returned text as an arbitrary
new evidence source, or expand a permitted fragment into a whole source document.

The current gateway resolver uses whole source versions/current claim revisions/
repository facts: [resolver](../../../crates/server/src/model_gateway.rs) (163–275).
That can violate recall's tighter span, time, rejection, context-budget and
eligibility boundaries if reused naively.

Add a typed internal bundle preserving:

- Principal, Brain, request/operation authority.
- Repository/area/collection/environment and exact manifest selection.
- Fact time, frozen knowledge time, eligibility and qualifications.
- Exact canonical identities/revisions, permitted spans and provenance.
- Content classes, transmission permission, memory epoch, deadlines.
- Missing/partial coverage and the bounded packed context.

Server-side retrieval constructs it. Revalidate before provider transmission and
before answer publication; changed permission/policy/correction/erasure/expiry
suppresses output. Reuse existing gateway quota/admission/post-call suppression
rather than a parallel ungoverned LLM client.

### Citation support and limitations

Require structured references drawn from the exact supplied bundle. Validate IDs,
scope, revision/span, publication-time eligibility and source resolution.
Unknown/foreign/omitted citation IDs fail; guessed URLs are not evidence.

A valid citation proves attribution, not entailment or source truth. Evaluate
the actual answer for supported assertions, contradiction preservation,
uncertainty and appropriate abstention. Multiple copies of one source are not
independent corroboration. Recorded intent, committed code and verified deployed
behavior retain different meanings.

### Selected API lifecycle

The names below are accepted; implementation and validation follow the active
retrieval-ask-experience pack. They are not claimed deployed by this design file:

| Endpoint | Proposed contract |
| --- | --- |
| POST /api/brains/{brain}/answer-requests | Stable request ID, bounded question/follow-up and scope; server retrieves and returns validated complete answer |
| GET /api/brains/{brain}/answer-requests/{id} | Authorized disposition/usage metadata, not stored conversation replay |
| POST /api/brains/{brain}/answer-requests/{id}/cancel | Request cancellation of own active work |

Authentication supplies actor/Brain authority; no trusted client actor fields
or impersonation of a paired coding agent. Define admission limits, metadata
retention, in-flight tracking and reconciliation in the execution contract.

Duplicate request IDs cannot cause another provider attempt. If an ephemeral
answer is no longer available, return the known disposition, not a silently
regenerated completion. Explicit retry is a new potentially charged attempt.

Initially show stages and return a validated completed answer. Token streaming
needs additional publication semantics because the current gateway checks after
the provider returns; emitted text cannot be retracted.

### Failures, fallback and cost

- Answering disabled/no approved model: clearly labelled exact/text Search
  evidence, with an appropriate settings link; no invented answer.
- Missing semantic/graph channel: show actual coverage and preserve requested
  criteria. Do not quietly call another provider or lower strictness.
- Insufficient support/missing bytes/conflict: useful partial evidence and an
  explicit limitation.
- Budget/concurrency/denial/refusal/timeout: distinguish states; no implicit paid
  retry or provider fallback.
- Cancel: distinguish before-dispatch from in-flight/uncertain. It does not
  promise billing cancellation or reversal of transmitted provider work.
- Permission/epoch/retention failure: clear invalid answer/evidence rather than
  leaving sensitive cached content next to an error.

## Focused acceptance matrix

Use actual production handlers with useful positive controls.

| Area | Forbidden behavior | Positive proof |
| --- | --- | --- |
| Brain/scope | Foreign canary, widened environment/repository | Selected eligible evidence answers correctly |
| Time/modes | New/current material replaces requested historical evidence | Exact qualified revision is cited |
| Correction/erasure | Excluded text re-enters via full source or old chat | Unaffected evidence still works |
| Concurrent change | Output survives permission/policy/expiry change | Stable input completes |
| Citations | Foreign/unknown/omitted/latest-substituted citation | Exact span opens |
| Answer quality | Citation masks unsupported conclusion | Supported answer plus explicit uncertainty |
| Prompt injection | Source text changes scope/tools/policy | Benign content remains useful |
| Authority | Question causes tool dispatch or memory write | Answer and allowed usage metadata only |
| Model permission | Unapproved content/provider/purpose fallback | Approved configuration used |
| Cost | Duplicate/uncertain request silently retried | Explicit attempt separately accounted |
| Cancellation | Claims in-flight work unbilled/reversed | Before-dispatch causes zero provider call |
| Fallback | Search result labelled generated answer | Useful correctly labelled exact/text results |
| Conversation | Prior Brain/history becomes new evidence | Fresh scoped follow-up |
| Account privacy | Brain admin sees others' task/path details | Each account sees own contexts |
| Unavailability | Missing bytes/empty evidence yields confident facts | Honest partial/no-support state |

Desktop cases: new/populated/archived Brain; reader/admin; disabled answering;
missing credentials; denied content; exhausted budget; conflicts; expiry; and
dependency outage. Existing tests/source were inspected, not rerun in this
planning pass.

## Authority and implementation follow-up

The accepted Ask contract and decision-complete pack now belong to
[hybrid retrieval](../epics/hybrid-retrieval.md). ADR 0014 fixes the transport,
ephemeral payload and authority model before implementation.

Reconcile [model policy](../../contracts/memory-provider-policy-and-learning.md),
[investigation UI](../../contracts/retrieval-investigation-ui.md),
[retention/erasure](../../contracts/memory-retention-and-erasure.md), and the
[exact](../../contracts/retrieval-exact-and-lexical.md),
[semantic](../../contracts/retrieval-semantic.md) and
[graph fusion](../../contracts/retrieval-graph-fusion.md) consumer boundaries.
Preserve [workspace scope](../../contracts/evidence-workspace-scope.md).
Update MCP authority only if separately selected agent extensions are included.

Navigation/theme, setup simplification, Ask, and future action-capable/saved
conversation behavior remain distinguishable delivery slices.
