# Automatic mapping review, graph preparation and local extraction proof

Observed: 2026-10-09 (Europe/Stockholm)
Confidence: verified source and retained local test receipts; normal-runtime graph acceptance remains open
Status: implementation paused at user request; partial delivery, acceptance open

## Scope and method

The user requested two agents to identify improvements to automatic information
mapping, entity creation and grouping, and compare Cognee, Graphify and other
open-source components. The active implementation goal also covers the complete
[dedicated plan](../research/automatic-knowledge-mapping-plan-2026-10-09.md).
This record separates those reviews, the locally implemented concurrency
prerequisite and a standalone local model experiment. It is not authority or
evidence that the complete plan is delivered.

The product audit inspected current Rust, SQL, protocol, privacy and UI seams.
The reuse audit checked current upstream source and the local prototype. Both
agents were read-only and made no model/provider calls. The coordinator repaired
test fixtures through existing production review handlers, ran native tests and
ran the extractor on authored synthetic development text. Reference checkouts,
normal Brain model policies and the 10 held-out extraction cases were preserved.

## User-requested pause and publication checkpoint

On 2026-10-09 the user paused implementation and requested a Git commit/push
of the existing work. The unfinished execution packs remain active; publication
of this checkpoint does not satisfy their remaining acceptance criteria.

Delivered changes include the OpenRouter provider/catalogue and budgeted
benchmark tooling, full-source answer evidence and history handling, semantic
discovery query repair, graph preparation outside long Brain locks, bounded
support-audit discovery, a checkpointed local updater, and repo-local Rust
Analyzer MCP/Graft navigation. The organization work delivers typed candidate
validation and persistence, with direct-source privacy controls; an automatic
producer and qualified entity/topic navigation are still absent.

The [benchmark report](openrouter-benchmark-proof-2026-10-08.md) retains all 50
recovered LongMemEval answers, 40% official overall accuracy, the judge audit,
and known versus unresolved billing. No comparable answer-quality benchmark
has been rerun after the later performance repairs.

The latest normal graph measurement remains **35/40 successful, p95 4.027
seconds**, while the 10k-node capture-only control passed **40/40, p95 0.513
seconds**. Digest/handover writer tenure and the full forced-audit/rebuild
workload remain open. Automatic aliases, overlapping topics, overrides,
incremental correction/erasure and their APIs/UI remain unfinished. Ancestral
excerpt model-input-fence cleanup remains a known privacy gap; automatic mapping
production is not enabled. The GLiNER experiment failed its extractor quality
gate, so it is not a selected production adapter.

The retained local runtime is migration 047 with build revision
`27bbd00f006e9684b25f18b47abcfc3110f22806-dirty`, built at
`2026-10-09T13:57:36Z`. Its checkpoint preserved the exact canonical inventory.
Committing/pushing this work does not rebuild or deploy that image. Earlier
references to uncommitted changes describe the state at their measurement time.

Publication checks passed: Rust formatting and workspace/all-target compilation,
73 Rust library tests (three native tests ignored), the Python script suite
(23 passed, one skipped), web design/typechecking/build, generation of 184 unique
OpenAPI operations and documentation validation (35 governance tests plus ten
support-baseline tests). Credential review found no new credential patterns and
no literal local secrets in the publication files. Receipts are the repository's
ignored `.cache/pause-publication-*.log` files. These checks supplement the dated
native/browser evidence below; they do not replace the failing normal graph
acceptance or establish a new quality score. No paid benchmark or deployment
was performed for this publication.

## What the two reviews establish

- Developer CodeGraph indexes code symbols, callers and dependencies. Enola is
  the existing product repository extraction adapter. Neither supplies automatic
  semantic organization of all Brain conversations and documents.
- The knowledge descriptor still describes claims, support and contributions.
  Claim subject/predicate/value strings do not have canonical semantic entity
  identities. Stable aliases, automatic topic records and memberships remain
  unimplemented. Source: [descriptor](../../crates/server/src/graph/descriptor.rs),
  [learning](../../crates/server/src/learning.rs).
- Automatic assignments need separate storage. The existing source organization
  endpoint replaces manual memberships; using it for recomputation would erase
  user choices. Source: [evidence](../../crates/server/src/evidence.rs).
- New semantic nodes need a declared protocol and canonical qualification.
  Existing `GraphNode` wraps `RecallItem`, whose graph candidate path handles
  claims, sources, repository facts and manifests. Extra Neo4j labels alone
  cannot make entities safely readable. Sources: [protocol](../../crates/protocol/src/graph.rs),
  [qualification](../../crates/server/src/retrieval/graph.rs).
