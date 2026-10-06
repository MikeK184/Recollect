# Live pipeline visualization: animated capture → learning → memory flow

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Give each Brain a visibly alive pipeline view — capture → learning → memory/graph — that animates as agents save and process information, using the existing polling feeds (no new server endpoint in v1).
- Non-goals: No SSE/WebSocket broadcast (recorded future enhancement); no new data collection; no changes to what Activity measures; no other pages.
- Delivery shape: One new view on the Activity surface built from existing bounded feeds, animated with the motion foundation primitives; live browser verification.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md) (amended 2026-10-03: live pipeline view rules)
- [Desktop ADR](../../../adr/0014-desktop-experience-and-answers.md)
- [claude-mem study, 2026-10-03](../../../research/claude-mem-study-2026-10-03.md) (SSE live-feed + processing/queue indicator pattern as visual reference; their broadcaster is not copied)
- [Owning epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: A "Pipeline" view on the Brain's Activity surface (first tab, ahead of Timeline): three stage nodes — Capture, Learning, Memory/Graph — connected by animated edges. Node states derive from existing feeds: capture event count/recency, learning job state (idle/running/failed) and queue depth where already exposed, memory/graph totals. When a poll returns new data versus the previous render, the view animates the diff: new items slide into the stage, the active edge pulses along its path, counts tick up, and a processing indicator glows while a learning job runs. A compact "recent flow" list under the diagram shows the last few events (capture received / memory created / graph updated) with timestamps from real feed data only. All bounds, authorization and feed cadences are the existing Activity ones.
- Out of scope: SSE/WebSocket endpoint; per-event server push; pipeline controls (no pausing jobs); other pages; backend changes; fabricated counts or causal ordering beyond what feeds state.
- Blockers: None — depends on `motion-design-foundation` tokens/primitives landing first.

## Surface and Interface Changes

- Interfaces: None — presentation over existing Activity reads.
- Storage: None.
- Ownership: Activity owns the view; feeds keep their owning endpoints.

## Data and Authority

- Inputs: Existing bounded Activity feeds (capture events, learning/processing jobs, memory totals, tool calls where relevant).
- Authority: Unchanged; the view is a read-only projection of authorized data.
- Blind spots: Polling cadence (seconds) means "live" is visually continuous but data-wise stepped; the UI must not imply sub-poll precision (no fake streaming text, no invented intermediate states).

## States and Edge Cases

- Loading: Skeleton diagram with idle nodes while first feeds resolve.
- Empty: A Brain with no recent activity shows the diagram at rest with "No recent activity — connect an agent or import a source" and the real totals (zero).
- Error: A failed feed degrades that node to a static "unavailable" state with the existing error affordance; other nodes keep animating from healthy feeds.
- Blocked: No new blocked state.
- No-access: Standard Brain access rules apply before the view renders.
- Duplicate or replay: Polls are idempotent reads; diff animation keys on stable event/item ids so replays do not re-animate.
- Stale data: Same polling cadence as the rest of Activity; last-updated time shown from feed data, never invented.
- Reconciliation divergence: None — no writes, no new data path.

## Integrations and Runtime Inputs

- Providers: None.
- Environment: None.
- Secrets: None.
- Failure handling: Per-feed degradation as above; reduced-motion users get the static diagram with updated values.

## Tests and Acceptance

- Automated: Web typecheck and design checks.
- Manual: Owner login at the live stack — open SWEG Brain Activity → Pipeline: diagram renders from real data; trigger a real capture (agent session or source import) and watch the next poll animate the new item through Capture → Learning → Memory with edge pulse and count tick; a running learning job shows the processing glow; an empty Brain shows the at-rest state; a failed feed degrades one node only.
- Acceptance: Browser verification passes on real data including at least one observed live diff animation; typecheck and design checks clean; `./scripts/validate.sh` passes; live stack rebuilt and healthy.

## Closeout

- Planned: Pipeline view on Activity, diff animation over polling, processing indicators, browser proof with a real capture.
- Shipped:
  - New `Pipeline` tab, first on the Brain Activity surface (`web/src/features/activity/PipelineView.tsx`, `web/src/features/activity/pipeline.css`; wired in `web/src/features/activity/ActivityPage.tsx`). Three stage nodes — Capture, Learning, Memory/Graph — joined by dashed edges; presentation only: no new endpoint, no backend change, no new npm dependency, motion built from the existing foundation (tokens, `StatusDot`, skeleton, CSS vars).
  - Node data comes from the existing bounded feeds with shared query keys so no extra requests are added: Capture = `/api/brains/{brain}/capture/events` at 10s; Learning = `/learning` at 10s plus the shared `["processing",id]` / `["jobs",id]` keys at 1500ms (identical to JobsPanel); Memory/Graph = `/claims` at 4s plus the shared `["graph",id,0]` key at 3000ms (identical to GraphPanel).
  - Diff animation keyed on stable item ids held in a ref, so replays and re-polls of unchanged data do not re-animate. On new data: count tick, node flash overlay, edge pulse (keyed SVG path remount with dash travel), and a Learning glow + live dot while jobs run. A "Recent flow" list merges the last 6 real events (capture items, claims, finished jobs) with staggered entry; timestamps come from feed data only.
  - States: loading skeleton diagram; empty Brain at rest with "No recent activity — connect an agent or import a source"; per-feed error degrades only that node with the existing error affordance; reduced motion renders the static diagram via the global rule in `styles.css`.
  - Browser proof on the live stack (`127.0.0.1:8787`, SWEG Brain `5c054930-d266-4c18-a42b-942729f942aa`):
    - At rest from real data: 55 captures / Idle / 78 memories, "Graph ready · 97 nodes" (`pipeline-sweg-base.png`).
    - Five real source imports (Sources → Add source) drove the live diffs: memory count ticked 78→83→86→87 and graph 97→103→107→109 nodes; the Learning node showed Running with the purple glow and "2 jobs in queue" (`pipeline-live-4.png`, `pipeline-live-9.png`); new real entries staggered into Recent flow.
    - Edge pulse: a persistent MutationObserver logged `.edge-pulse[data-edge="memory"]` insertions at each import's learning completion, and the animation ran to its final keyframe (dashoffset −1.35px, playState finished) (`pipeline-live-pulse.png`, `pipeline-edge-crop-src.png`). A WAAPI slowdown of the already-completed pulse captured mid-travel frames for visual evidence only (`pipeline-edge-mid-1.png`, `pipeline-edge-mid-2.png`); the live trigger and completion were observed without intervention.
    - Empty Brain at rest: the dataless "test" brain shows 0 / Idle / 0 with the at-rest message and an empty-flow hint, clean edges (`pipeline-empty.png`).
    - Console clean on both verified pages.
- Not shipped:
  - A live Capture-stage diff could not be triggered in this environment: `publishCapture` requires a device-authenticated agent hook (`auth.device_id`) and no device token is configured; source imports do not create capture events. The Capture node correctly held its real count (55). This is an environment gap, not a code defect — the animation path is shared across stages and was proven via the learning/memory diffs.
- New blockers: None.
- Docs updated: This pack's Closeout. The governing contract amendment (`docs/contracts/desktop-experience.md`, live pipeline view rules) was recorded during planning; no further doc changes were needed for this slice.
- Validation:
  - `cd web && npm run typecheck` — pass.
  - `./scripts/validate.sh` — pass (includes the web design checks).
  - `./scripts/stack.sh up --build` — rebuilt and healthy at `http://127.0.0.1:8787`; UI reports all services connected.
  - Browser verification on real data as itemized above, including at least one observed live diff animation (memory count tick + edge pulse from real source imports).
- Version: N/A: no release policy or version bump in this scope.
- Commit: Uncommitted; no commit, push or release performed.
