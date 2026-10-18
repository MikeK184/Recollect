# Publish and consume shared Brain guidance

## Purpose and Prerequisites

Brain managers publish skills and instructions once; connected agents discover
applicable metadata and fetch only the guidance/resources they need. Use a server
with the managed-skills migration and a plugin built from the same delivered
source. An older installed plugin may lack the new hook/CLI/bridge behavior.
The [first pack](../roadmap/execution/archive/mcp-managed-skills.md) records the
canonical registry/cache delivery. The [native successor](../roadmap/execution/archive/mcp-native-skill-reload.md)
adds actual Codex/Claude/OpenCode registration, lifetime and reload proof. These
local fixtures do not upgrade your installed package or deploy the server.

A signed-in Brain administrator publishes through the browser. Readers can consume
enabled applicable guidance; writer access alone does not permit publication.
The plugin needs its existing server/Brain connection and OS-stored credential.
No execution runner, additional provider credential or model call is needed.

[The contract](../contracts/mcp-managed-skills.md) defines the bounded initial
format: Markdown/plain-text references with `SKILL.md` or `AGENTS.md`, safe paths,
standard benign frontmatter, and no scripts, dynamic shell expansion or host
permission overrides. Authored guidance remains below current human, host and
applicable local repository instructions.

## Procedure

### Publish and maintain as a manager

1. Open the Brain's **Agents → Skills** tab and choose **Publish skill**.
2. Set a unique lower-case name, description, kind and applicability. Brain-wide
   guidance is available without an environment selection; choosing an environment
   adds that environment's guidance to the Brain-wide set.
3. Enter `SKILL.md` for a skill or `AGENTS.md` for general instructions. Add
   `.md`/`.txt` references as needed. For a skill, frontmatter `name` and
   `description` must exactly match the form fields, for example:

   ```markdown
   ---
   name: vault-inspection
   description: Inspect Vault configuration with attributable evidence.
   ---

   Read the applicable repository instructions first.
   Distinguish declared configuration from observed runtime behavior.
   Use references/evidence.md for the team's evidence format.
   ```

4. Optionally add technology tags and accompanying existing MCP profiles. A profile
   association is a helpful reference; actual tool execution still needs its
   independent Use permission.
5. Publish. The successful response supplies the stable ID and revision. Editing
   publishes a complete replacement with a new integer revision. A version
   conflict preserves the draft: reload the current record, compare changes and
   deliberately publish again.
6. To stop future consumption, disable the bundle. Managers can select **Include
   disabled** to inspect/edit it. Disable blocks new client fetches, including
   old revisions; it cannot retract a copy or conversation already delivered.

The catalog accepts at most 64 bundle identities per Brain, including disabled
ones. Publication validates combined metadata, per-file/bundle size, safe paths,
frontmatter and detected credential patterns before committing anything. Do not
store secrets in guidance; use existing credential/profile mechanisms.

### Consume through a connected coding agent

The plugin synchronizes applicable native skills at supported task/context/scope
boundaries, using temporary task ownership by default. It advertises compact
metadata and a private native catalog pointer; full bodies are loaded when a
host activates a skill. Automatic fetch has a 2.4-second network budget within
the 7.5-second hook deadline and 8-second process watchdog. An incomplete sync
reports its state and can be retried explicitly.
Ask the agent to use the applicable shared guidance; its bundled bridge skill
teaches this sequence:

1. Reuse a current read-capable immutable operation from the plugin's task context,
   or start a task/read operation through the existing workspace tools.
2. Call `skills.list` with `operation_id` and an optional metadata `query`.
3. Call `skills.fetch` with that operation and the selected `id`. The result
   contains the entrypoint, provenance and full resource path manifest.
4. Read/apply the relevant guidance. Fetch a referenced resource with the same
   `id`, operation and returned `version`, adding its exact `path`.

