# Graph analytics interface evidence — 2026-09-15

## Method and confidence

Read the clean Cognee checkout at `c0d18c80e24b7b78918e7642c03f6f128fdd2aee`,
the local Atlas patterns and current primary Neo4j documentation. The existing
user-requested reference agent independently cross-referenced scope, correction,
metrics and lifecycle behavior. References were not modified. Actual local
Neo4j Query API calls used synthetic UUID-owned data with cleanup in `finally`;
no customer source read, external write or model call was needed.

Installed metadata in `.cache/analytics-proof/interfaces.json` identifies GDS
2026.08.1 on the selected Neo4j 2026.08 distribution. Context7 resolved
`/neo4j/graph-data-science` but returned 2.13 material. Current official pages and
installed calls supersede that version-specific evidence. An initial web open of
the memory-estimation page failed; the reference agent subsequently read it.
These are interface probes, not shipped-product acceptance or capacity benchmarks.

## Reuse and limits

Cognee `neo4j_driver/adapter.py:2264–2361` uses native GDS projection, with a
fixed `myGraph` name, all labels/types and undirected edges. Its metrics helpers
call WCC, all-pairs shortest paths and local clustering coefficient. Reuse GDS,
not that unscoped projection or unbounded all-pairs collection. Its pipeline-run
metrics cache is keyed by run/completeness, not current correction eligibility.
PageRank and Leiden recipes here come from official GDS, not that Cognee helper.
See the broader [reuse review](graph-reuse-review-2026-09-15.md).

Atlas `content/patterns/evidence-before-belief.md:70` warns that reconstruction
can undo correction without correction state in its inputs. Its
`scope-as-a-first-class-key.md:228` reports an aggregate concerning an allowed
entity that still contains out-of-scope contributions. These are research
observations, not independently benchmarked implementations. Recollect's
accepted foundation requires complete aggregate invalidation/recomputation.

The second reference pass found two canonical subtleties: source materialization
changes chunk completeness without advancing `memory_epoch`
(`evidence.rs:1189–1208`), and expiration of a withheld conflicting claim can
admit a surviving claim (`memory_rules.rs:106`). The contract therefore uses a
separate analytics epoch plus complete-set requalification and a conservative
Brain retention watermark. Attempt cleanup uses the existing privacy sequence
recorded under the Brain lock, avoiding wall-clock ordering assumptions.

## Executed native probes

`.cache/analytics-proof/probe.py`, `probe.json` and `probe.log` prove:

- Aggregation function `gds.graph.project` accepts qualified integer identities;
  null targets preserve isolates. A four-node triangle plus isolate projects
  three directed or six undirected relationships.
- PageRank stream with the fixed recipe returns `1 - 0.85^50` for the three
  cycle vertices and `0.15` for the isolate. An initial assertion incorrectly
  expected the converged value 1; that failure and successful cleanup are retained
  in `probe-initial.json`. The corrected check is independent finite-iteration
  arithmetic. No convergence claim is made.
- WCC separates the isolate; seeded Leiden on the undirected graph does too.
- Native projection and algorithm memory estimates return numeric byte bounds.
  The 10,000-node/100,000-relationship projection estimate was 1,367,272 bytes;
  this one observation is not a promise for other topology/configuration.
- Concurrent Query API transaction DELETE returns HTTP 400
  `TransactionAccessedConcurrently`. `SHOW TRANSACTIONS` filtered by owned
  metadata plus `TERMINATE TRANSACTIONS` stops the executing transaction.
- `gds.graph.drop(name,false)` on a missing graph succeeds with zero rows.

`.cache/analytics-proof/guard-probe.py`, `guard-probe.json` and its log prove:

- Cypher 25 `WHEN` inside a `CALL` subquery executes the projection only in the
  open branch under Recollect's existing Brain guard. After the exact attempt
  fence is set and the graph dropped, a delayed retry returns null/zero counts
  and creates no catalog graph.
- A synthetic 10,000-node/50,000-edge PageRank appears in `gds.listProgress`;
  terminating its exact metadata-owned transaction produces a native GDS
  `TerminatedException`. This proves real GDS cancellation, not only ordinary
  Cypher cancellation. The stress probe used a deliberately long iteration
  bound and zero tolerance; these are not the product recipe.
- Both owned projections and the exact synthetic Brain guard/fence were removed;
  catalog absence was verified. No canonical Brain or unrelated graph was changed.

## Primary interfaces

