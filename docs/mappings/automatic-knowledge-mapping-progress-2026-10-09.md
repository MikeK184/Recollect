# Automatic mapping review, graph preparation and local extraction proof

Observed: 2026-10-09 (Europe/Stockholm)
Confidence: verified source and retained local test receipts; normal-runtime graph acceptance was open at this checkpoint
Status: dated checkpoint, reduced on 2026-10-19 to its measurements and failures. Current status lives in the [roadmap](../roadmap/epics/index.md).

The full 2026-10-09 text (design review, receipts lists, per-step narrative) is
at the pinned revision
[a0aea9d](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/mappings/automatic-knowledge-mapping-progress-2026-10-09.md).
Checkpoint commit `18a8ba7ee8399f143b6e22d16c71c81b2a608ae0` was published on
2026-10-09; the reduction reran nothing.

## Scope and method

The user asked two read-only agents to review automatic information mapping,
entity creation and grouping and to compare Cognee, Graphify and other open-source
components for the [dedicated plan](../research/automatic-knowledge-mapping-plan-2026-10-09.md).
The coordinator implemented the graph-preparation prerequisite, repaired fixtures
through production review handlers, ran native tests and ran a standalone local
extractor on authored synthetic development text. No model/provider call was paid;
reference checkouts, normal model policies and the 10 held-out extraction cases
were preserved. On 2026-10-09 the user paused implementation and requested a
commit and push of the existing work; unfinished packs stay active.

Upstream pins: Cognee `0ec7a9fa61c9ff04bf7e02e0d57af363a993a5e1`, Graphify
`5b74d7d74911cf435c8f1636b6f96ea202cc6246`, Graphiti
`1026ae7ae25e7e4cfcc1c7ebe00347b8a90f52a0`.

| Component | Useful seam | Limit |
| --- | --- | --- |
| GLiNER2.5 | Local schema-driven entity spans and classification candidates | Raw domain typing failed the development precision gate below; not an embedder or truth authority |
| Cognee | Extraction-derived summaries, entity/chunk associations, shared-entity scoring, incremental placement | NodeSets are supplied tags; automatic context indexing is separate; whole-population processing unmeasured |
| Graphify | Versioned code producer, deterministic community labels | Compare against Enola on the same committed fixture; it does not repair Brain lock contention |
| Graphiti | Temporal/entity-resolution components | Full pipeline has additional model operations; local NER alone does not make it free |

## Graph preparation checks

Migration 044 and the [concurrent preparation contract](../contracts/graph-concurrent-preparation.md)
let reads and worker descriptors prepare from provisional read-only snapshots
without holding Brain locks through native I/O; publication rechecks role,
session/device, scope, epoch and deadlines under the fair canonical lock.

| Check | Observed result |
| --- | --- |
| Read-only snapshot versus same-Brain writer; bypass controls | Passed on actual PostgreSQL; zero model requests |
| Pause actual Neo4j work during view/explore/path, change generation, resume | Passed; buffered output discarded, fresh read succeeds |
| Revoke browser session during native preparation | Passed; final response unauthorized |
| Native graph suite | First run 35 of 38 passed; the three failures passed after fixture repairs (not a new complete-suite run); later complete suite 40/40, then 41/41 after migration 046 |
| 10k-node capture workload (40 reads, concurrency two, 32 captures) | 40/40, p95 0.513 s; after migration 046 p95 0.519 s; zero model requests. Covers capture only, not audits or rebuilds |

**Failed first populated workload (retained):** 40/40 reads failed with
`graph_preparation_changed` while 26 captures succeeded, because the Brain-wide
epoch invalidated an immutable repository window on unrelated capture changes.
The contract now lets explicitly partial windows requalify buffered entities in
the fresh canonical transaction. An earlier setup run that looked for node 5,000
inside a 200-item page was a setup failure, not a latency sample.

Migration 045 and the Rust candidate module persist source-bound batches,
conservative identities, exact mentions and pending relationship/alias records
(exact UTF-8 spans, malformed/foreign/duplicate/unbounded rejection, atomic
rejection, replay, older-database journal replay, populated Brain deletion all
passed). Migration 045 also added `agent_brain_usage` and `source_import_scopes`
to the deletion inventory. Ancestral excerpt model-input-fence cleanup remained
a known privacy gap; automatic mapping was not enabled.

## Normal installation and graph read failures

