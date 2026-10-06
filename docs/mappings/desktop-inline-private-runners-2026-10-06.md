# Compact inline Private Runners delivery

Observed: 2026-10-06
Confidence: verified

## Sources and Method

- User approved the generated read and `inline-v2.png` concepts in
  `output/imagegen/2026-10-06-private-runners/` and requested implementation
  plus a comparison with ordinary MCP/plugin agents.
- [Desktop authority](../contracts/desktop-experience.md#private-runner-cards-and-inline-editing--2026-10-06),
  [private-runner contract](../contracts/mcp-vault-and-private-runners.md) and
  [plugin contract](../contracts/mcp-plugin-session-memory.md) govern behavior.
- Source: `web/src/McpPrivateRunners.tsx`, scoped `private-runners.css`, existing
  generated DTOs and unchanged `crates/server/src/mcp/private.rs` commands.
- CUA on an owned in-app browser tab: actual SWEG empty/Add/Cancel, followed by
  exact tab-scoped API fixtures for save/failure/permission states. No real
  registration writes or tool startup escaped the fixture intercepts.
- Context7 successfully resolved Mantine v8 and checked CopyButton/Switch usage
  against its primary docs. No new dependency or external executable assets.

## Observations

The Private Runners tab retains `tab=runners`. The old registration modal and
How it works disclosure are replaced by compact inline cards. Add uses a small
name field and current account active paired-device picker; existing bindings
are locked badges. Read/Edit share device and canonical status metadata and an
explicit Copy setup command. Enabled is a draft control; it is not live status.
Readers have no add/edit/setup actions. Existing polling hook remains available
to connection-placement consumers. Server authority and storage are unchanged.

Actual Add had no modal, excluded unavailable devices, enabled submission with
the chosen active device and nonempty name, and returned to empty on Cancel.
Controlled fixtures proved read Connected/Offline states, locked device editing,
draft Cancel, failed save retaining input, pending field/action freeze, successful
PUT retry with the same idempotency key, POST with null base_revision, retained
disabled history and the exact setup-copy payload. No runner was started.
Changed revision disabled Save without replacing the typed draft. Expired and
already-registered devices were absent from the creation picker; whitespace name
was blocked. Device failures blocked creation and recovered with explicit Refresh.
Failed registration reads hid stale cards/editor and blocked Add until refresh.

Reader/archive fixtures replaced both canonical Brain list and single-Brain reads;
a sidebar-only fixture is insufficient to test the Brain context. The corrected
fresh reader loaded three metadata cards with no Add/Edit and zero device reads.
Archived admin Add/Edit/setup controls were disabled. These are frontend guard
proofs, not new backend authentication tests; the backend was not changed.

Actual rendered comparison with the approved concept passed at 1384×1473 and
2260×1314: 1072px/1200px card width, 135px read card height, no horizontal overflow,
20px read name, 320×32px editable name and 26px device badge aligned beneath it.
The final small polish removes duplicate Disabled text beside its status badge.

Artifacts: `output/private-runners-inline-2026-10-06/proof.json`, actual
`live-empty.png`/`live-add.png`, controlled `fixture-view-laptop.png`,
`fixture-edit-laptop.png`, `fixture-view-wide.png`, `fixture-edit-wide.png`,
`fixture-stale.png`, `fixture-reader.png` and `preview-test-data.png`.
Connected sample cards are fictional test data, not observed live workers.

## Translation and Limits

Web type/design/production build, governance lint and 32 required governance
tests, whitespace and CodeGraph sync passed. Existing private-runner UI scenario
selectors now target the inline forms; its full Playwright/backend fixture was
not launched. Meaningful rendered interaction proof used CUA instead.

Normal local stack startup installed frontend-only image
`sha256:34d390b2f0bc11d8563b2ea5e28df9141568bde666a9d24a02bd4f2653fbc89f`;
`/health/ready` returned `{"ready":true}`. Existing backend unchanged. Logs are
`.cache/private-runners-inline-{build,image,deploy,validate,codegraph}.log`.
Interceptions/viewport were removed, clipboard restored and the owned tab closed.
Final actual SWEG view was still empty; user's original tab/session was preserved.
No private-runner registration, credentials or real worker connectivity added.
Version N/A; uncommitted. No commit, push or external release.

## Follow-up

None for this UI slice. Starting a real registered private runner is a separate
explicit operation. Agents use Brain knowledge and request tools; a private
runner executes permitted outbound tools on its paired network host. Ordinary
plugin/MCP access does not automatically start that opt-in execution worker.
