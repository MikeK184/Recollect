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
`scripts/probe-openai.mjs` checks external connectivity; it does not enable
Brain enrichment or replace the provider-policy contract and retention dependency.
The [connection proof](../../mappings/openai-provider-preflight-2026-09-14.md)
records the initially usable GPT-4.1 mini/embedding-3-small pair. After completing
organization verification, the user selected `gpt-5.6-luna` as the text model;
that exact model passed structured extraction and is now the probe's default.
The user subsequently selected `text-embedding-3-large` for embeddings, verified
at its default 3,072 dimensions. Use Luna with that embedding model when implementing the
gateway. The delivered [provider-learning proof](../../mappings/provider-learning-proof-2026-09-14.md)
records actual browser calls and local runtime. The earlier GPT-5 mini verification denial is historical, not a current
blocker for the selected model.

Own retention/erasure rules and deletion replay obligations. Evidence capture,
retrieval, graph analytics and MCP adapters implement their respective consumers;
operations proves restore behavior. This epic does not wait for every consumer
to exist before delivering its canonical rules, and consumers cannot ship without
testing those rules through their own paths.

## Decisions Before Implementation

- Specify claim/evidence/time identities, validity precision, allowed mutations,
  expected-version checks and actor/reviewer authority.
- Specify assertion rejection/applicability and explicit revalidation, conflict
  dispositions and current/historical eligibility without a universal semantic
  matching or newest-wins assumption.
- Specify per-class retention, minimal audit/deletion metadata and restore
  enforcement beyond the settled 30-day raw default; define model adapters,
  policy rules and budgets before enabling automatic learning.

## Slice Map

The [automated memory improvement plan](../../research/automated-memory-improvement-plan-2026-10-07.md)
authorizes source-support verification, automatic session-derived handovers with
permitted minimal excerpts, and bounded cross-session correction. These slices
are locally delivered under accepted contracts and archived execution packs.
[Native proof, independent review and the versioned actual-model comparison](../../mappings/memory-source-support-staging-2026-10-07.md)
complete local acceptance. No running-installation deployment is claimed.

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `memory-automatic-session-digests` | shipped | contract-backed | pack | Settled-session supported digests and exact permitted durable excerpts with privacy/recovery |
| `memory-cross-session-reconciliation` | shipped | contract-backed | pack | Post-extraction exact-family replacement/reuse hypotheses with independent support and protected authority |
| `memory-source-support-verification` | shipped | contract-backed | pack | Whole-assertion assessment, durable staged learning and shared usability guards with automatic audit and privacy/recovery proofs |
| `memory-support-evaluation-baseline` | shipped | contract-backed | pack | Frozen synthetic corpus, native actual-model measurements, retained failures and independently reviewed receipt/scoring evidence |
| `brain-openrouter-provider` | shipped | contract-backed | pack | Locally validated per-Brain OpenRouter catalogue, strict-output gateway, Qwen embeddings and live public proof; normal-stack deployment not performed |
| `brain-openrouter-luna` | shipped | contract-backed | pack | Reviewed GPT-6 Luna option through OpenRouter after GLM timeout diagnostics; same gateway/replay fences, native support diagnostic; full answer campaign remains GLM/Qwen |
| `brain-openrouter-glm-recovery` | shipped | contract-backed | small-fix: fixed cheap GLM endpoint after measured routing failures and explicit rerun authorization | Pin GLM 5.3 to reviewed DeepInfra endpoint; preserve strict schemas, price ceilings, no fallback and original timeout |
| `brain-openrouter-glm-throughput` | shipped | contract-backed | small-fix: isolated fixed adapter routing under accepted selection contract | Fixed GLM throughput preference under USD 0.15/0.50 per-million price ceilings; HTTP fixture proves strict schemas/no fallback and existing timeouts; live failures remain benchmark evidence |
| `brain-model-catalogue-and-selection` | shipped | contract-backed | pack | Account model catalogue, dated prices, per-Brain supported model selection and isolated embedding rebuilds |
| `memory-claims-and-time` | shipped | adr-backed, contract-backed | pack | Delivered evidence-linked proposals/decisions, independent states, temporal history, selective freshness and canonical eligibility with API/browser/SWEG proof |
| `memory-review-and-corrections` | shipped | contract-backed | pack | Delivered actionable review/conflicts, rejected-value rules, correction, withdrawal/revalidation and replay with API/browser proof |
| `memory-retention-and-erasure` | shipped | contract-backed | pack | Delivered class deadlines, exact excerpts, dependency erasure, native cleanup and durable restore replay with API/browser/runtime proof |
| `memory-provider-policy-and-learning` | shipped | contract-backed | pack | Delivered Luna/embedding-large gateway, policy/usage, canonical source learning and erasure with API/browser/real-model/runtime proof |

