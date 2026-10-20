# Recollect automatic memory plugin

The packaged plugin owns connection setup, native session capture and automatic
memory recall. Start Codex, Claude Code or OpenCode normally after connecting.
The [completed integration](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/mcp-plugin-session-memory.md)
was verified through actual installed hosts. Current source-workflow and rebuilt-binary acceptance limits are recorded in the
[plugin workflow report](../../docs/mappings/plugin-workflow-repair-2026-10-07.md). See the [evidence and platform limits](https://github.com/MikeK184/Recollect/blob/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs/mappings/plugin-session-memory-2026-10-01.md).

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

## Shared Brain skills and instructions

A rebuilt plugin advertises applicable manager-published guidance with its normal
scoped context. The bundled `recollect-memory` skill uses `skills.list` and
`skills.fetch` to load instructions and referenced text only when needed. Manage
bundles under the Brain's **Agents → Skills** tab; optional MCP profile links grant
no execution permission.

The rebuilt plugin also synchronizes owned native Codex, Claude Code and OpenCode
skills at supported task/context/scope boundaries. Temporary task references are
the default; explicit `--persist` retains native and canonical copies. A shared
workspace lease prevents conflicting Brain/environment installs across host kinds,
and one session cannot clean another's active reference. Instructions are native
skill wrappers retaining the original AGENTS resource; local AGENTS/CLAUDE files
and unowned skills remain intact.

The executable provides `skills install|sync|reload|uninstall|status|leases` beside
`list|fetch|save|discard`. Native uninstall and canonical-cache discard are separate.
Save alone remains a private cache operation. Reload reports placement/refresh
state and any necessary host action; it never calls a copied file "loaded".
See the [operator procedure](../../docs/runbooks/managed-skills.md) for exact
commands, host reload differences, copy lifetimes and safe recovery, and the
[native fixture proof](../../docs/mappings/native-skill-reload-host-proof-2026-10-18.md)
for actual installed-host evidence. Fixtures use isolated homes and synthetic
providers; this source change does not upgrade your installed package or deployed
server, and deleting files cannot retract an existing conversation's context.

## Peer coordination

A rebuilt plugin publishes a bounded goal and current task scope to the Brain's
coordination controller. Supported `Edit`, `Write`, `MultiEdit`, and `apply_patch`
hooks enqueue planned repository-relative paths. Shell text is not interpreted as
a reliable file list. Canonical Git origins must already be registered in the
Brain; unknown paths remain unannounced. No source file contents are uploaded for
coordination, and a peer agreement grants no tool or file-write permission.
The current checkout's registered canonical origin supplies topic context before
the first edit, without changing the task's selected scope. Unknown origins are
reported as incomplete context, never matched by a folder basename.

An owned background worker receives authenticated SSE and falls back to the
durable inbox. Prompt, pre-tool, resume and context hooks show attributed advisory
notices with account, coding host, device and environment. Delivery is at the next
supported host boundary; no idle turn or mid-generation interruption is promised.
OpenCode consumes peer context at its context callback, keeping pre-tool notices
queued until that callback can inject them.
Without a native hook for 90 seconds the worker stops heartbeats and presence
expires. Long silent tools may therefore appear inactive. Known access denial
clears cached peer events, and offline injection requires recent authentication.

The bundled runtime exposes:

```sh
recollect-plugin coordination peers
recollect-plugin coordination inbox --session SESSION_UUID --after 0
recollect-plugin coordination watch --session SESSION_UUID
recollect-plugin coordination status --session SESSION_UUID
recollect-plugin coordination send < peer-message.json
```

`watch` shows readable peer notices on stderr; protocol/command JSON stays on
stdout. `send` reads the contract's message JSON (`id`, `from_session_id`,
`to_session_id`, `kind`, `text`, `reply_to`) from stdin. Keep the same message UUID
when retrying uncertain delivery. Proposal acceptance requires a referenced
proposal. `announce` similarly reads the typed session declaration from stdin;
normal native sessions announce automatically. `heartbeat`, `ack` and `close`
are also available. Use the exact session UUID supplied by the hook, never a
directory's most recent unrelated task. See the
[coordination contract](../../docs/contracts/mcp-agent-coordination.md) for limits
and the distinction between receipt, hook output and model understanding.

This source addition does not upgrade an already installed package. Native hook
fixtures, actual installed-host delivery and live deployment have separate proof.

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

### Document evidence and agent visibility

Use the bundled memory skill and the advertised `source.import`, `source.list`
and `source.inspect` tools to retain authorized document evidence, find exact
source/version IDs and inspect citation spans. Imports use a write operation's
exact scope and a stable command ID. Automatic capture remains independent of
manual document import. Never guess internal Recollect paths or private API/auth
files to use memory. Update both server and installed plugin when these tools
are absent.

Successful memory/workspace/source requests now appear in the Brain's agent
roster. Actual reported coding hosts take precedence over stale pairing hints;
a reused credential can show more than one observed host. Manage your credentials
in My agents → Access tokens. Processing limits are visible in Settings → AI
permissions in the main form below Automatic processing.
