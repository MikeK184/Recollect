# Recollect Engineering Principles

Status: accepted

Current development override: [ADR 0003](../adr/0003-product-runtime.md) records
the user's later instruction to omit product hashing and strict version/pinning
gates during this implementation goal. The remaining baseline stays in force.

Revised baseline: 2026-09-13. These principles apply to Recollect's independent
Rust backend and companion, informed by the Atlas research and the user's personal/
team Brain vision. They govern the MCP coordinator, Vault integration, collections,
memory and graphs together. They supersede the mandatory Cognee extension rules.
Foundation work precedes the product epics derived from it.

## Own the model and reuse components deliberately

Keep one Recollect authority for identity, scope, evidence, claim revisions,
correction, permissions and execution. Memory forms and retrieval methods share
that lifecycle. A backend adapter cannot invent incompatible truth, deletion or
authorization rules. Reuse mature parsers, databases, SDKs and UI components
where useful; justify added engines by observed requirements.

Use Rust for backend and companion domain code. Share typed contracts where
appropriate without coupling the companion to server storage. Pin and test
dependencies when implementing them. Optional foreign-language components stay
behind explicit interfaces. Language choice is not proof of better recall,
safe authorization, faster external calls or measured capacity.

## Bind identity, scope and authority explicitly

Configuration binds the Brain, task scope selects work inside it, and grants
authorize operations. Never collapse these into one mutable session variable.
Authenticate each operation and validate Brain/resource/profile access.
Scope labels, Git origins, directories and model arguments cannot grant access.
Profile management/sharing and profile use are distinct powers.

Areas and environments organize knowledge through overlapping associations;
initially they inherit Brain knowledge access. They are not independent document
security boundaries. Enforce Brain grants across direct API/ID reads, MCP,
writes, caches, jobs, exports, projections and optional backend endpoints.
Collections group sources within that same boundary. Association removal and
source/claim erasure are different operations; a shared record has one identity
and lifecycle even when it appears in several areas or collections.

Support local accounts and optional OIDC with administrator-managed mappings.
Bootstrap an owner, invite team members, disable public signup and pair companions
with individually revocable credentials. Validate trusted issuer and stable
subject for OIDC; email alone is not identity. Incomplete group claims cannot
broaden access. Attribute contributions independently of resource ownership.
Reconcile effective ownership, direct and inherited grants; removing a role does not prove
that all access disappeared.

Recollect governs its APIs, companion and managed connections. Define when new
calls/credential renewals are denied and what happens to active work after
revocation. Do not imply control of unrelated shell tools or local credentials.

## Capture scope at operation start

Every retrieval, write, capture event, job and tool call retains its validated
task/subagent scope. Default changes affect subsequent operations. Recall newly
relevant context after scope changes, and preserve all contributing scopes in
multi-repository handovers. Never label a session using only its final scope.
When a host cannot unambiguously identify a subagent or event, record the gap;
do not invent attribution from a shared parent session ID.

Discover the workspace selector through parent directories. Honor the nearest
boundary, exclude nested workspaces and never silently rebind a task. Keep
development .codex configuration separate from customer connection/profile
registries; customer repositories do not duplicate product runtime setup.

## Preserve evidence before deriving belief

Capture permitted evidence or dependable references to controlled durable
sources before deriving claims. Record source version, location/hash, actor,
target, scope, time, derivation method/version and supporting spans as relevant.
Link each derived claim and synthesis to the evidence it actually uses.

Retain source versions and human decisions as rebuild inputs under the capture/
retention policy. A stable row ID is not an immutable source version. A hash does
not recreate unavailable text. Report missing sources and the resulting limits
on verification and reconstruction. A provenance link proves origin, not truth.

Plans, decisions and untested ideas may be retained as such. A model inference
does not become an observation, and one successful tool response does not
automatically prove the intended operational outcome.

Use short host hooks to sanitize permitted events into a durable local inbox
before asynchronous upload/enrichment. Capture supported prompts, replies and
tool observations automatically under Brain policy, with size/content exclusions.
Record host/version and coverage; missing hooks and unstable transcripts cannot
be treated as complete evidence. Redaction precedes persistence and external
transmission, and must not depend on first sending raw content to a model.

