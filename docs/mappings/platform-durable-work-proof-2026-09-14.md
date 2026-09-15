# Durable work local proof

Observed: 2026-09-14
Confidence: verified

## Sources and Method

The [contract](../contracts/platform-durable-work.md) is the acceptance source.
Executed `./scripts/test-platform.sh`, `./scripts/test-ui.sh`, Rust Clippy with
warnings denied, rustfmt check, generated OpenAPI/TypeScript client build and
`./scripts/validate.sh`. Tests use uniquely named disposable PostgreSQL databases
and the non-owner application role. The existing repository-owned pgvector and
Neo4j/GDS services answered actual dependency queries.

## Observations

| Behavior | Evidence |
| --- | --- |
| Atomic durable commands | Concurrent equal HTTP commands produce one Brain/audit/job/receipt; mismatched key input returns 409. An injected queue constraint rolls back canonical state and audit. Capacity returns 429 without partial changes. |
| Receipt lifetime and access | Actors have separate key namespaces; replays recheck access. Expired receipts can be replaced even after access to the former Brain is revoked. |
| Bounded durable work | Three concurrent claims admit two interactive leases. Expired leases reclaim with a new token; old execute, renew and failure calls cannot alter the replacement. Three failed attempts terminate. |
| Safe publication | Projection, completion and audit commit together from current canonical input. Duplicate execution cannot publish twice. A revoked actor cannot publish; an authorized admin retry can. |
| Operator controls and scope | Real cancel/retry handlers enforce admin and Brain/job matching. Reader and cross-Brain negative cases have positive controls. RLS hides unrelated receipts, jobs and projection rows. |
| Cleanup | Expired operational records are removed; canonical Brain and mutation audit remain. |
| Browser and native process | The desktop/mobile workflow displays queued work, cancels/retries it and observes current state after native `worker-once`. A fresh continuous native worker drains the persisted queue. Screenshots were inspected. |
| Combined startup | `./scripts/dev.sh` starts separate API and worker processes; `/health/ready` returns true. Runtime children do not inherit the migration/admin credentials. |

Four Rust integration scenarios and one browser workflow passed. Clippy, rustfmt,
frontend build, governance lint and all 32 governance checker tests passed.

## Translation and Limits

This is local delivery. Interrupted work is tested through persisted expired
leases and stale tokens; no OS-level crash during an external side effect is
claimed. Native process restart is separately exercised with queued work.
The only registered consumer is the real Brain-directory refresh. Other lanes
are bounded scheduling primitives, not implemented extraction/model/graph work.
Unknown kinds fail visibly. External deployment and release remain outside scope.

## Follow-up

Team access and device pairing are the next platform slices. Domain consumers
must supply their own eligibility, erasure and external-effect recovery proof.
