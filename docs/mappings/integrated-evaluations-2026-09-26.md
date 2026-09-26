# Integrated product evaluation and measured limits

Observed: 2026-09-26
Confidence: verified for the declared local acceptance and individual calls below.
The small synthetic corpus and sampled resource measurements do not establish
production capacity or fresh connectivity for integrations explicitly carried forward.

## Sources and Method

The [accepted contract](../contracts/operations-integrated-evaluations.md) froze
the corpus, labels, context/model budgets, workload and gates before execution.
`scripts/evaluate-integrated.py` uses the existing product APIs and workers in
the owned `proof-integrated-20260926` installation at loopback port 19792. Its
private quality directory is `.cache/integrated-20260926/`. No customer files
were imported. The workload uses a separate Brain with model policy disabled.
Only the selected ignored installation input contains the authorized OpenAI key.

Context7 `/python/cpython` and the official
[concurrent futures documentation](https://docs.python.org/3.14/library/concurrent.futures.html)
confirm independent worker futures and explicit result/exception collection.
The [Docker stats interface](https://docs.docker.com/reference/cli/docker/container/stats/)
and an actual selected Compose call establish the CPU/memory fields. Sampling
once after each bounded stats call is not a continuous peak measurement.

For the concurrency repair, Context7 `/websites/postgresql_17`, the official
[transaction advisory-lock functions](https://www.postgresql.org/docs/17/functions-admin.html#FUNCTIONS-ADVISORY-LOCKS),
[locking documentation](https://www.postgresql.org/docs/17/explicit-locking.html#ADVISORY-LOCKS)
and [identity columns](https://www.postgresql.org/docs/17/ddl-identity-columns.html)
establish transaction release, distinct two-integer/bigint key spaces and the
need for a uniqueness constraint on generated identities. PostgreSQL 17's
[lock manager source](https://github.com/postgres/postgres/blob/REL_17_STABLE/src/backend/storage/lmgr/lock.c)
checks conflicts with pending waiters before granting a new lock. That suggests
the admission design; the actual regression and workload, not that inference,
must establish its behavior in Recollect.

Context7 `/websites/postgresql_17` and the official
[CTE materialization](https://www.postgresql.org/docs/17/queries-with.html#QUERIES-WITH-CTE-MATERIALIZATION)
and [row security](https://www.postgresql.org/docs/17/ddl-rowsecurity.html)
documentation support statement-local reuse of authorized capture/source rows.
The real application-role query plan and eligibility regressions below establish
the result; no session cache or privileged retrieval function was introduced.

CodeGraph was synchronized before navigating `lock_brain`. Its symbol location
and several callers were useful; the reported edge from SQLx `fetch_one` to a
Cognee Python utility is spurious and was discarded after source inspection.

## Fixed-corpus actual-model comparison

Seven synthetic documents and ten claims produced 17 ready semantic records at
3,072 dimensions. Eight questions include six supported targets and two questions
whose invoice/signatory facts do not exist. Every lane used four items, an
8,192-byte ceiling and source diversity. All 32 recalls preceded generation;
automatic embedding was disabled before generated handovers could enter recall.

| Lane | Retrieved targets / 6 | Useful generated target answers / 6 | Unsupported questions with context / 2 | Irrelevant records across eight recalls | Recall p50 / p95, ms |
| --- | --- | --- | --- | --- | --- |
| Exact/lexical | 0 | 0 | 0 | 0 | 17.859 / 33.871 |
| Plus graph | 4 | 4 | 0 | 0 | 55.615 / 363.770 |
| Plus semantic | 3 | 3 | 2 | 17 | 320.222 / 1,616.265 |
| Full fusion | 6 | 5 | 2 | 14 | 423.138 / 495.069 |

Twenty-four handovers were generated through the governed synthesis endpoint:
four baseline, four graph, eight semantic and eight full. Lanes with no claim
contributors did not call a model. All calls used the installed `gpt-5.6-luna`,
identical question/title, product prompt/schema, 16,384-byte input ceiling,
1,024-token output ceiling and concurrency one. The daily Brain limit was
500,000 tokens. Persisted request/run identities prevent a blind paid retry.

The full lane retrieved all six targets but its worker handover omitted the
supplied rule about uncertain paid requests. That miss remains recorded; there
was no replacement generation or relabeling. Both unsupported questions were
explicitly unknown in the semantic/full outputs. Inspection of all 24 summaries,
completed items, next steps and risks found no unsupported factual completion or
authority assertions. Exact source/revision and structured contribution identities
were also checked mechanically. Suggestions are distinguished from claims that
work occurred. Some outputs retain internal review/status wording.

This is one implementation-agent annotation of a deliberately small synthetic
sample, not independent/blind review or evidence of general model superiority.
Semantic context on unsupported questions is an observed false-context limit,
not reliable abstention. The recall timings also overlap other local proof work
and are observations rather than an isolated performance benchmark.

Actual gateway/provider usage: one indexing request for 17 records plus sixteen
query-embedding requests, **1,071 embedding input tokens** total; 24 synthesis
requests, **33,517 input and 7,154 output tokens**. All 41 requests succeeded and
returned the selected model names `text-embedding-3-large` and `gpt-5.6-luna`.
Charged accounting totals **41,742 tokens**. No invoice or cached-token discount
is inferred. Baseline/graph recall itself makes no provider request; their
subsequent handover generation does.

Proof: private `retrieval-report.json`, `synthetic-answers.json`,
`generation-review.json`, `provider-usage.json` and stage logs under
`.cache/integrated-20260926/` / `.cache/integrated-quality-*-20260926.log`.
An initial fixture policy field was rejected with 422 before any provider
admission; correcting it did not repeat a paid attempt.

## Seven-capability evidence matrix

The production-handler cases below ran in the current platform regression.
They assert positive controls alongside forbidden outcomes. This matrix is
capability evidence, not a count-based substitute for the acceptance criteria.

| Capability | Concrete evidence |
| --- | --- |
| Rejected-value tombstones | `review_authority_durable_rules_revalidation_and_replay`, `canonical_recall_keeps_corrections_out_of_raw_copies_and_survives_rebuild`, and graph source-fragment rejection; actual restored checkpoint rejection/erase replay is carried from the recovery drill |
| Independent trust state | `claims_temporal_history_eligibility_scope_and_evidence_changes`, learning literal acceptance versus interpretation/conflict, and strict handover contributor-expiry gates; desktop review exposes the same states |
| Bi-temporal validity | Claim knowledge/fact-time history and `capture_knowledge_time_delayed_arrival_append_replay_lock_and_feedback_exclusion`; late evidence retains original capture and later knowledge time |
| Scope enforced | Workspace private paths/concurrent immutable tasks, MCP current permissions/scoped history, graph paired selection, and the two-account workload; private Brain absence has a permitted shared Brain counterpart |
| Transactional mutation audit | Durable command replay/atomicity, `review_scope_boundaries_and_failed_audit_are_atomic`, MCP stale edits and atomic audit; failed mutation/audit leaves canonical state unchanged |
| Optional actionable human review | Forged/stale review, conflict disposition and correction regressions; actual desktop review/correction/conflict workflow. Autonomous learning/revision/retirement/erasure tests run separately and require no person to review each memory |
| Paired negative evaluations | Source/derived erasure, older-state replay, invalid graph witnesses, provider policy/uncertain replay, current/private MCP authority and native receipt removal, each with preserved independent/permitted evidence |

## Runtime evidence and failures

Before migration 026, the full noninteractive platform run had 130 passes and
two observation timeouts. Both exact cases passed unchanged on isolated reruns
(4.83 and 7.90 seconds). Preserve both logs; the aggregate was not an uninterrupted
green run. Four actual native/coding-host tests passed in the owned Linux Secret
Service/internal-network fixture: Codex/Claude memory tools (9.45 seconds), native
capture/receipts (3.12), local stdio/HTTP (1.61) and private stdio/HTTP (1.71).
Cross-process credential tests use that fixture because a new macOS test binary
can require a Keychain prompt. No credential-store check was bypassed.

Fresh local Vault static rotation passed. The first dynamic test expired during
fixture process startup; the unchanged isolated rerun passed in 25.16 seconds,
including renewal of the same real PostgreSQL credential beyond its initial TTL,
denied renewal, natural expiry and database rejection. The fixture's PostgreSQL
readiness probe now checks TCP, avoiding its temporary initialization socket.
Logs: `.cache/integrated-vault-20260926.log` and
`.cache/integrated-vault-dynamic-recheck-20260926.log`. No Vault token or lease
was revoked. Enterprise KV/Proxy proof is carried from the
[Vault mapping](mcp-vault-private-2026-09-22.md); Enterprise dynamic database
connectivity remains explicitly unverified there.

The actual review and workspace desktop cases passed in 19.2 seconds against
the installed migration-025 image. Personal/shared HTTPS installation, two-user
browser/native access and failure recovery are carried from the
[installation acceptance](installation-2026-09-26.md). Actual encrypted SFTP,
source-offline restore, interrupted resume and receipt-only effect reconciliation
are carried from the [recovery evidence](recovery-2026-09-26.md). OIDC was excluded
from this new run; its predecessor proof is not described as fresh connectivity.

## Concurrent-workload investigation

The declared workload remains 50 repositories, two accounts, two environments,
200 approximately 1-KiB documents, 100 structural nodes/200 edges, four capture
producers, four recall readers, 120 captures/120 reader operations and four GDS
analyses. Additional exact-recall probes measure actual retained text becoming
readable. Explicit `recall_busy` refusals are counted, with bounded client wait
included in operation latency; application limits are unchanged.

The first setup requested graph rebuilding after facts became visible but before
publication finished. It received 503; the worker then built the correct graph.
The harness now waits for the completed automatic projection. The next attempt
also exposed a missing immutable selection in a probe and incomplete exception
collection. Those harness defects were corrected; failed attempts were retained.

The diagnostic rerun still failed: all four capture callers and analysis admission
received `database_unavailable`. Actual PostgreSQL logs show Brain row-lock
statement timeouts. Four continuous readers completed 30 requests each at roughly
534 ms median while writers waited up to their ten-second statement deadline.
`.cache/integrated-workload-recheck-20260926/database-waits.json` records the exact
wait chains. The focused pre-fix regression then failed with a newly arriving
reader bypassing an already waiting writer (0.58 seconds). This is a reproduced
product defect, not a successful load result or a reason to relax the gate.

Migration 026's writer-order regression passed in 0.57 seconds. The complete
noninteractive platform run then passed **133 cases**, zero failures (383.17
seconds); OIDC and four host cases were filtered. The four actual Linux/native
host cases also passed separately. Desktop review passed and workspace passed
after fixing its fixture to wait for navigation immediately after Brain creation;
the persistent installation contains older cards with the same display name.
These are separate passes, not a single uninterrupted two-case browser run.

The unchanged post-026 workload still failed: writers advanced, but all eight
callers eventually received recall `database_unavailable`. PostgreSQL reached
its two-CPU limit and the two-second statement deadline. The actual candidate
query plan showed repeated capture-event scans and 4,400 binding lookups for
220 source versions/20 captures. A query-only repair materializes authorized
capture associations and source knowledge within each SQL statement, retaining
the exact-version receipt time and original source-level scope for later versions.
On the same owned fixture/application role, before/after SQL execution was
**1,985.586 / 68.849 ms**, with all 101 ranked records byte-equivalent after JSON
normalization. The diagnostic transaction allowed 15 seconds to observe the old
plan; product deadlines, resource limits and workload gates remain unchanged.
Proof is under `.cache/integrated-workload-026-20260926/`, including both plans,
result comparisons, preserved failures and resource samples. This is a diagnostic
comparison, not the required concurrent acceptance result.

The intermediate candidate-query image was `recollect-product:integrated-proof`, local image ID
`aafb954ff08b543a2defe7f3868c1f25c9edcbd22c2debf290c2776cfe11e903`.
The ordinary product Dockerfile built Rust, generated OpenAPI, TypeScript and
Vite successfully; the selected installation is healthy on migration 026.
Both desktop review/workspace cases then passed together in **18.5 seconds**.
Workspace tests passed **26**, with **154** runtime-dependent tests ignored;
all-target Clippy and formatting passed. Ignored cases are not counted as proof.

The post-candidate-query full platform attempt had **128 passes / five failures** (767.46
seconds). Two failures were missing `RECOLLECT_TEST_RECOVERY_FIXTURE` in the
invocation; both passed with the existing owned SFTP fixture (4.07 / 3.68 seconds).
The native HTTP case passed unchanged alone (1.75 seconds). Managed observation
capture failed again alone because the fixture's 15-second completion wait
abandoned a still-starting call: its recorded claim began roughly 13 seconds
after admission, while the product permits 30 seconds of startup and this
fixture permits another 30 for execution. The test waiter now covers those
existing deadlines plus 15 seconds of queue/poll overhead; no product limit
changed. That case passed in 8.11 seconds. The strict handover contributor-expiry
case hit the overall recall deadline during the aggregate run, then passed
unchanged alone in 4.45 seconds, retaining its injected database delay and
positive independent claim. Thus all **133 distinct noninteractive cases** have
current passes, but this was not an uninterrupted green aggregate. Logs retain
the failures and focused reruns under `.cache/integrated-*-20260926.log`.

The first rebuilt-workload attempt then failed its ordinary recall positive
control before concurrency began. PostgreSQL identified the source-lineage query
as the two-second timeout, while candidate matching had already completed.
`source_versions` was automatically analyzed 47 seconds after that failure.
Subsequent standalone custom/generic plans took about 21–51 ms; the initial
plan was not retained, so stale cardinality is an inference rather than a proved
root cause. The lineage joins now use parameterized one-row lookups on the unique
excerpt, parent, original capture-version and binding identities, reusing the
existing semantic-query approach to avoid whole-Brain inner scans before fresh
statistics. RLS, missing-parent handling and current metadata expiry remain.
On the later analyzed state, both forms returned the same 100 rows (30.943 /
36.414 ms); this is equivalence evidence, not a speedup claim. Official
[LATERAL semantics](https://www.postgresql.org/docs/17/queries-table-expressions.html#QUERIES-LATERAL)
and [prepared plans](https://www.postgresql.org/docs/17/sql-prepare.html), also
queried through Context7, informed the bounded lookup and diagnosis. The failed
preflight and plans are retained in `.cache/integrated-workload-query-20260926/`.

## Final workload and operating envelope

The final standard product build is `recollect-product:integrated-proof`, local
image ID `4dda2d9c0d75b7f6c3010236117d44eb2c14c85a3cbf632dad285fb26693dc44`.
Rust, OpenAPI generation, TypeScript and Vite passed in the ordinary Dockerfile.
After the last lineage change, nine affected retrieval/graph/context/retention
cases passed in 26.08 seconds and the original-version capture scope/session/agent
case passed in 54.35 seconds. All-target Clippy and formatting also passed.
The 26 workspace passes and 133 distinct platform passes above precede this last
lineage change; the ten affected cases and installed proof below follow it.

The unchanged workload completed on migration 026: **50 repositories, two accounts,
200 documents / 220,600 bytes, eight callers, 120 accepted captures, 120 reader
operations and 213 successful readability probes**. There were **zero unexpected
request failures, scope leaks or paid provider requests**. The four concurrent
analyses correctly became stale as canonical input changed; no stale numeric
result was served. After writes settled, fresh WCC completed with all 100 expected
vertices in one component over the 200-edge structural fixture. Invalid and
over-limit input was refused with an ordinary successful request afterward.

| Measurement | p50, ms | p95, ms | p99, ms | Maximum, ms |
| --- | --- | --- | --- | --- |
| Capture admission | 115.901 | 440.804 | 514.617 | 526.913 |
| Recall operation, including capacity wait | 495.056 | 862.825 | 2,464.665 | 4,925.464 |
| Accepted write to readable retained content | 937.859 | 2,584.412 | 4,832.675 | 8,081.741 |

Import took 4.941 seconds, concurrent work 42.371 seconds and final queue drain
0.088 seconds. Database size grew by 1,671,168 bytes. **981 explicit `recall_busy`
HTTP 429 refusals** caused bounded 50-ms client waits; those waits are included
in the latency measurements. The 4.9-second recall tail remains a measured limit,
even though the contracted p95 gate passed. No worker, statement, application
deadline, resource limit or acceptance threshold was raised to obtain this pass.

The machine was macOS 26.6.2 ARM, 14 logical CPUs and 24 GiB host RAM. Docker
Desktop engine 29.7.2 ran Linux aarch64 with eight CPUs and 12,790,353,920 bytes
(11.91 GiB) VM RAM. Configured service caps remained API 1 GiB / two CPUs,
worker 2 GiB / four CPUs, PostgreSQL 1 GiB / two CPUs, and Neo4j 1.5 GiB / two CPUs.
Neo4j heap initial/max was 256/512 MiB and page cache 128 MiB. The optional
journal mirror was disabled during this workload and enabled for recovery below.

Fifteen resource observations were collected, one second after each bounded
Docker stats call, approximately three seconds apart. Sampled maxima were API
14.86 MiB, worker 6.72 MiB, PostgreSQL 285.30 MiB and Neo4j 679.80 MiB. The largest
combined sample was **1,033,234,285 bytes / 985.37 MiB**; individual maxima need
not occur together. Sampled CPU maxima were 30.63%, 1.12%, 205.2% and 16.79%
respectively. CPU percentages are per-core, sampled observations. These are
neither continuous peaks nor a capacity claim for the proposed 8-CPU/32-GB VM,
larger customer corpora or sustained production throughput.

Proof: `.cache/integrated-workload-lineage-20260926/workload-report.json`,
its complete state, samples and run log; build log
`.cache/integrated-product-lineage-build-20260926.log`; affected regression logs
`.cache/integrated-lineage-{retrieval,capture}-20260926.log`; latest Clippy/format
logs `.cache/integrated-{clippy,format}-lineage-20260926.log`. Earlier failures
remain retained and are not counted as passing runs.

## Final recovery, desktop and normal installation

The final image then completed an actual encrypted SFTP backup and fresh-volume
restore with the source API/worker stopped. Backup took **11.278 seconds** for a
5,582,360-byte archive; restore took **25.309 seconds**. Canonical installation
identity, all 21 Brain admission identities, retained source/claim counts and all
41 model-request rows survived. A newly created Brain received a distinct later
admission key. Exact captured content retained its original scope, the frozen
quality claim retained its revision, the owner's access to a member-private Brain
was refused, and the 100-node/200-edge graph was rebuilt. No new paid call occurred.
Proof: `.cache/integrated-recovery-20260926/report.json` and verified private state.

The restored installation `proof-integrated-restore-20260926` at loopback port
19794 serves the final image. Its predecessor at 19792 keeps its databases and
stopped application writers; do not run both canonical copies as writers.
Final **desktop review and paired workspace workflows passed together in 18.0
seconds** against the restored image. They cover corrections, conflict/stale
review, exact evidence, native pairing, original task/subagent scope and private
checkout history. The correction, operation-history and checkout screenshots
were inspected at 1440 × 960. Obsolete mobile assertions were removed from these
two cases to honor the user's desktop-only scope. Proof:
`.cache/integrated-desktop-final-20260926.log` and `.cache/ui-review-desktop.png`,
`.cache/ui/task-context.png`, `.cache/ui/workspace-checkouts.png`.

The normal native installation was backed up, drained and upgraded to migration
026. API/worker PIDs 9750/9749 are ready at `http://127.0.0.1:8787`. Exact inventory
comparison preserved seven Brains, 33 model requests and existing profiles/grants,
with zero active jobs. Retained SWEG exact recall returned one item / 1,630 bytes
in 49 ms, with no new paid call or customer file read. Its dump, inventory and
content-free verification are in `.cache/integrated-normal-20260926/`.

After all native tests, guarded Cargo/Clippy cleanup removed only inactive
`target/debug/{deps,incremental,examples,build,.fingerprint}`. No selected artifact
was open, and the server/agent/runner/bridge executable identities were preserved.
Actual free space increased **13.71 GiB**, to **121.81 GiB**; normal readiness and
both process identities passed again afterward. The Linux host cache, Docker
resources, data and reference checkouts were preserved. This final pass is recorded
in `.cache/build-cache-cleanup-final-20260926.json`, separately from the earlier
1.6-GiB pass. Subsequent builds will regenerate these caches.

## Closeout and remaining limits

All contracted local quality, workflow, concurrency, recovery and desktop gates
are satisfied. The seven-capability matrix distinguishes current checks from
unchanged predecessor evidence. `./scripts/validate.sh` passed governance lint
and all 32 checker tests; its final result is retained in
`.cache/integrated-governance-closeout-20260926.log`. `git diff --check` and focused
desktop-test formatting passed. The post-cleanup final health check again matched
the complete normal inventory and verified the restored image and stopped source
writers: `.cache/integrated-final-health-20260926.json`. These checks establish documentation
and formatting consistency, not product correctness by themselves.

The synthetic quality miss, unsupported semantic context, 4.9-second recall tail,
single-reviewer annotation, sampled-resource limit and carried OIDC/Enterprise
evidence remain explicit. External production cutover, mobile, high availability,
larger-scale tuning and a new paid experiment are outside this goal. All 29
original product slices plus the Atlas capture repair are delivered locally.
No Vault token or lease was revoked. Version is N/A; changes remain uncommitted.