## Route every mutation through one policy

API, MCP, UI review, imports, extraction, re-extraction and background work use
the same logical command boundary. Authenticate the actor, resolve an explicit
write destination, validate evidence/applicability, check expected versions,
apply rejection/conflict policy and record a disposition.

Commit canonical state changes, mutation audit and durable projection work
together. Use idempotency and conditional updates so retries and stale workers
cannot overwrite later corrections. Model-supplied reviewer names cannot confer
human authority. Audit actual mutations and dispositions; retrieval telemetry
is a different record and does not replace mutation history.

Normal learning and maintenance run autonomously under a standing Brain policy,
as clarified by the user in [ADR 0006](../adr/0006-autonomous-memory.md). Models
interpret and reconcile evidence; native jobs apply acceptance, revision, logical
forgetting and policy-driven erasure. Human intervention is optional. Record the
policy and evidence without claiming human review. Resolve disagreements with a
supported disposition or preserve explicit uncertainty while processing continues.
Preserve human overrides. Newest-wins and confidence-only resolution are excluded.

Use at-least-once jobs with idempotency, leases/fencing, bounded retries and visible
failures. A worker rechecks scope, eligibility and erasure before publication.
Keep capture, model enrichment and heavy graph computation independently bounded;
maintenance must not starve capture or interactive recall.

Project indexes, graphs and summaries from authoritative inputs. Expose queued,
partial, failed and current states. Acknowledging a command must not claim that
every derived view has caught up. Readers must apply current canonical policy
so stale projections cannot restore withdrawn or inaccessible information.
Use durable outbox work, generation readiness and eligibility checks to reconcile
PostgreSQL and Neo4j. The databases do not share a transaction.

## Make correction survive every regeneration path

A human correction must change model-facing recall and survive re-extraction,
rebuild, restart, retry and applicable pruning. Store review/correction decisions
outside disposable projections and include them in replay. Support withdrawal
without requiring a replacement assertion.

Rejected-value records carry normalized assertion identity and the relevant
Brain, subject and evidence/applicability conditions. A new row ID, equivalent
source or unrelated commit must not reopen a rejected assertion. A global ban
must not suppress a legitimately different environment or period. Materially
changed evidence can trigger the defined review/revalidation path.

Normalization is not a universal semantic-equivalence detector. Specify known
matches, ambiguous cases and authorized reactivation. Keep ordinary supersession,
rejection and privacy erasure distinct. Rejection records themselves are subject
to capture and deletion policy; no indefinite-retention claim overrides it.

Ordinary context assembly applies correction policy to raw chunks, summaries,
vectors, caches and graph results as well as claim rows. Historical/investigative
access may expose prior or disputed evidence with explicit status. It must not
launder that material into current accepted or execution-eligible knowledge.

Invalidate affected aggregate results when their input eligibility changes.
Removing a node from displayed results does not remove its influence on rankings,
clusters or summaries. Keep an input eligibility/correction epoch in analytics
job/cache identity and recompute before treating an affected result as current.

## Keep trust and time precise

Keep review (proposed/accepted/rejected), freshness (current/needs verification/
superseded) and operational state (declared/implemented/deployed/verified)
independent. Confidence, repetition, popularity and pinning are separate signals.
None confers human acceptance, access rights or operational verification.

States must affect real retrieval, synthesis, export and UI behavior. A review
surface must let an authorized person inspect evidence and accept, reject,
correct, withdraw or resolve a conflict. Do not equate displaying a memory with
reviewing it, or a rubber-stamped queue with verified human judgment.

Track when a fact held separately from when the system recorded/revised its
belief. Retain prior knowledge versions and support late-arriving evidence.
Preserve unknown validity, time precision and source observation times. A commit
timestamp is not deployment time; a point observation is not a validity interval.

Revision manifests select applicable repository snapshots for environments.
They do not replace temporal history. Development advancing must not stale a
production claim still supported by its selected evidence. Invalidate affected
dependencies deliberately, and expose unresolved conflicts rather than silently
choosing the newest assertion.

