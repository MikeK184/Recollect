# Encrypted backup and recoverable installation

Status: shipped
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: Restore a coherent installation without resurrecting erased/rejected
  checkpoint knowledge or replaying uncertain external actions.
- Non-goals: Production cutover, zero-loss replication, high availability or mobile.
- Delivery shape: Named operator commands, optional durable journal mirror,
  startup recovery barrier, canonical rebuild and actual isolated restore drills.

## Governing Sources

- [ADR 0013](../../../adr/0013-backup-and-recovery.md)
- [Recovery contract](../../../contracts/operations-recovery-drills.md)
- [Installation](../../../contracts/operations-local-and-shared.md)
- [Retention/erasure](../../../contracts/memory-retention-and-erasure.md)
- [Review/corrections](../../../contracts/memory-review-and-corrections.md)
- [Managed observations](../../../contracts/mcp-observation-capture.md)
- [Owning epic](../../epics/operational-readiness.md)

## Scope

- In scope: Full recovery contract, including encryption/transport/retention,
  separate journal durability, failed upgrade and permitted retained controls.
- Out of scope: Integrated workload/quality evaluation and external deployment.
- Blockers: None. Feature predecessors are shipped; current external targets are
  explicitly owned disposable installations and an isolated SFTP fixture.

## Surface and Interface Changes

- Interfaces: Named backup/status/prune/transfer/restore commands and scheduled
  entrypoint; optional journal mirror and privileged recovery preparation.
- Storage: Encrypted UUID/time archives, private status/hold records and separate
  deletion/analytics journals; reuse canonical tables and migrations.
- Ownership: Operator tooling manages packages and volumes; existing Rust privacy,
  authority, jobs, graph and runtime modules retain canonical decisions.

## Data and Authority

- Inputs: Validated saved installation, recipient and path/key references,
  authenticated archive, independently retained journals and destination config.
- Authority: Canonical checkpoint plus newer durable deletion fences; scoped
  review rules and ordinary access/policy state are recovered to that checkpoint.
- Blind spots: Ordinary post-checkpoint changes outside the stated recovery point,
  lost latest journal copies, disconnected devices and external source state.

## States and Edge Cases

- Loading: Drain, copy/encrypt/transfer, authenticate/import/reconcile and rebuild.
- Empty: Unconfigured recovery, no completed backup or fresh target volumes.
- Error: Storage/tool/transport failure, wrong key, malformed archive or migration.
- Blocked: Missing current journal/artifact, nonempty target or unavailable inputs.
- No-access: Fixed installation/remote ownership and verified SSH host identity;
  no ambient root Vault or inherited SSH proxy/configuration.
- Duplicate or replay: Stable backup/request identities; preserve completed archives,
  retry receipts/entries only, hold interrupted restores without external replay.
- Stale data: Checkpoint age and policy expiry; no imaginary post-checkpoint state.
- Reconciliation divergence: Latest deletion journal wins over older data; graph
  rebuild follows canonical authority and uncertain calls remain uncertain.

## Integrations and Runtime Inputs

- Providers: Existing PostgreSQL 17/pgvector and Neo4j/GDS, age 1.3.2-compatible
  X25519 CLI, OpenSSH SFTP and existing Compose installation tooling.
- Environment: Explicit selected-installation files and optional mirror references;
  no credential values in docs/output or provider guesses.
- Secrets: Operator age identity stays outside the runtime; mirror uses public
  recipients and a restricted SSH identity reference with a known-hosts file.
- Failure handling: Bounded subprocesses, durable status, private staging and
  fail-closed startup; no resets, unsupported downgrades or Vault revocations.

## Tests and Acceptance

- Automated: Contract's actual encrypted transfer/fresh-volume restore and paired
  failure cases, erasure/rejection/replay/rebuild, migration and retention controls.
- Manual: Inspect restored desktop evidence/readiness and verify normal installation
  state and unrelated services remain preserved.
- Acceptance: Full contracted behavior and measured corpus/timing limits, focused
  runtime proof and required checks before marking shipped.

## Closeout

- Planned: Complete encrypted backup/recovery with independent deletion durability.
- Shipped: Named encrypted backup/transfer/fetch/retention, optional synchronous
  deletion and analytics journal durability, offline fresh-target recovery,
  forward schema validation, durable holds and recoverable interrupted work.
  Actual restored recall/history/scope, graph rebuild and receipt-only uncertain
  effect reconciliation passed. Current erasure and replay remained excluded.
- Not shipped: External deployment, global scheduler installation, universal
  capacity guarantees and integrated workload evaluation are outside this slice.
- New blockers: None; the integrated evaluation successor still needs its contract.
- Docs updated: ADR, contract, epic/indexes, [evidence](../../../mappings/recovery-2026-09-26.md),
  [runbook](../../../runbooks/recovery.md) and continuation record.
- Validation: Actual SFTP and fresh-volume drills, nine refused preflights,
  failed upgrade, killed publication/operator, offline mirror continuity, final
  resumed desktop (3.7 s), 26 runtime scenarios, 26 workspace tests, all-target
  Clippy, generated API/web, focused Python checks and governance's 32 tests.
  Final interrupted restore/resume/product verification took 28.848 s for the
  recorded small corpus. Normal login/inventory preserved seven Brains, 33 model
  requests, 24 migrations and zero active jobs before its separate local upgrade.
- Version: N/A: no release policy.
- Commit: Uncommitted.
