# Discover workspaces and bind task context

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Purpose and Prerequisites

Use the [connected plugin](../../plugins/recollect/README.md), installed Git and
an accessible Brain. Publishing checkout metadata requires writer/admin access;
readers may create their own working tasks. Read-only local discovery needs no
connection. The plugin keeps endpoint and credential setup in personal OS storage. Browser sign-in and workspace selection do not grant tool
execution or repository publication authority.

## Procedure

Place `.recollect/workspace.toml` once at the root of the chosen workspace. It
contains only the Brain selector, for example an exact unique accessible name:

```toml
brain = "bankit"
```

Prefer the Brain UUID shown in the browser when names are ambiguous. Keep
the plugin endpoint and OS credential personal. Do not add endpoint or
credentials to the workspace file. A nearer nested selector defines a separate
boundary, including when it is invalid.

Use the installed plugin runtime to inspect, then publish metadata for your chosen
workspace directory:

```sh
PLUGIN_RUNTIME=/absolute/installed/plugin/bin/recollect-plugin
"$PLUGIN_RUNTIME" workspace discover /path/to/workspace
"$PLUGIN_RUNTIME" workspace refresh /path/to/workspace
"$PLUGIN_RUNTIME" workspace list /path/to/workspace
```

Source-checkout developers may still build `recollect-agent` and use its legacy
paired profile; a plugin consumer needs no Cargo or separate companion.

Discovery is read-only and works from a subdirectory below the selector. Refresh
uploads origin/branch/HEAD/dirty observations, not source contents. Examine
`complete`, `notes`, excluded nested workspaces and each checkout's status.
Local/file origins remain unsupported; Git URLs are normalized without credentials.
Partial scans preserve unseen registrations; a full refresh marks missing paths
as not seen without deleting their repository identities.

In the Brain sidebar, open **Repositories** for published repositories and **Your checkouts**. Checkout paths are private to your account. Admins may **Add origin**
for an explicitly confirmed alias or move. An origin already assigned to another
repository fails rather than merging identities.

Use **Agents → Your working contexts → New task scope** to select repositories, areas and an environment. Empty dimensions
include the whole Brain for that dimension. **Start subagent** copies the selected
parent into an independent child. **Change scope** affects future operations;
**Recorded operations** and **Scope history** retain their original selections.
**Close task** stops new operations without closing its children.

Normal plugin sessions create and close their own tasks automatically. The CLI
provides explicit operations for inspection or deliberate scope management. Set the ID variables below from the
catalogue and task JSON; these UUIDs identify resources and are not credentials:

```sh
"$PLUGIN_RUNTIME" scope start "$BRAIN_ID" "Investigate Vault" \
  --repository "$REPOSITORY_ID" --area "$AREA_ID" \
  --environment "$ENVIRONMENT_ID" --workspace "$WORKSPACE_ID"
"$PLUGIN_RUNTIME" scope begin "$BRAIN_ID" "$TASK_ID" context
"$PLUGIN_RUNTIME" scope fork "$BRAIN_ID" "$TASK_ID" "Repository review"
"$PLUGIN_RUNTIME" scope inspect "$BRAIN_ID" "$TASK_ID"
"$PLUGIN_RUNTIME" scope change "$BRAIN_ID" "$TASK_ID" "$BASE_SCOPE_ID" \
  --repository "$OTHER_REPOSITORY_ID"
"$PLUGIN_RUNTIME" scope close "$BRAIN_ID" "$TASK_ID"
```

Repeat repository/area flags for multiple selections. `scope change` replaces
the entire selection: omitted dimensions become Brain-wide. Task commands use
explicit Brain/task IDs, so changing directories cannot silently rebind them.
`scope begin` records an immutable binding only. It does not execute a tool or
retrieve evidence. A scope change returns a fresh-context handoff with retrieval
available. Begin a new `context` or `retrieval` operation and use
`scope recall BRAIN_UUID OPERATION_UUID QUERY` to obtain fresh attributed evidence;
see [exact and lexical recall](exact-lexical-recall.md).

## Verification

Compare an operation's scope before and after changing the parent task. The old
operation and existing children must retain their original UUIDs/selections.
Check `scope_valid` after a selected view is removed; a new operation must fail
until the task receives a valid selection. Current grant/device revocation must
deny subsequent access.

```sh
cargo test -p recollect-protocol -p recollect-agent --lib
./scripts/test-platform.sh
./scripts/test-ui.sh tests/workspace.spec.ts
```

These tests use isolated Git/worktree fixtures, real PostgreSQL/RLS, native
paired processes and the browser. See the [proof mapping](../mappings/workspace-scope-proof-2026-09-14.md).

## Failure and Recovery

- Missing/invalid selector: fix the nearest file; discovery never skips it to
  bind an enclosing Brain. Unknown fields are rejected without echoing contents.
- Incomplete scan or Git unavailable: inspect notes, install/repair Git or choose
  a smaller workspace, then refresh. No fetch, checkout or extraction occurs.
- Ambiguous/inaccessible Brain: use its UUID and verify current Brain membership.
- Stale scope: inspect the task, review the current selection and retry with its
  current `base_scope`. Do not treat an old handle as current authorization.
- Removed view: explicitly choose a valid scope. History preserves the former
  selection; removing a view never widens it into a Brain-wide new operation.
- Lost response: inspect catalogue/task history before repeating a change.
- Closed task or capacity reached: create a new task when appropriate, and close
  finished active tasks. Existing history is preserved.

Repository snapshots, environment revision manifests, host event capture,
retrieval and MCP execution remain separate capabilities; checkout observations
and operation bindings do not establish those results.

## Optional explicit-list integration probe

For an authorized read-only test of an external workspace without writing its
selector, the `workspace_probe` Cargo example reuses the native Git observation
helper and existing refresh API. Supply a JSON array of confirmed absolute
checkout paths inside one explicit root. Keep the list and resulting refresh
JSON in Recollect's ignored `.cache/` directory. Observe first, then inspect the
normalized metadata before publishing to a clearly labeled test Brain:

```sh
cargo run -p recollect-agent --example workspace_probe -- \
  observe /path/to/workspace .cache/checkout-paths.json > .cache/checkout-refresh.json
```

Publishing requires an explicit `RECOLLECT_URL`, a separately paired non-default
`RECOLLECT_DEVICE_PROFILE` and the test Brain UUID:

```sh
cargo run -p recollect-agent --example workspace_probe -- \
  publish "$BRAIN_ID" .cache/checkout-refresh.json
```

This test path always marks the refresh partial. It neither replaces normal
selector discovery nor establishes that the supplied list includes every checkout.
Revoke/remove the probe's own credential with `recollect-agent unpair` when done.
The [SWEG smoke mapping](../mappings/sweg-workspace-smoke-2026-09-14.md) records
actual native/browser results and the retained local test Brain.
