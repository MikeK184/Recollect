# Check the OpenAI provider connection

## Purpose and Prerequisites

Use the repository's Node 26 runtime and a usable `OPENAI_API_KEY` in the ignored
`.env` file to check model access before implementing or diagnosing model features.
This explicit probe makes two small billable API calls using only fixed synthetic
text. It does not read or submit Brain evidence or customer files.

## Procedure

Run from the Recollect root:

```sh
node --env-file=.env docs/research/scripts/probe-openai.mjs
```

It checks `text-embedding-3-large` and the text model hardcoded in the probe
(`gpt-5.6-luna`); it does not read `RECOLLECT_TEXT_MODEL`. The short
structured extraction uses `reasoning.effort:none`. Keep credentials solely in
the environment/ignored file. The output reports verification, returned model,
vector dimensions and token counts. It omits raw vectors and provider payloads.

## Verification

Both results must contain `verified: true` and the command must exit zero.
The embedding result must have 3,072 finite coordinates; the structured response
must extract the two expected synthetic fields and complete normally.
The [dated proof](https://github.com/MikeK184/Recollect/blob/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs/mappings/openai-provider-preflight-2026-09-14.md) records the
actual successful calls. These checks establish connectivity and interface shape,
not extraction quality or persistent semantic search. To verify a Brain through
the delivered gateway, use [Model learning → Check selected models](provider-learning.md).

## Failure and Recovery

Missing credentials fail before a network call. HTTP failures and 30-second
timeouts produce a nonzero result without printing provider error bodies. Check
account billing, model access and endpoint availability before retrying.

GPT-5 mini was initially listed but rejected execution with an organization-
verification requirement. After the user completed verification, the selected
GPT-5.6 Luna passed an actual call. If a future call requires verification, check the
[OpenAI organization settings](https://platform.openai.com/settings/organization/general)
and verify the selected model again. A listing alone is not execution proof.
Product model selection and customer-content access belong to the Brain provider
policy. This diagnostic does not change that policy or select fallback models.
