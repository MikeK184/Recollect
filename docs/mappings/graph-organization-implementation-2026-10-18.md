# Automatic organization, graph reliability and grouped 3D: evidence receipts

Observed: 2026-10-18 (Europe/Vienna)
Confidence: current source, independent review and explicitly named local checks; the `.cache/` receipts named below are ignored local files, so the hashes here are the durable identity
Status: evidence record only. Delivery status, priorities and remaining work live in the [roadmap](../roadmap/epics/index.md#graph-delivery-status); this record carries no tracker.

On 2026-10-19 this record was reduced from about 29,500 words to scope, frozen
input identities, a paid-campaign receipts table, the failure results and the
local proof receipts. The full 2026-10-18 text, including the requirement and
proof ledger and the 18,000-word execution-pack design and preparation ledger,
is at the pinned revision
[a0aea9d](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/mappings/graph-organization-implementation-2026-10-18.md).
Nothing below was re-measured: the reduction reran no model, performance,
browser or installed-runtime check.

## Scope and method

The user authorized implementing the complete October 18 graph research plan:
reliable automatic operation, semantic entities, reversible aliases, stable
overlapping topics, incremental privacy and recovery, qualified navigation,
grouped 3D and bounded feedback-ranking experiments. No completed subset
substitutes for that end state.

Authority remains [ADR 0020](../adr/0020-automatic-knowledge-organization.md),
the [organization](../contracts/automatic-knowledge-organization.md) and
[concurrent preparation](../contracts/graph-concurrent-preparation.md) contracts,
and [ADR 0021](../adr/0021-grouped-spatial-graph-navigation.md) with the
[grouped 3D contract](../contracts/graph-grouped-3d-navigation.md).

Method: a frozen corpus and recipe precede every evaluation; every failed,
rejected, truncated, unassigned and unknown outcome stays in its denominator;
model output is never repaired or rescored into a pass; quality is judged
separately from execution integrity and cost. Native fixtures use separately
owned disposable PostgreSQL databases with provider credentials unset, and the
normal API, worker, policy and data were not changed. Campaign databases were
removed only after ownership and zero-session checks, without forced drops;
sanitized archives and immutable exports are retained. Restoring an archive is
`NOT_TESTED`.

## Frozen inputs

| Input | SHA-256 | Note |
| --- | --- | --- |
| [Adversarial corpus](../research/graph-organization-adversarial-fixtures-2026-10-18.json): 20 cases (12 development, 8 held-out) | `a0ecfe7dd692c2517f00035c0be7205841e27e7b17551f7a21580edc6d1c916c` | Authored by an independent QA agent before recipe tuning; hash covers the UTF-8 JSON including its final newline. One source exceeds 5,000 bytes before its first mention; an unsupported German case stays in the denominator |
| [Original 40-document corpus](../research/automatic-knowledge-mapping-fixtures-2026-10-09.json) (30 development, 10 held-out) | `80e2040b63efa5738f888f99cae3234adfae6b0ad484ccd256da189d6bdd0dbc` | Computed 2026-10-19 from the tracked file, whose only commit is `f2673d7` |
| [Fresh 12-document holdout](../research/graph-organization-fresh-heldout-v1-2026-10-18.json) | `a01e61e95454d02aec7d4446530a13d16900dbe50550495c7be052f89bea3d67` | Frozen before development parameter selection and before any provider output; observed by the remaining111 campaign, so it cannot serve as untouched evidence for a retuned recipe |
| [Blind label rubric v1](../research/graph-organization-blind-label-rubric-v1-2026-10-18.json) | `bb45993468ed83f0d92ca438b7fef2f4058d13c755def32529d23c481e4f2a02` | Authored before any remaining real output |
| [Canonical reconciliation controls v3](../research/automatic-reconciliation-native-controls-v3-2026-10-18.json) (six controls) | `d15e0171a691e1656dcd4e3955eca39859c6b73a19b8257402c692e69101cf81` | Explicit update, historical, negation/withdrawal, unrelated, duplicate and protected-human-memory cases |
| Development selection (tuple `(3, 0.65, 0.25)`) | `084c075eb5f0a300861e2c95cd6c75ad17506d2c2b49b342070dbab45457056d` | Only the original twelve development documents may fit the declared grid |
| Candidate freeze v2 (198 Rust sources/manifests, 64 migrations) | `ffc4f4251823bde4060c2d1158482728b4913d955e508a1fef9b8cf0cb6c6431` | Pre-data semantic freeze; not an executable, body or price approval |
| Private24 corpus (24 synthetic sources, 134 mentions, 14 aliases, 65 relations, 18 identity controls, 276 pairs) | guide `0ecdcd8ad4f51ca0c83c5f6a1aa27ee7db4eac7207b5d7083578c81ad764f4fc`; original `f773d2dc718f56b8cc2bc4bf6497dfec72e2c044f854e3325ad20b3a34b5a096`; initial review `dc5c1a3815cca4f4fd01e6b0236ca990e5e325aea5bebbe2171c7ec303017ac6`; final corpus `837d2d130353b1e8aede130fa149cbe98bf2959c9bdabbe54993b87c3c6a650f`; freeze `3d019686181262f7f9dfad6d5d886538c8af6015131903af16499754fc796649`; final review `51ea9ff5046c64e5464c9337e93ff1bb61e40343b2a4e2576906f808b518f8c2`; additive manifest `91b2b04dfc6dd5344d4cf8bb70ab6a034df7d70f3ef2e2dc22e91f31e3431ca8`; admission receipt `d0a6edf7259b44f5b809baea07ae113f94f46956aa6d8f745d64daa53926ea66` | The initial review found two missing relations, one unsupported membership and one vocabulary error; the failed freeze is preserved. One source and 21 topic pairs stay unknown (255 known), so complete-topic gold is `NOT_READY` |
| Prospective22 corpus (22 sources, 221 mentions, 5 aliases, 111 relations, 21 identity controls, 231 pairs, zero unknowns) | manifest `55a7ae711f340b5bb32c7a9faa7b48d13c5b9fa2a986f0e01ced348bc6ba0232`; review receipt `a59881010655c28864be0a94f2e5e64ef81cf861e76c1a28b698fd84d3d6d345`; root verification `111e4e98d96a6169643a8bc6457da0a3304f1772509218093506e2f2e7b16d37` | Authored after the candidate freeze; status `READY`. Blinding on shared storage is procedural only |
| Source-only feeds | source-facts-4 recipe `d06932a766353e1e36b4abeff80d0cb32b4883bce22dcd35a8aa4fd8bfc91915`; source-reconciliation-4 recipe `146a020dc340ed72eefa2394ba602803552f7e03c7fe765111b34d47085e441b`; preparation receipt `904409f14ab49b6a491d643dfbc2fd4a01bfad90c1d4b8e84a0461b81c55c0bf` | Exact source text, title, scope and hashes; no gold |
| Tooling | scorer `0fc5da6216cf00acb82d4eabb8213f37021e0e00b737587dfe1a7a9917800a94`; preparer `282b1425a96989ce64b0831f0f15c257663d25d477d67c14cf90704203e58f58` | Both make no network, provider or database call |
| Pricing references | 24-hour public rates `95dd80239956cb0b8a7cd61e7411f6a2cc1e7f2f231c1f6dcc1f70d62cbb89b6`; Responses interface reference `41a82ea9564b36d9657259a4648fc769405e993d6bd217e498b6f9b9cca008a1` | The runner pins `service_tier: default`; omission would inherit Project settings and could select premium Fast processing |

## Paid campaign receipts

All approvals are the owner's explicit async replies. Spend is a frozen
public-rate estimate, not an invoice or provider billing. A change of scope,
model, prompt, corpus or cap needed a new approval; no grant was reused. Every
campaign ran once with no retries, no answer judging and no normal-installation
policy change.

| Campaign | Owner approval | Calls and tokens | Spend: estimate / worst reserved / cap | Frozen inputs | Outcome |
| --- | --- | --- | --- | --- | --- |
| source-facts-2, original 60 documents | `call_9e23214078af4ca690fcec1bc2aca8bf`: "Approve up to $2"; at most 60 calls to `gpt-5.6-luna` | 60 requests; 57,472 input / 19,001 output tokens | USD 0.037169200 / 1.308672000 / 2.00 | recipe `0db65e0d88161ae0e61778edbebd23a06174f0d7ec11401d68e2fd949a4822a0`; outcomes `d95e51c107db34e5db3b2222e157be0c819f2bcd6f616c8717387459d847ce17` | Failed |
| remaining111: 78 reconciliation-3 calls plus embeddings | `call_946abf4b257c4e91bd1b711aaae4dea7`, item 0; additional USD 2; no source-facts calls | 82 provider requests, all succeeded: 78 reconciliation, 4 embedding HTTP covering 33 chunks in 32 documents; 136,206 text input / 29,110 output / 2,247 embedding tokens | USD 0.069275610 / 1.736417280 / 2.00 | recipe `19a6a7b429c1eb1f911e1e0df6ddb2615c0422e4b83953a5b40102413ecc5bb5`; executable `f5a537faf2238a10ba34814582bd872418863beacb353bf2075fad6472d41307`; 43 source bindings, 63 migrations, rubric; execution receipt `0d2bba08e19b4c23d40f7f3abf4e34be43a53fb3f1d1461829e90e232cc2ae47`; summary `9d859e40bfcc6692be147c0a26dfe9a6c92b5670180b82c943d32ac55d42234f`; closeout `2aba80d88b1604029a30b209e2a88f057fd561e81ad5bf0efce85678903928d3` | Failed (`failed_quality_no_activation`) |
| support6 diagnostic v2: six `source-support-3` calls on retained outputs | `call_59d8e8d730d244e2aaf66df0269c8771`, item 0; USD 0.15; at most six calls | 6 requests, one at a time; 10,116 input (9,187 cache-write) / 642 output tokens; largest body 8,792 of 65,536 bytes | USD 0.003252950 / 0.130867200 / 0.15 | recipe `7fbb86e5fc2dfb0b791a436e0234c2828931e864c10e46a73e08919a307da088`; executable `ac4b63d6e8e29afd2255e340c725fb8717a1f8b5d2225c5f94642be43ad0a6f7`; v1 (never executed) executable `37c55bb22da0b681d6ab3bad62532d78f0bff3069a7f059a88cd8be6ddb21b9e`, semantic binding `2088e054f311ee54dc11b07d1089befff5cf81b68d90c05f1b31af9cec3e6afb`; model page retrieved 2026-10-18 13:23:48 UTC; execution receipt `397474e38d20d1ea1c6e6add0c31f0cd7296836fb4be386547fbcca79f4e0a73`; audit `ab958e4f1dfb4d7aac082b19e5c870aede306999ed6121ba892dc7f2e429c554` | Canonical behavior 4/6 |
| heldout46 current4: both families on 46 sources, six controls, conditional support | `call_abc61006c6d54f61bddd1a590266209e`: "Approve additional $2.20"; at most 105 calls | 105 calls: 86 extractor, 6 control, 7 shared embedding, 6 conditional support; 228,508 text input / 69,553 output / 4,682 embedding tokens | USD 0.14119926 / 2.19899904 (2,198,999,040 nano-USD) / 2.20 | recipe `f87821f8ebb68eb97bfae1b6fe44f68d6f2ad1d974efc997bc60f036155b4e07`; schedule `a60df3f131e82949c62ccd8b9c54879668b220f13a6d399112e929b27f8435cd`; executable `84c4699395b74cd9db847c14050303d846b97163486348c6534612a75ad9b080`; preparation `0b2ca70dc1c200b77f0e159cf025b90e4882971b50fbaa301d799c62c494026c`; root verification `10ba463e1ac46a862af69b4238e6fca36bae1b5e4ee3018122e2caca2620fbce`; pricing expiry 2026-10-19T15:45:22.262166Z; execution `2995ff2baa083e989f6baff3372e2ee40aabbf3179a89d7fd16133b7ca306190` | Failed |

Totals: four executed campaigns, 253 calls, USD 6.35 approved and about USD
0.250897 estimated at frozen rates (text input charged at the higher cache-write
rate). Retained but never approved or executed: the 147-request proposal
(maximum USD 2.932715520), the 171-request proposal (USD 3.299880960), the v1
support6 freeze, and every false approval template; all remain unapproved. The
USD 2.20 qualification, like the others, authorized no further paid call; any new
campaign needs its own exact approval.

## Completed frozen 46-source qualification

All 46 sources are held out (three exact-empty). Both families keep all 46
observations; the four export artifacts hold 92 observations and 460 mode rows.
Execution receipt `2995ff2baa083e989f6baff3372e2ee40aabbf3179a89d7fd16133b7ca306190`; export `67b90b893e3ec5ce6c7d84938858593a27195e28a1c5733a48d4ccbf81f8a233`;
scalar `af647834627ba4720217e19b0f4c404cecce4cc4dc0616c457fdb0591d200b26`; canonical
audit `154a5b1c76f4d3b9bd8aef7038359351c1bc949930a226cc7646509e8f25b7e9`;
independent scalar/native audit `6c56e2dd2bd9d176b8815d60b7732050ffc27fc8576c5f51b82e86ae6931c1b5`;
finalization `623f6934d98ca77b09045039064697136caba8d5db0945f205b1a2f7779f6d76`.

| Producer | Validated / rejected / absent / empty | Exact typed mention TP / FP / FN | Precision | Recall |
| --- | --- | --- | --- | --- |
| source-facts-4 | 8 / 35 / 0 / 3 | 31 / 29 / 324 | 51.67% | 8.73% |
| source-reconciliation-4 | 6 / 35 / 2 / 3 | 13 / 19 / 342 | 40.63% | 3.66% |

Both fail the frozen 95% precision / 80% recall gate. A source-derived
diagnostic (receipt `72d67b3749cb378f1ea919606f5f7975758c40f6db6e8f31cdcd4304fc987105`)
reproduces all 70 native rejections without changing an output: 66 span failures
(28 line/occurrence, 20 quote-location, 17 relation evidence excluding an
endpoint, one duplicate physical mention), three missing alias endpoints and one
incompatible alias kind. All 86 submitted bodies equal the frozen source bytes.

Five-mode comparison, 507 pairs per producer (486 known, 21 unknown; unknowns
stay outside agreement counts but visible in assignment diagnostics):

| Mode | Result |
| --- | --- |
| Native hybrid | 17 TP / 6 FP / 89 FN; precision 73.91%, recall 16.04%, F1 26.36% for both producers; assignment 27/46 (facts) and 26/46 (reconciliation); recovered-positive-source coverage 20/35 |
| Vector-only | F1 27.69% |
| Canonical metadata | F1 63.01% |
| Entity-overlap and native-weighted | F1 1.87% each |

Cross-Brain related-pair failures are zero in every mode. Native export records
40 fresh reports, 16 actual cleaned GDS attempts (2026.08.1); empty reports keep
a null engine version. Both families reuse the same 44 complete, ready vector
entries covering 43 nonempty sources.

Canonical behavior is **4/6**, strict required-action/citation agreement **3/6**:
rc02 (historical) wrongly retires current memory; rc03 revises a claim instead of
the required retirement; rc01 gets the replacement right but misses one exact
required citation span. rc04, rc05 (supported same-ID reuse, no duplicate) and
rc06 (reviewed memory preserved, conflict retained) behave correctly.

Outcome: extractor, grouping and canonical semantics fail qualification; no
producer or recipe is activated and blind label usefulness is unmeasured for this
campaign. Next quality work must address occurrence/quote selection and
historical-versus-retirement semantics, retain these failures and use a newly
frozen evaluation boundary.

## Earlier campaign failures

**source-facts-2 (original 60).** 57 batches rejected (32 invalid schema, 23
invalid span, 2 invalid key), 2 validated, 1 absent mapping. Strict all-60 exact
span/type score: 2 TP / 3 FP / 188 FN, precision 40%, recall 1.0526%; original-40
score 2 / 3 / 114, 40% / 1.7241%; the German unsupported-language control fails.
A development-only mechanical diagnostic found 22 unsupported relation labels,
nine evidence spans not enclosing their endpoints, seven quote/occurrence
failures and one invalid key. Rejected output was neither rescued nor removed.

**remaining111 (reconciliation-3 plus vectors).** 60 validated, 16 adapter-
rejected, 2 absent. Controls: the historical control replaces current port 8443
with a quoted 8080; the negation/withdrawal control proposes an affirmative
`uses=Redis` and omits the retirement; the duplicate control emits two unchanged
claims where zero or one is allowed. The selected hybrid on the fresh 12 sources
assigns 7/11 (63.636%) against 80%, with pair precision 100% and recall 1/10
(10%) over 66 pairs (nine unknown). No current support-assessment or publication
call was made, so these are unsafe extraction hypotheses, not publication.
Source-facts-3 remains separately unqualified. Retained original-development
audit: assignment 9/11, 6 TP / 0 FP / 21 FN over 66 pairs (57 known, nine unknown).

**support6 v2.** Eight stage verdicts supported and two contradicted across ten
claims; no retirements supplied.

| Control | Canonical outcome |
| --- | --- |
| Current replacement | FAIL: an extra sentence-valued port candidate conflicts with the correct replacement; old head and Proposed alternatives remain |
| Historical replacement | PASS: contradicted, history unchanged |
| Explicit withdrawal | FAIL: the Redis candidate is contradicted but the required retirement is absent |
| Unrelated new facts | PASS |
| Duplicate unchanged fact | PASS: one identity reused, no new revision; the raw duplicate failure remains |
| Reviewed head protection | PASS: reviewed 443 stays, new 7443 is a Proposed conflict |

**Blind label review** (new assessor; 204 groups, 271 memberships, 32 sources,
612 dimension results; ratings `06ab53493fcd60446007198be463428317e767ba5d9d4e661ce9602f16299b24`
durable before the arm mapping was read; revealed summary
`201f793ba081fec8b81026bc1fe50eddf9be1e61325c6894a10c50d343e3160c`).
Source fidelity passes 197/204, scope/time/authority 203/204, navigation
usefulness 176/204; any failed group fails that arm/partition.

| Arm | Diagnostic20 groups pass / total | Fresh12 groups pass / total | Disposition diagnostic / fresh |
| --- | --- | --- | --- |
| Native hybrid | 20 / 21 | 37 / 45 | fail / fail |
| Native weighted | 21 / 21 | 37 / 45 | pass / fail |
| Entity overlap | 14 / 14 | 30 / 38 | pass / fail |
| Canonical metadata | 8 / 8 | 8 / 8 | pass / pass |
| Eligible vectors | 0 / 3 | 1 / 1 | fail / pass |

Failures include generic deictic labels, a negated subject shown as a topic and a
product-specific label not supported by every contributor. 20 source titles and
three labels contain fixture identifiers (intrinsic blinding limit). This is
independent agent review, not paid judging or user testing.

**Strict scoring on exposed corpora** (diagnostic; fails 95% / 80%):

| Corpus | TP / FP / FN | Precision | Recall |
| --- | --- | --- | --- |
| Original40 | 79 / 38 / 37 | 67.52% | 68.10% |
| Adversarial20, English | 35 / 11 / 36 | 76.09% | 49.30% |
| Adversarial20, all languages | 35 / 14 / 39 | 71.43% | 47.30% |
| Adversarial20 positive relation endpoints | 1 / 21 / 5 | 4.55% | 16.67% |

**Grouping upper bound** (zero-call, ideal annotated mentions, recipe
`a3bc9fba26542aa631864f095adc45bea8a13e14145cf27115f121e67e86bf5e`): 13 of 19
annotated documents assigned (68.4%), 4 of 8 held out (50%), both below 80%;
native pair precision 1.0, recall 0.223684, F1 0.36559 (equal to the entity-facet
baseline, so the community layer adds no measured benefit); canonical metadata
precision 0.8095, recall 0.4474, F1 0.57627. 190 pairs retained (154 scored, 19
Brain boundaries, 17 unknown).

## Local proof receipts (no paid calls)

| Area | Result | Receipt |
| --- | --- | --- |
| Mixed 10k workload, migration 063 | 602/602 reads, zero errors, p95 0.459626 s during 8 captures, 16 forced audits, 64 cursor steps, 63 import windows; both rebuilds ready; 187.27 s | `graph-10k-mixed-final-063-pass.json` `e96f10ee2f7a42d66971d83c5ce5e7b3f68e1ffde1751d4f7ec79acfd71bbcf3`; cleanup `graph-10k-mixed-final-063-cleanup.json` |
| Mixed 10k, migration 061 (superseded, kept) | 600/600, p95 0.450670 s; earlier 598/582-request passes kept | `ea34d35c0f4dd8f0f3559ef9b39d5b3b606bb6417fe69be715756acfbbf37e20` |
| Mixed 10k first attempt (**failed**) | 40/40 reads at p95 0.771 s but imports began after reads and a rebuild exceeded its window; full mixed acceptance failed. The retry exposed a missing `graph_preparation_changed` failure transition and partial native edges surviving a new attempt, since fenced | `.cache/graph-implementation/` |
| Normal-data clones | 40/40 at p95 1.629 s (copied statistics) and 40/40 at p95 0.951 s (destination-only ANALYZE); identical 60-node hydrated payloads; restored statistics take 256-260 ms versus 26.5-26.7 s without ANALYZE | clones only; the normal installation is not updated |
| Qualified paging | 60/60 pages, p95 1.200133 s (pre-062); earlier 428.835 ms; initial 2,304-mention card query 1,995 ms before the fix | `.cache/graph-implementation/` |
| 5,000-source weighted/hybrid | 10,000 memberships from 5,002 nodes / 10,000 edges; 10.465 s calculation, 133 ms topic insert (partial body only); fallback 10.629 s | `weighted-5000-final-063.json`, `hybrid-5000-complete-fallback-final-063.json` |
| Grouped 3D adapter | 500 nodes / 2,000 edges, M4 Pro, Chrome 154 ANGLE Metal: pointer-to-inspector p95 60.0 ms, selection-to-paint p95 48.3 ms, cold first usable 2.606 s, 3.72 MiB heap growth over 12 forced-GC cycles. Not GPU render time, backend latency or a leak proof | `6addc754ba933b98c8f8b0fc8f3920d16027f1791f4d82352cbc15113e7b9b9b` |
| Semantic execution lease (main) | Six native regressions pass in 5.47 s: four cancellation/reclaim cases admit zero requests, transmissions, vectors or publications; two healthy cases publish with byte parity. Not cancellation-API, Linux or post-admission atomicity proof | main integration `a756c667d8c9b84e37e84c81d90b5ea453c69fdf1b13bcb540b7623a18e802f5`; verification `befc5fe68898a82347885eaeacf6839e6f5a7280ed6b94c1c2f81e5863eed09a`; counterfactual `e75c2ed2471f5fc8bff428a11ca57796fa29e650f39c56e02562502dd3ddf0be`; fixed `e4c7ae54d82aec312581b5ba8687bd1c44e6ae6648eeec33fbc16bdc3cba5f1a`; patch `b3f17c87a242cd2e68cc97b01780597525182da34148709876dddd5be79f906c` |
| Attributed feedback frozen comparison | 20 outcomes (18 completed, 2 unavailable scored zero), two bad promotions; toy NDCG/MRR/recall 0.166667 to 0.75 (development), 0.25 to 0.625 (held-out), 0.20 to 0.70 (all). Not real helpfulness; production RRF k=60 unchanged | result `5da5d53f3d3bcbfc825a4c60b33d9e449a861660cdedfe18415f332362dd2fcb` |
| Organization native suites | Source commands, overlays, nested split capacity (49,998 retained rows; a new split returns 422), privacy and older-backup replay, label-path repair, retained-vector candidate and unpublished evaluator cases pass in focused runs | named logs under `.cache/graph-implementation/` and `.cache/automatic-mapping/` |

## Defects found and repaired (local, no paid calls)

Each defect and its failed first attempt stay in the ignored logs named in the
full text; the fixes did not change any frozen recipe or completed receipt.

- **3D initial offscreen pause.** After a nonintersecting event arrived before
  measurement mounted the engine, render frames advanced from 11 to 19 while
  offscreen. Persisting intersection state and consulting it at initialization
  fixes it; the four-case adapter suite passes in 38.0 s (initial pause/resume,
  hidden/resize/camera preservation, path mesh colors/radii, keyboard
  selection, WebGL failure, bounded performance fixture).
- **Generation invalidation gap.** Memory/link epochs can stay unchanged while
  another client replaces a ready projection, so twenty newer unrelated
  generations could hide loaded and replacement rows indefinitely. Exact
  loaded-generation tracking passes two actual PostgreSQL API cases in 2.15 s;
  regenerated OpenAPI keeps 206 operations with the optional tracking field
  bounded at 3,774 bytes; eleven UI cases pass in 1.0 min.
- **Label-path discrepancy.** Production topic cards chose an eligible quote by
  mention UUID while shadow evaluation ranked representative terms by distinct
  contributing sources, and source-detail cards always chose the canonical title,
  so opening details could rename a topic. One shared distinct-source/canonical-
  term ranking and label composition now serve overview, entity centers, context
  and source details (250-source/80-detail bounds kept; 10,000 input / 50,000
  contributor overflow rejected). One new native regression (11.19 s), six
  existing native cases and five shadow-selector units pass. Diagnosis used
  neutral synthetic inputs, not observed held-out label strings.
- **Adapter occurrence matching.** Disjoint substring enumeration treated `aba`
  in `ababa` as unique. Adapter3 enumerates overlapping matches with streaming
  byte KMP (at most 4,096 prefix entries); 14,196 character-boundary and 196
  request-identity combinations pass; migration 064 preserves existing heads and
  seeds no catalogue entry.
- **Delayed-result race (migration 059).** An old retained response can no longer
  replace a newer published interpretation of the same source/context; the
  nine-case production/retention regression passes in 13.59 s.
- **Restored statistics.** The same canonical query takes 26.5-26.7 s without
  restored statistics and 256-260 ms after ANALYZE; recovery release now
  collects statistics before releasing its hold.
- **Pre-transmission lease window.** `ready()` validated the job lease and
  committed before gateway invocation. Four cancelled/reclaimed cases each
  recorded one model request and one loopback transmission and zero vectors or
  publications; binding the lease at execution makes all four record zero
  (see the lease row above).
- **Migration ordinal clash.** Independent migration 050 reused deletion ordinal
  680 from 047; only its `managed_skill_versions` ordinal changed (to 682).
- **Platform baselines.** Disposable-database application of migrations 001-052
  passed; the 256-deep descendant fence proof completed in 18.516 ms.

## Preparation, review and cleanup receipts

Worker, independent-review, root-verification and cleanup receipts that bound
the campaigns above. They are provenance, not quality evidence; every database
named in them was removed only after ownership and zero-session checks.

| Step | Receipts (SHA-256) |
| --- | --- |
| support6 v1 and v2 | v1 guard (rejects false template and both completed grants) `e63a03836b1bde1e74671bd0848d279e4266e0f0becba4b43f77c9111b722b94`; proposal `24dcaf4a1f0bc22d81cb2b6e96a7f968bfceb7013ca12d7a878ffd76db76dea5`; v1 cleanup `bdb299754969e6844b011d1f736ddd4e17434f6f59df6c0b9f56ee7a023811a3`; v2 cleanup `393abc9600725ea1a7992e79401d3a4e3769190975e7ac18cd97de73e403b8d8`, root verification `9b9f2d4792a92fbc42a58cfea8b66e958ba19e394884dedf7f9cd694b6e4bf62`; recovery of eight accidentally reformatted files `f15dbf03f6147f4d0dd3e071366aadbaa33f569e79d90fede10d91444161ad60` |
| Current4 builders (`source-facts-4`, adapter3, migration 064) | worker `a52d27895bac7043f01b9a29d913984d24b5cf1c6ca855f6f24d2154a61ad570`; root `02058dc948d7035205c1da29ff6b818cbb2e91b152910e9b3b413a410abe29c4` |
| Offline outcome evaluator | scorer test `7bea536d0c5aedb200560e5f4de0d3743f431b66aad4d86aaee5c8470dce38fb`; worker `b2018ce0b82c92cca016b32e13766faf2bb9585772ad48f2ef35f78cdeab0948`; review `9487fae5d47d807213949a7c4ba1279d2284ef637c391611e81ca547ad83efd6` |
| Source-only preparer | test `3294a9247eddbcad2c4c010e3aa219ed7feaff2c90728ac0d3ba4211256e5761`; worker `d37165d9d4e2c6eb47d9bfb6017ec0af3a18623a52fa88500bb38e83bced9268`; review `c5c008bd942024fbe2e19d803657989fbc3f8fa940df6631a1a49a2dc1b89945` |
| Native export harness | closeout-v2 `9421f0eeef4ff84cb84833a89d430efc7b6b99c83c6d88be6d1e01ad5719991a`; review `c8f2cc351ae229c66fe4ad998b94f9d0504823786ca49a4092d67557e485d233`; root verification `00e0fd3a6e9e1026155ae121481d328bdbeb1274672878f3a8d072b29d33023f` |
| Campaign runner (`revision4_qualification`) | redaction `be01cb009ada49ac3b9bba08265066a2d86aa24a950522037af4d7641225425a`; conformance `bea848407e68ec8ec80c82ef51a5d1dec727c77111bc0b4fdc06796d92c71763`; worker `45243f76cd6989bb1fcd05329bd372cfb83ed464e08a46b319f2256f8acdc621`; broad review `35d08464840970e6b2583c4ab1d1848a4e69527f346d3e7f197fd2397a14cfd3`; budget/transport audit `71ad0e91afe38549235d5e4e4ca29b543425bee3defa15ecd99fc4a28a09e575`; root `574491198e08bca0687fd8cd172673d65b35d0fa83edafff5a516515b9e95c28` |
| Semantic lease repair | design note `5fb46d42098c8912008631b0c5012146b88f6f87733a6a6f0c3809019f55cbc1`; unapplied patch `a909ef7bfed65047f0059626f94598f899d3dbfc32945d0f0120817cae901085`; rehearsal test module `3c11191c1df3bdc05f22e7202736e29cf77b2094ce6aca535b67e5ce5c1364c0`; failed-fixture cleanup `93bc2b19628cf0c686adab2945ce42b7d2ecd355e84b5521f40b942753f35879`; rehearsal review `cb2b8e74fee23c6140438d74d05fee36167f23e71b93d85fafd310f723ac970d`, closeout `67a4c7cc504eb754c771f9915510ac234287687d3540269a5a19b1c4c456195d`; permanent bundle root `2a365d1641a3ff12b771bf26fd3f323a55950e1c6f113112686b19203304ba8a`, review `b6edce7936120ec9bdb085b86d54f0529a0362cc34a4164d909e374f67129910`, closeout `3affbeb82a41fd3e5596e2a56b8cdc32b32e01098cecf94cdfee79cd9df4be0a` |
| heldout46 preparation | metadata review `c67c6e871e3c14bcfbfa6d2734e50e63cbb2c23e1d4285ff8cf01da76f653317`; prepared-database check `8d5969291bb4dc5c750e976ca1fcb8477b24f7d11d344c4eb069e74bf5e6ec50`; launcher and approval review `d836d572b13984d6abd3bde049282d625722e05f48c67b5ae01f7541302ced69` |

## Evidence boundaries

- Cost figures are conservative public-rate estimates; unknown billing is not
  zero cost and no invoice was read.
- Plumbing, enforcement and toy-fixture proof (parsers, leases, scorers, 3D
  adapter, feedback scorer, field-origin and privacy tests) does not establish
  extractor, grouping, label or answer quality.
- The Mac-host native proofs are not Linux runtime qualification; the normal
  installation was not updated; nothing here is deployment evidence.
- Receipts live in ignored `.cache/` directories on the maintainer machine;
  re-verify a hash before relying on a file that is no longer present.
- The 20-case corpus, the original 40 and the fresh 12 are observed and cannot be
  reused as untouched qualification data for a retuned recipe.
