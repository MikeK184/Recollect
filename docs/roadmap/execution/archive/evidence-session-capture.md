# Automatic host session capture

Status: shipped
Owning epic: `docs/roadmap/epics/evidence-and-workspaces.md`
Work type: product

## Summary

- Goal: Automatically capture permitted Codex/Claude events into durable scoped evidence and autonomous learning.
- Non-goals: Transcript scraping, hidden reasoning, recall injection before retrieval and coordinator-owned MCP execution.
- Delivery shape: Native hooks/inbox/worker, server admission/retention, browser status and real local proof; uncommitted.

## Governing Sources

[Capture contract](../../../contracts/evidence-session-capture.md),
[workspace scope](../../../contracts/evidence-workspace-scope.md),
[retention](../../../contracts/memory-retention-and-erasure.md),
[autonomous maintenance](../../../contracts/memory-autonomous-maintenance.md) and
[owning epic](../../epics/evidence-and-workspaces.md).

## Scope

- In scope: Host adapters, automatic configuration/launch, local sanitized SQLite inbox, fixed operation scope, current Brain capture policy, idempotent delivery, central source processing, scoped learning, erasure/replay and UI.
- Out of scope: Changing customer files, unsolicited installation into the developer's personal host configuration, Cognee runtime dependency and later retrieval/MCP coordination. The user's explicit managed launch registers its generated Codex plugin as specified by the contract.
- Blockers: None; actual Codex and Claude compatibility was verified in an isolated owned container.

## Surface and Interface Changes

- Interfaces: Capture policy/bindings/events endpoints and native bind/setup/hook/status/drain/run commands.
- Storage: Migration 014 for policy, bindings, event receipts/provenance and erasure fences; private local SQLite inbox.
- Ownership: Existing device/workspace commands, retained sources, autonomous model policy and deletion journal remain authoritative.

## Data and Authority

- Inputs: Bounded supported host JSON; retained content is sanitized before any local persistence.
- Authority: Current paired writer and immutable operation scope; capture and model transmission permissions are independent.
- Blind spots: Unsupported host tools/events, missing IDs, interrupted hooks and unknown secret encodings are explicit coverage limits.

## States and Edge Cases

- Loading: Binding setup, queued local evidence and asynchronous publication/processing.
- Empty: Disabled capture, no events or no paired companion.
- Error: Malformed/oversized event, identity conflict, inbox full or safe network/storage failure.
- Blocked: Stale cached policy, provider permission, revoked device or unavailable scope.
- No-access: Foreign Brain/binding/device denied; readers cannot change policy or publish.
- Duplicate or replay: Stable event UUID and native-key uniqueness; lost responses reuse the same receipt.
- Stale data: Turn scope remains fixed; policy/deletions refresh before upload.
- Reconciliation divergence: Local queued, centrally accepted and learned states are shown separately.

## Integrations and Runtime Inputs

- Providers: Codex/Claude documented command hooks; existing Luna gateway only after central admission.
- Environment: Repository-owned storage/services and isolated host fixtures; existing paired credentials.
- Secrets: OS store/environment only; no raw hook payload, transcript or credential output.
- Failure handling: Durable local transaction, bounded worker retry, explicit gap counters, expiry and deletion replay.

## Tests and Acceptance

- Automated: Native sanitizer/inbox/concurrency/restart, real API/database scope/policy/replay/erasure and browser state checks with positive controls.
- Manual: Actual bounded host invocation and normal-runtime source/model/preservation evidence; inspect desktop/mobile output.
- Acceptance: Every capture-contract requirement verified before archival; incomplete compatibility cannot count as shipped.

## Closeout

- Planned: Complete native-to-central automatic session capture and its lifecycle/UI.
- Shipped: Native sanitizer and durable inbox; generated setup/plugin and managed host launch; immutable turn/tool scope; asynchronous idempotent delivery and device reports; central source processing and scoped autonomous learning; local/central expiry, erasure and older-backup replay; browser policy, delivery/coverage states and canonical source inspection/erasure. Migration 014 and the current UI are running locally at port 8787.
- Not shipped: No incomplete slice acceptance. Historical transcript import, hidden reasoning, universal host coverage and automatic procedure execution remain explicit non-goals. Recall injection and coordinator-owned MCP observations belong to their named successors. No external deployment or personal Codex plugin installation was performed by development work.
- New blockers: None.
- Docs updated: Capture and retention contracts, session-capture runbook, README, owning epic, active/archive/epic/contract indexes and interface evidence. Retention replay normalizes known omitted optional empty arrays. Erasure clears cached content while retaining the Brain shell and progress dialog. Stale techstack review wording now matches accepted ADR 0006.
- Validation: Three real API/database capture scenarios passed; all 36 platform scenarios passed in 41.09s; seven native/protocol library scenarios and full-workspace Clippy passed. Actual Codex 0.154.0 and Claude Code 2.1.270 invoked the compiled native hook in an isolated container, including an unbound-session control and child exit-status proof (11.65s). Native hook process: 491 ms. Capture and retention browser tests: two passed in 21.9s, with inspected desktop/mobile output. Generated 113 API operations and built the frontend. Normal startup applied 014, returned ready and served the new UI. A newly paired temporary native profile set up, deduplicated and delivered one sanitized synthetic source with zero queued bodies or model calls, then removed its credential. The four prior Brains and SWEG evidence/retention remain preserved. Governance validation and full evidence are recorded in the [mapping](../../../mappings/session-capture-interfaces-2026-09-14.md).
- Version: N/A; no release requested.
- Commit: Uncommitted.