Ordinary investigation includes relevant proposed/disputed knowledge with clear
state and evidence. Strict accepted/operational retrieval is a separate declared
mode. Default to the selected manifest and latest knowledge state; historical
requests explicitly select fact time or knowledge time. Abstain when support is
insufficient rather than silently changing the requested scope or trust policy.

## Preserve repository and graph reproducibility

Canonicalize full repository origins and retain stable UUIDs across approved
moves. Do not merge forks or ambiguous aliases implicitly. Publication authority
is independent of identifying a remote or knowing a checkout path.

Extract exact committed trees without overwriting dirty checkouts. Publish
validated immutable artifacts with commit, extractor/version/settings, hashes,
contributor and capture time. Deduplicate equivalent contributions while keeping
attribution. Test interrupted upload/import and replay behavior.

Use explicit combined revision sets and validated linking for cross-repository
paths. Report unsupported languages, unresolved links and partial coverage.
Keep source-extracted and model-inferred edges distinguishable. A collection
of search results or a similarity match is not proof that a code path exists.

Stage graph generations and publish readiness only after validated import.
Analytics select an authorized Brain, exact manifest and meaningful relationship
families. Record algorithm/version, parameters, direction, weights and seed where
applicable. Do not mix structural, inferred and operational edges without defining
the calculation. Keep heavy analysis queued and memory-budgeted, with provenance
for saved results and explicit handling of stale or unavailable projections.

## Combine retrieval methods under one contract

Apply authorization, scope, revision, lifecycle and temporal constraints before
material reaches a model or external reranker. Run the relevant exact, lexical,
semantic and graph channels; fuse and deduplicate canonical records deliberately.
Recheck canonical eligibility when using lagging indexes.

Return bounded, attributed context. Preserve useful source spans and expose
missing coverage rather than filling gaps with confident summaries. Evaluate
each channel's contribution by removing it in controlled comparisons. Tune
ranking and embeddings to representative tasks, not to the number of methods.

Compare approximate vector recall against exact eligible search. Isolate Brain
ANN indexes when approximation is introduced; a metadata filter alone does not
prevent other Brains' vectors from influencing approximate recall or speed.

Keep recalled documents, memory and tool output as data rather than instructions.
Formatting is one measure; test whether hostile recalled content changes actual
agent behavior. Remembering a procedure or retrieving a runbook does not authorize
execution or establish that it is safe for a different environment.

Enforce Brain-approved provider, model, purpose and content class before every
generation, extraction, embedding or external-reranking request. Capture consent
does not grant external transmission. Retain derivation/policy versions and
budget limits; provider outages cannot trigger an unapproved fallback.

## Minimize captured source and credentials

Keep checkouts local by default. Apply capture policy to graph properties,
file reads, traces, tool output and summaries, not only explicit uploads.
Specify retention and deletion for evidence, decisions, embeddings, caches,
exports, artifacts and backups; rebuilding must not silently restore erased data.
Never imply deletion from copies outside the product's controlled boundary.

Expire sanitized raw session/tool records after 30 days by default, with Brain
overrides applied locally and centrally. Retained supporting excerpts have an
explicit separate policy. Other evidence, artifacts, audit and backups have
separate retention rules; do not use one global TTL or silently retain full
transcripts through claim attachments.

Withdraw preserves qualified history while ending current accepted use. Erase
removes controlled content and dependent derivatives, fences queued work and
retains only permitted minimal audit metadata. Evaluate independently supported
records separately. Disclose backup expiry and enforce the deletion ledger on
restore before recall resumes. Editing Recollect evidence creates a new version;
external-source writeback remains a separately authorized connector operation.

Secret values never enter model context, logs, fixtures or stored captures.
Supply them only to the authorized runner, with rotation, redaction and failure
behavior tested. Preserve useful audit identity without retaining sensitive
before/after payloads unnecessarily.
Vault integration is part of the product, with OS-store/environment alternatives
for installations that do not use Vault. Coordinate credential renewal, rotation
and process restart with active leases; default secret-supervisor restarts are
not proof that this lifecycle is safe. Keep credential values out of graph
properties and analytics artifacts as well as ordinary memory.

