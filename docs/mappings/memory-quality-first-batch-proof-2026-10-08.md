# Memory quality, graph performance and local update — first-batch proof

Observed: 2026-10-08 to 2026-10-09 (Europe/Stockholm)
Confidence: observed-once
Delivery: local code and normal root installation; uncommitted, no release or push.

## Result

The first batch repairs demonstrated evidence-delivery, answer-instruction,
graph-entry and update problems. It does **not** establish that all benchmark
failures are fixed. The completed official LongMemEval result remains **20/50
correct (40%)**, including **12/42 answerable (28.57%)** and **8/8 abstentions**.
New paid diagnostics did not complete, so there is no new comparable score.

The normal app was updated from migration 035 to 043 without resetting its
memory. Every checkpoint identity across the five measured inventory tables
survived; existing model-policy heads were unchanged. The final image is healthy
and automatic support verification is progressing, but authenticated graph reads
still time out under background contention. Graph performance acceptance remains
open; this first-batch pack is not marked shipped.

## Sources and Method

The [original complete benchmark report](openrouter-benchmark-proof-2026-10-08.md)
retains official scores and the combined recovery measurement. The
[improvement proposal](../research/memory-product-improvement-plan-2026-10-08.md)
retains the larger research phases and Cognee/Atlas comparison. This implementation
is governed by the [first-batch execution pack](../roadmap/execution/active/operations-memory-quality-first-batch.md) and accepted recall, answer,
graph-exploration and installation amendments, not by the research proposal.

Tests exercised production handlers in owned disposable PostgreSQL/Neo4j
fixtures, actual browser rendering, mocked model transport, and an encrypted
normal-installation checkpoint restored only into a fresh disposable database.
Normal customer content was not sent to external model providers by these tests.
Paid diagnostics used the original public histories with GLM/Qwen in their
isolated benchmark database. Reference checkouts were not changed.

Base commit: `27bbd00f006e9684b25f18b47abcfc3110f22806`, with uncommitted changes.
Build time differentiates local images sharing this dirty revision label. It is
informational, not a cryptographic release identity.

