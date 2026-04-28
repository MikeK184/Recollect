# 0007: Canonical graph projections and scoped traversal

Status: accepted

## Decision

Use the selected Neo4j Community Query API through a Rust adapter. PostgreSQL
stores generation identity, input selection, immutable projection descriptors,
publication state, audit and durable work. Retained canonical evidence supplies
the descriptors. Neo4j stores only typed relationships and opaque canonical
identities; labels, source text, claims and permissions are hydrated from the
canonical read boundary. No graph write can create a claim or acceptance.

Publish repository generations per immutable snapshot, reused across manifests.
Publish knowledge generations from current claim revisions and their explicitly
recorded support/contribution links. Shared canonical entity identities prevent
copying repository nodes for each environment. Never resolve ambiguous upstream
fact IDs by arbitrary record selection. Preserve unsupported/unresolved coverage.

Read qualification precedes traversal: select canonical nodes, exact input
generations and allowed typed edges, then use a bounded shortest-path pre-filter
inside the path selector. Recheck authority and retention at response publication.
Correction can therefore remove an intermediate path contribution before a
shorter path suppresses an eligible alternative. Old projections confer no read
authority. Analytics will consume this qualified input boundary and add its own
epoch invalidation contract in its owning slice.

Neo4j import is independently transactional and idempotent. Stage and validate
node memberships and complete edge descriptors before committing PostgreSQL
readiness under the current worker lease. No cross-database transaction is
claimed. Erasure uses replayable identity fences and a per-Brain Neo4j write
guard, so a delayed import cannot recreate an erased entity after cleanup.

## Why

The user authorized the complete roadmap and routine decisions within the
[foundations](../foundation/README.md). The [graph contract](../contracts/graph-projection-and-traversal.md)
and [interface evidence](../mappings/graph-interfaces-2026-09-15.md) specify the
testable boundary. The actual local Query API already proved an eligible longer
path survives exclusion of an intermediate node on the shorter route.

## Consequences

Graph work shares the existing single heavy lane and has bounded imports,
queries and cleanup. Outages remain visible without fabricating successful paths.
Current canonical qualification can be more expensive than querying a stale
graph directly; explicit scope limits and timeouts protect ordinary recall.
Cross-repository linking, analytics, graph exploration and retrieval fusion keep
their separate roadmap owners. No new model call, hash or strict version
handshake is introduced; [ADR 0003](0003-product-runtime.md) remains applicable.
The [Cognee/Atlas reuse review](../mappings/graph-reuse-review-2026-09-15.md)
records the libraries and implementation patterns reused, and the differences
that require Recollect's canonical authority boundary. Analytics and graph
rendering must use established engines/libraries rather than new implementations
of their underlying algorithms.

## Supersession

N/A: this implements the accepted graph baseline.
