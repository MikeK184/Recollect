# Capture and recall coding-session evidence

> Plugin migration: [ADR 0018](../adr/0018-plugin-managed-agent-memory.md)
> makes the packaged plugin the normal install/connect path for Codex, Claude Code
> and OpenCode. [Package instructions](../../plugins/recollect/README.md) describe
> the delivered implementation. Native ordinary-launch acceptance passed; the
> legacy procedures remain for draining existing queues and advanced use.
> Do not remove an old setup or revoke its device before its pending captures have
> drained under their original endpoint, Brain, task and binding. Do not enable both
> capture paths for one session. No credentials or host trust settings are migrated
> silently.

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Plugin-managed capture and recall

Open **Agents → Connect coding agent** and follow the [package setup](../../plugins/recollect/README.md).
Install the complete plugin, connect once, approve the host's hook trust, then start
Codex, Claude Code or OpenCode normally. This path uses the bundled
`recollect-plugin`; it needs no separate companion or `capture run` command.
Native-host acceptance and remaining limits are recorded in the
[plugin mapping](../mappings/plugin-session-memory-2026-10-01.md).

The plugin establishes the session's Brain/task scope, captures permitted events
locally, and uploads through an independent worker. The standing Brain capture
policy governs admission; model policy separately governs learning. Reader-only
access and disabled capture do not prevent authorized memory recall.

Before prompts reach the model, recall supplies up to 8 KiB of cited data under
the current task scope. Compaction retrieves again. The total hook deadline is
eight seconds; network or credential failure leaves coding available. The
plugin's own recalled context and transport results are excluded from fresh
evidence. No hidden reasoning or transcript files are scraped.

Use **Agents → Captured sessions** to inspect published sources and **Capture
coverage** for delivery reports and gaps. An empty Brain can legitimately return
no useful memory. A configured plugin is not proof of capture or useful recall.
The packaged executable supports:

```sh
/absolute/package/plugins/recollect-memory/bin/recollect-plugin status
/absolute/package/plugins/recollect-memory/bin/recollect-plugin drain
```

Pending events survive host exit and are retried on later launches. A delivery
worker has a bounded final drain. Retention expiry and synchronized erasure fences
apply before replay. Changing the selected device/server retains each old queue's
original destination and credential reference; it does not move old evidence into
the new Brain. Optional independent execution uses `connect --with-runner` and is
unrelated to capture delivery.

## Legacy capture setup and queue migration

The procedures below describe the earlier companion-managed launch. Use them to
inspect/drain existing queues or diagnose that explicit compatibility path. Do not
run it alongside the complete plugin for the same session. Drain before removing
the old host entry, enable the plugin, verify delivery, then retire the old setup
files. Installing a new plugin does not erase old canonical sources.

### Legacy prerequisites

Run `./scripts/dev.sh` and open `http://127.0.0.1:8787`. Use **Agents → Captured sessions** for published evidence and **Capture coverage** for companion delivery; the canonical policy editor is **Settings → Capture**. For this compatibility path, use a
[paired native companion](device-pairing.md) with writer access, and install the
selected host on PATH. Codex 0.154.0 and Claude Code 2.1.270 were exercised with
actual host processes and the compiled Recollect hook.

An admin enables **Settings → Capture → Capture policy** once for the Brain. Supported events then
flow automatically; no person accepts each memory. Capture permission is separate
from the [model policy](provider-learning.md). That policy governs automatic
learning and revision; [retention](retention-and-erasure.md) governs expiry and
erasure. The default for raw sessions and tool output is thirty days.

### Legacy procedure

From Recollect, prepare capture for an explicit Brain and working directory:

```sh
cargo run -p recollect-agent -- capture setup codex /absolute/workspace/path --brain BRAIN_UUID
```

Choose `claude_code` instead of `codex` for Claude. If the workspace has a nearest
`.recollect/workspace.toml` selector, `--brain` can be omitted. Setup reads that
selection without changing the workspace. It creates a Brain-wide task by default;
use `--task TASK_UUID` to capture in an existing owned task's scope. Directory
names alone do not select a repository or environment.

Setup returns an inspectable Brain URL, private setup/hooks paths and a copyable
`run_command`. Run that command to launch the host with background delivery:

```sh
target/debug/recollect-agent capture run /absolute/path/to/capture.json
```

