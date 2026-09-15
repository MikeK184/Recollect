# Exact and lexical retrieval interface evidence

Observed: 2026-09-15
Confidence: local implementation, regression and normal-runtime recall verified

## Sources and method

Context7 resolved PostgreSQL 17 as `/websites/postgresql_17`; queried full-text
configuration, query parsing, ranking and generated-column/GIN limits. Read the
official PostgreSQL 17 [query/ranking documentation](https://www.postgresql.org/docs/17/textsearch-controls.html)
and [table/index guidance](https://www.postgresql.org/docs/17/textsearch-tables.html).
`websearch_to_tsquery` accepts web-style phrases, OR and exclusions without SQL
query-syntax exceptions. `simple` avoids language-specific stemming. Weighted
vectors and `ts_rank_cd` supply cover-density ranking, not BM25. Bounded input
and source chunks keep vectors below the documented size/position limits.

The existing installation uses PostgreSQL 17.10. No database upgrade or additional
extension is needed for this slice. GIN/generated expressions derive from canonical
fields; current Brain/scope/privacy/claim checks remain in the Rust request path.
Canonical erasure clears/removes indexed payloads and a rebuild cannot add a second
authority. The selected initial retrieval baseline has no query cache or provider call.

## Existing local authority

CodeGraph status/sync and bounded source inspection located `memory::view`,
`memory_rules::matching/overlaps`, `memory_evidence::read_tx/row` and immutable
workspace operations. Canonical views already combine fact/knowledge time,
review/withdrawal/conflict state, source availability and procedure/handover
dependencies. Generic publication operations accept only write/capture kinds;
recall must validate its own existing `context` operation and actual actor/device.
This is an interface distinction, not permission to substitute publication scope.

Sources already retain processed chunks with exact byte/line ranges. Repository
facts retain exact snapshot and recorded structural data. Capture sources retain
original operations. These inputs can support canonical lexical search without a
new text-copy projection. The accepted [contract](../contracts/retrieval-exact-and-lexical.md)
specifies filters, current rejection checks on raw fragments and bounded context.

## Implemented consumer and focused proof

Migration 015, `retrieval_candidates.sql` and `retrieval.rs` implement the shared
Brain recall endpoint. The native `scope recall` command and React recall panel
call it with canonical operation/selection authority. The generated schema has
114 unique operations. Exact identity/literal matches precede `simple` lexical
cover-density scores. The best fragment of each identity precedes further
fragments, with a 100-row candidate cap, at most 20 results and bounded attributed
context. There is no semantic/graph channel or provider request in this slice.

The real PostgreSQL/API corpus proves exact facts/manifests/source identities,
native actor/device operations, changed task defaults, selected environments,
collections, areas and knowledge time. Three initial literal/phrase controls
passed (source title, phrase/JWT text and inert hostile source). The recorded
focused UTF-8 run examined 100 candidates, returned the long source and an
independent matching source in 3,817 bytes and took 94 ms. These are local sample
measurements, not throughput or semantic-retrieval benchmarks. Another control
places 101 explicitly ineligible proposals ahead of an accepted matching claim:
strict recall examines the eligible row and reports the withheld candidates.

Correction proof rejects a supported assertion, copies its raw text into a new
source UUID, rebuilds indexes and confirms that ordinary context still withholds
the blocked assertion. Replacement and unrelated evidence remain usable. History
retains explicit rule qualifications. Source erasure clears canonical bytes and
recall, missing artifacts cannot fall back to indexed text, and retained older
claim intervals are independently selected without reviving an erased latest one.
An ordinary replacement rationale may correctly mention the historical value;
supersession does not create a rejected-value rule.

The Atlas follow-up added old-format claim defaults, capture receipt knowledge
time, qualified retained claims after raw-support expiry, and final deadline
checks for strict handover contributions. Both retention/default regressions
passed in 5.37 seconds. The handover fixture delays the final database read by
1.2 seconds, crosses the contributing claim's deadline, excludes that strict
result, retains an independent accepted control and allows qualified history.
The [Atlas audit](atlas-implementation-audit-2026-09-15.md) records findings and
the distinction between source inspection, controlled tests and real models.

Browser recall, capture and claims tests passed with a disposable database and
worker. Recall exercised filters, bounded context, inert HTML, canonical evidence
inspection and source erasure on desktop/mobile. The recall test runner now starts
the processing worker automatically. Type generation/build and Clippy passed.
Final platform regression passed 44 tests in 99.84 seconds; all 32 governance
tests passed. Pack and index reconciliation accompanies local delivery.

## Normal runtime, 09:25 UTC

Restarted the owned API/worker through `./scripts/dev.sh`; migration 016 applied
at 09:16:46 UTC, and the final recall/default changes began serving at 09:25:15.
Readiness and current browser delivery passed. The read-only proof preserved all
five existing Brains and their model-request totals (0, 0, 3, 6 and 0). It made no
new model calls and read no customer files. This is local delivery, not external
deployment.

In SWEG Brain `5c054930-d266-4c18-a42b-942729f942aa`, exact and lexical recall found
retained fact `019c7bdb-d68f-43d6-aaa2-361f49e2a6c6` (`var.ssh_public_key`,
`variables_os.tf:10`) in snapshot `260c6505-f1b8-40ad-bb2a-c7db5cd48a1f` under
manifest revision `6f558274-57c4-4007-aefb-10ad028221ca`. API elapsed times were
69/81 ms and exact context was 1,584 bytes. The old-format proposed claim was
visible to investigation and excluded from strict recall without rewriting it.
The existing autonomous demo's current 9090 revision and sanitized captured source
were recalled without new learning. Desktop/mobile screenshots were inspected;
no horizontal overflow or missing evidence attribution was observed.

Evidence is recorded in `.cache/retrieval-runtime-proof/state.json`, its runnable
`run.mjs`, and the two `sweg-*.png` screenshots. The fixture uses existing retained
repository data; it does not re-read or republish SWEG checkouts.
A subsequent read-only run at 09:27 UTC preserved all six Brains after the
synthetic Luna demo was added. It measured 68/73 ms exact/lexical SWEG recall and
made no additional model calls. The [Atlas audit](atlas-implementation-audit-2026-09-15.md)
records the separate four-call synthetic session proof.

## Limits and next consumer

Exact/lexical recall is bounded, language-neutral matching. Complex dependency,
rule, fact-time and manifest checks can still consume candidate capacity before
withholding a row; coverage says so and no exhaustive eligible-search claim is made.
Canonical source diversity does not establish independent corroboration. Graph
and semantic retrieval, external answer generation and host prompt injection remain
their named successors. Semantic recall must preserve exact model/dimension identity,
pre-rank scope, privacy/reindex behavior and an exact eligible-vector control.
