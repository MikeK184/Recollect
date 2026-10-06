# Brain Dashboard and Knowledge inspection

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Implement the user-approved light-theme Brain pages proposal with real activity and usable growing libraries.
- Non-goals: TV closeout, fake telemetry, 3D, hashes, new frameworks, or model calls on navigation.
- Delivery shape: Local Rust/web changes and existing local installation, with UI reviewer.

## Governing Sources

- [Desktop](../../../contracts/desktop-experience.md), [Knowledge](../../../contracts/desktop-knowledge-surface.md), [ADR 0017](../../../adr/0017-desktop-knowledge-and-ask-experience.md).
- [Answers](../../../contracts/retrieval-answers.md), [graph](../../../contracts/graph-exploration.md).
- User approved the [reviewed proposal](../../../research/brain-pages-ui-review-2026-10-03.md).

## Scope

- In scope: Dashboard default and entry links; concise Ask/Search; readable Memory values; attributed Sources and one inspector; canvas-first Graph with optional Entities; searchable paginated Repositories, secondary checkouts, snapshot facts and secondary artifacts.
- Out of scope: Changing learning, retrieval, grants, retention, capture or mutation semantics.
- Blockers: None; user approved the design and explicitly requested reviewer delegation.

## Surface and Interface Changes

- Interfaces: Qualified `GraphView.expires_at`, scoped snapshot contributor names; `/dashboard`; GET `/api/brains/{brain}/workspace/repositories?q&offset` (50 records, total and next offset), exact GET of one repository and source summary. Existing workspace API remains compatible.
- Storage: None; reuse canonical records and Brain/RLS authority.
- Ownership: Existing domain handlers and shared inspector. No per-row queries.

## Data and Authority

- Inputs: Authorized pipeline, canonical claims/source versions, graph reads and repository snapshots.
- Authority: Canonical records and existing policies. Run counts are historical dispositions; claim-ID links explicitly open current memory.
- Blind spots: Bounded feeds are not whole-Brain totals; graph remains partial where reported. Host attribution is never guessed.

## States and Edge Cases

- Loading: Existing loading/skeleton states; no invented progress.
- Empty: Distinct empty library, filtered-empty, empty kind and no graph states.
- Error: Clear affected content immediately; retry preserves exact selection.
- Blocked: Keep coverage, missing bytes, unconfigured answering and oversize guidance.
- No-access: Existing Brain grants, archived/read-only controls and RLS; no foreign credential or checkout payload.
- Duplicate or replay: Canonical IDs, unchanged mutation receipts and conditional writes.
- Stale data: Keep protected-detail polling, retention deadlines and Brain-switch cancellation. Stable lists retain IDs only, never stale payloads.
- Reconciliation divergence: Projection state independent of exact processing; no inferred causality.

## Integrations and Runtime Inputs

- Providers: Existing stack and dependencies only. Context7 checked TanStack Query selectors/cancellation/cache behavior on 2026-10-03; no new package.
- Environment: Existing ignored local `.env`; no secret values in evidence.
- Secrets: Existing authorization and redaction; no private content in URLs.
- Failure handling: Existing cancellation, retry and freshness rules.

## Tests and Acceptance

- Automated: Existing relevant checks, web build/typecheck, Rust checks for changed handlers, validate.sh and whitespace. No new redundant test suites or hashes.
- Manual: Real local API/page proof, exact deep links, paging/search, failure clearing and desktop UI reviewer at 1440/1920.
- Acceptance: All approved destinations usable, actual values and attribution visible, source actions consolidated, no competing graph List view, repository requests bounded, Dashboard truthful and default; retained secondary actions reachable.

## Closeout

- Planned: Approved Brain page redesign and bounded browsing.
- Shipped: Light-theme Dashboard default and truthful pipeline; compact Ask; readable Memory with persistent search; attributed Sources with one bounded-content inspector; canvas Graph with independent Entities paging; bounded searchable Repositories and structured snapshots. Deployed to the existing local stack at `http://127.0.0.1:8787`.
- Not shipped: TV and overall epic closeout remain separate.
- New blockers: None.
- Docs updated: Existing contracts/ADR, research proposal, epic/indexes and handoff; this pack is the delivery record, with no extra mapping or hashes.
- Validation: Web build/design/typecheck, scoped clippy, existing workspace/source PostgreSQL tests, existing five Ask browser cases, CodeGraph and validate.sh pass. Live repository response is 50 of 79; a disposable database verified 122 repositories, search, aliases, paging, exact identity and grant denial. Real source identity/history remains canonical. Six pages at 1440/1920 pass overflow and settled accessibility checks (zero findings), with zero page errors; 1280×800 disabled Ask composer is visible. Repository reopen/origin controls, source scroll bounds and simulated Graph-status failure clearing pass. Independent read-only UI review has no remaining blocker. Ten browser fixtures were deleted; all original 16 Brains remain. Logs/screens are in ignored `.cache/brain-ui-implementation/`; no paid provider calls were made by the transport fixtures.
- Version: N/A; no release policy.
- Commit: Uncommitted.
