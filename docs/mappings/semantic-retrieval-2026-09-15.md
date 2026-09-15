# Semantic retrieval implementation and measured proof

Observed: 2026-09-15
Confidence: verified

## Method and boundary

The [semantic contract](../contracts/retrieval-semantic.md) and
[interface evidence](semantic-interfaces-2026-09-15.md) govern original slice 15.
Validation used the repository-owned PostgreSQL/pgvector installation, a controlled
HTTP provider, actual OpenAI embeddings, native recall and browser consumers.
The explicitly requested Atlas reviewer inspected source and assertions separately;
it did not run the tests. Both reference checkouts remained unchanged.

## Implementation and regression evidence

Migration 017 stores full `vector(3072)` projections with typed canonical parents,
Brain RLS, immutable profile generations, durable batch attempts and privacy fences.
Default-false standing embedding permission remains separate from capture and
learning. Canonical text resolution, model admission and publication recheck current
authority. Query vectors and text are not retained as a server result cache.

Seventeen semantic PostgreSQL/control-provider tests are included in the final
61-test platform pass (141.11 seconds; separately configured live OIDC excluded).
They exercise:

- Catch-up/new inputs, 20-input/8,000-byte batching, full-dimensional responses
  exceeding 1 MiB, exact chunk/artifact agreement and parent erasure dependencies.
- Separate bounded 5/30-minute confirmed retries, pre-admission budget resumption,
  interrupted paid attempts, lease recovery, partial retry and independent work.
- Independent cosine arithmetic, reversed provider indices, invalid model/shape,
  zero vectors, collection/Brain/operation isolation and current grant revocation.
- A 5,002-vector oversized-scope refusal before charging, with a useful narrow
  collection; recall continuing past 101 unreadable higher-scoring inputs.
- Canonical correction/raw-copy exclusion before/after reindex; a still-ready old
  vector excluded by current rules from historical strict recall but available in
  qualified history; independent accepted controls remain usable.
- In-flight erasure/policy/profile races, atomic publication across renewal and
  expiry, and a newer privacy journal replayed into an older populated database.
- Current-manifest repository revisions and three represented claims with one
  compatible selected claim; exact manifest compatibility precedes count/distance.
- A two-chunk source whose lexical and semantic matches differ: returned text and
  byte attribution follow the semantic score, while exact identity keeps priority.
- Required handover-contributor expiry during delayed publication, with an ordinary
  independently retained claim still publishable after its raw support expires.

All-target Clippy, eight native/protocol library tests, 117 generated API operations,
web type/build, CodeGraph sync and 32 governance tests passed. The browser recall
regression passed in 4.4 seconds against a live API/worker: policy invalidation and
erasure reset clear prior results without replaying a semantic POST; two explicit
submissions have distinct UUIDs. Its semantic transport response is controlled,
while its content/erasure operations use the real API. Actual-model UI proof follows.

The reviewer identified manifest prefiltering, mixed-chunk score attribution,
contributor TTL and causal historical-vector proof gaps in the initial semantic
implementation. Its bounded follow-up verified all four fixes and the corresponding
negative/positive assertions, with no remaining finding in that scope.

## Fixed real-model corpus

The exact seven documents and eight labeled questions are retained in
[the synthetic corpus](../../crates/server/tests/fixtures/semantic-corpus.json).
Documents describe Harbor rollout recovery, Cedar temporary database credentials,
Finch authenticated webhook intake, Quartz durable paid jobs, Orion telemetry,
coffee preparation and an interface palette. They contain no customer material.

All comparisons use the same eligible source versions, top-three limit, 4,096-byte
attributed context budget, `text-embedding-3-large`, 3,072 dimensions and default
minimum similarity zero. Six questions have an intended document; two deliberately
lack support. A hit requires that exact expected source version, not a shared word.
An irrelevant return means a source other than the declared expected document;
all returns on an unsupported question count as irrelevant. No answer is generated.

| Channel | Supported hits at 3 | Supported hits at 1 | Unsupported questions returning context | Irrelevant returned items | Mean latency | Query calls / recorded tokens | Largest context |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Exact + lexical | 2/6 | 2/6 | 0/2 | 0 | 21 ms | 0 / 0 | 1,341 bytes |
| Semantic | 6/6 | 6/6 | 2/2 | 18 | 393 ms | 8 / 84 | 3,491 bytes |
| Combined RRF | 6/6 | 6/6 | 2/2 | 18 | 363 ms | 8 / 84 | 3,511 bytes |

Exact title and quoted phrase matched in all modes. The four paraphrases matched
only with semantic participation. The two unsupported questions asked for a hosting
invoice and a contract approver; their strongest semantic matches were about 0.198
and 0.186. These are unrelated results, not supported answers. A threshold fitted to
this tiny corpus would not establish general abstention. These eight local timing
samples establish no universal quality, latency or fusion superiority.

## Actual runtime and browser proof

The owned API/worker restarted at 11:35:52 UTC with migration 017 and the matching
browser bundle. Readiness succeeded. The new
[Semantic engineering search demo](http://127.0.0.1:8787/brains/c34a8f58-047e-4d14-abca-d15626ab240b)
Brain alone enabled automatic embedding of documents and queries. Text learning and
autonomous memory are disabled for this fixed comparison corpus; existing Brains'
standing policies were not changed.

The real browser enabled indexing, observed all seven inputs blocked by a 1,000-token
daily allowance with zero provider calls, raised the allowance, and explicitly
started the replacement batch. All seven full-dimensional vectors became ready.
It ran combined recall, inspected canonical source bytes, rebuilt the index and
verified a second explicit query against the new generation. Rebuild invalidated
the prior displayed answer without resending its model request. Final readiness,
budget-error, source, desktop and 390-pixel mobile screenshots were inspected;
there was no horizontal overflow or browser exception.

At 11:38:54 UTC, all 20 actual requests succeeded: two seven-input corpus batches
(844 recorded tokens), sixteen comparison query embeddings (168), and two explicit
UI query embeddings (24), totaling 1,036 recorded tokens. These are gateway/provider
usage observations, not a billing invoice or a text-model capability comparison.
The complete proof at `.cache/semantic-live-proof/run.mjs` persists attempt identities
before transmission and refuses blind resumption of incomplete paid work. Completed
reruns perform read-only preservation checks. The state, comparison rows, usage and
screenshots remain in `.cache/semantic-live-proof/`.

All six pre-existing Brains retained their request and token totals. No customer
file was read or sent, and no new customer evidence was published. Separate read-only
normal-runtime proof before/after restart verified SWEG snapshot/manifest attribution,
the existing proposed claim's strict exclusion, autonomous 9090 replacement and
sanitized capture recall, with zero new model calls. The latter browser harness
needed an explicit scroll before opening a below-fold select; the repaired check
passed against both the old and new runtime. No production selector change was needed.

## Remaining owners

This closes original slice 15. Graph projections, linking, analytics and exploration
are 16–19; graph fusion/source-lineage measurements are 20; the combined investigation
UI is 21; host injection and MCP producers remain 25–26. Operational recovery and
broader quality/cost/load evaluation remain 27–29. No ANN, learned gate, generated
answer, external deployment or commit is claimed by this slice.
