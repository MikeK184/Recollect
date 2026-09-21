# Durable commands and bounded workers

Status: shipped
Owning epic: `docs/roadmap/epics/product-platform.md`
Work type: product

## Summary

- Goal: Canonical commands survive retries and background processing survives API/worker interruption with visible, scoped state.
- Non-goals: Domain consumers or external side-effect retries, shared deployment and release work.
- Delivery shape: Local Rust/SQL/browser changes, focused runtime proof and reconciled documentation.

## Governing Sources

- [Runtime ADR](../../../adr/0003-product-runtime.md)
- [Durable-work contract](../../../contracts/platform-durable-work.md)
- [Bootstrap contract](../../../contracts/platform-bootstrap.md)
- [Owning epic](../../epics/product-platform.md)

## Scope

- In scope: Atomic canonical/audit/job/receipt transactions; bounded lease-based workers; a real Brain-directory consumer; scoped processing/job API and controls; local worker startup/recovery.
- Out of scope: Knowledge extraction, semantic models, graph computation and MCP consumers owned by later slices.
- Blockers: None; platform-bootstrap is shipped and routine durability decisions are resolved in the accepted contract.

## Surface and Interface Changes

- Interfaces: Idempotency-Key on Brain create/update; processing/jobs/retry/cancel endpoints; native worker/worker-once roles; job/processing protocol types and generated browser client.
- Storage: Transactional jobs/outbox, command receipts and Brain-directory projection; a change UUID for projection eligibility; migration extends existing canonical state without resetting it.
- Ownership: Shared command helpers own receipts/audit/enqueue; code-owned consumers use worker claim/heartbeat/publication; feature UI displays canonical and processing state independently.

## Data and Authority

- Inputs: Authenticated actor, explicit Brain, validated normalized JSON, optional idempotency key and current canonical inputs.
- Authority: PostgreSQL canonical rows, grants, mutation audit and committed job/receipt records; projections never grant access or overwrite canonical state.
- Blind spots: No external side-effect completion or unimplemented domain-handler behavior is inferred from generic queue success.

## States and Edge Cases

- Loading: UI request indicators and job polling; bounded worker/database operations.
- Empty: Explicit no-jobs/missing-projection state, with canonical metadata still readable.
- Error: Input 400, receipt/transition conflicts 409, queue capacity 429, database 503; job errors use safe codes and remain visible.
- Blocked: Failed/exhausted jobs need an authorized retry; unsupported consumer kinds fail instead of claiming completion.
- No-access: RLS/API checks and worker revalidation; revoked actors cannot publish; unrelated permitted jobs continue.
- Duplicate or replay: A shared key serializes equal commands and returns the stored response after access checks; mismatched input conflicts; leased processing is idempotent and fenced.
- Stale data: Projection change UUID must match canonical state; old job tokens cannot publish; new canonical fields remain visible before projection catches up.
- Reconciliation divergence: Durable jobs remain queued during outages; expired leases recover after restart; projection/audit/completion share a transaction.

## Integrations and Runtime Inputs

- Providers: Existing SQLx/Tokio/PostgreSQL integration from bootstrap; no new external provider or library required.
- Environment: Existing DATABASE_URL and local configuration; worker execution does not require DATABASE_ADMIN_URL.
- Secrets: Exclude credentials/driver payloads from job records, receipts and telemetry; API receipts contain only validated Brain metadata.
- Failure handling: 20-second token leases, renewal primitive, three attempts with 1/2-second retry delays, globally bounded lanes and per-Brain backpressure; explicit cancel/retry.

## Tests and Acceptance

- Automated: Focused concurrent real-handler and worker/database scenarios for atomicity, replay, lease recovery/fencing, retries, revocation, backpressure, freshness and isolation; build checks and `./scripts/validate.sh`.
- Manual: Live native worker consumes queued work after restart; browser displays actual state and authorized controls.
- Acceptance: Every durable-work contract behavior has current-state proof, with downstream consumers explicitly unimplemented.

## Closeout

- Planned: Atomic command/outbox support, bounded durable workers and visible processing state.
- Shipped: Atomic command receipts/audit/outbox, bounded leased workers, current Brain-directory projection, scoped processing/job API, browser cancel/retry and combined local startup.
- Not shipped: Domain consumers and external-effect reconciliation remain with their named slices; no shared deployment or release.
- New blockers: None.
- Docs updated: Durable-work contract, this archived pack, owning epic/indexes, local-development runbook and dated proof mapping.
- Validation: [Local proof](../../../mappings/platform-durable-work-proof-2026-09-14.md): four Rust integration scenarios, one desktop/mobile browser workflow, native worker restart, live combined startup, Clippy/rustfmt/frontend build and governance lint plus 32 tests passed.
- Version: N/A: no release or strict versioning.
- Commit: uncommitted.
