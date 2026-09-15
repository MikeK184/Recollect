# Graph implementation reuse review — 2026-09-15

## Scope and method

The user explicitly requested checking Cognee, Atlas and other implementations
before continuing graph work, and avoiding unnecessary rewrites. Read the local
Cognee adapter, factory/configuration, provenance transitions/deletion planner,
retrieval graph/report code, metrics and frontend dependencies. Its clean checkout
was at `c0d18c80e24b7b78918e7642c03f6f128fdd2aee`. CodeGraph was synced and used
for the exact adapter outline; source reads established the implementation.
The existing user-authorized Atlas reviewer independently checked graph-focused
Atlas reports/patterns, selected upstream source and current Neo4j documentation.
Neither reference checkout was changed, and no provider calls were made.

Confidence is `verified` for the inspected local code and Neo4j interface proof,
`observed-once` for the reviewer's pinned upstream source inspection, and
`inferred` for proposed transfer to future Recollect slices. None is a benchmark
or proof that all behavior in another product matches Recollect's requirements.

## Cognee mechanisms and reuse decisions

| Mechanism | Inspected source | Recollect use |
| --- | --- | --- |
| Provider abstraction; local Ladybug default and separate Neo4j adapter | `cognee/cognee/infrastructure/databases/graph/config.py`, `get_graph_engine.py`, `graph_db_interface.py` | Keep the selected Neo4j/GDS engine behind a Rust adapter. Do not add another backend or Python runtime merely to reproduce this abstraction. |
| Official async Neo4j Python driver and Cypher | `cognee/cognee/infrastructure/databases/graph/neo4j_driver/adapter.py:194–274` | Reuse Neo4j itself, its constraints, transactions and path engine through the accepted Rust Query API boundary. The Python adapter is not a Rust client dependency. |
| Batched `UNWIND`/`MERGE`, stable identities and provenance folded into writes | `adapter.py:99–149`, `348–441`, `1204–1281` | Use atomic generation membership/edge writes. Recollect's canonical IDs and generation descriptors remain PostgreSQL-owned; no separate invented graph storage engine. |
| Deadlock/transient retry with bounded backoff | `neo4j_driver/deadlock_retry.py` | Use existing durable job retry/lease machinery; preserve useful Neo4j failure classification and avoid nested unbounded retries. |
| Bounded neighborhood queries | `adapter.py:1891–1976` | Reuse native Cypher expansion. Recollect additionally qualifies every intermediate entity/edge before shortest-path selection. |
| GDS projection, catalog cleanup, connected components and graph metrics | `adapter.py:2204–2361`, `neo4j_metrics_utils.py` | Slice 18 uses GDS algorithms/projection estimates/catalog operations, not a new Rust PageRank/community implementation. |
| NetworkX PageRank and in-memory triplet ranking | `cognee/cognee/modules/retrieval/graph_report_retriever.py:5–76`; `cognee/cognee/modules/graph/cognee_graph/CogneeGraph.py:417–505` | Useful quality references for slice 20. Do not add NetworkX or port Cognee's custom triplet scorer without target-corpus evidence. |
| Source-ref ownership and retryable deletion preserving shared artifacts | `cognee/cognee/infrastructure/databases/provenance/source_ref_state.py`; `cognee/cognee/infrastructure/databases/unified/provenance_delete_planner.py:70–170` | Preserve shared graph entities when one generation is removed; retain durable cleanup identities through failures. Recollect's existing privacy closure/journal supplies ownership and erased identities. |
| Existing React graph renderer and force layout | `cognee/cognee-frontend/package.json:24–35` (`d3`, `d3-force-3d`, `react-force-graph-2d`) | Slice 19 must evaluate/reuse an established renderer/layout. Current slice 16 supplies status, canonical entity/evidence browsing and bounded path results; no custom canvas/layout engine was introduced. |

Cognee's [published graph-store guide](https://docs.cognee.ai/setup-configuration/graph-stores)
documents Neo4j with APOC/GDS and transient retries. It still labels Kuzu as the
default; the inspected local configuration uses Ladybug. Preserve that source
discrepancy instead of presenting both as the same version. The local
`Neo4jCommunityDatasetDatabaseHandler.py:40–65` isolates datasets with separate
Community containers. Its unscoped whole-database queries cannot be copied into
Recollect's accepted shared instance without Brain/manifest qualification.
Current native Cypher supplies the fixed relation operations needed here; no
additional APOC dependency is required for those operations.

