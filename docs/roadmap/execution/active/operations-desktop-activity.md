# Authorized desktop activity and diagnostics

Status: in-progress
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: Explain background outcomes and actionable exceptions through one Activity destination and canonical detail links.
- Non-goals: New event store, causal total order, fabricated aggregate counts or widened audit/tool/private-task access.
- Delivery shape: Route-owned presentation of existing bounded authorized APIs, focused permission and desktop proof.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md)
- [Durable work](../../../contracts/platform-durable-work.md)
- [Model requests](../../../contracts/memory-provider-policy-and-learning.md)
- [Tool runtime](../../../contracts/mcp-runtime-and-credentials.md)
- [Retention](../../../contracts/memory-retention-and-erasure.md)
- [Workspace privacy](../../../contracts/evidence-workspace-scope.md)
- [Owning epic](../../epics/operational-readiness.md)

## Scope

- In scope: Timeline/Processing/Tool calls/Model usage tabs; actual feed state, supported filters, canonical route/inspector links, authorized retry/cancel/reconcile and owner diagnostics affordance.
- Out of scope: New unified cross-resource read API or server aggregation, joined unsupported totals and a mandatory human work queue.
- Blockers: None; use existing bounded APIs first. New endpoints would require a scoped contract/pack amendment before implementation.

## Surface and Interface Changes

- Interfaces: Existing jobs, model requests/learning, tool calls and admin audit endpoints; URL tabs are non-sensitive and validated. Presentation sorts available timestamped records without claiming completeness or causality.
- Storage: N/A: no new event persistence; canonical subsystem metadata retains its existing retention and pagination.
- Ownership: Operations owns presentation/diagnostic composition; each subsystem retains payload visibility, mutations and recovery semantics.

## Data and Authority

- Inputs: Current role, authorized bounded feed responses, actual timestamps/dispositions and safe links to their canonical owner.
- Authority: Existing endpoint permissions: knowledge read is not admin audit, profile Use or access to another account's private task/path.
- Blind spots: A loaded page is not a global event count. Independent timestamps do not prove causal ordering or full session coverage.

## States and Edge Cases

- Loading: Mount only active feed/tab; show separate loading/error for independently authorized sources.
- Empty: Explain no visible events versus excluded/unsupported feed; never imply global absence from a bounded result.
- Error: Display redacted subsystem failure and safe retry; failed protected reads clear old payload.
- Blocked: Failed jobs/uncertain calls link to the correct recovery path rather than generic retry.
- No-access: Hide forbidden feed requests and protect direct links; removal of permission clears visible detail.
- Duplicate or replay: Stable canonical IDs deduplicate display only; replay/retry follows owning command contract, never a new effectful universal action.
- Stale data: Brain/tab changes cancel prior reads; invalidate on mutation, observed epoch/access failure or applicable expiry.
- Reconciliation divergence: Preserve actual cancelled/uncertain/suppressed/partial outcomes, cost uncertainty and backup/erasure limits.

## Integrations and Runtime Inputs

- Providers: Existing same-origin subsystem endpoints; no new external integration or event transport.
- Environment: Existing installation configuration; N/A for new variables because aggregation is presentation only.
- Secrets: Feed content remains redacted; no private path, raw provider body, credential or unauthorized caller output in summaries.
- Failure handling: Independent feed retry cannot repeat the represented operation. Canonical recovery keeps its effect and accounting semantics.

## Tests and Acceptance

- Automated: Reader/admin/profile-Use combinations, foreign-account task/path canaries, empty/partial/error feeds, cancellation/late responses and canonical link routing; frontend and repository checks.
- Manual: Inspect actual failed processing, a permitted tool result and model usage; follow canonical detail/recovery and confirm owner diagnostics visibility.
- Acceptance: Useful exceptions are understandable and actionable, visible rows have a real source, and aggregation never grants access or invents completeness.

## Closeout

- Planned: Authorized Activity tabs, owner diagnostics and safe canonical links.
- Shipped: Not yet. Selected authorized audit/processing/tool/model/removal consumers and canonical target links are implemented in the final local image. MCP runtime recovery and current route/accessibility checks passed; source/claim exact links were independently exercised.
- Not shipped: Final Operations/feed role combinations, private-data canaries, current-error/late-response and exact recovery browser proof remain open. Generic route scans and MCP tests do not replace those Activity acceptance checks. Explicit product non-goals remain excluded.
- New blockers: No unresolved product decision is known. Implementation refinements and verification are tracked in the [dated mapping](../../../mappings/desktop-experience-implementation-2026-09-26.md).
- Docs updated: [Desktop guide](../../../runbooks/desktop-experience.md), [implementation/assets/dependency evidence](../../../mappings/desktop-experience-implementation-2026-09-26.md), [current handoff](../../../../CONTINUE_HERE.md), affected domain runbooks, owning epic and indexes.
- Validation: Frontend typecheck/design and final image build passed; workspace clippy and 21 unit tests passed with 3 live-Vault cases explicitly ignored; governance lint and 32 tests passed. Desktop seven cases passed across a six-pass run and the repaired font-fallback targeted rerun; real Team/OIDC two, Ask five and MCP setup/runtime six passed. The dated mapping separates each owner result, initial failures, fixture/provider boundaries and remaining checks.
- Version: N/A: no release requested.
- Commit: uncommitted.
