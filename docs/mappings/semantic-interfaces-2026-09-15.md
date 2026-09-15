# Semantic retrieval interface evidence

Observed: 2026-09-15
Confidence: verified

## Sources and method

Context7 resolved `/pgvector/pgvector`; the targeted query verified exact cosine
distance, full-vector storage, index dimensions and filtering limits. Opened the
official [pgvector 0.8.6 README](https://github.com/pgvector/pgvector/blob/v0.8.6/README.md).
The existing PostgreSQL 17.10 installation has pgvector 0.8.6. Full `vector` values
support 16,000 dimensions; `vector` HNSW/IVFFlat indexes support 2,000 and halfvec
indexes 4,000. Exact `<=>` computes cosine distance. Use full 3,072-dimensional
storage and exact search; no extension upgrade, half precision or ANN is required.

Read Recollect's existing Rust gateway first. No direct official OpenAI search
connector was available, so official-domain search and page retrieval verified
[Create embeddings](https://developers.openai.com/api/reference/resources/embeddings/methods/create)
and the [embedding guide](https://developers.openai.com/api/docs/guides/embeddings).
`text-embedding-3-large` defaults to 3,072 output dimensions. The API accepts float
encoding and an explicit dimensions argument, returns model/index/usage metadata,
limits each input to 8,192 tokens and total request input to 300,000 tokens.
The application's smaller byte/count limits stay within those bounds. No pricing
or account-tier assumption is needed; actual local connectivity is separately tested.

## Existing seams and selected translation

The shared gateway already checks installed/Brain model identity, canonical classes,
usage reservations, current grants and post-call input/policy validity. It verifies
returned dimensions and finite numeric input; semantic publication also rejects
nonfinite f32 conversions and zero norms. Existing query probes do not establish a
stored semantic projection or usable search channel.

Source processing creates canonical 4,096-byte UTF-8 spans. Semantic chunk input
must verify those spans against artifacts and retain parent source dependencies
for model suppression/erasure. Current claim text must use canonical defaults and
an explicitly defined representation rather than embedding mutable `ClaimView`
assessment JSON. Exact/lexical candidate and item helpers already enforce scope,
raw rules, retained bytes, eligibility, context bounds and deadline checks.

Existing model jobs have one active slot, twenty-second renewable leases and
500 pending/running jobs per Brain. Query embedding must release recall's shared
Brain lock before entering the gateway's write transaction. A new durable semantic
batch owns its request once; a recorded response cannot be regenerated invisibly
after worker loss. Their implementation and acceptance are tracked in the
[shipped pack](../roadmap/execution/archive/retrieval-semantic.md).

Context7 `/websites/postgresql_17` confirmed permissive policy OR semantics and
the snapshot boundary of policy subqueries in PostgreSQL's
[row-security documentation](https://www.postgresql.org/docs/17/ddl-rowsecurity.html).
The new semantic tables use statement-scoped readable/writable Brain sets through
the existing RLS-protected Brain table. They do not cache access between requests
or bypass the current actor/device/grant checks and Brain locks. A 5,002-vector
adversarial fixture exposed excessive repeated per-row role resolution; indexed
canonical joins and these Brain sets let the oversized-scope refusal and a narrow
positive query pass with the existing two-second SQL bound. Full current-consumer
permission/regression checks remain part of slice acceptance.

Context7 `/tanstack/query`, the installed Query 5.102 source and the official
[disabled-query guide](https://tanstack.com/query/latest/docs/framework/react/guides/disabling-queries)
confirmed that `enabled: false` prevents automatic invalidation/refetch execution.
Recall uses a disabled observer and explicit submit calls; cache invalidation or
reset clears displayed context without reissuing a model query. A browser transport
fixture covers policy invalidation and erasure reset, with a new UUID only after a
second explicit submit. The attempted official `/query/latest/docs/reference/QueryCache`
page failed to load; Context7 and installed source verified cache subscription.

The interactive Browser runtime could not bootstrap: its trusted worker referenced
the missing `browser/26.901.51231/scripts/browser-service.mjs` module. Its current
skill was read and connection attempted. Repository-owned Playwright/Chrome tests
provide local browser proof; no interactive-browser connection is claimed.

## Boundary

Fifteen original slices plus the Atlas repair are delivered. This mapping verifies
external interfaces; the separate [implementation and runtime proof](semantic-retrieval-2026-09-15.md)
records controlled/database tests, real synthetic embeddings, measured channel
comparisons, browser/native proof and deletion/reindex acceptance. Neither interface
documentation nor a configured model alone establishes connectivity or usefulness.
