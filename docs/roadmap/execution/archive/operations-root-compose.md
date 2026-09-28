# One local Compose stack

Status: shipped
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: Open the existing app at port 8787 and start/stop all normal services as one Compose project.
- Non-goals: UI changes, new infrastructure, data resets, external deployment or test-stack consolidation.
- Delivery shape: Root Compose, local scripts, documentation and actual local conversion.

## Governing Sources

- [ADR 0011](../../../adr/0011-local-and-shared-installation.md), including the user-authorized root follow-up.
- [Installation contract](../../../contracts/operations-local-and-shared.md).
- [Operational epic](../../epics/operational-readiness.md).

## Scope

- In scope: Existing database volumes, artifacts and journals; container API/worker; migration gate; native startup compatibility; verified fixture shutdown.
- Out of scope: Schema/product changes, credential rotation, deletion, Vault changes and unrelated projects.
- Blockers: None.

## Surface and Interface Changes

- Interfaces: Root `compose.yaml`; `scripts/stack.sh up [--build]`, `stop`, `down`, `status`, `logs`; existing `dev.sh`.
- Storage: Existing root PostgreSQL and graph storage retained; narrowly mounted `.data/runtime/` for account state and receipts. Legacy default account credentials preserved and configured for both launch modes.
- Ownership: Root `recollect` project; named `install.py` profiles remain independent.

## Data and Authority

- Inputs: Existing ignored `.env`, data directories and selected product image built from this checkout.
- Authority: Current user request and accepted installation decisions; no screenshot-based container mutation.
- Blind spots: Native-only connector executables and OS stores remain native runner prerequisites; no configured normal central connections are assumed without inspection.

## States and Edge Cases

- Loading: Wait for databases, migration completion and API readiness.
- Empty: Generate only missing local credentials/directories.
- Error: Failed migration prevents application startup.
- Blocked: Occupied API port rejects concurrent native/container startup.
- No-access: Missing credentials or unsupported custom account paths fail visibly without printing values.
- Duplicate or replay: Repeated setup preserves secrets; repeated startup uses the same project and persistent mounts.
- Stale data: Compare current database/account/Brain inventory before and after conversion.
- Reconciliation divergence: No restore or merging of independent proof datasets.

## Integrations and Runtime Inputs

- Providers: Existing PostgreSQL/pgvector and Neo4j/GDS; product Dockerfile; optional model provider retains Brain policy.
- Environment: Explicit allowlist of application inputs; administrative DB URL only on migration.
- Secrets: Ignored local files and container environment; never log resolved Compose environments.
- Failure handling: 90-second shutdown, bounded readiness waits, existing application retry/recovery behavior.

## Tests and Acceptance

- Automated: Focused resolved-Compose and launch failure checks; `./scripts/validate.sh`; `git diff --check`.
- Manual: Actual root image build, existing-data conversion, browser owner login, health, stack stop/start and retained inventory.
- Acceptance: Four long-running services and successful one-shot migration in one group; URL and simple commands documented; no duplicate native writers; preserved data and secrets.

## Closeout

- Planned: Complete root Compose workflow and verified local conversion.
- Shipped: Complete root Compose workflow; current app converted with preserved data, browser owner login/readiness, full stop/start and migration-failure gate verified. Six proof installations and three labeled fixtures stopped with data retained.
- Not shipped: UI work and external deployment. Subsequent authorized fixture deletion is recorded as `operations-proof-cleanup` in the owning epic and dated evidence.
- New blockers: None.
- Docs updated: Installation ADR/contract, epic and indexes, root README, installation runbook, continuation record and [dated evidence](../../../mappings/root-compose-2026-09-26.md).
- Validation: Actual product image build and Chrome login/dashboard/diagnostics; identical Brain/account inventory and model/migration/MCP counts across conversion/restart; readable retained artifacts/journals; occupied-port and failed-migration denials; effective runtime credential boundaries; three focused setup/Compose tests; shell syntax; 32 governance tests and diff checks passed.
- Version: N/A; no release requested.
- Commit: Uncommitted.