- Correction and erasure require exact derivative closure, physical cleanup and
  restore-journal replay. Epoch invalidation alone does not remove derived names
  or prevent old-backup resurrection. Independently supported entities must
  survive with recomputed labels/counts. Sources: [privacy](../../crates/server/src/privacy.rs),
  [journal](../../crates/server/src/privacy_journal.rs).

The coherent successor is typed source-linked mentions, scoped entity identities,
reversible aliases, separate overlapping topics with durable overrides, an
idempotent incremental worker and bounded topic/entity APIs and UI. It must not
fabricate a source mention for a paraphrased claim subject or represent an entity
as an accepted claim.

## Reuse result

| Component | Useful seam | Limit |
| --- | --- | --- |
| GLiNER2.5 | Local schema-driven entity spans and classification candidates | Raw domain typing failed the development precision gate below; not an embedder or canonical truth authority |
| Cognee | Extraction-derived summaries, entity/chunk associations, shared-entity scoring and incremental placement | NodeSets are supplied tags; optional automatic context indexing is separate; whole-population processing still needs measurement |
| Graphify | Versioned code producer and deterministic community labels/stable matching | Compare against Enola on the same committed fixture; it does not repair Brain lock contention |
| Graphiti | Candidate temporal/entity-resolution components | Full pipeline has additional model operations; local NER alone does not make the whole pipeline free |

