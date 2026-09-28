# Personal and shared installation

Status: accepted
Date: 2026-09-22

## Decision

Package the existing Rust server and generated desktop assets in one application
image. Personal and shared installations use the same Compose services:
PostgreSQL/pgvector, Neo4j/GDS, one explicit migration role, API and worker.
Personal access publishes only loopback HTTP. A shared overlay adds Caddy HTTPS
with operator-provided certificate/key files and exposes only that proxy.
Native companions remain native and install all three sibling executables.

Each installation has an explicit name, its own configuration directory and
Compose project/volumes. The existing native development stack remains separate.
Generate absent credentials locally, preserve existing configuration, and keep
administrative database credentials exclusive to the migration role. Do not
inherit shell secrets into containers or require Vault, OIDC, a model key or a
telemetry service to start. Models still require the existing standing Brain
policy and an actually configured provider for paid operations.

Use bounded structured logs, process HTTP counters/latency buckets and existing
canonical job state for diagnostics. Do not log request paths, queries, headers,
bodies, source text or credential values. Honor SIGINT and SIGTERM, drain bounded
work, and retain durable recovery records after forced termination.

## Why

The authorized full-product goal permits routine packaging decisions and owned
local fixtures. The [foundation](../foundation/techstack.md#delivery-and-verification)
selects Compose, native companions, private shared HTTPS and resource limits.
The owning slice depends only on completed platform work, so implementation may
proceed independently while later MCP integration tests await Docker recovery.
Official Compose dependency conditions and Caddy supplied-certificate support
are verified in the [mapping](../mappings/installation-2026-09-22.md).

## Consequences

No Kubernetes, mandatory Vault/SSO, external deployment or new memory runtime is
introduced. The operator supplies DNS, trusted certificates and any external
connection inputs. Owned test certificates never enter a machine-global trust
store. A configured package is not a verified installation; actual personal and
two-user HTTPS flows, restart, failure and preservation checks remain required.
Container limits are explicit initial budgets, not a measured capacity claim.
Backup/recovery and integrated capacity evaluations retain their successor slices.

## Supersession

N/A: implements the packaging seam reserved by platform bootstrap. Its existing
native development startup remains valid; no source or database migration is
silently applied to the normal running installation.

## Root Compose follow-up — 2026-09-26

The user now requests one Compose group for the existing local app. Extend the
root `compose.yaml` with the same application image, migration, API and worker
roles, retaining the root project's PostgreSQL volume, graph bind mounts,
artifacts, deletion journal and loopback origin. This explicitly authorized local
conversion is separate from named installations; `install.py` still cannot adopt
development volumes. Native development remains an alternate, exclusive runtime.
Keep application state in a narrowly mounted runtime directory and pass only
selected environment inputs. Preserve legacy account credentials when preparing
that directory; do not expose installation/operator files to application roles.
