# Browser-approved native companions

Status: shipped
Owning epic: `docs/roadmap/epics/product-platform.md`
Work type: product

## Summary

- Goal: Enroll a native companion and revoke each device independently through real product paths.
- Non-goals: Workspace discovery, capture and MCP/runner execution.
- Delivery shape: Protocol, Rust server/companion, migration, browser controls and local native-store proof.

## Governing Sources

- [Device contract](../../../contracts/platform-device-pairing.md)
- [Runtime ADR](../../../adr/0003-product-runtime.md)
- [Team contract](../../../contracts/platform-team-access.md)
- [Durable-work contract](../../../contracts/platform-durable-work.md)
- [Owning epic](../../epics/product-platform.md)

## Scope

- In scope: Bounded browser-approved pairing, retryable delivery, OS credential persistence, bearer identity, scoped device controls and worker revocation.
- Out of scope: Customer rollout, platform identity-provider setup and downstream companion roles.
- Blockers: None; predecessor delivered and detailed lifecycle is contracted.

## Surface and Interface Changes

- Interfaces: Pairing start/view/approve/poll/finish/cancel, device list/revoke, companion pair/whoami/brains/unpair/forget and shared request/response types.
- Storage: Device and ephemeral pairing rows; optional device ID on durable jobs; OS credential entry keyed by endpoint/profile.
- Ownership: Device module owns enrollment; common Auth/transaction boundary enforces caller identity; worker preserves submitting device; companion owns OS-store adapter.

## Data and Authority

- Inputs: Device name, private random code, matching browser approval, current enabled principal and device state.
- Authority: PostgreSQL device/claim/revocation state and current grants; OS store holds only the companion credential.
- Blind spots: No workspace, execution-profile or shell authority is inferred from pairing.

## States and Edge Cases

- Loading: Waiting approval with bounded polling, browser mutation pending and bounded HTTP calls.
- Empty: No devices or no stored profile are explicit.
- Error: Invalid input 400, invalid credential 401, protected browser-only operation 403, foreign ID 404, replay/transition 409, expired request 410 and capacity/poll throttling 429.
- Blocked: Unavailable native credential store or service gives a clear error; preserve existing credentials and independently eligible work.
- No-access: Per-call principal/device/grant checks and per-publication device revalidation.
- Duplicate or replay: Approval is single-use; credential delivery repeats until acknowledgement; acknowledgements/revocation are idempotent.
- Stale data: Expired device or OIDC identity fails new authentication; revoked devices cannot publish previously queued jobs.
- Reconciliation divergence: Store before acknowledgement; lost acknowledgement can be checked with authenticated identity. Unclaimed/expired credentials cannot authenticate.

## Integrations and Runtime Inputs

- Providers: Current native keyring library and existing Rust HTTP/database stack; local macOS native-store proof using a unique test entry.
- Environment: RECOLLECT_URL and RECOLLECT_DEVICE_PROFILE; existing server environment.
- Secrets: Native OS store and in-memory protocol delivery only; no credential in CLI output, source, audit or list metadata.
- Failure handling: Five-minute pairing, two-second polls, five-second HTTP timeout, bounded queues and explicit store/revocation recovery.

## Tests and Acceptance

- Automated: Real-handler negative/positive device lifecycle and queued-revocation scenarios; native companion and browser integration; generated protocol build and governance validation.
- Manual: Inspect device/approval screens and prove OS-store write/read/delete across native processes.
- Acceptance: Device contract behavior and native integration verified before archiving.

## Closeout

- Planned: Companion pairing, native credential persistence and independent device revocation.
- Shipped: Browser-approved companion enrollment, native OS credentials, bearer identity, independent device revocation and worker provenance; browser and real OIDC paths verified.
- Not shipped: Workspace/capture/MCP roles belong to their named successors; no shared or other-OS runtime proof is claimed.
- New blockers: None.
- Docs updated: Contract, pack, epic/indexes, device proof mapping, device/local-development runbooks and root README.
- Validation: Six core Rust scenarios, one real OIDC scenario, four browser workflows including native OS-store persistence/revocation, generated frontend build, Clippy, rustfmt, governance lint and all 32 checker tests passed. Current local readiness, owner login and device-list call succeeded. See [device proof](../../../mappings/platform-device-pairing-proof-2026-09-14.md).
- Version: N/A: no release or strict versioning.
- Commit: uncommitted.
