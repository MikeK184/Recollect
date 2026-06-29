# Personal/shared installation and diagnostics

Status: accepted

## Source

[ADR 0011](../adr/0011-local-and-shared-installation.md),
[bootstrap](platform-bootstrap.md), [durable work](platform-durable-work.md),
[pairing](platform-device-pairing.md),
[Vault/private execution](mcp-vault-and-private-runners.md) and the
[operational epic](../roadmap/epics/operational-readiness.md).

## Contract

### Package and instance ownership

Build one Linux application image from the locked Rust and web dependencies,
generating the web API types from that server during the image build. The runtime
contains the release server, desktop assets and certificate/health prerequisites.
Build context excludes secrets, data, caches, source control and reference
checkouts. Do not add product content hashes, digest gates or exact host-version
handshakes. Initial supported hosts are Linux with Docker Engine/Compose and
macOS with Docker Desktop/Compose; native companion binaries target their build
host. Record versions actually tested rather than claiming untested portability.

`scripts/install.py` creates an explicitly named personal or shared installation
under the repository's ignored `.data/install/`. Its generated credentials and
configuration files are private. Repeating initialization preserves them; conflicting
mode/origin/port/path inputs fail visibly. Names are bounded simple identifiers;
never infer a project from a customer directory or attach the native development
volumes. Commands select the exact saved configuration and Compose project.
An installation-local `overrides.yaml` can explicitly add provider-specific
executable/socket mounts or a selected runtime image. Render and validate this
overlay before starting; required data volumes remain in the saved project and
administrative credentials never enter API/worker environments.
Offer build, start, stop and read-only diagnostics. Stop preserves every volume;
there is no erase/prune/reset command. Local fixture removal names only its own
resources. Disk-space checks precede builds; do not auto-delete caches or data.

Personal and shared profiles share PostgreSQL 17/pgvector, Neo4j 2026.08/GDS,
migration, API and worker definitions. Preserve the existing compatible database
image lines. PostgreSQL, Neo4j and API/worker artifact, account-credential,
erasure-journal and central receipt paths use installation-owned persistent
volumes. The migration role alone receives the administrative DB URL and must
complete successfully before API/worker startup. Startup does not bypass privacy
reconciliation. Starting an existing installation first drains its API/worker/proxy,
then reruns migration; a failure leaves those application roles stopped rather
than serving against an unverified schema. Running application roles retain the
non-owner DB role.

### Access and operating inputs

Personal HTTP publishes on `127.0.0.1` only. Shared mode publishes HTTPS through
Caddy with a fixed public origin and operator-provided PEM certificate/key;
databases and the API have no shared host port. Keep browser Origin and upstream
Host semantics intact. No automatic public certificate request, system trust-store
change, HTTP credential fallback or external deployment occurs. A missing,
invalid or untrusted certificate fails explicitly. The owned HTTPS fixture uses
its own CA with explicit client trust and an untrusted-client negative control.

Native clients accept an optional `RECOLLECT_CA_FILE` containing at most 256 KiB
and 16 PEM certificates. Add those roots to normal certificate/hostname
verification for the configured Recollect service, including the native MCP HTTP
bridge. Invalid/missing selected files fail explicitly; never disable TLS
verification. Generated host settings freeze the selected absolute CA path.
This is client-local trust, without a machine-global certificate installation.

Retain configured owner bootstrap, invited accounts, optional OIDC and native
pairing. Neither a model key nor Vault is required for startup or basic knowledge
operations. The existing configured text and embedding models stay unchanged.
Operator-supplied runtime environment files may contain selected provider inputs;
do not copy the shell environment or the normal `.env`. Never inject the root
Vault token. Optional Vault Proxy/OS-store/stdio prerequisites belong to the
selected runner; unselected providers cannot block other connections. Container
stdio tools must exist in the operator's explicit runtime image; paired local and
private runners retain native paths and OS stores.

The companion packaging command builds and copies `recollect-agent`,
`recollect-mcp-runner` and `recollect-mcp-bridge` together into an explicit
repository-local output directory. It does not edit personal PATH, keychains,
host configuration or global Codex settings.

### Shutdown, resources and diagnostics

API and worker handle SIGINT and SIGTERM. Shutdown stops new work and drains
bounded in-flight work; Compose allows 90 seconds before forced termination.
Hard-stop recovery still uses persisted jobs, leases, receipts and privacy state;
termination never revokes a Vault token or lease. A dead worker must be visible
as a stopped/restarting service, not an invented healthy worker report.

Initial per-service memory/CPU/PID budgets are explicit and configurable. Default
memory ceilings are PostgreSQL 1 GiB, Neo4j 3 GiB, API 1 GiB, worker 2 GiB,
migration 1 GiB and optional proxy 128 MiB. Existing job lane, MCP concurrency and
context bounds remain in force. Logs rotate at 10 MiB with three files per service;
they contain fixed operational fields, safe error codes, counts and durations.
Proxy request access logging is disabled. Operator diagnostics exclude container
environment, command arguments, raw database output and arbitrary logs.

`GET /api/operations` requires a current installation owner. Return process
instance/uptime, bounded HTTP request/error/status counters and fixed cumulative
latency buckets, application DB-pool counts, and queued/running/failed job counts
under that caller's existing Brain RLS. Label the latter as accessible-Brain
counts; installation ownership never grants knowledge access to other Brains.
No request identifiers, user/Brain labels, paths, query strings or source/secret
content become metric dimensions. Counters are process-local and reset on restart;
they do not replace durable mutation audit or domain histories. Existing health
endpoints remain the authority for actual dependency connectivity.
The desktop installation-health section exposes these diagnostics only to the
installation owner, with loading, current error/retry and empty-counter states.

Read-only installation diagnostics report saved mode/origin, actual Compose
service state, bounded CPU/memory observations and actual HTTP liveness/readiness.
Report missing Docker, unavailable API, unhealthy dependencies or absent runtime
as distinct failures. Never equate a rendered configuration or running container
with a working dependency. Keep UI, API and native diagnostics consistent.

## Acceptance

Render both profiles with actual Compose; inspect complete service/environment,
port, dependency and volume boundaries without printing credentials. Build the
application image, start fresh personal and shared owned installations, and prove
UI/API owner login, second-user invite/Brain isolation and native pairing over
trusted HTTPS with invalid-origin and untrusted-TLS denials. Run useful permitted
memory reads and actual worker jobs; restart services and verify persisted state.
Prove migration failure blocks application startup and dependency failure produces
503 plus accurate diagnostics. Verify SIGTERM and SIGINT drain owned native or
container processes without revoking Vault credentials.

Exercise real HTTP metric collection and owner-only/RLS diagnostics, including
secret-bearing request canaries absent from telemetry. Prove initialization
preserves existing files and normal development state, both fixture profiles have
bounded resources/logs, and the complete native binary bundle is usable. Run
focused tests, workspace/Clippy/API/web checks and `./scripts/validate.sh`. Keep
Docker recovery and unexecuted integration requirements visible before archival.

## Explicit Deferrals

External installation, DNS/certificate issuance, high availability, Kubernetes,
global telemetry collectors, Windows binary support and automatic updater policy.
Encrypted off-machine backup/restore and integrated workload/capacity/quality
proof remain in the named operational successor slices.
