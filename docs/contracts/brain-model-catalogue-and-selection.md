# Brain model catalogue and selection

Status: accepted

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
