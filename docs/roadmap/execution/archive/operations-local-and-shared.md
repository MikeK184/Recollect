# Personal/shared installation and operational diagnostics

Status: shipped
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: Reproducible personal/shared packages with actual health and diagnostics.
- Non-goals: External deployment, backup drills, integrated capacity claims or mobile.
- Delivery shape: Application image, Compose profiles, native bundle, installer,
  bounded process metrics, shutdown behavior and owned integration fixtures.

## Governing Sources

- [ADR 0011](../../../adr/0011-local-and-shared-installation.md)
- [Installation contract](../../../contracts/operations-local-and-shared.md)
- [Bootstrap](../../../contracts/platform-bootstrap.md)
- [Durable work](../../../contracts/platform-durable-work.md)
- [Owning epic](../../epics/operational-readiness.md)

## Scope

- In scope: Contracted packaging, isolation, HTTPS, resources, diagnostics and proof.
- Out of scope: Named successor slices and unrelated/global configuration.
- Blockers: None for this slice. The authorized Docker recovery and actual
  installation acceptance are complete; named successors remain separate.

## Surface and Interface Changes

- Interfaces: Named installer commands, native packaging and owner diagnostics API.
- Storage: Separate persistent installation volumes/configuration; no new DB schema.
- Ownership: Existing canonical handlers/RLS; deployment owns lifecycle and metrics.

## Data and Authority

- Inputs: Explicit instance name, mode/origin, selected environment and TLS files.
- Authority: Operator installation configuration and current authenticated owner.
- Blind spots: Configuration is not connectivity; process counters are not audit.

## States and Edge Cases

- Loading: Image build, healthy database dependencies and successful migration.
- Empty: New private configuration/volumes; no inherited customer fixture data.
- Error: Low disk, invalid input/TLS, failed build/migration or unreachable runtime.
- Blocked: Missing selected provider/credential or unavailable container engine.
- No-access: Owner-only diagnostics, normal Brain RLS and actual TLS/origin checks.
- Duplicate or replay: Initialization preserves existing credentials and ownership.
- Stale data: Live health and current service observations supersede saved settings.
- Reconciliation divergence: Privacy barrier and durable recovery remain mandatory.

## Integrations and Runtime Inputs

- Providers: Existing database images, Docker Compose, Caddy supplied-certificate TLS.
- Environment: Generated explicit operator/runtime/DB inputs; optional providers.
- Secrets: Ignored private files, migration-only DB admin, no inherited Vault root.
- Failure handling: Bounded calls/build preflight, graceful shutdown, persisted work.

## Tests and Acceptance

- Automated: Config preservation/ownership, actual Compose rendering, metrics and
  shutdown, real personal/two-user HTTPS installation and failure controls.
- Manual: Desktop inspection and normal installation preservation.
- Acceptance: Full contract and dated runtime evidence before archival.

## Closeout

- Planned: Complete installation and operational-diagnostic behavior.
- Shipped: Actual personal/shared application packages, desktop/API diagnostics,
  verified native HTTPS pairing/MCP, persistent state and operational failures.
- Not shipped: External deployment, backup/restore and integrated evaluations
  belong to explicit deferrals or named successor slices.
- New blockers: None.
- Docs updated: ADR, contract, pack, epic, indexes, mapping, runbook and continuation.
- Validation: Two actual Compose cases, native CLI/MCP TLS controls, three HTTP/
  cancellation/signal cases and real Caddy HTTPS/header/failure/drain proof pass.
  Release companion bundle passes TLS/configuration proof. Workspace 25 passed,
  144 fixture cases explicitly skipped in the ordinary suite; all-target Clippy
  passes in 36.82 s, 156-operation API generation and desktop
  bundle build and 32 governance checks pass. Actual desktop loading/error/retry
  and member denial, PostgreSQL owner/RLS checks and normal preservation pass
  after Docker recovery. Actual personal/shared Docker UI/API, browser-approved
  Linux OS-store pairing and native MCP recall, SIGTERM/SIGINT drain, failed
  migration, dependency outage, restart persistence and credential/log/resource
  boundaries now pass. See the [current acceptance](../../../mappings/installation-2026-09-26.md)
  for proof and its browser certificate-trust boundary.
- Version: N/A: no release policy.
- Commit: Uncommitted.
