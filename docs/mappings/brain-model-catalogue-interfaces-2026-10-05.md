# Brain model catalogue interface evidence

Date: 2026-10-05
Scope: Primary documentation and local fixture proof for point 10 of the browser review.
Authority: [Accepted catalogue contract](../contracts/brain-model-catalogue-and-selection.md).

## Verified external interfaces

Official OpenAI pages were read on 2026-10-05. No installation-account API request,
paid probe or customer corpus transmission was performed during the original
interface investigation. During final implementation review, the CUA editor
refreshed actual account model-list metadata successfully; no paid inference or
customer corpus transmission was performed. [Current runtime evidence](desktop-browser-management-2026-10-05.md).

- [Model list](https://developers.openai.com/api/reference/resources/models/methods/list)
  provides account-visible IDs and basic metadata, without price or consumer
  compatibility. The server joins exact IDs to separate reviewed metadata.
- [GPT-5.6 Luna](https://developers.openai.com/api/docs/models/gpt-5.6-luna)
  supports Responses, structured output and reasoning effort none. Standard input,
  cached-input and output rates are $0.20/$0.02/$1.20 per million tokens.
- [GPT-4.1 Mini](https://developers.openai.com/api/docs/models/gpt-4.1-mini)
  supports Responses and structured output without a reasoning step. Rates are
  $0.40/$0.10/$1.60; its documented snapshot is `gpt-4.1-mini-2025-04-14`.
- [GPT-4.1](https://developers.openai.com/api/docs/models/gpt-4.1)
  supports the same bounded text consumer shapes. Rates are $2/$0.50/$8; its
  documented snapshot is `gpt-4.1-2025-04-14`.
- [GPT-4.1 nano](https://developers.openai.com/api/docs/models/gpt-4.1-nano)
  is marked deprecated and is excluded from offered choices.
- [Embedding large](https://developers.openai.com/api/docs/models/text-embedding-3-large)
  and [small](https://developers.openai.com/api/docs/models/text-embedding-3-small)
  cost $0.13 and $0.02 per million input tokens. The
  [embedding guide](https://developers.openai.com/api/docs/guides/embeddings)
  confirms default dimensions 3,072/1,536 and the server-side `dimensions`
  parameter for smaller compatible representations.

These prices are explanatory standard short-context metadata checked on that date,
not account-specific quotes. Cached, batch, regional and long-context terms remain
separate; entries older than thirty days expose stale pricing provenance.

Context7 resolve and query for `/pgvector/pgvector` succeeded. Its primary
[README](https://github.com/pgvector/pgvector#can-i-store-vectors-with-different-dimensions-in-the-same-column)
documents unconstrained `vector` columns for mixed dimensions and same-dimension
filtering before distance operations. Its API documents `vector_dims`. Recollect
retains exact bounded cosine queries and immutable model/profile filtering; it
adds no ANN index or inferred account model capabilities.

## Local implementation and proof

Implementation and focused results are recorded in the owning
[execution pack](../roadmap/execution/archive/brain-model-catalogue-and-selection.md).
The provider fixtures contain synthetic text and synthetic credentials only.
Installation credential presence, account availability and a successful model
call remain distinct evidence states. Runtime stack/screenshot proof is coordinated
with the broader desktop-management slice; this mapping alone is not deployment proof.