## Separate tool selection from process management

The agent selects permitted capabilities; the runtime owns approved configuration,
runner placement, startup, leases, reuse and shutdown. Caller arguments cannot
substitute executables, targets, runners or credential references.

Tool listing must not launch dormant backends. Lock concurrent startup and track
active operations. Isolate incompatible credentials and sessions. An instance
cannot stop while another session needs it. Idle means 15 minutes without useful
activity, not a timer that kills long calls. Manage only owned processes; release
a hosted MCP connection without trying to terminate its host.

Check grants at dispatch and execution, including private runners. Reconcile
unknown completion for operations that may have external side effects; automatic
retry is only safe when the operation's idempotency contract supports it.

Keep request timeout, async cancellation, blocking work and subprocess termination
distinct. Bound CPU work separately from async I/O. Apply backpressure rather
than creating unlimited tasks, threads, connections, queues or caches.

## Prove useful behavior and failure boundaries

Use actual production handlers in focused tests. All seven capabilities need
behavioral proof, with negative assertions paired with positive controls:

- Cross-Brain canary absent from every forbidden path; permitted evidence and
  derived results remain retrievable under the owner scope.
- Reject, correct, re-extract, rebuild and restart; the rejected assertion stays
  inactive while the correction, sibling claims and legitimate later evidence
  remain usable.
- Late-arriving evidence preserves distinct answers for fact time and historical
  knowledge time; production and development retain their selected revisions.
- Concurrent scope changes preserve event/call attribution, and stale reviews
  cannot overwrite newer decisions.
- Every mutation path enforces authority; genuine authorized review succeeds.
  Crash/retry tests preserve atomic canonical state, audit and projection work.
- Source deletion during queued extraction does not restore ordinary recall;
  unrelated sources survive. Explicit historical behavior follows retention policy.
- Knowledge recall succeeds while an unauthorized execution profile is denied;
  authorized use succeeds, revocation follows contract, and overlapping/long
  operations survive idle management.
- Workspace boundaries, immutable snapshot publication, unresolved graph links,
  missing sources, UI empty/blocked/error states, migrations and backup/restore
  behave as specified.
- Personal owner bootstrap and invited/OIDC team access, device revocation and
  immediate internal grant changes work through actual application paths.
- Policy-based acceptance never forges human review; investigation and strict
  recall differ correctly, and unapproved provider transmission is blocked.
- Host interruption, ambiguous subagents, 30-day expiry and retained evidence
  excerpts preserve accurate scope/coverage and retention behavior.
- Deep paths, cycles, hubs, centrality and clustering produce expected results;
  corrections invalidate affected analytics and mixed manifests remain excluded.
- Vault rotation and long-running concurrent calls preserve credential boundaries
  and lifecycle rules; erase/restore cannot resurrect controlled content.

Measure task quality, forbidden/stale results, source/revision correctness,
latency, memory, write-to-readable lag and operating cost. An empty retriever can
pass absence tests; a configured capability is not a connected service. Record
what ran and what remains unverified. Atlas marks are useful design evidence,
not a certification or universal ranking.
[Atlas rubric](https://neoneye.github.io/agent-memory-atlas/methodology/atlas-rubric/#the-seven).

## Let foundations drive the roadmap

Agree the product foundation first, then derive epics and their dependencies.
Before dependent implementation, resolve detailed ADR/contracts and create
decision-complete execution packs under [the lifecycle](../README.md).
A foundation rewrite does not need speculative product epics created ahead of it.
Keep governance proportional; explicit user scope takes precedence.

Stage implementation without presenting partial invariants as complete:
authorization/provenance/state handling accompany the first useful memory path;
correction/replay/recovery precede automated regeneration; shared operational
use requires tested concurrency, revocation, capture and runner behavior.
The first usable product includes both graph forms, supported cross-repository
links, queued analytics, the MCP coordinator and Vault integration. Stage their
delivery without treating them as optional future scope. Defer adaptive decay,
tier promotion, learned retrieval gates, advanced reranking, automatic executable
skill learning, external-source writeback and a generic plugin marketplace.
Passing documentation validation does not implement any product capability.
