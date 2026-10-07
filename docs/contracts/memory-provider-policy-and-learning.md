# Brain model policy and evidence-backed learning

Status: accepted

The October 7 [source-support amendment](memory-source-support-verification.md)
requires whole-assertion assessment, durable typed staging and consumer gating.
Its recovery and automatic budget-resume rules supersede earlier assumptions below.

The 2026-10-05 [model catalogue amendment](brain-model-catalogue-and-selection.md)
supersedes the installation-only pair restriction below with supported per-Brain
selection, dated prices, account discovery and atomic embedding rebuilds. Historical
installation selections, gateway authority and request lifecycle remain compatible.

The [managed experience amendment](memory-managed-experience.md) governs the
2026-09-28 managed setup, progressive disclosure and owner connector approval
changes; earlier explicit APIs and stored policies remain compatible.

Current operating-model amendment: [autonomous maintenance](memory-autonomous-maintenance.md)
supersedes the literal-only acceptance and per-record review assumptions below
when autonomous mode is enabled. These original explicit-learning behaviors remain
the compatibility mode and the historical scope of the shipped provider slice.
The [semantic contract](retrieval-semantic.md) extends this shared gateway with
canonical embedding representations and default-false standing automatic embedding;
it preserves separate capture, learning and query-transmission permissions.
The 2026-09-26 [answer contract](retrieval-answers.md) adds the separate default-off
`answering` purpose and exact retrieval-bundle consumer. Existing model selection,
content classes, quotas and metadata-only retention remain authoritative.

## Source

