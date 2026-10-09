# OpenRouter memory benchmark report — 2026-10-08

Observed: 2026-10-08
Confidence: observed-once
Delivery: local implementation and owned proof runtimes; uncommitted.

## Sources and Method

The authorized work added selectable OpenRouter models per Brain and evaluated
cheap GLM answers with Qwen embeddings through Recollect's actual product paths.
The campaign used the USD 10 key with a USD 8 reservation ceiling. Normal Brains
were excluded. Only public benchmark histories and synthetic fixtures were sent.
The base commit is `27bbd00f006e9684b25f18b47abcfc3110f22806`.

Primary references are [OpenRouter embeddings](https://openrouter.ai/docs/api/api-reference/embeddings/submit-an-embedding-request),
[provider routing](https://openrouter.ai/docs/guides/routing/provider-selection),
[GLM 5.3 Flash](https://openrouter.ai/z-ai/glm-5.3-flash),
[Qwen3-Embedding-8B](https://huggingface.co/Qwen/Qwen3-Embedding-8B), and the
[pinned official LongMemEval source](https://github.com/xiaowu0162/LongMemEval/tree/9e0b455f4ef0e2ab8f2e582289761153549043fc).
The local Atlas reference's benchmark guidance separates conversational QA from
correction/deletion tests; both kinds were exercised here. Reference checkouts
were not edited. The accepted
[run protocol](../contracts/operations-longmemeval-protocol.md) includes the user's
explicit authorization to rerun known failures and retain original results.

## Observations

### Result and practical assessment

**All 50 frozen LongMemEval questions now have completed answers. The official
judge marks 20/50 correct (40%).** Its six-task macro accuracy is **36.71%** and
abstention accuracy is **8/8 (100%)**. Repeating the first 25 judgments produced
**zero disagreements**. This repeat audit measures judge consistency, not
independent agreement with a human reviewer.

Cheap cloud calls were sufficient to expose real issues. Qwen retrieval achieved
99% supporting-document recall on the small HotpotQA sample, equal to Cognee's
vector lane. Recollect's lifecycle matrix had zero failures in its declared
coverage. Long conversation answering was substantially weaker: only **12/42
answerable questions** were correct, with particularly poor preference,
time-reasoning and knowledge-update results. Fixing provider availability did
not resolve these memory/context and answer-quality gaps.

### What was implemented

The backend and inline AI settings select a provider and reviewed models per
Brain. Explicit catalogue discovery and Save and rebuild preserve the existing
embedding-profile and in-flight request fences. OpenRouter text uses strict JSON
schema; Qwen uses 1,024 dimensions and its engineering retrieval query instruction.
Provider identity, returned model identity, tokens, known billed USD and replay
identities are retained. Older OpenAI receipts remain readable. Automatic memory
adoption preserves the selected provider. Keys stay in the environment.

GLM 5.3 Flash with low reasoning effort is the measured initial OpenRouter choice;
GLM 4.7 with optional reasoning disabled remains selectable. The final GLM 5.3
route is explicitly `deepinfra/fp4`, with required parameters, no fallback and
USD 0.15 input / 0.50 output per-million-token ceilings. The saved October 8
endpoint response quotes USD 0.075 / 0.25 for that endpoint. The 45-second native
gateway timeout was preserved. This route is inexpensive but still produced
rate limits during the first recovery wave.

Muse Contributor's preflight returned HTTP 403 requiring account age confirmation;
there is no successful Muse quality measurement. Its training-use disclosure is
shown in settings. The earlier additive Luna support diagnostic is reported
below; the LongMemEval recovery stayed on GLM/Qwen. No batch adapter, home-GPU
serving connection or normal-stack deployment was performed.

### LongMemEval: original run and explicit recovery

The deterministic LongMemEval-S sample contains all six task types and eight
unanswerable questions. Each question has its own Brain and complete historical
conversation set. History turns and conversation dates were ingested; gold
answers, questions and supporting-session labels were excluded from memory.

Ask used exact, lexical and semantic channels, limit 10, a 16,384-byte context
budget, source diversity, and `brain-answer-1`. Automatic extraction and learning
were disabled. Native policy allowed only embedding/answering, 32,768 input bytes,
4,096 output tokens, a two-million daily token ceiling and concurrency two.
Judging used exact dated `gpt-4o-2024-08-06` through OpenRouter. The official
upstream evaluator, prompts, yes-substring grading and metrics printer were
hash-pinned and unmodified.

The original complete measurement is experiment C, using throughput-ranked GLM
routes. It retains all 50 measured cases, including failures. Recovery retained
successful answers, reused independently verified compatible indexes, and
issued new identities only for known failed cases. The final result combines
22 original C answers, 22 answers from recovery wave one and six from wave two.
It is **a combined recovery result**, not a fresh single-pass availability score,
a controlled model comparison or evidence that every future request succeeds.

| Metric | Original C | Final combined recovery |
| --- | ---: | ---: |
| Available answers | 22/50 (44%) | 50/50 (100%) |
| Official overall accuracy | 12/50 (24%) | 20/50 (40%) |
| Official task-averaged accuracy | 20.39% | 36.71% |
| Official abstention accuracy | 7/8 (87.5%) | 8/8 (100%) |
| Repeat-judge disagreements | 0/25 | 0/25 |
| Completed-answer median / p95 Ask latency | 6.21 / 9.68 seconds (22 samples) | 9.73 / 18.94 seconds (50 samples) |
| Supporting-session recall, answerable retained responses | 92.11% (19 samples) | 89.09% (42 samples) |

| Official task type | Questions | C correct | Recovery correct | Recovery accuracy |
| --- | ---: | ---: | ---: | ---: |
| single-session-user | 13 | 6 | 8 | 61.54% |
| single-session-preference | 7 | 0 | 1 | 14.29% |
| single-session-assistant | 7 | 2 | 4 | 57.14% |
| multi-session | 9 | 3 | 4 | 44.44% |
| temporal-reasoning | 7 | 0 | 1 | 14.29% |
| knowledge-update | 7 | 1 | 2 | 28.57% |

Median delivered answering input across all 50 successful origin receipts: **4,998.5 tokens** (context plus instructions).

C's 28 failures were 27 GLM rate-limit responses and one history-admission
rejection. Its 49 attempted Ask calls produced 22 answers. After the admission
fix and explicit endpoint revision, recovery wave one retried the 28 failed
cases: 22 succeeded and six were rate-limited. Wave two retried those six only:
all six succeeded. No successful answer was reissued. Failed attempts, costs
and unavailable billing remain in their original reports.

The official C judge credited **four empty failed hypotheses as correct
abstentions** (`0862e8bf_abs`, `19b5f2b3_abs`, `29f2956b_abs`, `88432d0a_abs`).
Those official labels are preserved. Only eight of C's 22 available answers
were judged correct: 36.36% conditional on availability, or 8/50 usable correct
answers. Empty transport failures are not successful abstention behavior. The
final recovery has no failed hypotheses and all eight abstentions are actual
completed answers.

### Retrieval, lifecycle and comparator results

| Test | Actual result | Judge | Known billed USD |
| --- | --- | --- | ---: |
| HotpotQA lexical, 50 questions / 491 documents | 5% supporting-document recall @10; 1/50 complete support sets; 46 empty results | None | 0 |
| HotpotQA lexical + Qwen | 99% recall @10; 49/50 complete support sets; zero empty results | None | 0.00082907 |
| Atlas lifecycle API matrix | 36 pass, one `none` raw-evidence-refeed cell, three N/A, zero failures | None | 0 |
| Cognee 1.5.4 native vector lane, same HotpotQA corpus/Qwen | 99% recall; 49/50 complete support sets | None | 0.00075419 |
| Cognee full graph attempt | Stopped on an uncertain provider timeout; no full score | No completed grading | 0.00128837 known; unresolved bill retained |

HotpotQA's corpus SHA-256 is
`32dd92947d6c50194bc2e76588bc78ad6ad08805a4b05728336ac70cd7e19b96`.
The incomplete support set concerns Ralph Hefferline's university: Columbia
University was missed. This is retrieval scoring, without generated answers or
an LLM judge, and is not a Qwen-versus-OpenAI embedding comparison.

HotpotQA lexical median/p95 latency was 246/389 ms; Recollect Qwen was
2,920/14,031 ms; Cognee vector was 458/3,080 ms. Recollect used 97 successful
model requests; Cognee used 112. Cognee represented a whole title/passage per
native DataPoint and LanceDB vector; Recollect used governed chunk projections,
canonical eligibility checks and its API. These differences prevent a full-agent
performance ranking. Cognee required explicit initial collection creation to
avoid an observed nested creation lock, plus a transport shim for the configured
1,024 dimensions omitted by its embedding engine. Its research checkout was
unchanged; isolated runtime files are under `.cache/cognee-bench/`.

The lifecycle matrix exercises declared-memory correction/deletion through the
product API. It does not prove every generated-learning path, propagated copy,
external cache or handover deletion case. The one `none` and three N/A cells are
not counted as passes.

### Native memory-support diagnostic

These are actual canonical claim/support-worker runs on a frozen synthetic
50-case corpus with local gold verdicts. They require **no LLM judge** and are
separate from LongMemEval. Corpus SHA-256:
`4aafb2163cbe75777f60e3edecd1d9965e6e19855fb0595cde904c7f935337fd`.

| Model / sample | Completed | Verdict accuracy | Critical false-usable | Known billed USD |
| --- | ---: | ---: | ---: | ---: |
| GLM 4.7, 24 calibration cases | 24/24 | 91.7% | 0 | Included in 50-case total below |
| GLM 4.7, 26 held-out cases | 26/26 | 80.8% | 3 | 0.00491644 for all 50 |
| GLM 5.3 low, same 26 held-out cases | 25/26 | 84.6%, including failure | 0 | 0.00497662 |
| Earlier Luna, same 26 held-out cases | 26/26 | 84.6% | 0 | 0.00440597 |

Positive usability was 100% in each measured sample. GLM 4.7 nevertheless failed
the safety threshold: it admitted a conditional restart rule as an observed
certificate match, an imported instruction as deployment proof, and quickstart
omission as proof a cache was removed. It should not be recommended for
unattended memory acceptance from this evidence. GLM 5.3 failed the complete-run
threshold because one HTTP 429 call was unavailable. Zero observed critical
errors in a small sample is not a production safety guarantee. The earlier Luna
diagnostic completed before the renewed GLM/Qwen instruction; no Luna answering
benchmark or further Luna calls were used for this recovery.

### What went wrong and what was fixed

| Finding | Evidence / effect | Status |
| --- | --- | --- |
| Dynamic catalogue aliases rejected discovery | `~latest` entries invalidated reviewed model discovery | Fixed; adapter fixture passes |
| Qwen returned alias failed publication fence | Exact documented uppercase returned ID was rejected after a paid embedding call | Fixed; alias and durable publication fixture passes; discarded attempts retained |
| TypeScript property type mistaken for a credential | `apiKey: string;` inside a public fenced example rejected history `71017276` before Ask | Fixed with narrow fenced-type handling; literal credentials/configured secrets stay excluded; unchanged 49-session history now imports and answers |
| GLM routing availability | C had 27 HTTP 429 failures; first endpoint-pinned recovery had another six | All known failed questions recovered; first-pass reliability remains poor |
| Earlier provider timeouts | A stopped on a 45-second GLM 5.3 timeout; B stopped on a 45-second GLM 4.7 timeout; Cognee graph stopped at 60 seconds | Uncertain receipts preserved; no blind replay or full score claimed |
| Earlier ingestion lock timeout | B's import timed out waiting for a PostgreSQL advisory lock; committed sessions reconciled before resume | Recovered without re-answering successes; root cause under concurrent workload remains unproved |
| Earlier invalid answer citations | B question nine returned a charged `answer_citations_invalid` failure | Failure retained; not silently repaired or scored as success |
| Judge transport rejected valid short completion | Official evaluator sets a 10-token output cap; proxy incorrectly rejected `finish_reason=length` | Proxy corrected for dated judge; completed grades retained; bounded known-429 recovery; evaluator unchanged |
| Reporting/recovery integrity | Resume could overwrite elapsed observations; unknown charges and older paid attempts needed explicit retention | Original timings preserved or marked unavailable; unique receipts and recovery origins audited; uncertain/unknown-bill fixtures retained |
| Long-conversation context delivery | Correct conversation often retrieved while needed span is absent from model context | Unresolved; priority quality issue |
| Over-abstention / personalization | Some supplied facts are dismissed as unreviewed; preference questions often receive evidence commentary | Unresolved; answer-policy/prompt behavior needs a separate measured improvement |
| Temporal and update reasoning | Missing second event, older location/hours, incomplete counts | Unresolved; evidence selection and date/update handling need focused tests |

The final 50 calls all report fragment truncation and context-budget coverage;
23 also report candidate limits. **23 of the 30 incorrect answers have perfect
supporting-session recall.** Session recall alone therefore overstates useful
answer evidence. The current recall item path clips text at 2,048 bytes in
`crates/server/src/retrieval.rs`; source-identity ranking/context selection can
deliver a relevant conversation without the answer-bearing part. This is an
observed mechanism and a strong follow-up target, not proof that clipping alone
caused every miss.

Concrete examples from retained responses:

- `945e3d21`: the retrieved yoga fragment ends mid-frequency phrase. The answer
  abstains; the gold says three times weekly. Supporting-session recall is 1.0.
- `830ce83f`: the response gives an older city instead of the later suburb move.
  `71315a70` similarly gives earlier sculpture hours. Both have session recall 1.0.
- `0a995998`: two clothing items are counted where the gold requires three,
  despite all required sessions appearing in the context identities.
- `gpt4_f49edff3`: only one of three supporting sessions is retrieved, so the
  three-event order cannot be reconstructed from the supplied bundle.
- `6ade9755`: the supplied fragment explicitly names the yoga studio, but the
  answer treats unreviewed status as a reason to withhold confirmation. The
  response also unexpectedly switches to Chinese. This shows answer behavior
  can fail even when the needed name reaches context.
- `8a2466db`: the answer discusses historical Premiere Pro settings and unrelated
  retrieved material rather than satisfying the requested personalized resource
  recommendation. The official preference rubric marks it incorrect.

These observations support improving evidence-span delivery and historical
attribution before buying a larger answer model. They do not establish that
Qwen itself is the sole cause. No retrieval/prompt tuning was performed against
these gold answers during the frozen run.

### Spend and unresolved billing

| Campaign component | Known receipt cost, USD |
| --- | ---: |
| All LongMemEval embedding and answering, including A/B/C and recovery | 0.09136438 |
| All official judging, both 50+25 runs, preflight and failed charged attempt | 0.10916978 |
| Three native support diagnostics | 0.01429903 |
| HotpotQA Qwen | 0.00082907 |
| Both Cognee attempts | 0.00204256 |
| Remaining preflights and discarded Qwen attempts | 0.00012568 |
| **Total known receipt cost** | **0.21783050** |

Unknown costs are not recorded as zero. Forty attempts lack a reported bill;
USD 4.07071 remains conservatively reserved, including earlier uncertain calls,
rate limits, Muse's failed preflight and the failed judge attempt. This reserve
is headroom, not a claim that USD 4.07 was charged. Known receipt cost plus the
reserve is USD 4.28854, below the USD 8 ceiling and USD 10 key limit.

At the saved 2026-10-08 16:24:28 UTC account observation, the key reported
USD 0.144978114 consumed and USD 9.855021886 remaining. This is lower than the
USD 0.21783050088093991 sum of retained reported costs. The difference remains
unreconciled; the account counter was still increasing after calls completed.
A read-only generation audit confirms an older judge receipt's cost against
OpenRouter's generation API; the newest generation was not yet available (404).
This is consistent with delayed accounting, but does not prove the cause of the
whole difference. Neither value is presented as a final invoice. The campaign
gate conservatively checks the larger of known costs and key usage plus all
unresolved reservations. Five thousand five hundred fifteen identified paid
attempts are unique across all runs; locally rejected proxy calls and preserved
DB copies add no paid attempt.

### Validation and direct runtime checks

Direct read-only proof verifies **2,402 conversation sources**, exact retained
bytes, titles and original observation timestamps against the frozen history
turns. All 50 Brains have complete compatible, untruncated Qwen indexes. There
are zero active native model requests. Every final answer traces through copied
rows to its original successful gateway receipt; answer operation IDs and
receipts are distinct. The original C proof remains preserved as a historical
snapshot rather than being rewritten against the recovered database.

The focused OpenRouter route fixture, TypeScript credential-regression test,
OpenAI compatibility/concurrency/failure checks, selector browser proof and
frontend type checks pass. The current non-ignored workspace suite passes
**76 tests**, with **229 explicitly ignored** integration cases. The wider
platform suite previously passed 84/96; an unchanged HEAD archive passed 83/95
and reproduced the same 12 failures. Those existing failures remain unresolved;
this report does not describe that broader suite as green. Required repository
validation and final diff checks are recorded in the closeout evidence.

## Translation and Limits

The original and recovery 50-question scores, all three official metric forms,
per-type denominators, raw judgments and failure receipts remain separate.
Combined recovery yields 100% observed answer availability after retries; it
is not a measured first-pass service-level guarantee. Changing the GLM endpoint
and fixing admission during recovery makes this a useful diagnostic, not an
unconfounded experiment establishing the route's causal effect on accuracy.

The sample is small and deterministic, not the complete official LongMemEval
suite. LongMemEval here tests governed retrieval plus answer generation; learning,
extraction and memory consolidation were disabled. It cannot establish how well
those automatic memory features work. HotpotQA's 491-document distractor pool
is not full-Wikipedia retrieval. No million-document scaling claim, OpenAI-embedder
head-to-head, batch-discount proof or complete Cognee agent comparison is made.

The final answer median/p95 covers the retained successful Ask operations across
both routes, excludes ingestion and prior failed attempts, and does not include
whole recovery wall time. Original C's 17 cold history preparations had a median
8.52 minutes and p95 11.80 minutes; this spans creation/import/processing/index
readiness until query admission, not pure embedding speed. Reused histories are
excluded. The separately summarized final wave's 12 incremental receipts are
not the total cost or delivered-token sample for all 50 retained answers.

Version: N/A. Commit/push: not performed. Implementation and proof are local;
the normal Recollect installation was not upgraded. Raw reports/artifacts are
in ignored `.cache/` and available in this workspace; the dated summary is a
tracked Markdown deliverable. Owned fixtures with unresolved receipts are
retained for reconciliation.

## Follow-up

1. Test turn-aware evidence spans and bounded neighboring/alternate fragments,
   measuring answer-bearing content as well as session recall. Preserve canonical
   eligibility and the context budget; rerun a frozen held-out set for comparison.
2. Test historical attribution without treating every unreviewed conversation
   as unusable, plus personalized recommendations and response-language consistency.
3. Test latest-state selection, multi-session counting and relative-time math
   with explicit old/new/date fixtures and local gold scoring.
4. Keep support-safety false admissions in regression coverage; measure a larger
   held-out support sample before enabling a cheap model for automatic acceptance.
5. Reconcile delayed/missing billing from retained receipts before deleting owned
   fixtures. Broader baseline test failures and normal-stack rollout remain separate.

## Reproduction and retained evidence

Data revision: `98d7416c24c778c2fee6e6f3006e7a073259d48f`.
Upstream commit: `9e0b455f4ef0e2ab8f2e582289761153549043fc`.

| Frozen file | SHA-256 |
| --- | --- |
| `longmemeval_s_cleaned.json` | `d6f21ea9d60a0d56f34a05b609c79c88a451d2ae03597821ea3d5a9678c3a442` |
| `subset-50.json` | `ae3c70ac7e8410ec86008066211dd975cb379ee87148d98089d0c97949aa633c` |
| `evaluate_qa.py` | `ecce9c4c79dc89d99534ac17b383a5cbb5b9f0c69ee98adaf0684742e3d95251` |
| `print_qa_metrics.py` | `e9283933a0cefb7a0ded7365e436ae3d1be5aac41853325e6155d83bf07607f0` |

The fetcher is `scripts/fetch-longmemeval.py`; the opt-in native harness is
`web/tests/longmemeval.spec.ts` through `scripts/test-ui.sh`. Paid dispatch requires
explicit opt-in and an owned database. `scripts/judge-longmemeval.py` runs the
unchanged evaluator through `scripts/benchmark-openrouter-proxy.mjs`.
The following commands regenerate summaries **offline**, without provider calls:

```sh
python3 scripts/summarize-longmemeval.py .cache/longmemeval-glm53-throughput
python3 scripts/summarize-longmemeval.py .cache/longmemeval-glm53-recovery-wave2
python3 scripts/summarize-openrouter-campaign.py --recovered-run .cache/longmemeval-glm53-recovery-wave2
```

The LongMemEval directories each retain `report.json`, original responses,
request ledgers and frozen configuration. Original C and final recovery also
retain `summary.json`, 50 primary raw judgments, 25 audit raw judgments, official
metrics text and `judge-audit.json`. Their `ingestion-readiness-proof.json` files
are direct database/source checks. `longmemeval-glm53-recovery` retains the
intermediate six rate-limit failures. Proxy ledgers and durable public success
responses are under `.cache/longmemeval-judge-proxy*/`.

Additional evidence:

- Campaign cost sheet: `.cache/openrouter-campaign-accounting-final.json`;
  account snapshot and generation audit: `.cache/openrouter-account-snapshot.json`
  and `.cache/openrouter-generation-cost-audit.json`.
- HotpotQA: `.cache/public-benchmark-4a06d4ee-1108-4288-aec0-42c165ee66dc/`
  and `.cache/public-benchmark-6e5def27-89fe-4548-8868-514a100bb529/`.
- Lifecycle: `.cache/atlas-lifecycle-70f17709-9e39-40c1-9921-965cd1416207/`.
- Support: `.cache/openrouter-support/`, `.cache/openrouter-support-glm53/`,
  `.cache/openrouter-support-gpt6-luna/`.
- Cognee: `.cache/cognee-bench/{report.json,proxy/,vector/,vector-proxy/}`.
- Original stopped A: `.cache/longmemeval-1fdefad9-748f-4c5a-8f4f-7d9b1406445f/`;
  B: `.cache/longmemeval-glm47-no-reasoning/`; restore-verified owned snapshot:
  `.cache/longmemeval-owned-state-6a27376a547a4e85a26ace91f6021fd7/`.
- Current checks: `.cache/openrouter-typescript-admission-test.log`,
  `.cache/openrouter-fixture-glm-recovery.log`,
  `.cache/openrouter-workspace-tests-recovery.log`,
  `.cache/openrouter-codegraph-final.log`, `.cache/openrouter-validation-final.log`
  and the read-only recovery audit script `.cache/verify-longmemeval-recovery.py`.

## All 50 final question outcomes

`C` means the original measured run; `R1` and `R2` mean successful answers from
the first and second explicit recovery waves. Every row has a completed native
answer; “incorrect” is the official judge label, not a transport failure.
Supporting-session recall does not prove the necessary answer span was delivered.

| Question ID | Task type | Official result | Answer origin | Session recall |
| --- | --- | --- | --- | ---: |
| `6a1eabeb` | knowledge-update | correct | R2 | 1 |
| `0a995998` | multi-session | incorrect | C | 1 |
| `7161e7e2` | single-session-assistant | incorrect | C | 1 |
| `8a2466db` | single-session-preference | incorrect | R1 | 1 |
| `e47becba` | single-session-user | incorrect | C | 1 |
| `gpt4_59149c77` | temporal-reasoning | incorrect | R1 | 1 |
| `0862e8bf_abs` | single-session-user | correct | R1 | 1 |
| `6aeb4375` | knowledge-update | incorrect | R1 | 1 |
| `830ce83f` | knowledge-update | incorrect | R1 | 1 |
| `852ce960` | knowledge-update | correct | C | 1 |
| `945e3d21` | knowledge-update | incorrect | R1 | 1 |
| `d7c942c3` | knowledge-update | incorrect | C | 1 |
| `71315a70` | knowledge-update | incorrect | C | 1 |
| `6d550036` | multi-session | incorrect | C | 0.5 |
| `gpt4_59c863d7` | multi-session | incorrect | R1 | 0.75 |
| `b5ef892d` | multi-session | correct | C | 1 |
| `e831120c` | multi-session | correct | R1 | 1 |
| `3a704032` | multi-session | incorrect | C | 0.6666666666666666 |
| `gpt4_d84a3211` | multi-session | incorrect | C | 1 |
| `c4f10528` | single-session-assistant | incorrect | R1 | 1 |
| `89527b6b` | single-session-assistant | correct | C | 1 |
| `e9327a54` | single-session-assistant | incorrect | R1 | 1 |
| `4c36ccef` | single-session-assistant | correct | R2 | 1 |
| `6ae235be` | single-session-assistant | correct | C | 1 |
| `7e00a6cb` | single-session-assistant | correct | R2 | 1 |
| `06878be2` | single-session-preference | incorrect | C | 1 |
| `75832dbd` | single-session-preference | incorrect | R1 | 0 |
| `0edc2aef` | single-session-preference | incorrect | C | 1 |
| `35a27287` | single-session-preference | incorrect | R1 | 1 |
| `32260d93` | single-session-preference | incorrect | R1 | 0 |
| `195a1a1b` | single-session-preference | correct | R1 | 1 |
| `118b2229` | single-session-user | correct | C | 1 |
| `51a45a95` | single-session-user | incorrect | R2 | 1 |
| `58bf7951` | single-session-user | incorrect | C | 1 |
| `1e043500` | single-session-user | incorrect | C | 1 |
| `c5e8278d` | single-session-user | correct | R1 | 1 |
| `6ade9755` | single-session-user | incorrect | C | 1 |
| `gpt4_f49edff3` | temporal-reasoning | incorrect | C | 0.3333333333333333 |
| `71017276` | temporal-reasoning | incorrect | R1 | 1 |
| `b46e15ed` | temporal-reasoning | incorrect | R1 | 0.5 |
| `gpt4_fa19884c` | temporal-reasoning | incorrect | R1 | 1 |
| `0bc8ad92` | temporal-reasoning | correct | R1 | 0.6666666666666666 |
| `af082822` | temporal-reasoning | incorrect | R2 | 1 |
| `15745da0_abs` | single-session-user | correct | R1 | 1 |
| `bc8a6e93_abs` | single-session-user | correct | C | 0 |
| `19b5f2b3_abs` | single-session-user | correct | R1 | 1 |
| `29f2956b_abs` | single-session-user | correct | R2 | 0 |
| `f4f1d8a4_abs` | single-session-user | correct | C | 0 |
| `88432d0a_abs` | multi-session | correct | R1 | 0.25 |
| `80ec1f4f_abs` | multi-session | correct | C | 1 |
