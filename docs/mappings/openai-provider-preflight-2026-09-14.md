# OpenAI provider connection preflight

Observed: 2026-09-14
Confidence: verified

## Sources and Method

The user supplied `OPENAI_API_KEY` in Recollect's ignored `.env` and authorized
using it for required model operations. Its value was neither printed nor copied.
Read the user's [Cognee provider guide](https://docs.cognee.ai/setup-configuration/llm-providers)
as reference: Cognee separates text extraction/reasoning from embeddings. This
does not reintroduce Cognee as Recollect's runtime dependency.

Queried Context7 `/websites/developers_openai_api` and fetched primary OpenAI
documentation for [structured outputs](https://developers.openai.com/api/docs/guides/structured-outputs),
[embeddings](https://developers.openai.com/api/docs/guides/embeddings),
[GPT-5 mini](https://developers.openai.com/api/docs/models/gpt-5-mini) and
[GPT-4.1 mini](https://developers.openai.com/api/docs/models/gpt-4.1-mini).
After the user completed organization verification and selected Luna, fetched
the [GPT-5.6 Luna reference](https://developers.openai.com/api/docs/models/gpt-5.6-luna)
and verified that exact model through the API.
The user then selected [text-embedding-3-large](https://developers.openai.com/api/docs/models/text-embedding-3-large)
for embeddings; its default dimensionality was verified with a real call.
Used Node 26 native fetch to make actual calls to `https://api.openai.com/v1`.
No SDK dependency, custom hash, snapshot pin or version gate was added.

## Observations

| Call | Actual result |
| --- | --- |
| `GET /models` | HTTP 200. Model listing included GPT-5 mini and both embedding-3 variants. Listing did not establish permission to execute each model. |
| `POST /embeddings`, `text-embedding-3-small` | Passed with one finite 1,536-dimensional vector and correct index; six input/total tokens for fixed synthetic text. |
| `POST /responses`, `gpt-5-mini` | HTTP 404, `model_not_found`. A second bounded diagnostic call established the provider's explicit reason: organization verification is required. No successful execution is claimed for this model. |
| `POST /responses`, `gpt-4.1-mini` | Passed with `store:false` and a JSON schema. Returned model `gpt-4.1-mini-2025-04-14`; completed response extracted exactly the expected service/environment from synthetic text. 67 total tokens. |
| `POST /responses`, `gpt-5.6-luna`, after user verification | Passed with `store:false`, `reasoning.effort:none` and the same JSON schema. Returned model `gpt-5.6-luna`; completed response extracted the expected synthetic fields. 74 total tokens. |
| `POST /embeddings`, user-selected `text-embedding-3-large` | Passed with one finite 3,072-dimensional vector and correct index; six input/total tokens. No dimensions reduction was requested. |

The user selected GPT-5.6 Luna as Recollect's text model. The
[opt-in probe](../../scripts/probe-openai.mjs) now uses the verified Luna and
embedding-3-large pair. It prints only model/status/shape/usage information,
never vectors, input files, credentials or provider error bodies. The final
successful pair was observed at `2026-09-14T10:11:58Z`; GPT-4.1 mini had passed
earlier at `10:01:20Z`. A model alias may return
a concrete revision, which is observed metadata rather than a strict pin.

Standard uncached prices observed in the model references, USD per million
input/output tokens: Luna `0.20/1.20`, GPT-4.1 mini `0.40/1.60`, GPT-5 mini
`0.25/2.00`. Luna's cached-input price is `0.02`, giving the user's full
input/cached-input/output tuple `0.20/0.02/1.20`. Embedding-3-large costs `0.13`
per million input tokens. Luna's stated prices apply through 272,000 input tokens; longer
prompts have higher rates. Actual cost also depends on token consumption.
The reference positions Luna in the cost-sensitive tier and confirms reasoning
and structured output support. This one synthetic extraction does not establish
superior quality to either other model on Recollect's actual workloads.

## Translation and Limits

This establishes working account credentials and two actual provider operations.
The probe sends only its fixed synthetic strings; SWEG files, Brain evidence and
other customer content were not transmitted. It is an explicit diagnostic, not
an automatic ingestion or enrichment path. The application server was not
restarted or given a new model gateway as part of this preflight.

At preflight, the [memory epic](../roadmap/epics/memory-lifecycle.md) still owned
the pending provider/model/purpose/content contract and gateway. The subsequent
[provider-learning proof](provider-learning-proof-2026-09-14.md) records delivery
of that gateway, real browser calls and normal local runtime. Semantic retrieval
still owns persistent embeddings and index behavior; this diagnostic alone does
not establish those capabilities.
The earlier GPT-5 mini rejection is historical evidence from before the user's
verification; the subsequently selected Luna now executes successfully. No
current blanket account restriction is inferred. See the
[preflight runbook](../runbooks/provider-preflight.md) before repeating calls.

Validation: actual synthetic provider calls passed; `./scripts/validate.sh`
passed all 32 checker tests. Version: N/A; commit: uncommitted.
