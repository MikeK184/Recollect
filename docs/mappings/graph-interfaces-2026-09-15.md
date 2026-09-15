# Graph interface evidence — 2026-09-15

Status: current

## Sources and method

Read the accepted graph foundations, repository publication and canonical recall
contracts, the retained Enola facts schema under `.cache/enola-reference/v0.4.19/`
and actual Rust publication/materialization paths. Atlas research remains covered
by the [implementation audit](atlas-implementation-audit-2026-09-15.md).
Context7 `/websites/neo4j` returned mostly older HTTP API material;
`/websites/neo4j_cypher-manual_current` returned bounded shortest-path, inline
predicate and relationship-uniqueness examples but not the complete pre-filter
explanation. Use the official specific pages and runtime proof for that gap.

The Query API documentation root initially timed out. The specific official
[query page](https://neo4j.com/docs/query-api/current/query/),
[transaction page](https://neo4j.com/docs/query-api/current/transactions/) and
[shortest path page](https://neo4j.com/docs/cypher-manual/current/patterns/shortest-paths/)
were read successfully. Query responses require parsing errors even with HTTP
202. `maxExecutionTime` is seconds; parameters carry values; application IDs
avoid unstable internal element IDs. A WHERE outside the shortest selector is a
post-filter; enclosing the path and predicate inside parentheses is a pre-filter.

## Local proof

The existing repository-owned loopback Neo4j Query API returned Neo4j Community
2026.08.1 and GDS 2026.08.1. No product graph publication was established by that
version call. A separate synthetic, UUID-owned `RecollectQueryProof` fixture
actually exercised the proposed bounded pre-filter: A–B–D selected at first;
excluding intermediate B selected A–C–E–D; excluding its C–E edge returned no
path. The C–A cycle did not prevent completion. The exact owned fixture was
removed after proof. Evidence: `.cache/graph-proof/query-api.json`.

## Limits

The 100,001-fact admission fixture exposed a ten-second timeout in the existing
repository fact SELECT policy: it called the effective-role function for every
fact. PostgreSQL's [RLS documentation](https://www.postgresql.org/docs/18/ddl-rowsecurity.html)
and Context7 `/websites/postgresql_18` confirmed sub-SELECT policies and the need
to preserve concurrency boundaries. Migration 018 reuses the visible-Brain set
already used by semantic tables for this SELECT policy. The shared Brain locks
still fence publication/read authorization. The actual scoped/RLS and large-input
proof then passed in 8.91 seconds, including a 5,001-node read refusal and a
100,001-node unpublished input failure. This is fixture evidence, not a general
large-workspace performance claim.

The full regression also reproduced a timeout in the existing semantic
5,001-representation admission test. `EXPLAIN (ANALYZE,BUFFERS)` on that retained
owned fixture found 85,199 buffer hits in its source-chunk scan, dominated by
the same per-row role lookup (about 862 ms in a warm plan). The SELECT policy
for source chunks now uses the same visible-Brain set; write policies and the
two-second recall SQL limit remain unchanged. That policy repair reduced the
warm admission plan from about 1,052 ms to 197 ms, but the fresh-fixture query
still exceeded its bound before statistics caught up. The existing unique
semantic entry key now supplies a bounded lateral lookup for each canonical
candidate, preventing that unbounded join plan. The existing semantic-limit
regression then passed in 2.99 seconds including fixture setup, and the native
retention regression passed in 1.03 seconds. The native retention regression
needed a current timestamp for its retained control bundle: the old static
fixture date had legitimately crossed the configured one-day offline TTL.

This is interface/design evidence. The subsequent
[executed validation](graph-validation-2026-09-15.md) records the implementation,
failure scenarios, canonical gates and browser proof used for slice closeout.
No customer source read, external model call or reference-checkout mutation was
needed for these checks.
