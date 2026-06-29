# Platform bootstrap

Status: accepted

## Source

[Runtime decision](../adr/0003-product-runtime.md),
[platform epic](../roadmap/epics/product-platform.md) and the accepted
[vision](../foundation/vision.md). The user authorized implementation and routine
contract decisions on 2026-09-13, including the speed-first override in the ADR.

## Contract

### Packaging and startup

`./scripts/dev.sh` initializes absent local environment values, starts the two
repository-owned databases, applies pending migrations, builds the React assets
and starts the native Rust API on `127.0.0.1:8787`. It preserves existing data
and credentials. Rust protocol types contain no database dependencies; the
agent binary supplies a health command using that protocol. The operations slice
owns full containerized application deployment. Native Rust/Node and Docker are
prerequisites. Schema migrations are ordered and transactional, without custom
checksum/version gates. Package lockfiles record resolved dependencies without
enforcing an exact host toolchain or container digest.

### Identity and commands

The first migration command, invoked by local startup, creates one configured
local owner if no owner exists.
The username and password come from `RECOLLECT_OWNER_USERNAME` and
`RECOLLECT_OWNER_PASSWORD`; the password never enters database rows or output.
The installation owner administers installation health but has knowledge access
only to owned or explicitly granted Brains. No public account registration exists.
Owner bootstrap is serialized and restart-safe. Changing the configured username
after bootstrap does not create another owner; recover the existing owner instead.

`POST /api/auth/login` accepts username/password JSON. A valid login returns user
and CSRF metadata and sets an opaque HttpOnly, SameSite=Strict, 12-hour cookie.
`GET /api/auth/me` checks the current persisted session and enabled account.
`POST /api/auth/logout` revokes that session. Mutating cookie-authenticated calls
require `X-CSRF-Token`; requests with a browser Origin must match the configured
public origin, including login. Wrong credentials return a generic 401; bounded
in-memory throttling permits at most ten attempts per username per minute.
Recovery is an operator-only CLI command that invalidates owner sessions; the
operator changes the password in the ignored environment and restarts the server.

`GET/POST /api/brains` lists accessible Brains or creates an active Brain owned
by the caller. `GET/PATCH /api/brains/{id}` reads or renames/archives/reopens one.
Names are trimmed, 1–120 characters; descriptions allow up to 2,000 characters.
Archive preserves data and history. Each resource has one UUID and owner.
`reader`, `writer`, `admin` grants are additive to ownership; removing a grant
cannot remove ownership. Grant administration and invited identities are owned
by platform-team-access. Only admins can change Brain metadata or view its audit.
Missing and inaccessible IDs both return 404. Authenticated readers receive 403
for admin operations, with an authorized owner positive control.

Every Brain mutation authenticates the actor, starts a transaction, binds the
actor to that transaction, checks effective access, and writes canonical changes
plus audit atomically. Audit records actor, Brain, action, target, time and
non-sensitive disposition metadata; it excludes credentials and source payloads.
`GET /api/brains/{id}/audit` returns the latest 100 events to a Brain admin.
The runtime DB role cannot bypass RLS and does not own tables. A security-definer
effective-role function reads grants under a fixed search path; HTTP callers
cannot choose another actor. Installation owner status does not bypass Brain RLS.

### Health, browser and failures

`/health/live` reports process liveness. `/health/ready` returns 200 only after
real PostgreSQL, pgvector and Neo4j/GDS calls succeed; otherwise 503. Authenticated
`GET /api/status` exposes individual dependency status and observed versions,
never credentials. Graph checks time out after five seconds; database connection
acquisition is bounded. A configured graph without a successful query is degraded.
Database failures return a generic 503 without exposing driver payloads.

`GET /api/openapi.json` describes the implemented API using Utoipa; the frontend
uses types generated from it. Unknown API routes return JSON 404 rather than the
SPA. The browser provides login/logout, Brain listing/create/edit/archive/reopen,
audit and service health, with explicit loading, empty, forbidden, expired-session
and server-error behavior. Cached data is cleared at logout. Brain identity is
in the route and never carried as mutable authentication-session scope.

## Acceptance

Build the Rust workspace and React application; use real HTTP handlers and
PostgreSQL to prove login/logout, owner restart/recovery, invalid input, CSRF,
unauthorized requests, two-Brain isolation, reader denial/owner success, atomic
audit, persisted Brain state and non-owner RLS. Check PostgreSQL/pgvector and
Neo4j/GDS through real calls, including a degraded graph target. Inspect the
browser workflow. Run focused proof plus `./scripts/validate.sh` before closeout.

## Explicit Deferrals

Outbox/jobs, invitations/OIDC and device pairing remain in their platform slices.
Knowledge, retrieval, graphs beyond connectivity, MCP and full shared operation
remain in the rest of the 29-slice goal. Bootstrap success does not establish them.
