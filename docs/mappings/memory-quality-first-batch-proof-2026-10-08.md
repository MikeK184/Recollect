# Memory quality, graph performance and local update — first-batch proof

Observed: 2026-10-08 to 2026-10-09 (Europe/Stockholm)
Confidence: observed-once
Delivery: historical staged implementation/deployment proof, later committed in f2673d7.
Status: superseded for runtime status by the [October 9 checkpoint](automatic-knowledge-mapping-progress-2026-10-09.md)
(migration 047, 35/40 normal graph reads). Reduced on 2026-10-19 to the result,
the installation receipts, the graph runtime failures and the paid-diagnostic
cost ledger; the full text is at the pinned revision
[a0aea9d](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/mappings/memory-quality-first-batch-proof-2026-10-08.md).

## Result

The first batch repaired demonstrated evidence-delivery, answer-instruction,
graph-entry and update problems. It does **not** establish that all benchmark
failures are fixed. The completed official LongMemEval result remains **20/50
correct (40%)**: **12/42 answerable (28.57%)** and **8/8 abstentions**
([benchmark baseline](openrouter-benchmark-proof-2026-10-08.md)). New paid
diagnostics did not complete, so there is no new comparable score.

The normal app moved from migration 035 to 043 without resetting memory; every
checkpoint identity across five measured inventory tables survived and model-policy
heads were unchanged. Authenticated graph reads still timed out under background
contention, so graph performance acceptance stayed open and the first-batch pack
was not shipped. Behavior is governed by the
[first-batch pack](../roadmap/execution/active/operations-memory-quality-first-batch.md)
and the accepted recall, answer, graph-exploration and installation contracts.

## Method

Production handlers ran in owned disposable PostgreSQL/Neo4j fixtures, actual
browser rendering, mocked model transport and an encrypted normal-installation
checkpoint restored only into a fresh disposable database. Normal customer
content was not sent to external providers. Paid diagnostics used the original
public histories with GLM/Qwen in their isolated benchmark database. Base commit
`27bbd00f006e9684b25f18b47abcfc3110f22806` with uncommitted changes (build time
distinguishes images; it is not a release identity). Context7 checked the current
Cytoscape grid layout guidance.

## Delivered changes (proof limits kept in the full text)

Whole 4,096-byte canonical source chunks survive context packing and publication;
up to three nonoverlapping windows per source after primary diversity;
`brain-answer-2` accepts attributed user reports and follows the question language
with strict JSON and exact citations; large graph entry is an explicitly partial
250-node/1,000-edge window with 100-descriptor entity pages and one initial read;
requested-entity filtering and identity narrowing precede support checks and
history loads; legacy support verification uses a current-root fast path (read-only
real-Brain selection of four audits in 2,638 ms, later about 716 ms against 4,104
ms); migrations 042 and 043 repair the dependency predicate with differential
comparison against 037/039; graph rebuild qualifies roots once under one lock; and
`stack.sh update` takes a locked encrypted checkpoint, builds, migrates and
verifies build/schema/API-worker alignment with pinned Compose project settings.
Focused native, browser, recovery and update checks passed (including eight native
answer scenarios, 8/8 dependency/lineage controls, three browser scenarios and
six root-update checks); the 500-node/2,000-edge browser stress fixture is mocked.
One early `--help` check entered the update path and paused the old app for a
backup before it was stopped; the app resumed. That mistake is retained.

## Installation and recovery receipts

- Independent original checkpoint `50554529-d4ac-45b9-9854-6a73638351bc`
  (encrypted bundle/journal validation and isolated database restore passed).
- First normal update checkpoint `e27a3111-0eb7-402d-8e7c-7ebc597d8515`.
- Final update checkpoint `5e905eb0-c6cf-4690-be5c-c8943a17b183`, build
  `2026-10-08T23:04:06Z`, revision `27bbd00f006e9684b25f18b47abcfc3110f22806-dirty`.
- `.cache/memory-first-batch-inventory-proof.json`: zero missing checkpoint IDs;
  17 Brains, 1,624 claims, 841 source versions, 15 MCP calls and 1,456 model
  requests at the checkpoint; live counts at `2026-10-08T23:11:34Z` were 17,
  1,625, 1,034, 15 and 1,548 (ordinary capture grows inventory). All model-policy
  heads matched.

