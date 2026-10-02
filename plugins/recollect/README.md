# Recollect automatic memory plugin

The packaged plugin owns connection setup, native session capture and automatic
memory recall. Start Codex, Claude Code or OpenCode normally after connecting.
The [completed integration](../../docs/roadmap/execution/archive/mcp-plugin-session-memory.md)
was verified through actual installed hosts. See the [evidence and platform limits](../../docs/mappings/plugin-session-memory-2026-10-01.md).

## Build an installable package

Maintainers run `python3 scripts/package-plugin.py` from the repository root.
It prints a fresh package directory containing a native executable, manifests,
hooks and skills. Build on each supported target platform. The receiving machine
needs the coding host and plugin package, not Cargo or a separate companion.
The package also includes the platform's Enola extractor and its license/notice
for repository publication. `repository` uses this bundled executable; it does
not search the original build checkout.

Install the printed package with the host's plugin installer:

- Codex: `codex plugin marketplace add PACKAGE_DIRECTORY`, then
  `codex plugin add recollect-memory@recollect`.
- Claude Code: `claude plugin marketplace add PACKAGE_DIRECTORY`, then
  `claude plugin install recollect-memory@recollect`.
- OpenCode V2: add the package's
  `plugins/recollect-memory/opencode/index.mjs` to the project's `plugins` array.

Use the `recollect-connect` skill to choose the Recollect URL and Brain and
complete the browser device authorization. Credentials go to the OS store.
Trust the plugin's hooks through the host's normal review. Hooks cannot be
trusted silently. Then use the coding host normally; no `capture run` wrapper.

Independent local/private tool execution is optional. Add `--with-runner` during
connection setup; use `--runner-id UUID` for an existing private registration.
Without the flag, memory use starts no execution runner. Existing connector grants,
approved targets and Vault credentials remain required.

The runtime is bundled under `plugins/recollect-memory/bin/recollect-plugin`.
Its `status` command checks the connection and reports delivery state.
`workspace discover DIRECTORY` inspects local checkout metadata without connecting
or uploading. `workspace refresh DIRECTORY` publishes that metadata with current
Brain access; `scope` and `repository` retain the existing explicit workflows.
`RECOLLECT_PLUGIN_DATA` optionally selects a private managed data directory.
Normal installs use the user's application data directory automatically.
Connection records the installed executable's absolute path in a private
`runtime-path` file there. The Codex/Claude MCP bootstrap reads that pointer with
a quoted `exec`; Codex 0.154.0 does not expand plugin path variables in legacy
MCP command fields. Moving the package requires reconnecting from its new path.
OpenCode 2.0.21 uses native MCP tools (`codemode: false`): its Code Mode catalogue
can omit plugin-added tools on first startup even when their server is connected.
This preserves the same scoped tools and host permissions.

Before replacing a legacy setup, drain its pending captures using the original
setup file and credential. Remove that host's previous first-party `recollect`
MCP entry, enable the plugin, then verify a successful read and capture delivery.
Retain old setup files until their pending count is zero. Never run the old
`capture run` wrapper and the complete plugin for the same session. Installation
does not overwrite host configuration or migrate queues to a different Brain.
`status` reports detected Codex/Claude legacy entries by path without printing
their configuration values. Plugin hooks with a conflicting setup withhold
capture; OpenCode rejects an existing incompatible first-party MCP entry through
its native configuration transform. Other servers/plugins are left alone.

Changing the plugin's selected account or server preserves original destination
and OS-credential references for pending plugin captures. Background delivery
retries each original destination; it never relabels those events into the new
Brain. Invalid connection attempts leave the previous configuration and credential
intact. Credentials remain in the OS store; destination records contain references
only. Explicit revocation, policy expiry and Brain deletion still fence delivery.

## Advanced direct HTTP transport

The following transport remains available for clients that only need explicit MCP
calls. It does not install automatic capture or prompt-context hooks. Avoid a
second `recollect` server entry when the complete plugin is installed.

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

In Recollect, open Agents, choose Connect coding agent, expand Advanced · Direct
MCP connection and choose your host, then create an access token. The token is
shown once. It inherits your current Brain grants
and is not restricted to one Brain. Revoke it under Devices when it is no
longer needed.

Without a browser at setup time, start a device-code request with plain
HTTPS, approve the shown code in the browser later, then poll and finish:

```sh
curl -s -X POST "$RECOLLECT_URL/api/devices/pairings" \
  -H 'content-type: application/json' \
  -d '{"name":"Codex plugin · personal laptop"}'
```

## 2. Connect MCP transport

Direct HTTP needs no plugin installation. The current packaged plugin already
provides an MCP entry; do not add the direct entry alongside it. Source manifests
without a built runtime are not an installable memory plugin.

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

## 3. Verify

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
- Automatic capture and prompt recall belong to the complete packaged plugin above.
