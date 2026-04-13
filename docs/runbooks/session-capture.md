# Capture Codex and Claude session evidence

## Purpose and Prerequisites

Run `./scripts/dev.sh` and open `http://127.0.0.1:8787`. The Brain's **Session
capture** panel shows policy, companion delivery and published evidence. Use a
[paired native companion](device-pairing.md) with writer access, and install the
selected host on PATH. Codex 0.154.0 and Claude Code 2.1.270 were exercised with
actual host processes and the compiled Recollect hook.

An admin enables **Capture policy** once for the Brain. Supported events then
flow automatically; no person accepts each memory. Capture permission is separate
from the [model policy](provider-learning.md). That policy governs automatic
learning and revision; [retention](retention-and-erasure.md) governs expiry and
erasure. The default for raw sessions and tool output is thirty days.

## Procedure

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
installer. Other plugins/settings are preserved. The installed hook captures
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

## Verification

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

## Failure and Recovery

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

Use source **Erase** and the Brain retention panel for controlled content. Delivery
synchronizes deletion fences and clears affected local bodies before acknowledging
privacy progress or selecting uploads. A disconnected device applies new erasure
when it next connects. Removing a host plugin or setup file is not source erasure.

## Local proof

`./scripts/test-capture-hosts.sh` runs pinned real Codex/Claude hosts in a
repository-owned container. The execution phase has networking disabled and uses
synthetic local provider responses; personal host configuration and credentials
are not mounted. The test covers actual native hooks, an unbound Codex session and
child exit-status preservation. `./scripts/test-platform.sh` covers native
setup/delivery, scope, policy, replay and deletion against the real API/database.
`./scripts/test-ui.sh tests/capture.spec.ts tests/retention.spec.ts` exercises
browser delivery states, canonical source erasure and lost-response recovery.
See the [dated integration evidence](../mappings/session-capture-interfaces-2026-09-14.md)
for measured results and coverage limits.
