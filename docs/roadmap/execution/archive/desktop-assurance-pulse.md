# Exception-first assurance and Activity drill-down

Status: shipped
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: Make the ordinary autonomous state legible as "nothing needs you", surface only authorized exceptions in the standing band, and reorder Activity so diagnosis reaches the failing or uncertain item before the full audit feed.
- Non-goals: No new aggregation endpoint, unified feed, joined counts, background refresh, role or authorization rule. No change to job state, erasure progress, tool-call reconciliation, retention or audit semantics.
- Delivery shape: React presentation change composing existing bounded authorized feeds, plus explicit omission of any figure no verified response supplies; focused browser and authorization proof; reconciled epic and execution indexes.

## Governing Sources

- [ADR 0017: Assurance band](../../../adr/0017-desktop-knowledge-and-ask-experience.md)
- [Tiered desktop surface contract](../../../contracts/desktop-knowledge-surface.md)
- [Contextual desktop experience](../../../contracts/desktop-experience.md)
- [Autonomous Brain maintenance](../../../contracts/memory-autonomous-maintenance.md)
- [Retention and controlled erasure](../../../contracts/memory-retention-and-erasure.md)
- [Managed MCP calls and credentials](../../../contracts/mcp-runtime-and-credentials.md)
- [Durable commands and workers](../../../contracts/platform-durable-work.md)
- [Brain model policy and evidence-backed learning](../../../contracts/memory-provider-policy-and-learning.md)
- [Vision: correction and learning loop](../../../foundation/vision.md#correction-and-learning-loop)
- [Owning epic](../../epics/operational-readiness.md)

## Scope

- In scope: Current-blocker-only prominent warnings per the user’s 2026-10-01 decision, with historical failures and pending work retained in Activity. The band's ordinary and escalated states composed only from existing authorized reads; learned, revised, retired and captured figures where a verified response field supplies them; current blocker selection for canonical failed processing, unresolved uncertain tool outcomes, erasure errors and missing provider credentials; historical failed jobs/calls, pending erasure and capture reports remain diagnostic entries; Activity reordered to exceptions first with the full authorized audit feed reachable; canonical detail links from every band and list entry into its owning inspector; explicit configured-versus-connected distinction for models, connections and devices.
- Out of scope: Any new server read, aggregate, cache or unified pagination. Account-private scope and path exposure through aggregation. Turning the band into a mandatory review inbox. Changing what autonomous maintenance may do, or restoring any per-record approval step.
- Blockers: None. [ADR 0017](../../../adr/0017-desktop-knowledge-and-ask-experience.md) and the tiered surface contract are accepted; the band's data boundary is constrained to existing authorized feeds so no new read contract is required.

## Surface and Interface Changes

- Interfaces: No API change. Activity keeps its existing tabs and adds an exception-first default ordering within the already-authorized feeds it composes.
- Storage: N/A: no schema or migration. No new persisted telemetry; retrieval telemetry stays separate from mutation audit as already defined.
- Ownership: Operations owns the assembled presentation and diagnostics. Memory lifecycle owns learning and maintenance semantics, MCP coordination owns call and runtime state, Evidence owns capture coverage, and Platform owns erasure and retention execution. Each figure maps to its owning read; this slice implements no derived metric of its own.

## Data and Authority

- Inputs: Existing per-Brain automation status, learning and processing feeds, model usage, tool-call and observation records, erasure request status, capture device and binding reports, connection and profile state, and the caller's effective role.
- Authority: Every figure is a passthrough of an authorized response. Aggregation never widens visibility, and admin-only audit, profile outputs and private task scope retain independent authorization.
- Blind spots: Offline companion coverage, backup copy lifetime and uncontrolled external copies cannot be asserted complete. Independent events may overlap in time, so no total causal order is claimed. Any figure without a verified response field is omitted rather than estimated client-side, and that omission is recorded in the closeout.

## States and Edge Cases

- Loading: Band shows a pending position rather than zeros; Activity tabs keep independent loading.
- Empty: A Brain with no activity yet says so and points at the connect flow, distinct from an exception state.
- Error: A failed authorized read degrades only its own figure and reports the failure rather than implying success.
- Blocked: Pending erasure, journal export failure or inaccessible storage shows as pending with its retry path, never as complete.
- No-access: A reader sees permitted knowledge status only; admin-only audit and caller outputs stay absent, and losing access mid-session clears cached figures.
- Duplicate or replay: Refetching must not double-count; figures come from server responses, and repeated acknowledgement of an uncertain call stays idempotent under the existing reconciliation rules.
- Historical failure: A terminal failure alone does not trigger a global warning; retain it in Activity even when current processing is healthy. Pending cleanup is progress, and a reported capture gap is historical coverage evidence.
- Stale data: Advertised expiry, epoch change and Brain switching clear the band and its drill-down payloads; late responses cannot populate the new Brain.
- Reconciliation divergence: A configured-but-uncalled model, connection or device is shown as configured. A queued job is not shown as running. A partial result is not shown as success.

## Integrations and Runtime Inputs

- Providers: OpenAI and embedding providers appear only as installed-policy status. The band issues no provider call; any explicit connectivity check remains a separately labeled administrator action with its stated cost and effect.
- Environment: No new variable. Existing provider, database, artifact and journal configuration names apply.
- Secrets: No key, token, credential value or private local path is rendered in the band, Activity or any fixture.
- Failure handling: Feed reads use existing bounded polling per active route. No band refresh may start a job, rebuild, analytics run, provider call or tool execution.

## Tests and Acceptance

- Automated: Browser cases proving the ordinary state renders no-action-required, that each injected current blocker escalates and links to its canonical owner inspector, that reader roles see no admin-only figure or caller output, that configured-only states never render connected, and that Brain switching and expiry clear the band. Request capture proving no navigation or refresh issues a provider, rebuild or tool request. Existing processing, erasure, tool-call and capture regressions pass. Web typecheck, `./scripts/validate.sh` and `git diff --check`.
- Manual: Operator walkthrough on a disposable Brain with one deliberately failed processing run and one uncertain tool outcome, confirming each is reachable in one step from the band and that recovery controls appear only for authorized roles.
- Acceptance: The band equals authorized feed counts; the ordinary autonomous state is explicit rather than inferred from an empty list; every exception reaches its canonical owner inspector with correct role-appropriate controls; Activity is diagnosable without scrolling a uniform audit wall; no aggregation widens visibility or leaks account-private data.

## Closeout

- Planned: Assurance band, exception selection, Activity exception-first ordering, canonical detail links and configured-versus-connected separation.
- Shipped: Locally delivered on 2026-10-01. Standing assurance band and an exception-first Activity overview composed from existing authorized feeds. Ordinary state is explicit; current blockers link to canonical inspectors; past failures and pending work remain separate in Activity without a global warning. Learning figures are the latest completed-run window; captured-event count is explicitly Brain-wide. Private MCP payloads are removed before band caching. See the [continuation evidence](../../../mappings/desktop-continuation-2026-10-01.md).
- Not shipped: Brain-lifetime learned/revised/retired/suppressed totals are omitted because existing feeds do not supply them. No new aggregation endpoint, permission, provider call or mandatory review queue was added.
- New blockers: None for this slice. Unrelated desktop acceptance packs and the held Ask slice remain open.
- Docs updated: Governing desktop priority amendment, dated evidence, desktop/agent/capture runbooks as applicable, this pack, owning epic and epic/execution indexes, and CONTINUE_HERE.md.
- Validation: Five focused browser cases passed normal/window counts, current-blocker versus historical-event separation, direct diagnostics, authorization/private-payload limits and Brain-switch clearing. Request capture showed no mutations from navigation or band refresh. Processing, retention, capture and MCP-runtime browser regressions passed; the second agent approved Activity and assurance visuals, with quiet states also inspected from disposable-fixture screenshots. Web design/typecheck/build, workspace Rust tests, platform integration with documented focused reruns, Clippy, formatting, all 32 governance checker tests and git diff --check passed. The evidence mapping distinguishes opt-in skips, real external results and current deployment.
- Version: N/A: no release policy or version bump in this scope.
- Commit: Uncommitted; no commit, push or release performed.
