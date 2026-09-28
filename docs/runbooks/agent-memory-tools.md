# Connect coding hosts to scoped memory

## Purpose and Prerequisites

For ordinary MCP access, open **Connections → Coding agents** and select Codex
or Claude Code. Copy the direct HTTP configuration into the project's
`.codex/config.toml` or `.mcp.json`, preserving other servers. Create a revocable
access token and save it using the dialog's credential steps. On macOS, Codex
defaults to Keychain: copy the **Keychain secret**, run the displayed native
`security add-generic-password ... -w` command, and paste at the hidden password
prompt (again if asked to confirm). Keep `-w` last so the secret is not a command
argument or shell-history entry. The stored value is authorization-header JSON;
Codex's `http_headers_helper` reads it using the native `security` command.
Start Codex normally after saving the project configuration. This needs no
Recollect binary, shell-profile modification or environment export.

For Claude Code or environment-based Codex setup, keep `RECOLLECT_MCP_TOKEN`
literally in the copied configuration. It is the **variable name**, not a place
to paste the token. Follow the dialog's bash/zsh hidden-input, export and launch
block in the same terminal. An already running agent does not receive newly
exported variables; repeat this setup for a fresh terminal or use the host's
supported secret-store integration.
The token is displayed once; Devices lists and revokes it. It uses your account's
current Brain grants. No companion is required for recall, deliberate contribution
or published workspace metadata. A successful tool call, not saving settings,
confirms access.

Native OAuth browser login is not implemented by Recollect yet. Codex and
Claude can support OAuth, but the server must expose its discovery, authorization
and token endpoints. A Recollect pairing code is the existing device-approval
flow, not a substitute for those endpoints. A packaged plugin is not published.

The following optional path adds local workspace discovery and host capture:

Use the native companion to expose one Brain's memory, workspaces, graphs and
approved managed tools to Codex or Claude Code. Install `recollect-agent`,
`recollect-mcp-bridge` and `recollect-mcp-runner` together. Pair a named device
profile through the [existing pairing flow](device-pairing.md). The selected
service must run a build containing this capability; an older running native
process does not acquire it when its executable is rebuilt.

Vault is optional per managed connection. The bridge uses the paired credential
in the OS store; Linux requires an unlocked Secret Service session. macOS may
request Keychain access for the bridge executable. Approve that OS prompt only
for the intended paired profile. Secrets do not belong in host settings.

## Procedure

Set the companion's endpoint and profile to the values used during pairing. Render
one host's settings, supplying the intended Brain UUID and absolute workspace:

```sh
recollect-agent mcp-config codex --brain BRAIN_UUID --directory /absolute/workspace
recollect-agent mcp-config claude --brain BRAIN_UUID --directory /absolute/workspace
```

Use the returned `host_arguments` for that host launch, or put the returned
configuration in its project MCP settings. Rendering with an explicit Brain UUID
needs neither a live service nor credential access. Its `configured_only` result
therefore does not establish connectivity. Linux Codex settings allow forwarding
the current session's DBus/XDG variable names, without copying their values.

For a managed capture session, follow [automatic capture](session-capture.md).
`recollect-agent capture run` supplies both hooks and MCP settings for that launch.
The coding-agent dialog's automatic-capture disclosure presents the paired setup path.
An explicitly selected CA file follows the [installation trust procedure](installation.md).

Start a task through `workspace.start_task` with explicit selection and a
`context_query`. Its result contains the immutable scope, refreshed context and
retrieval operation ID. Use that ID for memory and graph reads. Start separate
`write` or `tool` operations for contributions/handovers or managed calls. A scope
change affects future operations; existing operations and child scopes remain
bound to their original selections. Models cannot supply another Brain or
override the selection carried by an operation.

## Verification

A connected host discovers `workspace.list` and `memory.recall` and completes a
tool call. A successful scope change returns `context.state=ready` with the same
scope ID as its handoff. Discovery alone starts no managed connector or model.
Handover generation requires standing model policy; results remain attributed
model synthesis, without fabricated human review. Managed tools still require
independent Use permission; follow [outcome reconciliation](mcp-runtime.md).

Actual Codex 0.154.0 and Claude Code 2.1.270, the Linux native bridge, PostgreSQL,
Neo4j and isolated Secret Service pass the owned fixture. Scope changes reach the
next model turn; recalled memory is excluded from independently captured evidence.
See the [dated evidence](../mappings/mcp-tools-2026-09-22.md) for commands and limits.

Installed macOS Codex 0.157.1 also passed a direct HTTP `workspace.list` call
against the actual SWEG Brain using its project config and Keychain, without
token environment injection or the Recollect companion. A loopback synthetic
model drove only that read; no paid inference or customer content transmission
was required. See the [credential repair evidence](../mappings/mcp-auth-repair-2026-09-28.md).

## Failure and Recovery

Pairing, current Brain access and operation kind are checked on every request.
Check host MCP status and OS-store access if tools are absent. Configuration
cannot substitute for a successful handshake.

If `bearer_token_env_var` contains the token itself, Codex looks for an
environment variable with that name and cannot authenticate. Remove the secret
from config and use the displayed Keychain helper or the literal variable name
with a token actually exported into the starting terminal. A successful raw HTTP
request does not prove that the configured coding host can obtain its credential.

If `context.state=unavailable`, the scope change still committed. Inspect the task
and begin retrieval using its expected scope after the failure is resolved. Do
not reuse old context. A capture `gap` similarly preserves the committed task
change and previous capture default; existing turns are never relabeled.

A transport failure can leave a mutation's outcome unknown. Inspect its existing
history/request ID before retrying; neither bridge nor server replays writes
automatically. Current retention, rejection and erasure affect new reads, but
cannot retract context already delivered to an external host. Never revoke a
Vault token or lease as troubleshooting or fixture cleanup.
