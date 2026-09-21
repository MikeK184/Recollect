# Runnable product platform

Status: shipped
Owning epic: `docs/roadmap/epics/product-platform.md`
Work type: product

## Summary

- Goal: Start Recollect locally, sign in, manage isolated Brains and observe real database health.
- Non-goals: Later platform/domain slices and external deployment.
- Delivery shape: Uncommitted local Rust/React application, Compose dependencies and operating documentation.

## Governing Sources

- [Runtime ADR](../../../adr/0003-product-runtime.md)
- [Bootstrap contract](../../../contracts/platform-bootstrap.md)
- [Accepted stack](../../../foundation/techstack.md)
- [Owning epic](../../epics/product-platform.md)

## Scope

- In scope: Cargo protocol/server/agent structure, same-origin browser, Postgres/pgvector and Neo4j/GDS connectivity, migrations, owner sessions/recovery, auditable Brain commands, RLS and local start.
- Out of scope: Durable jobs, team enrollment and pairing, assigned to subsequent platform slices.
- Blockers: None; routine details resolved in the accepted contract, including the user's hashing/version override.

## Surface and Interface Changes

- Interfaces: The health, auth, Brain, audit and OpenAPI endpoints defined in the bootstrap contract; server migrate/recover-owner/openapi commands and agent health command.
- Storage: Accounts, sessions, Brains, grants, mutation audit and ordered transactional migrations in PostgreSQL; a project-labelled PostgreSQL volume and ignored `.data/` graph/artifact directories.
- Ownership: Protocol types shared without database drivers; server owns auth/storage/HTTP, browser owns interaction, scripts own repository-local setup.

## Data and Authority

- Inputs: Environment credentials, validated JSON commands, persisted sessions/grants and actual dependency responses.
- Authority: PostgreSQL canonical rows and audit under authenticated command transactions; accepted contract determines behavior.
- Blind spots: No memory content, deployed infrastructure or graph projection exists in this slice.

## States and Edge Cases

- Loading: Browser query/mutation indicators; bounded connection and graph-call waits.
- Empty: First-login Brain creation prompt and explicit empty audit/list states.
- Error: Validation 400, login 401, privilege/CSRF 403, inaccessible resource 404, dependency 503; browser exposes retry.
- Blocked: Dependency health stays degraded until an actual check succeeds.
- No-access: RLS plus command checks; owner status does not bypass unrelated Brain access.
- Duplicate or replay: Owner bootstrap/migrations serialize; Brain create uses fresh UUIDs and each successful command is audited. Request idempotency follows in durable-work.
- Stale data: Sessions checked per call; browser refetches after mutations, clears cached data on logout; readiness calls live dependencies.
- Reconciliation divergence: Audit and canonical writes share a transaction; database/graph health shown separately.

## Integrations and Runtime Inputs

- Providers: Repository-owned PostgreSQL/pgvector and Neo4j/GDS; no external model or mandatory identity provider.
- Environment: DATABASE_URL, DATABASE_ADMIN_URL, RECOLLECT_OWNER_USERNAME, RECOLLECT_OWNER_PASSWORD, RECOLLECT_BIND, RECOLLECT_PUBLIC_ORIGIN, RECOLLECT_STATIC_DIR, NEO4J_URL, NEO4J_USERNAME, NEO4J_PASSWORD, POSTGRES_PASSWORD, RECOLLECT_DB_PASSWORD.
- Secrets: Ignored `.env`, restricted file permissions; no credentials in logs/docs/fixtures or returned health.
- Failure handling: Five-second graph timeout, bounded DB pool, generic errors, rerunnable non-destructive setup and operator session recovery.

## Tests and Acceptance

- Automated: Focused real-handler/database lifecycle and authorization tests, build/type checks, dependency probes, migration replay and `./scripts/validate.sh`.
- Manual: Browser login/create/edit/archive/reopen/audit/logout and inspect loading/error/empty presentation.
- Acceptance: All bootstrap contract behavior proven by actual local paths; no claim that downstream slices ship with bootstrap.

## Closeout

- Planned: Runnable Rust/React foundation and real databases with local identity, Brain isolation and audit.
- Shipped: Cargo protocol/server/agent workspace; same-origin React/Mantine UI and generated API types; repository-owned PostgreSQL/pgvector and Neo4j/GDS; repeatable local startup; environment-backed owner sessions/recovery; auditable Brain administration and tested RLS.
- Not shipped: N/A for this slice. Durable work, team access, pairing and all domain capabilities remain explicit successor slices; external deployment is excluded.
- New blockers: None for bootstrap. Browser plugin startup was unavailable; isolated Chrome proof completed the UI acceptance without changing personal configuration.
- Docs updated: Runtime ADR, bootstrap contract, dependency/runtime mappings, local-development runbook, root/foundation navigation and all owning lifecycle indexes.
- Validation: Two real-handler/database integration scenarios passed, including degraded graph and rollback controls; one desktop/mobile Chrome workflow passed; visual screenshots inspected; one-command dev startup and agent/readiness calls passed. Rust build/format/Clippy, frontend generation/type/build, governance lint and all 32 checker tests passed. See [proof mapping](../../../mappings/platform-bootstrap-proof-2026-09-13.md).
- Version: N/A: no application release; ordinary package metadata/lockfiles only, under the user's no-strict-versioning instruction.
- Commit: uncommitted.
