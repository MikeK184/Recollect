# Agent surface, Ask chat and desktop chrome refinements, 2026-10-03

Source: live local stack at `http://127.0.0.1:8787` (Docker api/worker, image
rebuilt from this working tree via `./scripts/stack.sh up --build`), owner
session. Observation date 2026-10-03. Verification: real browser (owner login)
plus focused platform tests on disposable databases; confidence `verified`.

## Global agent roster (`GET /api/agents`)

- Owner session sees all 12 active account devices grouped as "owner — 12
  agents"; revoked/expired devices are excluded and reported only as the hidden
  count (22). Members see only their own devices (asserted in test).
- Per-agent `brains` usage lists each accessible Brain with last use: Recollect
  plugin (OpenCode) → "SWEG — test — last used 02/10/2026, 14:02:42"; Codex
  live plugin test → 29/09/2026; Codex MCP · SWEG — test → 02/10/2026,
  20:14:12. Devices without usage read "Not used on any Brain yet". No
  credential material appears in the payload (asserted in test).
- Query fix found during live verification: the first `usage` CTE cross-joined
  per-brain mcp-call and capture aggregates, so a Brain with calls from one
  device and captures from another attributed every row to the calling device.
  The shipped query unions both sources and groups by (device, brain), taking
  the max timestamp. Verified against the live database: SWEG now reports all
  three of its used devices.

## Per-Brain roster (used-only)

- The SWEG Brain (`5c054930-d266-4c18-a42b-942729f942aa`) Agents page shows
  exactly the three agents with real last-use on that Brain, each with host ·
  integration, last-used time and Revoke. The "Show revoked and expired" toggle
  reveals hidden rows of the same used set; an "All account agents" link points
  to the global page. Empty Brains get a dedicated empty state.

## Desktop chrome

- Healthy SWEG Brain renders **no** assurance band on any page (the strip now
  renders only for a blocker, pending read, failed feed, archived Brain,
  disabled autonomous memory, or an empty first session).
- The four Knowledge pages no longer render the horizontal
  Memory/Sources/Graph/Repositories switcher (`.knowledge-switcher` absent);
  the contextual sidebar is the sole navigation and intra-page sub-tabs remain
  (Memory: All memory / Claims / Decisions / Procedures / Handovers). The
  inspector's "Show in…" cross-view links are untouched.

## Ask as a chat conversation

- The landing reads like a chat: the prompt block is centered in the
  conversation area with the composer beneath it, starting as a single line and
  growing with input (previously the composer sat above the welcome block).
- Live question on the SWEG Brain ("What do we know about the SWEG project?")
  produced a completed model-grounded answer turn with citation markers in the
  thread; follow-up composer, per-turn failure states and "New conversation"
  are unchanged. No console errors on any verified page.

## Checks

- Focused platform tests on disposable databases: `account_agent_roster`
  (grouping, usage ordering, member scoping, hidden count, no credential
  material) and the existing `pairing_markers_and_brain_agent_roster` both pass.
- `cargo clippy -p recollect-server -p recollect-protocol` clean; web typecheck
  and design checks clean; `./scripts/validate.sh` passes.
- The `/devices` direct-URL surface still renders (pairing deep link preserved).
- Screenshots in the session temp directory: `proof2-agents-global.png`,
  `proof2-brain-roster.png`, `proof2-memory-notabs.png`, `proof2-ask-chat.png`,
  `proof2-ask-thread.png`.
