# Local product development

## Purpose and Prerequisites

Run the locally delivered Recollect platform with Rust/Cargo, Node/npm, Python 3
and Docker Compose. Verified with Rust 1.94, Node 26 and Docker 29 on macOS ARM.
The [runtime ADR](../adr/0003-product-runtime.md) records the user's speed-first
development posture. Local credentials are environment-backed; no password hashes,
artifact digests or strict release/version gates are introduced.

## Procedure

From the repository root:

```sh
./scripts/dev.sh
```

Open `http://127.0.0.1:8787`. Sign in with `RECOLLECT_OWNER_USERNAME` and
`RECOLLECT_OWNER_PASSWORD` from the generated, ignored `.env` file. Do not paste
that file into logs or issues. Initial setup creates random credentials and sets
file permissions to 0600. Re-running setup preserves the existing file.

The command starts the repository's PostgreSQL/pgvector and Neo4j/GDS services,
applies missing migrations, builds the generated API client and browser, and runs
the native Rust API and worker. Ctrl-C stops both child processes; data and databases
remain. Stop only this project's databases with:

```sh
./scripts/docker.sh compose stop
```

PostgreSQL data belongs to the Compose-labelled `recollect_postgres_data` Docker
volume. Graph data/logs/plugins live under ignored `.data/neo4j/`. The abandoned
`.data/postgres/` directory, if present from the initial failed macOS bind-mount
attempt, is not used by the service. Do not remove volumes as a restart procedure.

Host database ports are loopback-only: PostgreSQL 55432, Neo4j HTTP 57474. Other
Docker projects and reference checkouts are untouched. The Docker wrapper uses a
repository-local anonymous registry configuration for public development images,
avoiding the blocked desktop credential helper without editing personal config.

For API-only iteration after initial setup, load the environment and run:

```sh
set -a
source .env
set +a
cargo run -p recollect-server -- serve
```

After server interface changes run `./scripts/generate-api.sh`, then
`npm --prefix web run build`. Generation checks for duplicate operation IDs before
building the typed client.
The browser is served from `web/dist` at the same origin. Brain Knowledge sources
supports imports, source history and overlapping views; see
[source evidence](evidence-collections.md). Artifacts live under
`RECOLLECT_ARTIFACT_DIR`, default `.data/artifacts`, and must be available to both
the API and worker.
The [workspace panel and companion](workspace-scope.md) provide checkout
registration and task/subagent scope without uploading ordinary source files.
The [publication workflow](repository-publication.md) adds isolated committed
extraction, shared snapshots and environment revision histories. Install the
companion's Enola adapter with `python3 scripts/setup-enola.py` before publishing.
An optional [OpenAI connection probe](provider-preflight.md) uses the user's
environment key with fixed synthetic text. The delivered [Brain model policy and
autonomous maintenance](provider-learning.md) use the selected Luna/embedding-large
pair through a shared gateway. Each Brain starts with transmission disabled. Once
configured, the native worker learns permitted sources, revises machine-maintained
memory and refreshes [handovers](procedures-and-handovers.md) without individual review.
The [graph panel](graph-projection-and-traversal.md) shows automatically built
repository/knowledge projections, exact inputs, evidence and bounded paths.
Use [combined repository graphs](graph-cross-repository-views.md) for exact
manifest-selected repositories and parsed pinned dependency links.

## Verification

```sh
curl -fsS http://127.0.0.1:8787/health/ready
cargo run -p recollect-agent -- health
./scripts/test-platform.sh
./scripts/test-ui.sh
./scripts/validate.sh
```

Readiness requires actual PostgreSQL, pgvector distance and Neo4j/GDS queries.
The authenticated installation health view shows individual connection state
and observed versions. A running container alone is insufficient.