The comparison used a unique owned temporary database, the existing recovery
validation and an ownership check before removal; the normal database was never
overwritten. Identity preservation does not prove every stored field identical.
A complete off-machine restore was not repeated.

## Real graph runtime failures (retained)

- Migration 041, before SQL pushdown: both large-Brain windowed endpoints returned
  `503/database_unavailable` at about two seconds; small fixtures had missed it.
- An intermediate repair still timed out. The current knowledge generation had
  zero nodes (versus 1,230/1,065 superseded) because the migration-037 support
  checks correctly withheld unaudited legacy memory; metadata showed 1,048 current
  claims, learning enabled and zero support assessments; the ten-second failure
  was in legacy target selection.
- After the scheduler repair four audits were admitted but two jobs failed with
  `database_error` before model work (migration 042 repaired the predicate). The
  original four then completed (one supported, three insufficient), more audits
  were admitted and graph construction failed again: a running support scan
  blocked Brain advisory/row locks and readers exhausted their two-second limit;
  graph descriptor construction repeated four whole-Brain scans under the writer
  lock.
- Single-leaf probes: human-review check 1.579 ms, revision support 5.829 ms,
  dependency support 2,501.643 ms, full digest-aware gate 5,097.559 ms; the
  frontier query returned its single dependency in 0.079 ms (migration 043).
- First migration-043 observation: a four-node/two-edge generation and a
  successful centered read, but one overview failed; a later six-node/three-edge
  overview succeeded while the centered read failed (successful requests took
  about 3.3-5.0 s). Not acceptance.
- Final image (`2026-10-08T23:11:27Z`): 71 succeeded audits (16 supported, 55
  insufficient), eight failed (six database errors, one lease expiry, one provider
  timeout), four queued; 16 current roots with positive assessments; latest ready
  generation 23 nodes / 17 edges. Its first authenticated windowed view failed
  `503/database_unavailable` in 3,033.95 ms and overview exploration failed
  `503/graph_timeout` in 15,005.10 ms; two further diagnostic pairs failed (about
  15 s each); no centered read was possible.
- Read-only phase sampling: advisory Brain-lock waits up to 9.679 s, a background
  source-fragment query up to 8.696 s, a support gate active 2.994 s; the candidate
  query itself returned 29 fragments for 23 identities in 900.603 ms. Locks were
  not removed and timeouts not raised.
- Job history carried unresolved learning failures (45 `source.learn/model_input_too_large`,
  93 `source.learn/provider_shape`, 1,830 `semantic.generate/model_budget_exhausted`);
  accumulated historical counts, not all caused by this update.

## Paid diagnostic results and cost

Both diagnostics repeated the original 50-question manifest (canonical histories,
GLM answers, Qwen embeddings, 16 KiB context) with extraction and learning OFF;
neither is the frozen held-out sample.

| Diagnostic | Case outcomes | Model requests | Known additional USD | Unknown reserved USD |
| --- | --- | ---: | ---: | ---: |
| DeepInfra FP4, `brain-answer-2` | 6 completed, 19 failed, 1 uncertain, 24 unattempted | 52 | 0.00286372475 | 1.90 |
| Separate frozen Novita FP8 route probe | 0 completed, 16 failed, 34 unattempted | 32 | 0.00001232 | 1.60 |

DeepInfra failed mainly with HTTP 429; an uncertain timeout stopped that arm.
Novita's 16 attempts failed through the native provider interface; its saved quote
did not advertise `structured_outputs`, but the precise cause was not proved
because raw provider errors were not captured. The adapter was restored to the
reviewed DeepInfra FP4 route before deployment; normal Brain policies were not
switched.

Combined campaign ledger: **USD 0.22070654563093992 known billed** plus **USD
7.570710000000002 unresolved reservations** equals **USD 7.791416545630942
commitment against an USD 8 ceiling**; only USD 0.00287604475 of known cost came
from this batch. Unknown billing is not zero cost (`.cache/memory-first-batch-campaign-accounting.json`).
No campaign request remains running and no further paid benchmark or judge call
was issued. The fresh held-out 50 questions remain frozen and unexecuted; the
baseline had full supporting sessions retrieved in 23 of 30 wrong answers, so
learning absence alone does not explain the failures.

Final `./scripts/validate.sh` passed at that time; neither it nor focused checks
establish live graph or answer quality. Version N/A; uncommitted at the time.