Checkpointed local updates reached migration 045, 046 and 047 (API/worker build
`27bbd00f006e9684b25f18b47abcfc3110f22806-dirty`; the 047 image built at
`2026-10-09T13:57:36Z`, encrypted checkpoint `b62691a2-9b5b-4789-9100-a30309be0344`;
earlier checkpoints `8f835cf0-3756-47c7-8a8c-b6ee560c3c73`,
`515f90e6-15f0-4ed3-834d-574cd0012fe4`, `04fc5bcb-362f-4a31-9e28-3ce02b188229`).
Each update preserved all 17 Brains, 1,232 sources, 1,633 claims and one
repository snapshot with no added or missing IDs. The image is a dirty local
build, not a release.

After migration 045 the previously failing normal Brain still returned
`graph_timeout` or `database_unavailable` after about 2-15 s. A canonical
dependency-deadline lookup took 1.344-1.841 s per revision; migration 046's
lateral lookup evaluated it in 4.0-4.5 ms (rolled-back probe), and a complete
graph-candidate probe returned 67 representations in 0.698 s. These were query
measurements, not graph p95.

After migration 046, ordinary-worker-on probe rounds:

| Read | First round | Second round |
| --- | --- | --- |
| Windowed view | 503 `database_unavailable`, 3.638 s | 200, 56 nodes, 4.127 s |
| Uncentered exploration | 200, 56 nodes / 45 edges, 4.786 s | 200, 56 nodes / 45 edges, 2.349 s |
| Centered one-hop exploration | 200, 2 nodes / 1 edge, 0.737 s | 200, 2 nodes / 1 edge, 0.271 s |

Semantic-discovery SQL (normal-data, read-only, app role): previous query 10.053 s
timeout / 19,638.714 ms / 17,739.522 ms versus revised 225.003 / 334.499 /
227.311 ms; the 100-input consumer window returned the same 8 IDs. All 22
semantic controls passed (101 earlier unavailable claims survived). The first
sequential series after this repair:

| Series | Result | Failures retained |
| --- | --- | --- |
| Migration 046, 40 reads | 37/40; p95 3.520 s; max 4.389 s | ordinal 33 view 503 3.520 s; 34 uncentered exploration 503 4.098 s; 35 centered exploration 503 2.214 s |
| **Migration 047, 40 reads (latest normal result)** | **35/40; p95 4.027 s; max 4.396 s** | ordinal 22 uncentered 503 4.121 s; 23 centered 503 2.258 s; 24 view 503 4.027 s; 25 uncentered 503 4.396 s; 26 centered 503 2.196 s |

Migration 047 bounded support-audit discovery (at most 16 missing identities per
preparation, two seconds or four positives; a first read-only normal probe made
four passes examining 5/2/2/2 identities with two timeouts each). Native controls
passed: a 35-record unavailable prefix, restart/wrap, competing preparations,
review, policy change, expiry, erasure, account disablement and a 500 ms
same-Brain writer gate.

A payload-free PostgreSQL blocker sampler found blocked sessions in 58 of 60
samples, 23.637 s of cumulative transaction age in the digest family (at most
2.143 s in one statement), and the same Brain holding admission during
digest/handover refresh for up to 16.247 s with seven waiters; the oldest hash
resolved to the handover refresh candidate query. This identified the next seam
(cumulative digest/handover writer tenure) but did not attribute each failure.
Normal-runtime graph acceptance failed at this checkpoint. The
[latest local gates](graph-organization-implementation-2026-10-18.md) are
separate local-fixture and clone results.

## Local GLiNER development experiment

An isolated environment installed `gliner2[local]==2.0.0` with PyTorch 2.14.1; the
first load failed on a missing `protobuf`/`sentencepiece` dependency, then
succeeded. Checkpoint `fastino/gliner2.5-base-v1` at
`54785d51df8d86a0d8f2eb212fec15871aad2c4d` (Apache-2.0, English) ran on Mac CPU
with Python 3.12.14, two Torch threads and threshold 0.5. The
[40-document fixture](../research/automatic-knowledge-mapping-fixtures-2026-10-09.json)
holds 30 development and 10 held-out cases (English, ASCII, at most 93
characters); only the 30 development cases were inferred, scored by exact typed
spans against authored gold with no LLM judge.

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

The run **fails** the 95% precision / 80% recall gates: several software names
were typed as people, services or environments and high confidence did not prevent
wrong typing, so raw confidence is not entity-resolution authority. These are
short toy measurements, not throughput, long-document, alias, relation, grouping,
multilingual or answer-quality acceptance; the model came from its public
repository and offline flags are not a network sandbox. The probe returns
half-open Python character offsets; canonical `SourceSpan` uses UTF-8 bytes, so it
now converts and revalidates them (checked on an emoji/accented-prefix control
after the inference receipt; actual Unicode extraction was untested).
