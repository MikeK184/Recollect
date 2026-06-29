# Durable commands and workers

Status: accepted

## Source

[Runtime ADR](../adr/0003-product-runtime.md),
[bootstrap contract](platform-bootstrap.md),
[platform epic](../roadmap/epics/product-platform.md) and
[mutation principles](../foundation/engineering-principles.md#route-every-mutation-through-one-policy).
Routine implementation choices are authorized by the user's full-product goal.

## Contract

### Command boundary and outbox

Brain create/update commands atomically persist canonical state, mutation audit,
a durable `brain.refresh` job and an optional idempotency receipt. Queue insertion
failure rolls back the entire command. The jobs table is the transactional outbox;
there is no separate unreliable publish hop. Domain slices add code-owned handlers
through this boundary, with their own evidence, erasure and publication policy.

`Idempotency-Key` is an optional 1–128 character printable non-space ASCII key. Its scope is
the authenticated actor, operation and destination; a key cannot be reused for a
different request within that actor's 24-hour receipt lifetime. Store/compare
normalized JSON directly, without hashes. Concurrent equal requests with one key
return the same canonical response and create one mutation/audit/job. Different
inputs with that key return 409. Replays recheck current resource access; they
cannot bypass revocation. A missing key is a new operation. Client form retries
reuse a key; after a successful mutation the next deliberate change gets a new key.
No client revision header or strict version handshake is required.

Brain reads always use canonical rows. A directory projection stores current Brain
metadata under the same Brain RLS and exposes processing readiness through
`GET /api/brains/{id}/processing`. A random change identifier only fences derived
publication; it is not an API version requirement. Reads label a projection
`current` only when its input identifier equals the canonical one. Otherwise they
show `queued`, `running`, `failed` or `missing` from actual durable work. Current
canonical changes remain visible while background work is delayed.

### Worker lifecycle

The native `recollect-server worker` role uses a bounded PostgreSQL pool under a
non-owner application role; `worker-once` attempts one job and exits. The regular
worker has two interactive slots, two capture slots, one model slot and one heavy
slot. PostgreSQL serializes admission per lane and enforces these limits across
worker processes. No unbounded task spawning or in-memory-only queue exists.
This slice registers only the interactive `brain.refresh` consumer; later slices
own capture/model/heavy handlers. Unsupported stored job kinds fail visibly.

Claim with `FOR UPDATE SKIP LOCKED`, persist a unique lease token and a 20-second
deadline, and increment the attempt count. A heartbeat can renew the matching
lease. Publication must match the current token and a live deadline; an old worker
cannot publish or finish reclaimed/cancelled work. Dispatch and publication check
the persisted actor's enabled state and current Brain write permission. Revoked
or missing inputs cancel the job without publishing content. The consumer reads
the current canonical Brain under a row lock, so an older job cannot overwrite
newer metadata. Completion and projection/audit commit together.

Jobs transition `queued → running → succeeded`, or to `queued` for retry,
`failed` after three attempts, or `cancelled` after revocation/operator action.
Transient failures retry after 1 and 2 seconds; expired leases are reclaimed.
Expired third attempts become failed. Keep safe error codes, progress 0–100,
timestamps, attempt count and lease state; never store driver/credential payloads.
Lease expiry after a process crash is recovery, not proof the original execution
never happened. Consumers must remain idempotent. External side effects need a
domain-specific reconciliation contract and are not blindly retried by this slice.

Enqueue admits at most 500 queued/running jobs per Brain, locking that Brain while
checking capacity. At capacity, commands return 429 without partial mutation.
Receipts expire after 24 hours; unreferenced terminal job operational records expire
after seven days. Publication, learning and handover history retain their referenced
job identities so cleanup cannot break durable foreign keys or stop worker admission.
Canonical mutation audit and projections are not deleted by this queue cleanup.

### API, UI and operation

`GET /api/brains/{id}/jobs` returns the latest 100 safe job records to Brain readers.
`POST /api/brains/{id}/jobs/{job}/retry` lets a Brain admin retry failed/cancelled
work, setting that approving actor as the new submitter; completed/active work
returns 409. `POST .../cancel` cancels queued/running jobs and clears their lease;
already-terminal calls are harmless. Both changes are auditable transactions.
All job/processing paths enforce Brain/resource matching; inaccessible IDs return
404. The UI shows processing state, attempt/progress/error information and admin
retry/cancel controls with pending/error states and polling while open.

`./scripts/dev.sh` starts API and worker, preserves databases/data and terminates
its own two child processes on Ctrl-C. A separate worker may be restarted without
the API; queued work survives. Shutdown stops claiming and drains in-flight tasks
within their bounded execution time. Hard termination relies on lease recovery.
The worker pool never uses the migration principal or claims arbitrary user-supplied
executables, SQL or connection targets.

## Acceptance

Use real handlers and PostgreSQL to prove concurrent idempotent replay/mismatch,
canonical/audit/outbox rollback on insertion failure, bounded admission, restart
with queued work, expired-lease reclaim and stale-token fencing, visible exhausted
failure and authorized retry/cancel, revocation denial plus a permitted control,
projection freshness after a later canonical change and cross-Brain job isolation.
Exercise the live native worker and browser processing state. Run focused tests,
Rust/frontend build checks and `./scripts/validate.sh` before archiving the pack.

## Explicit Deferrals

Domain-specific extraction, models, graphs, retention and external execution remain
with their named slices. The lane/handler seam does not claim those consumers are
implemented. Shared operation/recovery targets remain with operational readiness.