| `memory-procedures-and-handovers` | shipped | adr-backed, contract-backed | pack | Delivered typed procedures, governed handovers and autonomous learning/revision/retirement/refresh with API/browser/real-Luna/runtime/erasure proof; human review is optional |
| `memory-capture-reconciliation` | shipped | contract-backed | pack | Delivered attributed model inputs, bound session reconciliation, maintenance at capacity, recall-feedback exclusions and independent capture/knowledge time with database/native/browser/real-Luna proof |
| `memory-desktop-workflows` | shipped | adr-backed, contract-backed | pack | Readable memory/handover inspectors and standing settings with literal assertion search and preserved autonomous authority |
| `memory-managed-experience` | shipped | contract-backed | pack | Managed autonomous defaults, simple Ask/notes and actionable owner MCP setup |

`brain-openrouter-glm-throughput` uses the small-fix exception: the accepted
selection contract and renewed user GLM/Qwen instruction establish authority;
this isolated adapter preference adds no storage, API/UI controls, fallback or
new lifecycle state. Official OpenRouter and Context7 documentation verify
throughput sorting and the per-million price ceiling; the existing HTTP fixture
proves the exact fields and unchanged fences before the separately frozen run.

## Slice Dependencies

| Slice ID | Predecessors |
| --- | --- |
| `memory-claims-and-time` | `evidence-repository-publication` |
| `memory-review-and-corrections` | `memory-claims-and-time` |
| `memory-retention-and-erasure` | `memory-review-and-corrections` |
| `memory-provider-policy-and-learning` | `memory-retention-and-erasure` |
| `memory-procedures-and-handovers` | `memory-review-and-corrections` |
| `memory-capture-reconciliation` | `memory-procedures-and-handovers`, `evidence-session-capture` |
| `memory-desktop-workflows` | `platform-desktop-shell` |
| `memory-managed-experience` | `platform-desktop-shell`, `memory-capture-reconciliation`, `mcp-desktop-setup` |

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

## Desktop implementation evidence, 2026-09-26

The [dated implementation mapping](../../mappings/desktop-experience-implementation-2026-09-26.md)
records implemented routes/assets and the verified final local image separately
from remaining current-code domain and integrated acceptance. The
[desktop guide](../../runbooks/desktop-experience.md) documents the current function
locations. Product/proof slices stay in progress until their required checks pass;
prior shipped domain records remain historical evidence rather than redesign proof.

## Managed experience follow-up, 2026-09-28

The [managed setup slice](../execution/archive/memory-managed-experience.md) is
delivered locally with native/browser proof and preserved existing Brain data.
The [dated mapping](../../mappings/managed-memory-experience-2026-09-28.md) records
the operating defaults, initial failures, completed reruns and limits. The epic
remains active for the separate desktop workflow and integrated acceptance scope.

## Final desktop and tooling acceptance — 2026-10-01

The owning desktop/tooling slices are shipped with [current deployed and host evidence](../../mappings/desktop-final-acceptance-2026-10-01.md). Earlier pending checks above describe the September 26 snapshot; their remaining acceptance is now complete. No release, commit or push was performed.

## Browser feedback closeout — 2026-10-05

The supported account model catalogue, dated prices, per-Brain model selection and dimension-safe embedding generation/rebuild successor has completed isolated backend and current UI acceptance. Actual metadata availability was refreshed once; no paid model call or real Brain rebuild was performed for screenshots.

[Coordinated delivery, validation and limits](../../mappings/desktop-browser-management-2026-10-05.md). Version N/A; work uncommitted, no push or external release.

The October 8 fixed GLM recovery endpoint passes the native HTTP fixture;
all 28 original failed LongMemEval cases now have successful answers after two
explicit recovery waves. Original throughput results and paid failure receipts
remain retained in the [dated benchmark report](../../mappings/openrouter-benchmark-proof-2026-10-08.md).
The fixed route preserves strict schemas, no fallback, price ceilings and the
45-second timeout. This is local implementation/proof; normal-stack deployment
was not performed.
