# Flat desktop control panel and truthful live pipeline

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Implement the user-approved light Flow deck with real current/last-observed source processing and agent provenance.
- Non-goals: Decorative 3D, fake activity, token streaming, private-task surveillance, TV completion, external deployment or release.
- Delivery shape: Rust read endpoint, React global pages and shared Activity pipeline, local runtime validation and documentation.

## Governing Sources

- [Desktop contract and live amendment](../../../contracts/desktop-experience.md)
- [Desktop ADR](../../../adr/0014-desktop-experience-and-answers.md)
- [Vision](../../../foundation/vision.md), [stack](../../../foundation/techstack.md), [principles](../../../foundation/engineering-principles.md)
- [Owning epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Brains selection/list/creation, global Agents, Team and invitation/account states, diagnostics, consistent sign-in/support surfaces, exact source pipeline on Brains and Activity.
- Out of scope: Changes to learning decisions, capture/model permissions, retention policy, knowledge graph renderer or existing mutations; TV and its closeout.
- Blockers: None. User approved the proposal and required real attributed processing on 2026-10-03.

## Surface and Interface Changes

- Interfaces: Read-only `/api/brains/{brain}/pipeline`, generated client. At most 30 current/recent inputs; active work priority; exact capture/source/job/run joins; observation time, validity deadline and truncation. Graph projection is independent.
- Storage: No second event store or producer schema; existing canonical records remain authority.
- Ownership: Server pipeline module composes existing domain records under Brain RLS; shared frontend pipeline consumes only selected Brain. Global pages retain existing handlers.

## Data and Authority

- Inputs: Published capture/source provenance, exact processing jobs, learning runs and recorded outcome counts, Brain graph generation.
- Authority: Authenticated Brain read permission, current privacy/retention and existing private-context boundaries. Device credential validity never implies live presence.
- Blind spots: Polling can miss transient intermediate stages. Unreported host/subagent identity stays unknown. Source-free lifecycle captures do not imply learning; model success can yield no memories. Graph lacks a per-source causal identifier.

## States and Edge Cases

- Loading: Static loading state; no invented figures or flow.
- Empty: Explicit no activity; real invitation/agent empty states.
- Error: Clear protected payload, stop animation and offer retry.
- Blocked: Display actual failed/cancelled job and error code; link to existing canonical controls.
- No-access: Standard denied response and no cached content.
- Duplicate or replay: Initial history sets baseline; stable IDs/state transitions pulse once, including concurrent inputs; no auto demonstration replay.
- Stale data: Two-second visible polling, six-second maximum display validity, earliest source expiry; hidden tab paused, refocus revalidates. Selection changes cancel requests and reset baseline.
- Reconciliation divergence: Show recorded run dispositions; do not equate historical accepted counts with current accepted inventory. Do not animate a source-to-graph assertion.

## Integrations and Runtime Inputs

- Providers: None invoked by dashboard reads. Existing model worker remains unchanged.
- Environment: Existing local Compose and test database inputs; names and credentials unchanged.
- Secrets: No new credentials, no source bodies in URLs/logs, no private contexts. Exact source detail remains canonical.
- Failure handling: Bounded reads, cancellation, no background polling on hidden pages, explicit unavailable states.

## Tests and Acceptance

- Automated: API exact joins, concurrent origins, zero-result run, unrelated jobs, old active work, access isolation and expired/erased source exclusion; UI duplicate/history/multi-entry updates, selection races, stale/error/reduced-motion; frontend build, Rust formatting/clippy, governance validation.
- Manual: Desktop 1440/1920 screenshots and useful interactions for each global surface; real disposable capture/source processing through production handlers proves identity and observed live state changes; local stack rebuild with inventory preservation.
- Acceptance: All displayed activity comes from authorized records. Current and historical modes are clear; no fake online/processing states. Approved light layout and existing actions remain usable. Independent agent review findings resolved.

## Closeout

- Planned: Approved flat light global pages and exact attributed current/last-observed processing.
- Shipped: Bounded authorized pipeline API and generated client; shared Brains/Activity diagram; source/job/outcome inspector; global Agents and per-agent inspector; compact Brain selection and creation drawer; Team tabs/drawers; core-store diagnostics and cumulative latency bars; truthful credential labels. Existing mutation and sign-in/support paths retained.
- Not shipped: TV acceptance/closeout, SSE/token streaming, decorative 3D or the deferred research backlog.
- New blockers: None for this slice. TV remains a separate active pack.
- Docs updated: Contract amendment, owning epic/index, archive/active indexes, dated [evidence](../../../mappings/desktop-live-control-panel-2026-10-03.md), mapping index and handoff.
- Validation: Exact-origin/worker/retry/retention/access API proof passed; final control-panel browser suite 4/4; separate list/diagnostics regression run 7/7; Team invitation/access/disable proof passed (organization-provider test explicitly skipped without its provider). Resize assertions cover all nodes and all filters at 1024/1440/1920. Reduced motion, stale/failed polling and inspector identity passed; axe found no tested-page violations. Server/protocol clippy with warnings denied, focused Rust formatting, frontend build/design/typecheck, dependency audit (zero vulnerabilities), CodeGraph sync, whitespace and 32 governance tests passed. Independent review has no remaining material finding.
- Runtime: Local Compose image rebuilt and replaced; API/worker/stores healthy. Deployed owner browser verified cream theme even with dark preference, real bounded pipeline (30 inputs, four origin contexts), populated Agents/drawer, Team, diagnostics and creation; no browser errors. Inventory unchanged: 16 Brains, 1 account, 52 source versions, 117 claim revisions, 34 devices.
- Version: N/A; no release policy or version bump.
- Commit: Uncommitted. No commit, push or release.
