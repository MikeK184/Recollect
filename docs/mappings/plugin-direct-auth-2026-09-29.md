# Plugin/MCP direct auth without CLI, 2026-09-29

Observed: 2026-09-29
Confidence: observed-once

## Sources and Method

- Installed Codex 0.157.1 (`codex mcp add --help`, `codex mcp get`) with an
  isolated `CODEX_HOME` scratch directory; personal `~/.codex` untouched.
- Context7 queries 2026-09-29: `/openai/codex` (plugin manifest with
  `skills`/`mcpServers` keys, `bearer_token_env_var`, `http_headers_helper`),
  `/anthropics/claude-code` (plugin layout, `${VAR}` header expansion),
  `/websites/opencode_ai_v2` (`remote` type, `{env:VAR}` headers,
  `oauth: false`).
- Live dev stack at `http://127.0.0.1:8787` (`ready:true`): built
  `recollect-agent mcp-config codex-remote|claude-remote|opencode-remote`,
  unauthenticated pairing start/poll/cancel, MCP endpoint status codes.
- `cargo test --locked -p recollect-agent --lib` 17/17 (3 new),
  `cargo clippy --locked --all-targets -p recollect-agent` clean,
  web `typecheck` clean. No secrets in commands, files, or output below.

## Observations

- `codex mcp add recollect --url .../api/brains/{brain}/mcp/agent
  --bearer-token-env-var RECOLLECT_MCP_TOKEN` in the scratch home exits 0;
  `codex mcp get recollect` reports `transport: streamable_http`,
  the exact URL, and `bearer_token_env_var: RECOLLECT_MCP_TOKEN`.
- Rendered `codex-remote` TOML carries the Brain endpoint URL plus
  `bearer_token_env_var`; `claude-remote` JSON carries
  `Bearer ${RECOLLECT_MCP_TOKEN}` headers; `opencode-remote` JSON carries
  `type: remote`, `oauth: false`, and `Bearer {env:RECOLLECT_MCP_TOKEN}`
  headers. All three envelopes report `configured_only` with empty
  `host_arguments`.
- Unauthenticated `POST /api/devices/pairings` returns a verification URL,
  eight-character user code, expiry, and 2-second interval. Polling the
  private device code returns `pending` with no token. Cancelling returns
  204, leaving no credential behind.
- `POST /api/brains/{brain}/mcp/agent` without auth returns 401; with a
  garbage UUID bearer returns 401; with a wrong `Origin` returns 403.
- `grep` for bearer values, token assignments, and API-key shapes over
  `plugins/recollect/` returns nothing. Rendered configurations parse in the
  host's own tooling and contain only the variable name.
- Codex 0.157.1 binary strings confirm discoverable manifests
  (`.codex-plugin/plugin.json`, `.claude-plugin/plugin.json`), plugin MCP
  server loading, and `RawMcpServerConfig` fields (`url`,
  `bearer_token_env_var`, `http_headers_helper`).

## Translation and Limits

- The device bearer credential serves as the plugin/MCP user token on the
  existing endpoint; no server code changed in this slice. Promoted into
  [ADR 0015](../adr/0015-direct-plugin-user-auth.md) and the
  [plugin direct-auth contract](../contracts/mcp-plugin-direct-auth.md).
- Plugin-bundled MCP server entries were deliberately not shipped: the
  endpoint URL is deployment-specific, and the exact bundled-file schema was
  not verified end to end. The skill gives the verified `codex mcp add`
  command instead. The static marketplace root mirrors the live-proven
  generated marketplace layout but its `marketplace add` install was not
  re-run here to avoid touching personal host configuration.
- Full acceptance (plugin-only scoped recall plus evidence-backed write, and
  revocation denial) still needs a user token on a live host; the PG-gated
  server suite already proves the Bearer device-token tool path, cited by
  the contract.

## Follow-up

- Completed 2026-09-29 with the owner login on the local dev stack: plugin-initiated device-code start, browser approve/poll/finish, Bearer-token MCP initialize (22 tools), `workspace.list`, evidence-backed `memory.contribute` (`device_authored`/`proposed`), `memory.recall` with provenance, owner revocation then 401 denial. Proof script loads `.env` in-process and prints statuses only: `/private/var/folders/j6/6lx_5zxs5377bhb8fq09pvrc0000gn/T/opencode/plugin-live-proof.py`. Test Brain `Plugin live-token proof` and its claim remain on the dev stack; the proof device stays revoked.
- Retry the model-driven skill turn and live OpenCode handshake once ChatGPT-workspace credits allow (tracks under `mcp-codex-plugin`, not this slice); revoke the first test device when convenient.
- Specify successor slices (device dedupe, review-UI removal, Ask/Search simplification).
