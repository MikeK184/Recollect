# Native managed-skill installation, lifetime and refresh

Status: accepted
Date: 2026-10-18

## Source

The user's follow-up explicitly requires testing temporary/retained copies and
implementing real native install/reload for Codex, Claude Code and OpenCode.
[ADR 0022's amendment](../adr/0022-brain-managed-skills.md), the existing
[managed-skills contract](mcp-managed-skills.md),
[workspace scope](evidence-workspace-scope.md) and
[plugin lifecycle](mcp-plugin-session-memory.md) govern this successor.
The [first shipped pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-managed-skills.md)
remains delivery history, not a native-install proof.

## Contract

### Native exports and authority

Keep the server's existing canonical bundles, format validation, current
eligibility and read operations. Native adaptation is owned by the local plugin;
no server schema, permission grant or model provider is introduced. Cache `save`
and native `install` have separate ownership/state. A native export contains
validated source files and a generated, namespaced `SKILL.md` with provenance,
applicability and the required canonical-read check. Instructions bundles are
presented as native instruction skills, retaining the source `AGENTS.md` as a
resource. This is not editing/reloading the host's AGENTS/CLAUDE hierarchy.

Every generated entrypoint states its endpoint/Brain, environment applicability,
source bundle/version and that the agent must use a current scoped `skills.fetch`
operation before following its cached body/references. A changed canonical
version, unavailable/disabled bundle or mismatched scope requires current guidance
or stopping that guidance. Human, host and applicable local repository instructions
retain priority. Native files are workspace-visible exports: a task controls their
lifetime, not confidentiality from other local programs or host sessions.

Resolve `--directory` through the same nearest Recollect workspace boundary or
Git root used by the plugin. Refuse a Brain selector inconsistent with the
operation, filesystem root, home directory or a global host configuration root.
Use canonical absolute workspace identity; do not follow a changed working
directory into another Brain. An explicit isolated fixture workspace is permitted.
A module below a root must not accidentally register only at the module or home.

| Host | Workspace native root / adapter |
| --- | --- |
| `codex` | `.agents/skills/{owned-alias}/SKILL.md`; supported app-server refresh or verified native watcher |
| `claude_code` | `.claude/skills/{owned-alias}/SKILL.md`; verified watcher, `/reload-skills` where necessary |
| `opencode` | `.opencode/skills/{owned-alias}/SKILL.md` and native plugin skill transform/reload |

Aliases are stable `recollect-{codex|claude|opencode}-{bundle UUID without dashes}`
namespaces; no existing local skill is shadowed or semantically merged.
OpenCode scans compatibility roots too, so its adapter reconciles/deduplicates
owned exports by canonical source identity and chooses its own host export.
Unowned local skills and host permission/configuration files are preserved.

### Workspace-wide scope and lifetimes

Use a workspace-wide lock/lease ledger across all three host kinds because their
native discovery roots overlap. An active scope is the exact endpoint, Brain and
selected environment, including null. Same-scope tasks may coexist and each holds
its own host/export reference. A conflicting active scope yields
`native_scope_busy` and makes no native export changes; scoped MCP remains
available. Host-only locks are insufficient. This prevents competing managed
writes; it does not claim isolation from unrelated local tools reading the files.

When no active lease remains, another scope may become active. Deactivate/remove
only unchanged, receipt-owned exports from the old scope, retaining separately
owned persistent canonical copies for later activation. No environment-specific
persistent export is silently republished as global guidance. On scope changes,
release only the changing task's references, then reconcile its new scope.

Task-temporary installation is the automatic default. Its session value is the
validated operation's task UUID. Persistent installation is explicit `--persist`.
The ledger records source/version, canonical workspace, host, endpoint/Brain/
environment, exact generated path/content manifest, owner and temporary task refs
or persistent intent. Same-scope tasks sharing an export do not let one task's
closure delete another's active copy. Persistent intent survives temporary task
closure; unregistering a native export is distinct from erasing canonical cache.

Supported task/session end, recovery of ended tasks and explicit uninstall
release the matching lease and clean eligible unchanged temporary exports.
Persisted copies survive session end and are removed only by explicit persistent
uninstall/discard. Cross-process locking and ended-task fencing prevent late save/
install work from resurrecting closed task exports. Before network fetch, record
a server-observed task scope generation and its creation ordering. Native apply
and recovery/catalogue metadata publication recheck the expected generation under
the workspace lock; a scope transition or offline release invalidates old work.
A delayed fetch cannot reinstall the old scope or overwrite a newer snapshot,
and a delayed denial may release only the generation it actually checked. Test two live tasks and
closure in both orders, alongside simultaneous retained copies.

The shared registry/lock live under workspace
`.recollect/managed-skills-native/registry.json` and `registry.lock`, with owned
`receipts/` and `staging/` subdirectories. This common workspace guard coordinates
separate plugin data roots/devices and all host kinds. Registry records metadata
and pending transaction pointers; private receipt UUID files carry the exact
rendered bytes, bounded to 2 MiB and bound to workspace/endpoint/Brain/environment/
device/host/source/version. No product hash is introduced. Keep generated local
state out of Git; the installer does not edit repository ignore rules silently.

### Safe filesystem lifecycle

Create/replace through a same-filesystem owned staging location with validation
of the actual staged paths/bytes before success. Revalidate source bundles before
native transformation; generate only benign supported frontmatter and retain
relative references. Never execute scripts, dynamic shell interpolation, native
permission overrides or installation hooks. Existing text-only bounds apply.

Reject symlinked roots/targets, traversal, case/prefix collisions, unknown aliases,
unowned directories and modified receipt-owned files. An existing target may be
replaced/removed only when its complete actual manifest equals the owned receipt.
A conflict preserves user edits and reports exact safe recovery status. Never use
a force-delete fallback or silently adopt foreign directories. Preserve orphaned
crash-stage or unknown files for explicit inspection. No content purge is promised
from a host conversation, external copy or another application's cache.

### CLI and automatic synchronization

The bundled plugin CLI adds:

```text
skills install ID --operation UUID --host codex|claude_code|opencode
  --directory PATH (--session TASK_UUID | --persist)
skills sync --operation UUID --host HOST --directory PATH
  [--session TASK_UUID | --persist]
skills reload --host HOST --directory PATH
skills uninstall ID --host HOST --directory PATH
  (--session TASK_UUID | --persist)
skills status --host HOST --directory PATH
skills leases --directory PATH
```

Read/install/sync reuse existing private credential resolution. Task-bound
operations must belong to this account/device/Brain and be read-capable. Sync
without a lifetime flag derives temporary lifetime from its operation's task;
`--persist` is explicit retained intent. Status/leases/uninstall inspect only
matching local ownership and do not confer server permission. No credential,
command/executable, endpoint override or arbitrary host RPC target is accepted
from authored skill text.

Native plugin task/prompt/scope lifecycle automatically reconciles applicable
enabled metadata and materializes needed changed bundles under the existing
bounded hook deadline (2.4 seconds for automatic network work, 7.5 seconds for
the hook and an 8-second process watchdog). It rechecks canonical eligibility; stale files/caches do
not authorize a fresh sync. Changes/disable remove only eligible owned native
registrations while preserving explicitly retained canonical copies. Unchanged
versions avoid rewriting local files. Incomplete or timed-out sync reports
partial/unavailable state, preserving valid ownership and allowing explicit
`skills sync` retry. Private task recovery pointers live under plugin-data
`native-skills/tasks`, and metadata-only host snapshots under
`native-skills/catalogues`. They bind task/scope/device/workspace; snapshot
`owned_exports` covers all three host roots for verified OpenCode deduplication.
The final escaped 8 KiB hook envelope preserves the native pointer and Claude
startup rescan action after coordination/memory output is merged. It must not
exceed the existing hook budget or block ordinary
capture/recall indefinitely. OpenCode retains an established peer registry on scope-busy results; an
unavailable transition with no peer clears the changing task's old view.
Scope/ownership conflicts do not fall through to
another Brain or global installation.

### Host refresh and observable states

Keep distinct `placed`, `discovered`, `loaded`, `updated`, `removed`,
`refresh_requested`, `reload_required`, `restart_required`, `scope_busy`,
`conflict`, `partial` and `unavailable` outcomes. Filesystem success establishes
placement only. A host registry/catalog confirms discovery; actual native skill
activation with the expected content confirms loading. A notification invalidates
an old catalog, not proof of consumption. Replacing/removing files cannot retract
instructions already in a conversation.

- Codex: use the installed version's supported `skills/list` force-reload and
  watcher behavior when connected to the actual owned host. Current generated
  0.162.1 schema exposes `cwds`/`forceReload` and process-only
  `skills/extraRoots/set`; latest documentation fields absent from that schema
  cannot be assumed. A separately spawned probe proves its own host only.
- Claude Code: existing discovered skill directories may be watched; creating a
  previously absent root can require `/reload-skills`. Installed 2.1.302 exposes
  the `reload_skills` control subtype and SessionStart hook-specific
  `reloadSkills: true`; use these verified native capabilities for startup/control
  refresh, and test their actual response/consumption. Older hosts without the
  supported capability return the precise reload/restart action. A root created
  after startup is not watched even after `/reload-skills`; repeat that action
  after later changes there in the same session, or use a fresh session with the
  root already present. Do not inject
  a fake model prompt and label it a native reload control operation.
- OpenCode: maintain the owned transform with add/update/remove, call the native
  `ctx.skill.reload()` API after relevant state changes, and verify the resulting
  native registry and activation. Use the real installed API shape; no universal
  version/config assumption is accepted.

The public result/status includes source/version/path, lifetime/lease scope,
placement and refresh state, and the next reload/restart action when required.
Never return `native_loaded=true` or equivalent merely from a successful copy,
watcher expectation, API acknowledgement or another host's test receipt.

## Acceptance

- Temporary, persistent and combined copies pass filesystem/CLI lifecycle tests,
  including multiple task refs, same-scope cross-host coexistence, conflicting
  cross-host environment/Brain leases, scope switches, delayed installs, supported
  end/recovery, crash stages, local edits, unknown paths and symlink denial.
- Real Codex, Claude Code and OpenCode fixtures prove native catalog discovery and
  native activation of the expected body/reference. Within the same running host,
  exercise add, version update and removal; verify old/new markers and future
  native availability. If the host needs a supported reload/restart, exercise it
  and report that boundary instead of claiming seamless hot reload.
- Test instructions-as-native-skill with the original AGENTS resource preserved
  and existing local AGENTS/CLAUDE/skills intact. Current scoped canonical fetch
  remains required; stale/disabled/revoked content never grants execution rights.
- Use owned isolated workspaces/home/config and synthetic provider endpoints;
  record actual binary versions/platforms. Claude may use the existing Linux
  native-host fixture when absent on macOS. No paid provider run or personal
  credential/global host configuration mutation is required.
- Focused protocol/native/plugin tests, host-adapter interface tests, installed
  host proof, `git diff --check` and `./scripts/validate.sh` pass. Update the
  operator/public guide and record separate placement/discovery/activation proof.

## Explicit Deferrals

Global home/admin skill installation, modification of AGENTS/CLAUDE instruction
hierarchies, executable bundles/automatic learning, arbitrary marketplace import,
unsupported universal hot reload, privacy from arbitrary local filesystem readers
and retraction of already-delivered conversation context. Installed-host proof is
required for this successor and is no longer deferred.
