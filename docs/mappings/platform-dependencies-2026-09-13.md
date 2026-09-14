# Platform dependency interfaces

Observed: 2026-09-13
Confidence: observed-once

## Sources and Method

Successful Context7 resolve/query calls for `/tokio-rs/axum`,
`/transact-rs/sqlx` and `/mantinedev/mantine` confirmed Router/State/serve,
PostgreSQL pools and transactions, and MantineProvider/style imports.
Primary sources checked:

- [Axum routing](https://docs.rs/axum/latest/axum/)
- [SQLx features](https://github.com/transact-rs/sqlx)
- [Mantine Vite setup](https://mantine.dev/guides/vite/)
- [pgvector Docker tags](https://github.com/pgvector/pgvector#docker)
- [Neo4j Docker GDS plugin](https://neo4j.com/docs/graph-data-science/current/installation/installation-docker/)
- [Neo4j Query API](https://neo4j.com/docs/query-api/current/)
- [Utoipa OpenAPI](https://docs.rs/utoipa/latest/utoipa/)

## Observations

The available native toolchain is Rust/Cargo 1.94 and Node 26/npm 11. Docker
Engine 29.7.2 answers on linux/aarch64 and initially had no running containers.
The upstream pgvector image supports a PostgreSQL 17 family tag. Neo4j documents
the `graph-data-science` Docker plugin and Community images without an enterprise
suffix. Context7 calls succeeded; no fallback was needed for those three libraries.

## Translation and Limits

Use compatible ranges and family tags under the user's no-strict-versioning
instruction. Documentation and Docker availability are not runtime compatibility
proof. Record actual package resolution and successful database/API calls in the
bootstrap closeout. Graph algorithms and model providers need later slice proof.

## Follow-up

Build, live database/health calls, browser inspection and authorization/recovery
proof completed in the [bootstrap proof](platform-bootstrap-proof-2026-09-13.md).
Recheck new dependency interfaces as subsequent slices require them.

Durable-work interface check, same session: reuse the verified SQLx transaction
interface. Primary [PostgreSQL SELECT](https://www.postgresql.org/docs/current/sql-select.html),
[row locking](https://www.postgresql.org/docs/current/explicit-locking.html) and
[Tokio JoinSet](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html) docs
confirm the queue locking/bounded-task primitives. No new dependency is introduced.
Runtime contention, recovery and fencing still require the durable-work proof.
