# Reviewed Luna option through OpenRouter

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: select and use the reviewed `openai/gpt-6-luna` through the existing OpenRouter gateway after diagnosed GLM timeout failures.
- Non-goals: model/provider fallback, longer timeouts, routing knobs, local serving or normal-stack deployment.
- Delivery shape: additive catalogue option, supported reasoning `none`, focused adapter proof and native support diagnostic. The separate operations pack owns the full GLM/Qwen answer/judge benchmark after the renewed user instruction.

## Governing Sources

[Selection contract](../../../contracts/brain-model-catalogue-and-selection.md),
[provider policy](../../../contracts/memory-provider-policy-and-learning.md),
[budgeted evaluation protocol](../../../contracts/operations-longmemeval-protocol.md),
and [epic](../../epics/memory-lifecycle.md).

## Scope

- In scope: one exact reviewed model, dated reference prices, existing inline catalogue selection, strict adapter and native diagnostic.
- Out of scope: other models, automatic normal-Brain selection changes or a production quality guarantee.
- Blockers: none; the user delegated cheap model selection within the USD 10 campaign.

## Surface and Interface Changes

- Interfaces: additive existing catalogue entry; no new API fields.
- Storage: N/A; existing policy/provider/cost/replay receipts apply.
- Ownership: provider gateway/catalogue; operations owns the separately named benchmark.

## Data and Authority

- Inputs: public/synthetic preflight and unchanged support corpus; no gold labels reach the model.
- Authority: existing permissions, strict schemas, exact model identity and paid-attempt ledger.
- Blind spots: model quality is measured once; transport success does not imply safe automatic acceptance.

## States and Edge Cases

- Loading: explicit catalogue discovery before saving.
- Empty: missing credential is denied through existing gates.
- Error: wrong identity, malformed/incomplete output and provider errors fail closed.
- Blocked: original token/concurrency and 45-second limits remain.
- No-access: original Brain grants, admin/browser and CSRF gates apply.
- Duplicate or replay: existing paid identities cannot be resent.
- Stale data: fresh discovery and dated pricing qualifications remain required.
- Reconciliation divergence: changing only the text model preserves compatible Qwen generation; uncertain GLM attempts remain in their original reports.

## Integrations and Runtime Inputs

- Providers: fixed OpenRouter endpoint, exact Luna model with reasoning effort `none`, no fallback.
- Environment: existing `OPENROUTER_API_KEY`; no new runtime controls.
- Secrets: installation environment only; public/synthetic inputs only.
- Failure handling: original timeout/body/replay guards; no blind retry.

## Tests and Acceptance

- Automated: HTTP fixture verifies exact selection, supported reasoning wire shape, billed usage and replay; existing OpenAI proof and repository validation.
- Manual: live strict-JSON preflight and native held-out support diagnostic, with actual receipts and failures retained.
- Acceptance: the model can be selected and called through the governed gateway with a preserved Qwen generation; no existing Brain policy changes; diagnostic outcomes qualify recommendations.

## Closeout

- Planned: above.
- Shipped: exact reviewed catalogue option and reasoning `none`, fixture selection/wire/billing/replay and preserved Qwen profile, browser Cancel/text-only save/pricing proof, live native held-out diagnostic.
- Not shipped: normal-stack deployment or any Luna full answer benchmark; the user renewed GLM/Qwen for that separate operations run.
- New blockers: none requiring user input.
- Docs updated: selection/protocol, epic and active index.
- Validation: live preflight 1126 ms / USD 0.000011187; HTTP fixture and browser 1/1 passed; native held-out 26/26 completed, 84.6% verdict accuracy, 100% positive usability, zero critical false-usable, USD 0.00440597025. Runtime receipts independently reconciled. No production acceptance is inferred.
- Version: N/A; no release requested.
- Commit: uncommitted.