- [Scoped Cypher projection](https://neo4j.com/docs/graph-data-science/current/management-ops/graph-creation/graph-project-cypher-projection/)
- [Memory estimation](https://neo4j.com/docs/graph-data-science/current/common-usage/memory-estimation/)
- [PageRank](https://neo4j.com/docs/graph-data-science/current/algorithms/page-rank/)
- [Leiden](https://neo4j.com/docs/graph-data-science/current/algorithms/leiden/)
- [Progress](https://neo4j.com/docs/graph-data-science/current/common-usage/logging/)
- [Catalog drop](https://neo4j.com/docs/graph-data-science/current/management-ops/graph-drop/)
- [GDS transaction handling](https://neo4j.com/docs/graph-data-science/current/production-deployment/transaction-handling/)
- [Query API endpoints](https://neo4j.com/docs/query-api/current/endpoints/)
- [Native conditional queries](https://neo4j.com/docs/cypher-manual/current/queries/composed-queries/conditional-queries/)

GDS projections are independent in-memory state. GDS write modes can commit
outside caller rollback; the product uses stream results and explicit cleanup.
The [analytics contract](../contracts/graph-analytics.md) turns this evidence
into bounded behavior. Product acceptance remains to be executed by its slice.

The first integration fixture passed all three native recipes on a cycle/isolate,
then exposed an existing projection-import self-loop defect: one Cypher MATCH
clause attempted to reuse the same generation-membership relationship for both
endpoints. Cypher's relationship uniqueness excluded that row. Two sequential
MATCH clauses preserve exact endpoint gates and allow the self loop. The failed
owned database `recollect_test_0ce1a7bd1e4b473d9ecc7cfcad390656` is retained.
The regression fixture also covers parallel relationships and incoming direction;
its final acceptance result will be recorded after execution.

## Mac resumption and concurrency evidence — 2026-09-22

Work resumed in the original Mac checkout. Before migration, the normal database
was verified at 019 and backed up with its artifacts/journal. Nine focused PostgreSQL/GDS
checks passed in 42.25 seconds, including the native recipe fixture above, the
corrected independent accepted control, complete-set invalidation, bounded and
paired scope, journal ownership mismatch, an actual older PostgreSQL copy,
interrupted worker tasks and late replies after lease replacement. Abrupt task
interruption is not a claim of an operating-system process-kill drill.

The additional publication/final-read deadline fixture passed in 12.80 seconds.
The final browser workflow passed in 23.9 seconds after waiting for the correction's
current automatic graph generation before recomputation and including a long
unbroken label. Real repository labels exposed mobile overflow that was repaired
with wrapping and a bounded flex width. Desktop and 390-pixel screenshots were
inspected; no horizontal overflow or evidence HTML execution remained.

Product cleanup was connected to observed running native GDS work using a real
attempt's ownership and a bounded 10,000-node/50,000-edge stress fixture. Initial
checks expected transaction retirement too quickly and used a five-second HTTP
client timeout. The final test allows bounded eventual cleanup, confirms a native
termination error, verifies transaction/catalog absence and refuses publication.
The product still uses its fixed recipe, 90-second query and 120-second attempt.
An independent Query API column probe confirmed that returning only transaction
ID does execute termination; that was not the cause of the delayed retirement.

All 90 platform tests passed in 283.55 seconds, including eleven analytics
scenarios; the unrelated live OIDC test was excluded. Thirteen workspace tests,
Clippy with warnings denied, 124 generated API operations and 32 governance
checks passed. Two existing graph browser regressions passed in 17.7 seconds.
Normal migration 020, API/worker startup and runtime/browser proof passed: seven
Brains, the exact SWEG generation (99 vertices/183 edges), qualified recall,
autonomous replacement and capture recall remain intact, with all 33 model
requests unchanged. Its native WCC report is ready with cleanup confirmed and
GDS 2026.08.1. Original partial extraction coverage remains visible. No customer
file read or new model call was needed. Logs, screenshots and the private pre-020
backup are in `.cache/analytics-proof/resume-20260922/`.

Context7 resolved `/websites/postgresql_18`; its row-lock guidance and the
official [locking reference](https://www.postgresql.org/docs/18/explicit-locking.html)
and [policy reference](https://www.postgresql.org/docs/18/sql-createpolicy.html)
confirm that `FOR SHARE` blocks non-key stale updates and applies UPDATE RLS.
A narrowly authorized metadata function therefore takes the report lock after
Brain/job/auth queries and holds it through the final deadline check and commit.
Both concurrency orderings were exercised through an ordinary-reader HTTP request
and isolated database barriers, with an unchanged analytics epoch. A stale update
that commits first clears buffered rows; one that arrives after the final lock
waits for the reader to commit. The same fixture verifies foreign-Brain denial.

The retained attempt journal now compares all immutable canonical fields,
including report ID, privacy sequence and parsed timestamps. Missing installation
markers or unexpired ownership entries refuse cleanup. A changed privacy sequence
with actual owned scratch kept erasure incomplete; restoring the exact record
allowed completion. An older PostgreSQL copy with no analytical attempt rows used
the retained journal to clean the exact restored scratch, preserve an unrelated
installation's graph and replay canonical erasure.
