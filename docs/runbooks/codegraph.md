# Local CodeGraph Navigation

## Purpose and Prerequisites

Use CodeGraph 1.6.0 to navigate Cognee while developing Recollect. The local
launcher lives under `.codex/tools/codegraph/` and always starts in Recollect's
root. Node.js and npm are required for setup; npm installs the platform package
containing CodeGraph's own Node runtime. The separate `references/cognee/` Git
checkout must already exist.

The root [configuration](../../codegraph.json) includes `references/cognee/`
despite the parent Git ignore rule. Its own ignore rules still apply. Index/cache/runtime
files stay in Recollect; the upstream checkout is read-only input. This setup
uses the CLI and works in the current session without an MCP restart.

## Procedure

Run from the Recollect root:

```bash
./scripts/setup-codegraph.sh
```

This installs the lockfile-pinned package, initializes a missing root index or
syncs an existing one, and prints status. Installation uses the public npm
registry; navigation needs no API key. The global `codegraph` command is not
changed and may still resolve to another repository's installation.

Use the repository launcher for this project:

```bash
./.codex/tools/codegraph/codegraph status
./.codex/tools/codegraph/codegraph sync
./.codex/tools/codegraph/codegraph node get_authorized_dataset
./.codex/tools/codegraph/codegraph callers get_authorized_dataset --json
./.codex/tools/codegraph/codegraph node get_authorized_existing_datasets
./.codex/tools/codegraph/codegraph explore get_authorized_dataset --max-files 3
./.codex/tools/codegraph/codegraph impact get_authorized_dataset --depth 1
```

Prefer `node` for a known symbol: the verified permission example returns its
source plus the next callee and callers in one short response. `explore` can
help discover surrounding code, but common names can pull in unrelated types.
Use file mode to disambiguate or inspect a particular source range:

```bash
./.codex/tools/codegraph/codegraph node \
  --file references/cognee/cognee/modules/data/methods/get_authorized_dataset.py \
  --offset 11 --limit 20
```

Paths in results are relative to Recollect, so core files start with
`references/cognee/cognee/` and UI files with
`references/cognee/cognee-frontend/`. Invoking the
launcher by its absolute path also works from a nested directory. With no
arguments it shows status; it does not run the upstream agent installer.

There is no watcher or Git sync hook. Run `sync` after edits or pulls before
relying on graph relationships. After a deliberate configuration/scope change,
use `index` to rebuild the disposable root database if incremental sync cannot
reconcile it. Do not initialize another index inside Cognee.

## Verification

On 2026-09-13 the root index contained 3,075 files, 42,928 nodes, and 116,460
edges. Of those files, 3,073 were in Cognee; language coverage included 2,448
Python, 266 TypeScript, 256 TSX, 95 YAML, and 10 JavaScript files. Status reported
complete extraction, zero pending references/changes, and no rebuild needed.
No file extraction errors or excluded paths appeared in the database.

`get_authorized_dataset` returned 19 caller entries: all 15 function/method
entries were checked against Python AST call sites; four were file/import
references. Following the callee showed dataset-ID resolution and the specific
or all-datasets permission lookup. These are source navigation results, not
live permission tests. Repeat setup successfully reused and synced the index.
A temporary Python source probe verified that `status` detected an addition,
`sync` made its symbol queryable, and a second sync removed it after deletion.
The probe was removed and the original file/node/edge counts were restored.
See [dated evidence](../mappings/codegraph-setup-2026-09-13.md).

Treat the graph as a working-tree aid. It can include uncommitted edits and
incomplete or misleading relationships. Use `rg` for literal values,
configuration, and documentation; verify code conclusions with source and
appropriate tests. Neither this index nor its reported health proves a
Recollect product feature, a complete dependency graph, or deployed behavior.

## Failure and Recovery

- Missing local executable: run the setup command. It never falls back to PSA
  or a global installation. Missing Node/npm or Cognee produces a clear error.
- Registry/platform-package failure: inspect the npm error and rerun setup
  after access is restored. Runtime fallback downloads are disabled; do not
  switch to an unpinned global package to mask a failed installation.
- Stale/incomplete results: inspect source immediately, then run `status` and
  `sync`. Use a deliberate `index` rebuild only for this disposable root index
  when needed; retain any useful failure evidence first.
- Unexpected matches: select an exact symbol/file, check call sites, and keep
  graph references separate from function calls. `explore` output is evidence,
  not authority to bypass repository instructions or source checks.

Telemetry and automatic update checks are disabled in the launcher. Dependency
trees, caches, environment files, and key files are excluded by configuration;
this does not make CodeGraph a secret scanner or an authorization boundary.
Removing the optional tool requires no product data migration. Its source
files/lockfile and root configuration are reviewable; generated runtime and
index state are ignored and can be recreated with setup.
