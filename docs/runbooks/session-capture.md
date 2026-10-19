# Capture and recall coding-session evidence

> Plugin migration: [ADR 0018](../adr/0018-plugin-managed-agent-memory.md)
> makes the packaged plugin the normal install/connect path for Codex, Claude Code
> and OpenCode. [Package instructions](../../plugins/recollect/README.md) describe
> the delivered implementation. Native ordinary-launch acceptance passed. The
> companion-managed launch (`capture setup`, `run`, `status`) was retired; only
> `capture drain SETUP_FILE` remains, to empty old queues.
> Do not remove an old setup or revoke its device before its pending captures have
> drained under their original endpoint, Brain, task and binding. Do not enable both
> capture paths for one session. No credentials or host trust settings are migrated
> silently.

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Plugin-managed capture and recall

Open **Agents → Connect coding agent** and follow the [package setup](../../plugins/recollect/README.md).
Install the complete plugin, connect once, approve the host's hook trust, then start
Codex, Claude Code or OpenCode normally. This path uses the bundled
`recollect-plugin`; it needs no separate companion or launch wrapper.
Native-host acceptance and remaining limits are recorded in the
[plugin mapping](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/mappings/plugin-session-memory-2026-10-01.md).

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

## Retired companion-managed launch

The earlier companion-managed launch (`recollect-agent capture setup`, `run`,
`status` and its generated hooks) no longer exists, including its embedded Codex
plugin generator. Its setup files, hooks and queued evidence can still be present
on a machine that used it. The plugin detects them (legacy `recollect-agent capture
hook` entries, an enabled `recollect-capture` plugin, a first-party `recollect` MCP
server, or `RECOLLECT_CAPTURE_SETUP`/`RECOLLECT_CAPTURE_LAUNCH`) and withholds
plugin capture until they are removed, so one session never records twice. A
leftover legacy hook is now a silent no-op that prints a one-line notice.

Migrate in this order:

1. Empty the old queue with its original setup file and paired profile; repeat
   while events remain (at most twenty per pass):

   ```sh
   recollect-agent capture drain /absolute/path/to/capture.json
   ```

   The command honours the setup's original endpoint and device profile and
   refuses another. Source text is removed locally after acknowledgment.
2. Remove the old hook entries and first-party MCP entry from the host
   configuration, and delete the old setup files once nothing is pending.
3. Install the complete plugin as described above and verify delivery.

Do not revoke the old device credential while its queue still holds events.
Installing the plugin does not erase old canonical sources.

## Verifying capture

An admin enables **Settings → Capture → Capture policy** once for the Brain.
Supported events then flow automatically; no person accepts each memory. Capture
permission is separate from the [model policy](provider-learning.md), which governs
automatic learning and revision; [retention](retention-and-erasure.md) governs
expiry and erasure. The default for raw sessions and tool output is thirty days.

The panel distinguishes **Configured only** from actual device reports and
server-confirmed publications. **View captured source** opens the retained,
sanitized evidence. Policy learning appears in the existing model activity and
claim views. A successful connection alone does not prove capture or model
connectivity. Source history shows when Recollect recorded an event separately
from when the host captured it. Delayed uploads do not appear in earlier
knowledge-time recall and do not restart the raw-content retention period.

When automatic learning is enabled, canonical role and event attribution accompany
the original source lines. Supported later corrections in the same bound session
can revise an existing memory automatically. Assistant suggestions and historical
quotations are qualified evidence; neither supplies operational verification.
Recognizable Recollect capture/recall commands produce a content-free coverage
marker, preventing recalled text from becoming new evidence. Hooks perform only
bounded normalization and a local SQLite transaction, with no model or network
call. They do not read transcripts or hidden reasoning.

## Shared privacy and recovery

Offline work remains in the durable inbox and resumes on the next launch or drain.
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
`./scripts/test-platform.sh` covers plugin-session capture and delivery, scope,
policy, replay and deletion against the real API/database.
`./scripts/test-ui.sh tests/capture.spec.ts tests/retention.spec.ts` exercises
browser delivery states, canonical source erasure and lost-response recovery.
See the [dated integration evidence](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/mappings/session-capture-interfaces-2026-09-14.md)
for measured results and coverage limits.
