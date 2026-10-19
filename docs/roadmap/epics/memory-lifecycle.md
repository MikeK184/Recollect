# Memory Lifecycle, Learning and Review

Status: active

[Managed experience](../../contracts/memory-managed-experience.md) governs the 2026-09-28 agent-first setup correction.

## Purpose

Own how evidence becomes claims, decisions and procedures; how trust/time are
represented; and how review, correction, retention and erasure survive every
subsequent ingestion, retrieval and rebuild path.

## Governing Sources

- [Desktop contract](../../contracts/desktop-experience.md)
- [Desktop ADR](../../adr/0014-desktop-experience-and-answers.md)
- [Canonical claims and time](../../adr/0005-canonical-claims-and-time.md)
- [Claims contract](../../contracts/memory-claims-and-time.md)
- [Review and corrections](../../contracts/memory-review-and-corrections.md)
- [Retention and erasure](../../contracts/memory-retention-and-erasure.md)
- [Provider policy and learning](../../contracts/memory-provider-policy-and-learning.md)
- [Procedures and handovers](../../contracts/memory-procedures-and-handovers.md)
- [Autonomous memory decision](../../adr/0006-autonomous-memory.md)
- [Autonomous maintenance](../../contracts/memory-autonomous-maintenance.md)
- [Capture reconciliation](../../contracts/memory-capture-reconciliation.md)
- [Source support verification](../../contracts/memory-source-support-verification.md)
- [Automatic-memory evaluation baseline](../../contracts/operations-integrated-evaluations.md#automatic-memory-support-baseline-2026-10-07)
- [Vision: correction and learning](../../foundation/vision.md#correction-and-learning-loop)
- [Vision: capture, retention and model policy](../../foundation/vision.md#capture-retention-and-model-policy)
- [Vision: seven capabilities](../../foundation/vision.md#seven-required-capabilities)
- [Engineering principles: mutation policy](../../foundation/engineering-principles.md#route-every-mutation-through-one-policy)
- [Stack: authority and retrieval](../../foundation/techstack.md#storage-ownership-and-hybrid-retrieval)

## Dependencies and Boundaries

Consume versioned evidence and manifests through platform commands/jobs. Own
claim/support identity, independent review/freshness/operational states, fact
validity versus knowledge history, correction decisions and current eligibility.
Each writer and downstream projection must use this authority; no backend or
model may bypass it.

Own actionable evidence/claim review screens and APIs, including conflict
resolution, Withdraw and Erase. An accepted status is distinct from human
review, and human reviewer authority cannot be supplied by model arguments.
Runbooks and handovers are governed memory records with conditions/outcomes,
not executable permissions.

Own Brain provider/model/purpose/content policy and the shared model gateway
before any enrichment, embedding or external reranking. Named acceptance policies
activate supported knowledge without individual human acceptance. Models reconcile
evidence; unresolved material stays uncertain while processing continues. Human
review is an optional override. Deployment provider choices remain inputs.

Provider input, 2026-09-14: the user supplied `OPENAI_API_KEY` in Recollect's ignored
`.env` and authorized OpenAI for the required model operations. Use a separate
text model and embedding model behind the shared gateway. The opt-in synthetic
`docs/research/scripts/probe-openai.mjs` checks external connectivity; it does not enable
Brain enrichment or replace the provider-policy contract and retention dependency.
The [connection proof](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/mappings/openai-provider-preflight-2026-09-14.md)
records the initially usable GPT-4.1 mini/embedding-3-small pair. After completing
organization verification, the user selected `gpt-5.6-luna` as the text model;
that exact model passed structured extraction and is now the probe's default.
The user subsequently selected `text-embedding-3-large` for embeddings, verified
at its default 3,072 dimensions. Use Luna with that embedding model when implementing the
gateway. The delivered [provider-learning proof](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/mappings/provider-learning-proof-2026-09-14.md)
records actual browser calls and local runtime. The earlier GPT-5 mini verification denial is historical, not a current
blocker for the selected model.

Own retention/erasure rules and deletion replay obligations. Evidence capture,
retrieval, graph analytics and MCP adapters implement their respective consumers;
operations proves restore behavior. This epic does not wait for every consumer
to exist before delivering its canonical rules, and consumers cannot ship without
testing those rules through their own paths.

## Slice Map

The [automated memory improvement plan](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/research/automated-memory-improvement-plan-2026-10-07.md)
authorizes source-support verification, automatic session-derived handovers with
permitted minimal excerpts, and bounded cross-session correction. These slices
are locally delivered under accepted contracts and archived execution packs.
[Native proof, independent review and the versioned actual-model comparison](../../mappings/memory-source-support-staging-2026-10-07.md)
complete local acceptance. No running-installation deployment is claimed.

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `memory-automatic-session-digests` | shipped | contract-backed | archived | Settled-session supported digests and exact permitted durable excerpts with privacy/recovery |
| `memory-cross-session-reconciliation` | shipped | contract-backed | archived | Post-extraction exact-family replacement/reuse hypotheses with independent support and protected authority |
| `memory-source-support-verification` | shipped | contract-backed | archived | Whole-assertion assessment, durable staged learning and shared usability guards with automatic audit and privacy/recovery proofs |
| `memory-support-evaluation-baseline` | shipped | contract-backed | archived | Frozen synthetic corpus, native actual-model measurements, retained failures and independently reviewed receipt/scoring evidence |
| `brain-openrouter-provider` | shipped | contract-backed | archived | Locally validated per-Brain OpenRouter catalogue, strict-output gateway, Qwen embeddings and live public proof; normal-stack deployment not performed |
| `brain-openrouter-luna` | shipped | contract-backed | archived | Reviewed GPT-6 Luna option through OpenRouter after GLM timeout diagnostics; same gateway/replay fences, native support diagnostic; full answer campaign remains GLM/Qwen |
| `brain-openrouter-glm-recovery` | shipped | contract-backed | inline: fixed cheap GLM endpoint after measured routing failures and explicit rerun authorization | Pin GLM 5.3 to reviewed DeepInfra endpoint; preserve strict schemas, price ceilings, no fallback and original timeout |
| `brain-openrouter-glm-throughput` | shipped | contract-backed | inline: isolated fixed adapter routing under accepted selection contract | Fixed GLM throughput preference under USD 0.15/0.50 per-million price ceilings; HTTP fixture proves strict schemas/no fallback and existing timeouts; live failures remain benchmark evidence |
| `brain-model-catalogue-and-selection` | shipped | contract-backed | archived | Account model catalogue, dated prices, per-Brain supported model selection and isolated embedding rebuilds |
| `memory-claims-and-time` | shipped | adr-backed, contract-backed | archived | Delivered evidence-linked proposals/decisions, independent states, temporal history, selective freshness and canonical eligibility with API/browser/test-Brain proof |
| `memory-review-and-corrections` | shipped | contract-backed | archived | Delivered actionable review/conflicts, rejected-value rules, correction, withdrawal/revalidation and replay with API/browser proof |
| `memory-retention-and-erasure` | shipped | contract-backed | archived | Delivered class deadlines, exact excerpts, dependency erasure, native cleanup and durable restore replay with API/browser/runtime proof |
| `memory-provider-policy-and-learning` | shipped | contract-backed | archived | Delivered Luna/embedding-large gateway, policy/usage, canonical source learning and erasure with API/browser/real-model/runtime proof |

| `memory-procedures-and-handovers` | shipped | adr-backed, contract-backed | archived | Delivered typed procedures, governed handovers and autonomous learning/revision/retirement/refresh with API/browser/real-Luna/runtime/erasure proof; human review is optional |
| `memory-capture-reconciliation` | shipped | contract-backed | archived | Delivered attributed model inputs, bound session reconciliation, maintenance at capacity, recall-feedback exclusions and independent capture/knowledge time with database/native/browser/real-Luna proof |
| `memory-desktop-workflows` | shipped | adr-backed, contract-backed | archived | Readable memory/handover inspectors and standing settings with literal assertion search and preserved autonomous authority |
| `memory-managed-experience` | shipped | contract-backed | archived | Managed autonomous defaults, simple Ask/notes and actionable owner MCP setup |
| `models-explicit-text-model-selection` | shipped | contract-backed | inline: no code default text model; an installation without one keeps capture on and transmits nothing (model_not_selected) until a Brain admin chooses a listed model, and saved policies are untouched; platform, unit, installer and Playwright proof | Removed the built-in gpt-5.6-luna fallback: optional explicit RECOLLECT_TEXT_MODEL, Choose a model state in Ask and the AI card, preset re-applied by the chosen model, installer writes no default. Validated by two new platform tests (190 pass and 17 pre-existing failures across the 207-test models/managed/operations/brain/memory/retention/retrieval/review subset, identical failures before at 188 of 205), config, protocol and installer unit tests, Playwright model-selection and model-settings (3 pass, the 11 failures in five regression specs unchanged from main); no live provider call |
| `memory-support-gate-test-alignment` | shipped | contract-backed | inline: align platform tests with the accepted source-support gate, keep the bounded digest walk from starving later partitions and reconcile the recall fragment limit with its 2026-10-08 amendment; platform suite, library tests and validate.sh | Twelve stale tests now use support verdicts, human acceptance or history mode and the extra assessment calls; the digest walk records coverage for empty settled partitions so 101 sessions no longer starve later work (rotation test passes); the contract and runbook state the 4 KiB whole-chunk limit the code and answer revalidation rely on (a 2 KiB clip broke three answer tests). Platform suite on this code passed 314 of 319 (two SFTP cases passed separately with the fixture; the other failures are listed below and in the test-harness slice), clippy and library tests clean. Open: the corrections-recall test passes only when its 101 supported proposals fit the 2 s statement limit (support predicate costs about 20 ms per revision under RLS), which the retrieval support-predicate work owns; replaying a rejected learning value proposes a blocked duplicate that the contracts neither require nor forbid; capture_publication and organization_fields failures belong to the retrieval support-predecessor work |
| `memory-supersession-support-boundary` | shipped | contract-backed | archived | Learning basis edges to every offered input stay provenance, erasure and audit dependencies but leave the support, batch and deadline closure, while handover and digest contributions stay genuine dependencies (migration 071, classified by a handover parent revision); proved by the independent support_gate reference plus replacement, different-offered-input, erasure and handover-contributor cases (the new and changed cases fail against the previous same-claim-only edge filter), the capture publication case, a 147/159 platform subset with only the 12 baseline failures, fmt, Clippy and validate.sh |

`brain-openrouter-glm-throughput` is an inline slice (no pack): the accepted
selection contract and renewed user GLM/Qwen instruction establish authority;
this isolated adapter preference adds no storage, API/UI controls, fallback or
new lifecycle state. Official OpenRouter and Context7 documentation verify
throughput sorting and the per-million price ceiling; the existing HTTP fixture
proves the exact fields and unchanged fences before the separately frozen run.

## Completion Criteria

- Evidence, claim and synthesis views distinguish origin, review, freshness and
  operational proof; late evidence preserves distinct fact-time/knowledge-time answers.
- Actual review changes eligibility; stale reviews and forged human authority
  fail. Different valid environments/periods can coexist without newest-wins.
- Corrections/rejections survive alternate ingestion adapters, retries, restart
  and canonical replay while replacement and unrelated claims remain usable.
- Withdraw and Erase have distinct behavior. Retention and queued-work tests
  prevent re-entry, preserve allowed excerpts and identify independently supported
  records; downstream invalidation obligations include aggregate analytics.
- Unapproved model transmission/fallback is denied. Automatic acceptance records
  its policy instead of inventing human review. Procedures do not grant execution.

## Current status

Automatic learning, source-support verification, settled-session digests, exact-family cross-session reconciliation, review and erasure/recovery are implemented. This epic remains active as a dependency of organization privacy and incremental correction: ancestor model-input fences do not yet cover every descendant mapping input. The graph organization pack owns that unfinished integration; automatic learning itself is not a missing feature.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Shipped packs](https://github.com/MikeK184/Recollect/tree/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive) were removed from the tree on 2026-10-19 and remain in Git history.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/memory-lifecycle.md).
Dated test/deployment observations are not a current runtime health claim.
