# Brain model catalogue and selection

Status: accepted

## OpenRouter amendment — 2026-10-08

The user authorizes OpenRouter as a selectable per-Brain provider and live public
benchmarks within the installation key's USD 10 limit. This supersedes the
OpenAI-only provider/catalogue statements below. Existing Brain defaults and
saved policies remain unchanged. Provider credentials stay installation-local:
`OPENAI_API_KEY` and `OPENROUTER_API_KEY`; fixed server-owned endpoints prevent
client-supplied URLs. A Brain selects one provider for text and embeddings.
Saved pre-provider command responses remain readable. Missing installed-provider
lists retain their legacy installation metadata; missing request-provider identity
means the historical OpenAI adapter, and missing cost remains unknown.

OpenRouter initially supports `meta/muse-spark-1.3-contributor`,
`z-ai/glm-5.3-flash`, `z-ai/glm-4.7-flash`, `openai/gpt-6-luna`, and `qwen/qwen3-embedding-8b`
(32–4096 dimensions, with 1024 as the economical initial selection). Catalogue
refresh fetches both text and embedding model lists, with provider-separated
caches and existing admin/CSRF, timeout, backoff and response-size guards.
Changing provider requires fresh discovery and explicit Save and rebuild,
including if model/dimension values otherwise match. Profiles and derivations
record the actual provider; old generations cannot publish or join new queries.

The shared gateway preserves all canonical input, purpose, scope, retention,
secret, token, concurrency and replay fences. OpenRouter text uses Chat
Completions strict JSON schema, required parameter support, a fixed selected
model and no model/provider fallback. Incomplete/refused/error responses fail
closed. Reviewed exact returned model aliases are permitted; arbitrary routing
substitutions are rejected. Embeddings preserve requested dimensions and validate
all indices and finite nonzero vectors. Qwen query embeddings use its documented
retrieval instruction; document text stays canonical. Usage includes completion
and reasoning tokens. Provider keys are excluded from model inputs and output.
GLM 5.3 Flash requires reasoning and defaults to max upstream; the adapter
selects its supported low effort for bounded latency and cost. The initial
OpenRouter selection is GLM 5.3 Flash at low effort, plus Qwen at 1024 dimensions.
GLM 4.7 remains selectable with optional reasoning disabled, as verified by
a strict-JSON live preflight. Its held-out support audit admitted three unsafe
cases; the separate GLM 5.3 diagnostic admitted none but failed one call closed
on rate limiting. Neither single run establishes production acceptance. This
initial provider selection does not replace any existing saved Brain policy.
The user's delegated budgeted model selection also permits GPT-6 Luna through
OpenRouter after both GLM LongMemEval experiments stopped on uncertain provider
timeouts. Luna uses supported reasoning effort `none`, the same strict JSON and
no-fallback adapter, and dated reference prices. Its live preflight returned the
exact namespaced model. Adding this option does not change an existing selection;
native support diagnostics govern any later default recommendation.
Following the user's renewed GLM/Qwen instruction, reviewed GLM text calls
prefer provider throughput within maximum USD 0.15 input and USD 0.50 output
per million tokens. This fixed adapter preference preserves required parameter
support and disabled fallbacks; no client routing controls or longer timeout
are introduced. A changed route is a separately frozen benchmark experiment,
not a retry or replacement of the uncertain prior attempts.
The user's explicit failure-rerun instruction permits a separately recorded
GLM 5.3 recovery route restricted to the reviewed `deepinfra/fp4` endpoint,
within the same price ceilings. Strict schemas, required parameters, disabled
fallbacks and the original timeout remain unchanged. Original throughput-run
failures remain evidence; the recovered result does not replace them.
Published rates
are dated reference rates: OpenRouter's selected upstream route can charge
different prices. Actual billed USD is retained on the request ledger when
the provider supplies it, including failed responses that carry usage.

Muse Contributor's model card discloses provider use of prompts/outputs for
improvement; surface this when selecting it. Live benchmark calls use public or
synthetic inputs only, owned disposable Brains/databases, frozen settings, a
durable request/cost ledger and a conservative cap below USD 10. Initial delivery
uses synchronous calls; asynchronous Batch support remains a separate adapter.

## Source

The user's 2026-10-05 instruction authorizes all ten browser management changes,
including inline AI permissions, account model discovery, explanatory prices,
per-Brain model selection and explicit embedding rebuilds. This amendment governs
selection where the original [provider policy](memory-provider-policy-and-learning.md)
and [semantic contract](retrieval-semantic.md) required the installation's pair.
Their authority, budgets, erasure, canonical-input and request fences remain.

## Contract

### Catalogue and compatible models

