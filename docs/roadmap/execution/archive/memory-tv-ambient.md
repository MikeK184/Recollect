# Memory TV ambient mode: fullscreen cycling memory display

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Add an ambient fullscreen "memory TV" mode for a Brain — recent memories cycle one at a time with slow fade/scale transitions, category accent glow, timestamp and provenance — for a second monitor or demo wall. Toggled from the Activity surface; hidden from navigation.
- Non-goals: No multi-screen sync, audio, persistence of TV settings, no new data feeds beyond existing bounded reads, no other presentation pages.
- Delivery shape: One hidden route + component built on the motion foundation primitives; live browser verification.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md) (amended 2026-10-03: ambient mode rules)
- [claude-mem study, 2026-10-03](../../../research/claude-mem-study-2026-10-03.md) (tv.html ambient cycling pattern as visual reference)
- [Owning epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: A route `/brains/{brain}/tv` (not in any navigation list; reachable via an explicit "Ambient display" button on the Activity surface and by direct URL). Fullscreen light cream layout cycling the Brain's recent memories/claims (existing bounded memory list read, newest first): one item per screen with title, kind badge in its category accent, short body, timestamp and source chip; slow fade+scale transition (~1.2s ambient duration token) every ~8s; Esc or browser Back exits to Activity. The cycle uses only data from the authorized read — no mock items, no invented recency.
- Out of scope: Settings for interval/order; multiple simultaneous displays syncing; audio; embedding in other pages; new endpoint or storage changes; the existing claim-page read may advertise its canonical retention deadline.
- Blockers: None — depends on `motion-design-foundation` tokens/primitives landing first.

## Acceptance — 2026-10-04

The palette correction below is historical. TV now clears protected display
content on failed refresh and canonical expiry, preserves exact revision identity,
and shows all review/freshness/operational and conflict qualifications. Focus is
contained in the ambient surface and returns on exit. The existing claim-page
read advertises a canonical content deadline; no new endpoint or store was added.
See the [final dated evidence](../../../mappings/desktop-visual-closeout-2026-10-04.md).

## Surface and Interface Changes

- Interfaces: Hidden presentation route over the existing bounded claim read; optional ClaimPage.expires_at is the earliest canonical retention deadline across the displayed revisions and supporting evidence whose retained labels were emitted. No new endpoint.
- Storage: None; reuse the existing retention-deadline SQL function.
- Ownership: The Brain's memory read owns the data; the TV view is a projection.

## Data and Authority

- Inputs: Existing bounded authorized memory list for the Brain.
- Authority: Unchanged; the route applies the same Brain access rules as Memory.
- Blind spots: A very large list cycles only the recent window already returned by the read; that is honest "recent memories", not exhaustive playback.

## States and Edge Cases

- Loading: Light screen with a subtle pulsing wordmark while the first read resolves.
- Empty: "Nothing remembered yet" centered on the light canvas; no cycling.
- Error: Clear current/outgoing content and feed timestamp on failure or deadline, then show quiet Retry. Preserve exact revision selection across refresh insertions, and show review/freshness/operational and conflict/rule qualifications.
- Blocked: No new blocked state.
- No-access: Forbidden access renders the standard no-access view, not the TV.
- Duplicate or replay: Re-entry re-reads; cycling keys on item ids.
- Stale data: The read refreshes on the existing list cadence while open; last-updated shown small in a corner from feed data only.
- Reconciliation divergence: None — no writes.

## Integrations and Runtime Inputs

- Providers: None.
- Environment: None.
- Secrets: None.
- Failure handling: Reduced-motion users get crossfade-only transitions (no scale); tab-hidden pauses the timer via visibilitychange to avoid battery drain.

## Tests and Acceptance

- Automated: Focused canonical claim-page deadline proof and browser failure/expiry/identity/reduced-motion/pause/exit checks; web typecheck and design checks.
- Manual: Actual owner browser on the rebuilt SWEG stack displays retained memories with proposed/conflict qualifications; cycling and explicit exit are verified separately from the disposable failure/expiry/empty/access-loss cases. Direct route, reduced motion and keyboard focus/exit passed in the owned browser harness.
- Acceptance: Browser verification passes on real data; typecheck and design checks clean; `./scripts/validate.sh` passes; live stack rebuilt and healthy.

## Closeout

- Planned: Hidden ambient route, recent-memory cycling, Activity entry and safe content lifecycle.
- Shipped: Light fullscreen presentation, exact revision selection, current/outgoing/footer clearing on failure or canonical deadline, qualifications, hidden-tab pause, reduced motion, focus containment and explicit/keyboard exit. Rebuilt locally; [dated evidence](../../../mappings/desktop-visual-closeout-2026-10-04.md).
- Not shipped: Interval settings, multi-screen sync, audio, exhaustive playback and the separate research/benchmark backlog are outside scope.
- New blockers: None.
- Docs updated: Desktop contract, this archived pack, epic and indexes, dated evidence and handoff.
- Validation: Two focused ambient browser cases, canonical retention deadline test, independent UI review, build/typecheck/design, clippy, final governance/whitespace and synchronized CodeGraph; local readiness and served assets verified.
- Version: N/A; no release policy or version bump.
- Commit: Uncommitted; no commit, push or external release.