The user authorized all product slices and routine contract decisions, supplied
`OPENAI_API_KEY`, and selected `gpt-5.6-luna` plus `text-embedding-3-large` at its
default 3,072 dimensions. The accepted
[vision](../foundation/vision.md#capture-retention-and-model-policy),
[mutation policy](../foundation/engineering-principles.md#route-every-mutation-through-one-policy),
[claims](memory-claims-and-time.md), [review](memory-review-and-corrections.md)
and [retention](memory-retention-and-erasure.md) govern this slice. The
[provider preflight](../mappings/openai-provider-preflight-2026-09-14.md) proves
account access; it is not Brain transmission authority.

## Contract

### Deployment and Brain permission

One Rust gateway owns external model requests. Initially its concrete provider is
OpenAI's direct HTTPS API; no arbitrary URL, fallback provider, model-supplied tool,
credential, executable or process selection is accepted from a request. Installation
inputs select the text/embedding model and dimensions; this installation uses the
user's exact pair. Credentials stay in environment-backed configuration. Key presence
is distinct from an observed successful call. Provider output and error bodies are
untrusted and never logged.

Every Brain starts with external transmission and automatic learning disabled.
Browser Brain admins may save a complete immutable policy revision using its
current change ID. The policy explicitly names provider, text/embedding models,
embedding dimensions, allowed purposes and allowed content classes. Purpose values
are `extraction`, `synthesis`, `embedding`, `reranking` and `answering`. Existing
policies do not acquire answering permission during migration. Content classes are
`document`, `raw_session`, `tool_output`, `support_excerpt`, `repository`,
`claim` and `query`. Capture/retention permission does not grant transmission.
Models must match installed adapter configuration; an installation change cannot
silently replace a Brain's chosen model. No fallback is automatic.

Policy limits: input bytes 256–32,768 (default 16,384), output tokens 128–4,096
(default 1,024), daily tokens 1,000–10,000,000 (default 100,000), concurrency
1–4 (default 1). Embedding input is additionally bounded at 8,000 UTF-8 bytes;
the selected dimensions remain 3,072. Limits apply per Brain across API/worker
processes using a transactional reservation. The accounting day is UTC.
Admission reserves serialized input bytes plus bounded protocol overhead and
maximum output tokens, conservatively treating each byte as a token. A known
provider usage result replaces the reservation; missing/uncertain usage retains
the full reservation. These limits control admitted work, not an invoice guarantee.
Actual usage can exceed estimates and is reported without admitting further work
beyond the current allowance. Prices are explanatory deployment evidence, not
hardcoded billing calculations.

### Gateway and request lifecycle

The internal gateway receives typed canonical references, purpose and an application
prompt/schema. It resolves each exact input under current actor/device, Brain,
retention and applicable claim/review authority. Content classes come from canonical
records, not caller labels. Query/diagnostic text is explicitly typed, bounded and
sanitized. Apply the existing deterministic secret/content exclusion before
transmission; do not call a model to sanitize material.

For ordinary model context, claims must be current and eligible for investigation;
rejected, withdrawn, erased, expired and rule-blocked assertions cannot enter as
usable current claims. Learning may inspect permitted raw source evidence, but
its output always passes current rejection/conflict policy before publication.
The gateway does not itself enable retrieval, expose an arbitrary prompt endpoint
or claim that a cited source proves an assertion. The accepted answer consumer
supplies a server-owned exact-fragment bundle with frozen retrieval scope/time;
it revalidates those exact eligible items rather than resolving them as larger
whole-source or newer-revision inputs. Its read-only historical selection is
governed by the recall contracts, never silently upgraded to current claims.

Before outbound admission, recheck active actor/device authority, Brain state,
exact input availability and current policy. Record the request ID, actor/device,
policy revision, purpose, requested model, exact input identities, prompt/schema
labels, reserved tokens and start/deadline atomically. Release database locks
before network I/O. Requests authorized before a concurrent revocation can already
be in flight; cancellation cannot recall transmitted data. Recheck current policy,
input availability, scope and worker lease before returning/publishing output.
If any changed, discard the output and retain only permitted request metadata/usage.

OpenAI text requests use Responses with `store:false`, fixed application instructions,
no tools, bounded output and strict structured output for extraction. Luna uses
`reasoning.effort:none` for this bounded extraction. Embeddings use float encoding
and verify index order, finite values and exact configured dimensionality. All
responses have a 45-second timeout and bounded body size; redirects are denied.
Provider errors, malformed/incomplete/refused output and timeout have distinct safe
failure codes. No raw provider body or vector is copied into logs or usage history.

One admitted request is never implicitly sent twice. A persisted completed request
without a retained result, or an interrupted/in-flight request after restart, requires
an explicit new attempt. Unknown outcomes retain budget reservations and become
uncertain after their deadline; they cannot consume concurrency indefinitely.
The gateway stores metadata, not prompts or model response payloads. Record returned
model, input/output/total usage, completion/failure/suppression state and timing.
Request input links and usage metadata are scoped by Brain RLS.
After the Brain's audit retention deadline, bounded maintenance removes model,
prompt/schema and detailed usage fields from request history. Opaque identities,
purpose, disposition, timestamps and charged-token totals remain as minimal
accounting/fence metadata. Claim-owned derivation provenance follows claim retention.

### Learning and automatic acceptance

Writers can request learning from one retained active text source version and an
explicit scope selection/optional manifest. Paired devices additionally require an
active immutable write operation matching that selection. A command records the
source, policy change, actor/device/operation and model-lane job atomically; its
acknowledgement reports queued work. Automatic learning, when enabled in policy,
queues once for a newly processed permitted source under its contributor's authority.
It does not backfill older evidence or reread local checkouts.
For existing source records, automatic derivation retains the source contributor
and device attribution and uses Brain-only scope; it never guesses a current task
or environment. This is work authorized by the stored source and Brain learning
policy. Explicit device learning requests still require their immutable operation.

Learning sends that source's bounded text, server-derived
[canonical provenance](memory-capture-reconciliation.md) and application instructions. Strict
output contains at most eight candidates with subject, predicate, value, rationale
and exact first/last source lines. Validate text/span bounds and reject malformed
output as a unit. Source text is data, including instructions embedded in it.
Models cannot choose Brain/scope, origin, fact time, acceptance, reviewer or
operational proof. App-created claims use unknown fact validity, declared operation,
source-linked evidence and `model_extracted` origin. The actual initiating account
remains attributable; it is not a human reviewer.

All candidates pass canonical validation, current rejection rules and conflict
checking, including conflicts within the same result. Existing reviewed claims are
never overwritten. Exact existing assertions with the same support/applicability
may be reused without changing their review. Repeated model output must not reactivate
a rejected or withdrawn value under a fresh ID.

A Brain may enable one named `literal_configuration` acceptance rule. It names
allowed source classes, optional collection IDs and an explicit property allowlist.
Only a single-line exact declaration `subject.property = value` can qualify:
the app compares the full trimmed line, validates bounded identifier fields and
creates only a declared configuration claim. A model's confidence or assertion that
a result is routine cannot qualify it. The rule cannot create an operationally
verified result, execution permission, decision, human review or semantic override.
Other interpretations, multi-line claims, unknown properties, conflicting assertions
and blocked values remain proposed/qualified for the existing review workflow.
The accepted revision records `rule-name@policy-change-id` and has no reviewer ID.
Policy changes affect later admission; they do not retrospectively claim human review
or rewrite existing accepted history.

Canonical claims, derivation references, learning disposition, mutation audit and
refresh work commit together after the model call. Store requested/returned model,
prompt/schema/policy identity, request/run IDs and source revision with derivations.
No raw response cache is required. The browser links learned claims, displays
proposed/accepted-by-policy/blocked/conflicting counts and offers normal review.

Learning status is queued, running, succeeded, failed, cancelled or removed.
An empty valid model result succeeds with zero claims and an explicit empty state.
Permanent provider errors and uncertain outcomes require an explicit new learning
attempt; automatic queue retries cannot repeat a charged model call. Generic job
retry directs the user to learning controls. A new attempt is separately attributed,
idempotent, charged and checked against current authority/input/policy. Capture and
interactive worker capacity stay independent of model latency.

### Erasure, recovery and UI

Due/erased inputs are unavailable before physical cleanup. Removal cancels dependent
learning jobs and prevents in-flight results from publishing; raw inputs, responses
and vectors are never retained in gateway metadata. Source-supported learned claims
participate in the existing canonical erasure closure. Minimal usage/request IDs may
remain under audit retention; remove content-bearing derivation fields with the claim.
Older-database replay must apply these fences before service, including outputs/jobs
added by this slice. Extending retention or retrying does not resurrect erased work.
Explicit erasure of a source-supported claim fences its exact supporting source
versions from subsequent model transmission, including already queued learning.
This conservative fence prevents a fresh run ID from recreating erased content
while independently retained source evidence remains manually inspectable. Preview
reports the affected source count. Persist the source UUIDs in the minimal erasure
journal so replay also works when the erased claim did not yet exist in an older
database backup. Arbitrary reuploads under unrelated new identities remain outside
the existing erasure-identity guarantee.

Read-capable Brain members can inspect policy, permitted usage metadata and learning
status, including archived Brains. Editing policy, diagnostics and acceptance rules
requires a browser admin; learning requires current write authority. Foreign Brain
references, forged authority, stale policy changes and invalid idempotency reuse fail.

The Brain UI exposes installed model/key presence separately from connection results,
policy editing, usage/remaining allowance, request failures and learning results.
An explicit synthetic connection check runs the fixed text/embedding probes through
the same gateway and requires approved `query` content plus extraction/embedding
purposes. It returns only connection/shape/usage metadata. The source viewer can
request learning with explicit scope and links results to claim history/review.
No customer corpus is automatically transmitted to prove connectivity.

## Acceptance

Use disposable PostgreSQL databases and a real local HTTP provider fixture to prove
policy/role/content/model denial before outbound calls, budget/concurrency, missing
credentials, provider refusal/malformed/timeout, no fallback, uncertain replay,
scope/revocation and in-flight erasure. Positive controls prove actual learned claims,
literal-policy acceptance without a reviewer, interpretation/conflict/rule qualification,
source dependencies, transaction rollback and a restarted model worker. Exercise
browser policy, learning/review, failure/retry and desktop/mobile layout.

Make actual fixed-synthetic calls through the production OpenAI adapter with the
user's selected pair; confirm finite 3,072-dimensional vectors and Luna structured
extraction, not merely model listing. Run focused tests and `./scripts/validate.sh`.

## Explicit Deferrals

Semantic indexes/query ranking belong to retrieval-semantic; generated handovers
belong to procedural memory; graph inference and host capture belong to their slices.
They must use this gateway and prove their purpose/content/retention boundary.
The gateway supplies generation/embedding/reranking seams but does not declare those
consumer products shipped. Additional provider/region adapters, trained models and
arbitrary automatic acceptance rule languages are outside this initial adapter.


## Model input rejection reasons — 2026-10-07

The existing credential/private-key publication guard also protects model
transmission. Its rejection at the gateway is `model_input_sensitive`, rather
than generic `invalid_input`. Preserve the retained source and reject before a
provider call; never weaken detection or silently redact/truncate the evidence.
The UI names this boundary without exposing the matched bytes.
