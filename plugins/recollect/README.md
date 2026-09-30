# Recollect memory plugin (no companion required)

Connect Codex, Claude Code, or OpenCode to a hosted Recollect server through
the plugin/MCP integration alone. No CLI download, no companion pairing. The
plugin carries memory skills and setup guidance; it grants no authority. The
local companion remains only for optional automatic session capture.

## 0. Three separate credentials (read this first)

- **Recollect server key (`OPENAI_API_KEY`):** operator-only. It pays for the
  server's own embeddings and autonomous memory. A plugin user never needs it,
  never sends it, and never puts it in any host config.
- **Recollect user token (`RECOLLECT_MCP_TOKEN`):** you. This is the only
  credential the plugin/MCP integration uses, as a bearer header on
  `/api/brains/{brain}/mcp/agent`.
- **Host model billing (not Recollect):** Codex bills your ChatGPT workspace,
  Claude bills your Claude plan/API, OpenCode bills its own configured
  provider. An "out of credits" error from the host is host billing and never
  a Recollect defect — the MCP tools are unaffected.

## 1. Create an access token

In Recollect, open Connections, choose your coding host, and use Create
access token. The token is shown once. It inherits your current Brain grants
and is not restricted to one Brain. Revoke it under Devices when it is no
longer needed.

Without a browser at setup time, start a device-code request with plain
HTTPS, approve the shown code in the browser later, then poll and finish:

```sh
curl -s -X POST "$RECOLLECT_URL/api/devices/pairings" \
  -H 'content-type: application/json' \
  -d '{"name":"Codex plugin · personal laptop"}'
```

## 2. Install the skills plugin

From this repository root:

```sh
codex plugin marketplace add ./plugins/recollect
codex plugin add recollect-memory@recollect
```

This installs the `recollect-memory` skill. It bundles no MCP server entry
because the endpoint URL is deployment-specific; the next step connects it.

## 3. Connect MCP transport

Export the token in the terminal that starts your agent (an already running
agent does not receive it):

```sh
printf 'Recollect access token: '
read -r -s RECOLLECT_MCP_TOKEN
printf '\n'
export RECOLLECT_MCP_TOKEN
```

Then add one server entry per Brain:

- Codex:

```sh
codex mcp add recollect \
  --url "$RECOLLECT_URL/api/brains/$BRAIN_ID/mcp/agent" \
  --bearer-token-env-var RECOLLECT_MCP_TOKEN
```

On macOS, Codex can read the token from Keychain instead; the Recollect
coding-agent dialog shows that alternative.

- Claude Code: copy `plugins/recollect-memory/mcp-template.json` to your
  project's `.mcp.json`, replacing `$RECOLLECT_URL` and `$BRAIN_ID`, keeping
  `${RECOLLECT_MCP_TOKEN}` literally.

- OpenCode: merge `opencode-mcp.example.json` into your `opencode.json`,
  replacing `$RECOLLECT_URL` and `$BRAIN_ID`, keeping
  `{env:RECOLLECT_MCP_TOKEN}` literally.

Copy-paste configuration without a companion binary is also available from
`recollect-agent mcp-config codex-remote|claude-remote|opencode-remote
--brain BRAIN_UUID`, and from the Recollect coding-agent dialog, which
additionally covers OpenCode.

## 4. Verify

Ask the agent to run Recollect's `workspace.list`. A successful tool call
confirms the connection; saving settings does not. Then work in a task scope
(`workspace.start_task`), recall before answering (`memory.recall`), and
contribute deliberately (`memory.contribute`) as the bundled skill describes.

## Rules

- Secrets never enter plugin files, generated settings, URLs, commands, or
  shell history. Only the variable name `RECOLLECT_MCP_TOKEN` is written down.
- This is not OAuth: Recollect authenticates the header bearer token only.
- One server entry selects one Brain. The token itself works for every Brain
  your account may access.
- Optional automatic session capture still needs the companion and its host
  hooks; see the session-capture runbook.