Fresh primary-source pins remained Cognee
`0ec7a9fa61c9ff04bf7e02e0d57af363a993a5e1`, Graphify
`5b74d7d74911cf435c8f1636b6f96ea202cc6246` and Graphiti
`1026ae7ae25e7e4cfcc1c7ebe00347b8a90f52a0`.
See the dedicated plan for exact source and license links. The coordinator also
read the [GLiNER model card](https://huggingface.co/fastino/gliner2.5-base-v1),
[Cognee classification source](https://github.com/topoteretes/cognee/blob/0ec7a9fa61c9ff04bf7e02e0d57af363a993a5e1/cognee/tasks/documents/classify_documents.py)
and [Graphify clustering source](https://github.com/Graphify-Labs/graphify/blob/5b74d7d74911cf435c8f1636b6f96ea202cc6246/graphify/cluster.py).

## Locally implemented graph preparation

The accepted [concurrent preparation contract](../contracts/graph-concurrent-preparation.md)
and active [pack](../roadmap/execution/active/graph-concurrent-preparation.md)
govern migration 044 and the uncommitted Rust changes. Provisional read-only
snapshots prepare graph reads and worker descriptors without holding Brain
locks through native I/O. Fresh authenticated publication rechecks role,
session/device, scope, epoch and deadlines under the fair canonical lock.

Retained checks:

| Check | Observed result |
| --- | --- |
| Read-only snapshot versus same-Brain writer; mutable/exclusive bypass controls | Passed actual PostgreSQL test; zero model requests |
| Pause actual Neo4j work during view/explore/path, change generation, resume | Passed; writer commits while preparation is paused, buffered output is discarded, fresh read succeeds |
| Revoke browser session during actual native preparation | Passed; final response is unauthorized |
| Native graph suite | Initial run: 35 passed, 3 failed out of 38 |
| Three failure reruns | All 3 passed after fixture/configuration repairs; not a new complete-suite run |
| Formatting, focused server compile and documentation validation | Passed |

Two older fixtures expected unassessed manual proposals to enter the knowledge
graph. Source-support-3 requires authority, so they now use explicit fixture
review through real handlers. The conflict-expiry test uses explicit review
correction to retain its contradictory peer; ordinary acceptance refuses that
conflict. Its conflict-withholding, clock-expiry and positive-control assertions
remain. The recovery test needed the owned SFTP fixture configured. That exact
fixture was created, proved connected and removed after the rerun, with ownership
labels and bind mount checked first. No unrelated container was removed.

Receipts remain under `.cache/automatic-mapping/`: native graph suite,
`native-regression-repairs.log`, `governance-validation.log`, recovery setup and
cleanup records. The preparation tests also retain
`.cache/automatic-mapping-graph-preparation-tests.log`.

Those initial receipts did not establish normal-installation repair. The
expanded checks below replace the earlier small-fixture performance evidence;
the normal-installation result remains a separate gate. The complete mapping
goal and both performance packs remain open.

## Expanded graph and Rust candidate implementation

The populated test materializes 10,000 repository facts and 9,999 native edges
through production publication/projection handlers. It runs 40 bounded view and
one-hop neighborhood requests with concurrency two while actual source capture
and capture-lane processing continue. Every response is counted, including errors.

The first completed workload failed **40/40 reads** with
`graph_preparation_changed` while 26 captures succeeded. The Brain-wide analytics
epoch invalidated the selected immutable repository window for unrelated capture
changes. A prior setup run also failed because its fixture looked for node 5,000
inside a 200-item page; that setup failure was not a latency sample.

The accepted contract now permits explicitly partial windows to requalify every
buffered entity in the fresh canonical transaction when the Brain epoch advances.
Exact ready-generation metadata and descriptor membership, complete hydrated
payload equality, current scope identity and deadlines must still match. This
performs no native I/O under the final lock. Full path/absence, analytics and worker
publication keep their stricter epoch boundary. An actual collection-membership
removal during paused native work rejects the entire buffered response; restoring
membership makes the fresh two-node control readable again.

| Expanded check | Result | Retained receipt |
| --- | --- | --- |
| Complete current native graph suite, including owned connected SFTP recovery | PASS, 40/40 | `.cache/automatic-mapping/native-graph-complete.log` |
| 10k-node concurrent view/neighborhood reads during capture | PASS, 40/40; p95 0.513 s; 32 captures, zero capture failures; zero model requests | `.cache/automatic-mapping/graph-10k-concurrency.json` |
| Exact UTF-8 spans, malformed/foreign/duplicate/unbounded input and endpoint/kind gates | PASS, three Rust test groups | `.cache/automatic-mapping/native-candidates.log` |
| Actual PostgreSQL candidate staging, original IDs, cross-Brain/unknown-instance separation, atomic rejection, replay, pending aliases and final erasure/fence/session gates | PASS | `.cache/automatic-mapping/native-mapping-storage-latest.log` |
| Actual older PostgreSQL copy plus journal replay physically removes candidate payloads, mentions, relationships and orphan names/contexts; independent support and manual membership survive | PASS | same storage receipt |
| Populated Brain deletion plus catalog coverage | PASS, 2/2 | `.cache/automatic-mapping/mapping-brain-deletion-verified.log` |
| Real Rust Analyzer MCP resolves the new cross-module candidate call; Graft MCP reads the new file API and verifies freshness | PASS; no paid requests | `.cache/rust-navigation/native-mapping-navigation.log` |

Migration 045 and the new
[Rust candidate module](../../crates/server/src/knowledge_mapping.rs) and
[staging implementation](../../crates/server/src/knowledge_mapping/storage.rs)
persist source-bound candidate batches, conservative entity identities, exact
mentions and pending relationship/alias records. Preparation reads immutable
retained text outside Brain locks; final staging reauthenticates the browser and
checks writer authority, exact current source, epoch, input fences and deadlines.
Repeated identical input/context/adapter batches reuse the same IDs. Context JSON
is resolved under the writer boundary rather than indexed as an unbounded key.
Aliases do not destructively combine original entities. There is no candidate
retrieval/topic API or automatic producer selection yet.

The privacy manifest records exact mapping inputs; cleanup removes their raw
payloads and quoted descendants and rederives survivors by retaining independent
mentions. Restore replay also matches canonical source/model-input dependencies
when an older manifest lacks the new field. Full privacy acceptance additionally
requires ancestral excerpt-fence and future worker/topic/override coverage; it is
not established by these direct-source controls alone.

The broad audit found migration 035's cascading `agent_brain_usage` and
`source_import_scopes` missing from the explicit deletion inventory. Migration
045 adds them. The original deletion fixture also lacked later support/digest
rows. It now populates every listed table and retains the strict coverage and
content-marker checks. These direct SQL teardown rows do not prove model or
semantic publication quality. The staged-source test separately exercises the
authenticated Rust boundary against real imported text.

Failed checks were retained: one graph run lacked the required SFTP environment;
the complete rerun uses an ownership-verified connected fixture. Early staging
controls used incorrect hand-counted Unicode offsets and an invalid fake privacy
request ID; the fixtures were corrected without weakening production validation.
Rich deletion fixture repairs satisfy the existing immutable excerpt/digest/run
constraints. Exact owned failed setup databases and the new SFTP fixture were
removed with identity/ownership checks; unrelated resources were preserved.

The 10k result covers capture concurrency, not simultaneous audits/rebuilds or
the normal Brain. Extractor precision/recall, qualified semantic publication,
automatic processing, reversible alias decisions, topic baselines/overrides,
typed navigation/UI and their complete correction/privacy lifecycle remain open.
No new benchmark score or automatic-organization superiority is claimed.

## Normal installation and exact dependency queries

The checkpointed local updater completed migration 045 with API/worker build
`27bbd00f006e9684b25f18b47abcfc3110f22806-dirty`, built at
`2026-10-09T12:07:05Z`. Its encrypted checkpoint is
`8f835cf0-3756-47c7-8a8c-b6ee560c3c73` (local, not an off-machine copy).
Exact pre/post ID comparison preserved all 17 Brains, 1,232 sources, 1,633 claims
and the one repository snapshot; no missing or added IDs in those tables.

The previously failing normal Brain still failed after that update. Repeated
model-free view/explore requests returned `graph_timeout` or
`database_unavailable` after approximately 2–15 seconds. Actual PostgreSQL
sampling observed active canonical work and advisory-lock waits; it did not
establish that all time was spent waiting for locks. Query text, source bodies
and credentials were excluded from the profiling receipts.

Parameterized app-role/RLS probes located a repeated canonical dependency
deadline lookup taking 1.344–1.841 seconds for a single revision. Migration 046
resolves exact dependency/revision/support/manifest identities with parameterized
lateral lookups before metadata joins. Support sets and selected manifest
entries remain complete. The pre-digest gate also avoids rechecking the already
checked root's full positive support, while retaining its cycle check and every
exact/current-head descendant check. Graph requested-identity lookups use the
same bounded immutable-identity principle and retain complete logical histories.

A unique, uncommitted function probe on the actual normal Brain evaluated the
new deadline in 4.0–4.5 ms and returned the identical canonical deadline within
the same transaction. The transaction was rolled back and the probe function's
absence was verified. A similarly rolled-back complete graph-candidate probe
returned 67 representations from the 56-node descriptor window in 0.698 seconds;
earlier canonical probes took approximately 1.867–2.181 seconds. These are query
measurements under changing background work, not an end-to-end graph p95 claim.

The Rust integration control compares old and new canonical functions through
the actual app role. It covers exact and current-head contributors, finite claim
retention, independently retained reviewed claims after support expiry and
foreign-Brain RLS. All comparisons pass with zero model requests. The complete
native graph rerun passes **41/41** after migration 046, including the new
deadline test. Its 10k capture workload passes **40/40** reads at p95 **0.519 s**
with **32** successful captures and zero model requests. Five additional support
controls pass: scope/rules/half-open validity, current contributor heads and
reference-only review, dependency expiry/cache invalidation, cyclic lineage,
and selected manifest lifetimes/missing/erased inputs. They also compare the
support predicates with the accepted earlier SQL where applicable.

Receipts: `.cache/automatic-mapping/normal-inventory-comparison-045.json`,
`normal-phase-probe-045.log`, `normal-detailed-phase-probe-045.log`,
`normal-sql-profile-045.json`, `normal-positive-helper-profile-045.json`,
`normal-deadline-rollback-probe-046.json`, `normal-candidates-rollback-046.json`,
`exact-deadlines-controls-latest.log`, `native-graph-046-complete.log`,
`graph-10k-concurrency-046.json` and the five `exact-support-*-controls.log` files.
The newly owned SFTP fixture and the exact remaining failed deletion databases
were removed after verified ownership; receipts retain that cleanup evidence.

The checkpointed local update to migration 046 completed. API/worker readiness
reports build `27bbd00f006e9684b25f18b47abcfc3110f22806-dirty`, built at
`2026-10-09T12:36:55Z`, with encrypted local checkpoint
`515f90e6-15f0-4ed3-834d-574cd0012fe4`. The second exact ID comparison again
preserves all 17 Brains, 1,232 sources, 1,633 claims and one repository snapshot.

Two ordinary-worker-on probe rounds on the previously failing Brain produced:

| Read | First round | Second round |
| --- | --- | --- |
| Windowed view | 503 `database_unavailable`, 3.638 s | 200, 56 nodes, 4.127 s |
| Windowed exploration without a center | 200, 56 nodes / 45 edges, 4.786 s | 200, 56 nodes / 45 edges, 2.349 s |
| Centered one-hop exploration | 200, 2 nodes / 1 edge, 0.737 s | 200, 2 nodes / 1 edge, 0.271 s |

Both successful overview rounds honestly report partial coverage because the
window uses an older knowledge generation. They do not prove absence or a
complete current graph. Failed outcomes remain in the record; these six
sequential outcomes do not establish p95 acceptance. Sanitized PostgreSQL error
classification identifies statement timeouts in `recollect_lock_brain` during
the graph attempts. Background source-fragment/support-discovery work also
reaches its ten-second statement budget. Remaining background lock tenure and
overview hydration therefore still need repair; the normal performance gate
does not pass merely because centered reads now work.

The first round's results are retained in this table; the second is in
`normal-graph-proof-046.json` and `normal-detailed-phase-probe-046.log`. Additional
receipts: `normal-update-046.log`, `normal-inventory-comparison-046.json`,
`normal-timeout-phases-046.json`. Queries/values are not printed by the sanitized
error classifier. Automatic publication/topics, populated audit/rebuild
acceptance and ancestral excerpt model-input-fence closure remain required.

## Semantic discovery and Rust navigation follow-up

The user requested Rust Analyzer MCP plus Graft. Both repository-local servers
pass actual stdio MCP initialization and navigation. The Rust proof now resolves
the semantic queue's `representation` call to the Rust implementation, in addition
to graph qualification and mapping preparation. Graft file API and nested-directory
CLI freshness pass. These are developer tools; a fresh host session is needed to
load the registrations into Codex's current tool catalog. No deep summaries,
paid provider requests or home configuration changes were used. See the
[tooling proof](rust-semantic-navigation-proof-2026-10-09.md).

The remaining normal lock trace includes semantic discovery. Its former query
applied expensive canonical predicates to populated source/revision relations
before excluding identities already represented in the active profile. Rust-owned
SQL now materializes exact missing-input cohorts first and resolves metadata by
parameterized primary-key lookup. Canonical scope, source, support, retention,
manifest and input-fence predicates still qualify every missing input. All seven
entry states exclude duplicate discovery; new profiles and current revisions do
not inherit old-profile exclusion. The 100-input limit follows qualification.

Normal-data app-role read-only measurements on the same Brain:

| Probe | Previous query | Revised query | Outcome |
| --- | --- | --- | --- |
| Initial ten-second statement limit | Timeout, 10.053 s wall | 225.003 ms SQL | Real failure retained |
| Thirty-second limit, first measurement | 19,638.714 ms SQL | 334.499 ms SQL | Both completed |
| Subsequent measurement | 17,739.522 ms SQL | 227.311 ms SQL | Both completed |
| Same repeatable-read snapshot, actual 100-input consumer window | 8 IDs | 8 IDs | Symmetric difference zero |

These observations include ordinary background activity; they are query-plan
measurements, not controlled graph-request p95 or a general performance claim.
The revised plan qualifies 47 missing source-chunk identities and 50 missing
claim identities through exact metadata lookups rather than requalifying the
entire relations. No persistent eligibility cache or production mutation was
introduced by the probes.

The nine semantic controls passed initially. The broader thirteen-control run
exposed five failures, and all five reproduced against the exact previous SQL
and both previous consumers; the temporary counterfactual restored the current
source bytes exactly. Four positive fixtures submitted unreviewed proposals that
the accepted source-support contract excludes from usable embeddings/graphs.
They now perform real authenticated review commands. One older expectation
allowed a corrected historical claim into model-facing recall: the updated
control verifies canonical history inspection remains available while current
support/correction prevents that claim entering recall. Product gates were not
relaxed. Reviewed surviving representations become graph anchors, so they receive
no extra graph vote; the frozen-query test still excludes late knowledge and
proves a fresh query sees it.

All **22** semantic controls pass across focused runs: nine semantic cases,
twelve corrected recovery cases and the remaining corrected frozen-graph case.
The strengthened cohort test additionally creates **101** earlier unavailable
claims and proves valid later inputs survive, all-state exclusion, missing/new
profile behavior, current-head change, rejection and foreign-Brain RLS. It makes
zero model requests. Other semantic controls use only the local fake provider;
no paid model or LLM judge was used. The exact failed disposable database/native
resources are cleaned after ownership verification.

Receipts: `.cache/automatic-mapping/normal-semantic-discovery-probe-046*.json`,
`normal-semantic-discovery-probe-046*.log`, `native-semantic-discovery-cohorts.log`,
`native-semantic-discovery-regression.log`, `native-semantic-discovery-recovery*.log`,
`native-semantic-frozen-graph-current.log`, `semantic-recovery-counterfactual.json`,
`semantic-failed-fixture-cleanup.json` and
`.cache/rust-navigation/semantic-discovery-navigation.log`.

The checkpointed normal update completed with build time
`2026-10-09T13:07:42Z`, migration `046_exact_memory_deadlines`, API/worker readiness
and encrypted local checkpoint `04fc5bcb-362f-4a31-9e28-3ce02b188229`.
Exact before/after identities preserve all **17 Brains, 1,232 sources, 1,633 claims
and one repository snapshot**, with no added or missing IDs. This is the local
dirty/uncommitted image, not a published release.

The first ordinary-worker-on graph round succeeds for view/exploration/centered
exploration at **1.854 / 1.505 / 0.229 seconds**, respectively. It returns 56 nodes
and the exploration returns 45 edges; coverage still declares `graph_window`.
The longer **40-read sequential series passes 37/40**, with **p95 3.520 seconds**
including every outcome, and a maximum of **4.389 seconds**. It retains:

| Ordinal | Read | Result | Time |
| --- | --- | --- | --- |
| 33 | View | 503 `database_unavailable` | 3.520 s |
| 34 | Uncentered exploration | 503 `database_unavailable` | 4.098 s |
| 35 | Centered exploration | 503 `database_unavailable` | 2.214 s |

This does **not** satisfy reliable normal-runtime graph acceptance. Sanitized
PostgreSQL logs show admission-lock timeouts and ongoing support-audit discovery
statements containing exact dependency/acyclic/current-support checks. The next
repair seam is `memory_support_audit::enqueue`, whose current and historical
metadata selection still needs measured query-plan/lock-tenure repair. This
classification identifies the remaining statement family; the parser alone is
not a complete per-request blocker attribution. Forced 10k audit/rebuild
concurrency and the full automatic-organization goal remain open.

Additional receipts: `normal-update-semantic.log`,
`normal-inventory-comparison-semantic.json`, `normal-graph-proof-semantic.json`,
`normal-graph-series-semantic.json` and `normal-timeout-phases-semantic.json`.

## Bounded support-audit discovery follow-up

The pure missing-identity SQL rewrite passes native current/historical,
policy/verifier, every-recorded-state, RLS and bottom-up/cycle controls. It is
insufficient on the normal Brain: both old and rewritten complete selection
statements exceed a 30-second read-only deadline. The missing current cohort
contains **914 revisions**. Checking one small candidate's reviewed, exact
dependency and cycle helpers individually takes 86–106 ms including separate
`psql` processes; that sample is not a full-cohort throughput measurement.

The repair therefore changes scheduling structure in Rust. Migration 047 stores
only Brain/policy/verifier-scoped UUID scan progress and a monotonic version,
with writer RLS and canonical Brain deletion inventory. Preparation examines
at most 16 missing immutable identities without holding Brain admission locks.
It starts no additional qualification after two seconds or four positives;
an in-flight complete statement has a one-second limit. Unavailable or timed-out
candidates remain unresolved and are revisited after the walk wraps. No exact
dependency closure is truncated and no timeout becomes a verdict.

A fresh writer transaction compares standing policy and cursor version and fully
requalifies at most four candidates, using a 750 ms statement deadline and a
one-second budget for starting further qualification. Existing capacity,
assessment exclusion, native recovery, provider retry and budget rules remain.
Audit progress commits independently from later learning/digest maintenance.
An unrelated epoch advance can proceed because fresh complete eligibility is
decisive; a prepared Boolean is never sufficient authority.

The first read-only normal probe performs four passes. It examines **5 / 2 / 2 /
2 identities**, with **2 timeouts in each pass** and **one eligible candidate in
the first pass**. Wall times including separate `psql` processes are **2.768 /
2.154 / 2.197 / 2.187 seconds**. This proves bounded progress past difficult rows,
not a claim that their underlying qualification is now fast. Normal cursor state
and provider requests are untouched by this probe.

Native production-path controls pass for a **35-record unavailable prefix**,
restart/wrap, competing preparations, complete fresh qualification after review,
policy change, expiry, erasure and account disablement. A native gate pauses the
actual preparation statement while a same-Brain writer commits within 500 ms.
An actual statement timeout advances progress without creating an assessment;
the same candidate is admitted after wrap when the gate is released. The existing
recorded-request/lease/native recovery, retained-verdict, inflight erasure,
older-backup and exact historical/cycle controls also pass. The populated Brain
deletion and catalog invariant include the new cursor table and pass. These use
local controlled provider fixtures; no paid provider or LLM judge is used.

Rust Analyzer MCP resolves the scheduler's call into
`autonomous/support_discovery.rs`; Graft inspects the new Rust prepare/publish
API and reports fresh structural data. The actual stdio MCP proof and nested CLI
proof pass, alongside isolation checks. Current Codex tool-catalog exposure still
requires a fresh session. Documentation validation passes. Four exact failed
development fixture databases/native identities were removed only after verifying
their harness ownership; the failures were a wrong test cursor start and an
incorrect erasure request shape, plus the prior savepoint/policy-order controls.

Relevant source: [Rust discovery](../../crates/server/src/autonomous/support_discovery.rs),
[qualification](../../crates/server/src/memory_support_audit/qualify.sql),
[migration](../../crates/server/migrations/047_bounded_support_discovery.sql).
PostgreSQL's [SET semantics](https://www.postgresql.org/docs/18/sql-set.html)
confirm that rollback to an earlier savepoint cancels transaction-local timeout
changes; [savepoint rollback](https://www.postgresql.org/docs/18/sql-rollback-to.html)
allows continuing the enclosing transaction. Context7 returned these official
references successfully on October 9.

Receipts: `.cache/automatic-mapping/normal-audit-discovery-probe-046.json`,
`normal-audit-helper-probe-046.json`, `normal-audit-bounded-probe-046.json`,
`native-support-discovery-bounded.log`, `native-support-audit-bounded-regression.log`,
`native-support-discovery-lineage-bounded.log`, `native-support-discovery-direct.log`,
`native-support-discovery-deletion.log`, `support-discovery-failed-fixture-cleanup.json`,
`support-discovery-validation.log` and
`.cache/rust-navigation/support-discovery-navigation.log`.

The checkpointed local update completed with build time
`2026-10-09T13:57:36Z`, migration `047_bounded_support_discovery`, API/worker
readiness and local encrypted checkpoint `b62691a2-9b5b-4789-9100-a30309be0344`.
Exact before/after IDs preserve **17 Brains, 1,232 sources, 1,633 claims and one
repository snapshot**, with no added or missing identities. The image is the
dirty/uncommitted checkout, not a published release.

The new **40-read normal series passes only 35/40**, with **p95 4.027 seconds**
including all outcomes and maximum **4.396 seconds**. Successful uncentered
exploration returns 56 nodes / 45 edges; centered exploration returns two nodes /
one edge. Coverage remains explicitly partial with `graph_window`. Every failure
is retained (ordinals are zero-based):

| Ordinal | Read | Result | Time |
| --- | --- | --- | --- |
| 22 | Uncentered exploration | 503 `database_unavailable` | 4.121 s |
| 23 | Centered exploration | 503 `database_unavailable` | 2.258 s |
| 24 | View | 503 `database_unavailable` | 4.027 s |
| 25 | Uncentered exploration | 503 `database_unavailable` | 4.396 s |
| 26 | Centered exploration | 503 `database_unavailable` | 2.196 s |

This fails runtime performance acceptance. The previous 37/40 and current 35/40
series both have ambient ordinary worker activity; they are not a controlled
same-work speedup or causal regression measurement. No fresh memory benchmark
accuracy is established by this repair.

A subsequent payload-free PostgreSQL blocker sampler records PID, query hash,
statement/transaction ages and `pg_blocking_pids`. In 60 samples, **58** contain
blocked sessions. It observes **23.637 seconds** of cumulative transaction age
in the broadly classified digest family, with at most **2.143 seconds** in one
currently sampled statement. Hashes resolve against exact Rust SQL literals to
the session-digest candidate query and handover-refresh candidate query; no
source/query payload is printed. Thus the generic `digest` classification also
includes handover refresh and cannot be treated as a single function attribution.

A second sampler joins advisory Brain admission keys to canonical Brain IDs.
It observes the **same tested Brain** holding admission during digest/refresh
maintenance, with transaction age reaching **16.247 seconds**, and seven sampled
waiters requesting that Brain. The oldest sample's hash resolves to
`handovers.rs`'s refresh candidate query. These are post-series blocker observations,
not an exact trace joining all five graph failures to individual statements.
They establish the next concrete repair seam: one writer transaction accumulates
repeated partition qualification/preflight and handover discovery. Bounded audit
discovery alone does not shorten that remaining transaction.

Current native totals are **10 support/audit controls, two Brain-deletion controls
and four graph preparation controls**, all passing. The current 10k capture-only
workload again returns **40/40**, p95 **0.513 seconds**, zero model requests.
Full forced-audit/rebuild concurrency, ancestral excerpt model-input-fence closure
and automatic entity/alias/topic publication remain open. Next implementation
must shorten digest/handover writer tenure while retaining complete supported
contributors, exact generation identity, current policy, privacy and deadlines.

Additional receipts: `normal-update-support-047.log`,
`normal-inventory-comparison-support-047.json`, `normal-graph-series-support-047.json`,
`normal-writer-blockers-047.json`, `normal-writer-blockers-047-brain.json`,
`native-graph-preparation-support-047.log`, `graph-10k-concurrency-support-047.json`
and `support-discovery-validation-final.log`.

## Local GLiNER development experiment

An isolated repository-owned Python environment installed `gliner2[local]==2.0.0`
and PyTorch 2.14.1. The initial load failed because the tokenizer compatibility
path lacked `protobuf`; `protobuf` and `sentencepiece` were installed and model
loading/inference then succeeded. No global package environment was changed.
The checkpoint is `fastino/gliner2.5-base-v1` at
`54785d51df8d86a0d8f2eb212fec15871aad2c4d`, an Apache-2.0 English model.

The [probe](../../scripts/probe-local-mapping.py) used CPU on the Mac, Python
3.12.14, two Torch threads, threshold 0.5 and the frozen schema. The
[40-document fixture](../research/automatic-knowledge-mapping-fixtures-2026-10-09.json)
contains 30 development and 10 held-out cases, all English and ASCII, maximum
93 characters. Only the 30 development cases were inferred. Exact typed spans
are scored against authored gold annotations, with no LLM judge.

| Measurement | Development result |
| --- | --- |
| Gold mentions | 79 |
| Correct typed spans | 62 |
| Incorrect extra typed spans | 25 |
| Missed typed spans | 17 |
| Precision | 71.26% |
| Recall | 78.48% |
| Per-document p95 inference | 44.1 ms |
| Model loading | 4.29 seconds |
| Paid inference/provider cost | $0 |

The run fails the plan's 95% precision and 80% recall gates. Several software
names were labeled as people, services or environments; other entities had
multiple incompatible labels. High confidence did not prevent wrong typing.
This rules out treating raw confidence as sufficient entity-resolution authority.
Compare improved schema definitions and deterministic canonical metadata with
the raw baseline using development cases before the untouched final subset.

These are short toy NER measurements, not product throughput, long-document,
alias, relation, grouping, multilingual or answer-quality acceptance. Topics and
Brain labels are annotated but the current probe does not score them. No
customer material or hosted inference service was used. The model was downloaded
from its public repository. Offline environment flags are requested after load;
they are not a network sandbox or proof of enforced offline deployment.

The review identified a coordinate mismatch: GLiNER returns half-open Python
character offsets, while canonical `SourceSpan` uses UTF-8 bytes. The probe now
rejects boolean/negative/empty/out-of-range offsets and converts/revalidates UTF-8
byte spans. A separate `--check-only` emoji/accented-prefix control passed. That
conversion change was made after the retained inference receipt; the receipt
above describes the earlier character-span scoring run. Actual Unicode model
extraction and product import remain untested.

## Next acceptance work

1. Finish populated concurrent graph and normal-runtime proof; preserve all
   stale/access/retention controls and count failures in latency acceptance.
2. Complete the persisted end-to-end mapping slice under accepted ADR 0020 and
   the organization contract; resolve an optional local-adapter runtime ADR
   before enabling such an adapter. Keep manual groups
   separate and declare coordinate/input identity and derivative erasure closure.
3. Extend fixtures with relation endpoints, explicit aliases, DEV/PROD and
   cross-Brain controls, Unicode and longer inputs. Independently score topics,
   overlap, idempotency, correction/erasure and stable user overrides.
4. Compare Graphify and Enola on identical committed repository inputs before
   adopting another code producer. No benchmark superiority is established.

Version: N/A — local development and research; no release requested.
Commit: user-requested publication checkpoint; identify the actual revision
from Git. The deployed build identity is recorded separately above.