The server fetches the installation account's OpenAI `GET /v1/models` only on an
explicit browser-admin edit action or refresh through POST; merely opening settings
never contacts OpenAI. Entering Edit refreshes unobserved/stale availability while
retaining the saved selection, and exposes retry on failure.
Never send the credential to the browser. Require Brain read authority before
returning the cached catalogue. Use the configured
installation endpoint with redirects denied, five-second timeout, one-MiB response
bound and a shared serialized five-minute cache; failed calls back off for one
minute. Return safe failure codes, never provider bodies or credentials. A failed
refresh retains earlier metadata with its observation date and a stale state.
Account listing proves availability, not a successful generation or embedding.

Join exact model IDs to a dated, application-maintained compatibility and pricing
catalogue derived from official OpenAI model pages. Initially support GPT-5.6 Luna,
GPT-4.1 and GPT-4.1 Mini plus their explicitly documented GPT-4.1 snapshots, and
text-embedding-3-large/small. Exclude deprecated, unknown, fine-tuned, audio,
realtime and tool-specific entries. All offered text models support Responses,
strict JSON output and the existing bounded extraction, synthesis, answering and
reranking request formats. Luna uses reasoning effort none; GPT-4.1 omits reasoning.
An unchanged historical installation pair remains valid without discovery; changing
either model requires a fresh successful account observation of the chosen IDs.
No arbitrary endpoint, provider, fallback or customer-supplied model is accepted.

Prices are USD per million standard input/output tokens, separate from cached,
batch, regional and long-context rates. Each entry includes source URL and checked
date; entries older than thirty days are visibly stale, missing amounts show
Unknown. They explain relative cost, never guarantee invoices. The application's
32,768-byte input bound stays below the catalogue's long-context thresholds.

### Revision, permissions and rebuild

Admins edit the existing AI card in place, including transmission, independent
purposes, content classes, automatic memory and selected models. Changes stay local
until Save; Cancel restores the saved revision. Reader/archived views cannot mutate.
Save retains expected policy revision and idempotency. Purpose dependency changes
disable dependent automatic flags rather than enabling other purposes or content
classes. Installed credentials are distinct from Brain-selected models.

Embedding dimensions support positive integers up to 3,072 for large and 1,536 for
small. The picker offers supported dimension presets and retains current custom
dimensions. Every gateway consumer uses the saved Brain policy's model/dimensions.
Gateway output checks and publication fences still reject wrong model/dimensions,
nonfinite/zero vectors, missing/duplicate indices and changed policy/authority.

Changing embedding model or dimensions requires explicit `rebuild_embeddings:true`
on the policy update. Present Save and rebuild, with a concise cost consequence
only while that selection has changed. Policy revision, retirement of old vectors,
cancellation of old queued/running projection jobs and creation of a new immutable
generation commit atomically. A duplicate receipt returns the same revision and
generation without repeating rebuild work. An old in-flight result cannot publish.
No incompatible projection remains usable or joins a new query generation.

Store full-precision vectors in an unconstrained pgvector column, enforcing each
entry's dimension against its immutable profile in the database. Exact cosine
queries remain bounded and filter the active profile before distance calculations;
do not introduce ANN indexes or mix dimensions/models. Existing 3,072-dimensional
vectors survive migration without an automatic provider call.

The standing worker enumerates and queues only canonical, retained, permitted
inputs for the new generation, under current policy, budgets and erasure fences.
When standing embedding or transmission is disabled, or credentials are missing,
the generation remains visibly blocked and causes no provider call. Coverage
distinguishes indexing/blocked/failed/ready; lexical and exact access continue.
Changing text model does not rebuild embeddings. Saving unrelated policy controls
does not create a generation. New Brain creation keeps its existing atomic managed
defaults; existing policies are never overwritten by migration or a UI preset.
Remove recommended-default controls from normal settings.

## Acceptance

Isolated HTTP/provider and disposable PostgreSQL fixtures prove catalogue bounds,
unknown/deprecated filtering, roles, refresh/cache failures, provenance, supported
selection, stale revisions and unchanged historical defaults. Prove small/large
dimensions, profile guards, atomic Save-and-rebuild/replay, old-output suppression,
policy/erasure fences and lexical access during rebuilding. Browser tests prove
inline edit/Cancel/Save, purpose dependencies, readers, real picker choices/prices,
the changed-embedding cost message and removal of preset controls. No paid calls
or customer corpus transmission are needed for this implementation proof.

## Explicit Deferrals

Additional providers, arbitrary/fine-tuned models, automatic compatibility inference,
billing estimates and ANN indexing require their own authority and adapter proof.


## First-batch availability observation — 2026-10-08

A separate authorized diagnostic evaluated the published `novita/fp8` quote
under the same strict-schema and price constraints. Runtime requests failed;
price alone did not establish interface compatibility. It is not an accepted
product route. The adapter retains `deepinfra/fp4`, strict schemas, required
parameters, disabled fallbacks and its original timeout. Reports retain both
failed/uncertain diagnostics and their unresolved billing reservations. No
score improvement is inferred from incomplete provider runs.
