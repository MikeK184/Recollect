# CodeGraph Local Setup Evidence

Observed: 2026-09-13
Confidence: verified

## Sources and Method

- [CodeGraph v1.6.0 README](https://github.com/colbymchenry/codegraph/blob/v1.6.0/README.md)
  and the installed package's `project-config.js`, `extraction/index.js`,
  `directory.js`, `bin/codegraph.js`, and installer/watch-policy implementation.
- Context7 library `/colbymchenry/codegraph`: successful configuration/CLI
  documentation query for ignored nested repositories, installation, and
  telemetry. Its results mixed current `include` and `includeIgnored` guidance;
  the installed 1.6.0 implementation confirmed `includeIgnored` for this layout.
- Public raw-source lookups for `v1.6.0/src/config.ts` and `src/cli.ts` returned
  404. The installed version's actual `project-config.js` and `bin/codegraph.js`
  supplied the evidence instead. No behavior depends on those failed lookups.
- `./scripts/setup-codegraph.sh` ran twice. The npm lockfile identifies exact
  package/platform versions, registry URLs, and integrity hashes. The local
  launcher reported 1.6.0; the second setup synced the existing database.
- `status --json`, `explore get_authorized_dataset --max-files 3`,
  `callers get_authorized_dataset --json`, and `node` navigation; a nested-CWD
  invocation also returned the root-index symbol. Direct Python AST checks
  verified the function/method callers against their named source definitions.
- Read-only SQLite queries inspected language counts, extraction errors, and
  indexed file paths. Local raw proof is disposable under ignored
  `.cache/codegraph/proof/`; the observations below retain the portable evidence.

## Observations

The previous PATH command was a symlink to the PSA checkout. Recollect now has
its own npm-installed package and bundled platform runtime under
`.codex/tools/codegraph/`. The launcher does not use that global command.

The root index finished its initial build in the CLI-reported 2.0 seconds:

| Measure | Observed |
| --- | --- |
| Indexed files | 3,075; 3,073 inside Cognee |
| Nodes / edges | 42,928 / 116,460 |
| Python / TypeScript / TSX files | 2,448 / 266 / 256 |
| YAML / JavaScript files | 95 / 10 |
| Database | 132,460,544 bytes; built-in Node SQLite, WAL mode |
| Index state | complete; extraction version 25 |
| Pending changes / references | zero / zero |
| Rebuild recommended | false |
| File extraction errors / excluded-path matches | zero / zero |

Exact `node get_authorized_dataset` returned source and its caller/callee trail
in approximately 0.11 seconds in one tool invocation. The next `node` call
followed `get_authorized_existing_datasets` to dataset-ID resolution and
`get_specific_user_permission_datasets` / `get_all_user_permission_datasets`.

The caller query returned 19 entries: 15 function/method callers all contained
the named Python call according to independent AST inspection. Four remaining
entries were file/import references, not four additional runtime function calls.
Direct inspection confirmed the local `datasets.list_data` branch calls this
authorization helper; its remote-client branch takes a different path.

Broader `explore` also surfaced unrelated frontend types named `Dataset` and
cross-language relationships that should not be treated as verified code paths.
This supports preferring exact symbol/file navigation and checking broad output.

Before setup, Cognee was clean at
`c0d18c80e24b7b78918e7642c03f6f128fdd2aee`; its 3,593 tracked files had combined
content/mode SHA256
`71e01ebae75f23dab36c1db3ae4f3cadbfc2cc82e4cf4bdd34a9988c8425ce1c`.
The final comparison matched the same HEAD and all tracked file contents/modes;
the worktree remained clean and no `cognee/.codegraph/` was created.

A temporary Python file under Recollect's `scripts/` verified actual incremental
refresh: `status` detected the new file, `sync` made its symbol queryable, then
deleting that owned probe and syncing removed it. Final file/node/edge counts
matched the original index with zero pending changes/references. The probe was
removed. This used no changes to Cognee source.

The launcher rejected the global agent installer with exit 2 and a missing
local installation with exit 1 and the setup command. Shell syntax checks
passed. Full governance results are recorded in the execution-pack closeout.

## Translation and Limits

The current development session can use this CLI immediately. No CodeGraph MCP
server, background watcher, Git sync hook, product service, or external Brain
connection was installed. CLI calls open the local index; explicit `sync`
refreshes it. Setup suppresses the optional upstream watch-fallback hook offer.

These timings are single local observations, not a benchmark or a promised
speedup. Exact navigation combined source and relationships in one call; no
controlled comparison against `rg` was performed on Cognee. The graph reflects
the local working tree, not an immutable published commit snapshot. Excluded
paths and zero extraction errors do not prove full semantic coverage or
authorization correctness. No Cognee application dependencies/tests were run.

## Follow-up

Use the [runbook](../runbooks/codegraph.md) to refresh after source changes.
Recheck exact source and run appropriate application tests when implementing
features. Revalidate configuration and platform behavior before changing the
pinned CodeGraph version or adding a watcher/MCP integration.
