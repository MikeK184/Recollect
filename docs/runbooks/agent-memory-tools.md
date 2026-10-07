# Connect coding hosts to scoped memory

> Plugin migration: [ADR 0018](../adr/0018-plugin-managed-agent-memory.md)
> makes the packaged plugin the normal install/connect path for Codex, Claude Code
> and OpenCode. [Package instructions](../../plugins/recollect/README.md) describe
> the delivered implementation. Native ordinary-launch acceptance passed; the
> legacy procedures remain for draining existing queues and advanced use.
> Do not remove an old setup or revoke its device before its pending captures have
> drained under their original endpoint, Brain, task and binding. Do not enable both
> capture paths for one session. No credentials or host trust settings are migrated
> silently.

## Plugin setup

Open **Agents → Connect coding agent**. Install the built package for the local
computer, connect once to the server and Brain, then start the coding host
normally. The [package guide](../../plugins/recollect/README.md) gives the native
Codex/Claude installation commands and OpenCode V2 plugin entry. A source-only
marketplace without its bundled executable is not an installable package.

The `recollect-connect` skill runs the bundled `recollect-plugin connect --url URL
--brain UUID` command. Approve the displayed device code in the browser. The
credential stays in the OS store; no personal shell export or host-launch wrapper
is needed. Host plugin/hook trust remains the host's own explicit decision.
A nearest `.recollect/workspace.toml` can select another accessible Brain; an
invalid selector does not fall through to a different destination.

The plugin creates an independent task for each native session, injects bounded
cited memory before prompts, captures permitted events and delivers them through
a durable queue. Compaction triggers fresh scoped retrieval. Recalled context
contains the task and scope identifiers to use for subsequent operations.
The [capture guide](session-capture.md) explains delivery, expiry and migration.

Use the packaged executable's `status` command to inspect connection, current
Brain, pending delivery and detected legacy configuration. Successful tools and
server-confirmed capture prove functionality; installed files alone do not.
The current [evidence mapping](../mappings/plugin-session-memory-2026-10-01.md)
records exact native-host versions, verification and deployment boundaries.

## What happens automatically

Under the Brain's standing capture/content/model policy, permitted inputs become
canonical evidence, extracted candidates receive a separate whole-assertion support
check, and supported knowledge becomes usable. Settled sessions produce current
scoped digests. Subsequent plugin prompts receive relevant recall plus a small
supported project brief and authenticated continuation when unambiguous, within
the same 8 KiB/eight-second hook budget. No extra model call assembles the brief.

Late evidence, supported corrections, expiry and erasure invalidate affected
context before later delivery. Minimal exact evidence survives raw expiry only
when both original-content and excerpt permissions allow it. Ambiguous or
unsupported outcomes stay inspectable; they do not create a required review task
or ask the coding agent to explore Recollect's source. Unknown external completion
is not blindly charged again. [The operating flow](provider-learning.md#automatic-support-session-digests-and-later-context)
and [dated proof](../mappings/memory-source-support-staging-2026-10-07.md) distinguish
implemented behavior from the currently installed runtime.

## Optional independent execution

Memory does not start a local execution runner. Include `--with-runner` in the
connect command only when Recollect should independently execute approved tools
on this computer or its private network. Add `--runner-id UUID` for an existing
private registration. Reconnect without the flag to disable the plugin-owned
runner. Existing Use grants, approved targets, leases and Vault rules still apply.
The coding host's own shell and file tools do not need this runner.

## Advanced direct HTTP

Expand **Advanced · Direct MCP connection** in the coding-agent dialog when only
explicit memory/tool calls are needed. The generated project configuration points
at `/api/brains/BRAIN_UUID/mcp/agent`; it does not install capture or automatic
prompt hooks. Keep only one first-party Recollect MCP entry for a host.

Create a revocable user token and follow the dialog's credential instructions.
For environment-based setup, `RECOLLECT_MCP_TOKEN` is a literal variable name in
configuration, not a place to paste a token. Export its value through the shown
hidden-input command in the terminal that starts the host. On macOS, the direct
Codex alternative uses a Keychain authorization-header helper instead.
Secrets never belong in source, host settings, command arguments or transcripts.

Device authorization and bearer tokens use Recollect's existing authentication;
this is not an OAuth implementation. The token inherits the account's current
Brain grants. The server's model-provider credential and the coding host's own
model billing remain separate from this user credential.

## Scoped tools

Start a task through `workspace.start_task` with explicit selection and a
`context_query`. Its result contains the immutable scope, refreshed context and
retrieval operation ID. Use that ID for memory and graph reads. Start separate
`write` or `tool` operations for contributions/handovers or managed calls. A scope
change affects future operations; existing operations and child scopes remain
bound to their original selections. Models cannot supply another Brain or
override the selection carried by an operation.

## Legacy companion migration

Drain old captures with their original setup file and credential before removing
the old first-party MCP entry or enabling plugin capture. Keep the old setup until
its pending count is zero; do not revoke a credential still needed by queued work.
See the exact status/drain commands in [capture recovery](session-capture.md).
The plugin reports collisions without rewriting host configuration. Existing
unrelated MCP servers, plugins and credentials remain the owner's configuration.

The legacy native `mcp-config codex|claude|opencode` and `capture run` commands
remain compatibility tools. Their older separate bridge/companion installation is
not the ordinary plugin setup. Prior macOS bridge proof is recorded in the
[desktop acceptance mapping](../mappings/desktop-final-acceptance-2026-10-01.md).

## Verification

Three credentials stay separate: the server's `OPENAI_API_KEY` (operator-only,
never in host config), your `RECOLLECT_MCP_TOKEN` (the only plugin credential),
and the host's own model billing (ChatGPT/Claude/OpenCode provider). A host
"out of credits" error is host billing, never a Recollect defect.

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

Run `./scripts/test-mcp-hosts.sh` for the isolated installed-host, native capture
and local/private managed-tool proofs. It uses disposable databases, Secret
Service and synthetic model responses without a paid model call. To diagnose
one case, set `RECOLLECT_MCP_HOST_FILTER` to its exact Rust test name. The
[2026-10-01 continuation evidence](../mappings/desktop-continuation-2026-10-01.md)
records the current rerun separately from the running installation.

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

## Retain documents through supported tools

The native catalogue includes `source.import`, `source.list` and `source.inspect`.
Use `workspace.begin` with `kind: write` on the plugin's task, then import
authorized text with `operation_id`, a stable `request_id`, title, media type and
`retain_content: true`. Keep the returned source and version UUIDs. Use a context
or retrieval operation for title listing and exact version inspection; incompatible
environments are excluded. Later versions retain the import scope. Normal memory
use does not require browsing Recollect's Rust implementation or private auth files.