This review reused mechanisms and underlying engines; no Cognee source file was
copied or installed as a mandatory product runtime. Its source carries Apache-2.0;
any future code copying must retain applicable attribution and notices.

## Independent Atlas and upstream comparison

| Source and evidence boundary | Transferable mechanism | Boundary |
| --- | --- | --- |
| Atlas `content/systems/graphiti.md:73` and [pinned Graphiti search source](https://github.com/getzep/graphiti/blob/425bf2481b51437e43455e09d241c5f46e3d95f3/graphiti_core/search/search_utils.py#L411), read by the reviewer | Existing graph adapters, search recipes, bounded expansion and RRF | Its inspected fallback expands paths before filtering returned edges. Preserve Recollect's intermediate eligibility pre-filter. |
| [Pinned HippoRAG source](https://github.com/OSU-NLP-Group/HippoRAG/blob/e37fba2af1a951ac340d837a7c02efb9d8c9544a/src/hipporag/HippoRAG.py#L1533), read by the reviewer | Calls igraph Personalized PageRank and retains entities/triples with surviving chunk references | Associative undirected diffusion is not directional path evidence. Use a GDS implementation only if the owning evaluation justifies it. |
| Atlas `content/systems/basic-memory.md:87` | Canonical content and rebuildable relational projections; source-owned relations and reconciliation | Atlas-reported behavior, not a live integration test. Recollect already has the transferable canonical/projection split. |
| Atlas `content/systems/graphify.md:193` | Tree-sitter extraction and distinct extracted/inferred relationships | Continue using retained Enola outputs. Its hashing/scoring choices are not requirements under ADR 0003. |
| Atlas `content/systems/neo4j-agent-memory.md:84` | Temporal preferences and reasoning traces backed by graph queries | The report identifies unscoped accessors; adopting the package would not establish Recollect's authorization boundary. |

The reviewer confirmed the current Neo4j [shortest-path pre-filter](https://neo4j.com/docs/cypher-manual/current/patterns/shortest-paths/)
matches the native mechanism. Its Context7 guessed lookup failed and the resolved
library returned older 2.13 material for GDS; current official documentation was
used for scoped projections, algorithms, memory estimates and catalog cleanup.
Future slice 18 verifies the actual installed GDS interfaces before implementation.

## Concrete change from this review

The reviewer found a repeated heartbeat self-block pattern in the new graph
worker: it awaited renewal while suspended work could hold the same job row.
The already-correct learning/handovers/semantic pattern is now one shared
`worker::with_lease` helper, used by all four outbound-I/O workers. This removes
duplication and keeps publication polled while renewal waits. The existing
semantic delayed-publication/expiry regression passed in 7.28 seconds. The first
actual Neo4j/PostgreSQL graph proof passed in 2.23 seconds, covering duplicate
upstream identities, direction/hop bounds, rejection of an intermediate entity,
selection of a longer eligible path, rebuild and superseded-generation cleanup.
Evidence is retained in `.cache/graph-proof/shared-heartbeat-semantic.log` and
the subsequent complete `.cache/graph-proof/platform.log`. The
[validation mapping](graph-validation-2026-09-15.md) records all later acceptance.

The expanded platform regression subsequently caught stack overflow in an
existing autonomous retry test. A temporary, read-only future-size probe measured
49,952 bytes for the composed worker future. Allocating the outbound work before
constructing the shared heartbeat future reduced it to 3,272 bytes; the existing
autonomous retry/override test then passed in 1.71 seconds. The diagnostic source
was removed, with measurements and the passing result retained under
`.cache/graph-proof/worker-future-{before,after}.log` and `stack-fixed.log`.

The [graph slice](../roadmap/execution/archive/graph-projection-and-traversal.md)
is shipped with separate executed evidence. This comparison does not declare graph analytics,
cross-repository linking, the interactive renderer or retrieval fusion shipped.