The operation fixes the environment. Change workspace scope and begin a new read
operation to select another environment; do not supply a different Brain or
identity. [Scoped tools](agent-memory-tools.md#scoped-tools) explain task creation
and operation kinds. Direct MCP clients can call these tools deliberately; they
receive no automatic plugin-context guarantee.

### Install and refresh native host skills

Use the bundled executable, an absolute `WORKSPACE` path and a current read
operation. `NATIVE_HOST` is `codex`, `claude_code` or `opencode`. `TASK_UUID` is
that operation's task ID and `SKILL_UUID` comes from discovery.

```sh
"$PLUGIN" skills install "$SKILL_UUID" --operation "$READ_OPERATION_UUID" --host "$NATIVE_HOST" --directory "$WORKSPACE" --session "$TASK_UUID"
"$PLUGIN" skills sync --operation "$READ_OPERATION_UUID" --host "$NATIVE_HOST" --directory "$WORKSPACE"
"$PLUGIN" skills status --host "$NATIVE_HOST" --directory "$WORKSPACE"
"$PLUGIN" skills leases --directory "$WORKSPACE"
"$PLUGIN" skills reload --host "$NATIVE_HOST" --directory "$WORKSPACE"
```

Install selects one bundle; sync reconciles the complete applicable catalog.
Sync defaults to the operation's temporary task. Add explicit `--persist` to
install or sync to pin the native export and its separately owned canonical cache
copy. A `--session` value must equal the read operation's task; it cannot be
combined with `--persist`. The plugin resolves the nearest Recollect workspace or
Git root and rejects home/global-host roots or an inconsistent Brain selector.

Exports use owned aliases under `.agents/skills` for Codex, `.claude/skills` for
Claude and `.opencode/skills` for OpenCode. An instructions bundle becomes a
namespaced `SKILL.md` wrapper while preserving its original `AGENTS.md` resource.
Existing local AGENTS/CLAUDE files and unowned skills are preserved. Before
following the wrapper's body/references, use its required current scoped
`skills.fetch` check. Authored text never grants tools or overrides local rules.

The workspace-wide `.recollect/managed-skills-native/` ledger coordinates all
three hosts and separate plugin data roots. Same-scope tasks share references;
one ending does not remove another's active copy. A different active endpoint,
Brain or environment returns `native_scope_busy` without modifying peer exports.
A temporary session controls lifetime, not privacy from other local readers.
Keep generated native roots/ledger receipts out of Git deliberately; the plugin
does not silently edit ignore rules.

Placement and refresh are separate results. CLI install/reload returns a refresh
request and the next action, with `native_loaded=false`; it does not attach to an
unrelated host and claim that your existing host loaded the files.

| Host | Refresh behavior |
| --- | --- |
| Codex | Native file watching; a connected supported app-server can force-refresh its catalog. If the current host has not observed a change, refresh its catalog or restart. |
| Claude Code | Supported startup hooks request `reloadSkills: true`. Existing discovered skill roots watch SKILL.md changes. A root created after session start is not watched even after `/reload-skills`: repeat that command after each later change in that session, or start a fresh session with the root already present. The verified 2.1.302 native control path uses `reload_skills`. |
| OpenCode | The Recollect adapter reconciles its owned transform and calls `ctx.skill.reload()` at the next supported context boundary, including owned compatibility-root deduplication. |

To remove the matching native reference or persistent pin:

```sh
"$PLUGIN" skills uninstall "$SKILL_UUID" --host "$NATIVE_HOST" --directory "$WORKSPACE" --session "$TASK_UUID"
"$PLUGIN" skills uninstall "$SKILL_UUID" --host "$NATIVE_HOST" --directory "$WORKSPACE" --persist
```

Uninstall affects native registration. Canonical cached copies are separate:
use `skills discard` below to remove them. Supported session end and ended-task
recovery release temporary native/cache references, including offline cleanup.
Retained pins survive temporary cleanup. Already-loaded conversation text cannot
be retracted by either command.

### Fetch, retain or discard a local copy

Use the packaged `bin/recollect-plugin` executable. In the examples, `PLUGIN` is
its absolute path, `READ_OPERATION_UUID` is the current read operation,
`TASK_UUID` its task ID and `SKILL_UUID` an ID returned by discovery. These are
ordinary identifiers, not credentials.

```sh
"$PLUGIN" skills list --operation "$READ_OPERATION_UUID" --query vault
"$PLUGIN" skills fetch "$SKILL_UUID" --operation "$READ_OPERATION_UUID"
"$PLUGIN" skills fetch "$SKILL_UUID" --operation "$READ_OPERATION_UUID" --path references/evidence.md
"$PLUGIN" skills save "$SKILL_UUID" --operation "$READ_OPERATION_UUID" --session "$TASK_UUID"
"$PLUGIN" skills save "$SKILL_UUID" --operation "$READ_OPERATION_UUID" --persist
```

Fetch returns the entrypoint by default and exact referenced text with `--path`.
Save retrieves all validated files and returns the saved entrypoint's absolute
path. `--version N` optionally selects a historical revision while still checking
current access, enabled state and applicability. Temporary save's `--session`
value must equal the operation's task ID; a random host-session string is invalid.
Choose exactly one lifetime: `--session TASK_UUID` or `--persist`.

The returned copy stays in a private plugin-owned endpoint/Brain/device/version
namespace. Read the returned entrypoint explicitly; save reports
`native_registered=false` and `native_loaded=false`. It does not edit local
AGENTS files, register a host skill or prove native reload.

To remove the addressed bundle's unchanged saved versions:

```sh
"$PLUGIN" skills discard "$SKILL_UUID" --session "$TASK_UUID"
"$PLUGIN" skills discard "$SKILL_UUID" --persist
```

Supported SessionEnd/SubagentStop cleans temporary copies under a lock and fences
late saves. Ended-session delivery recovery repeats cleanup when required.
Persistent copies require explicit discard. Neither operation removes the
instructions from a host conversation that has already received them.

## Verification

A working publication returns a versioned bundle and refreshes the manager list.
Discovery returns metadata without file bodies. An entrypoint fetch returns one
file plus the full path manifest; a resource fetch returns only that resource.
Global/environment scope, current access and disabled state govern subsequent
fetches. Profile associations never create tool permission.

A successful save returns its actual cache path and identifies its lifetime.
Check supported session cleanup or explicit discard against that exact path.
Native status proves owned placement. A host catalog proves discovery, and native
activation with the expected body/reference proves consumption. The
[installed-host proof](../mappings/native-skill-reload-host-proof-2026-10-18.md)
records same-process updates/removal on Codex 0.162.1, Claude 2.1.302 and OpenCode
2.0.22 in isolated fixtures, including temporary/retained copy lifetimes. It does
not establish your running host's state or a deployed server upgrade.

To reproduce native transport/lifetime proof from this repository:

```sh
python3 scripts/test-native-skills.py --host all
```

The script builds the native fixture example and uses isolated homes/config,
owned workspace state and loopback synthetic model providers. It prints the local
results receipt path. Use `--host codex`, `claude_code` or `opencode` for one host;
`RECOLLECT_CODEX_BIN`, `RECOLLECT_CLAUDE_BIN` and `RECOLLECT_OPENCODE_BIN` select
explicit installed executable paths. No paid inference or personal credential is
used. This proves host transport/activation and local lifecycle; actual canonical
service connectivity and model obedience to guidance remain distinct checks.

## Failure and Recovery

- **Native scope busy:** Inspect `skills leases`; preserve the live peer's scope.
  End/release the relevant task before switching native scope. Scoped MCP fetch
  remains usable. OpenCode keeps the peer's established registry on this result.
- **Native sync unavailable:** No incomplete member fetch is treated as deletion.
  A failed scope transition releases the changing task's old refs, preserves live
  peer refs, and clears only its own stale view when no peer holds it. Retry sync
  with the current operation when connectivity returns.
- **Native refresh pending:** Follow the exact host action returned by reload.
  Native placement, expected watchers and a successful control call are not by
  themselves proof that the current conversation consumed a new body.

- **Guidance unavailable:** Check current Brain access, operation ownership/kind,
  task closure and environment. Disabled/wrong-environment bundles are not
  available through old revisions. An administrator can inspect disabled content
  through the browser; paired-device publication is always denied.
- **Catalog unavailable in a prompt:** Normal work and recall remain usable.
  Retry `skills.list` with the current read operation; a bounded catalog failure
  does not authorize inspecting server internals or inventing guidance.
- **Validation/capacity conflict:** Correct the displayed field/path/frontmatter
  error or edit an existing identity. No script/host-override fallback is accepted.
- **Local cache conflict or cleanup warning:** The plugin preserves modified,
  unowned and symlinked content. Inspect the exact returned managed directory,
  preserve wanted edits and resolve the conflict deliberately before retrying.
  There is no force-delete option. Partial `pending-*` staging directories left
  by a crash are retained for inspection, not claimed as automatically removed.
- **Session already ended:** Begin a new task/read operation and use its task ID.
  The ended-task fence intentionally prevents delayed saves into the old lifetime.
- **Offline persistent copy:** It is a last-fetched export. Manual reading cannot
  establish current eligibility, and disable/revocation cannot recall exported
  bytes. Reconnect for a current authenticated fetch before automatic activation.
