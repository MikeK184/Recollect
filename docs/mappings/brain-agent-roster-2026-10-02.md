# Per-Brain agent roster and external-MCP-through-Brain proof, 2026-10-02

Source: live local stack at `http://127.0.0.1:8787` (Docker api/worker, image
rebuilt from this working tree via `./scripts/stack.sh up --build`), migration
031 applied, owner session plus the SWEG host device credential. Observation
date 2026-10-02. Verification: real browser (owner login) and real JSON-RPC
calls from the SWEG host device token; confidence `verified`.

## UI restructure

- Global navigation renders exactly Brains and Team; Devices is absent. The
  `/devices` route still renders by direct URL as the hidden pairing-approval
  and full-account-list surface (34 devices: 12 connected, 22 history).
- SWEG Brain Agents page shows the roster above the tabs: group header
  "owner — 12 agents"; the live plugin row reads Recollect plugin · OpenCode ·
  Recollect plugin · active dot · last used on this Brain. Connect-an-agent
  tab holds three one-line cards; the product-model banner is gone.
- The roster Revoke modal states the keycard is removed from all Brains
  (account-wide); opened and cancelled during proof, nothing revoked.
- Rendered text scan of Agents, Connections and Settings found zero
  `companion` / `recollect-agent` strings; console clean on all pages.
- Screenshots: `proof-nav.png`, `proof-agents-roster.png`,
  `proof-revoke-modal.png`, `proof-devices-direct.png`, `proof-activity.png`
  in the session temp directory.

## End-to-end external MCP through the Brain (central placement)

- Owner setup on SWEG Brain `5c054930-d266-4c18-a42b-942729f942aa`:
  connection `63bc0290-4b97-4bb3-9e5d-796133e3a2df` (Context7,
  streamable_http, target `https://mcp.context7.com/mcp`, placement central),
  Use grant on profile `e698b099-fa1c-46cc-adb5-c2d14fce10ff`, and the
  connection attached to that profile (an enabled profile must contain its
  connections; the first call attempts failed with
  `mcp_profile_unavailable` until it was).
- Host side, SWEG device Bearer token from the OS keychain against
  `/api/brains/{brain}/mcp/agent`: `initialize` → `tools/list` (22 tools) →
  `workspace.start_task` (task `57335ba1-adf3-4243-bfb1-15b0a70b01e6`) →
  `workspace.begin` kind tool (operation `3a301602-b519-49bf-be71-162ac3d42463`)
  → `mcp.call` `resolve-library-id` (call `f7375104-0dc1-499f-a20b-6f38592ceb69`)
  → `mcp.status` terminal `succeeded`, reason `connector_response`, runner
  central, ~1.8 s.
- Retained output (in `mcp_call_payloads.result`) is the real Context7
  response naming `/reactjs/react.dev`; `mcp_calls` for the Brain went 0 → 1;
  Activity → Tool calls shows the row with the sanitized result and the
  published terminal observation.

## Blind spots and limits

- Central placement executed inside the api container, which has no `node`, so
  the stdio `everything` server was not used there; the external target is the
  already-approved Context7 HTTP definition proving the same host → Brain MCP
  → external-server path.
- The local-placement variant (stdio on the PC via a plugin runner) was not
  executed: the packaged dist ships no `recollect-mcp-runner` helper and the
  live SWEG bridge runs without `--with-runner`; re-pairing with a runner was
  deliberately skipped to avoid disrupting the live host. It remains
  configuration-only on this host.
- Roster "last used on this Brain" is point-in-time evidence from `mcp_calls`
  and capture events, not live presence; `host_kind` stays null for legacy
  devices without capture bindings (no name parsing).