The core Rust integration scenarios create uniquely named disposable
databases and exercise production handlers, authorization/RLS, atomic commands,
concurrent replay, lease recovery, retries, backpressure and dependency health.
They also cover invited identity, recovery and effective role/ownership changes.
Device scenarios cover browser approval, bearer authority and queued revocation.
Evidence scenarios cover real retained files, immutable versions, shared views,
capture policy, source spans and missing/storage-failure recovery.
Workspace scenarios cover private checkout paths, shared repository identities,
concurrent child scopes, immutable operations and effective revocation. Run
`cargo test -p recollect-protocol -p recollect-agent --lib` for the real Git/worktree
discovery and origin-normalization proof.
Publication adds immutable artifacts, contributor deduplication, scoped processing,
missing-artifact recovery and manifest history. The real native extraction fixture
is `cargo test -p recollect-agent --test publication` after Enola setup.
Claims/review scenarios cover temporal evidence, human review, rejection and
revalidation, conflict dispositions, replay and independent applicability. See
[claims and time](claims-and-time.md) and [review/corrections](review-and-corrections.md).
Retention scenarios cover deadlines, independent excerpts, dependency erasure,
cleanup retry, actual older-database replay and native offline-copy isolation.
The [retention runbook](retention-and-erasure.md) describes the durable erasure
journal, which must survive separately from database backups.
Model scenarios cover actual HTTP responses, policy/role/budget admission, explicit
retry, source learning, rejected/conflicting values and in-flight erasure/revocation.
Procedure/handover scenarios cover typed observations, combined scope, exact
contributors, synthesis and restoration. Autonomous scenarios cover catch-up,
same-identity revision, handover refresh, retirement, uncertainty, bounded retries,
durable human overrides and physical expiry without individual acceptance calls.
The opt-in real OpenAI browser workflow is documented in the
[model runbook](provider-learning.md).
The browser tests run desktop/mobile workflows with a fresh disposable database
per test file and a separate API on 8788,
using an isolated Chrome session. Google Chrome is required for that check;
neither personal browser state nor real workspace Brains are changed. Screenshots
are in `.cache/ui/`; traces/video are disabled to avoid recording credentials.
It also starts a fresh native worker to drain persisted queued work.
Test browser assets are built under `.cache/ui/web-dist`, preserving the normal
application's served bundle during focused browser checks.
Native companion tests create a unique OS-store profile and delete only that
test entry; see [device pairing](device-pairing.md).
Optional OIDC signed exchange and its full browser workflow are checked with
`./scripts/test-oidc.sh` and `./scripts/test-oidc.sh ui`; see [team access](team-access.md).

## Failure and Recovery

- Database unavailable: inspect `./scripts/docker.sh compose ps`. PostgreSQL's
  health check executes a database query, detecting incomplete initialization.
- Graph degraded: confirm the GDS plugin loaded and the configured graph target
  is reachable. Recollect does not label a failed Query API call connected.
- Login failed: verify local environment names without printing passwords.
  Ten attempts for one username per minute cause temporary throttling.
- Owner recovery: stop the API, edit the owner password in the ignored `.env`,
  load the environment as above, run `cargo run -p recollect-server -- recover-owner`,
  and restart. This re-enables the persisted owner and revokes its sessions; it
  does not change the owner identity or grant unrelated Brain access.
- Port conflict: stop only a known Recollect process, or adjust its bind and
  public origin together. Never kill an unrelated listener to free a port.
- Processing delayed: open the Brain's Processing panel for actual queue state,
  attempts and safe failure reasons. Admins can cancel queued/running work and
  retry failed/cancelled work. The worker log is `.cache/runtime/worker.log`.
  With the environment loaded, run `cargo run -p recollect-server -- worker`
  to resume the durable queue independently of the API. Interrupted leases
  recover after 20 seconds; three attempts exhaust automatic retries. Current
  canonical Brain metadata stays available while its projection catches up.

This runbook covers local bootstrap, durable work, team access, devices and
source evidence, workspace/task scope, committed repository publication, claims/review
and retention/erasure plus governed model learning. Delivered recall and graph
procedures have linked runbooks. Analytics, graph exploration, MCP/Vault
and shared/recovery operation retain their roadmap slices.
