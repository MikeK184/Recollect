# Governed semantic representations and recall

Status: accepted

Browser presentation follow-up, 2026-10-06: semantic readiness, counts and
recorded failures remain available through optional diagnostics. Normal AI
settings do not display a coverage alarm or ask a person to review/retry items.
The bounded automatic replacements, pre-request block reconciliation and
uncertain/permanent/exhausted idle outcomes below remain unchanged. Exact and
lexical retrieval continue independently; no UI removal broadens paid retries,
provider transmission, policy, generation or erasure authority.

The 2026-10-05 [model selection amendment](brain-model-catalogue-and-selection.md)
supersedes the fixed 3,072-dimensional storage and installation-pair assumptions
below. It adds model/dimension-safe full-precision storage and an atomic explicit
Save-and-rebuild generation while preserving scope, canonical input and erasure fences.

## Source

The full product goal authorizes this original slice and its routine decisions.
The user selected `text-embedding-3-large` at 3,072 dimensions, with
`gpt-5.6-luna` for separate text operations. The accepted
[retrieval baseline](retrieval-exact-and-lexical.md),
[provider policy](memory-provider-policy-and-learning.md),
[autonomous maintenance](memory-autonomous-maintenance.md),
[retention](memory-retention-and-erasure.md),
[capture reconciliation](memory-capture-reconciliation.md) and
[stack](../foundation/techstack.md#storage-ownership-and-hybrid-retrieval) govern.
[Interface evidence](../mappings/semantic-interfaces-2026-09-15.md) records current
provider/database limits. The [Atlas audit](../mappings/atlas-implementation-audit-2026-09-15.md)
supplies consumer obligations, not a competing authority.

## Contract

### Standing permission and representation identity

Add `automatic_embedding`, default false for old/new Brain model policies.
An admin enables it once through the existing model policy. It requires approved
embedding purpose and exact installed model/dimensions. Automatic learning remains
an independent permission. Source classes are server-derived and must be permitted;
query embedding additionally requires `query`. Existing Brains do not acquire new
transmission permission during migration. No per-record human approval is required.

Each Brain's semantic profile records a UUID generation, provider, model, dimensions,
representation label, initiating policy/actor and creation time. The first enabled
profile is created automatically. A model/dimension/representation mismatch makes
the existing profile unavailable and requires an explicit admin reindex through
the currently approved installation/policy. Never substitute or silently shorten
the requested embeddings. Generation UUIDs identify rebuild work; add no content
hashes, strict user version-pinning workflow or alternate canonical writer.

Use full-precision `vector(3072)` with exact cosine distance. Store vectors only in
the Brain-scoped projection, never in Neo4j, logs, model-request history or browser
status responses. Reject wrong returned model, dimensions, duplicate/missing batch
indices, nonfinite components and zero-length norms. No ANN/half-precision index is
introduced. Model requests and entries retain the exact profile/representation and
canonical input identity; incompatible vectors cannot participate in ranking.

### Canonical text representation

`engineering-text-1` is deterministic, bounded UTF-8 text resolved by the shared
gateway from canonical IDs. Clients cannot supply index text, vectors, classes or
provider URLs. Do not serialize mutable eligibility assessments into embeddings.

- Source chunks: source title, canonical capture role when applicable, and the
  existing exact chunk text. Verify the stored span against retained artifact
  bytes. Original byte/line attribution remains attached to the result. Missing,
  reference-only or unprocessed content supplies coverage, not invented vectors.
- Claims/decisions/procedures/handovers: labeled kind, subject, property, value,
  rationale, declared operational state and typed procedure/handover content.
  Resolve the exact current revision through canonical eligibility. Do not copy
  all supporting documents, contributor histories or surrounding conversations.
- Repository facts: the exact fact's bounded canonical JSON with its repository
  path/name information. Environment/deployment applicability stays a separate
  canonical filter; embedding a fact does not verify deployment.
  Current indexing includes the latest known repository snapshot and retained
  snapshots selected by current manifests, so a current production selection can
  remain searchable while development has a newer repository revision.
- Manifests: the exact retained manifest revision's bounded canonical JSON.
  This is a declaration with original snapshot attribution, not an inference.

Limit each representation to 6,000 bytes on UTF-8 boundaries and record truncation
as coverage. Source chunks are currently at most 4,096 bytes before the bounded
title/role prefix. Query text uses the existing 512-byte recall bound unchanged.
Provider responses are bounded to 2 MiB for full-dimension embedding batches;
text-operation responses retain their existing 1 MiB bound.
Gateway accounting for embeddings counts the text actually transmitted; provenance
identities are retained separately. A batch has at most 20 inputs and at most the
smaller of 8,000 bytes or the Brain's configured input allowance. An individual
input that cannot fit its policy stays visibly blocked until policy/input changes.

Index newly eligible current canonical inputs and catch up existing ones when the
standing policy is enabled. Preserve previously built retained revisions for
qualified historical retrieval. Historical semantic coverage is limited to versions
actually represented; do not transmit now-rejected or fenced claims to fill history.
Canonical latest-known selection and current correction/erasure rules still decide
which retained historical vectors may be used. Reindex is rebuild of permitted
canonical inputs, never recovery from an obsolete text/vector export.

### Durable work and reindex

Discovery first excludes exact input identities already present in the active
profile, independently of entry state. Materialized missing-input cohorts may
then undergo the same canonical eligibility checks. Existing entries must not
be rediscovered or charged merely to recheck support. This is a query-planning
boundary, not an eligibility cache: missing inputs still require current source,
claim, snapshot, manifest, retention and model-input-fence qualification. Apply
the 100-input limit after qualification so unavailable inputs cannot hide later
eligible ones. A missing profile has no represented identities; historical
profiles do not suppress discovery in a new profile.

Track per-input pending, queued, running, ready, blocked, failed and removed states,
with content-free reason codes. Capture/source processing success is distinct from
embedding readiness. Bounded background discovery and mutation invalidation resume
after restart. Consider at most 100 inputs per Brain per maintenance pass; continue
from durable state. Keep at most 50,000 active-generation entries per Brain and
report capacity/backlog rather than dropping capture or canonical writes.

Group compatible inputs into a durable batch owning a model-lane job and one new
gateway request identity. Entry assignment, batch, audit and queue commit together;
the existing 500-job Brain capacity and model concurrency/token policy apply.
Worker execution validates current actor/grants, profile, policy, inputs and lease,
releases database locks before the provider call, then checks them again under the
Brain write lock before atomic vector publication. Renew the lease while continuing
to poll canonical work. Recheck relevant retention deadlines immediately before
commit. One invalidated input discards the batch output; independent batches proceed.
Required handover-contributor retention is part of this deadline. Ordinary raw
support and historical reasoning ancestry do not become the claim's own TTL.

A recorded charged attempt cannot be silently resent after interruption. Store
the outcome and expose uncertain/interrupted work. Under standing policy, confirmed
rate-limit/unavailable failures may create at most two separately recorded replacement
attempts after five and thirty minutes, as for autonomous learning. Pre-admission
budget/concurrency/policy blocks can resume when their constraint changes. Unknown
outcomes require an explicit new attempt; there is no automatic retry loop. Writers
may explicitly retry a failed batch through the shared command boundary.
For a partially removed batch, an explicit retry can select its remaining eligible
inputs; removed inputs stay removed. If no eligible inputs remain, retry is refused.
This is a new recorded attempt, never recovery of discarded vectors.

Admin reindex creates a new active generation transactionally, invalidates old
vectors for search immediately and rebuilds through the same queue. Partial new
coverage is visible; no mixture of generations or models supplies an unlabeled
fallback. Obsolete vectors are cleared in bounded maintenance. Already erased
inputs, suppressed requests and durable input fences cannot be rediscovered.

`GET /api/brains/{brain}/semantic` shows profile compatibility, standing permission,
coverage/counts and paged recent batches. `POST /semantic/reindex` is browser-admin
only and idempotent. `POST /semantic/batches/{batch}/retry` requires current writer
access and creates a separately attributed attempt. Raw vectors are internal.

### Semantic recall and fusion boundary

Add opt-in `semantic` to the existing recall channels. Existing defaults remain
exact plus lexical. Semantic requests require nonblank query text and a fresh
`semantic_request_id` UUID, identifying one admitted query-model attempt independently
of its immutable workspace context operation. A repeated request ID cannot charge
the provider again; because no query-vector cache is retained, an interrupted or
completed request without a response reports its recorded-attempt state. Browser
and native clients do not automatically resubmit paid queries.

Validate Brain/actor/device, operation selection, manifest/time and current query
permission before any provider call. Freeze knowledge time at admission, release
locks for query embedding and recheck access/profile/policy/current privacy before
reading results. Never hold the recall read lock while calling the write-authorized
gateway. If no compatible scoped representations exist, report missing coverage
without a model call. A denied/unavailable semantic channel is explicit; do not
silently return exact/lexical results as successful semantic search.

Filter Brain, scope, collection, known revision, manifest and current privacy before
distance ranking. Exact cosine search scans only the selected compatible projection.
Apply shared full claim eligibility, raw-assertion rules, artifact availability and
deadline guards while collecting candidates; continue past withheld rows to obtain
the best eligible semantic identities. Bound the scoped scan to 5,000 representations;
a larger selection returns a clear narrow-scope error instead of silently truncating
the exact eligible baseline. Strict modes contain no raw evidence. Current retention,
rejection and erasure override any historical vector or time selection.

Return up to 100 eligible semantic candidates before the existing result/context
limits. Expose cosine similarity and a `semantic_similarity_not_truth` qualification.
An optional minimum similarity in [0,1] permits explicit filtering; default 0 adds
no unmeasured learned relevance gate. Record match/miss behavior against the fixed
corpus before closeout. Similarity never establishes acceptance, corroboration or
operational verification; no generated answer is introduced.

When semantic is combined with exact/lexical, preserve explicit identity/literal
priority and combine remaining channel ranks using equal-weight reciprocal rank
fusion with k=60, stable canonical ties and deduplication. Report channel attribution
and the algorithm label. A fused source shows the exact semantic scoring fragment;
if its lexical rank comes from another fragment, expose that qualification while
preserving source identity and exact priority. Preserve the full attributed context-byte budget. Graph
candidates and graph ablations belong to the later fusion slice. Compare semantic,
lexical and combined modes on identical eligible corpus/context limits without
claiming that a more complex method always wins.

Keep four concurrent recalls per process. Exact/lexical retains its ten-second
operation bound; semantic permits ninety seconds including the existing 45-second
provider limit and bounded local canonical checks. SQL statements remain bounded
to two seconds. Timeout returns a safe error, never partial unqualified context.
Only model accounting/request IDs are retained for queries, not query text, vectors
or copied context. Fresh calls observe canonical changes with no result cache.

### Deletion and user surfaces

Canonical source/version, claim-revision and snapshot/manifest erasure immediately
invalidates the associated projection and in-flight batch publication. Clear vector
payloads when erasure/expiry is applied; minimal IDs/dispositions may follow existing
audit/fence retention. Lost-response replay and replaying a newer privacy journal
into an older populated semantic database cannot restore erased vectors or jobs.
Ordinary support expiry does not erase an independently retained claim, though
strict retrieval remains governed by its current support. Rule-blocked raw copies
remain excluded after a new source UUID or reindex, with permitted controls.

Extend the existing model-policy and recall panels: standing automatic embedding,
semantic readiness/progress/blocked reasons, admin reindex and explicit retry,
semantic-only/combined selection, and distances/qualifications with existing source
and claim inspection. Use readable labels, not internal generation IDs as primary
navigation. Show loading, disabled, missing/partial, mismatch, denied, budget,
provider and insufficient-support states. Preserve responsive desktop/mobile use.
Native `scope recall` accepts optional channel selection while retaining its original
operation scope; it generates one query-attempt ID per explicit invocation.

## Acceptance

- Real PostgreSQL/control-provider generation, batching, current input/model/dimension
  checks, policy denial, missing bytes, scope isolation and finite/nonzero vectors.
- Automatic catch-up/new input processing; lease/restart/lost-result behavior,
  bounded retry/capacity and an independent permitted batch after a failed one.
- Exact eligible cosine control with adversarial out-of-scope vectors, near matches,
  duplicate fragments and more than 100 ineligible high-scoring inputs; useful
  permitted results remain. Compare independent cosine math and all three channels.
- Rejection, copied raw assertions, reindex, expiry, in-flight erasure and older
  populated database/privacy replay preserve canonical behavior and no resurrection.
- A small declared engineering corpus with exact phrases, paraphrases, independent
  distractors and no-support questions; report recall hits, irrelevant returns,
  context budgets, latency and actual request/token cost per channel. Do not infer
  universal abstention from a single numeric threshold or semantic answer generation.
- Browser/native real API proof, normal-runtime preservation, a bounded real
  `text-embedding-3-large` run using synthetic content only, generated API/build,
  Clippy, focused tests and `./scripts/validate.sh`.

## Explicit Deferrals

ANN, dimension reduction, graph candidates, learned gates/reranking, query expansion,
host prompt injection, response caching and generated answers are outside this slice.
No customer content is newly transmitted in local proof. External deployment and
commits remain outside the goal. These exclusions do not defer current-consumer
scope, rejection, retention, erasure, provenance or recovery acceptance.


## Budget-blocked scheduling — 2026-10-07

A semantic batch blocked before transmission by `model_budget_exhausted` waits
until the next UTC day or a changed model policy before automatic rescheduling.
Do not recreate blocked batches every minute. Explicit authorized retry remains
available and still passes the gateway's current budget and authority checks.