Append host arguments after `--`, for example `-- exec 'Describe this project'`
for Codex. Normal host sign-in and workspace trust still apply. Codex's explicit
managed launch registers the generated Recollect plugin through Codex's own
installer. Other plugins/settings are preserved. The bundle also carries the
`recollect-memory` skill, which describes the existing scoped MCP tools for that
Brain; skills grant no authority and change no capture behavior. The installed hook captures
only when that launch supplies its setup binding; ordinary unbound Codex sessions
do not inherit a Brain destination. Claude loads the generated settings directly.

`RECOLLECT_URL` and `RECOLLECT_DEVICE_PROFILE` select the endpoint and paired
profile at setup time. The returned command retains those choices. Storage defaults
to Recollect's companion evidence directory; `RECOLLECT_PUBLICATION_DIR` can select
another private directory inside Recollect. `--output DIRECTORY` selects a new
setup directory inside Recollect and refuses to overwrite existing setup files.

Reuse the returned command across sessions. Host upgrades automatically create or
reuse a binding for the new host version while preserving the original operation
scope. A changed task scope requires a new setup. Concurrent setups keep their
own bindings; no shared current-Brain variable labels their events.

### Legacy verification

The panel distinguishes **Configured only** from actual device reports and
server-confirmed publications. **View captured source** opens the retained,
sanitized evidence. Policy learning appears in the existing model activity and
claim views. A successful setup alone does not prove capture or model connectivity.
Source history shows when Recollect recorded an event separately from when the
host captured it. Delayed uploads do not appear in earlier knowledge-time recall
and do not restart the raw-content retention period.

When automatic learning is enabled, canonical role and event attribution accompany
the original source lines. Supported later corrections in the same bound session
can revise an existing memory automatically. Assistant suggestions and historical
quotations are qualified evidence; neither supplies operational verification.
Recognizable Recollect capture/recall commands produce a content-free coverage
marker, preventing the companion's own recalled text from becoming new evidence.

Inspect queued work without contacting the service or OS credential store:

```sh
target/debug/recollect-agent capture status /absolute/path/to/capture.json
```

The result reports local queue/denial counts, coverage gaps and the last policy
refresh. Its `connection: not_checked` is deliberate. Finish a delivery pass
independently of the host with:

```sh
target/debug/recollect-agent capture drain /absolute/path/to/capture.json
```

The managed launcher delivers every two seconds and attempts a bounded final
drain. Independent drain processes at most twenty queued events per pass; repeat
it if more remain. Source text is removed locally after acknowledgment. Hooks
perform only bounded normalization and a local SQLite transaction, with no model
or network call. They do not read transcripts or hidden reasoning.

## Shared privacy and recovery

Offline work remains in the durable inbox and resumes on the next run/drain.
Capture permission cached for more than 24 hours cannot admit new content until
refreshed. Local expiry still clears old bodies while offline. A stale device
report is displayed as **Offline or stopped** after thirty seconds; this does
not distinguish a closed launcher from network loss.

Revoked access or disabled policy denies publication. Restored writer permission
and enabled policy allow retry after the bounded denial delay. Conflicting event
identities remain visible; they are never overwritten. Queue limits preserve
existing pending evidence and count arrivals that could not be saved. Unsupported
host events, ambiguous child attribution and missing output remain explicit gaps.

Use source **Erase** and **Settings → Retention & privacy** for controlled content. Delivery
synchronizes deletion fences and clears affected local bodies before acknowledging
privacy progress or selecting uploads. A disconnected device applies new erasure
when it next connects. Removing a host plugin or setup file is not source erasure.

## Verification commands

`./scripts/test-mcp-hosts.sh` includes installed-plugin native Codex, Claude Code
and OpenCode proofs against owned databases and a synthetic local model.
`./scripts/test-capture-hosts.sh` separately runs pinned legacy Codex/Claude hosts in a
repository-owned container. The execution phase has networking disabled and uses
synthetic local provider responses; personal host configuration and credentials
are not mounted. The test covers actual native hooks, an unbound Codex session and
child exit-status preservation. `./scripts/test-platform.sh` covers native
setup/delivery, scope, policy, replay and deletion against the real API/database.
`./scripts/test-ui.sh tests/capture.spec.ts tests/retention.spec.ts` exercises
browser delivery states, canonical source erasure and lost-response recovery.
See the [dated integration evidence](../mappings/session-capture-interfaces-2026-09-14.md)
for measured results and coverage limits.