Current Cytoscape guidance was checked through Context7 against the
[primary grid implementation](https://github.com/cytoscape/cytoscape.js/blob/unstable/src/extensions/layout/grid.mjs)
and [layout lifecycle documentation](https://github.com/cytoscape/cytoscape.js/blob/unstable/documentation/md/layout/run.md).
The label-aware fallback uses that renderer's existing grid layout.

## What went wrong and what changed

| Observed problem | Delivered change | Proof and limit |
| --- | --- | --- |
| A canonical 4,096-byte source chunk was clipped to 2,048 bytes | Preserve the whole canonical chunk through context packing and publication | Native late-window test verifies exact bytes/coordinates and a cited second window; still subject to the context budget |
| Semantic ranking collapsed a source to one fragment | Retain ranked alternatives and pack up to three nonoverlapping windows per source after primary diversity | Tests verify separate cosines/channels, lexical alternatives cannot crowd out semantic windows, and tight budgets do not silently substitute an unscored lexical fragment |
| Answer instructions over-refused historical/personal evidence and mismatched the question language | `brain-answer-2` accepts attributed user reports/preferences, distinguishes past/current/verified facts and follows question language | Strict JSON, exact citations, current authorization and erasure gates remain; no official score improvement proved |
| Large graph entry qualified the entire selection and performed redundant reads | Optional explicitly partial 250-node/1,000-edge overview or one-hop window; 100-descriptor entity pages; one initial exploration read | Native 501-node fixture and actual browser request/usage assertions; complete APIs retain overflow refusal |
| Requested entity filtering happened after expensive support checks | Materialize requested claims/sources/facts/manifests before support checks and chunk fan-out | Latest-at-time selection stays unfiltered; all requested source chunks and final exact identity checks remain; native superseded and rejected-fragment controls pass |
| Even an empty graph scanned unrelated source/capture histories | Narrow logical claim/source identities before loading histories, retaining every version of each requested identity | A superseded exact revision cannot become current; the independent native review/window controls pass |
| Legacy support verification never queued on the normal Brain | Compute the canonical current-root dependency closure once; evaluate costly child/cycle gates against a materialized candidate stream and stop after four eligible admissions | Read-only real-Brain selection returns four in 2,638 ms; lineage/cycle and direct/human-authority native scenarios pass; full backfill completion remains a separate runtime state |
| Layout completion could still leave small disconnected node pairs overlapping | Two-dimensional initial positioning and settled node/label collision detection with a built-in grid fallback | Geometry assertions and independent screenshot review pass at 1,280, 1,440 and 1,920 pixels; this is bounded-fixture visual proof |
| Queued legacy audits then timed out during execution | Migration 042 gives the existing target predicate an exact current-root fast path and shared historical dependency closure | Differential comparison against the original predicate and independent review pass; execution/backfill runtime outcome recorded below |
| A supported leaf still paid for every contributor edge in its Brain | Migration 043 follows only the reachable exact/current-head dependency frontier and rejects unassessed roots before expensive positive checks | Differential dependency-set and support-predicate comparisons against migrations 037/039 include historical identities, absent IDs and damaged cycles; all positive guards remain |
| Graph rebuild held the Brain lock while repeating full qualification four times | Materialize exact reviewed/current-policy assessed authority IDs, qualify current roots once and reuse their IDs under the same lock | Exact decision actor/transition checks remain; canonical eligibility and final epoch/retention checks remain; deployed populated-graph proof below |
| Audit scheduling still blocked graph readers while scanning all historical edges | Prefer eligible current roots and search the full historical closure only for remaining capacity | Same canonical admission gates and four-audit limit; live read-only probe falls from about 4,104 ms to 716 ms; historical lineage/recovery controls pass |
| Repository code was newer than the installed app | `stack.sh update`: locked encrypted checkpoint, build, migrate and exact build/schema/API-worker verification | Normal update preserves memory; fresh isolated restore and all measured old IDs preserved; no automatic remote update schedule |
| Inherited Compose settings could redirect the checked update | Pin project/directory/environment/config for every root stack operation | Test actually runs a stubbed stack with conflicting inherited and `.env` overrides and verifies all four commands target the owned installation |

The implementation preserves exact provenance and all existing truth/privacy
checks. Tests explicitly reject unchecked assertions before owner acceptance,
withhold old revisions after a correction, and withhold the whole graph source
vertex when one of its fragments is unsafe. A safe remaining raw fragment may
still be readable outside whole-vertex graph traversal. Acceptance-fixture repairs
update stale assumptions about support eligibility and multiwindow counts; they
do not weaken the production gates.

## Focused validation

| Check | Result | Retained receipt |
| --- | --- | --- |
| Eight native answer scenarios | PASS, 8/8 | `.cache/memory-first-batch-answer-tests.log` |
| Whole-chunk/nonoverlap/context controls | PASS | `.cache/memory-first-batch-span-test.log` |
| Multiwindow citation and in-flight erasure | PASS | `.cache/memory-first-batch-window-publication.log` |
| Semantic alternatives and independent rank controls | PASS | `.cache/memory-first-batch-semantic-windows.log`, `.cache/memory-first-batch-semantic-controls.log` |
| Native exploration, overflow, excluded hub and damaged projection | PASS, 3/3 after SQL pushdown | `.cache/memory-first-batch-graph-sql-tests.log` |
| Native latest-revision and whole-source withheld-window controls | PASS, 2/2 | `.cache/memory-first-batch-graph-window-gates.log` |
| Legacy exact-lineage/cycle, original-predicate equivalence and direct audit/human-authority controls on migration 042 | PASS, 2/2 | `.cache/memory-first-batch-legacy-lineage.log`, `.cache/memory-first-batch-direct-audit.log` |
| Native MCP handover pagination, scope and model-free graph reads | PASS, 1/1 | `.cache/memory-first-batch-mcp-graph-controls.log` |
| Final 043 differential authority/dependency, lineage/cycle, rules/scope, expiry and native knowledge/exploration controls | PASS, 8/8 | `.cache/memory-first-batch-043-support-equivalence.log`, `-support-lineage.log`, `-support-rules.log`, `-support-deadline.log`, `-graph-knowledge.log`, `-graph-exploration.log` with the same prefix |
| Current-root scheduling, historical fallback and bounded recovery | PASS, 3/3 | `.cache/memory-first-batch-final-scheduler-direct.log`, `-lineage.log`, `-recovery.log` with the same prefix |
| Actual graph/chrome/recall browser scenarios | PASS, 3/3 | `.cache/memory-first-batch-final-browser-tests.log` |
| Root update help, lock, ownership, failure/schema gates and override isolation | PASS, 6/6 | `.cache/memory-first-batch-local-update-tests.log` |
| Existing root/installation/recovery Python checks | PASS, 3 + 2 + 4 | earlier focused receipts retained in `.cache/` |
| Typecheck and design rules | PASS | `.cache/memory-first-batch-final-typecheck.log` |
| Governance validation and support-corpus controls | PASS, 32 governance regressions and 10 support-baseline checks; corpus valid with zero provider calls | `.cache/memory-first-batch-final-validation.log` |

The browser stress fixture rendered 500 nodes/2,000 edges and exercised pointer
selection in about 2.2 seconds. Its stress data are mocked; this does not measure
backend selection latency or unlimited graph capacity. Independent visual review
passes all three desktop widths after the collision repair.

One early `--help` check accidentally entered the update path before argparse was
added and paused the old app for backup. It was stopped before build/migration,
and the app resumed. The side-effect-free help regression now passes. Later
deliberate checkpoints and updates completed. This operational mistake is retained
rather than described as a successful update.

## Normal installation and recovery evidence

- Independent original checkpoint proof: `50554529-d4ac-45b9-9854-6a73638351bc`;
  encrypted bundle/journal validation and isolated database restore passed.
- Actual first normal update checkpoint: `e27a3111-0eb7-402d-8e7c-7ebc597d8515`.
  Its restored ID sets were compared with the final running migration-043 database.
- Final update checkpoint: `5e905eb0-c6cf-4690-be5c-c8943a17b183`.
  Build: `2026-10-08T23:04:06Z`, revision `27bbd00f006e9684b25f18b47abcfc3110f22806-dirty`.
  Update completed with API/worker image alignment, readiness and current schema.
  Receipt: `.cache/memory-first-batch-final-scheduler-update.log`.
- `.cache/memory-first-batch-inventory-proof.json` records zero missing checkpoint
  IDs: 17 Brains, 1,624 claims, 841 source versions, 15 MCP calls and 1,456 model
  requests. At `2026-10-08T23:11:34Z` the live counts were
  17, 1,625, 1,034, 15 and 1,548.
  Ordinary ongoing capture/processing accounts for a growing inventory; equality
  of counts was not assumed. All existing model-policy heads matched.

The comparison uses a unique owned temporary database, validates the decrypted
bundle with existing recovery code, compares metadata in memory, and checks the
ownership marker before removing that database. The normal database was never
overwritten. Temporary plaintext was removed. Identity preservation does not
prove every stored field identical. A complete off-machine restore/host activation
was not repeated. The default backup and recovery key remain on this machine.

A new image supplies current code; migrations bring persistent data into the
schema expected by that code. Backup/restore does not make migration history
unnecessary. The update now checks this alignment explicitly.

## Real graph runtime observation

Before SQL pushdown, the already updated migration-041 app was healthy, but both
large-Brain windowed endpoints returned `503/database_unavailable` at about two
seconds. Small fixture success had missed this production query timeout. The
before receipt is `.cache/memory-first-batch-runtime-before-sql-proof.json`.

An intermediate repair still timed out. Inspection then found the current
knowledge generation had **zero nodes**, versus 1,230/1,065 in its superseded
generation: migration-037 support checks correctly withheld unaudited legacy
memory. Metadata showed 1,048 current claims, automatic learning/autonomous memory
enabled, and **zero support assessments**. Worker warnings and active-query
sampling located the ten-second failure in legacy target selection. The scheduler
repair preserves the exact canonical target closure and four-admission bound.
Its materialized input orders candidates, but outer scheduling order is
best-effort; no strict oldest-first or whole-worker throughput claim is made.
The local metadata receipts are `.cache/memory-first-batch-normal-eligibility-audit.json`
and `.cache/memory-first-batch-legacy-query-profile.log` (the initial 2,638 ms
observation; later profile output contains the separate 4,104 ms observation).

The scheduling repair admitted four audits. Their execution exposed the same
expensive canonical target check; two jobs failed with `database_error` before
model work. Migration 042 repairs that existing predicate without changing
target eligibility. Independent source review confirms identical Brain scoping,
exact/current-head dependencies, cycle termination and function privileges.
The new differential test compares current, historical and absent fixture
targets with the original migration-037 predicate. Cross-Brain and damaged-cycle
equivalence was source-reviewed alongside existing controls; it is not a new
exhaustive differential test.

After migration 042, normal automatic recovery completed the original four
audits: one supported and three insufficient. The insufficient verdicts withheld
those revisions; normal audit verdicts were not independently scored. More audits were
admitted, but graph construction failed again. Runtime sampling found a running
support scan blocking Brain advisory/row locks; graph readers then exhausted
their two-second statement limit before reaching the windowed candidate query.
Source inspection located four repeated whole-Brain scans in graph descriptor
construction under the exclusive Brain writer lock.

Individual read-only probes on one supported leaf took 1.579 ms for human-review
checking, 5.829 ms for revision support, 2,501.643 ms for dependency support and
5,097.559 ms for the full digest-aware support gate. The old dependency function
materialized the Brain's entire exact/current-head edge set, even for a leaf.
The equivalent frontier query returned its single dependency in 0.079 ms before
deployment. These are diagnostic single observations, not throughput guarantees.
Migration 043 preserves both edge kinds, Brain scoping and recursive deduplication;
negative support roots short-circuit, while every positive rule, manifest,
privacy, dependency, cycle and digest condition remains.

Graph construction now computes the direct-authority set with the exact existing
decision/reviewer and both transition identity checks, plus current-policy positive
assessments. It then runs the full canonical gate on survivors once. Support,
count and contribution queries reuse that exact qualified set in the same locked
transaction. Publication epoch checks, input deadlines and graph-read canonical
revalidation remain. Assessment state changes still invalidate epochs and can
cancel a queued generation; reducing that churn is not part of this repair.
The retained diagnostic receipts include
`.cache/memory-first-batch-positive-support-profile.json`,
`.cache/memory-first-batch-dependency-frontier-profile.json` and
`.cache/memory-first-batch-post042-graph-phase.log`.

The first migration-043 observation had a real four-node/two-edge generation
and a successful centered two-node/one-edge read, but one overview failed.
A later six-node/three-edge overview succeeded while the centered read failed;
successful requests took roughly 3.3–5.0 seconds. This was remaining contention,
not reliable performance acceptance. Read-only profiling located about
4,104 ms in audit selection under the Brain lock. The current-root fast path
retains every admission gate and returns four in about 716 ms, with the complete
historical search used when current roots cannot fill the allowance. Local and
provider retry caps remain unchanged. This is best-effort priority, not an
oldest-first or historical-service-time guarantee. The diagnostic receipt is
`.cache/memory-first-batch-current-root-fast-profile.json`; final runtime
availability is recorded separately below.

Empty eligible output is not a performance win over displaying the old graph,
and missing verification is not permission to restore unchecked evidence. Normal
background verification remains subject to the existing OpenAI Brain policies,
daily allowances and concurrency. Those ordinary model calls are distinct from
the stopped isolated OpenRouter benchmark campaign; no new normal provider
policy was adopted by the update.

After the final scheduler image, metadata at `2026-10-08T23:11:27Z` showed
**71 succeeded audits: 16 supported and 55 insufficient**, eight failed audits
(six database errors, one lease expiry and one provider timeout), and four queued
assessments. Sixteen current roots had positive current-policy assessments. The
latest ready generation contained **23 nodes and 17 edges**. The actor-scoped
four-target predicate probe completed in 18.430 ms. These are an advancing,
partially verified inventory, not full legacy-backfill completion or model quality
proof. Receipt: `.cache/memory-first-batch-final-audit-proof.json`.

The final image's first authenticated windowed view failed with
`503/database_unavailable` in 3,033.95 ms and overview exploration failed with
`503/graph_timeout` in 15,005.10 ms. Two further bounded diagnostic pairs also
failed; the last pair took about 15 seconds per endpoint. No final centered read
was possible because neither endpoint supplied an eligible center. Earlier
successful centered reads remain intermediate observations, not final acceptance.
Receipts: `.cache/memory-first-batch-final-scheduler-graph-failure.json`,
`.cache/memory-first-batch-final-graph-phase-probe.log` and
`.cache/memory-first-batch-lock-metadata.log`.

Read-only phase sampling found advisory Brain-lock waits up to 9.679 seconds,
an active background source-fragment query up to 8.696 seconds, and a support
gate active for 2.994 seconds. The final graph candidate query itself requested
23 identities, returned 29 eligible fragments and completed in 900.603 ms. Its
plan still visited 1,125 claim-revision index entries to produce 16 selected
claims (about 322 ms); an independent review identified a per-logical-claim
latest-version lookup as a possible optimization, retaining the entire relevant
history and applying privacy/support filters only after latest selection. That
optimization was not implemented or measured. Existing appropriate indexes
already exist; this is not evidence to add an index blindly.

The candidate-query cost alone does not explain a 15-second request. Repeated
per-item evidence/support qualification and background transactions remain the
next profiling targets. The locks serialize truth/privacy changes with reads;
removing them or increasing timeouts is not the accepted repair. No speedup or
working large-Brain graph is claimed from health or generation counts alone.
Receipt: `.cache/memory-first-batch-final-sql-profile.log`.

The October 9 [reuse follow-up](../research/memory-product-improvement-plan-2026-10-08.md#graph-performance-reuse-follow-up--2026-10-09)
compares Cognee's bounded visualization, claude-mem's staged WAL-backed retrieval
and Graphify's cached graph serving. It records candidate read/preparation changes
and optional Graphify extraction separately from delivered code. That later lookup
did not change this runtime or establish that another system solves this workload.

## Paid diagnostic results and cost

Both diagnostics retain the original 50-question manifest, canonical histories,
GLM answers, Qwen embeddings and 16 KiB context; they are diagnostic repeats of
that sample. Neither is the separately frozen held-out sample. Extraction and
learning were OFF, so these runs cannot measure a learning benefit.

| Diagnostic | Case outcomes | Model requests | Known additional USD | Unknown reserved USD |
| --- | --- | ---: | ---: | ---: |
| DeepInfra FP4, `brain-answer-2` | 6 completed, 19 failed, 1 uncertain, 24 unattempted | 52 | 0.00286372475 | 1.90 |
| Separate frozen Novita FP8 route probe | 0 completed, 16 failed, 34 unattempted | 32 | 0.00001232 | 1.60 |

DeepInfra failed mainly with HTTP 429; an uncertain timeout stopped that arm.
Novita's 16 answer attempts failed through the native provider interface. The
saved endpoint quote did not advertise `structured_outputs`; the precise cause
was not proved because raw provider error content was not captured. The cheaper
quote does not establish compatibility with the required strict answer interface.
The production adapter was restored to the reviewed DeepInfra FP4 route, with
required parameters and no fallback, before deployment. Normal Brain policies
were not switched. The separate Novita transport experiment is not a quality
comparison.

The combined campaign ledger has **USD 0.22070654563093992 known billed**, plus
**USD 7.570710000000002 unresolved reservations**, for **USD 7.791416545630942
commitment against an USD 8 ceiling**. Only USD 0.00287604475 of known cost was
added by this batch. Unknown billing is not zero cost. The retained reconciliation
is `.cache/memory-first-batch-campaign-accounting.json`; no campaign requests
remain running. No further paid benchmark/provider-diagnostic calls or official
judge calls were issued after stopping. Ordinary authorized OpenAI background
work continued under its existing policies. Incomplete transport outcomes cannot
provide a new accuracy figure.

## Remaining work

First-batch acceptance is still open for reliable nonempty default overview and
centered graph reads during ordinary background verification. Profile the actual
long source/support statements, repeated per-item qualification and lock tenure,
then repair the measured work without changing canonical eligibility. Repeat
the same model-free live reads after the repair. A passing fixture or empty graph
cannot satisfy this acceptance.

Normal-Brain job history also contains unresolved learning failures for oversized
inputs, invalid input, provider response shape and exhausted budgets. The
intermediate metadata sample included 45 `source.learn/model_input_too_large`,
93 `source.learn/provider_shape` and 1,830
`semantic.generate/model_budget_exhausted` failures. These are accumulated
historical counts, not failures all caused by this update, and are separate from
the learning-OFF public benchmark. This batch does not clear that backlog or
raise allowances. Repair/recovery needs bounded input handling, compatibility
proof and budget-aware scheduling without blindly repeating paid work.

The proposed later phases remain separate: learning ON/OFF experiments with equal
budgets; engineering gold questions; controlled temporal/alias reconciliation;
automatic topic collection UX and topic scoring; vector-index/ANN and database
phase profiling; broader scale and failure scenarios; and a full Cognee graph
comparison. Current descriptors still load within a 64 MiB envelope, and complete
multi-hop/path APIs retain their existing bounds. This batch does not establish
million-document or graph-wide throughput.

The fresh held-out 50 questions remain frozen and unexecuted. Before claiming
answer improvement, obtain a complete reliable run and use the frozen official
judge/equal-model protocol; report availability separately from quality. Before
attributing a benefit to automatic learning, run the controlled ON/OFF arm. The
existing baseline had full supporting sessions retrieved in 23 of 30 wrong
answers, so learning absence alone does not explain the failures.

## Closeout

Runtime evidence and inventory reconciliation are recorded above. The code is
deployed locally on migration 043, but the owning pack remains **in-progress**
because final live graph performance acceptance failed. Focused native, browser,
recovery and update checks passed. Final `./scripts/validate.sh` passed governance,
32 governance regressions, corpus validation and 10 support-baseline checks;
`git diff --check` passed. Neither check establishes live graph or answer quality.
Later research phases are not delivered by this batch. Version: N/A (no release
policy). Commit: uncommitted. No commit,
push or remote deployment was performed.
